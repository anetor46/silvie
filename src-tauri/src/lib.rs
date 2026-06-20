mod auth;
mod auth0;
mod config;

use auth::OAuthTokens;
use auth0::AuthUser;
use tauri::State;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::{Auth0Config, Config, GoogleOAuthConfig};

#[tauri::command]
async fn start_google_oauth(
    app: tauri::AppHandle,
    config: State<'_, GoogleOAuthConfig>,
) -> Result<OAuthTokens, String> {
    auth::google_oauth_flow(&app, &config.client_id, &config.client_secret)
        .await
        .map_err(|e| format!("{e:#}"))
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
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn auth0_logout() -> Result<(), String> {
    auth0::logout().map_err(|e| format!("{e:#}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("silvie=debug")),
        )
        .with_target(false)
        .init();

    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|e| {
        // Log and fall back to empty defaults so the app still launches in
        // environments where some vars are intentionally unset (e.g. CI).
        tracing::warn!("Configuration incomplete: {e:#}");
        Config {
            auth0: Auth0Config {
                domain: std::env::var("AUTH0_DOMAIN").unwrap_or_default(),
                client_id: std::env::var("AUTH0_CLIENT_ID").unwrap_or_default(),
                audience: std::env::var("AUTH0_AUDIENCE").unwrap_or_default(),
                connection: std::env::var("AUTH0_CONNECTION").unwrap_or_default(),
            },
            google_oauth: None,
        }
    });

    let google_oauth = config.google_oauth.clone().unwrap_or(GoogleOAuthConfig {
        client_id: String::new(),
        client_secret: String::new(),
    });

    tauri::Builder::default()
        .manage(config.auth0)
        .manage(google_oauth)
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
