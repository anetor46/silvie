use chrono::NaiveDate;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};

use crate::db::DbPool;
use crate::payments::{log_issuing_card_creation, mark_issuing_card_cancelled, PaymentClient};
use super::auth::fetch_access_token;
use super::error::{make_api_error, TravelportError};

// TODO: verify exact booking endpoint from Travelport+ API docs
const HOTEL_BOOK_URL: &str = "https://api.travelport.com/11/hotel/offers/book";

const DESCRIPTION: &str = include_str!("../../../prompts/travelport/hotel_book.md");

pub struct HotelBookTool {
    tp_client_id: String,
    tp_client_secret: String,
    stripe_key: String,
    customer_id: String,
    payment_method_id: String,
    http_client: reqwest::Client,
    db_pool: DbPool,
}

impl HotelBookTool {
    pub fn new(
        tp_client_id: String,
        tp_client_secret: String,
        stripe_key: String,
        customer_id: String,
        payment_method_id: String,
        db_pool: DbPool,
    ) -> Self {
        Self {
            tp_client_id,
            tp_client_secret,
            stripe_key,
            customer_id,
            payment_method_id,
            http_client: reqwest::Client::new(),
            db_pool,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HotelBookArgs {
    /// Hotel property ID from hotel_search results.
    pub hotel_id: String,
    /// Human-readable hotel name (for the confirmation message).
    pub hotel_name: String,
    /// Check-in date in YYYY-MM-DD format.
    pub check_in: String,
    /// Check-out date in YYYY-MM-DD format.
    pub check_out: String,
    /// Number of guests (default 1).
    pub guests: Option<u32>,
    /// Total stay cost in the currency's smallest unit (e.g. cents for USD/EUR).
    pub total_price_minor_units: u64,
    /// ISO 4217 currency code, lowercase (e.g. "usd", "eur").
    pub currency: String,
    /// Rate plan / room type ID from hotel_search, if available.
    pub rate_id: Option<String>,
}

#[derive(Serialize)]
pub struct HotelBookOutput {
    pub confirmation_number: String,
    pub hotel_name: String,
    pub check_in: String,
    pub check_out: String,
    pub total_charged_minor_units: u64,
    pub currency: String,
    /// Last 4 digits of the Issuing card used (for the user's receipt reference).
    pub card_last4: String,
    pub status: String,
}

#[derive(Deserialize)]
struct TravelportBookingResponse {
    // TODO: adjust field names to match actual Travelport+ booking response schema
    #[serde(alias = "confirmationNumber", alias = "ConfirmationNumber")]
    confirmation_number: Option<String>,
    #[serde(alias = "bookingReference", alias = "BookingReference")]
    booking_reference: Option<String>,
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
                "required": ["hotel_id", "hotel_name", "check_in", "check_out", "total_price_minor_units", "currency"],
                "properties": {
                    "hotel_id": {
                        "type": "string",
                        "description": "Property ID from hotel_search results."
                    },
                    "hotel_name": {
                        "type": "string",
                        "description": "Hotel name for the confirmation message."
                    },
                    "check_in": {
                        "type": "string",
                        "description": "Check-in date in YYYY-MM-DD format."
                    },
                    "check_out": {
                        "type": "string",
                        "description": "Check-out date in YYYY-MM-DD format."
                    },
                    "guests": {
                        "type": "integer",
                        "description": "Number of guests. Defaults to 1.",
                        "minimum": 1
                    },
                    "total_price_minor_units": {
                        "type": "integer",
                        "description": "Total stay cost in the currency's smallest unit (cents for USD/EUR). \
                            Multiply the displayed price by 100 (e.g. $150.00 → 15000)."
                    },
                    "currency": {
                        "type": "string",
                        "description": "ISO 4217 currency code, lowercase (e.g. \"usd\", \"eur\")."
                    },
                    "rate_id": {
                        "type": "string",
                        "description": "Rate plan or room type ID from hotel_search, if available."
                    }
                }
            }),
        }
    }

    #[instrument(skip(self), fields(
        hotel_id = %args.hotel_id,
        check_in = %args.check_in,
        check_out = %args.check_out,
        currency = %args.currency
    ))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate dates
        NaiveDate::parse_from_str(&args.check_in, "%Y-%m-%d").map_err(|e| {
            TravelportError::InvalidArg(format!("check_in not a valid date: {e}"))
        })?;
        NaiveDate::parse_from_str(&args.check_out, "%Y-%m-%d").map_err(|e| {
            TravelportError::InvalidArg(format!("check_out not a valid date: {e}"))
        })?;

        info!(
            hotel_id = %args.hotel_id,
            total_price_minor_units = args.total_price_minor_units,
            currency = %args.currency,
            "booking hotel via Travelport+ with Stripe Issuing card"
        );

        // Step 1: Issue a single-use Stripe virtual card for the booking amount.
        let payment_client = PaymentClient::new(self.stripe_key.clone());
        let issuing_card = payment_client
            .create_booking_card(
                &self.customer_id,
                &self.payment_method_id,
                args.total_price_minor_units,
                &args.currency,
            )
            .await
            .map_err(|e| {
                error!("Stripe Issuing card creation failed: {e:#}");
                TravelportError::InvalidArg(format!("Payment setup failed: {e}"))
            })?;

        let card_id = issuing_card.id.clone();
        let card_last4 = issuing_card.pan.chars().rev().take(4).collect::<Vec<_>>()
            .into_iter().rev().collect::<String>();

        // Audit-log the new Issuing card. Best-effort: the financial reality
        // is in Stripe; a missed DB write must never block the booking. We
        // log via warn! and continue.
        if let Err(e) = log_issuing_card_creation(
            &self.db_pool,
            &self.payment_method_id,
            &card_id,
            args.total_price_minor_units as i64,
            &args.currency,
        )
        .await
        {
            warn!("failed to record issuing_card_log entry for {card_id}: {e:#}");
        }

        // Step 2: Fetch a Travelport+ bearer token.
        let token =
            fetch_access_token(&self.tp_client_id, &self.tp_client_secret, &self.http_client)
                .await?;

        // Step 3: Submit the hotel booking with the Issuing card as Form of Payment.
        // TODO: adjust request body shape to match actual Travelport+ booking API schema.
        let exp_month = format!("{:02}", issuing_card.exp_month);
        let exp_year = issuing_card.exp_year.to_string();
        let request_body = serde_json::json!({
            "HotelReservation": {
                "HotelProperty": {
                    "HotelCode": args.hotel_id,
                },
                "HotelStay": {
                    "CheckinDate": args.check_in,
                    "CheckoutDate": args.check_out,
                },
                "NumberOfAdults": args.guests.unwrap_or(1),
                "RatePlanCode": args.rate_id,
                "FormOfPayment": {
                    "CreditCard": {
                        "Number": issuing_card.pan,
                        "ExpDate": format!("{exp_month}/{exp_year}"),
                        "CVV": issuing_card.cvv,
                        "Type": "VI",
                    }
                }
            }
        });

        let book_resp = self
            .http_client
            .post(HOTEL_BOOK_URL)
            .bearer_auth(&token)
            .json(&request_body)
            .send()
            .await?;

        let book_status = book_resp.status();
        let book_body = book_resp.text().await?;
        debug!("TravelPort hotel booking status: {book_status}");

        // Step 4: Cancel the Issuing card regardless of booking outcome.
        if let Err(e) = payment_client.cancel_issuing_card(&card_id).await {
            warn!("failed to cancel Issuing card {card_id}: {e:#}");
        }
        if let Err(e) = mark_issuing_card_cancelled(&self.db_pool, &card_id).await {
            warn!("failed to mark issuing_card_log row cancelled for {card_id}: {e:#}");
        }

        if !book_status.is_success() {
            return Err(make_api_error(book_status, book_body));
        }

        let booking: TravelportBookingResponse = serde_json::from_str(&book_body)
            .map_err(|e| TravelportError::Parse(format!("{e}: {book_body}")))?;

        let confirmation_number = booking
            .confirmation_number
            .or(booking.booking_reference)
            .unwrap_or_else(|| "PENDING".to_string());

        info!(
            confirmation_number = %confirmation_number,
            hotel = %args.hotel_name,
            "hotel booking confirmed"
        );

        Ok(HotelBookOutput {
            confirmation_number,
            hotel_name: args.hotel_name,
            check_in: args.check_in,
            check_out: args.check_out,
            total_charged_minor_units: args.total_price_minor_units,
            currency: args.currency,
            card_last4,
            status: "confirmed".to_string(),
        })
    }
}
