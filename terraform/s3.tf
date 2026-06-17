resource "aws_s3_bucket" "parmail" {
  bucket        = "${var.project_name}-${data.aws_caller_identity.current.account_id}"
  force_destroy = var.force_destroy_bucket
}

resource "aws_s3_bucket_notification" "email_notification" {
  bucket = aws_s3_bucket.parmail.id

  topic {
    topic_arn     = aws_sns_topic.email_notify.arn
    events        = ["s3:ObjectCreated:*"]
    filter_prefix = "emails/"
  }

  depends_on = [aws_sns_topic_policy.allow_s3]
}
