# --- Gatekeeper Lambda ---

resource "aws_ecr_repository" "gatekeeper" {
  name                 = "${var.project_name}/gatekeeper"
  image_tag_mutability = "MUTABLE"
  force_delete         = var.force_delete_ecr

  image_scanning_configuration {
    scan_on_push = true
  }
}

data "aws_ecr_image" "gatekeeper" {
  repository_name = aws_ecr_repository.gatekeeper.name
  image_tag       = "latest"
}

resource "aws_lambda_function" "gatekeeper" {
  function_name = "${var.project_name}-gatekeeper"
  role          = aws_iam_role.gatekeeper_role.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.gatekeeper.repository_url}@${data.aws_ecr_image.gatekeeper.image_digest}"
  timeout       = 30
  memory_size   = 128
  architectures = ["arm64"]

  image_config {
    command = ["lambda"]
  }

  environment {
    variables = {
      CONFIRMER_QUEUE_URL = aws_sqs_queue.confirmer.url
      RUST_LOG            = "parmail_gatekeeper=info"
    }
  }
}

output "ecr_gatekeeper_url" {
  value = aws_ecr_repository.gatekeeper.repository_url
}

output "gatekeeper_function_name" {
  value = aws_lambda_function.gatekeeper.function_name
}
