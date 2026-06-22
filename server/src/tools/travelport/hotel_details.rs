use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::{split_property_id, HotelDetails};

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
    /// Composite property id from `hotel_search` (e.g. "DT-35429").
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
                    "property_id": { "type": "string", "description": "Composite property id from hotel_search (chainCode-propertyCode)." }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(property_id = %args.property_id))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let (chain, code) = split_property_id(&args.property_id).ok_or_else(|| {
            TravelportError::InvalidArg(format!(
                "property_id '{}' is not in 'chainCode-propertyCode' form",
                args.property_id
            ))
        })?;
        let resp = self.client.hotel_details(&chain, &code).await?;
        let info = resp
            .properties_response
            .and_then(|p| p.properties)
            .and_then(|p| p.property_info.into_iter().next())
            .ok_or_else(|| {
                TravelportError::Parse("details response contained no PropertyInfo".into())
            })?;
        let property = info.property.ok_or_else(|| {
            TravelportError::Parse("details response missing Property body".into())
        })?;
        let (address, city) = property
            .address
            .map(|a| (a.address_line.join(", "), a.city.unwrap_or_default()))
            .unwrap_or_default();
        let amenities = property
            .property_amenity
            .into_iter()
            .filter_map(|a| a.description)
            .collect();
        let photos = property
            .image
            .into_iter()
            .filter_map(|i| i.value)
            .collect();

        Ok(HotelDetails {
            property_id: args.property_id,
            name: property.name.unwrap_or_default(),
            description: None,
            amenities,
            photos,
            address,
            city,
        })
    }
}
