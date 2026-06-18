# --- Confirmer Lambda ---

resource "aws_ecr_repository" "confirmer" {
  name                 = "${var.project_name}/confirmer"
  image_tag_mutability = "MUTABLE"
  force_delete         = var.force_delete_ecr

  image_scanning_configuration {
    scan_on_push = true
  }
}

data "aws_ecr_image" "confirmer" {
  repository_name = aws_ecr_repository.confirmer.name
  image_tag       = "latest"
}

resource "aws_lambda_function" "confirmer" {
  function_name = "${var.project_name}-confirmer"
  role          = aws_iam_role.confirmer_role.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.confirmer.repository_url}@${data.aws_ecr_image.confirmer.image_digest}"
  timeout       = 30
  memory_size   = 128
  architectures = ["arm64"]

  image_config {
    command = ["lambda"]
  }

  environment {
    variables = {
      EXTRACTOR_QUEUE_URL = aws_sqs_queue.extractor.url
      BUCKET_NAME         = aws_s3_bucket.parmail.id
      RUST_LOG            = "parmail_confirmer=info"
    }
  }
}

output "ecr_confirmer_url" {
  value = aws_ecr_repository.confirmer.repository_url
}

output "confirmer_function_name" {
  value = aws_lambda_function.confirmer.function_name
}
