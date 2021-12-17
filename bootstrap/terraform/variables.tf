variable "region" {
  default     = "eu-central-1"
  description = "AWS region"
}

variable "db_password" {
  description = "RDS root user password"
  sensitive   = true
}
