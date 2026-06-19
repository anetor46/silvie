variable "project" {
  type = string
}

variable "env" {
  type = string
}

variable "region" {
  type = string
}

variable "params" {
  type = object({
    vpc = object({
      cidr = string
      subnets = list(object({
        az = string
        private_cidr = string
        public_cidr = string
      }))
    })
    server = object({
      port = number
      cpu = number
      memory = number
      min_capacity = number
      max_capacity = number
    })
  })
}
