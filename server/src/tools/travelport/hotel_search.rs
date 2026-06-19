use chrono::NaiveDate;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use super::auth::fetch_access_token;
use super::common::{map_property, ApiSearchResponse, HotelProperty, Rate};
use super::error::{make_api_error, TravelportError};

// TODO: verify exact search endpoint from Travelport+ developer portal once credentials arrive
const HOTEL_SEARCH_URL: &str = "https://api.travelport.com/11/hotel/offers/search";

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_search.md");

pub struct HotelSearchTool {
    client_id: String,
    client_secret: String,
    http_client: reqwest::Client,
}

impl HotelSearchTool {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HotelSearchArgs {
    /// IATA city code, e.g. "PAR" for Paris, "LON" for London.
    pub destination: String,
    /// Check-in date in YYYY-MM-DD format.
    pub check_in: String,
    /// Check-out date in YYYY-MM-DD format.
    pub check_out: String,
    /// Number of adults per room (default 1).
    pub adults: Option<u32>,
    /// Number of rooms required (default 1).
    pub rooms: Option<u32>,
    /// Maximum number of properties to return (default 10).
    pub max_results: Option<u32>,
    /// Only return hotels priced at or below this nightly rate.
    pub max_rate_per_night: Option<f64>,
    /// Minimum star rating filter (1.0–5.0).
    pub star_rating_min: Option<f32>,
}

#[derive(Serialize)]
pub struct HotelSearchOutput {
    pub hotels: Vec<HotelProperty>,
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
                    "destination": {
                        "type": "string",
                        "description": "IATA city code (3 letters), e.g. \"PAR\" for Paris, \"LON\" for London, \"NYC\" for New York."
                    },
                    "check_in": {
                        "type": "string",
                        "description": "Check-in date in YYYY-MM-DD format."
                    },
                    "check_out": {
                        "type": "string",
                        "description": "Check-out date in YYYY-MM-DD format."
                    },
                    "adults": {
                        "type": "integer",
                        "description": "Number of adults per room. Defaults to 1.",
                        "minimum": 1
                    },
                    "rooms": {
                        "type": "integer",
                        "description": "Number of rooms required. Defaults to 1.",
                        "minimum": 1
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of hotels to return. Defaults to 10.",
                        "minimum": 1,
                        "maximum": 50
                    },
                    "max_rate_per_night": {
                        "type": "number",
                        "description": "Maximum nightly rate in the property's currency. Omit to return all prices."
                    },
                    "star_rating_min": {
                        "type": "number",
                        "description": "Minimum star rating (1–5). Use 4 for 4-star-and-above, 5 for luxury only.",
                        "minimum": 1,
                        "maximum": 5
                    }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(destination = %args.destination, check_in = %args.check_in, check_out = %args.check_out))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate and compute nights
        let check_in = NaiveDate::parse_from_str(&args.check_in, "%Y-%m-%d")
            .map_err(|e| TravelportError::InvalidArg(format!("check_in not a valid date: {e}")))?;
        let check_out = NaiveDate::parse_from_str(&args.check_out, "%Y-%m-%d")
            .map_err(|e| TravelportError::InvalidArg(format!("check_out not a valid date: {e}")))?;
        if check_out <= check_in {
            return Err(TravelportError::InvalidArg(
                "check_out must be after check_in".to_string(),
            ));
        }
        let nights = (check_out - check_in).num_days() as u32;

        let adults = args.adults.unwrap_or(1);
        let rooms = args.rooms.unwrap_or(1);
        let max_results = args.max_results.unwrap_or(10) as usize;

        debug!(
            client_id_len = self.client_id.len(),
            nights,
            adults,
            rooms,
            max_results,
            "searching hotels via Travelport+"
        );

        let token = fetch_access_token(&self.client_id, &self.client_secret, &self.http_client)
            .await?;

        // TODO: adjust request body to match exact Travelport+ schema from API docs
        let request_body = serde_json::json!({
            "HotelSearchModifiers": {
                "NumberOfAdults": adults,
                "NumberOfRooms": rooms,
            },
            "HotelStay": {
                "CheckinDate": args.check_in,
                "CheckoutDate": args.check_out,
            },
            "HotelLocation": {
                "LocationCode": args.destination.to_uppercase(),
            },
        });

        let response = self
            .http_client
            .post(HOTEL_SEARCH_URL)
            .bearer_auth(&token)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        debug!("hotel search response status: {status}");

        if !status.is_success() {
            return Err(make_api_error(status, body));
        }

        let api_resp: ApiSearchResponse = serde_json::from_str(&body)
            .map_err(|e| TravelportError::Parse(format!("{e}: {body}")))?;

        // Collect properties from whichever nesting level the response uses
        let raw_properties: Vec<_> = if !api_resp.hotel_properties.is_empty() {
            api_resp.hotel_properties
        } else {
            api_resp
                .response
                .map(|r| r.hotel_properties)
                .unwrap_or_default()
        };

        let mut hotels: Vec<HotelProperty> = raw_properties
            .into_iter()
            .filter_map(|p| map_property(p, nights))
            .filter(|h| {
                if let Some(min_stars) = args.star_rating_min {
                    h.star_rating.map_or(false, |s| s >= min_stars)
                } else {
                    true
                }
            })
            .filter(|h| {
                if let Some(max_rate) = args.max_rate_per_night {
                    h.lowest_rate
                        .as_ref()
                        .map_or(true, |r: &Rate| r.amount <= max_rate)
                } else {
                    true
                }
            })
            .take(max_results)
            .collect();

        // Sort by lowest nightly rate (cheapest first)
        hotels.sort_by(|a, b| {
            let ra = a.lowest_rate.as_ref().map(|r| r.amount).unwrap_or(f64::MAX);
            let rb = b.lowest_rate.as_ref().map(|r| r.amount).unwrap_or(f64::MAX);
            ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
        });

        info!(
            destination = %args.destination,
            nights,
            count = hotels.len(),
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
