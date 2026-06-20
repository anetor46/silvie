use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{auth::Principal, db::DbPool, schema::users};

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub auth0_sub: String,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUser<'a> {
    auth0_sub: &'a str,
    email: &'a str,
    name: &'a str,
}

// ── DB operations ───────────────────────────────────────────────────────────

/// Look up a user by their Auth0 `sub` claim. Returns `None` if no row exists
/// or the row is soft-deleted.
#[instrument(skip(pool), fields(sub_len = sub.len()))]
pub async fn find_by_sub(pool: &DbPool, sub: &str) -> Result<Option<User>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    users::table
        .filter(users::auth0_sub.eq(sub))
        .filter(users::deleted_at.is_null())
        .select(User::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to query users by auth0_sub")
}

/// Find a user by `auth0_sub`, or insert a new row with the given email +
/// name if none exists. The DB is the source of truth — if the row already
/// exists, the provided `email`/`name` are ignored.
///
/// Uses a single `INSERT … ON CONFLICT (auth0_sub) DO NOTHING RETURNING *`.
/// On conflict (no row returned by INSERT), falls back to a SELECT.
#[instrument(skip(pool, email, name), fields(sub_len = sub.len(), email_len = email.len(), name_len = name.len()))]
pub async fn find_or_create(
    pool: &DbPool,
    sub: &str,
    email: &str,
    name: &str,
) -> Result<User> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;

    let inserted: Option<User> = diesel::insert_into(users::table)
        .values(NewUser {
            auth0_sub: sub,
            email,
            name,
        })
        .on_conflict(users::auth0_sub)
        .do_nothing()
        .returning(User::as_select())
        .get_result(&mut conn)
        .await
        .optional()
        .context("Failed to insert user")?;

    if let Some(u) = inserted {
        info!(user_id = %u.id, "created new user");
        return Ok(u);
    }

    // Conflict path — the row already exists. Fetch it.
    users::table
        .filter(users::auth0_sub.eq(sub))
        .select(User::as_select())
        .first(&mut conn)
        .await
        .context("Failed to fetch existing user after conflict")
}

// ── HTTP handlers ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SyncUserRequest {
    pub email: String,
    pub name: String,
}

/// `POST /users` — find-or-create. Used by the client immediately after a
/// successful Auth0 flow (signup or login, in-app or browser). Idempotent:
/// returns the existing row unchanged if it already exists.
#[handler]
pub async fn create_user_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
    Json(req): Json<SyncUserRequest>,
) -> poem::Result<Json<User>> {
    find_or_create(pool, &principal.sub, &req.email, &req.name)
        .await
        .map(Json)
        .map_err(|e| {
            error!("user upsert failed: {e:#}");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })
}

/// `GET /users/me` — strict lookup. Returns 404 if the caller's `sub` has no
/// corresponding row (they need to complete signup first).
#[handler]
pub async fn users_me_handler(
    principal: Principal,
    Data(pool): Data<&DbPool>,
) -> poem::Result<Json<User>> {
    let user_opt = find_by_sub(pool, &principal.sub).await.map_err(|e| {
        error!("user lookup failed: {e:#}");
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    user_opt
        .map(Json)
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
}
