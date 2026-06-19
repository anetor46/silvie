resource "aws_secretsmanager_secret" "sever_secret" {
  name   = "/${var.env}/${var.region}/${var.project}/server"
  region = var.region
}
