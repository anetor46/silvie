//! Google OAuth handshake. We only own the browser-side ceremony here — once
//! the user grants consent and we have a code, we exchange it for tokens and
//! return them to the frontend. Persistence (and refresh) happens on the
//! backend via the `integrations` table; this module never touches the keychain.

use anyhow::{anyhow, Context, Result};
use oauth2::{
    basic::BasicClient, url::Url, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_oauth::OauthConfig;
use tauri_plugin_opener::OpenerExt;
use tracing::{debug, error, info, instrument, warn};

/// Fixed loopback port for the Google Calendar OAuth callback.
/// Must be registered in Google Cloud Console as an authorized redirect URI:
///   http://127.0.0.1:1422
/// Using a fixed port lets us use a Web application client (which requires
/// exact URI matching) rather than a Desktop app client.
const GOOGLE_OAUTH_PORT: u16 = 1422;

/// Tokens returned to the frontend after a successful OAuth dance. The frontend
/// forwards these to `POST /users/me/integrations` so the backend takes over
/// storage + refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Seconds until the access token expires.
    pub expires_in: Option<i64>,
    /// Google's stable subject identifier — used as `provider_account_id`.
    pub provider_account_id: String,
    pub email: String,
    pub scopes: Vec<String>,
}

/// Parsed fields from the Google `/userinfo` endpoint.
struct GoogleUserInfo {
    sub: String,
    email: String,
}

#[instrument(skip(app, client_secret))]
pub async fn google_oauth_flow(
    app: &AppHandle,
    client_id: &str,
    client_secret: &str,
) -> Result<OAuthTokens> {
    info!("starting Google OAuth flow");

    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let sender = Arc::new(Mutex::new(Some(tx)));

    let port = tauri_plugin_oauth::start_with_config(
        OauthConfig {
            ports: Some(vec![GOOGLE_OAUTH_PORT]),
            response: None,
        },
        move |url| {
            debug!("loopback server received redirect: {url}");
            if let Ok(mut guard) = sender.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(url);
                }
            }
        },
    )
    .map_err(|e| anyhow!("Failed to start OAuth server on port {GOOGLE_OAUTH_PORT}: {e}"))?;
    info!("loopback server started on port {port}");

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("Failed to build HTTP client")?;

    let client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_client_secret(ClientSecret::new(client_secret.to_string()))
        .set_auth_uri(
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
                .context("Invalid auth URL")?,
        )
        .set_token_uri(
            TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .context("Invalid token URL")?,
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("http://127.0.0.1:{GOOGLE_OAUTH_PORT}"))
                .context("Invalid redirect URL")?,
        );

    let calendar_scope = "https://www.googleapis.com/auth/calendar.events".to_string();
    let scopes = vec![
        calendar_scope.clone(),
        "email".to_string(),
        "profile".to_string(),
    ];

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let mut auth = client.authorize_url(CsrfToken::new_random);
    for s in &scopes {
        auth = auth.add_scope(Scope::new(s.clone()));
    }
    // Force a refresh_token on every consent — Google only issues one on the
    // first authorization for a given (client, user) pair otherwise.
    let (auth_url, _csrf_token) = auth
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .set_pkce_challenge(pkce_challenge)
        .url();

    debug!("auth URL built: {auth_url}");
    info!("opening system browser");
    app.opener()
        .open_url(auth_url.as_str(), None::<&str>)
        .map_err(|e| anyhow!("Failed to open browser: {e}"))?;

    info!("waiting for OAuth redirect (timeout 5 min)");
    let redirect_url = match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
        Ok(Ok(url)) => {
            info!("received redirect URL");
            debug!("redirect URL: {url}");
            url
        }
        Ok(Err(_)) => {
            warn!("OAuth oneshot channel closed — flow cancelled");
            let _ = tauri_plugin_oauth::cancel(port);
            return Err(anyhow!("OAuth flow was cancelled"));
        }
        Err(_) => {
            warn!("OAuth flow timed out after 5 minutes");
            let _ = tauri_plugin_oauth::cancel(port);
            return Err(anyhow!("OAuth login timed out. Please try again."));
        }
    };
    let _ = tauri_plugin_oauth::cancel(port);

    let parsed = Url::parse(&redirect_url).context("Failed to parse redirect URL")?;
    let code = parsed
        .query_pairs()
        .find(|(k, _v)| k == "code")
        .map(|(_k, v)| v.to_string())
        .ok_or_else(|| {
            let error = parsed
                .query_pairs()
                .find(|(k, _v)| k == "error")
                .map(|(_k, v)| v.to_string())
                .unwrap_or_else(|| "unknown error".to_string());
            anyhow!("OAuth error from Google: {error}")
        })?;

    info!("authorization code received (len={})", code.len());
    info!("exchanging authorization code for tokens");

    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .map_err(|e| {
            error!("token exchange failed: {e}");
            anyhow!("Token exchange failed: {e}")
        })?;

    let access_token = token_response.access_token().secret().to_string();
    let refresh_token = token_response.refresh_token().map(|t| t.secret().to_string());
    let expires_in = token_response.expires_in().map(|d| d.as_secs() as i64);

    info!(
        has_refresh_token = refresh_token.is_some(),
        access_token_len = access_token.len(),
        expires_in,
        "token exchange succeeded"
    );

    info!("fetching userinfo");
    let userinfo = fetch_userinfo(&access_token).await.map_err(|e| {
        error!("userinfo fetch failed: {e:#}");
        anyhow!("Failed to retrieve your Google account details. Please try again.")
    })?;
    info!(email_len = userinfo.email.len(), "userinfo fetched");

    Ok(OAuthTokens {
        access_token,
        refresh_token,
        expires_in,
        provider_account_id: userinfo.sub,
        email: userinfo.email,
        scopes,
    })
}

#[instrument(skip(access_token))]
async fn fetch_userinfo(access_token: &str) -> Result<GoogleUserInfo> {
    const USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

    #[derive(Deserialize)]
    struct UserInfo {
        sub: String,
        email: String,
    }

    debug!("GET {USERINFO_URL}");
    let client = reqwest::Client::new();
    let response = client
        .get(USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .context("Failed to send userinfo request")?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read userinfo response body")?;

    if !status.is_success() {
        error!(%status, "userinfo request failed: {body}");
        return Err(anyhow!("userinfo returned HTTP {status}"));
    }

    let info: UserInfo = serde_json::from_str(&body).map_err(|e| {
        error!("failed to parse userinfo response: {e} — body: {body}");
        anyhow!("Unexpected response format from Google. Please try again.")
    })?;

    Ok(GoogleUserInfo {
        sub: info.sub,
        email: info.email,
    })
}
