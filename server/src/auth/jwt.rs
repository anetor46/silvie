//! Auth0 JWT validation. Caches the JWKS in memory and refreshes once on a
//! `kid` cache miss to handle key rotation.

use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// Claims we care about from the Auth0-issued access token. The rest
/// (`aud`, `iss`, `exp`) are validated by the jsonwebtoken `Validation`.
#[derive(Deserialize)]
pub struct Claims {
    pub sub: String,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<JwkEntry>,
}

#[derive(Deserialize)]
struct JwkEntry {
    kid: String,
    kty: String,
    n: String,
    e: String,
}

pub struct JwtValidator {
    issuer: String,
    audience: String,
    jwks_url: String,
    http_client: reqwest::Client,
    keys: RwLock<HashMap<String, DecodingKey>>,
}

impl JwtValidator {
    #[instrument(skip(domain, audience), fields(domain_len = domain.len(), audience_len = audience.len()))]
    pub async fn new(domain: &str, audience: &str) -> Result<Self> {
        let validator = Self {
            issuer: format!("https://{domain}/"),
            audience: audience.to_string(),
            jwks_url: format!("https://{domain}/.well-known/jwks.json"),
            http_client: reqwest::Client::new(),
            keys: RwLock::new(HashMap::new()),
        };
        validator.refresh_jwks().await?;
        info!("Auth0 JWT validator initialised");
        Ok(validator)
    }

    #[instrument(skip(self))]
    async fn refresh_jwks(&self) -> Result<()> {
        debug!(url = %self.jwks_url, "fetching JWKS");
        let resp = self
            .http_client
            .get(&self.jwks_url)
            .send()
            .await
            .context("Failed to fetch JWKS")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("JWKS endpoint returned HTTP {status}: {body}"));
        }

        let jwks: Jwks = resp.json().await.context("Failed to parse JWKS JSON")?;
        let mut new_map = HashMap::new();
        for entry in jwks.keys {
            if entry.kty != "RSA" {
                continue;
            }
            match DecodingKey::from_rsa_components(&entry.n, &entry.e) {
                Ok(key) => {
                    new_map.insert(entry.kid, key);
                }
                Err(e) => {
                    warn!(error = %e, "failed to parse JWK entry, skipping");
                }
            }
        }
        let count = new_map.len();
        *self.keys.write().await = new_map;
        info!(count, "JWKS cache refreshed");
        Ok(())
    }

    /// Validate the given JWT. On a `kid` cache miss the JWKS is refetched
    /// once and the lookup retried.
    pub async fn validate(&self, token: &str) -> Result<Claims> {
        let header = decode_header(token).context("malformed JWT header")?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow!("JWT header missing 'kid'"))?;
        let key = self.find_key(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.audience]);
        validation.set_issuer(&[&self.issuer]);

        let data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| anyhow!("JWT validation failed: {e}"))?;
        Ok(data.claims)
    }

    async fn find_key(&self, kid: &str) -> Result<DecodingKey> {
        {
            let cache = self.keys.read().await;
            if let Some(k) = cache.get(kid) {
                return Ok(k.clone());
            }
        }
        debug!(kid, "JWKS cache miss, refreshing");
        self.refresh_jwks().await?;
        let cache = self.keys.read().await;
        cache
            .get(kid)
            .cloned()
            .ok_or_else(|| anyhow!("JWKS still has no key for kid={kid} after refresh"))
    }
}
