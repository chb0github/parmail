variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "project_name" {
  description = "Project name used for resource naming"
  type        = string
  default     = "parmail"
}

variable "forward_email" {
  description = "Email address to forward incoming mail to (for USPS confirmation)"
  type        = string
}
