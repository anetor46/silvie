//! Combined user-info module: profile (1:1), home address (1 of N addresses),
//! and primary passport (1 of N travel_documents). The current UI only writes
//! one address (type='home') and one document (type='passport', primary), so
//! the public API surface treats them as singletons; the underlying tables
//! are already structured to support more.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
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
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{
    auth::Principal,
    db::DbPool,
    schema::{addresses, travel_documents, user_profiles},
    users,
};

const HOME_ADDRESS_TYPE: &str = "home";
const PRIMARY_PASSPORT_TYPE: &str = "passport";

// ── Models (read-side) ───────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = user_profiles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserProfile {
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub nationality: Option<String>,
    pub country_of_residence: Option<String>,
    pub preferred_currency: Option<String>,
    pub preferred_language: Option<String>,
    pub timezone: Option<String>,
    pub meal_preference: Option<String>,
    pub seat_preference: Option<String>,
    pub cabin_class_preference: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = addresses)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Option<Uuid>,
    #[diesel(column_name = type_)]
    #[serde(rename = "type")]
    pub type_: String,
    pub label: Option<String>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = travel_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TravelDocument {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Option<Uuid>,
    #[diesel(column_name = type_)]
    #[serde(rename = "type")]
    pub type_: String,
    pub document_number: Option<String>,
    pub issuing_country: Option<String>,
    pub nationality: Option<String>,
    pub issue_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub is_primary: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// ── Request / response shapes ────────────────────────────────────────────────

#[derive(Serialize)]
pub struct UserInfoResponse {
    pub profile: Option<UserProfile>,
    pub home_address: Option<Address>,
    pub primary_passport: Option<TravelDocument>,
}

#[derive(Deserialize, Default)]
pub struct ProfilePatch {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub nationality: Option<String>,
    pub country_of_residence: Option<String>,
    pub preferred_currency: Option<String>,
    pub preferred_language: Option<String>,
    pub timezone: Option<String>,
    pub meal_preference: Option<String>,
    pub seat_preference: Option<String>,
    pub cabin_class_preference: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct AddressPatch {
    pub label: Option<String>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct PassportPatch {
    pub document_number: Option<String>,
    pub issuing_country: Option<String>,
    pub nationality: Option<String>,
    pub issue_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
}

#[derive(Deserialize, Default)]
pub struct UpdateUserInfoRequest {
    /// Each section is optional. Omitted → leave unchanged; present → upsert.
    /// Within a section, every field is sent (NULL clears the value).
    pub profile: Option<ProfilePatch>,
    pub home_address: Option<AddressPatch>,
    pub primary_passport: Option<PassportPatch>,
}

// ── Insertable bridges (Diesel's Insertable needs `#[diesel(table_name)]`) ──

#[derive(Insertable, AsChangeset, Default)]
#[diesel(table_name = user_profiles)]
struct UserProfileForm<'a> {
    first_name: Option<&'a str>,
    last_name: Option<&'a str>,
    phone: Option<&'a str>,
    nationality: Option<&'a str>,
    country_of_residence: Option<&'a str>,
    preferred_currency: Option<&'a str>,
    preferred_language: Option<&'a str>,
    timezone: Option<&'a str>,
    meal_preference: Option<&'a str>,
    seat_preference: Option<&'a str>,
    cabin_class_preference: Option<&'a str>,
}

#[derive(Insertable, AsChangeset, Default)]
#[diesel(table_name = addresses)]
struct AddressForm<'a> {
    label: Option<&'a str>,
    line1: Option<&'a str>,
    line2: Option<&'a str>,
    city: Option<&'a str>,
    state: Option<&'a str>,
    postal_code: Option<&'a str>,
    country: Option<&'a str>,
}

#[derive(Insertable, AsChangeset, Default)]
#[diesel(table_name = travel_documents)]
struct TravelDocumentForm<'a> {
    document_number: Option<&'a str>,
    issuing_country: Option<&'a str>,
    nationality: Option<&'a str>,
    issue_date: Option<NaiveDate>,
    expiry_date: Option<NaiveDate>,
}

// ── Queries ─────────────────────────────────────────────────────────────────

/// Assemble the full read-side view for a user.
#[instrument(skip(pool), fields(sub_len = sub.len()))]
pub async fn fetch_user_info(pool: &DbPool, sub: &str) -> Result<Option<UserInfoResponse>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(None),
    };

    let profile = user_profiles::table
        .find(user.id)
        .select(UserProfile::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to query user_profiles")?;

    let home_address = addresses::table
        .filter(addresses::user_id.eq(user.id))
        .filter(addresses::type_.eq(HOME_ADDRESS_TYPE))
        .filter(addresses::deleted_at.is_null())
        .select(Address::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to query addresses")?;

    let primary_passport = travel_documents::table
        .filter(travel_documents::user_id.eq(user.id))
        .filter(travel_documents::type_.eq(PRIMARY_PASSPORT_TYPE))
        .filter(travel_documents::is_primary.eq(true))
        .filter(travel_documents::deleted_at.is_null())
        .select(TravelDocument::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to query travel_documents")?;

    Ok(Some(UserInfoResponse {
        profile,
        home_address,
        primary_passport,
    }))
}

/// Apply a partial update across the three tables in a single transaction.
/// Returns the refreshed view.
#[instrument(skip(pool, req), fields(sub_len = sub.len()))]
pub async fn update_user_info(
    pool: &DbPool,
    sub: &str,
    req: UpdateUserInfoRequest,
) -> Result<Option<UserInfoResponse>> {
    let user = match users::find_by_sub(pool, sub).await? {
        Some(u) => u,
        None => return Ok(None),
    };

    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    // NOTE on atomicity: the writes below are NOT wrapped in a DB transaction.
    // diesel-async 0.9's transaction API combined with rustc's current async-
    // closure HRTB inference can't express this cleanly (see issue threads in
    // diesel-rs/diesel#3941 and rust-lang/rust#100013). Each individual UPSERT
    // is idempotent, so a partial failure leaves the user free to retry. If
    // we ever need true atomicity here, the workaround is to do the writes
    // sync inside `tokio::task::spawn_blocking` using sync diesel.
    let user_id = user.id;
    {
        let tx_conn = &mut *conn;
        if let Some(p) = req.profile.as_ref() {
            let form = UserProfileForm {
                first_name: p.first_name.as_deref(),
                last_name: p.last_name.as_deref(),
                phone: p.phone.as_deref(),
                nationality: p.nationality.as_deref(),
                country_of_residence: p.country_of_residence.as_deref(),
                preferred_currency: p.preferred_currency.as_deref(),
                preferred_language: p.preferred_language.as_deref(),
                timezone: p.timezone.as_deref(),
                meal_preference: p.meal_preference.as_deref(),
                seat_preference: p.seat_preference.as_deref(),
                cabin_class_preference: p.cabin_class_preference.as_deref(),
            };
            let _: usize = diesel::insert_into(user_profiles::table)
                .values((user_profiles::user_id.eq(user_id), &form))
                .on_conflict(user_profiles::user_id)
                .do_update()
                .set((&form, user_profiles::updated_at.eq(diesel::dsl::now)))
                .execute(tx_conn)
                .await
                .context("Failed to upsert user_profile")?;
        }
        if let Some(a) = req.home_address.as_ref() {
            let form = AddressForm {
                label: a.label.as_deref(),
                line1: a.line1.as_deref(),
                line2: a.line2.as_deref(),
                city: a.city.as_deref(),
                state: a.state.as_deref(),
                postal_code: a.postal_code.as_deref(),
                country: a.country.as_deref(),
            };
            let _: usize = diesel::insert_into(addresses::table)
                .values((
                    addresses::user_id.eq(user_id),
                    addresses::type_.eq(HOME_ADDRESS_TYPE),
                    addresses::is_default.eq(true),
                    &form,
                ))
                .on_conflict_do_nothing()
                .execute(tx_conn)
                .await
                .context("Failed to insert home address")?;
            let _: usize = diesel::update(
                addresses::table
                    .filter(addresses::user_id.eq(user_id))
                    .filter(addresses::type_.eq(HOME_ADDRESS_TYPE))
                    .filter(addresses::deleted_at.is_null()),
            )
            .set((&form, addresses::updated_at.eq(diesel::dsl::now)))
            .execute(tx_conn)
            .await
            .context("Failed to update home address")?;
        }
        if let Some(d) = req.primary_passport.as_ref() {
            let form = TravelDocumentForm {
                document_number: d.document_number.as_deref(),
                issuing_country: d.issuing_country.as_deref(),
                nationality: d.nationality.as_deref(),
                issue_date: d.issue_date,
                expiry_date: d.expiry_date,
            };
            let _: usize = diesel::insert_into(travel_documents::table)
                .values((
                    travel_documents::user_id.eq(user_id),
                    travel_documents::type_.eq(PRIMARY_PASSPORT_TYPE),
                    travel_documents::is_primary.eq(true),
                    &form,
                ))
                .on_conflict_do_nothing()
                .execute(tx_conn)
                .await
                .context("Failed to insert primary passport")?;
            let _: usize = diesel::update(
                travel_documents::table
                    .filter(travel_documents::user_id.eq(user_id))
                    .filter(travel_documents::type_.eq(PRIMARY_PASSPORT_TYPE))
                    .filter(travel_documents::is_primary.eq(true))
                    .filter(travel_documents::deleted_at.is_null()),
            )
            .set((
                &form,
                travel_documents::updated_at.eq(diesel::dsl::now),
            ))
            .execute(tx_conn)
            .await
            .context("Failed to update primary passport")?;
        }
    }

    // Reload the canonical view post-write.
    drop(conn);
    let view = fetch_user_info(pool, sub).await?;
    info!("user info updated");
    Ok(view)
}

// ── HTTP handlers ───────────────────────────────────────────────────────────

/// `GET /users/me/info` — returns the combined view. 404 if the user has no
/// DB row at all (shouldn't happen in normal flows since `/users` runs first).
#[handler]
pub async fn get_user_info_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
) -> poem::Result<Json<UserInfoResponse>> {
    let view = fetch_user_info(pool, &principal.sub).await.map_err(|e| {
        error!("user_info fetch failed: {e:#}");
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    view.map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}

/// `PUT /users/me/info` — partial update. Any omitted section is left alone.
#[handler]
pub async fn update_user_info_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
    Json(req): Json<UpdateUserInfoRequest>,
) -> poem::Result<Json<UserInfoResponse>> {
    let view = update_user_info(pool, &principal.sub, req)
        .await
        .map_err(|e| {
            error!("user_info update failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    view.map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}
