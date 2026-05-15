resource "aws_ses_receipt_rule_set" "main" {
  rule_set_name = "${var.project_name}-rules"
}

resource "aws_ses_active_receipt_rule_set" "main" {
  rule_set_name = aws_ses_receipt_rule_set.main.rule_set_name
}

resource "aws_ses_receipt_rule" "store_email" {
  name          = "${var.project_name}-store"
  rule_set_name = aws_ses_receipt_rule_set.main.rule_set_name
  recipients    = [var.recipient_email]
  enabled       = true
  scan_enabled  = true

  s3_action {
    bucket_name = aws_s3_bucket.email_storage.id
    position    = 1
  }

  depends_on = [aws_s3_bucket_policy.allow_ses_write]
}

resource "aws_s3_bucket_policy" "allow_ses_write" {
  bucket = aws_s3_bucket.email_storage.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "AllowSESPuts"
        Effect    = "Allow"
        Principal = { Service = "ses.amazonaws.com" }
        Action    = "s3:PutObject"
        Resource  = "${aws_s3_bucket.email_storage.arn}/*"
        Condition = {
          StringEquals = {
            "AWS:SourceAccount" = data.aws_caller_identity.current.account_id
          }
        }
      }
    ]
  })
}
