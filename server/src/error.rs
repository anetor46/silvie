//! Single application error type for HTTP handlers.
//!
//! Replaces the per-handler boilerplate of:
//!   `foo(…).await.map_err(|e| { error!("…"); poem::Error::from_status(…) })?`
//! with plain `?` propagation. Implementing `poem::error::ResponseError` lets
//! the blanket `From<T> for poem::Error` carry it through.

use poem::{error::ResponseError, http::StatusCode, Body, Response};
use tracing::{error, warn};

#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] // BadRequest/Unauthorized are part of the public surface
pub enum ApiError {
    /// 401 — no/invalid bearer token (issued by the JWT extractor).
    #[error("unauthorized")]
    Unauthorized,
    /// 404 — the requested resource doesn't exist (or isn't owned by the caller).
    #[error("not found")]
    NotFound,
    /// 400 — caller-correctable error. Body carries the public message.
    #[error("{0}")]
    BadRequest(String),
    /// 503 — a required external service / config knob is missing. Body
    /// carries the public message (e.g. "Stripe is not configured").
    #[error("{0}")]
    Unavailable(String),
    /// 500 — anything not categorized above. Server logs the chain at
    /// `error!`; the client only sees a generic status.
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ResponseError for ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Build the HTTP response, logging server-side context at the right
    /// level. This is poem's hook that runs when the error is converted to
    /// the wire response — so logging here happens exactly once per request.
    fn as_response(&self) -> Response {
        let status = self.status();
        let body = match self {
            ApiError::Internal(e) => {
                error!("internal error: {e:#}");
                Body::empty()
            }
            ApiError::Unauthorized | ApiError::NotFound => Body::empty(),
            ApiError::BadRequest(msg) => {
                warn!("bad request: {msg}");
                Body::from_string(msg.clone())
            }
            ApiError::Unavailable(msg) => {
                warn!("service unavailable: {msg}");
                Body::from_string(msg.clone())
            }
        };
        Response::builder().status(status).body(body)
    }
}

/// Convenience: turn an `Option<T>` into a 404 at the call site.
///
/// ```ignore
/// let row = repo::find_one(…).await?.or_not_found()?;
/// ```
pub trait OptionExt<T> {
    fn or_not_found(self) -> Result<T, ApiError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_not_found(self) -> Result<T, ApiError> {
        self.ok_or(ApiError::NotFound)
    }
}

/// Same idea for `Result<Option<T>, _>` — collapse the Ok(None) case into 404.
///
/// ```ignore
/// let row = repo::find_one(…).await.into_required()?;
/// ```
pub trait ResultOptionExt<T> {
    fn into_required(self) -> Result<T, ApiError>;
}

impl<T, E> ResultOptionExt<T> for Result<Option<T>, E>
where
    ApiError: From<E>,
{
    fn into_required(self) -> Result<T, ApiError> {
        self?.or_not_found()
    }
}

/// Public alias every handler can use as its return type.
pub type ApiResult<T> = Result<T, ApiError>;
