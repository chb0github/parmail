terraform {
  required_version = ">= 1.5"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      StackId = var.stack_id
    }
  }
}

resource "terraform_data" "stack_id" {
  input = var.stack_id
}

data "aws_caller_identity" "current" {}
data "aws_region" "current" {}
