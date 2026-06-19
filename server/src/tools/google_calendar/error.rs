use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Calendar API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },
    #[error("Failed to parse Calendar API response: {0}")]
    Parse(String),
    #[error("Invalid argument: {0}")]
    InvalidArg(String),
}

pub(super) fn make_api_error(status: reqwest::StatusCode, body: String) -> CalendarError {
    error!("calendar API error ({status}): {body}");
    CalendarError::ApiError {
        status: status.as_u16(),
        body,
    }
}
