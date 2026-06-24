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
    access_group: String,
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
            .field("access_group_len", &self.access_group.len())
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
    pub access_group: String,
}

impl TravelportClient {
    pub fn new(creds: TravelportClientCreds) -> Self {
        Self {
            base_url: creds.env.base_url().to_string(),
            token_url: creds.env.token_url().to_string(),
            access_group: creds.access_group,
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
            .header("XAUTH_TRAVELPORT_ACCESSGROUP", &self.access_group)
            .header("Accept", "application/json")
            .header("Accept-Version", "11")
            .header("Content-Version", "11")
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
            .header("XAUTH_TRAVELPORT_ACCESSGROUP", &self.access_group)
            .header("Accept", "application/json")
            .header("Accept-Version", "11")
            .header("Content-Version", "11")
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

    async fn put_empty(&self, path: &str) -> Result<String, TravelportError> {
        let token = self.token().await?;
        let resp = self
            .http
            .put(format!("{}{path}", self.base_url))
            .bearer_auth(token)
            .header("XAUTH_TRAVELPORT_ACCESSGROUP", &self.access_group)
            .header("Accept", "application/json")
            .header("Accept-Version", "11")
            .header("Content-Version", "11")
            .header("Content-Length", "0")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        debug!("PUT {path} -> {status}");
        if !status.is_success() {
            return Err(make_api_error(status, text));
        }
        Ok(text)
    }
}

// ── Hotel endpoints ─────────────────────────────────────────────────────────
//
// Paths verified against the public Hotel v11 API reference pages on
// `support.travelport.com/webhelp/JSONAPIs/Hotelv11/`. All hotel endpoints
// sit under `/11/hotel/`.

const PATH_SEARCH_LOCATION: &str = "/11/hotel/search/properties/search";
const PATH_DETAILS: &str = "/11/hotel/search/propertiesdetail";
const PATH_AVAILABILITY: &str = "/11/hotel/availability/catalogofferingshospitality";
const PATH_RESERVATIONS_BUILD: &str = "/11/hotel/book/reservations/build";
const PATH_RESERVATION_TEMPLATE: &str = "/11/hotel/book/reservations/{locator}";
const PATH_RESERVATION_CANCEL_TEMPLATE: &str =
    "/11/hotel/book/reservations/{locator}/canceloffer";

impl TravelportClient {
    #[instrument(skip(self, req))]
    pub(super) async fn search_by_location(
        &self,
        req: SearchByLocationReq,
    ) -> Result<SearchResp, TravelportError> {
        let body = self.post(PATH_SEARCH_LOCATION, &req).await?;
        deserialize_or_log(&body, "search")
    }

    #[instrument(skip(self), fields(chain_code, property_code))]
    pub(super) async fn hotel_details(
        &self,
        chain_code: &str,
        property_code: &str,
    ) -> Result<DetailsResp, TravelportError> {
        let path =
            format!("{PATH_DETAILS}?chainCode={chain_code}&propertyCode={property_code}");
        let body = self.get(&path).await?;
        deserialize_or_log(&body, "details")
    }

    #[instrument(skip(self, req))]
    pub(super) async fn availability(
        &self,
        req: AvailabilityReq,
    ) -> Result<AvailabilityResp, TravelportError> {
        let body = self.post(PATH_AVAILABILITY, &req).await?;
        deserialize_or_log(&body, "availability")
    }

    #[instrument(skip(self, req))]
    pub(super) async fn book(&self, req: BookReq) -> Result<ReservationResp, TravelportError> {
        let body = self.post(PATH_RESERVATIONS_BUILD, &req).await?;
        deserialize_or_log(&body, "book")
    }

    #[instrument(skip(self), fields(aggregator_locator))]
    pub(super) async fn retrieve(
        &self,
        aggregator_locator: &str,
    ) -> Result<ReservationResp, TravelportError> {
        let path = PATH_RESERVATION_TEMPLATE.replace("{locator}", aggregator_locator);
        let body = self.get(&path).await?;
        deserialize_or_log(&body, "retrieve")
    }

    /// Cancel — PUT, with the supplier locator as a query parameter.
    #[instrument(skip(self), fields(aggregator_locator, supplier_locator))]
    pub(super) async fn cancel(
        &self,
        aggregator_locator: &str,
        supplier_locator: &str,
    ) -> Result<ReservationResp, TravelportError> {
        let path = format!(
            "{}?supplierLocator={supplier_locator}",
            PATH_RESERVATION_CANCEL_TEMPLATE.replace("{locator}", aggregator_locator)
        );
        let body = self.put_empty(&path).await?;
        if body.trim().is_empty() {
            return Ok(ReservationResp {
                reservation_response: None,
            });
        }
        deserialize_or_log(&body, "cancel")
    }
}

/// Deserialize a Travelport response into the typed struct, or log the
/// full body and return a user-friendly `UnexpectedResponse`. Used by every
/// endpoint so the deserialize step has uniform diagnostic behaviour: if
/// Travelport ever returns a shape that doesn't match our model, we
/// capture the raw body in the logs once — no recompiling needed to see
/// what came back.
fn deserialize_or_log<T: serde::de::DeserializeOwned>(
    body: &str,
    context: &'static str,
) -> Result<T, TravelportError> {
    serde_json::from_str(body).map_err(|e| {
        super::error::log_and_unexpected(
            context,
            "The hotel supplier returned a response we couldn't read.",
            body,
            &[("deserialize_error", &e.to_string())],
        )
    })
}
