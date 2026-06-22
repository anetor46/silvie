use chrono::NaiveDate;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::db::DbPool;
use crate::repos::hotel_bookings::{self, mark_confirmed, mark_failed, ENTITY_TYPE};
use crate::repos::payments::{log_issuing_card_creation, mark_issuing_card_cancelled};
use crate::services::stripe::PaymentClient;

use super::client::TravelportClient;
use super::error::TravelportError;
use super::models::{BookFormOfPayment, BookReq};

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_book.md");

pub struct HotelBookTool {
    travelport: TravelportClient,
    stripe_key: String,
    customer_id: String,
    payment_method_id: String,
    user_id: Uuid,
    conversation_id: Option<Uuid>,
    db_pool: DbPool,
}

pub struct HotelBookToolDeps {
    pub travelport: TravelportClient,
    pub stripe_key: String,
    pub customer_id: String,
    pub payment_method_id: String,
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub db_pool: DbPool,
}

impl HotelBookTool {
    pub fn new(deps: HotelBookToolDeps) -> Self {
        Self {
            travelport: deps.travelport,
            stripe_key: deps.stripe_key,
            customer_id: deps.customer_id,
            payment_method_id: deps.payment_method_id,
            user_id: deps.user_id,
            conversation_id: deps.conversation_id,
            db_pool: deps.db_pool,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HotelBookArgs {
    /// Property ID from `hotel_search`.
    pub property_id: String,
    /// Offer ID from `hotel_availability` (rate is freshest there).
    pub offer_id: String,
    /// Rate ID from `hotel_availability`.
    pub rate_id: String,
    pub hotel_name: String,
    pub check_in: String,
    pub check_out: String,
    /// Number of guests on the reservation.
    pub guests: u32,
    /// Lead guest full name (required by the GDS).
    pub guest_name: String,
    pub guest_email: Option<String>,
    /// Stay total in the currency's smallest unit (e.g. cents). Multiply
    /// displayed price by 100 for USD/EUR/GBP.
    pub total_price_minor_units: u64,
    /// ISO 4217 currency code, uppercase ("USD", "EUR", "GBP").
    pub currency: String,
}

#[derive(Serialize)]
pub struct HotelBookOutput {
    pub booking_id: Uuid,
    pub reservation_id: String,
    pub hotel_name: String,
    pub check_in: String,
    pub check_out: String,
    pub total_charged_minor_units: u64,
    pub currency: String,
    pub status: String,
    pub refundable: bool,
}

impl Tool for HotelBookTool {
    const NAME: &'static str = "hotel_book";

    type Error = TravelportError;
    type Args = HotelBookArgs;
    type Output = HotelBookOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: DESCRIPTION.trim().to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": [
                    "property_id", "offer_id", "rate_id", "hotel_name",
                    "check_in", "check_out", "guests", "guest_name",
                    "total_price_minor_units", "currency"
                ],
                "properties": {
                    "property_id":   { "type": "string" },
                    "offer_id":      { "type": "string" },
                    "rate_id":       { "type": "string" },
                    "hotel_name":    { "type": "string" },
                    "check_in":      { "type": "string", "description": "YYYY-MM-DD" },
                    "check_out":     { "type": "string", "description": "YYYY-MM-DD" },
                    "guests":        { "type": "integer", "minimum": 1 },
                    "guest_name":    { "type": "string", "description": "Lead guest full name." },
                    "guest_email":   { "type": "string" },
                    "total_price_minor_units": { "type": "integer", "minimum": 1, "description": "Stay total in minor units (multiply USD/EUR/GBP by 100)." },
                    "currency":      { "type": "string", "enum": ["USD", "EUR", "GBP"] }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(
        property_id = %args.property_id,
        offer_id = %args.offer_id,
        currency = %args.currency,
        total = args.total_price_minor_units,
    ))]
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
        let currency = args.currency.to_uppercase();
        if !matches!(currency.as_str(), "USD" | "EUR" | "GBP") {
            return Err(TravelportError::InvalidArg(format!(
                "unsupported currency '{currency}'; allowed: USD, EUR, GBP"
            )));
        }

        // Step 1: persist a pending booking row so we have our own id.
        let booking_id = hotel_bookings::insert_pending(
            &self.db_pool,
            hotel_bookings::InsertPending {
                user_id: self.user_id,
                conversation_id: self.conversation_id,
                travelport_property_id: &args.property_id,
                travelport_offer_id: Some(&args.offer_id),
                hotel_name: &args.hotel_name,
                check_in,
                check_out,
                guests: args.guests as i32,
                rooms: 1,
                total_amount_minor_units: args.total_price_minor_units as i64,
                currency: &currency,
                payment_method_id: Some(&self.payment_method_id),
            },
        )
        .await
        .map_err(|e| {
            error!("failed to insert pending hotel_bookings row: {e:#}");
            TravelportError::InvalidArg(format!("could not record pending booking: {e}"))
        })?;

        let payment_client = PaymentClient::new(self.stripe_key.clone());

        // Step 2: authorise the customer's PM for the booking amount.
        let intent = match payment_client
            .create_and_confirm_intent(
                &self.customer_id,
                &self.payment_method_id,
                args.total_price_minor_units,
                &currency,
                &[("booking_id", &booking_id.to_string())],
            )
            .await
        {
            Ok(i) => i,
            Err(e) => {
                let reason = format!("payment authorisation declined: {e}");
                error!("{reason}");
                let _ = mark_failed(&self.db_pool, booking_id, &reason).await;
                return Err(TravelportError::InvalidArg(reason));
            }
        };
        if let Err(e) =
            hotel_bookings::attach_payment_intent(&self.db_pool, booking_id, &intent.id).await
        {
            warn!("failed to attach payment intent {} to booking {booking_id}: {e:#}", intent.id);
        }

        // Step 3: issue the single-use virtual card Travelport will charge.
        let card = match payment_client
            .create_booking_card(
                &self.customer_id,
                &self.payment_method_id,
                args.total_price_minor_units,
                &currency,
            )
            .await
        {
            Ok(c) => c,
            Err(e) => {
                let reason = format!("issuing card creation failed: {e}");
                error!("{reason}");
                let _ = payment_client.cancel_intent(&intent.id).await;
                let _ = mark_failed(&self.db_pool, booking_id, &reason).await;
                return Err(TravelportError::InvalidArg(reason));
            }
        };
        let card_id = card.id.clone();
        if let Err(e) = log_issuing_card_creation(
            &self.db_pool,
            &self.payment_method_id,
            &card_id,
            args.total_price_minor_units as i64,
            &currency,
            Some(ENTITY_TYPE),
            Some(booking_id),
        )
        .await
        {
            warn!("failed to record issuing_card_log entry for {card_id}: {e:#}");
        }

        // Step 4: submit the Travelport reservation.
        let book_req = BookReq {
            property_id: &args.property_id,
            offer_id: &args.offer_id,
            rate_id: &args.rate_id,
            check_in: &args.check_in,
            check_out: &args.check_out,
            guests: args.guests,
            guest_name: &args.guest_name,
            guest_email: args.guest_email.as_deref(),
            form_of_payment: BookFormOfPayment {
                card_number: &card.pan,
                exp_month: card.exp_month,
                exp_year: card.exp_year,
                cvv: &card.cvv,
            },
        };
        let book_outcome = self.travelport.book(book_req).await;

        // Step 5: cancel the issuing card regardless of outcome.
        if let Err(e) = payment_client.cancel_issuing_card(&card_id).await {
            warn!("failed to cancel Issuing card {card_id}: {e:#}");
        }
        if let Err(e) = mark_issuing_card_cancelled(&self.db_pool, &card_id).await {
            warn!("failed to mark issuing_card_log row cancelled for {card_id}: {e:#}");
        }

        match book_outcome {
            Ok(resp) => {
                let reservation_id = resp
                    .reservation_id
                    .ok_or_else(|| TravelportError::Parse("missing reservation id".into()))?;
                let refundable = resp
                    .cancellation_policy
                    .as_ref()
                    .and_then(|p| p.refundable)
                    .unwrap_or(false);
                let policy_json = resp.cancellation_policy.as_ref().and_then(|p| {
                    serde_json::to_value(p.clone().into_domain(&currency)).ok()
                });

                // Capture the pre-auth.
                if let Err(e) = payment_client.capture_intent(&intent.id).await {
                    warn!("Travelport booking succeeded but PaymentIntent capture failed: {e:#}");
                    let reason = format!(
                        "booking confirmed at supplier ({reservation_id}) but payment capture failed: {e}"
                    );
                    let _ = mark_failed(&self.db_pool, booking_id, &reason).await;
                    return Err(TravelportError::InvalidArg(reason));
                }

                if let Err(e) = mark_confirmed(
                    &self.db_pool,
                    booking_id,
                    &reservation_id,
                    policy_json,
                )
                .await
                {
                    warn!("booking confirmed at Travelport but DB update failed: {e:#}");
                }

                info!(
                    booking_id = %booking_id,
                    reservation_id = %reservation_id,
                    "hotel booking confirmed"
                );
                Ok(HotelBookOutput {
                    booking_id,
                    reservation_id,
                    hotel_name: args.hotel_name,
                    check_in: args.check_in,
                    check_out: args.check_out,
                    total_charged_minor_units: args.total_price_minor_units,
                    currency,
                    status: resp.status.unwrap_or_else(|| "confirmed".into()),
                    refundable,
                })
            }
            Err(e) => {
                // Booking failed — release the hold so we don't charge the customer.
                if let Err(cancel_err) = payment_client.cancel_intent(&intent.id).await {
                    warn!("failed to release PaymentIntent {} after booking failure: {cancel_err:#}", intent.id);
                }
                let reason = format!("Travelport booking failed: {e}");
                let _ = mark_failed(&self.db_pool, booking_id, &reason).await;
                Err(e)
            }
        }
    }
}
