mod auth;
mod payment;

use auth::ConnectedAccount;
use payment::StoredPaymentMethod;
use tauri::State;
use tracing_subscriber::{fmt, EnvFilter};

pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[tauri::command]
async fn start_google_oauth(
    app: tauri::AppHandle,
    config: State<'_, OAuthConfig>,
) -> Result<ConnectedAccount, String> {
    if config.client_id.is_empty() {
        return Err("GOOGLE_CLIENT_ID is not configured. Add it to your .env file.".to_string());
    }
    if config.client_secret.is_empty() {
        return Err(
            "GOOGLE_CLIENT_SECRET is not configured. Add it to your .env file.".to_string(),
        );
    }
    auth::google_oauth_flow(&app, &config.client_id, &config.client_secret)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_google_calendar_account() -> Option<ConnectedAccount> {
    auth::load_google_account()
}

#[tauri::command]
fn disconnect_google_calendar() -> Result<(), String> {
    auth::remove_google_account().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_google_access_token(config: State<'_, OAuthConfig>) -> Result<Option<String>, String> {
    tracing::info!("get_google_access_token called");
    auth::get_fresh_access_token(&config.client_id, &config.client_secret)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn store_payment_method(data: StoredPaymentMethod) -> Result<(), String> {
    payment::store_payment_method(&data).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_payment_method() -> Option<StoredPaymentMethod> {
    payment::load_payment_method()
}

#[tauri::command]
fn remove_payment_method() -> Result<(), String> {
    payment::remove_payment_method().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise tracing → logs appear in the terminal running `pnpm tauri dev`.
    // Override level with RUST_LOG env var, e.g. RUST_LOG=silvie=debug.
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("silvie=debug")),
        )
        .with_target(false)
        .init();

    dotenvy::dotenv().ok();

    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();

    tauri::Builder::default()
        .manage(OAuthConfig {
            client_id,
            client_secret,
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_oauth::init())
        .invoke_handler(tauri::generate_handler![
            start_google_oauth,
            get_google_calendar_account,
            disconnect_google_calendar,
            get_google_access_token,
            store_payment_method,
            get_payment_method,
            remove_payment_method,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
