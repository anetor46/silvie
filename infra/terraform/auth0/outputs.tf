// Values the application needs in its .env file.
// Read them with: `terragrunt output` from this directory.

output "auth0_domain" {
  description = "Auth0 tenant domain — set as AUTH0_DOMAIN in the app .env"
  value       = var.auth0_domain
}

output "auth0_audience" {
  description = "API audience — set as AUTH0_AUDIENCE in server/.env"
  value       = auth0_resource_server.api.identifier
}

output "auth0_client_id" {
  description = "Desktop application client ID — set as AUTH0_CLIENT_ID for the Tauri app"
  value       = auth0_client.silvie_app.client_id
}

output "auth0_database_connection_name" {
  description = "Database connection name (passed to /authorize via the connection parameter, if needed)"
  value       = auth0_connection.database.name
}
