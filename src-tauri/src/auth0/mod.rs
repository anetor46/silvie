//! Auth0 integration. Sub-modules:
//!
//! - `types`    — `AuthUser`, `TokenSet`, `StoredAuthCredentials`
//! - `keychain` — OS-keychain reads/writes (persist, load_user, logout, refresh)
//! - `client`   — Auth0 HTTP flows (password, signup, reset, browser PKCE)

mod client;
mod keychain;
mod types;

pub use client::{login_browser, login_password, request_password_reset, signup};
pub use keychain::{get_fresh_access_token, load_user, logout};
pub use types::AuthUser;
