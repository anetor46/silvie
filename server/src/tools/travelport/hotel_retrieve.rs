use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::db::DbPool;
use crate::repos::hotel_bookings::{self};

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::CancellationPolicy;

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_retrieve_booking.md");

pub struct HotelRetrieveBookingTool {
    travelport: TravelportClient,
    db_pool: DbPool,
    user_id: Uuid,
}

impl HotelRetrieveBookingTool {
    pub fn new(travelport: TravelportClient, db_pool: DbPool, user_id: Uuid) -> Self {
        Self {
            travelport,
            db_pool,
            user_id,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HotelRetrieveBookingArgs {
    /// Our booking id (UUID) — returned by `hotel_book`.
    pub booking_id: Uuid,
}

#[derive(Serialize)]
pub struct HotelRetrieveBookingOutput {
    pub booking_id: Uuid,
    pub reservation_id: Option<String>,
    pub hotel_name: String,
    pub check_in: String,
    pub check_out: String,
    pub status: String,
    pub total_amount_minor_units: i64,
    pub currency: String,
    pub cancellation_policy: Option<CancellationPolicy>,
    pub supplier_status: Option<String>,
}

impl Tool for HotelRetrieveBookingTool {
    const NAME: &'static str = "hotel_retrieve_booking";

    type Error = TravelportError;
    type Args = HotelRetrieveBookingArgs;
    type Output = HotelRetrieveBookingOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["booking_id"],
                "properties": {
                    "booking_id": { "type": "string", "description": "Booking UUID returned by hotel_book." }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(booking_id = %args.booking_id))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let row = hotel_bookings::get_by_id(&self.db_pool, self.user_id, args.booking_id)
            .await
            .map_err(|e| TravelportError::InvalidArg(format!("DB lookup failed: {e}")))?
            .ok_or_else(|| TravelportError::InvalidArg("booking not found".into()))?;

        let mut supplier_status = None;
        if let Some(reservation_id) = row.travelport_reservation_id.as_deref() {
            match self.travelport.retrieve(reservation_id).await {
                Ok(resp) => supplier_status = resp.status,
                Err(e) => warn!("Travelport retrieve failed for {reservation_id}: {e}"),
            }
        }

        let cancellation_policy: Option<CancellationPolicy> = row
            .cancellation_policy
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        Ok(HotelRetrieveBookingOutput {
            booking_id: row.id,
            reservation_id: row.travelport_reservation_id,
            hotel_name: row.hotel_name,
            check_in: row.check_in.to_string(),
            check_out: row.check_out.to_string(),
            status: row.status,
            total_amount_minor_units: row.total_amount_minor_units,
            currency: row.currency,
            cancellation_policy,
            supplier_status,
        })
    }
}
