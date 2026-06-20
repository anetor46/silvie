module "ecs_cluster" {
  source  = "terraform-aws-modules/ecs/aws"
  version = "~> 7.4.0"

  cluster_name = "${var.project}-${var.region}-${var.env}"
  region       = var.region
  cluster_configuration = {
    execute_command_configuration = {
      logging = "OVERRIDE"
      log_configuration = {
        cloud_watch_log_group_name = "/aws/ecs/${var.project}/${var.env}/${var.region}"
      }
    }
  }

  cluster_capacity_providers = ["FARGATE"]
  default_capacity_provider_strategy = {
    FARGATE = {
      weight = 100
    }
  }

  services = {
    "server" = {
      family = "${var.project}-server-${var.region}-${var.env}"
      cpu    = var.params.server.cpu
      memory = var.params.server.memory

      load_balancer = {
        service = {
          target_group_arn = module.alb.target_groups["server"].arn
          container_name   = "main"
          container_port   = var.params.server.port
        }
      }

      tasks_iam_role_name = "${var.project}-${var.region}-${var.env}"

      task_exec_iam_role_name = "${var.project}-exec-${var.region}-${var.env}"
      task_exec_iam_statements = [
        {
          sid     = "SecretManagerAccess"
          effect  = "Allow"
          actions = ["secretsmanager:GetSecretValue"]
          resources = [aws_secretsmanager_secret.sever_secret.arn]
        }
      ]

      requires_compatibilities = ["FARGATE"]
      capacity_provider_strategy = {
        FARGATE = {
          base              = 1
          weight            = 100
          capacity_provider = "FARGATE"
        }
      }

      availability_zone_rebalancing      = "ENABLED"
      deployment_maximum_percent         = 200
      deployment_minimum_healthy_percent = 100
      autoscaling_min_capacity           = var.params.server.min_capacity
      autoscaling_max_capacity           = var.params.server.max_capacity
      autoscaling_policies = {
        cpu = {
          policy_type = "TargetTrackingScaling"
          target_tracking_scaling_policy_configuration = {
            predefined_metric_specification = {
              predefined_metric_type = "ECSServiceAverageCPUUtilization"
            }
            target_value       = 70
            scale_in_cooldown  = 300
            scale_out_cooldown = 60
          }
        }
      }

      container_definitions = {
        server = { # PLACEHOLDER will be replaced by CI/CD
          image = "hello-world",
          portMappings = [{
            containerPort = var.params.server.port
            hostPort      = var.params.server.port
            protocol      = "tcp"
          }]
          cloudwatch_log_group_name = "/aws/ecs/${var.project}/${var.env}/${var.region}/server"
        }
      }

      network_mode = "awsvpc"
      subnet_ids   = module.vpc.private_subnets

      security_group_egress_rules = {
        all = {
          from_port   = -1
          to_port     = -1
          ip_protocol = "-1"
          cidr_ipv4   = "0.0.0.0/0"
        }
      }

      security_group_ingress_rules = {
        vpc_ingress = {
          from_port   = var.params.server.port
          to_port     = var.params.server.port
          ip_protocol = "tcp"
          description = "Service port"
          cidr_ipv4   = module
        }
      }

      enable_ecs_managed_tags        = true
      enable_execute_command         = true
      propagate_tags                 = "SERVICE"
      ignore_task_definition_changes = true
      track_latest                   = false
    }
  }
}
