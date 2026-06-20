// Username/password database connection — primary auth method for Silvie users.
// Social connections (Google, etc.) can be added later as additional auth0_connection
// resources without changing this one.
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

// Enable this connection for the desktop client.
resource "auth0_connection_clients" "database_clients" {
  connection_id   = auth0_connection.database.id
  enabled_clients = [auth0_client.silvie_app.id]
}
