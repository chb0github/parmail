# --- SQS Pipeline: S3 → Gatekeeper → Confirmer → Extractor ---

# SQS₁: S3 → Gatekeeper
resource "aws_sqs_queue" "inbound_dlq" {
  name                      = "${var.project_name}-inbound-dlq"
  message_retention_seconds = 1209600 # 14 days
}

resource "aws_sqs_queue" "inbound" {
  name                       = "${var.project_name}-inbound"
  visibility_timeout_seconds = 60
  message_retention_seconds  = 86400 # 1 day

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.inbound_dlq.arn
    maxReceiveCount     = 3
  })
}

resource "aws_sqs_queue_policy" "allow_s3" {
  queue_url = aws_sqs_queue.inbound.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { Service = "s3.amazonaws.com" }
        Action    = "sqs:SendMessage"
        Resource  = aws_sqs_queue.inbound.arn
        Condition = {
          ArnLike = { "aws:SourceArn" = aws_s3_bucket.parmail.arn }
        }
      }
    ]
  })
}

# SQS₂: Gatekeeper → Confirmer
resource "aws_sqs_queue" "confirmer_dlq" {
  name                      = "${var.project_name}-confirmer-dlq"
  message_retention_seconds = 1209600
}

resource "aws_sqs_queue" "confirmer" {
  name                       = "${var.project_name}-confirmer"
  visibility_timeout_seconds = 60
  message_retention_seconds  = 86400

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.confirmer_dlq.arn
    maxReceiveCount     = 3
  })
}

# SQS₃: Confirmer → Extractor
resource "aws_sqs_queue" "extractor_dlq" {
  name                      = "${var.project_name}-extractor-dlq"
  message_retention_seconds = 1209600
}

resource "aws_sqs_queue" "extractor" {
  name                       = "${var.project_name}-extractor"
  visibility_timeout_seconds = 360
  message_retention_seconds  = 86400

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.extractor_dlq.arn
    maxReceiveCount     = 3
  })
}

# --- Event Source Mappings ---

resource "aws_lambda_event_source_mapping" "gatekeeper" {
  event_source_arn                   = aws_sqs_queue.inbound.arn
  function_name                      = aws_lambda_function.gatekeeper.arn
  batch_size                         = 10
  maximum_batching_window_in_seconds = 5
  function_response_types            = ["ReportBatchItemFailures"]
}

resource "aws_lambda_event_source_mapping" "confirmer" {
  event_source_arn                   = aws_sqs_queue.confirmer.arn
  function_name                      = aws_lambda_function.confirmer.arn
  batch_size                         = 1
  maximum_batching_window_in_seconds = 0
  function_response_types            = ["ReportBatchItemFailures"]
}

resource "aws_lambda_event_source_mapping" "extractor" {
  event_source_arn                   = aws_sqs_queue.extractor.arn
  function_name                      = aws_lambda_function.extractor.arn
  batch_size                         = 1
  maximum_batching_window_in_seconds = 0
  function_response_types            = ["ReportBatchItemFailures"]
}
