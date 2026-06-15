locals {
  src_hash = sha1(join("", [
    filesha1("${path.module}/../Dockerfile"),
    filesha1("${path.module}/../Cargo.toml"),
    filesha1("${path.module}/../Cargo.lock"),
    sha1(join("", [for f in fileset("${path.module}/../src", "**") : filesha1("${path.module}/../src/${f}")])),
  ]))
}

# --- Interpreter ---

resource "aws_ecr_repository" "interpreter" {
  name                 = "${var.project_name}/interpreter"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "docker_registry_image" "interpreter" {
  name          = "${aws_ecr_repository.interpreter.repository_url}:latest"
  keep_remotely = true

  build {
    context    = abspath("${path.module}/..")
    dockerfile = abspath("${path.module}/../Dockerfile")
    platform   = "linux/arm64"
    target     = "interpreter"
    no_cache   = true
  }

  triggers = {
    dir_sha1 = local.src_hash
  }
}

resource "aws_lambda_function" "interpreter" {
  function_name = "${var.project_name}-interpreter"
  role          = aws_iam_role.lambda_role.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.interpreter.repository_url}@${docker_registry_image.interpreter.sha256_digest}"
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

  depends_on = [docker_registry_image.interpreter]
}

resource "aws_lambda_permission" "allow_s3_interpreter" {
  statement_id  = "AllowS3Invoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.interpreter.function_name
  principal     = "s3.amazonaws.com"
  source_arn    = aws_s3_bucket.parmail.arn
}

output "ecr_interpreter_url" {
  value = aws_ecr_repository.interpreter.repository_url
}

output "interpreter_function_name" {
  value = aws_lambda_function.interpreter.function_name
}
