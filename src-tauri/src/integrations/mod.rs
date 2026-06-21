//! Third-party integration OAuth handshakes. Each provider lives in its own
//! submodule and exposes a single `run()` function that opens the system
//! browser, captures the OAuth redirect on a loopback port, and returns the
//! tokens. Persistence + refresh happen on the backend — this module never
//! touches the keychain.
//!
//! Add a new provider by creating `integrations/<provider>.rs` with the same
//! shape as `google.rs`, then registering a Tauri command in `lib.rs`.

pub mod google;

use serde::{Deserialize, Serialize};

/// Tokens returned to the frontend after a successful OAuth dance. The frontend
/// forwards these to `POST /users/me/integrations` so the backend takes over
/// storage + refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Seconds until the access token expires.
    pub expires_in: Option<i64>,
    /// Provider-stable account identifier (e.g. Google's `sub`).
    pub provider_account_id: String,
    pub email: String,
    pub scopes: Vec<String>,
}
