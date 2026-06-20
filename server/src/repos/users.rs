//! `users` ORM — the `User` model plus the queries that read/write it.
//! Handlers live in `crate::api::users`.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{db::DbPool, schema::users};

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

/// Look up a user by their Auth0 `sub` claim. Returns `None` if no row exists
/// (or the row is soft-deleted).
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

    users::table
        .filter(users::auth0_sub.eq(sub))
        .select(User::as_select())
        .first(&mut conn)
        .await
        .context("Failed to fetch existing user after conflict")
}
