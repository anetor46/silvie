use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum OutlookError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Outlook API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },
    #[error("Failed to parse Outlook API response: {0}")]
    Parse(String),
}

pub(super) fn make_api_error(status: reqwest::StatusCode, body: String) -> OutlookError {
    error!("Outlook API error ({status}): {body}");
    OutlookError::ApiError {
        status: status.as_u16(),
        body,
    }
}
