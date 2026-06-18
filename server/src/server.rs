//! Wires up the HTTP server: routes, CORS, shared state, graceful shutdown.

use std::sync::Arc;

use anyhow::Result;
use poem::{
    get,
    handler,
    listener::TcpListener,
    middleware::{AddData, Cors},
    post, EndpointExt, Route, Server,
};
use tracing::info;

use crate::{chat::chat_handler, llm::LlmClient};

#[handler]
fn health() -> &'static str {
    "OK"
}

pub async fn run(api_key: String, port: u16) -> Result<()> {
    let llm = Arc::new(LlmClient::new(&api_key));

    let cors = Cors::new()
        .allow_origin("http://localhost:1420") // Tauri dev URL
        .allow_origin("tauri://localhost")     // macOS / Linux prod webview
        .allow_origin("https://tauri.localhost") // Windows prod webview
        .allow_methods(["GET", "POST", "OPTIONS"])
        .allow_headers(["content-type"]);

    let app = Route::new()
        .at("/health", get(health))
        .at("/chat", post(chat_handler))
        .with(AddData::new(llm))
        .with(cors);

    let addr = format!("127.0.0.1:{port}");
    info!("silvie-server listening on http://{addr}");

    // TODO(deploy): the server is currently loopback-only and unauthenticated.
    // Before exposing it to the network, add per-user auth + rate limiting.
    Server::new(TcpListener::bind(&addr)).run(app).await?;

    Ok(())
}
