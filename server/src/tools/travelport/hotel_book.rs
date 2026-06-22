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
use super::models::{
    BookReq, BuildFromCatalogOfferingHospitality, EmailValue, FormOfPaymentPaymentCard,
    MoneyAmountOut, Payment, PaymentCardDetail, PersonName, PlainTextField, ReservationBuild,
    ReservationQueryBuild, ReservationResp, Traveler, Value,
};

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
    /// Composite property id from `hotel_search` ("chainCode-propertyCode").
    pub property_id: String,
    /// CatalogOffering identifier from `hotel_availability.rates[].offer_id`.
    pub offer_id: String,
    /// `bookingCode` from `hotel_availability.rates[].rate_id`.
    pub rate_id: String,
    pub hotel_name: String,
    pub check_in: String,
    pub check_out: String,
    pub guests: u32,
    /// Lead guest first name (Traveler.PersonName.Given).
    pub guest_given_name: String,
    /// Lead guest surname (Traveler.PersonName.Surname).
    pub guest_surname: String,
    pub guest_email: Option<String>,
    /// Stay total in the currency's smallest unit (cents for USD/EUR/GBP).
    pub total_price_minor_units: u64,
    pub currency: String,
}

#[derive(Serialize)]
pub struct HotelBookOutput {
    pub booking_id: Uuid,
    /// AggregatorLocatorCode — what we feed into retrieve/cancel.
    pub reservation_id: String,
    /// Supplier-side locator (sourceContext=Supplier in the Receipt).
    pub supplier_locator: Option<String>,
    pub hotel_name: String,
    pub check_in: String,
    pub check_out: String,
    pub total_charged_minor_units: u64,
    pub currency: String,
    pub status: String,
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
                    "check_in", "check_out", "guests",
                    "guest_given_name", "guest_surname",
                    "total_price_minor_units", "currency"
                ],
                "properties": {
                    "property_id":       { "type": "string", "description": "Composite property id from hotel_search." },
                    "offer_id":          { "type": "string", "description": "CatalogOffering identifier (Identifier.value) from hotel_availability." },
                    "rate_id":           { "type": "string", "description": "bookingCode from hotel_availability." },
                    "hotel_name":        { "type": "string" },
                    "check_in":          { "type": "string", "description": "YYYY-MM-DD" },
                    "check_out":         { "type": "string", "description": "YYYY-MM-DD" },
                    "guests":            { "type": "integer", "minimum": 1 },
                    "guest_given_name":  { "type": "string", "description": "Lead guest first name." },
                    "guest_surname":     { "type": "string", "description": "Lead guest surname." },
                    "guest_email":       { "type": "string" },
                    "total_price_minor_units": { "type": "integer", "minimum": 1 },
                    "currency":          { "type": "string", "enum": ["USD", "EUR", "GBP"] }
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

        // Step 1: persist a pending row so we own the id immediately.
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

        // Step 2: authorise the user's stored PM for the booking amount.
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

        // Step 3: issue the virtual card.
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

        // Step 4: submit the Travelport reservation (reference payload).
        let total_major = (args.total_price_minor_units as f64) / 100.0;
        let card_code = travelport_card_code(card.pan.as_str());
        let expire = format!("{:02}{:02}", card.exp_month, card.exp_year % 100);
        let card_holder = format!("{} {}", args.guest_given_name, args.guest_surname);

        let email_vec = args
            .guest_email
            .as_deref()
            .map(|e| vec![EmailValue { value: e.to_string() }])
            .unwrap_or_default();

        let req = BookReq {
            query: ReservationQueryBuild {
                type_: "ReservationQueryBuild",
                reservation_build: ReservationBuild {
                    type_: "ReservationBuildFromCatalogOffering",
                    build_from: BuildFromCatalogOfferingHospitality {
                        type_: "BuildFromCatalogOfferingHospitality",
                        catalog_offering_identifier: Value {
                            value: args.offer_id.clone(),
                        },
                    },
                    traveler: vec![Traveler {
                        type_: "Traveler",
                        person_name: PersonName {
                            given: args.guest_given_name.clone(),
                            surname: args.guest_surname.clone(),
                        },
                        email: email_vec,
                    }],
                    form_of_payment: vec![FormOfPaymentPaymentCard {
                        type_: "FormOfPaymentPaymentCard",
                        payment_card: PaymentCardDetail {
                            type_: "PaymentCardDetail",
                            expire_date: expire,
                            card_type: "Credit",
                            card_code,
                            card_holder_name: card_holder,
                            card_number: PlainTextField {
                                type_: "CardNumber",
                                plain_text: card.pan.clone(),
                            },
                            series_code: PlainTextField {
                                type_: "SeriesCode",
                                plain_text: card.cvv.clone(),
                            },
                        },
                    }],
                    payment: vec![Payment {
                        type_: "Payment",
                        amount: MoneyAmountOut {
                            code: currency.clone(),
                            value: total_major,
                        },
                        guarantee_ind: true,
                        deposit_ind: false,
                    }],
                },
            },
        };
        let book_outcome = self.travelport.book(req).await;

        // Step 5: cancel the issuing card regardless of outcome.
        if let Err(e) = payment_client.cancel_issuing_card(&card_id).await {
            warn!("failed to cancel Issuing card {card_id}: {e:#}");
        }
        if let Err(e) = mark_issuing_card_cancelled(&self.db_pool, &card_id).await {
            warn!("failed to mark issuing_card_log row cancelled for {card_id}: {e:#}");
        }

        match book_outcome {
            Ok(resp) => {
                let (aggregator, supplier) = extract_locators(&resp);
                let aggregator = aggregator.ok_or_else(|| {
                    TravelportError::Parse(
                        "booking response missing aggregator (Travelport) locator".into(),
                    )
                })?;

                if let Err(e) = payment_client.capture_intent(&intent.id).await {
                    warn!("Travelport booking succeeded but PaymentIntent capture failed: {e:#}");
                    let reason = format!(
                        "booking confirmed at supplier ({aggregator}) but payment capture failed: {e}"
                    );
                    let _ = mark_failed(&self.db_pool, booking_id, &reason).await;
                    return Err(TravelportError::InvalidArg(reason));
                }

                if let Err(e) = mark_confirmed(
                    &self.db_pool,
                    booking_id,
                    &aggregator,
                    supplier.as_deref(),
                    None,
                )
                .await
                {
                    warn!("booking confirmed at Travelport but DB update failed: {e:#}");
                }

                info!(
                    booking_id = %booking_id,
                    aggregator_locator = %aggregator,
                    supplier_locator = ?supplier,
                    "hotel booking confirmed"
                );

                let status = resp
                    .reservation_response
                    .as_ref()
                    .and_then(|r| r.receipt.first())
                    .and_then(|r| r.confirmation.as_ref())
                    .and_then(|c| c.offer_status.as_ref())
                    .and_then(|s| s.status.clone())
                    .unwrap_or_else(|| "Confirmed".into());

                Ok(HotelBookOutput {
                    booking_id,
                    reservation_id: aggregator,
                    supplier_locator: supplier,
                    hotel_name: args.hotel_name,
                    check_in: args.check_in,
                    check_out: args.check_out,
                    total_charged_minor_units: args.total_price_minor_units,
                    currency,
                    status,
                })
            }
            Err(e) => {
                if let Err(cancel_err) = payment_client.cancel_intent(&intent.id).await {
                    warn!(
                        "failed to release PaymentIntent {} after booking failure: {cancel_err:#}",
                        intent.id
                    );
                }
                let reason = format!("Travelport booking failed: {e}");
                let _ = mark_failed(&self.db_pool, booking_id, &reason).await;
                Err(e)
            }
        }
    }
}

/// Pull (aggregator, supplier) locators out of a Travelport reservation
/// response. The same response shape is returned by Book, Retrieve and Cancel,
/// with each receipt's `sourceContext` saying which one it is.
fn extract_locators(resp: &ReservationResp) -> (Option<String>, Option<String>) {
    let Some(inner) = resp.reservation_response.as_ref() else {
        return (None, None);
    };
    let mut aggregator = None;
    let mut supplier = None;
    for receipt in &inner.receipt {
        let Some(conf) = receipt.confirmation.as_ref() else {
            continue;
        };
        let Some(loc) = conf.locator.as_ref() else {
            continue;
        };
        let Some(value) = loc.value.as_ref() else {
            continue;
        };
        match loc.source_context.as_deref() {
            Some("Travelport") => {
                if aggregator.is_none() {
                    aggregator = Some(value.clone());
                }
            }
            Some("Supplier") => {
                if supplier.is_none() {
                    supplier = Some(value.clone());
                }
            }
            _ => {}
        }
    }
    (aggregator, supplier)
}

/// Map the Stripe-Issuing card brand to Travelport's two-letter card code.
fn travelport_card_code(pan: &str) -> String {
    let first = pan.chars().next().unwrap_or('0');
    match first {
        '4' => "VI".into(),     // Visa
        '5' | '2' => "CA".into(), // Mastercard
        '3' => "AX".into(),     // American Express
        '6' => "DS".into(),     // Discover
        _ => "VI".into(),
    }
}
