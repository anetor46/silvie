//! OS-keychain persistence for Auth0 credentials. All reads and writes to the
//! keychain go through this module. The rest of the crate never calls `keyring`
//! directly.

use anyhow::{anyhow, Context, Result};
use tracing::{debug, info, warn};

use super::types::{AuthUser, StoredAuthCredentials, TokenSet};

const KEYRING_SERVICE: &str = "com.silvie";
const KEYRING_ACCOUNT: &str = "auth0";

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| anyhow!("Keyring unavailable: {e}"))
}

/// Write a fresh `TokenSet` + `AuthUser` to the OS keychain, replacing any
/// previously stored credentials.
pub fn persist(token: &TokenSet, user: &AuthUser) -> Result<()> {
    let payload = serde_json::to_string(&StoredAuthCredentials {
        access_token: token.access_token.clone(),
        refresh_token: token.refresh_token.clone(),
        expires_at: token.expires_at,
        user: user.clone(),
    })?;
    entry()?
        .set_password(&payload)
        .map_err(|e| anyhow!("Failed to store credentials: {e}"))?;
    info!("Auth0 credentials stored in keychain");
    Ok(())
}

/// Return the cached `AuthUser` without any network call. Used to gate the UI
/// on startup. Returns `None` if the user is not logged in.
pub fn load_user() -> Option<AuthUser> {
    let payload = entry().ok()?.get_password().ok()?;
    let creds: StoredAuthCredentials = serde_json::from_str(&payload).ok()?;
    debug!("Auth0 credentials loaded from keychain");
    Some(creds.user)
}

/// Remove all stored credentials. Subsequent calls to `load_user` return `None`.
pub fn logout() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(_) => {
            info!("Auth0 credentials removed from keychain");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            debug!("no Auth0 credentials to remove");
            Ok(())
        }
        Err(e) => Err(anyhow!("Failed to remove credentials: {e}")),
    }
}

/// Return a valid access token, refreshing via the stored refresh-token if the
/// current one is within 60 seconds of expiry.
///
/// Returns `Ok(None)` when the user is not logged in (no keychain entry).
/// Clears the keychain and returns `Ok(None)` when there is no refresh token
/// to use. Returns `Err` only on hard failures (keychain I/O, network errors).
pub async fn get_fresh_access_token(
    cfg: &crate::config::Auth0Config,
) -> Result<Option<String>> {
    let e = entry()?;

    let payload = match e.get_password() {
        Ok(p) => p,
        Err(_) => {
            debug!("no Auth0 credentials stored — user not logged in");
            return Ok(None);
        }
    };

    let mut creds: StoredAuthCredentials =
        serde_json::from_str(&payload).context("Failed to parse stored Auth0 credentials")?;

    let now = chrono::Utc::now().timestamp();
    let needs_refresh = creds.expires_at.map_or(true, |exp| now + 60 >= exp);

    if !needs_refresh {
        debug!(
            expires_in_secs = creds.expires_at.map(|exp| exp - now).unwrap_or(-1),
            "access token still valid"
        );
        return Ok(Some(creds.access_token));
    }

    info!("access token expiring soon, refreshing");
    let refresh_token_str = match creds.refresh_token.as_ref() {
        Some(rt) => rt.clone(),
        None => {
            warn!("no refresh token stored — clearing credentials and forcing re-login");
            let _ = e.delete_credential();
            return Ok(None);
        }
    };

    let new_tokens = super::client::refresh_access_token(cfg, &refresh_token_str).await?;

    creds.access_token = new_tokens.access_token.clone();
    creds.expires_at = new_tokens.expires_at;
    if let Some(new_rt) = new_tokens.refresh_token {
        creds.refresh_token = Some(new_rt);
    }

    info!(
        access_token_len = creds.access_token.len(),
        expires_at = creds.expires_at,
        "token refresh succeeded"
    );

    let updated = serde_json::to_string(&creds)?;
    e.set_password(&updated)
        .map_err(|e| anyhow!("Failed to update credentials after refresh: {e}"))?;

    Ok(Some(creds.access_token))
}
