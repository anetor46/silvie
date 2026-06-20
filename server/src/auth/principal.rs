//! Lightweight extractor that validates a Bearer JWT and yields the `sub`
//! claim. Use [`Principal`] for endpoints that don't need to touch the DB
//! user row; otherwise prefer [`super::AuthUser`].

use std::sync::Arc;

use poem::{http::StatusCode, Error as PoemError, FromRequest, Request, RequestBody, Result};
use tracing::warn;

use super::jwt::JwtValidator;

pub struct Principal {
    pub sub: String,
}

impl<'a> FromRequest<'a> for Principal {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let token = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| PoemError::from_status(StatusCode::UNAUTHORIZED))?;

        let validator = req.data::<Arc<JwtValidator>>().ok_or_else(|| {
            tracing::error!("JwtValidator not present in poem request data");
            PoemError::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        let claims = validator.validate(token).await.map_err(|e| {
            warn!(error = %e, "rejected unauthenticated request");
            PoemError::from_status(StatusCode::UNAUTHORIZED)
        })?;

        Ok(Principal { sub: claims.sub })
    }
}
