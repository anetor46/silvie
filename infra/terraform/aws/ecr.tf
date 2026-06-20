module "server_repo" {
  source  = "terraform-aws-modules/ecr/aws"
  version = "~> 3.2.0"
  region  = var.region

  repository_name               = "${var.project}-server-${var.region}-${var.env}"
  repository_type               = "private"
  repository_image_scan_on_push = false
  repository_lifecycle_policy   = jsonencode({
    rules = [
      {
        rulePriority = 1,
        description  = "Keep last 30 images",
        selection = {
          tagStatus     = "tagged",
          countType     = "imageCountMoreThan",
          countNumber   = 10
        },
        action = {
          type = "expire"
        }
      }
    ]
  })

  repository_force_delete       = true
}
