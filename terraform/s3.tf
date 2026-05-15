resource "aws_s3_bucket" "email_storage" {
  bucket = "${var.project_name}-emails-${data.aws_caller_identity.current.account_id}"
}

resource "aws_s3_bucket" "image_storage" {
  bucket = "${var.project_name}-images-${data.aws_caller_identity.current.account_id}"
}

resource "aws_s3_bucket_notification" "email_notification" {
  bucket = aws_s3_bucket.email_storage.id

  lambda_function {
    lambda_function_arn = aws_lambda_function.parmail.arn
    events              = ["s3:ObjectCreated:*"]
  }

  depends_on = [aws_lambda_permission.allow_s3]
}
