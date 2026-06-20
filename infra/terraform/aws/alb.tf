data "aws_acm_certificate" "cert" {
  domain      = "silvie.uk"
  types       = ["AMAZON_ISSUED"]
}

module "alb" {
  source  = "terraform-aws-modules/alb/aws"
  version = "~> 10.5.0"

  enable_deletion_protection = false

  name               = "${var.project}-alb-${var.region}-${var.env}"
  region             = var.region
  load_balancer_type = "application"

  vpc_id             = module.vpc.vpc_id
  subnets            = module.vpc.public_subnets_cidr_blocks

  security_group_ingress_rules = merge(
    {
      https = {
        name        = "${var.project}-alb-ingress-https-${var.region}-${var.env}"
        description = "HTTPS"
        from_port   = 443
        to_port     = 443
        ip_protocol = "tcp"
        cidr_ipv4   = "0.0.0.0/0"
      }
    }
  )

  security_group_egress_rules = {
    http_server_egress = {
      name        = "${var.project}-alb-egress-server-${var.region}-${var.env}"
      description = "HTTP Proxy Egress"
      from_port   = var.params.server.port
      to_port     = var.params.server.port
      ip_protocol = "tcp"
      cidr_ipv4   = module.vpc.vpc_cidr_block
    }
  }

  target_groups = {
    server = {
      target_type                       = "ip"
      protocol                          = "HTTP"
      port                              = var.params.server.port
      protocol_version                  = "HTTP1"
      deregistration_delay              = 30
      load_balancing_cross_zone_enabled = true
      health_check = {
        enabled             = true
        healthy_threshold   = 3
        interval            = 30
        matcher             = "200-299"
        path                = "/health"
        port                = "traffic-port"
        protocol            = "HTTP"
        timeout             = 5
        unhealthy_threshold = 5
      }
      slow_start        = 60
      create_attachment = false
    }
  }

  listeners = {
    https = {
      port            = 443
      certificate_arn = data.aws_acm_certificate.cert.arn
      protocol        = "HTTPS"
      forward = {
        target_group_key = "http"
      }
    }
  }
}
