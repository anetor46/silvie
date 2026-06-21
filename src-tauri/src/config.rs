//! Centralised environment-variable reading. `Config::from_env()` runs once
//! at startup in `run()`. No other module calls `std::env::var`.

use anyhow::{anyhow, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub auth0: Auth0Config,
    pub google_oauth: Option<GoogleOAuthConfig>,
    pub outlook_oauth: Option<OutlookOAuthConfig>,
}

#[derive(Debug, Clone)]
pub struct Auth0Config {
    pub domain: String,
    pub client_id: String,
    pub audience: String,
    pub connection: String,
}

impl Auth0Config {
    /// Return an error if any required field is empty. Used by auth flows
    /// that need the full config to be present before making HTTP calls.
    pub fn ensure_configured(&self) -> Result<()> {
        if self.domain.is_empty()
            || self.client_id.is_empty()
            || self.audience.is_empty()
            || self.connection.is_empty()
        {
            return Err(anyhow!(
                "Auth0 is not configured. Set AUTH0_DOMAIN, AUTH0_CLIENT_ID, \
                 AUTH0_AUDIENCE and AUTH0_CONNECTION in src-tauri/.env."
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone)]
pub struct OutlookOAuthConfig {
    pub client_id: String,
}

impl Config {
    /// Load and validate all configuration from environment variables.
    /// Fails immediately and clearly if any required variable is missing.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            auth0: Auth0Config {
                domain: require("AUTH0_DOMAIN")?,
                client_id: require("AUTH0_CLIENT_ID")?,
                audience: require("AUTH0_AUDIENCE")?,
                connection: require("AUTH0_CONNECTION")?,
            },
            google_oauth: optional_pair("GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET").map(
                |(client_id, client_secret)| GoogleOAuthConfig {
                    client_id,
                    client_secret,
                },
            ),
            outlook_oauth: optional("OUTLOOK_CLIENT_ID")
                .map(|client_id| OutlookOAuthConfig { client_id }),
        })
    }
}

fn require(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow!("{name} is not set. Add it to src-tauri/.env (see .env.example).")
        })
}

fn optional(name: &str) -> Option<String> {
    env::var(name).ok().filter(|s| !s.is_empty())
}

fn optional_pair(a: &str, b: &str) -> Option<(String, String)> {
    match (optional(a), optional(b)) {
        (Some(va), Some(vb)) => Some((va, vb)),
        _ => None,
    }
}
