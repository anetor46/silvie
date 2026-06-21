//! Generic OAuth credential store for third-party services (Google Calendar,
//! Microsoft Outlook, etc.). Tokens are persisted in Postgres; the OAuth
//! client_id/secret used to refresh them lives in env vars on the server.
//!
//! The OAuth handshake itself still happens on the **client** (Tauri opens a
//! browser, captures the redirect via the loopback OAuth helper) — once the
//! initial tokens come back, the client POSTs them to `/users/me/integrations`
//! and the backend takes over storage + refresh.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use diesel::{
    AsChangeset, ExpressionMethods, Insertable, OptionalExtension, QueryDsl, Queryable,
    Selectable, SelectableHelper,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{db::DbPool, schema::integrations};

/// Provider slug for the unified Google integration (Gmail + Calendar).
pub const GOOGLE_PROVIDER: &str = "google";

/// Provider slug for the unified Microsoft Outlook integration (Mail + Calendar).
pub const OUTLOOK_PROVIDER: &str = "outlook";

/// Runtime credentials needed to refresh OAuth tokens for each provider.
pub struct IntegrationsConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub outlook_client_id: String,
}

impl IntegrationsConfig {
    fn ensure_google(&self) -> Result<()> {
        if self.google_client_id.is_empty() || self.google_client_secret.is_empty() {
            return Err(anyhow!(
                "Google OAuth refresh is not configured on the server (set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET)."
            ));
        }
        Ok(())
    }

    fn ensure_outlook(&self) -> Result<()> {
        if self.outlook_client_id.is_empty() {
            return Err(anyhow!(
                "Outlook OAuth refresh is not configured on the server (set OUTLOOK_CLIENT_ID)."
            ));
        }
        Ok(())
    }
}

impl From<&crate::config::Config> for IntegrationsConfig {
    fn from(cfg: &crate::config::Config) -> Self {
        let (google_id, google_secret) = cfg
            .google_oauth
            .as_ref()
            .map(|g| (g.client_id.clone(), g.client_secret.clone()))
            .unwrap_or_default();
        let outlook_id = cfg
            .outlook_oauth
            .as_ref()
            .map(|o| o.client_id.clone())
            .unwrap_or_default();
        Self {
            google_client_id: google_id,
            google_client_secret: google_secret,
            outlook_client_id: outlook_id,
        }
    }
}

// ── Models ──────────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = integrations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Integration {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_account_id: String,
    pub provider_account_email: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expiry: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public-safe view (no tokens) returned by list/upsert endpoints.
#[derive(Serialize, Debug, Clone)]
pub struct IntegrationView {
    pub id: Uuid,
    pub provider: String,
    pub provider_account_id: String,
    pub provider_account_email: Option<String>,
    pub scopes: Vec<String>,
    pub status: String,
    pub token_expiry: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Integration> for IntegrationView {
    fn from(i: Integration) -> Self {
        IntegrationView {
            id: i.id,
            provider: i.provider,
            provider_account_id: i.provider_account_id,
            provider_account_email: i.provider_account_email,
            scopes: i.scopes,
            status: i.status,
            token_expiry: i.token_expiry,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = integrations)]
struct NewIntegration<'a> {
    user_id: Uuid,
    provider: &'a str,
    provider_account_id: &'a str,
    provider_account_email: Option<&'a str>,
    access_token: &'a str,
    refresh_token: Option<&'a str>,
    token_expiry: Option<DateTime<Utc>>,
    scopes: &'a [String],
}

#[derive(AsChangeset)]
#[diesel(table_name = integrations)]
struct IntegrationChanges<'a> {
    provider_account_email: Option<&'a str>,
    access_token: &'a str,
    refresh_token: Option<&'a str>,
    token_expiry: Option<DateTime<Utc>>,
    scopes: &'a [String],
    status: &'a str,
}

// ── Request / response shapes ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UpsertIntegrationRequest {
    pub provider: String,
    pub provider_account_id: String,
    pub provider_account_email: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Seconds until the access token expires (as returned by the provider).
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Serialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub expires_at: Option<DateTime<Utc>>,
}

// ── DB operations ───────────────────────────────────────────────────────────

#[instrument(skip(pool))]
pub async fn list_user_integrations(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Vec<IntegrationView>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let rows: Vec<Integration> = integrations::table
        .filter(integrations::user_id.eq(user_id))
        .select(Integration::as_select())
        .load(&mut conn)
        .await
        .context("Failed to list integrations")?;
    Ok(rows.into_iter().map(IntegrationView::from).collect())
}

#[instrument(skip(pool, req), fields(provider = %req.provider, account_id_len = req.provider_account_id.len()))]
pub async fn upsert_integration(
    pool: &DbPool,
    user_id: Uuid,
    req: &UpsertIntegrationRequest,
) -> Result<IntegrationView> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let token_expiry = req.expires_in.map(|s| Utc::now() + Duration::seconds(s));

    let existing_id: Option<Uuid> = integrations::table
        .filter(integrations::user_id.eq(user_id))
        .filter(integrations::provider.eq(&req.provider))
        .filter(integrations::provider_account_id.eq(&req.provider_account_id))
        .select(integrations::id)
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to look up existing integration")?;

    let row: Integration = if let Some(id) = existing_id {
        let changes = IntegrationChanges {
            provider_account_email: req.provider_account_email.as_deref(),
            access_token: &req.access_token,
            // Only overwrite refresh_token if the caller supplied one. Some
            // providers return a refresh_token only on the first authorization.
            refresh_token: req.refresh_token.as_deref(),
            token_expiry,
            scopes: &req.scopes,
            status: "active",
        };
        let n: usize = diesel::update(integrations::table.filter(integrations::id.eq(id)))
            .set((&changes, integrations::updated_at.eq(diesel::dsl::now)))
            .execute(&mut conn)
            .await
            .context("Failed to update integration")?;
        info!(updated = n, "integration updated");
        integrations::table
            .filter(integrations::id.eq(id))
            .select(Integration::as_select())
            .first(&mut conn)
            .await
            .context("Failed to fetch updated integration")?
    } else {
        let row = NewIntegration {
            user_id,
            provider: &req.provider,
            provider_account_id: &req.provider_account_id,
            provider_account_email: req.provider_account_email.as_deref(),
            access_token: &req.access_token,
            refresh_token: req.refresh_token.as_deref(),
            token_expiry,
            scopes: &req.scopes,
        };
        let inserted: Integration = diesel::insert_into(integrations::table)
            .values(&row)
            .returning(Integration::as_returning())
            .get_result(&mut conn)
            .await
            .context("Failed to insert integration")?;
        info!(id = %inserted.id, "integration created");
        inserted
    };

    Ok(row.into())
}

#[instrument(skip(pool))]
pub async fn delete_integration_by_id(pool: &DbPool, user_id: Uuid, id: Uuid) -> Result<bool> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    let n: usize = diesel::delete(
        integrations::table
            .filter(integrations::id.eq(id))
            .filter(integrations::user_id.eq(user_id)),
    )
    .execute(&mut conn)
    .await
    .context("Failed to delete integration")?;
    info!(rows = n, "integration deleted");
    Ok(n > 0)
}

/// Look up a user's integration for a given provider (returns the first
/// matching active one — for MVP single-account-per-provider).
async fn find_active_for_provider(
    pool: &DbPool,
    user_id: Uuid,
    provider: &str,
) -> Result<Option<Integration>> {
    let mut conn = pool.get().await.context("Failed to get DB connection")?;
    integrations::table
        .filter(integrations::user_id.eq(user_id))
        .filter(integrations::provider.eq(provider))
        .filter(integrations::status.eq("active"))
        .select(Integration::as_select())
        .first(&mut conn)
        .await
        .optional()
        .context("Failed to query integration")
}

/// Returns a fresh access token for the given provider — refreshing via
/// the stored refresh_token if the cached access_token has expired (or is
/// within a 60-second grace window of expiry). Updates the row on refresh.
/// Returns `Ok(None)` if the user has no active integration for that provider.
#[instrument(skip(pool, cfg), fields(provider))]
pub async fn fresh_access_token(
    pool: &DbPool,
    cfg: &IntegrationsConfig,
    user_id: Uuid,
    provider: &str,
) -> Result<Option<AccessTokenResponse>> {
    let integration = match find_active_for_provider(pool, user_id, provider).await? {
        Some(i) => i,
        None => return Ok(None),
    };

    let now = Utc::now();
    let needs_refresh = integration
        .token_expiry
        .map(|exp| now + Duration::seconds(60) >= exp)
        .unwrap_or(true);

    if !needs_refresh {
        debug!("integration access token still valid");
        return Ok(Some(AccessTokenResponse {
            access_token: integration.access_token,
            expires_at: integration.token_expiry,
        }));
    }

    let Some(refresh_token) = integration.refresh_token.as_deref() else {
        warn!(
            integration_id = %integration.id,
            "no refresh token stored — marking integration expired"
        );
        let mut conn = pool.get().await?;
        let _: usize = diesel::update(integrations::table.filter(integrations::id.eq(integration.id)))
            .set((
                integrations::status.eq("expired"),
                integrations::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .await?;
        return Ok(None);
    };

    let refreshed = match provider {
        GOOGLE_PROVIDER => {
            cfg.ensure_google()?;
            refresh_google_token(
                &cfg.google_client_id,
                &cfg.google_client_secret,
                refresh_token,
            )
            .await?
        }
        OUTLOOK_PROVIDER => {
            cfg.ensure_outlook()?;
            refresh_outlook_token(&cfg.outlook_client_id, refresh_token).await?
        }
        _ => return Err(anyhow!("refresh not implemented for provider {provider}")),
    };

    let new_expiry = refreshed
        .expires_in
        .map(|s| Utc::now() + Duration::seconds(s));
    let new_refresh = refreshed.refresh_token.as_deref();

    // Persist whatever the provider returned. Many providers only issue a new
    // refresh token on first auth — preserve the old one if none came back.
    let mut conn = pool.get().await?;
    let _: usize = diesel::update(integrations::table.filter(integrations::id.eq(integration.id)))
        .set((
            integrations::access_token.eq(&refreshed.access_token),
            new_refresh
                .map(|r| integrations::refresh_token.eq(Some(r.to_string())))
                .unwrap_or_else(|| integrations::refresh_token.eq(integration.refresh_token.clone())),
            integrations::token_expiry.eq(new_expiry),
            integrations::status.eq("active"),
            integrations::updated_at.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)
        .await
        .context("Failed to persist refreshed token")?;
    info!("integration access token refreshed");

    Ok(Some(AccessTokenResponse {
        access_token: refreshed.access_token,
        expires_at: new_expiry,
    }))
}

// ── Provider-specific refresh ───────────────────────────────────────────────

struct RefreshedTokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[instrument(skip(client_id, refresh_token))]
async fn refresh_outlook_token(
    client_id: &str,
    refresh_token: &str,
) -> Result<RefreshedTokens> {
    // Public-client token refresh: no client_secret for PKCE desktop apps.
    let http = reqwest::Client::new();
    let resp = http
        .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
        .form(&[
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("scope", "offline_access Mail.ReadWrite Mail.Send Calendars.ReadWrite User.Read"),
        ])
        .send()
        .await
        .context("Failed to call Microsoft token endpoint")?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        error!(%status, "Microsoft token refresh failed: {body}");
        return Err(anyhow!("Microsoft token refresh failed: HTTP {status}"));
    }

    #[derive(Deserialize)]
    struct MsTokenResponse {
        access_token: String,
        #[serde(default)]
        refresh_token: Option<String>,
        #[serde(default)]
        expires_in: Option<i64>,
    }

    let parsed: MsTokenResponse = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse Microsoft token response: {body}"))?;
    Ok(RefreshedTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_in: parsed.expires_in,
    })
}

#[instrument(skip(client_id, client_secret, refresh_token))]
async fn refresh_google_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<RefreshedTokens> {
    let http = reqwest::Client::new();
    let resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .context("Failed to call Google token endpoint")?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        error!(%status, "Google token refresh failed: {body}");
        return Err(anyhow!("Google token refresh failed: HTTP {status}"));
    }
    let parsed: GoogleTokenResponse = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse Google token response: {body}"))?;
    Ok(RefreshedTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_in: parsed.expires_in,
    })
}
