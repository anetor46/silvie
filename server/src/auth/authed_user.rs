//! `AuthUser` — extractor that validates the JWT *and* resolves the matching
//! DB user row in one shot. Saves every protected handler from doing the
//! `find_by_sub(pool, &principal.sub)` dance.
//!
//! Use [`super::Principal`] only when you genuinely don't need the DB row
//! (e.g. the `POST /users` endpoint that creates the row).

use poem::{http::StatusCode, web::Data, Error as PoemError, FromRequest, Request, RequestBody, Result};
use tracing::error;

use crate::{db::DbPool, repos::users};

use super::principal::Principal;

pub struct AuthUser {
    /// Raw JWT claims. Kept available for callers that need `sub` directly
    /// (e.g. logging or per-request integrations queries).
    #[allow(dead_code)]
    pub principal: Principal,
    pub user: users::User,
}

impl<'a> FromRequest<'a> for AuthUser {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        // Validate the JWT first (reuses Principal's extractor).
        let principal = Principal::from_request(req, body).await?;

        // Then look up the corresponding DB row. The pool comes from poem's
        // shared data — same place the handlers themselves get it from.
        let Data(pool) = <Data<&DbPool>>::from_request(req, body).await.map_err(|e| {
            error!("DbPool not present in poem request data: {e:#}");
            PoemError::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        let user = users::find_by_sub(pool, &principal.sub).await.map_err(|e| {
            error!("user lookup in AuthUser extractor failed: {e:#}");
            PoemError::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
        let user = user.ok_or_else(|| PoemError::from_status(StatusCode::NOT_FOUND))?;

        Ok(AuthUser { principal, user })
    }
}
