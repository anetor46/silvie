//! ORM layer for `hotel_bookings`. Owns all SQL for hotel-booking
//! persistence — the LLM booking tools call into here to insert a pending
//! row before talking to Travelport, then transition the row to confirmed /
//! failed / cancelled.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use diesel::{
    ExpressionMethods, Insertable, OptionalExtension, QueryDsl, Queryable, Selectable,
    SelectableHelper,
};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{db::DbPool, schema::hotel_bookings};

pub const ENTITY_TYPE: &str = "hotel_booking";

// ── Models ──────────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = hotel_bookings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct HotelBooking {
    pub id: Uuid,
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub travelport_reservation_id: Option<String>,
    pub travelport_supplier_locator: Option<String>,
    pub travelport_property_id: String,
    pub travelport_offer_id: Option<String>,
    pub hotel_name: String,
    pub check_in: NaiveDate,
    pub check_out: NaiveDate,
    pub guests: i32,
    pub rooms: i32,
    pub total_amount_minor_units: i64,
    pub currency: String,
    pub cancellation_policy: Option<serde_json::Value>,
    pub status: String,
    pub failure_reason: Option<String>,
    pub payment_method_id: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub refunded_amount_minor_units: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = hotel_bookings)]
struct NewHotelBooking<'a> {
    user_id: Uuid,
    conversation_id: Option<Uuid>,
    travelport_property_id: &'a str,
    travelport_offer_id: Option<&'a str>,
    hotel_name: &'a str,
    check_in: NaiveDate,
    check_out: NaiveDate,
    guests: i32,
    rooms: i32,
    total_amount_minor_units: i64,
    currency: &'a str,
    payment_method_id: Option<&'a str>,
}

pub struct InsertPending<'a> {
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub travelport_property_id: &'a str,
    pub travelport_offer_id: Option<&'a str>,
    pub hotel_name: &'a str,
    pub check_in: NaiveDate,
    pub check_out: NaiveDate,
    pub guests: i32,
    pub rooms: i32,
    pub total_amount_minor_units: i64,
    pub currency: &'a str,
    pub payment_method_id: Option<&'a str>,
}

// ── Queries ─────────────────────────────────────────────────────────────────

/// Insert a `pending` booking row before contacting Travelport. The returned
/// id is what gets passed back into the LLM and used as `entity_id` on the
/// matching `issuing_card_log` row.
#[instrument(skip(pool, req), fields(
    user_id = %req.user_id,
    property = %req.travelport_property_id,
    amount_minor_units = req.total_amount_minor_units,
    currency = %req.currency,
))]
pub async fn insert_pending(pool: &DbPool, req: InsertPending<'_>) -> Result<Uuid> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let row = NewHotelBooking {
        user_id: req.user_id,
        conversation_id: req.conversation_id,
        travelport_property_id: req.travelport_property_id,
        travelport_offer_id: req.travelport_offer_id,
        hotel_name: req.hotel_name,
        check_in: req.check_in,
        check_out: req.check_out,
        guests: req.guests,
        rooms: req.rooms,
        total_amount_minor_units: req.total_amount_minor_units,
        currency: req.currency,
        payment_method_id: req.payment_method_id,
    };
    let id: Uuid = diesel::insert_into(hotel_bookings::table)
        .values(&row)
        .returning(hotel_bookings::id)
        .get_result(&mut conn)
        .await
        .context("Failed to insert hotel_bookings row")?;
    info!(booking_id = %id, "hotel booking inserted (pending)");
    Ok(id)
}

#[instrument(skip(pool))]
pub async fn attach_payment_intent(pool: &DbPool, id: Uuid, intent_id: &str) -> Result<()> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(hotel_bookings::table.filter(hotel_bookings::id.eq(id)))
        .set(hotel_bookings::stripe_payment_intent_id.eq(intent_id))
        .execute(&mut conn)
        .await
        .context("Failed to attach payment intent")?;
    info!(rows = n, "payment intent attached to booking");
    Ok(())
}

#[instrument(skip(pool, policy))]
pub async fn mark_confirmed(
    pool: &DbPool,
    id: Uuid,
    aggregator_locator: &str,
    supplier_locator: Option<&str>,
    policy: Option<serde_json::Value>,
) -> Result<()> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(hotel_bookings::table.filter(hotel_bookings::id.eq(id)))
        .set((
            hotel_bookings::status.eq("confirmed"),
            hotel_bookings::travelport_reservation_id.eq(aggregator_locator),
            hotel_bookings::travelport_supplier_locator.eq(supplier_locator),
            hotel_bookings::cancellation_policy.eq(policy),
            hotel_bookings::confirmed_at.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)
        .await
        .context("Failed to mark booking confirmed")?;
    info!(rows = n, "hotel booking confirmed");
    Ok(())
}

#[instrument(skip(pool))]
pub async fn mark_failed(pool: &DbPool, id: Uuid, reason: &str) -> Result<()> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(hotel_bookings::table.filter(hotel_bookings::id.eq(id)))
        .set((
            hotel_bookings::status.eq("failed"),
            hotel_bookings::failure_reason.eq(reason),
        ))
        .execute(&mut conn)
        .await
        .context("Failed to mark booking failed")?;
    info!(rows = n, "hotel booking marked failed");
    Ok(())
}

#[instrument(skip(pool))]
pub async fn mark_cancelled(
    pool: &DbPool,
    id: Uuid,
    refunded_minor_units: Option<i64>,
) -> Result<()> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::update(hotel_bookings::table.filter(hotel_bookings::id.eq(id)))
        .set((
            hotel_bookings::status.eq("cancelled"),
            hotel_bookings::refunded_amount_minor_units.eq(refunded_minor_units),
            hotel_bookings::cancelled_at.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)
        .await
        .context("Failed to mark booking cancelled")?;
    info!(rows = n, "hotel booking cancelled");
    Ok(())
}

#[instrument(skip(pool))]
pub async fn get_by_id(pool: &DbPool, user_id: Uuid, id: Uuid) -> Result<Option<HotelBooking>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    hotel_bookings::table
        .filter(hotel_bookings::id.eq(id))
        .filter(hotel_bookings::user_id.eq(user_id))
        .select(HotelBooking::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to load hotel booking")
}

#[allow(dead_code)] // wired through to the future "My Stays" UI; keep available now
#[instrument(skip(pool))]
pub async fn list_for_user(pool: &DbPool, user_id: Uuid) -> Result<Vec<HotelBooking>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    hotel_bookings::table
        .filter(hotel_bookings::user_id.eq(user_id))
        .order(hotel_bookings::check_in.desc())
        .select(HotelBooking::as_select())
        .load(&mut conn)
        .await
        .context("Failed to list hotel bookings")
}
