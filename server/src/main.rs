mod chat;
mod llm;
mod server;
mod settings;
mod tools;
mod types;

use std::env;
use anyhow::{anyhow, Context, Result};
use tracing_subscriber::EnvFilter;
use crate::settings::Settings;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present (non-fatal if missing).
    let _ = dotenvy::dotenv();
    let env = env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".into());
    let settings = Settings::new(&env)?;

    // Logging — RUST_LOG=info,silvie_server=debug by default.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,silvie_server=debug")),
        )
        .init();

    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow!("GEMINI_API_KEY is not set. Copy server/.env.example to server/.env and fill it in."))?;

    server::run(&api_key, &settings.server.host, settings.server.port)
        .await
        .context("server failed")?;

    Ok(())
}
