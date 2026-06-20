mod auth;
mod chat;
mod conversations;
mod db;
mod integrations;
mod llm;
mod payments;
mod schema;
mod server;
mod settings;
mod tools;
mod types;
mod user_info;
mod users;

use std::env;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use tracing_subscriber::EnvFilter;

use crate::auth::JwtValidator;
use crate::integrations::IntegrationsConfig;
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

    let api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow!("GEMINI_API_KEY is not set. Copy server/.env.example to server/.env and fill it in."))?;
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| anyhow!("DATABASE_URL is not set. Add it to server/.env (see .env.example)."))?;
    let auth0_domain = env::var("AUTH0_DOMAIN")
        .map_err(|_| anyhow!("AUTH0_DOMAIN is not set. Add it to server/.env (see .env.example)."))?;
    let auth0_audience = env::var("AUTH0_AUDIENCE")
        .map_err(|_| anyhow!("AUTH0_AUDIENCE is not set. Add it to server/.env (see .env.example)."))?;

    // Apply any pending schema migrations before opening the pool.
    db::run_pending_migrations(&database_url)
        .await
        .context("database migrations failed")?;

    let pool = db::init_pool(&database_url)
        .await
        .context("failed to initialise database pool")?;

    let jwt_validator = Arc::new(
        JwtValidator::new(&auth0_domain, &auth0_audience)
            .await
            .context("failed to initialise JWT validator")?,
    );

    // OAuth refresh credentials for server-side token refresh (currently Google).
    // Optional — endpoints that need refresh will return a clear error if missing.
    let integrations_config = Arc::new(IntegrationsConfig {
        google_client_id: env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
        google_client_secret: env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
    });

    server::run(
        &api_key,
        &settings.server.host,
        settings.server.port,
        pool,
        jwt_validator,
        integrations_config,
    )
    .await
    .context("server failed")?;

    Ok(())
}
