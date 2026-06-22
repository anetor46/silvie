use chrono::NaiveDate;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::{to_minor_units, AvailabilityReq, RateQuote};

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_availability.md");

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
                    "property_id": { "type": "string", "description": "Property ID from hotel_search." },
                    "check_in":    { "type": "string", "description": "Check-in (YYYY-MM-DD)." },
                    "check_out":   { "type": "string", "description": "Check-out (YYYY-MM-DD)." },
                    "adults":      { "type": "integer", "minimum": 1, "description": "Adults per room. Default 1." },
                    "rooms":       { "type": "integer", "minimum": 1, "description": "Rooms required. Default 1." }
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

        let req = AvailabilityReq {
            property_id: &args.property_id,
            check_in: &args.check_in,
            check_out: &args.check_out,
            adults: args.adults.unwrap_or(1),
            rooms: args.rooms.unwrap_or(1),
        };
        let resp = self.client.availability(req).await?;
        let rates: Vec<RateQuote> = resp
            .rates
            .into_iter()
            .filter_map(|r| {
                let rate_id = r.rate_id?;
                let currency = r.currency.unwrap_or_else(|| "USD".into()).to_uppercase();
                let total = to_minor_units(r.total, &currency)?;
                Some(RateQuote {
                    offer_id: r.offer_id.unwrap_or_default(),
                    rate_id,
                    room_description: r.room_description,
                    total_minor_units: total,
                    currency: currency.clone(),
                    refundable: r.refundable.unwrap_or(false),
                    cancellation_policy: r.cancellation_policy.map(|p| p.into_domain(&currency)),
                })
            })
            .collect();
        info!(count = rates.len(), "availability returned rates");
        Ok(HotelAvailabilityOutput {
            property_id: args.property_id,
            rates,
        })
    }
}
