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

variable "parent_domain" {
  description = "Parent domain you own (must have a Route53 hosted zone). Subdomain parmail.<domain> will be created."
  type        = string
  default     = ""

  validation {
    condition     = var.parent_domain != ""
    error_message = "parent_domain is required. Set TF_VAR_parent_domain env var or pass -var=\"parent_domain=example.com\""
  }
}

variable "email_user" {
  description = "Local part of the email address (e.g. 'usps' becomes usps@parmail.<domain>)"
  type        = string
  default     = "usps"
}

variable "forward_email" {
  description = "Email address to forward incoming mail to (for USPS confirmation)"
  type        = string
  default     = ""

  validation {
    condition     = var.forward_email != ""
    error_message = "forward_email is required. Set TF_VAR_forward_email env var or pass -var=\"forward_email=you@example.com\""
  }
}
