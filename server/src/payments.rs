use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use diesel::{
    AsChangeset, ExpressionMethods, Insertable, OptionalExtension, QueryDsl, Queryable,
    Selectable, SelectableHelper,
};
use diesel_async::RunQueryDsl;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

use crate::{
    auth::Principal,
    db::DbPool,
    schema::{addresses, payment_methods},
    users,
};

const BILLING_ADDRESS_TYPE: &str = "billing";

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

// ── Client ────────────────────────────────────────────────────────────────────

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

// ── Phase 1: SetupIntent (card collection) ────────────────────────────────────

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

// ── Phase 1: PaymentMethod details ────────────────────────────────────────────

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

// ── Phase 2: Stripe Issuing (virtual card for GDS booking) ───────────────────

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

// ── HTTP handlers ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetPaymentMethodRequest {
    // Kept for future validation that the PM belongs to this customer.
    #[allow(dead_code)]
    customer_id: String,
    payment_method_id: String,
}

/// POST /payment/setup — creates a Stripe Customer + SetupIntent.
/// The frontend uses the returned `client_secret` with Stripe Elements to
/// collect the card without it ever touching this server.
#[handler]
pub async fn payment_setup_handler(
    Data(client): Data<&Arc<Option<PaymentClient>>>,
) -> poem::Result<Json<SetupIntentResponse>> {
    // `**client` dereferences &Arc<Option<…>> to get &Option<PaymentClient> as a place,
    // then .as_ref() converts that to Option<&PaymentClient> without moving.
    let client = (**client).as_ref().ok_or_else(|| {
        poem::Error::from_status(StatusCode::SERVICE_UNAVAILABLE)
    })?;

    client.create_setup_intent().await.map(Json).map_err(|e| {
        error!("payment setup failed: {e:#}");
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })
}

/// POST /payment/method — retrieves display-safe card metadata (last4, brand,
/// expiry) for a PaymentMethod that the frontend just confirmed via Stripe Elements.
#[handler]
pub async fn payment_method_handler(
    Data(client): Data<&Arc<Option<PaymentClient>>>,
    Json(req): Json<GetPaymentMethodRequest>,
) -> poem::Result<Json<PaymentMethodDetails>> {
    let client = (**client).as_ref().ok_or_else(|| {
        poem::Error::from_status(StatusCode::SERVICE_UNAVAILABLE)
    })?;

    client
        .get_payment_method_details(&req.payment_method_id)
        .await
        .map(Json)
        .map_err(|e| {
            error!("get payment method failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })
}

// ── DB-backed user payment-method storage ───────────────────────────────────
//
// Source of truth for "which card does this user have saved" — replaces the
// keychain plumbing we used to ship. The Stripe IDs + display metadata live
// in `payment_methods`; the billing address (if set) lives in `addresses`
// with type='billing' and is referenced by `payment_methods.billing_address_id`.
//
// Single-card-per-user is the current UI assumption; the table is N:1 to make
// multi-card support a UI-only change later.

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = payment_methods)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PaymentMethod {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub stripe_customer_id: String,
    pub stripe_payment_method_id: String,
    pub last4: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<i16>,
    pub exp_year: Option<i16>,
    pub label: Option<String>,
    pub is_default: bool,
    pub billing_address_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = addresses)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BillingAddress {
    pub id: Uuid,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = payment_methods)]
struct NewPaymentMethod<'a> {
    user_id: Uuid,
    stripe_customer_id: &'a str,
    stripe_payment_method_id: &'a str,
    last4: Option<&'a str>,
    brand: Option<&'a str>,
    exp_month: Option<i16>,
    exp_year: Option<i16>,
    is_default: bool,
}

#[derive(AsChangeset)]
#[diesel(table_name = payment_methods)]
struct PaymentMethodChanges<'a> {
    stripe_customer_id: &'a str,
    stripe_payment_method_id: &'a str,
    last4: Option<&'a str>,
    brand: Option<&'a str>,
    exp_month: Option<i16>,
    exp_year: Option<i16>,
}

#[derive(Insertable, AsChangeset, Default)]
#[diesel(table_name = addresses)]
struct BillingAddressForm<'a> {
    line1: Option<&'a str>,
    line2: Option<&'a str>,
    city: Option<&'a str>,
    state: Option<&'a str>,
    postal_code: Option<&'a str>,
    country: Option<&'a str>,
}

#[derive(Serialize)]
pub struct PaymentMethodResponse {
    pub payment_method: PaymentMethod,
    pub billing_address: Option<BillingAddress>,
}

#[derive(Deserialize)]
pub struct CreatePaymentMethodRequest {
    pub stripe_customer_id: String,
    pub stripe_payment_method_id: String,
    pub last4: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<i16>,
    pub exp_year: Option<i16>,
}

#[derive(Deserialize)]
pub struct UpdateBillingRequest {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// Look up the user's primary payment method + its billing address (if linked).
#[instrument(skip(pool), fields(sub_len = sub.len()))]
pub async fn fetch_payment_method(
    pool: &DbPool,
    sub: &str,
) -> Result<Option<PaymentMethodResponse>> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(None),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let pm: Option<PaymentMethod> = payment_methods::table
        .filter(payment_methods::user_id.eq(user.id))
        .filter(payment_methods::is_default.eq(true))
        .filter(payment_methods::deleted_at.is_null())
        .select(PaymentMethod::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to query payment_methods")?;

    let Some(pm) = pm else {
        return Ok(None);
    };

    let billing = if let Some(addr_id) = pm.billing_address_id {
        addresses::table
            .filter(addresses::id.eq(addr_id))
            .filter(addresses::deleted_at.is_null())
            .select(BillingAddress::as_select())
            .first(&mut conn)
            .await
            .optional()
            .context("Failed to query billing address")?
    } else {
        None
    };

    Ok(Some(PaymentMethodResponse {
        payment_method: pm,
        billing_address: billing,
    }))
}

/// Find-or-update the user's default payment method. Preserves
/// `billing_address_id` across card replacements (so changing card doesn't
/// orphan the saved billing address).
#[instrument(skip(pool, req), fields(sub_len = sub.len()))]
pub async fn upsert_payment_method(
    pool: &DbPool,
    sub: &str,
    req: &CreatePaymentMethodRequest,
) -> Result<Option<PaymentMethodResponse>> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(None),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let existing_id: Option<Uuid> = payment_methods::table
        .filter(payment_methods::user_id.eq(user.id))
        .filter(payment_methods::is_default.eq(true))
        .filter(payment_methods::deleted_at.is_null())
        .select(payment_methods::id)
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to look up existing payment method")?;

    if let Some(existing_id) = existing_id {
        let changes = PaymentMethodChanges {
            stripe_customer_id: &req.stripe_customer_id,
            stripe_payment_method_id: &req.stripe_payment_method_id,
            last4: req.last4.as_deref(),
            brand: req.brand.as_deref(),
            exp_month: req.exp_month,
            exp_year: req.exp_year,
        };
        let n: usize = diesel::update(
            payment_methods::table
                .filter(payment_methods::id.eq(existing_id))
                .filter(payment_methods::deleted_at.is_null()),
        )
        .set((&changes, payment_methods::updated_at.eq(diesel::dsl::now)))
        .execute(&mut conn)
        .await
        .context("Failed to update payment method")?;
        info!(updated = n, "payment method updated");
    } else {
        let row = NewPaymentMethod {
            user_id: user.id,
            stripe_customer_id: &req.stripe_customer_id,
            stripe_payment_method_id: &req.stripe_payment_method_id,
            last4: req.last4.as_deref(),
            brand: req.brand.as_deref(),
            exp_month: req.exp_month,
            exp_year: req.exp_year,
            is_default: true,
        };
        let n: usize = diesel::insert_into(payment_methods::table)
            .values(&row)
            .execute(&mut conn)
            .await
            .context("Failed to insert payment method")?;
        info!(inserted = n, "payment method created");
    }

    drop(conn);
    fetch_payment_method(pool, sub).await
}

/// Soft-delete the user's primary payment method.
#[instrument(skip(pool), fields(sub_len = sub.len()))]
pub async fn soft_delete_payment_method(pool: &DbPool, sub: &str) -> Result<bool> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(false),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let n: usize = diesel::update(
        payment_methods::table
            .filter(payment_methods::user_id.eq(user.id))
            .filter(payment_methods::is_default.eq(true))
            .filter(payment_methods::deleted_at.is_null()),
    )
    .set((
        payment_methods::deleted_at.eq(diesel::dsl::now),
        payment_methods::updated_at.eq(diesel::dsl::now),
    ))
    .execute(&mut conn)
    .await
    .context("Failed to soft-delete payment method")?;
    info!(rows = n, "payment method soft-deleted");
    Ok(n > 0)
}

/// Upsert the user's billing address (in `addresses` with type='billing') and
/// link it to their default payment method via `billing_address_id`. The card
/// must already exist; returns Ok(None) if the user has no default card yet.
#[instrument(skip(pool, req), fields(sub_len = sub.len()))]
pub async fn update_billing_address(
    pool: &DbPool,
    sub: &str,
    req: &UpdateBillingRequest,
) -> Result<Option<PaymentMethodResponse>> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(None),
    };
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let pm_id: Option<Uuid> = payment_methods::table
        .filter(payment_methods::user_id.eq(user.id))
        .filter(payment_methods::is_default.eq(true))
        .filter(payment_methods::deleted_at.is_null())
        .select(payment_methods::id)
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to look up payment method")?;

    let Some(pm_id) = pm_id else {
        return Ok(None);
    };

    let form = BillingAddressForm {
        line1: req.line1.as_deref(),
        line2: req.line2.as_deref(),
        city: req.city.as_deref(),
        state: req.state.as_deref(),
        postal_code: req.postal_code.as_deref(),
        country: req.country.as_deref(),
    };

    // INSERT (skipped on partial-unique conflict) + UPDATE — same idempotent
    // pattern user_info.rs uses for the home address row.
    let _: usize = diesel::insert_into(addresses::table)
        .values((
            addresses::user_id.eq(user.id),
            addresses::type_.eq(BILLING_ADDRESS_TYPE),
            addresses::is_default.eq(true),
            &form,
        ))
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await
        .context("Failed to insert billing address")?;
    let _: usize = diesel::update(
        addresses::table
            .filter(addresses::user_id.eq(user.id))
            .filter(addresses::type_.eq(BILLING_ADDRESS_TYPE))
            .filter(addresses::deleted_at.is_null()),
    )
    .set((&form, addresses::updated_at.eq(diesel::dsl::now)))
    .execute(&mut conn)
    .await
    .context("Failed to update billing address")?;

    // Re-link the payment method to the (possibly newly created) billing row.
    let addr_id: Uuid = addresses::table
        .filter(addresses::user_id.eq(user.id))
        .filter(addresses::type_.eq(BILLING_ADDRESS_TYPE))
        .filter(addresses::deleted_at.is_null())
        .select(addresses::id)
        .first(&mut conn)
        .await
        .context("Failed to look up billing address id")?;

    let _: usize = diesel::update(
        payment_methods::table
            .filter(payment_methods::id.eq(pm_id))
            .filter(payment_methods::deleted_at.is_null()),
    )
    .set((
        payment_methods::billing_address_id.eq(addr_id),
        payment_methods::updated_at.eq(diesel::dsl::now),
    ))
    .execute(&mut conn)
    .await
    .context("Failed to link billing address to payment method")?;

    drop(conn);
    fetch_payment_method(pool, sub).await
}

// ── HTTP handlers (DB-backed) ───────────────────────────────────────────────

/// GET /users/me/payment-method — fetch the user's primary card + billing.
/// Returns 404 if none.
#[handler]
pub async fn get_user_payment_method_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
) -> poem::Result<Json<PaymentMethodResponse>> {
    let row = fetch_payment_method(pool, &principal.sub)
        .await
        .map_err(|e| {
            error!("payment_method fetch failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    row.map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}

/// POST /users/me/payment-method — find-or-update the user's primary card
/// (called from the frontend after `stripe.confirmSetup` succeeds).
#[handler]
pub async fn create_user_payment_method_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
    Json(req): Json<CreatePaymentMethodRequest>,
) -> poem::Result<Json<PaymentMethodResponse>> {
    let row = upsert_payment_method(pool, &principal.sub, &req)
        .await
        .map_err(|e| {
            error!("payment_method upsert failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    row.map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}

/// DELETE /users/me/payment-method — soft-delete the user's primary card.
#[handler]
pub async fn delete_user_payment_method_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
) -> poem::Result<StatusCode> {
    let removed = soft_delete_payment_method(pool, &principal.sub)
        .await
        .map_err(|e| {
            error!("payment_method delete failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    Ok(if removed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    })
}

/// PUT /users/me/payment-method/billing — upsert billing address + link it
/// to the default payment method. 404 if the user has no card yet.
#[handler]
pub async fn update_user_billing_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
    Json(req): Json<UpdateBillingRequest>,
) -> poem::Result<Json<PaymentMethodResponse>> {
    let row = update_billing_address(pool, &principal.sub, &req)
        .await
        .map_err(|e| {
            error!("billing address update failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    row.map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}

