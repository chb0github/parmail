resource "aws_iam_role" "lambda_role" {
  name = "${var.project_name}-lambda-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "lambda_policy" {
  name = "${var.project_name}-lambda-policy"
  role = aws_iam_role.lambda_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
        ]
        Resource = "${aws_s3_bucket.parmail.arn}/emails/*"
      },
      {
        Effect = "Allow"
        Action = [
          "s3:PutObject",
        ]
        Resource = "${aws_s3_bucket.parmail.arn}/output/*"
      },
      {
        Effect = "Allow"
        Action = [
          "bedrock:InvokeModel",
        ]
        Resource = "arn:aws:bedrock:${data.aws_region.current.name}::foundation-model/anthropic.claude-sonnet-4-20250514"
      },
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents",
        ]
        Resource = "arn:aws:logs:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:*"
      }
    ]
  })
}
