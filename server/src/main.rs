mod chat;
mod llm;
mod server;
mod types;

use anyhow::{anyhow, Context, Result};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present (non-fatal if missing).
    let _ = dotenvy::dotenv();

    // Logging — RUST_LOG=info,silvie_server=debug by default.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,silvie_server=debug")),
        )
        .init();

    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow!("GEMINI_API_KEY is not set. Copy server/.env.example to server/.env and fill it in."))?;

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    server::run(api_key, port)
        .await
        .context("server failed")?;

    Ok(())
}
