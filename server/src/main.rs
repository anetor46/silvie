mod api;
mod auth;
mod config;
mod db;
mod error;
mod llm;
mod repos;
mod schema;
mod server;
mod services;
mod settings;
mod tools;
mod types;

use std::env;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::auth::JwtValidator;
use crate::config::Config;
use crate::llm::{ConfirmationRegistry, LlmClient};
use crate::repos::integrations::IntegrationsConfig;
use crate::server::ServerState;
use crate::settings::Settings;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present (non-fatal if missing).
    let _ = dotenvy::dotenv();
    let env_name = env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".into());
    let settings = Settings::new(&env_name)?;

    // Logging — RUST_LOG=info,silvie_server=debug by default.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,silvie_server=debug")),
        )
        .init();

    // Single source of truth for env-derived configuration.
    let config = Config::from_env().context("failed to load configuration")?;
    info!(
        google_oauth_configured = config.google_oauth.is_some(),
        stripe_configured = config.stripe.is_some(),
        travelport_configured = config.travelport.is_some(),
        "configuration loaded"
    );

    // Apply any pending schema migrations before opening the pool.
    db::run_pending_migrations(&config.database_url)
        .await
        .context("database migrations failed")?;

    let pool = db::init_pool(&config.database_url)
        .await
        .context("failed to initialise database pool")?;

    let jwt_validator = Arc::new(
        JwtValidator::new(&config.auth0.domain, &config.auth0.audience)
            .await
            .context("failed to initialise JWT validator")?,
    );

    let integrations_config = Arc::new(IntegrationsConfig::from(&config));

    // Shared between the chat stream task (which registers pending entries
    // when write tools fire) and the /chat/confirmations endpoint (which
    // resolves them when the user clicks Approve / Reject).
    let confirmation_registry = Arc::new(ConfirmationRegistry::new());

    let llm = Arc::new(
        LlmClient::new(
            &config.gemini_api_key,
            &config,
            pool.clone(),
            confirmation_registry.clone(),
        )
        .context("failed to initialise LLM client")?,
    );

    server::run(ServerState {
        host: settings.server.host.clone(),
        port: settings.server.port,
        pool,
        llm,
        jwt_validator,
        integrations_config,
        stripe_secret_key: config.stripe.as_ref().map(|s| s.secret_key.clone()),
        confirmation_registry,
    })
    .await
    .context("server failed")?;

    Ok(())
}
