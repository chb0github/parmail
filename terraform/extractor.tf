locals {
  src_hash = sha1(join("", [
    filesha1("${path.module}/../Dockerfile"),
    filesha1("${path.module}/../Cargo.toml"),
    filesha1("${path.module}/../Cargo.lock"),
    sha1(join("", [for f in fileset("${path.module}/../src", "**") : filesha1("${path.module}/../src/${f}")])),
  ]))
}

# --- Extractor ---

resource "aws_ecr_repository" "extractor" {
  name                 = "${var.project_name}/extractor"
  image_tag_mutability = "MUTABLE"
  force_delete         = true

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "docker_registry_image" "extractor" {
  name          = "${aws_ecr_repository.extractor.repository_url}:latest"
  keep_remotely = true

  build {
    context    = abspath("${path.module}/..")
    dockerfile = abspath("${path.module}/../Dockerfile")
    platform   = "linux/arm64"
    target     = "extractor"
    no_cache   = true
  }

  triggers = {
    dir_sha1 = local.src_hash
  }
}

resource "aws_lambda_function" "extractor" {
  function_name = "${var.project_name}-extractor"
  role          = aws_iam_role.lambda_role.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.extractor.repository_url}@${docker_registry_image.extractor.sha256_digest}"
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

  depends_on = [docker_registry_image.extractor]
}


output "ecr_extractor_url" {
  value = aws_ecr_repository.extractor.repository_url
}

output "extractor_function_name" {
  value = aws_lambda_function.extractor.function_name
}
