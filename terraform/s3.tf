resource "aws_s3_bucket" "parmail" {
  bucket = "${var.project_name}-${data.aws_caller_identity.current.account_id}"
}

resource "aws_s3_bucket_notification" "email_notification" {
  bucket = aws_s3_bucket.parmail.id

  lambda_function {
    lambda_function_arn = aws_lambda_function.parmail.arn
    events              = ["s3:ObjectCreated:*"]
    filter_prefix       = "emails/"
  }

  depends_on = [aws_lambda_permission.allow_s3]
}
