// ── Primary database connection — username/password auth for Silvie users.
resource "auth0_connection" "database" {
  name     = var.params.database_connection.name
  strategy = "auth0"

  options {
    password_policy = var.params.database_connection.password_policy
    password_complexity_options {
      min_length = var.params.database_connection.password_min_length
    }
    brute_force_protection = true
    disable_signup         = false
    requires_username      = false
  }
}

resource "auth0_connection_clients" "database_clients" {
  connection_id   = auth0_connection.database.id
  enabled_clients = [auth0_client.silvie_app.id]
}

// ── Google social connection — "Continue with Google" on the login screen.
//
// Reuses the SAME Google OAuth client that the app uses for Calendar API
// access (GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET in src-tauri/.env).
//
// Required Google Cloud Console setup — add BOTH redirect URIs to the
// OAuth client's "Authorized redirect URIs":
//   1. http://localhost:{port}                          — Tauri loopback (Calendar)
//   2. https://${var.auth0_domain}/login/callback       — Auth0 social login
resource "auth0_connection" "google" {
  name     = "google-oauth2"
  strategy = "google-oauth2"

  options {
    client_id     = var.google_client_id
    client_secret = var.google_client_secret
    scopes        = ["email", "profile"]
  }
}

resource "auth0_connection_clients" "google_clients" {
  connection_id   = auth0_connection.google.id
  enabled_clients = [auth0_client.silvie_app.id]
}

// ── Disable Auth0's default "Username-Password-Authentication" connection
// for this app. The tenant comes with this default DB connection enabled on
// every new application; we manage our own (`auth0_connection.database`) so
// the default is redundant and clutters Universal Login.
//
// Note: this is safe for a single-app tenant. If you ever add another app
// outside Terraform's control to this tenant, it'll also lose access to the
// default connection — manage it via Terraform or stop using this resource.
data "auth0_connection" "default_database" {
  name = "Username-Password-Authentication"
}

resource "auth0_connection_clients" "default_database_disabled" {
  connection_id   = data.auth0_connection.default_database.id
  enabled_clients = []
}
