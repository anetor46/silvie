//! ORM layer for `payment_methods`, the linked billing-address row in
//! `addresses`, and the `issuing_card_log` audit table. Stripe API calls
//! live in [`crate::services::stripe`] — this module is DB only.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use diesel::{
    AsChangeset, ExpressionMethods, Insertable, OptionalExtension, QueryDsl, Queryable,
    Selectable, SelectableHelper,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{
    db::DbPool,
    schema::{addresses, issuing_card_log, payment_methods},
};

const BILLING_ADDRESS_TYPE: &str = "billing";

// ── Models ──────────────────────────────────────────────────────────────────

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

// ── Request / response shapes ───────────────────────────────────────────────

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

// ── Queries ─────────────────────────────────────────────────────────────────

/// Look up the user's primary payment method + its billing address (if linked).
#[instrument(skip(pool))]
pub async fn fetch_payment_method(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Option<PaymentMethodResponse>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let pm: Option<PaymentMethod> = payment_methods::table
        .filter(payment_methods::user_id.eq(user_id))
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
#[instrument(skip(pool, req))]
pub async fn upsert_payment_method(
    pool: &DbPool,
    user_id: Uuid,
    req: &CreatePaymentMethodRequest,
) -> Result<PaymentMethodResponse> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let existing_id: Option<Uuid> = payment_methods::table
        .filter(payment_methods::user_id.eq(user_id))
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
            user_id,
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
    fetch_payment_method(pool, user_id)
        .await?
        .ok_or_else(|| anyhow!("payment method missing after upsert"))
}

/// Soft-delete the user's primary payment method.
#[instrument(skip(pool))]
pub async fn soft_delete_payment_method(pool: &DbPool, user_id: Uuid) -> Result<bool> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let n: usize = diesel::update(
        payment_methods::table
            .filter(payment_methods::user_id.eq(user_id))
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
/// link it to their default payment method via `billing_address_id`. Returns
/// `Ok(None)` if the user has no default card yet.
#[instrument(skip(pool, req))]
pub async fn update_billing_address(
    pool: &DbPool,
    user_id: Uuid,
    req: &UpdateBillingRequest,
) -> Result<Option<PaymentMethodResponse>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let pm_id: Option<Uuid> = payment_methods::table
        .filter(payment_methods::user_id.eq(user_id))
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
            addresses::user_id.eq(user_id),
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
            .filter(addresses::user_id.eq(user_id))
            .filter(addresses::type_.eq(BILLING_ADDRESS_TYPE))
            .filter(addresses::deleted_at.is_null()),
    )
    .set((&form, addresses::updated_at.eq(diesel::dsl::now)))
    .execute(&mut conn)
    .await
    .context("Failed to update billing address")?;

    let addr_id: Uuid = addresses::table
        .filter(addresses::user_id.eq(user_id))
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
    fetch_payment_method(pool, user_id).await
}

// ── Issuing-card audit log ──────────────────────────────────────────────────
//
// Every Stripe Issuing virtual card we create gets a row here so we can
// reconcile against Stripe's dashboard later. The PAN/CVC are never stored —
// only the Stripe card ID, the spending limit, and the cancel timestamp.
// Best-effort: log failures NEVER prevent the booking — the financial reality
// is in Stripe, not our DB.

#[instrument(skip(pool), fields(stripe_card_id, stripe_pm_id_len = stripe_pm_id.len(), amount_minor_units, currency))]
pub async fn log_issuing_card_creation(
    pool: &DbPool,
    stripe_pm_id: &str,
    stripe_card_id: &str,
    amount_minor_units: i64,
    currency: &str,
) -> Result<()> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    // The payment-method lookup determines who owns this issuing card. If we
    // can't find it (e.g. the user removed the card between the chat request
    // and now), still record an orphan row so the operations team can
    // reconcile with Stripe — just leave user_id / payment_method_id NULL.
    let pm: Option<(Uuid, Uuid)> = payment_methods::table
        .filter(payment_methods::stripe_payment_method_id.eq(stripe_pm_id))
        .filter(payment_methods::deleted_at.is_null())
        .select((payment_methods::id, payment_methods::user_id))
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to look up payment method for issuing log")?;

    let (pm_id_opt, user_id_opt) = match pm {
        Some((pm_id, user_id)) => (Some(pm_id), Some(user_id)),
        None => (None, None),
    };

    let currency_uc = currency.to_uppercase();
    let n: usize = diesel::insert_into(issuing_card_log::table)
        .values((
            issuing_card_log::user_id.eq(user_id_opt),
            issuing_card_log::payment_method_id.eq(pm_id_opt),
            issuing_card_log::stripe_issuing_card_id.eq(stripe_card_id),
            issuing_card_log::amount_minor_units.eq(amount_minor_units),
            issuing_card_log::currency.eq(&currency_uc),
        ))
        .execute(&mut conn)
        .await
        .context("Failed to insert issuing_card_log row")?;
    info!(rows = n, "issuing card creation logged");
    Ok(())
}

#[instrument(skip(pool), fields(stripe_card_id))]
pub async fn mark_issuing_card_cancelled(pool: &DbPool, stripe_card_id: &str) -> Result<()> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(
        issuing_card_log::table
            .filter(issuing_card_log::stripe_issuing_card_id.eq(stripe_card_id)),
    )
    .set(issuing_card_log::cancelled_at.eq(diesel::dsl::now))
    .execute(&mut conn)
    .await
    .context("Failed to mark issuing_card_log cancelled")?;
    info!(rows = n, "issuing card cancellation logged");
    Ok(())
}
