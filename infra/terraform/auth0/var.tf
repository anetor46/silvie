variable "env" {
  type        = string
  description = "Environment name (dev, prod, etc.)"
}

variable "auth0_domain" {
  type        = string
  description = "Auth0 tenant domain (e.g. silvie.eu.auth0.com)"
}

variable "params" {
  description = "Auth0 configuration loaded from config/<env>.yaml"
  type = object({
    api = object({
      name           = string
      identifier     = string
      token_lifetime = number
    })
    native_app = object({
      name        = string
      description = string
      callbacks   = list(string)
      logout_urls = list(string)
      refresh_token = object({
        rotation_type       = string
        expiration_type     = string
        token_lifetime      = number
        idle_token_lifetime = number
      })
    })
    database_connection = object({
      name                = string
      password_policy     = string
      password_min_length = number
    })
  })
}
