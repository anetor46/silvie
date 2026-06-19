use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use oauth2::url::Url;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Context, Result};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tracing::{debug, error, info, instrument, warn};

const KEYRING_SERVICE: &str = "com.silvie";
const KEYRING_ACCOUNT: &str = "google-calendar";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedAccount {
    pub email: String,
}

#[derive(Serialize, Deserialize)]
struct StoredCredentials {
    access_token: String,
    refresh_token: Option<String>,
    email: String,
}

#[instrument(skip(app, client_secret))]
pub async fn google_oauth_flow(
    app: &AppHandle,
    client_id: &str,
    client_secret: &str,
) -> Result<ConnectedAccount> {
    info!("starting Google OAuth flow");

    // 1. Channel to receive the redirect URL from the loopback server thread.
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let sender = Arc::new(Mutex::new(Some(tx)));

    // 2. Start the loopback server; capture port for redirect_uri.
    let port = tauri_plugin_oauth::start(move |url| {
        debug!("loopback server received redirect: {url}");
        if let Ok(mut guard) = sender.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(url);
            }
        }
    })
    .map_err(|e| anyhow!("Failed to start OAuth server: {e}"))?;
    info!("loopback server started on port {port}");

    // 3. Build the oauth2 client (Policy::none required for token exchange).
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
            RedirectUrl::new(format!("http://127.0.0.1:{port}"))
                .context("Invalid redirect URL")?,
        );

    // 4. Generate PKCE and build the authorization URL.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar.events.readonly".to_string(),
        ))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    debug!("auth URL built: {auth_url}");

    // 5. Open the system browser.
    info!("opening system browser");
    app.opener()
        .open_url(auth_url.as_str(), None::<&str>)
        .map_err(|e| anyhow!("Failed to open browser: {e}"))?;

    // 6. Wait for the redirect, with a 5-minute timeout.
    info!("waiting for OAuth redirect (timeout 5 min)");
    let redirect_url = match tokio::time::timeout(
        std::time::Duration::from_secs(300),
        rx,
    )
    .await
    {
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

    // 7. Parse the authorization code from the redirect URL.
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

    // 8. Exchange the code for tokens.
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
    let refresh_token = token_response
        .refresh_token()
        .map(|t| t.secret().to_string());

    info!(
        has_refresh_token = refresh_token.is_some(),
        access_token_len = access_token.len(),
        "token exchange succeeded"
    );

    // 9. Fetch the user's email with a separate client (default redirect policy).
    info!("fetching userinfo");
    let email = fetch_email(&access_token).await?;
    info!("userinfo fetched, email={email}");

    // 10. Store credentials in the OS keychain.
    info!("storing credentials in keychain");
    let payload = serde_json::to_string(&StoredCredentials {
        access_token,
        refresh_token,
        email: email.clone(),
    })?;
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| anyhow!("Keyring unavailable: {e}"))?
        .set_password(&payload)
        .map_err(|e| anyhow!("Failed to store credentials: {e}"))?;

    info!("OAuth flow complete");
    Ok(ConnectedAccount { email })
}

#[instrument(skip(access_token))]
async fn fetch_email(access_token: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct UserInfo {
        email: String,
    }

    debug!("building userinfo HTTP client");
    let client = reqwest::Client::new();

    debug!("sending GET https://www.googleapis.com/oauth2/v2/userinfo");
    let response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .context("Failed to send userinfo request")?;

    let status = response.status();
    debug!("userinfo response status: {status}");

    let body = response
        .text()
        .await
        .context("Failed to read userinfo response body")?;

    if status.is_success() {
        debug!("userinfo body (success): {body}");
    } else {
        error!("userinfo body (error {status}): {body}");
        return Err(anyhow!("userinfo returned HTTP {status}: {body}"));
    }

    let info: UserInfo = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse userinfo JSON: {body}"))?;

    Ok(info.email)
}

pub fn load_google_account() -> Option<ConnectedAccount> {
    let payload = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .ok()?
        .get_password()
        .ok()?;
    let creds: StoredCredentials = serde_json::from_str(&payload).ok()?;
    Some(ConnectedAccount { email: creds.email })
}

pub fn remove_google_account() -> Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| anyhow!("Keyring unavailable: {e}"))?
        .delete_credential()
        .map_err(|e| anyhow!("Failed to remove credentials: {e}"))?;
    Ok(())
}
