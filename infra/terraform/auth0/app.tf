// Native application — the Silvie desktop (Tauri) client.
// Uses Authorization Code with PKCE; no client secret stored in the app.
resource "auth0_client" "silvie_app" {
  name            = var.params.native_app.name
  description     = var.params.native_app.description
  app_type        = "native"
  oidc_conformant = true
  is_first_party  = true

  callbacks           = var.params.native_app.callbacks
  allowed_logout_urls = var.params.native_app.logout_urls
  grant_types         = ["authorization_code", "refresh_token"]

  jwt_configuration {
    alg = "RS256"
  }

  refresh_token {
    rotation_type                = var.params.native_app.refresh_token.rotation_type
    expiration_type              = var.params.native_app.refresh_token.expiration_type
    token_lifetime               = var.params.native_app.refresh_token.token_lifetime
    idle_token_lifetime          = var.params.native_app.refresh_token.idle_token_lifetime
    leeway                       = 0
    infinite_token_lifetime      = false
    infinite_idle_token_lifetime = false
  }
}

// Auth method — PKCE flow, no secret.
resource "auth0_client_credentials" "sivie_app_pkce" {
  client_id             = auth0_client.silvie_app.id
  authentication_method = "none"
}
