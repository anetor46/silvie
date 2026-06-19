use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum TravelportError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Travelport API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },
    #[error("Failed to parse Travelport response: {0}")]
    Parse(String),
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
