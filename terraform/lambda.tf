resource "aws_lambda_function" "parmail" {
  function_name = var.project_name
  role          = aws_iam_role.lambda_role.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.parmail.repository_url}:latest"
  timeout       = 300
  memory_size   = 512

  image_config {
    command = ["lambda"]
  }

  environment {
    variables = {
      STORAGE_DIR = "s3://${aws_s3_bucket.parmail.id}/output"
      RUST_LOG    = "parmail=info"
    }
  }
}

resource "aws_lambda_permission" "allow_s3" {
  statement_id  = "AllowS3Invoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.parmail.function_name
  principal     = "s3.amazonaws.com"
  source_arn    = aws_s3_bucket.parmail.arn
}

resource "aws_ecr_repository" "parmail" {
  name                 = var.project_name
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

output "ecr_repository_url" {
  value = aws_ecr_repository.parmail.repository_url
}

output "lambda_function_name" {
  value = aws_lambda_function.parmail.function_name
}
