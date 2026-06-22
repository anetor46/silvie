use chrono::NaiveDate;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::{
    split_property_id, to_minor_units, AvailGuestCount, AvailGuestCounts, AvailRoomStayCandidate,
    AvailabilityReq, CatalogOfferingsQueryRequest, CatalogOfferingsRequestHospitality,
    HotelSearchCriterion, PropertyKey, PropertyRequest, RateQuote, RoomStayCandidates, StayDates,
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
    /// ISO 4217. Default "USD". Travelport requires a `requestedCurrency`.
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

        let (chain, code) = split_property_id(&args.property_id).ok_or_else(|| {
            TravelportError::InvalidArg(format!(
                "property_id '{}' is not in 'chainCode-propertyCode' form",
                args.property_id
            ))
        })?;
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

        // Strict envelope parsing: if Travelport's shape diverges from what
        // we expect, surface that as a concrete parse error rather than
        // silently returning "no availability".
        let catalog_offerings_response = resp.catalog_offerings_response.ok_or_else(|| {
            TravelportError::Parse(
                "availability response missing top-level CatalogOfferingsResponse".into(),
            )
        })?;
        let catalog_offerings = catalog_offerings_response.catalog_offerings.ok_or_else(|| {
            TravelportError::Parse(
                "availability response missing CatalogOfferings inside CatalogOfferingsResponse"
                    .into(),
            )
        })?;
        let offerings = catalog_offerings.catalog_offering;

        let rates = offerings
            .into_iter()
            .enumerate()
            .map(|(idx, o)| -> Result<RateQuote, TravelportError> {
                let offer_id = o
                    .identifier
                    .and_then(|i| i.value)
                    .ok_or_else(|| TravelportError::Parse(format!(
                        "availability offering #{idx} missing Identifier.value (offer_id)"
                    )))?;
                let product = o.product.into_iter().next().ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "availability offering #{idx} has no Product entries"
                    ))
                })?;
                let booking_code = product.booking_code.ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "availability offering #{idx} Product missing bookingCode (rate_id)"
                    ))
                })?;
                let room_description = product.product_description.and_then(|t| t.value);

                let (total_major, currency) = if let Some(price) = o.price.as_ref() {
                    (
                        price.total_price,
                        price
                            .currency_code
                            .as_ref()
                            .and_then(|c| c.value.clone())
                            .ok_or_else(|| TravelportError::Parse(format!(
                                "availability offering #{idx} Price missing CurrencyCode.value"
                            )))?,
                    )
                } else if let Some(tp) = o.total_price.as_ref() {
                    (
                        tp.value,
                        tp.code.clone().ok_or_else(|| TravelportError::Parse(format!(
                            "availability offering #{idx} TotalPrice missing currency code"
                        )))?,
                    )
                } else {
                    return Err(TravelportError::Parse(format!(
                        "availability offering #{idx} has neither Price nor TotalPrice"
                    )));
                };
                let total_minor = to_minor_units(total_major).ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "availability offering #{idx} total amount is missing or non-numeric"
                    ))
                })?;

                Ok(RateQuote {
                    offer_id,
                    rate_id: booking_code,
                    room_description,
                    total_minor_units: total_minor,
                    currency: currency.to_uppercase(),
                    // Travelport v11 doesn't surface refundability on the
                    // availability rate object in a stable way — leave it
                    // false until the booking response confirms otherwise.
                    refundable: false,
                    cancellation_policy: None,
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
