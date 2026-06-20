//! HTTP layer for third-party integration credentials.

use std::sync::Arc;

use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json, Path},
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::DbPool,
    error::{ApiResult, ResultOptionExt},
    repos::integrations::{
        self, AccessTokenResponse, IntegrationView, IntegrationsConfig, UpsertIntegrationRequest,
    },
};

/// GET /users/me/integrations — list of (id, provider, email, status, …) for
/// the current user. Tokens are intentionally NOT returned.
#[handler]
pub async fn list_integrations_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
) -> ApiResult<Json<Vec<IntegrationView>>> {
    Ok(Json(
        integrations::list_user_integrations(pool, auth.user.id).await?,
    ))
}

/// POST /users/me/integrations — find-or-update an integration. Called from
/// the client immediately after a successful OAuth handshake.
#[handler]
pub async fn upsert_integration_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Json(req): Json<UpsertIntegrationRequest>,
) -> ApiResult<Json<IntegrationView>> {
    Ok(Json(
        integrations::upsert_integration(pool, auth.user.id, &req).await?,
    ))
}

/// DELETE /users/me/integrations/:id — disconnect one integration.
#[handler]
pub async fn delete_integration_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let removed = integrations::delete_integration_by_id(pool, auth.user.id, id).await?;
    Ok(if removed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    })
}

/// GET /users/me/integrations/:provider/access-token — returns a fresh
/// access token for that provider (refreshing server-side via the stored
/// refresh_token if needed). 404 if the user has no integration.
#[handler]
pub async fn get_access_token_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Data(cfg): Data<&Arc<IntegrationsConfig>>,
    Path(provider): Path<String>,
) -> ApiResult<Json<AccessTokenResponse>> {
    let token = integrations::fresh_access_token(pool, cfg, auth.user.id, &provider)
        .await
        .into_required()?;
    Ok(Json(token))
}
