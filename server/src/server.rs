use std::sync::Arc;

use anyhow::Result;
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{AddData, Cors},
    post, put, EndpointExt, Route, Server,
};
use tracing::info;

use crate::{
    auth::JwtValidator,
    chat::chat_handler,
    db::DbPool,
    llm::LlmClient,
    payments::{
        create_user_payment_method_handler, delete_user_payment_method_handler,
        get_user_payment_method_handler, payment_method_handler, payment_setup_handler,
        update_user_billing_handler, PaymentClient,
    },
    user_info::{get_user_info_handler, update_user_info_handler},
    users::{create_user_handler, users_me_handler},
};

#[handler]
fn health() -> &'static str {
    "OK"
}

pub async fn run(
    api_key: &str,
    host: &str,
    port: u16,
    pool: DbPool,
    jwt_validator: Arc<JwtValidator>,
) -> Result<()> {
    let llm = Arc::new(LlmClient::new(&api_key));

    let stripe_key = std::env::var("STRIPE_SECRET_KEY").ok();
    let payment: Arc<Option<PaymentClient>> = Arc::new(stripe_key.map(PaymentClient::new));
    if payment.is_some() {
        info!("Stripe payment client initialised");
    }

    let cors = Cors::new()
        .allow_origin("http://127.0.0.1:1420") // Tauri dev URL
        .allow_origin("http://localhost:1420") // Tauri dev URL
        .allow_origin("tauri://localhost") // macOS / Linux prod webview
        .allow_origin("https://tauri.localhost") // Windows prod webview
        .allow_methods(["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allow_headers(["content-type", "authorization"]);

    let app = Route::new()
        .at("/health", get(health))
        .at("/chat", post(chat_handler))
        .at("/payment/setup", post(payment_setup_handler))
        .at("/payment/method", post(payment_method_handler))
        .at("/users", post(create_user_handler))
        .at("/users/me", get(users_me_handler))
        .at("/users/me/info", get(get_user_info_handler).put(update_user_info_handler))
        .at(
            "/users/me/payment-method",
            get(get_user_payment_method_handler)
                .post(create_user_payment_method_handler)
                .delete(delete_user_payment_method_handler),
        )
        .at(
            "/users/me/payment-method/billing",
            put(update_user_billing_handler),
        )
        .with(AddData::new(llm))
        .with(AddData::new(payment))
        .with(AddData::new(pool))
        .with(AddData::new(jwt_validator))
        .with(cors);

    let addr = format!("{host}:{port}");
    info!("silvie-server listening on http://{addr}");

    // TODO(deploy): the server is currently loopback-only.
    // /chat is still unauthenticated; add the Principal extractor when wiring
    // per-user state. /users/* are already protected.
    Server::new(TcpListener::bind(&addr)).run(app).await?;

    Ok(())
}
