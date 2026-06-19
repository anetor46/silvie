use serde::Deserialize;
use tracing::{debug, instrument};

use super::error::TravelportError;

// TODO: verify exact token endpoint URL from Travelport+ developer portal
const TOKEN_URL: &str = "https://oauth.travelport.com/oauth/oauth20/token";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Fetches a short-lived bearer token using OAuth2 client credentials flow.
/// Called once per tool invocation — add token caching here if rate limits become a concern.
#[instrument(skip(client_id, client_secret, http_client), fields(client_id_len = client_id.len()))]
pub async fn fetch_access_token(
    client_id: &str,
    client_secret: &str,
    http_client: &reqwest::Client,
) -> Result<String, TravelportError> {
    debug!("fetching Travelport+ access token");

    let resp = http_client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;
    debug!("token endpoint status: {status}");

    if !status.is_success() {
        return Err(TravelportError::Auth(format!(
            "token endpoint returned HTTP {status}: {body}"
        )));
    }

    let token_resp: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| TravelportError::Auth(format!("failed to parse token response: {e}")))?;

    Ok(token_resp.access_token)
}
