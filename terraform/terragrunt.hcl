locals {
  project       = "silvie"
  region        = get_env("TG_REGION", "us-east-2")
  env           = get_env("TG_ENV", "prd")
  account_id    = 269494629935
  params        = yamldecode(
    templatefile(
      "config/${local.env}.yaml",
      { env = local.env, region = local.region }
    )
  )
  tags = {
    environment = local.env
    project     = local.project
  }
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
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }
}

provider "aws" {
  region              = "${local.region}"
  allowed_account_ids = ["${local.account_id}"]
  default_tags {
    tags = {
      environment = "${local.tags.environment}"
      project     = "${local.tags.project}"
    }
  }
}
EOF
}

inputs = {
  env     = local.env
  project = local.project
  region  = local.region
  params  = local.params
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
