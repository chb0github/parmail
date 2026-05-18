resource "aws_ecr_repository" "parmail" {
  name                 = var.project_name
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "docker_registry_image" "parmail" {
  name          = "${aws_ecr_repository.parmail.repository_url}:latest"
  keep_remotely = true

  build {
    context    = "${path.module}/.."
    dockerfile = "Dockerfile"
    platform   = "linux/arm64"
    provenance = "false"
  }

  triggers = {
    dir_sha1 = sha1(join("", [
      filesha1("${path.module}/../Dockerfile"),
      filesha1("${path.module}/../Cargo.toml"),
      filesha1("${path.module}/../Cargo.lock"),
      sha1(join("", [for f in fileset("${path.module}/../src", "**") : filesha1("${path.module}/../src/${f}")])),
    ]))
  }
}

resource "aws_lambda_function" "parmail" {
  function_name = var.project_name
  role          = aws_iam_role.lambda_role.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.parmail.repository_url}@${docker_registry_image.parmail.sha256_digest}"
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

  depends_on = [docker_registry_image.parmail]
}

resource "aws_lambda_permission" "allow_s3" {
  statement_id  = "AllowS3Invoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.parmail.function_name
  principal     = "s3.amazonaws.com"
  source_arn    = aws_s3_bucket.parmail.arn
}

output "ecr_repository_url" {
  value = aws_ecr_repository.parmail.repository_url
}

output "lambda_function_name" {
  value = aws_lambda_function.parmail.function_name
}
