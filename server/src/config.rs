//! Centralised environment-variable reading. The rest of the crate is
//! parameterised by [`Config`] — no other module calls `std::env::var`.
//!
//! Boundaries:
//!   * `Config::from_env()` runs once at startup (in `main`).
//!   * Required vars (DATABASE_URL, AUTH0_*, GEMINI_API_KEY) fail-fast with
//!     a clear message naming the missing var.
//!   * Optional groups (Stripe, Google OAuth, Travelport) collapse to `None`
//!     when any var in the group is missing. Endpoints that need them
//!     handle the `None` case explicitly (e.g. 503).

use std::env;

use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub gemini_api_key: String,
    pub auth0: Auth0Config,
    pub google_oauth: Option<GoogleOAuthCredentials>,
    pub outlook_oauth: Option<OutlookOAuthCredentials>,
    pub stripe: Option<StripeConfig>,
    pub travelport: Option<TravelportCredentials>,
}

#[derive(Debug, Clone)]
pub struct Auth0Config {
    pub domain: String,
    pub audience: String,
}

#[derive(Debug, Clone)]
pub struct GoogleOAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone)]
pub struct OutlookOAuthCredentials {
    pub client_id: String,
}

#[derive(Debug, Clone)]
pub struct StripeConfig {
    pub secret_key: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // wired through in the LlmClient refactor (Slice 3)
pub struct TravelportCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: require("DATABASE_URL")?,
            gemini_api_key: require("GEMINI_API_KEY")?,
            auth0: Auth0Config {
                domain: require("AUTH0_DOMAIN")?,
                audience: require("AUTH0_AUDIENCE")?,
            },
            google_oauth: optional_pair("GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET").map(
                |(client_id, client_secret)| GoogleOAuthCredentials {
                    client_id,
                    client_secret,
                },
            ),
            outlook_oauth: optional("OUTLOOK_CLIENT_ID")
                .map(|client_id| OutlookOAuthCredentials { client_id }),
            stripe: optional("STRIPE_SECRET_KEY").map(|secret_key| StripeConfig { secret_key }),
            travelport: optional_pair("TRAVELPORT_CLIENT_ID", "TRAVELPORT_CLIENT_SECRET").map(
                |(client_id, client_secret)| TravelportCredentials {
                    client_id,
                    client_secret,
                },
            ),
        })
    }
}

fn require(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("{name} is not set. Add it to server/.env (see .env.example)."))
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
