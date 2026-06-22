use chrono::Utc;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::db::DbPool;
use crate::repos::hotel_bookings::{self, mark_cancelled};
use crate::services::stripe::PaymentClient;

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::CancellationPolicy;

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_cancel_booking.md");

pub struct HotelCancelBookingTool {
    travelport: TravelportClient,
    stripe_key: String,
    db_pool: DbPool,
    user_id: Uuid,
}

impl HotelCancelBookingTool {
    pub fn new(
        travelport: TravelportClient,
        stripe_key: String,
        db_pool: DbPool,
        user_id: Uuid,
    ) -> Self {
        Self {
            travelport,
            stripe_key,
            db_pool,
            user_id,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HotelCancelBookingArgs {
    /// Our booking id (UUID).
    pub booking_id: Uuid,
    /// Belt-and-braces: must be `true` (the LLM also needs to confirm via the
    /// write-tool harness; this prevents accidental cancellation if the LLM
    /// invokes the tool without explicit user consent).
    pub confirm: bool,
}

#[derive(Serialize)]
pub struct HotelCancelBookingOutput {
    pub booking_id: Uuid,
    pub status: String,
    pub cancelled_at: String,
    pub refunded_amount_minor_units: i64,
    pub currency: String,
    pub refundable: bool,
}

impl Tool for HotelCancelBookingTool {
    const NAME: &'static str = "hotel_cancel_booking";

    type Error = TravelportError;
    type Args = HotelCancelBookingArgs;
    type Output = HotelCancelBookingOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["booking_id", "confirm"],
                "properties": {
                    "booking_id": { "type": "string", "description": "Booking UUID returned by hotel_book." },
                    "confirm":    { "type": "boolean", "description": "Must be true — explicit user confirmation that the cancellation should proceed." }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(booking_id = %args.booking_id))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if !args.confirm {
            return Err(TravelportError::InvalidArg(
                "cancellation requires explicit user confirmation".into(),
            ));
        }
        let row = hotel_bookings::get_by_id(&self.db_pool, self.user_id, args.booking_id)
            .await
            .map_err(|e| TravelportError::InvalidArg(format!("DB lookup failed: {e}")))?
            .ok_or_else(|| TravelportError::InvalidArg("booking not found".into()))?;

        if row.status != "confirmed" {
            return Err(TravelportError::InvalidArg(format!(
                "booking is in status '{}' and cannot be cancelled",
                row.status
            )));
        }
        let reservation_id = row.travelport_reservation_id.clone().ok_or_else(|| {
            TravelportError::InvalidArg("booking has no Travelport reservation id".into())
        })?;

        let policy: Option<CancellationPolicy> = row
            .cancellation_policy
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let refundable_minor = compute_refundable_minor(&policy, row.total_amount_minor_units);

        // Cancel at the supplier first.
        let cancel_resp = self.travelport.cancel(&reservation_id).await?;

        // Issue the refund (if any). Best-effort: even if refund fails we
        // still want the booking marked cancelled at our side; the user
        // will see the discrepancy and can ask us to retry.
        let mut refunded = 0_i64;
        if refundable_minor > 0 {
            if let Some(intent_id) = row.stripe_payment_intent_id.as_deref() {
                let pc = PaymentClient::new(self.stripe_key.clone());
                match pc.refund_intent(intent_id, refundable_minor as u64).await {
                    Ok(refund) => {
                        refunded = refund.amount.unwrap_or(refundable_minor);
                        info!(refund_id = %refund.id, amount = refunded, "refund issued");
                    }
                    Err(e) => warn!("refund failed for booking {}: {e:#}", row.id),
                }
            } else {
                warn!(
                    "no payment intent on booking {} — refund of {} {} skipped",
                    row.id, refundable_minor, row.currency
                );
            }
        }

        if let Err(e) = mark_cancelled(&self.db_pool, row.id, Some(refunded)).await {
            warn!("failed to mark booking {} cancelled in DB: {e:#}", row.id);
        }

        let now = Utc::now().to_rfc3339();
        Ok(HotelCancelBookingOutput {
            booking_id: row.id,
            status: cancel_resp.status.unwrap_or_else(|| "cancelled".into()),
            cancelled_at: now,
            refunded_amount_minor_units: refunded,
            currency: row.currency,
            refundable: refundable_minor > 0,
        })
    }
}

/// Refundable amount in minor units. The simple model: if the policy is
/// refundable and we're before the deadline (or no deadline given), refund
/// the full total minus any quoted penalty. Otherwise no refund.
fn compute_refundable_minor(
    policy: &Option<CancellationPolicy>,
    total_minor: i64,
) -> i64 {
    let Some(p) = policy else {
        return 0;
    };
    if !p.refundable {
        return 0;
    }
    if let Some(deadline) = &p.refund_deadline {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(deadline) {
            if dt < Utc::now() {
                return 0;
            }
        }
    }
    let penalty = p.penalty_minor_units.unwrap_or(0);
    (total_minor - penalty).max(0)
}
