mod auth;
mod auth0;

use auth::OAuthTokens;
use auth0::{Auth0Config, AuthUser};
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
) -> Result<OAuthTokens, String> {
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
async fn auth0_login(
    cfg: State<'_, Auth0Config>,
    email: String,
    password: String,
) -> Result<AuthUser, String> {
    auth0::login_password(&cfg, &email, &password)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn auth0_signup(
    cfg: State<'_, Auth0Config>,
    email: String,
    password: String,
    name: String,
) -> Result<AuthUser, String> {
    auth0::signup(&cfg, &email, &password, &name)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn auth0_request_password_reset(
    cfg: State<'_, Auth0Config>,
    email: String,
) -> Result<(), String> {
    auth0::request_password_reset(&cfg, &email)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn auth0_login_browser(
    app: tauri::AppHandle,
    cfg: State<'_, Auth0Config>,
    connection: Option<String>,
) -> Result<AuthUser, String> {
    auth0::login_browser(&app, &cfg, connection.as_deref())
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn auth0_get_user() -> Option<AuthUser> {
    auth0::load_user()
}

#[tauri::command]
async fn auth0_get_access_token(cfg: State<'_, Auth0Config>) -> Result<Option<String>, String> {
    auth0::get_fresh_access_token(&cfg)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn auth0_logout() -> Result<(), String> {
    auth0::logout().map_err(|e| e.to_string())
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
    let auth0_config = Auth0Config {
        domain: std::env::var("AUTH0_DOMAIN").unwrap_or_default(),
        client_id: std::env::var("AUTH0_CLIENT_ID").unwrap_or_default(),
        audience: std::env::var("AUTH0_AUDIENCE").unwrap_or_default(),
        connection: std::env::var("AUTH0_CONNECTION").unwrap_or_default(),
    };

    tauri::Builder::default()
        .manage(OAuthConfig {
            client_id,
            client_secret,
        })
        .manage(auth0_config)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_oauth::init())
        .invoke_handler(tauri::generate_handler![
            start_google_oauth,
            auth0_login,
            auth0_signup,
            auth0_request_password_reset,
            auth0_login_browser,
            auth0_get_user,
            auth0_get_access_token,
            auth0_logout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
