// Resource Server — represents the silvie backend API.
// The backend validates incoming JWTs against this audience.
resource "auth0_resource_server" "api" {
  name                                            = var.params.api.name
  identifier                                      = var.params.api.identifier
  signing_alg                                     = "RS256"
  token_lifetime                                  = var.params.api.token_lifetime
  allow_offline_access                            = true
  skip_consent_for_verifiable_first_party_clients = true
  enforce_policies                                = false
  token_dialect                                   = "access_token"
}
