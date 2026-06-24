use chrono::NaiveDate;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::{
    to_minor_units, AvailGuestCount, AvailGuestCounts, AvailRoomStayCandidate, AvailabilityReq,
    CancellationPolicy, CancelPenalty, CatalogOfferingsQueryRequest,
    CatalogOfferingsRequestHospitality, HotelSearchCriterion, PropertyKey, PropertyRequest,
    RateQuote, RoomStayCandidates, StayDates,
};

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_availability.md");

const AGGREGATOR_TVPT: &str = "TVPT";

pub struct HotelAvailabilityTool {
    client: TravelportClient,
}

impl HotelAvailabilityTool {
    pub fn new(client: TravelportClient) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HotelAvailabilityArgs {
    pub property_id: String,
    pub check_in: String,
    pub check_out: String,
    pub adults: Option<u32>,
    pub rooms: Option<u32>,
    pub currency: Option<String>,
}

#[derive(Serialize)]
pub struct HotelAvailabilityOutput {
    pub property_id: String,
    pub rates: Vec<RateQuote>,
}

impl Tool for HotelAvailabilityTool {
    const NAME: &'static str = "hotel_availability";

    type Error = TravelportError;
    type Args = HotelAvailabilityArgs;
    type Output = HotelAvailabilityOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["property_id", "check_in", "check_out"],
                "properties": {
                    "property_id": { "type": "string", "description": "Composite property id from hotel_search (chainCode-propertyCode)." },
                    "check_in":    { "type": "string", "description": "YYYY-MM-DD" },
                    "check_out":   { "type": "string", "description": "YYYY-MM-DD" },
                    "adults":      { "type": "integer", "minimum": 1 },
                    "rooms":       { "type": "integer", "minimum": 1 },
                    "currency":    { "type": "string", "description": "ISO 4217 (USD/EUR/GBP). Default USD." }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(property_id = %args.property_id))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        NaiveDate::parse_from_str(&args.check_in, "%Y-%m-%d")
            .map_err(|e| TravelportError::InvalidArg(format!("check_in not a valid date: {e}")))?;
        NaiveDate::parse_from_str(&args.check_out, "%Y-%m-%d")
            .map_err(|e| TravelportError::InvalidArg(format!("check_out not a valid date: {e}")))?;

        let (chain, code) = super::models::split_property_id(&args.property_id).ok_or_else(
            || {
                TravelportError::InvalidArg(format!(
                    "property_id '{}' is not in 'chainCode-propertyCode' form",
                    args.property_id
                ))
            },
        )?;
        let adults = args.adults.unwrap_or(1);
        let rooms = args.rooms.unwrap_or(1);
        let currency = args
            .currency
            .clone()
            .unwrap_or_else(|| "USD".into())
            .to_uppercase();

        let req = AvailabilityReq {
            query: CatalogOfferingsQueryRequest {
                catalog_offerings_request: vec![CatalogOfferingsRequestHospitality {
                    type_: "CatalogOfferingsRequestHospitality",
                    requested_currency: currency.clone(),
                    stay_dates: StayDates {
                        start: args.check_in.clone(),
                        end: args.check_out.clone(),
                    },
                    hotel_search_criterion: HotelSearchCriterion {
                        type_: "HotelSearchCriterion",
                        number_of_rooms: rooms,
                        aggregator_list: vec![AGGREGATOR_TVPT],
                        property_request: vec![PropertyRequest {
                            type_: "PropertyRequest",
                            property_key: PropertyKey {
                                chain_code: chain,
                                property_code: code,
                            },
                        }],
                        room_stay_candidates: RoomStayCandidates {
                            room_stay_candidate: vec![AvailRoomStayCandidate {
                                guest_counts: AvailGuestCounts {
                                    guest_count: vec![AvailGuestCount {
                                        count: adults.to_string(),
                                    }],
                                },
                            }],
                        },
                    },
                }],
            },
        };
        let resp = self.client.availability(req).await?;

        // Doc-verified path:
        //   CatalogOfferingsHospitalityResponse.CatalogOfferings.CatalogOffering[]
        let offerings = resp
            .catalog_offerings_response
            .and_then(|r| r.catalog_offerings)
            .map(|c| c.catalog_offering)
            .unwrap_or_default();

        let rates = offerings
            .into_iter()
            .enumerate()
            .map(|(idx, o)| -> Result<RateQuote, TravelportError> {
                // CatalogOffering.id is the cached offer identifier we
                // pass back to the booking endpoint as
                // CatalogOfferingIdentifier.value.
                let offer_id = o.id.ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "availability offering #{idx} missing CatalogOffering.id"
                    ))
                })?;

                // bookingCode lives at ProductOptions[].Product[].bookingCode
                // — pick the first Product of the first ProductOption.
                let product = o
                    .product_options
                    .into_iter()
                    .next()
                    .and_then(|po| po.product.into_iter().next())
                    .ok_or_else(|| {
                        TravelportError::Parse(format!(
                            "availability offering #{idx} has no ProductOptions[].Product[]"
                        ))
                    })?;
                let booking_code = product.booking_code.ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "availability offering #{idx} Product missing bookingCode (rate_id)"
                    ))
                })?;
                let room_description = product
                    .room_type
                    .and_then(|rt| rt.description)
                    .and_then(|d| d.value);

                let price = o.price.ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "availability offering #{idx} missing Price"
                    ))
                })?;
                let total_major = price.total_price.ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "availability offering #{idx} missing Price.TotalPrice"
                    ))
                })?;
                let currency = price
                    .currency_code
                    .as_ref()
                    .and_then(|c| c.value.clone())
                    .ok_or_else(|| {
                        TravelportError::Parse(format!(
                            "availability offering #{idx} missing Price.CurrencyCode.value"
                        ))
                    })?;
                let total_minor = to_minor_units(Some(total_major)).ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "availability offering #{idx} total amount is non-numeric"
                    ))
                })?;

                let cancellation_policy = o
                    .terms_and_conditions
                    .and_then(|t| t.cancel_penalty)
                    .map(|p| map_cancel_penalty(p, total_minor, &currency));
                let refundable = cancellation_policy
                    .as_ref()
                    .map(|p| p.refundable)
                    .unwrap_or(false);

                Ok(RateQuote {
                    offer_id,
                    rate_id: booking_code,
                    room_description,
                    total_minor_units: total_minor,
                    currency: currency.to_uppercase(),
                    refundable,
                    cancellation_policy,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        info!(count = rates.len(), "availability returned rates");
        Ok(HotelAvailabilityOutput {
            property_id: args.property_id,
            rates,
        })
    }
}

/// Translate Travelport's `CancelPenalty` into our domain
/// `CancellationPolicy`. Travelport encodes refundability as the strings
/// "Yes" / "No" — both are accepted defensively (lowercase too) and
/// anything else falls back to non-refundable.
fn map_cancel_penalty(p: CancelPenalty, total_minor: i64, _currency: &str) -> CancellationPolicy {
    let refundable = matches!(
        p.refundable.as_deref().map(str::trim).unwrap_or(""),
        "Yes" | "yes" | "YES" | "true"
    );
    let refund_deadline = p
        .deadline
        .and_then(|d| d.specific_date)
        .and_then(|s| s.start);
    // Penalty: prefer absolute amount → percent of total → nights
    // unhandled. If only Percent is present we apply it to the total.
    let penalty_minor_units = p.hotel_penalty.as_ref().and_then(|hp| {
        if let Some(amt) = hp.amount {
            to_minor_units(Some(amt))
        } else if let Some(pct) = hp.percent {
            Some(((total_minor as f64) * pct / 100.0).round() as i64)
        } else {
            None
        }
    });
    CancellationPolicy {
        refundable,
        refund_deadline,
        penalty_minor_units,
        description: p.description,
    }
}
