//! HTTP layer for the user-info aggregate.

use poem::{handler, web::{Data, Json}};

use crate::{
    auth::AuthUser,
    db::DbPool,
    error::ApiResult,
    repos::user_info::{self, UserInfoPatch, UserInfoResponse},
};

/// `GET /users/me/info` — returns the combined view. The user row must
/// exist (the `AuthUser` extractor enforces that).
#[handler]
pub async fn get_user_info_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
) -> ApiResult<Json<UserInfoResponse>> {
    Ok(Json(user_info::fetch_user_info(pool, auth.user.id).await?))
}

/// `PUT /users/me/info` — partial update. Any omitted section is left alone.
#[handler]
pub async fn update_user_info_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Json(req): Json<UserInfoPatch>,
) -> ApiResult<Json<UserInfoResponse>> {
    Ok(Json(
        user_info::update_user_info(pool, auth.user.id, req).await?,
    ))
}
