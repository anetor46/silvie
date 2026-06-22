//! Shared HTTP client for the Travelport Stays v11 API.
//!
//! One `TravelportClient` is constructed per chat turn (in `build_tool_auth`)
//! and shared by reference across every Travelport tool the LLM invokes. The
//! bearer-token cache lives here so a single search → details → availability
//! → book chain only authenticates once.
//!
//! Wire-level request / response types live in [`models`](super::models);
//! this module deals only with auth, base URL, and HTTP transport.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument, warn};

use super::error::{make_api_error, TravelportError};
use super::models::*;

// Travelport JSON API hosts. Auth and the API live on different subdomains
// of `travelport.net` (per the developer portal — note `.net`, not `.com`).
const TOKEN_URL_DEV: &str = "https://auth.pp.travelport.net/oauth/token";
const TOKEN_URL_PROD: &str = "https://auth.travelport.net/oauth/token";

const DEV_BASE_URL: &str = "https://api.pp.travelport.net";
const PROD_BASE_URL: &str = "https://api.travelport.net";

/// Selected Travelport environment. `Dev` (pre-prod / sandbox) is the
/// default; flipping to `Prod` is a single env-var change.
#[derive(Debug, Clone, Copy)]
pub enum TravelportEnv {
    Dev,
    Prod,
}

impl TravelportEnv {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "prod" | "production" | "live" => Self::Prod,
            "dev" | "preprod" | "pre-prod" | "sandbox" | "" => Self::Dev,
            other => {
                warn!("unknown TRAVELPORT_ENV='{other}', defaulting to dev");
                Self::Dev
            }
        }
    }

    pub fn base_url(self) -> &'static str {
        match self {
            TravelportEnv::Dev => DEV_BASE_URL,
            TravelportEnv::Prod => PROD_BASE_URL,
        }
    }

    pub fn token_url(self) -> &'static str {
        match self {
            TravelportEnv::Dev => TOKEN_URL_DEV,
            TravelportEnv::Prod => TOKEN_URL_PROD,
        }
    }
}

#[derive(Clone)]
struct CachedToken {
    bearer: String,
    expires_at: Instant,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Clone)]
pub struct TravelportClient {
    base_url: String,
    token_url: String,
    pcc: String,
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
    http: reqwest::Client,
    token: Arc<Mutex<Option<CachedToken>>>,
}

impl std::fmt::Debug for TravelportClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TravelportClient")
            .field("base_url", &self.base_url)
            .field("pcc", &self.pcc)
            .field("client_id_len", &self.client_id.len())
            .field("username_len", &self.username.len())
            .finish()
    }
}

pub struct TravelportClientCreds {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
    pub env: TravelportEnv,
    pub pcc: String,
}

impl TravelportClient {
    pub fn new(creds: TravelportClientCreds) -> Self {
        Self {
            base_url: creds.env.base_url().to_string(),
            token_url: creds.env.token_url().to_string(),
            pcc: creds.pcc,
            client_id: creds.client_id,
            client_secret: creds.client_secret,
            username: creds.username,
            password: creds.password,
            http: reqwest::Client::new(),
            token: Arc::new(Mutex::new(None)),
        }
    }

    /// Fetch a bearer token, caching the result until `expires_in - 60s`.
    async fn token(&self) -> Result<String, TravelportError> {
        // Fast path: cached token still valid.
        {
            let guard = self.token.lock().await;
            if let Some(t) = guard.as_ref() {
                if t.expires_at > Instant::now() {
                    return Ok(t.bearer.clone());
                }
            }
        }

        debug!("fetching Travelport access token from {}", self.token_url);
        // Travelport's token endpoint uses the OAuth2 **password** grant —
        // the four credentials go in the form body together. Token is valid
        // for 24h per the developer portal.
        let resp = self
            .http
            .post(&self.token_url)
            .header("Accept", "application/json")
            .form(&[
                ("grant_type", "password"),
                ("username", self.username.as_str()),
                ("password", self.password.as_str()),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
            ])
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(TravelportError::Auth(format!(
                "token endpoint returned HTTP {status}: {body}"
            )));
        }
        let parsed: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| TravelportError::Auth(format!("failed to parse token response: {e}")))?;

        // Default 24h per the developer portal. Subtract 60s skew.
        let ttl = parsed.expires_in.unwrap_or(86_400).saturating_sub(60);
        let cached = CachedToken {
            bearer: parsed.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(ttl),
        };
        *self.token.lock().await = Some(cached);
        info!(ttl_secs = ttl, "Travelport access token cached");
        Ok(parsed.access_token)
    }

    async fn post<B>(&self, path: &str, body: &B) -> Result<String, TravelportError>
    where
        B: serde::Serialize + ?Sized,
    {
        let token = self.token().await?;
        let resp = self
            .http
            .post(format!("{}{path}", self.base_url))
            .bearer_auth(token)
            .header("XAUTH_TRAVELPORT_ACCESSGROUP", &self.pcc)
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        debug!("POST {path} -> {status}");
        if !status.is_success() {
            return Err(make_api_error(status, text));
        }
        Ok(text)
    }

    async fn get(&self, path: &str) -> Result<String, TravelportError> {
        let token = self.token().await?;
        let resp = self
            .http
            .get(format!("{}{path}", self.base_url))
            .bearer_auth(token)
            .header("XAUTH_TRAVELPORT_ACCESSGROUP", &self.pcc)
            .header("Accept", "application/json")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        debug!("GET {path} -> {status}");
        if !status.is_success() {
            return Err(make_api_error(status, text));
        }
        Ok(text)
    }

    async fn delete(&self, path: &str) -> Result<String, TravelportError> {
        let token = self.token().await?;
        let resp = self
            .http
            .delete(format!("{}{path}", self.base_url))
            .bearer_auth(token)
            .header("XAUTH_TRAVELPORT_ACCESSGROUP", &self.pcc)
            .header("Accept", "application/json")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        debug!("DELETE {path} -> {status}");
        if !status.is_success() {
            return Err(make_api_error(status, text));
        }
        Ok(text)
    }
}

// ── Hotel endpoints ─────────────────────────────────────────────────────────
//
// Paths are derived from the public Travelport v11 docs ToC and named
// `/11/hotel/...` to match the documented prefix. Exact URL segments must be
// verified against developer-portal sample payloads when the first request
// is made — if a path 404s, only the constants in this section need updating.

const PATH_SEARCH_LOCATION: &str = "/11/hotel/search/location";
const PATH_DETAILS_TEMPLATE: &str = "/11/hotel/properties/{property_id}";
const PATH_AVAILABILITY: &str = "/11/hotel/offers/availability";
const PATH_RESERVATIONS: &str = "/11/hotel/reservations";
const PATH_RESERVATION_TEMPLATE: &str = "/11/hotel/reservations/{reservation_id}";

impl TravelportClient {
    #[instrument(skip(self), fields(location = req.location_code, check_in = req.check_in, check_out = req.check_out))]
    pub(super) async fn search_by_location(
        &self,
        req: SearchByLocationReq<'_>,
    ) -> Result<SearchResp, TravelportError> {
        let body = self.post(PATH_SEARCH_LOCATION, &req).await?;
        serde_json::from_str(&body).map_err(|e| TravelportError::Parse(format!("{e}: {body}")))
    }

    #[instrument(skip(self), fields(property_id))]
    pub(super) async fn hotel_details(&self, property_id: &str) -> Result<DetailsResp, TravelportError> {
        let path = PATH_DETAILS_TEMPLATE.replace("{property_id}", property_id);
        let body = self.get(&path).await?;
        serde_json::from_str(&body).map_err(|e| TravelportError::Parse(format!("{e}: {body}")))
    }

    #[instrument(skip(self), fields(property_id = req.property_id, check_in = req.check_in, check_out = req.check_out))]
    pub(super) async fn availability(
        &self,
        req: AvailabilityReq<'_>,
    ) -> Result<AvailabilityResp, TravelportError> {
        let body = self.post(PATH_AVAILABILITY, &req).await?;
        serde_json::from_str(&body).map_err(|e| TravelportError::Parse(format!("{e}: {body}")))
    }

    #[instrument(skip(self, req), fields(property_id = req.property_id, offer_id = req.offer_id))]
    pub(super) async fn book(&self, req: BookReq<'_>) -> Result<BookResp, TravelportError> {
        let body = self.post(PATH_RESERVATIONS, &req).await?;
        serde_json::from_str(&body).map_err(|e| TravelportError::Parse(format!("{e}: {body}")))
    }

    #[instrument(skip(self), fields(reservation_id))]
    pub(super) async fn retrieve(
        &self,
        reservation_id: &str,
    ) -> Result<ReservationResp, TravelportError> {
        let path = PATH_RESERVATION_TEMPLATE.replace("{reservation_id}", reservation_id);
        let body = self.get(&path).await?;
        serde_json::from_str(&body).map_err(|e| TravelportError::Parse(format!("{e}: {body}")))
    }

    #[instrument(skip(self), fields(reservation_id))]
    pub(super) async fn cancel(&self, reservation_id: &str) -> Result<CancelResp, TravelportError> {
        let path = PATH_RESERVATION_TEMPLATE.replace("{reservation_id}", reservation_id);
        let body = self.delete(&path).await?;
        if body.trim().is_empty() {
            // Some Travelport endpoints respond 204; synthesize a minimal
            // success payload so callers can treat the result uniformly.
            return Ok(CancelResp {
                status: Some("cancelled".into()),
                refund_amount: None,
                currency: None,
            });
        }
        serde_json::from_str(&body).map_err(|e| TravelportError::Parse(format!("{e}: {body}")))
    }
}
