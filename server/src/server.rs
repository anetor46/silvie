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
    api::{
        chat::chat_handler,
        conversations::{
            create_conversation_handler, delete_conversation_handler, get_conversation_handler,
            list_conversations_handler, update_conversation_handler,
        },
        integrations::{
            delete_integration_handler, get_access_token_handler, list_integrations_handler,
            upsert_integration_handler,
        },
        payments::{
            create_user_payment_method_handler, delete_user_payment_method_handler,
            get_user_payment_method_handler, payment_method_handler, payment_setup_handler,
            update_user_billing_handler,
        },
        tool_responses::tool_response_handler,
        user_info::{get_user_info_handler, update_user_info_handler},
        users::{create_user_handler, users_me_handler},
    },
    auth::JwtValidator,
    config::Config,
    db::DbPool,
    llm::LlmClient,
    repos::integrations::IntegrationsConfig,
    services::stripe::PaymentClient,
};

/// Bundled state passed to `run` by `main`. Keeps the function signature flat.
pub struct ServerState {
    pub host: String,
    pub port: u16,
    pub pool: DbPool,
    pub llm: Arc<LlmClient>,
    pub jwt_validator: Arc<JwtValidator>,
    pub integrations_config: Arc<IntegrationsConfig>,
    pub stripe_secret_key: Option<String>,
    pub config: Arc<Config>,
}

#[handler]
fn health() -> &'static str {
    "OK"
}

pub async fn run(state: ServerState) -> Result<()> {
    let ServerState {
        host,
        port,
        pool,
        llm,
        jwt_validator,
        integrations_config,
        stripe_secret_key,
        config,
    } = state;

    let payment: Arc<Option<PaymentClient>> = Arc::new(stripe_secret_key.map(PaymentClient::new));
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
        .at("/chat/tool-responses", post(tool_response_handler))
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
        .at(
            "/users/me/integrations",
            get(list_integrations_handler).post(upsert_integration_handler),
        )
        .at(
            "/users/me/integrations/:id",
            poem::delete(delete_integration_handler),
        )
        .at(
            "/users/me/integrations/:provider/access-token",
            get(get_access_token_handler),
        )
        .at(
            "/users/me/conversations",
            get(list_conversations_handler).post(create_conversation_handler),
        )
        .at(
            "/users/me/conversations/:id",
            get(get_conversation_handler)
                .put(update_conversation_handler)
                .delete(delete_conversation_handler),
        )
        .with(AddData::new(llm))
        .with(AddData::new(payment))
        .with(AddData::new(pool))
        .with(AddData::new(jwt_validator))
        .with(AddData::new(integrations_config))
        .with(AddData::new(config))
        .with(cors);

    let addr = format!("{host}:{port}");
    info!("silvie-server listening on http://{addr}");

    Server::new(TcpListener::bind(&addr)).run(app).await?;

    Ok(())
}
