//! Shared data types for the Auth0 integration.

use serde::{Deserialize, Serialize};

/// The user record returned from Auth0 and persisted in the OS keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
}

/// Token set returned from any Auth0 grant. Short-lived — consumed immediately
/// by the caller to build `StoredAuthCredentials`.
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) at which the access token expires.
    pub expires_at: Option<i64>,
}

/// What we actually persist in the OS keychain (JSON-encoded).
#[derive(Serialize, Deserialize)]
pub struct StoredAuthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub user: AuthUser,
}
