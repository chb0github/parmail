# --- Extractor ---

resource "aws_ecr_repository" "extractor" {
  name                 = "${var.project_name}/extractor"
  image_tag_mutability = "MUTABLE"
  force_delete         = var.force_delete_ecr

  image_scanning_configuration {
    scan_on_push = true
  }
}

data "aws_ecr_image" "extractor" {
  repository_name = aws_ecr_repository.extractor.name
  image_tag       = "latest"
}

resource "aws_lambda_function" "extractor" {
  function_name = "${var.project_name}-extractor"
  role          = aws_iam_role.lambda_role.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.extractor.repository_url}@${data.aws_ecr_image.extractor.image_digest}"
  timeout       = 300
  memory_size   = 512
  architectures = ["arm64"]

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

output "ecr_extractor_url" {
  value = aws_ecr_repository.extractor.repository_url
}

output "extractor_function_name" {
  value = aws_lambda_function.extractor.function_name
}
