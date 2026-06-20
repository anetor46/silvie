locals {
  project = "silvie"
  env     = get_env("TG_ENV", "dev")
  params  = yamldecode(
    templatefile(
      "config/${local.env}.yaml",
      { env = local.env }
    )
  )
  auth0_domain        = get_env("AUTH0_DOMAIN")
  auth0_client_id     = get_env("AUTH0_TF_CLIENT_ID")
  auth0_client_secret = get_env("AUTH0_TF_CLIENT_SECRET")
  // Shared with src-tauri/.env — same Google OAuth client used for Calendar.
  google_client_id     = get_env("GOOGLE_CLIENT_ID")
  google_client_secret = get_env("GOOGLE_CLIENT_SECRET")
}

remote_state {
  backend = "local"
  generate = {
    path      = "backend.tf"
    if_exists = "overwrite"
  }
  config = {
    path = "${get_parent_terragrunt_dir()}/state/${local.env}/terraform.tfstate"
  }
}

generate "provider" {
  path      = "provider.tf"
  if_exists = "overwrite"
  contents  = <<EOF
terraform {
  required_version = ">= 1.11"

  required_providers {
    auth0 = {
      source  = "auth0/auth0"
      version = "1.41.0"
    }
  }
}

provider "auth0" {
  domain        = "${local.auth0_domain}"
  client_id     = "${local.auth0_client_id}"
  client_secret = "${local.auth0_client_secret}"
}

EOF
}

inputs = {
  env                  = local.env
  auth0_domain         = local.auth0_domain
  auth0_tf_client_id   = local.auth0_client_id
  google_client_id     = local.google_client_id
  google_client_secret = local.google_client_secret
  params               = local.params
}

// When switching environments each module needs to be reconfigured
terraform {
  extra_arguments "init_reconfigure" {
    commands = [
      "init"
    ]
    arguments = [
      "-reconfigure"
    ]
  }
}
