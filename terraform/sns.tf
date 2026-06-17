resource "aws_sns_topic" "email_notify" {
  name = "${var.project_name}-email-notify"
}

resource "aws_sns_topic_policy" "allow_s3" {
  arn = aws_sns_topic.email_notify.arn

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { Service = "s3.amazonaws.com" }
        Action    = "SNS:Publish"
        Resource  = aws_sns_topic.email_notify.arn
        Condition = {
          ArnLike = { "aws:SourceArn" = aws_s3_bucket.parmail.arn }
        }
      }
    ]
  })
}

resource "aws_sns_topic_subscription" "interpreter" {
  topic_arn = aws_sns_topic.email_notify.arn
  protocol  = "lambda"
  endpoint  = aws_lambda_function.interpreter.arn
}

resource "aws_sns_topic_subscription" "confirmer" {
  topic_arn = aws_sns_topic.email_notify.arn
  protocol  = "lambda"
  endpoint  = aws_lambda_function.confirmer.arn
}

resource "aws_lambda_permission" "sns_interpreter" {
  statement_id  = "AllowSNSInvoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.interpreter.function_name
  principal     = "sns.amazonaws.com"
  source_arn    = aws_sns_topic.email_notify.arn
}

resource "aws_lambda_permission" "sns_confirmer" {
  statement_id  = "AllowSNSInvoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.confirmer.function_name
  principal     = "sns.amazonaws.com"
  source_arn    = aws_sns_topic.email_notify.arn
}
