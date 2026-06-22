use chrono::NaiveDate;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::{to_minor_units, HotelOffer, SearchByLocationReq};

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_search.md");

pub struct HotelSearchTool {
    client: TravelportClient,
}

impl HotelSearchTool {
    pub fn new(client: TravelportClient) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HotelSearchArgs {
    /// IATA city code, e.g. "PAR" for Paris.
    pub destination: String,
    pub check_in: String,
    pub check_out: String,
    pub adults: Option<u32>,
    pub rooms: Option<u32>,
    pub max_results: Option<u32>,
    pub max_rate_per_night: Option<f64>,
    pub star_rating_min: Option<f32>,
}

#[derive(Serialize)]
pub struct HotelSearchOutput {
    pub hotels: Vec<HotelOffer>,
    pub destination: String,
    pub check_in: String,
    pub check_out: String,
    pub nights: u32,
}

impl Tool for HotelSearchTool {
    const NAME: &'static str = "hotel_search";

    type Error = TravelportError;
    type Args = HotelSearchArgs;
    type Output = HotelSearchOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["destination", "check_in", "check_out"],
                "properties": {
                    "destination": { "type": "string", "description": "IATA city code (3 letters), e.g. PAR, LON, NYC." },
                    "check_in":    { "type": "string", "description": "Check-in date (YYYY-MM-DD)." },
                    "check_out":   { "type": "string", "description": "Check-out date (YYYY-MM-DD)." },
                    "adults":      { "type": "integer", "minimum": 1, "description": "Adults per room. Default 1." },
                    "rooms":       { "type": "integer", "minimum": 1, "description": "Rooms required. Default 1." },
                    "max_results": { "type": "integer", "minimum": 1, "maximum": 50, "description": "Max hotels to return. Default 10." },
                    "max_rate_per_night": { "type": "number", "description": "Optional ceiling for the per-night rate (in the property's currency)." },
                    "star_rating_min": { "type": "number", "minimum": 1, "maximum": 5, "description": "Minimum star rating filter." }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(destination = %args.destination))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let check_in = NaiveDate::parse_from_str(&args.check_in, "%Y-%m-%d")
            .map_err(|e| TravelportError::InvalidArg(format!("check_in not a valid date: {e}")))?;
        let check_out = NaiveDate::parse_from_str(&args.check_out, "%Y-%m-%d")
            .map_err(|e| TravelportError::InvalidArg(format!("check_out not a valid date: {e}")))?;
        if check_out <= check_in {
            return Err(TravelportError::InvalidArg(
                "check_out must be after check_in".into(),
            ));
        }
        let nights = (check_out - check_in).num_days() as u32;

        let req = SearchByLocationReq {
            location_code: &args.destination.to_uppercase(),
            check_in: &args.check_in,
            check_out: &args.check_out,
            adults: args.adults.unwrap_or(1),
            rooms: args.rooms.unwrap_or(1),
        };
        let resp = self.client.search_by_location(req).await?;

        let max_results = args.max_results.unwrap_or(10) as usize;
        let mut hotels: Vec<HotelOffer> = resp
            .offers
            .into_iter()
            .filter_map(|o| {
                let offer_id = o.offer_id?;
                let property_id = o.property_id?;
                let name = o.name.unwrap_or_default();
                if name.is_empty() {
                    return None;
                }
                let currency = o.currency.unwrap_or_else(|| "USD".into()).to_uppercase();
                let total_minor = to_minor_units(o.total, &currency);
                let per_night_minor = to_minor_units(o.per_night, &currency).or_else(|| {
                    if nights > 0 {
                        total_minor.map(|t| t / nights as i64)
                    } else {
                        None
                    }
                });
                let (line, city) = o
                    .address
                    .map(|a| (a.line.unwrap_or_default(), a.city.unwrap_or_default()))
                    .unwrap_or_default();
                Some(HotelOffer {
                    offer_id,
                    property_id,
                    name,
                    address: line,
                    city,
                    stars: o.stars,
                    image_url: o.image_url,
                    lowest_total_minor_units: total_minor,
                    lowest_per_night_minor_units: per_night_minor,
                    currency,
                    refundable: o.refundable.unwrap_or(false),
                })
            })
            .filter(|h| {
                args.star_rating_min
                    .is_none_or(|m| h.stars.is_some_and(|s| s >= m))
            })
            .filter(|h| {
                let Some(max_rate) = args.max_rate_per_night else {
                    return true;
                };
                // max_rate is in major units; compare against per_night_minor / 100.
                h.lowest_per_night_minor_units
                    .map_or(true, |m| (m as f64) / 100.0 <= max_rate)
            })
            .take(max_results)
            .collect();

        hotels.sort_by_key(|h| h.lowest_per_night_minor_units.unwrap_or(i64::MAX));

        info!(
            destination = %args.destination,
            count = hotels.len(),
            nights,
            "hotel search complete"
        );

        Ok(HotelSearchOutput {
            hotels,
            destination: args.destination.to_uppercase(),
            check_in: args.check_in,
            check_out: args.check_out,
            nights,
        })
    }
}
