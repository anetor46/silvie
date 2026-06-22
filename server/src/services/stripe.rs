//! Stripe HTTP client — wraps the bits of the Stripe API we use:
//! SetupIntent + Customer (card collection), PaymentMethod retrieval, and
//! Issuing virtual-card creation/cancellation for booking flows.
//!
//! Pure transport layer. No DB access. No request/response framework
//! coupling. The HTTP handlers in [`crate::api::payments`] and the
//! booking tools in [`crate::tools::travelport`] call into here.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

pub struct PaymentClient {
    pub stripe_key: String,
    http_client: reqwest::Client,
}

impl PaymentClient {
    pub fn new(stripe_key: String) -> Self {
        Self {
            stripe_key,
            http_client: reqwest::Client::new(),
        }
    }
}

// ── SetupIntent (card collection) ────────────────────────────────────────────

#[derive(Serialize)]
pub struct SetupIntentResponse {
    pub client_secret: String,
    pub customer_id: String,
}

#[derive(Deserialize)]
struct StripeCustomer {
    id: String,
}

#[derive(Deserialize)]
struct StripeSetupIntent {
    client_secret: String,
}

impl PaymentClient {
    /// Creates a Stripe Customer then a SetupIntent so the frontend can
    /// collect and tokenise a card via Stripe Elements.
    #[instrument(skip(self), fields(stripe_key_len = self.stripe_key.len()))]
    pub async fn create_setup_intent(&self) -> Result<SetupIntentResponse> {
        info!("creating Stripe customer");
        let cust_resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/customers"))
            .basic_auth(&self.stripe_key, Some(""))
            .send()
            .await?;

        let status = cust_resp.status();
        let body = cust_resp.text().await?;
        debug!("Stripe create customer status: {status}");
        if !status.is_success() {
            error!("Stripe create customer error ({status}): {body}");
            return Err(anyhow!("Stripe customer creation failed: HTTP {status}"));
        }
        let customer: StripeCustomer = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse Stripe customer: {e}"))?;

        info!("creating Stripe SetupIntent for customer {}", customer.id);
        let si_resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/setup_intents"))
            .basic_auth(&self.stripe_key, Some(""))
            .form(&[("customer", customer.id.as_str()), ("usage", "off_session")])
            .send()
            .await?;

        let status = si_resp.status();
        let body = si_resp.text().await?;
        debug!("Stripe SetupIntent status: {status}");
        if !status.is_success() {
            error!("Stripe SetupIntent error ({status}): {body}");
            return Err(anyhow!("Stripe SetupIntent creation failed: HTTP {status}"));
        }
        let si: StripeSetupIntent = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse Stripe SetupIntent: {e}"))?;

        Ok(SetupIntentResponse {
            client_secret: si.client_secret,
            customer_id: customer.id,
        })
    }
}

// ── PaymentMethod details ────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PaymentMethodDetails {
    pub last4: String,
    pub brand: String,
    pub exp_month: u32,
    pub exp_year: u32,
}

#[derive(Deserialize)]
struct StripePaymentMethod {
    card: StripeCard,
}

#[derive(Deserialize)]
struct StripeCard {
    last4: String,
    brand: String,
    exp_month: u32,
    exp_year: u32,
}

impl PaymentClient {
    /// Retrieves display-safe card metadata (last4, brand, expiry) from Stripe.
    /// Never returns the full card number.
    #[instrument(skip(self), fields(stripe_key_len = self.stripe_key.len(), payment_method_id))]
    pub async fn get_payment_method_details(
        &self,
        payment_method_id: &str,
    ) -> Result<PaymentMethodDetails> {
        let resp = self
            .http_client
            .get(format!("{STRIPE_API_BASE}/payment_methods/{payment_method_id}"))
            .basic_auth(&self.stripe_key, Some(""))
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        debug!("Stripe get payment method status: {status}");
        if !status.is_success() {
            error!("Stripe get payment method error ({status}): {body}");
            return Err(anyhow!("Stripe PM retrieval failed: HTTP {status}"));
        }

        let pm: StripePaymentMethod = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse payment method: {e}"))?;

        Ok(PaymentMethodDetails {
            last4: pm.card.last4,
            brand: pm.card.brand,
            exp_month: pm.card.exp_month,
            exp_year: pm.card.exp_year,
        })
    }
}

// ── Issuing (single-use virtual card) ────────────────────────────────────────

/// Card details returned by Stripe Issuing. Lives in memory only for the
/// duration of a booking request — never logged or persisted.
pub struct IssuingCard {
    pub id: String,
    pub pan: String,
    pub exp_month: u32,
    pub exp_year: u32,
    pub cvv: String,
}

#[derive(Deserialize)]
struct StripeCardholder {
    id: String,
}

#[derive(Deserialize)]
struct StripeIssuingCard {
    id: String,
    number: Option<String>,
    exp_month: u32,
    exp_year: u32,
    cvc: Option<String>,
}

impl PaymentClient {
    /// Creates a single-use Stripe Issuing virtual card capped at `amount_cents`.
    /// The card's PAN and CVV are returned in memory only.
    ///
    /// In test mode (`sk_test_…`), cardholders and cards are created immediately
    /// with no approval. In production, Stripe Issuing access must be enabled on
    /// your Stripe account first.
    ///
    /// TODO(production): replace the hardcoded placeholder cardholder data with
    /// real user name and billing address from the user's profile.
    #[instrument(skip(self, customer_id, _payment_method_id), fields(
        stripe_key_len = self.stripe_key.len(),
        amount_cents,
        currency
    ))]
    pub async fn create_booking_card(
        &self,
        customer_id: &str,
        // Reserved for charging the customer's stored PM via PaymentIntent before
        // the booking is submitted (full charge flow to be added in a follow-up).
        _payment_method_id: &str,
        amount_cents: u64,
        currency: &str,
    ) -> Result<IssuingCard> {
        // Step 1: Create a cardholder.
        // Production: persist cardholder_id per customer and reuse.
        info!("creating Stripe Issuing cardholder for customer {customer_id}");
        let terms_date = "1640995200"; // 2022-01-01 — placeholder for terms acceptance
        let ch_resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/issuing/cardholders"))
            .basic_auth(&self.stripe_key, Some(""))
            .form(&[
                ("name", "Silvie Traveler"),
                ("type", "individual"),
                ("email", "traveler@silvie.app"),
                ("billing[address][line1]", "123 Main St"),
                ("billing[address][city]", "San Francisco"),
                ("billing[address][state]", "CA"),
                ("billing[address][postal_code]", "94105"),
                ("billing[address][country]", "US"),
                (
                    "individual[card_issuing][user_terms_acceptance][date]",
                    terms_date,
                ),
                (
                    "individual[card_issuing][user_terms_acceptance][ip]",
                    "127.0.0.1",
                ),
            ])
            .send()
            .await?;

        let status = ch_resp.status();
        let body = ch_resp.text().await?;
        debug!("Stripe cardholder status: {status}");
        if !status.is_success() {
            error!("Stripe cardholder creation error ({status}): {body}");
            return Err(anyhow!("Stripe cardholder creation failed: HTTP {status}"));
        }
        let cardholder: StripeCardholder = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse cardholder: {e}"))?;

        // Step 2: Issue a virtual card with a spending limit = the booking total.
        info!("issuing Stripe virtual card (amount={amount_cents} {currency})");
        let amount_str = amount_cents.to_string();
        let card_resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/issuing/cards"))
            .basic_auth(&self.stripe_key, Some(""))
            .form(&[
                ("type", "virtual"),
                ("cardholder", cardholder.id.as_str()),
                ("currency", currency),
                (
                    "spending_controls[spending_limits][0][amount]",
                    amount_str.as_str(),
                ),
                (
                    "spending_controls[spending_limits][0][interval]",
                    "all_time",
                ),
                ("expand[]", "number"),
                ("expand[]", "cvc"),
            ])
            .send()
            .await?;

        let status = card_resp.status();
        let body = card_resp.text().await?;
        debug!("Stripe Issuing card status: {status}");
        if !status.is_success() {
            error!("Stripe Issuing card error ({status}): {body}");
            return Err(anyhow!("Stripe Issuing card creation failed: HTTP {status}"));
        }

        let card: StripeIssuingCard = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse issuing card: {e}"))?;

        let pan = card
            .number
            .ok_or_else(|| anyhow!("Stripe did not expand card number — check expand[]=number"))?;
        let cvv = card
            .cvc
            .ok_or_else(|| anyhow!("Stripe did not expand CVC — check expand[]=cvc"))?;

        Ok(IssuingCard {
            id: card.id,
            pan,
            exp_month: card.exp_month,
            exp_year: card.exp_year,
            cvv,
        })
    }

    /// Cancels a Stripe Issuing card after use, ensuring single-use semantics.
    /// Always called after a booking attempt, whether it succeeded or failed.
    #[instrument(skip(self), fields(stripe_key_len = self.stripe_key.len(), card_id))]
    pub async fn cancel_issuing_card(&self, card_id: &str) -> Result<()> {
        let resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/issuing/cards/{card_id}"))
            .basic_auth(&self.stripe_key, Some(""))
            .form(&[("status", "canceled")])
            .send()
            .await?;

        let status = resp.status();
        debug!("cancel Issuing card status: {status}");
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            error!("Stripe cancel card error ({status}): {body}");
            return Err(anyhow!("Failed to cancel Issuing card: HTTP {status}"));
        }
        info!("Stripe Issuing card {card_id} cancelled");
        Ok(())
    }
}

// ── PaymentIntent (customer-PM pre-charge) ───────────────────────────────────
//
// We use a manual-capture PaymentIntent on the user's saved PM as a hold
// against the booking amount. The Issuing virtual card is what actually pays
// the supplier — the PaymentIntent moves money from the user's card to our
// Stripe balance. Order is: create intent (hold) → book → capture-or-cancel.

#[derive(Deserialize, Debug)]
pub struct StripePaymentIntent {
    pub id: String,
    pub status: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub amount: Option<i64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub currency: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct StripeRefund {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub amount: Option<i64>,
}

impl PaymentClient {
    /// Create a manual-capture PaymentIntent against the customer's saved
    /// payment method and confirm it immediately. On success the intent is in
    /// `requires_capture` — funds are held. Use [`capture_intent`] to charge,
    /// or [`cancel_intent`] to release.
    ///
    /// Caller surfaces `requires_action` / 3DS as a clean error — we do not
    /// currently flow the SCA challenge through chat.
    #[instrument(skip(self), fields(
        stripe_key_len = self.stripe_key.len(),
        customer_id,
        amount_minor_units,
        currency
    ))]
    pub async fn create_and_confirm_intent(
        &self,
        customer_id: &str,
        payment_method_id: &str,
        amount_minor_units: u64,
        currency: &str,
        metadata: &[(&str, &str)],
    ) -> Result<StripePaymentIntent> {
        let amount_str = amount_minor_units.to_string();
        let mut form: Vec<(String, String)> = vec![
            ("amount".into(), amount_str),
            ("currency".into(), currency.to_string()),
            ("customer".into(), customer_id.to_string()),
            ("payment_method".into(), payment_method_id.to_string()),
            ("payment_method_types[]".into(), "card".into()),
            ("capture_method".into(), "manual".into()),
            ("off_session".into(), "true".into()),
            ("confirm".into(), "true".into()),
        ];
        for (k, v) in metadata {
            form.push((format!("metadata[{k}]"), (*v).to_string()));
        }

        let resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/payment_intents"))
            .basic_auth(&self.stripe_key, Some(""))
            .form(&form)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        debug!("Stripe PaymentIntent create status: {status}");
        if !status.is_success() {
            error!("Stripe PaymentIntent error ({status}): {body}");
            return Err(anyhow!("Stripe PaymentIntent failed: HTTP {status}: {body}"));
        }

        let intent: StripePaymentIntent = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse PaymentIntent: {e}"))?;

        if intent.status != "requires_capture" {
            return Err(anyhow!(
                "PaymentIntent {} ended in status '{}' — booking requires an authorisation hold",
                intent.id,
                intent.status
            ));
        }
        info!(intent_id = %intent.id, "PaymentIntent authorised (held)");
        Ok(intent)
    }

    /// Capture a previously-authorised PaymentIntent. Charges the customer
    /// for the booking amount.
    #[instrument(skip(self), fields(stripe_key_len = self.stripe_key.len(), intent_id))]
    pub async fn capture_intent(&self, intent_id: &str) -> Result<StripePaymentIntent> {
        let resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/payment_intents/{intent_id}/capture"))
            .basic_auth(&self.stripe_key, Some(""))
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        debug!("Stripe PaymentIntent capture status: {status}");
        if !status.is_success() {
            error!("Stripe capture error ({status}): {body}");
            return Err(anyhow!("Stripe capture failed: HTTP {status}: {body}"));
        }
        let intent: StripePaymentIntent = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse captured PaymentIntent: {e}"))?;
        info!(intent_id = %intent.id, status = %intent.status, "PaymentIntent captured");
        Ok(intent)
    }

    /// Cancel an authorised but uncaptured PaymentIntent. Releases the hold.
    #[instrument(skip(self), fields(stripe_key_len = self.stripe_key.len(), intent_id))]
    pub async fn cancel_intent(&self, intent_id: &str) -> Result<StripePaymentIntent> {
        let resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/payment_intents/{intent_id}/cancel"))
            .basic_auth(&self.stripe_key, Some(""))
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        debug!("Stripe PaymentIntent cancel status: {status}");
        if !status.is_success() {
            error!("Stripe cancel-intent error ({status}): {body}");
            return Err(anyhow!("Stripe cancel-intent failed: HTTP {status}: {body}"));
        }
        let intent: StripePaymentIntent = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse cancelled PaymentIntent: {e}"))?;
        info!(intent_id = %intent.id, "PaymentIntent cancelled (hold released)");
        Ok(intent)
    }

    /// Refund (part of) a captured PaymentIntent. Used during hotel
    /// cancellations when the supplier policy allows.
    #[instrument(skip(self), fields(stripe_key_len = self.stripe_key.len(), intent_id, amount_minor_units))]
    pub async fn refund_intent(
        &self,
        intent_id: &str,
        amount_minor_units: u64,
    ) -> Result<StripeRefund> {
        let amount_str = amount_minor_units.to_string();
        let resp = self
            .http_client
            .post(format!("{STRIPE_API_BASE}/refunds"))
            .basic_auth(&self.stripe_key, Some(""))
            .form(&[
                ("payment_intent", intent_id),
                ("amount", amount_str.as_str()),
            ])
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        debug!("Stripe refund status: {status}");
        if !status.is_success() {
            error!("Stripe refund error ({status}): {body}");
            return Err(anyhow!("Stripe refund failed: HTTP {status}: {body}"));
        }
        let refund: StripeRefund = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse refund: {e}"))?;
        info!(refund_id = %refund.id, status = %refund.status, "refund created");
        Ok(refund)
    }
}
