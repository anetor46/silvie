use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum TravelportError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Travelport API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },
    /// Genuine JSON parse failure — the body wasn't valid JSON.
    #[error("Failed to parse Travelport response: {0}")]
    Parse(String),
    /// 200 OK but the body's structure isn't one we recognise. The wrapped
    /// String is the user-facing message (kept short and concrete); the
    /// raw body lives in the logs at `error!` level via
    /// [`log_and_unexpected`].
    #[error("{0}")]
    UnexpectedResponse(String),
    #[error("Invalid argument: {0}")]
    InvalidArg(String),
    #[error("Authentication failed: {0}")]
    Auth(String),
}

pub(super) fn make_api_error(status: reqwest::StatusCode, body: String) -> TravelportError {
    error!("travelport API error ({status}): {body}");
    TravelportError::ApiError {
        status: status.as_u16(),
        body,
    }
}

/// Build an `UnexpectedResponse` while emitting the full raw body and any
/// useful debug fields at `error!` level. Use this anywhere we get a 200
/// OK back but the body's shape isn't what we expected — so the next time
/// the bug shows up we can grep the logs instead of recompiling.
///
/// `user_msg` is the short message the LLM (and therefore the user) sees;
/// keep it concrete and non-technical. `context` is a tracing-friendly
/// label for the call site (e.g. "availability"). `raw_body` is the
/// truncated body string.
pub(super) fn log_and_unexpected(
    context: &'static str,
    user_msg: impl Into<String>,
    raw_body: &str,
    extra: &[(&str, &str)],
) -> TravelportError {
    // Truncate aggressively — Travelport responses can be tens of KB and
    // we don't want to wedge log shippers. 8 KB is more than enough to
    // see the envelope shape and a sample offering.
    const MAX_BODY_LOG: usize = 8 * 1024;
    let truncated = if raw_body.len() <= MAX_BODY_LOG {
        raw_body.to_string()
    } else {
        format!(
            "{}…[truncated, {} bytes total]",
            &raw_body[..MAX_BODY_LOG],
            raw_body.len()
        )
    };
    error!(
        context = context,
        body = %truncated,
        extra = ?extra,
        "Travelport response shape unexpected; full body logged for investigation"
    );
    TravelportError::UnexpectedResponse(user_msg.into())
}
