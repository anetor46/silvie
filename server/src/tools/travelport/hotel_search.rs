use chrono::NaiveDate;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::{
    join_property_id, to_minor_units, GuestCount, GuestCounts, HotelOffer, PropertiesQuerySearch,
    RoomStayCandidate, SearchByCity, SearchByLocationReq, SearchRadius,
};

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_search.md");

const AGGREGATOR_TVPT: &str = "TVPT";
const ADULT_AGE_QUALIFYING_CODE: &str = "10";

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
    /// IATA city code (3 letters), e.g. PAR, LON, NYC.
    pub destination: String,
    pub check_in: String,
    pub check_out: String,
    pub adults: Option<u32>,
    pub rooms: Option<u32>,
    pub max_results: Option<u32>,
    /// Search radius around the city centre, in miles. Default 5.
    pub radius_miles: Option<u32>,
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
                    "radius_miles": { "type": "integer", "minimum": 1, "maximum": 50, "description": "Search radius around the city centre. Default 5." },
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

        let adults = args.adults.unwrap_or(1);
        let rooms = args.rooms.unwrap_or(1);

        // Build one RoomStayCandidate per requested room, each with `adults`
        // adult-age guests.
        let mut room_stay_candidate = Vec::with_capacity(rooms as usize);
        for _ in 0..rooms.max(1) {
            room_stay_candidate.push(RoomStayCandidate {
                type_: "RoomStayCandidate",
                guest_counts: GuestCounts {
                    type_: "GuestCounts",
                    guest_count: vec![GuestCount {
                        type_: "GuestCount",
                        count: adults,
                        age_qualifying_code: ADULT_AGE_QUALIFYING_CODE,
                    }],
                },
            });
        }

        let req = SearchByLocationReq {
            query: PropertiesQuerySearch {
                check_in_date: args.check_in.clone(),
                check_out_date: args.check_out.clone(),
                aggregator_list: vec![AGGREGATOR_TVPT],
                room_stay_candidate,
                search_by: SearchByCity {
                    type_: "SearchByCity",
                    search_radius: SearchRadius {
                        value: args.radius_miles.unwrap_or(5),
                        unit_of_distance: "Miles",
                    },
                    search_city: args.destination.to_uppercase(),
                },
            },
        };
        let resp = self.client.search_by_location(req).await?;
        // Strict envelope parsing — if Travelport's shape diverges,
        // `UnexpectedResponse` is emitted by the client with the body
        // already logged. The optional fields here only catch the case
        // where the documented envelope is present but inner sections
        // are missing.
        let properties_response = resp.properties_response.ok_or_else(|| {
            TravelportError::Parse(
                "search response missing top-level PropertiesResponse".into(),
            )
        })?;
        let properties = properties_response.properties.ok_or_else(|| {
            TravelportError::Parse(
                "search response missing Properties inside PropertiesResponse".into(),
            )
        })?;
        let infos = properties.property_info;

        let max_results = args.max_results.unwrap_or(10) as usize;
        let mut hotels: Vec<HotelOffer> = infos
            .into_iter()
            .enumerate()
            .map(|(idx, pi)| -> Result<HotelOffer, TravelportError> {
                let property = pi.property.ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "search result #{idx} missing Property body"
                    ))
                })?;
                let property_key = property.property_key.ok_or_else(|| {
                    TravelportError::Parse(format!(
                        "search result #{idx} missing Property.PropertyKey"
                    ))
                })?;
                let property_id =
                    join_property_id(&property_key.chain_code, &property_key.property_code);
                let name = property.name.unwrap_or_default();
                if name.is_empty() {
                    return Err(TravelportError::Parse(format!(
                        "search result #{idx} ({property_id}) has empty Property.name"
                    )));
                }

                let (line, city) = property
                    .address
                    .map(|a| (a.address_line.join(", "), a.city.unwrap_or_default()))
                    .unwrap_or_default();
                let stars = property.rating.into_iter().filter_map(|r| r.value).next();
                let image_url = property.image.into_iter().filter_map(|i| i.value).next();

                // LowestAvailableRate is documented but may legitimately be
                // absent for sold-out properties — leave the price fields as
                // None in that case rather than erroring.
                let rate = pi.lowest_available_rate;
                let total_minor = rate.as_ref().and_then(|r| to_minor_units(r.value));
                let per_night_minor = total_minor.and_then(|t| {
                    if nights > 0 {
                        Some(t / nights as i64)
                    } else {
                        None
                    }
                });
                let currency = rate
                    .and_then(|r| r.code)
                    .unwrap_or_else(|| "USD".into())
                    .to_uppercase();

                let distance_km = pi.distance.and_then(|d| {
                    let v = d.value?;
                    let units = d.unit_of_distance.unwrap_or_default().to_lowercase();
                    if units.contains("mi") {
                        Some(v * 1.609_34)
                    } else {
                        Some(v)
                    }
                });

                Ok(HotelOffer {
                    property_id,
                    name,
                    address: line,
                    city,
                    stars,
                    image_url,
                    lowest_total_minor_units: total_minor,
                    lowest_per_night_minor_units: per_night_minor,
                    currency,
                    distance_km,
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|h| {
                args.star_rating_min
                    .is_none_or(|m| h.stars.is_some_and(|s| s >= m))
            })
            .filter(|h| {
                let Some(max_rate) = args.max_rate_per_night else {
                    return true;
                };
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

