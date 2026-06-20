//! Auth0 HTTP flows: password-realm login, signup, password reset, and
//! browser-based PKCE. Token refresh lives here too (called from keychain.rs).

use anyhow::{anyhow, Context, Result};
use oauth2::{
    basic::BasicClient, url::Url, AuthUrl, AuthorizationCode, ClientId, CsrfToken,
    PkceCodeChallenge, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_oauth::OauthConfig;
use tauri_plugin_opener::OpenerExt;
use tracing::{debug, error, info, instrument, warn};

use crate::config::Auth0Config;

use super::{
    keychain::persist,
    types::{AuthUser, TokenSet},
};

/// Fixed loopback port — must match the callback registered in Auth0 Terraform
/// (`http://localhost:1421/callback` in `infra/terraform/auth0/config/dev.yaml`).
const LOOPBACK_PORT: u16 = 1421;

/// Generic user-facing message when Auth0 is unreachable. The detailed error
/// is always logged — only this safe string reaches the frontend.
const NETWORK_ERROR_USER_MSG: &str =
    "Couldn't reach the authentication service. Please check your network connection and try again.";

// ── Public flows ─────────────────────────────────────────────────────────────

/// Resource Owner Password Grant (Password-Realm). Pure in-app login — no
/// browser window opens.
#[instrument(skip(cfg, password), fields(email_len = email.len()))]
pub async fn login_password(cfg: &Auth0Config, email: &str, password: &str) -> Result<AuthUser> {
    cfg.ensure_configured()?;
    info!("starting Auth0 password-realm login");

    let http = build_http_client()?;
    let token = request_password_grant(&http, cfg, email, password).await?;
    let user = fetch_userinfo(&cfg.domain, &token.access_token).await?;
    info!("userinfo fetched");
    persist(&token, &user)?;
    Ok(user)
}

/// Sign up a new user, then immediately log them in.
#[instrument(skip(cfg, password), fields(email_len = email.len(), name_len = name.len()))]
pub async fn signup(
    cfg: &Auth0Config,
    email: &str,
    password: &str,
    name: &str,
) -> Result<AuthUser> {
    cfg.ensure_configured()?;
    info!("starting Auth0 signup");

    let http = build_http_client()?;

    let body = serde_json::json!({
        "client_id": cfg.client_id,
        "email": email,
        "password": password,
        "name": name,
        "connection": cfg.connection,
    });
    let url = format!("https://{}/dbconnections/signup", cfg.domain);
    info!(%url, "POST signup");

    let resp = http
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!(%url, chain = %format_error_chain(&e), "signup HTTP request failed");
            anyhow!(NETWORK_ERROR_USER_MSG)
        })?;

    let status = resp.status();
    let resp_body = resp
        .text()
        .await
        .context("Failed to read signup response body")?;

    if !status.is_success() {
        error!(%status, body = %resp_body, "signup failed");
        return Err(parse_auth0_error(status, &resp_body, "Signup failed"));
    }
    debug!("signup response OK");

    info!("signup successful, logging in");
    let token = request_password_grant(&http, cfg, email, password).await?;
    let user = fetch_userinfo(&cfg.domain, &token.access_token).await?;
    persist(&token, &user)?;
    Ok(user)
}

/// Trigger a password-reset email. Auth0 always returns 200 regardless of
/// whether the email exists (to prevent user enumeration).
#[instrument(skip(cfg), fields(email_len = email.len()))]
pub async fn request_password_reset(cfg: &Auth0Config, email: &str) -> Result<()> {
    cfg.ensure_configured()?;
    info!("requesting password reset email");

    let http = build_http_client()?;
    let body = serde_json::json!({
        "client_id": cfg.client_id,
        "email": email,
        "connection": cfg.connection,
    });
    let url = format!("https://{}/dbconnections/change_password", cfg.domain);
    info!(%url, "POST change_password");

    let resp = http
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!(%url, chain = %format_error_chain(&e), "password reset HTTP request failed");
            anyhow!(NETWORK_ERROR_USER_MSG)
        })?;

    let status = resp.status();
    let resp_body = resp
        .text()
        .await
        .context("Failed to read password reset response body")?;

    if !status.is_success() {
        error!(%status, body = %resp_body, "password reset failed");
        return Err(parse_auth0_error(status, &resp_body, "Password reset failed"));
    }

    info!("password reset email requested");
    Ok(())
}

/// Browser-based PKCE flow. Used for social logins, MFA, or any case the
/// in-app form can't handle. Opens the system browser to Auth0 Universal Login.
///
/// When `connection` is `Some("google-oauth2")` (or another connection slug),
/// Auth0 skips the Universal Login chooser and goes straight to that provider.
#[instrument(skip(app, cfg))]
pub async fn login_browser(
    app: &AppHandle,
    cfg: &Auth0Config,
    connection: Option<&str>,
) -> Result<AuthUser> {
    cfg.ensure_configured()?;
    info!("starting Auth0 browser login flow");

    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let sender = Arc::new(Mutex::new(Some(tx)));

    let port = tauri_plugin_oauth::start_with_config(
        OauthConfig {
            ports: Some(vec![LOOPBACK_PORT]),
            response: None,
        },
        move |url| {
            debug!("loopback server received redirect");
            if let Ok(mut guard) = sender.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(url);
                }
            }
        },
    )
    .map_err(|e| anyhow!("Failed to start OAuth loopback server on port {LOOPBACK_PORT}: {e}"))?;
    info!(port, "loopback server started");

    let http = build_http_client()?;

    let client = BasicClient::new(ClientId::new(cfg.client_id.clone()))
        .set_auth_uri(
            AuthUrl::new(format!("https://{}/authorize", cfg.domain))
                .context("Invalid Auth0 auth URL")?,
        )
        .set_token_uri(
            TokenUrl::new(format!("https://{}/oauth/token", cfg.domain))
                .context("Invalid Auth0 token URL")?,
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("http://localhost:{port}/callback"))
                .context("Invalid redirect URL")?,
        );

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let mut request = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("offline_access".to_string()))
        .add_extra_param("audience", cfg.audience.clone())
        .set_pkce_challenge(pkce_challenge);
    if let Some(conn) = connection {
        request = request.add_extra_param("connection", conn.to_string());
        info!(connection = conn, "targeting specific Auth0 connection");
    }
    let (auth_url, _csrf_token) = request.url();

    info!("opening system browser to Auth0 Universal Login");
    app.opener()
        .open_url(auth_url.as_str(), None::<&str>)
        .map_err(|e| anyhow!("Failed to open browser: {e}"))?;

    info!("waiting for Auth0 redirect (timeout 5 min)");
    let redirect_url = match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
        Ok(Ok(url)) => url,
        Ok(Err(_)) => {
            warn!("OAuth oneshot channel closed — flow cancelled");
            let _ = tauri_plugin_oauth::cancel(port);
            return Err(anyhow!("Auth0 login was cancelled"));
        }
        Err(_) => {
            warn!("Auth0 login timed out after 5 minutes");
            let _ = tauri_plugin_oauth::cancel(port);
            return Err(anyhow!("Auth0 login timed out. Please try again."));
        }
    };
    let _ = tauri_plugin_oauth::cancel(port);

    let parsed = Url::parse(&redirect_url).context("Failed to parse redirect URL")?;
    let code = parsed
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| {
            let error = parsed
                .query_pairs()
                .find(|(k, _)| k == "error")
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| "unknown error".to_string());
            anyhow!("Auth0 returned error: {error}")
        })?;

    info!(code_len = code.len(), "authorization code received");
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http)
        .await
        .map_err(|e| {
            error!("token exchange failed: {e}");
            anyhow!("Token exchange failed: {e}")
        })?;

    let token = TokenSet {
        access_token: token_response.access_token().secret().to_string(),
        refresh_token: token_response.refresh_token().map(|t| t.secret().to_string()),
        expires_at: token_response
            .expires_in()
            .map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64),
    };

    info!(
        access_token_len = token.access_token.len(),
        has_refresh_token = token.refresh_token.is_some(),
        expires_at = token.expires_at,
        "browser login token exchange succeeded"
    );

    let user = fetch_userinfo(&cfg.domain, &token.access_token).await?;
    persist(&token, &user)?;
    Ok(user)
}

// ── Token refresh (called from keychain.rs) ───────────────────────────────

/// Exchange a refresh token for a fresh access token. Called by
/// `keychain::get_fresh_access_token`; not part of the public Tauri command
/// surface.
pub async fn refresh_access_token(cfg: &Auth0Config, refresh_token: &str) -> Result<TokenSet> {
    let http = build_http_client()?;

    let client = BasicClient::new(ClientId::new(cfg.client_id.clone()))
        .set_auth_uri(
            AuthUrl::new(format!("https://{}/authorize", cfg.domain))
                .context("Invalid Auth0 auth URL")?,
        )
        .set_token_uri(
            TokenUrl::new(format!("https://{}/oauth/token", cfg.domain))
                .context("Invalid Auth0 token URL")?,
        );

    let now = chrono::Utc::now().timestamp();
    let token_response = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(&http)
        .await
        .map_err(|e| {
            error!("token refresh failed: {e}");
            anyhow!("Token refresh failed: {e}")
        })?;

    Ok(TokenSet {
        access_token: token_response.access_token().secret().to_string(),
        refresh_token: token_response.refresh_token().map(|t| t.secret().to_string()),
        expires_at: token_response.expires_in().map(|d| now + d.as_secs() as i64),
    })
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn build_http_client() -> Result<reqwest::Client> {
    reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("Failed to build HTTP client")
}

/// POST /oauth/token with grant_type=password-realm — Auth0's Resource Owner
/// Password Grant variant that targets a specific DB connection (the "realm").
#[instrument(skip(http, cfg, password), fields(email_len = email.len()))]
async fn request_password_grant(
    http: &reqwest::Client,
    cfg: &Auth0Config,
    email: &str,
    password: &str,
) -> Result<TokenSet> {
    let url = format!("https://{}/oauth/token", cfg.domain);
    info!(%url, realm = %cfg.connection, "POST token (grant_type=password-realm)");

    #[derive(Serialize)]
    struct Body<'a> {
        grant_type: &'static str,
        client_id: &'a str,
        username: &'a str,
        password: &'a str,
        audience: &'a str,
        realm: &'a str,
        scope: &'static str,
    }

    let body = Body {
        grant_type: "http://auth0.com/oauth/grant-type/password-realm",
        client_id: &cfg.client_id,
        username: email,
        password,
        audience: &cfg.audience,
        realm: &cfg.connection,
        scope: "openid profile email offline_access",
    };

    let resp = http
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!(%url, chain = %format_error_chain(&e), "token HTTP request failed");
            anyhow!(NETWORK_ERROR_USER_MSG)
        })?;

    let status = resp.status();
    let resp_body = resp
        .text()
        .await
        .context("Failed to read token response body")?;

    if !status.is_success() {
        error!(%status, body = %resp_body, "token request failed");
        return Err(parse_auth0_error(status, &resp_body, "Login failed"));
    }

    #[derive(Deserialize)]
    struct TokenResp {
        access_token: String,
        #[serde(default)]
        refresh_token: Option<String>,
        #[serde(default)]
        expires_in: Option<i64>,
    }

    let parsed: TokenResp = serde_json::from_str(&resp_body)
        .context("Failed to parse Auth0 token response")?;

    let now = chrono::Utc::now().timestamp();
    let expires_at = parsed.expires_in.map(|secs| now + secs);

    info!(
        access_token_len = parsed.access_token.len(),
        has_refresh_token = parsed.refresh_token.is_some(),
        expires_at,
        "password-realm grant succeeded"
    );

    Ok(TokenSet {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_at,
    })
}

#[instrument(skip(access_token))]
async fn fetch_userinfo(domain: &str, access_token: &str) -> Result<AuthUser> {
    #[derive(Deserialize)]
    struct UserInfo {
        sub: String,
        email: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        nickname: Option<String>,
        #[serde(default)]
        picture: Option<String>,
    }

    let url = format!("https://{domain}/userinfo");
    debug!("GET {url}");

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
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
        error!(%status, "userinfo failed: {body}");
        return Err(anyhow!("userinfo returned HTTP {status}: {body}"));
    }

    let info: UserInfo = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse userinfo JSON: {body}"))?;

    let name = info
        .name
        .filter(|s| !s.is_empty())
        .or(info.nickname.filter(|s| !s.is_empty()))
        .unwrap_or_else(|| {
            info.email
                .split('@')
                .next()
                .unwrap_or(&info.email)
                .to_string()
        });

    Ok(AuthUser {
        sub: info.sub,
        email: info.email,
        name,
        picture: info.picture,
    })
}

/// Walk an error's `source()` chain and join with " → ". reqwest's Display
/// only shows the top-level message; the real cause is buried in the chain.
fn format_error_chain(e: &(dyn std::error::Error + 'static)) -> String {
    let mut parts = vec![e.to_string()];
    let mut src = e.source();
    while let Some(cause) = src {
        parts.push(cause.to_string());
        src = cause.source();
    }
    parts.join(" → ")
}

/// Extract a user-friendly message from an Auth0 JSON error body, falling
/// back to the HTTP status if the body can't be parsed.
fn parse_auth0_error(
    status: reqwest::StatusCode,
    body: &str,
    fallback: &str,
) -> anyhow::Error {
    #[derive(Deserialize)]
    struct Err1 {
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        error_description: Option<String>,
    }
    #[derive(Deserialize)]
    struct Err2 {
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        message: Option<String>,
    }

    if let Ok(e) = serde_json::from_str::<Err1>(body) {
        if let Some(desc) = e.error_description.filter(|s| !s.is_empty()) {
            return anyhow!("{desc}");
        }
        if let Some(code) = e.error.filter(|s| !s.is_empty()) {
            return anyhow!("{code}");
        }
    }
    if let Ok(e) = serde_json::from_str::<Err2>(body) {
        if let Some(desc) = e.description.filter(|s| !s.is_empty()) {
            return anyhow!("{desc}");
        }
        if let Some(msg) = e.message.filter(|s| !s.is_empty()) {
            return anyhow!("{msg}");
        }
    }
    anyhow!("{fallback} (HTTP {status})")
}
