//! Microsoft Outlook OAuth handshake. Single integration covering both Outlook
//! Mail and Calendar — all scopes are requested in one consent screen so the
//! user only authenticates once.

use anyhow::{anyhow, Context, Result};
use oauth2::{
    basic::BasicClient, url::Url, AuthUrl, AuthorizationCode, ClientId, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_oauth::OauthConfig;
use tauri_plugin_opener::OpenerExt;
use tracing::{debug, error, info, instrument, warn};

use super::OAuthTokens;

/// Fixed loopback port for the Microsoft OAuth callback. Must be registered in
/// the Azure app registration as `http://127.0.0.1:1423` under
/// "Mobile and desktop applications" (which permits plain HTTP).
const OAUTH_PORT: u16 = 1423;

/// All scopes requested on connect. `offline_access` is what triggers refresh
/// token issuance in Microsoft's v2.0 endpoint.
const SCOPES: &[&str] = &[
    "openid",
    "email",
    "profile",
    "offline_access",
    "Mail.ReadWrite",
    "Mail.Send",
    "Calendars.ReadWrite",
    "User.Read",
];

#[instrument(skip(app, client_id))]
pub async fn run(app: &AppHandle, client_id: &str) -> Result<OAuthTokens> {
    info!("starting Microsoft OAuth flow");

    if client_id.is_empty() {
        return Err(anyhow!(
            "OUTLOOK_CLIENT_ID is not configured. Add it to src-tauri/.env."
        ));
    }

    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let sender = Arc::new(Mutex::new(Some(tx)));

    let port = tauri_plugin_oauth::start_with_config(
        OauthConfig {
            ports: Some(vec![OAUTH_PORT]),
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
    .map_err(|e| anyhow!("Failed to start OAuth server on port {OAUTH_PORT}: {e}"))?;
    info!("loopback server started on port {port}");

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("Failed to build HTTP client")?;

    // Public-client PKCE flow: no client secret.
    let client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_auth_uri(
            AuthUrl::new(
                "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
            )
            .context("Invalid auth URL")?,
        )
        .set_token_uri(
            TokenUrl::new(
                "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
            )
            .context("Invalid token URL")?,
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("http://127.0.0.1:{OAUTH_PORT}"))
                .context("Invalid redirect URL")?,
        );

    let scopes: Vec<String> = SCOPES.iter().map(|s| (*s).to_string()).collect();

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let mut auth = client.authorize_url(CsrfToken::new_random);
    for s in &scopes {
        auth = auth.add_scope(Scope::new(s.clone()));
    }
    // `select_account` prompts the user to pick a Microsoft account, which is
    // helpful when they have both a personal and a work account.
    let (auth_url, _csrf_token) = auth
        .add_extra_param("prompt", "select_account")
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
            anyhow!("OAuth error from Microsoft: {error}")
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

    info!("fetching Microsoft Graph userinfo");
    let userinfo = fetch_userinfo(&access_token).await.map_err(|e| {
        error!("userinfo fetch failed: {e:#}");
        anyhow!("Failed to retrieve your Microsoft account details. Please try again.")
    })?;
    info!(email_len = userinfo.email.len(), "userinfo fetched");

    Ok(OAuthTokens {
        access_token,
        refresh_token,
        expires_in,
        provider_account_id: userinfo.id,
        email: userinfo.email,
        scopes,
    })
}

struct MicrosoftUserInfo {
    id: String,
    email: String,
}

#[instrument(skip(access_token))]
async fn fetch_userinfo(access_token: &str) -> Result<MicrosoftUserInfo> {
    const USERINFO_URL: &str = "https://graph.microsoft.com/v1.0/me";

    #[derive(Deserialize)]
    struct UserInfo {
        id: String,
        mail: Option<String>,
        #[serde(rename = "userPrincipalName")]
        user_principal_name: Option<String>,
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
        anyhow!("Unexpected response format from Microsoft. Please try again.")
    })?;

    let email = info
        .mail
        .or(info.user_principal_name)
        .unwrap_or_else(|| info.id.clone());

    Ok(MicrosoftUserInfo {
        id: info.id,
        email,
    })
}
