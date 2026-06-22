use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::HotelDetails;

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_details.md");

pub struct HotelDetailsTool {
    client: TravelportClient,
}

impl HotelDetailsTool {
    pub fn new(client: TravelportClient) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HotelDetailsArgs {
    /// Property ID returned by `hotel_search` (carry verbatim).
    pub property_id: String,
}

impl Tool for HotelDetailsTool {
    const NAME: &'static str = "hotel_details";

    type Error = TravelportError;
    type Args = HotelDetailsArgs;
    type Output = HotelDetails;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["property_id"],
                "properties": {
                    "property_id": { "type": "string", "description": "Property ID from hotel_search." }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(property_id = %args.property_id))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let resp = self.client.hotel_details(&args.property_id).await?;
        let (line, city) = resp
            .address
            .map(|a| (a.line.unwrap_or_default(), a.city.unwrap_or_default()))
            .unwrap_or_default();
        Ok(HotelDetails {
            property_id: resp.property_id.unwrap_or(args.property_id),
            name: resp.name.unwrap_or_default(),
            description: resp.description,
            amenities: resp.amenities,
            photos: resp.photos,
            address: line,
            city,
            policies: resp.policies,
        })
    }
}
