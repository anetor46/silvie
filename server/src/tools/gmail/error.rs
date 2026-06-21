use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum GmailError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Gmail API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },
    #[error("Failed to parse Gmail API response: {0}")]
    Parse(String),
}

pub(super) fn make_api_error(status: reqwest::StatusCode, body: String) -> GmailError {
    error!("Gmail API error ({status}): {body}");
    GmailError::ApiError {
        status: status.as_u16(),
        body,
    }
}
