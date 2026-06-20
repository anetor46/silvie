use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use poem::{
    http::StatusCode, Error as PoemError, FromRequest, Request, RequestBody,
    Result as PoemResult,
};
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// Claims we care about from the Auth0-issued access token.
#[derive(Deserialize)]
pub struct Claims {
    pub sub: String,
    // The jsonwebtoken crate validates aud/iss/exp via the `Validation` struct.
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

/// Validates Auth0 JWTs against the tenant's JWKS, caching keys in memory.
/// On a `kid` cache miss (likely key rotation) the JWKS is refetched once.
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

    /// Validate the given JWT against the cached keys. On a `kid` cache miss
    /// the JWKS is refetched once and the lookup retried.
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
        // Cache miss — likely key rotation. Refresh once and retry.
        debug!(kid, "JWKS cache miss, refreshing");
        self.refresh_jwks().await?;
        let cache = self.keys.read().await;
        cache
            .get(kid)
            .cloned()
            .ok_or_else(|| anyhow!("JWKS still has no key for kid={kid} after refresh"))
    }
}

/// Authenticated principal extracted from a validated JWT. Add `principal:
/// Principal` to any handler to require + receive the bearer's identity.
pub struct Principal {
    pub sub: String,
}

impl<'a> FromRequest<'a> for Principal {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> PoemResult<Self> {
        let token = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| PoemError::from_status(StatusCode::UNAUTHORIZED))?;

        let validator = req.data::<Arc<JwtValidator>>().ok_or_else(|| {
            tracing::error!("JwtValidator not present in poem request data");
            PoemError::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        let claims = validator.validate(token).await.map_err(|e| {
            warn!(error = %e, "rejected unauthenticated request");
            PoemError::from_status(StatusCode::UNAUTHORIZED)
        })?;

        Ok(Principal { sub: claims.sub })
    }
}
