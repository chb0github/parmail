variable "enable_ec2" {
  description = "Set to true to create a management EC2 instance with access to all project resources"
  type        = bool
  default     = false
}

variable "ec2_instance_type" {
  description = "Instance type for the management EC2"
  type        = string
  default     = "c7g.4xlarge"
}

variable "ec2_spot_max_price" {
  description = "Maximum hourly price for spot instance (empty string = on-demand cap)"
  type        = string
  default     = "0.25"
}

resource "aws_key_pair" "mgmt" {
  count      = var.enable_ec2 ? 1 : 0
  key_name   = "${var.project_name}-mgmt-key"
  public_key = file(pathexpand("~/.ssh/id_ed25519.pub"))
}

data "aws_vpc" "default" {
  count   = var.enable_ec2 ? 1 : 0
  default = true
}

data "aws_ami" "al2023_arm" {
  count       = var.enable_ec2 ? 1 : 0
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["al2023-ami-*-arm64"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

resource "aws_security_group" "ec2_mgmt" {
  count       = var.enable_ec2 ? 1 : 0
  name        = "${var.project_name}-mgmt-sg"
  description = "Security group for parmail management instance"
  vpc_id      = data.aws_vpc.default[0].id

  ingress {
    description = "SSH"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${var.project_name}-mgmt"
  }
}

resource "aws_iam_role" "ec2_mgmt_role" {
  count = var.enable_ec2 ? 1 : 0
  name  = "${var.project_name}-ec2-mgmt-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "ec2_mgmt_policy" {
  count = var.enable_ec2 ? 1 : 0
  name  = "${var.project_name}-ec2-mgmt-policy"
  role  = aws_iam_role.ec2_mgmt_role[0].id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "S3FullAccess"
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
          "s3:ListBucket",
        ]
        Resource = [
          aws_s3_bucket.parmail.arn,
          "${aws_s3_bucket.parmail.arn}/*",
        ]
      },
      {
        Sid    = "LambdaManage"
        Effect = "Allow"
        Action = [
          "lambda:GetFunction",
          "lambda:InvokeFunction",
          "lambda:UpdateFunctionCode",
          "lambda:UpdateFunctionConfiguration",
          "lambda:ListVersionsByFunction",
          "lambda:GetFunctionConfiguration",
        ]
        Resource = [
          aws_lambda_function.extractor.arn,
          aws_lambda_function.confirmer.arn,
        ]
      },
      {
        Sid    = "ECRAccess"
        Effect = "Allow"
        Action = [
          "ecr:GetAuthorizationToken",
        ]
        Resource = "*"
      },
      {
        Sid    = "ECRRepoAccess"
        Effect = "Allow"
        Action = [
          "ecr:BatchCheckLayerAvailability",
          "ecr:GetDownloadUrlForLayer",
          "ecr:BatchGetImage",
          "ecr:PutImage",
          "ecr:InitiateLayerUpload",
          "ecr:UploadLayerPart",
          "ecr:CompleteLayerUpload",
          "ecr:DescribeImages",
          "ecr:ListImages",
        ]
        Resource = [
          aws_ecr_repository.extractor.arn,
          aws_ecr_repository.confirmer.arn,
        ]
      },
      {
        Sid    = "BedrockInvoke"
        Effect = "Allow"
        Action = [
          "bedrock:InvokeModel",
        ]
        Resource = [
          "arn:aws:bedrock:*:${data.aws_caller_identity.current.account_id}:inference-profile/us.anthropic.claude-haiku-4-5-20251001-v1:0",
          "arn:aws:bedrock:*::foundation-model/anthropic.claude-haiku-4-5-20251001-v1:0",
        ]
      },
      {
        Sid    = "CloudWatchLogs"
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents",
          "logs:GetLogEvents",
          "logs:FilterLogEvents",
          "logs:DescribeLogGroups",
          "logs:DescribeLogStreams",
        ]
        Resource = "arn:aws:logs:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:*"
      },
      {
        Sid    = "SESManage"
        Effect = "Allow"
        Action = [
          "ses:GetIdentityVerificationAttributes",
          "ses:ListReceiptRuleSets",
          "ses:DescribeReceiptRule",
          "ses:DescribeActiveReceiptRuleSet",
        ]
        Resource = "*"
      },
    ]
  })
}

resource "aws_iam_instance_profile" "ec2_mgmt" {
  count = var.enable_ec2 ? 1 : 0
  name  = "${var.project_name}-ec2-mgmt-profile"
  role  = aws_iam_role.ec2_mgmt_role[0].name
}

resource "aws_instance" "mgmt" {
  count                  = var.enable_ec2 ? 1 : 0
  ami                    = data.aws_ami.al2023_arm[0].id
  instance_type          = var.ec2_instance_type
  iam_instance_profile   = aws_iam_instance_profile.ec2_mgmt[0].name
  vpc_security_group_ids = [aws_security_group.ec2_mgmt[0].id]
  key_name               = aws_key_pair.mgmt[0].key_name

  instance_market_options {
    market_type = "spot"
    spot_options {
      spot_instance_type = "one-time"
      max_price          = var.ec2_spot_max_price
    }
  }

  user_data = <<-EOF
    #!/bin/bash
    set -e

    # Build dependencies for Rust + AWS SDK crates
    dnf install -y gcc gcc-c++ make openssl-devel pkg-config git jq

    # Install rustup as ec2-user
    su - ec2-user -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
  EOF

  tags = {
    Name = "${var.project_name}-mgmt"
  }
}

output "ec2_instance_id" {
  value       = var.enable_ec2 ? aws_instance.mgmt[0].id : null
  description = "Management EC2 instance ID"
}

output "ec2_public_ip" {
  value       = var.enable_ec2 ? aws_instance.mgmt[0].public_ip : null
  description = "Management EC2 public IP"
}
