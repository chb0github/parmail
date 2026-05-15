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

variable "ses_domain" {
  description = "Domain configured for SES email receiving"
  type        = string
}

variable "recipient_email" {
  description = "Email address that will receive USPS Informed Delivery emails"
  type        = string
}
