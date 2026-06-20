//! `users` HTTP handlers. Queries + model live in `crate::repos::users`.

use poem::{handler, web::{Data, Json}};
use serde::Deserialize;

use crate::{
    auth::{AuthUser, Principal},
    db::DbPool,
    error::ApiResult,
    repos::users::{self, User},
};

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
) -> ApiResult<Json<User>> {
    let user = users::find_or_create(pool, &principal.sub, &req.email, &req.name).await?;
    Ok(Json(user))
}

/// `GET /users/me` — the user row matching the JWT. 404 if it doesn't exist
/// yet (the client should `POST /users` to create it).
#[handler]
pub async fn users_me_handler(auth: AuthUser) -> ApiResult<Json<User>> {
    Ok(Json(auth.user))
}
