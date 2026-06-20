// The Terraform M2M client needs an explicit grant on our newly-created API
// for Auth0 to allow subsequent Management-API reads of that resource.
//
// Without this, `terragrunt plan/apply` fails on refresh with:
//   "Client X is not authorized to access resource server https://api.silvie.app/dev.
//    You need to create a client-grant associated to this API."
//
// The Auth0 dashboard's "Create API" wizard adds this grant automatically;
// the Terraform provider does not, so we declare it explicitly.
resource "auth0_client_grant" "tf_to_api" {
  client_id = var.auth0_tf_client_id
  audience  = auth0_resource_server.api.identifier
  scopes    = []
}
