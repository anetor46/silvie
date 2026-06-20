module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 6"
  region  = var.region

  name = "${var.project}-${var.region}-${var.env}"
  cidr = var.params.vpc.cidr

  azs             = [for subnet in var.params.vpc.subnets: "${var.region}${subnet.az}"]
  private_subnets = [for subnet in var.params.vpc.subnets: subnet.private_cidr]
  public_subnets  = [for subnet in var.params.vpc.subnets: subnet.public_cidr]

  enable_nat_gateway = true
  enable_vpn_gateway = false
}

module "vc_endpoints" {
  source  = "terraform-aws-modules/vpc/aws//modules/vpc-endpoints"
  version = "~> 6"

  region = var.region

  create_security_group = true
  security_group_name_prefix = "${var.project}-vpc-endpoints-"
  security_group_description = "VPC endpoint security group"
  security_group_rules = {
    ingress_https = {
      description = "HTTPS from VPC"
      cidr_blocks = [module.vpc.vpc_cidr_block]
    }
  }

  endpoints = {
    ecr_dkr = {
      service             = "ecr.dkr"
      private_dns_enabled = true
      subnet_ids          = module.vpc.private_subnets
    }
    logs = {
      service             = "logs"
      private_dns_enabled = true
      subnet_ids          = module.vpc.private_subnets
    }
    stream-logs = {
      service             = "stream-logs"
      private_dns_enabled = true
      subnet_ids          = module.vpc.private_subnets
    }
    monitoring = {
      service             = "monitoring"
      private_dns_enabled = true
      subnet_ids          = module.vpc.private_subnets
    }
    secretsmanager = {
      service             = "secretsmanager"
      private_dns_enabled = true
      subnet_ids          = module.vpc.private_subnets
    }
  }
}
