# parmail

Process USPS Informed Delivery emails to extract scanned mail images and analyze them using AI.

## What it does

USPS Informed Delivery sends daily emails with scanned images of incoming mail pieces. This tool:

1. Parses those emails (MIME format) and extracts the scanned images
2. Stores images organized by email date and source
3. Analyzes each image via AWS Bedrock (Claude) to extract:
   - Sender and recipient addresses
   - Mail classification (advertising, political, personal, financial, government)
   - Full text transcription
4. Writes a `manifest.json` per email with all metadata

## Why

At individual scale, this gives you a searchable archive of your physical mail. At larger scale — if many people opt in — it could provide aggregate insight into the kind of mail people receive (political mail volume during elections, advertising trends, etc).

## Usage

```bash
# Validate AWS credentials and Bedrock access
parmail validate

# Process a single .eml file
parmail process message.eml

# Process a directory of .eml files recursively
parmail process emails/ --storage-dir ./data

# Mix files, directories, and S3 URIs
parmail process emails/ s3://my-bucket/emails/ extra.eml

# Process from S3 (lists all .eml files under prefix)
parmail process s3://my-bucket/incoming/

# Output to S3 instead of local disk
parmail process emails/ --storage-dir s3://my-bucket/output/

# S3 to S3
parmail process s3://input-bucket/emails/ --storage-dir s3://output-bucket/results/

# Run as AWS Lambda (S3 event trigger)
parmail lambda
```

### Verbosity

```bash
parmail -qq process emails/   # Silent (errors only)
parmail -q process emails/    # Spinner + percentage
parmail process emails/       # Progress bar + one-liner per file (default)
parmail -v process emails/    # All processing steps
parmail -vv process emails/   # All steps + JSON dump
```

## Multi-Model Comparison

The tool supports running multiple Bedrock models against the same email corpus to compare accuracy, cost, and resolvability.

### Model Configuration

Models are defined in `models.default.json`. Each entry specifies a model ID and how to invoke it:

```json
{
  "us.amazon.nova-pro-v1:0": { "format": "json_prompt", "prompt": "..." },
  "us.anthropic.claude-haiku-4-5-20251001-v1:0": { "format": "tool_use" }
}
```

- `tool_use`: Uses Bedrock Converse API forced tool use (Claude models)
- `json_prompt`: Embeds JSON schema in the prompt text, parses response (non-Claude models)

### Models Under Evaluation

| Model | Format | Status | Notes |
|-------|--------|--------|-------|
| Claude Haiku 4.5 | tool_use | Baseline (production) | Currently deployed in Lambda |
| Claude Sonnet 4 | tool_use | Pending test | Expected high accuracy, ~$58/full-run |
| Claude Sonnet 4.6 | tool_use | Pending test | Latest, ~$58/full-run |
| Claude Opus 4 | tool_use | Pending test | Highest accuracy expected, ~$290/full-run |
| Amazon Nova Pro | json_prompt | Tested — 90% parse success | ~$14/full-run |
| Amazon Nova Lite | json_prompt | Tested — 52% parse success | ~$1/full-run |
| Meta Llama 4 Scout | json_prompt | Failed — 0% image support | Dropped |
| Meta Llama 4 Maverick | json_prompt | Failed — 0% image support | Dropped |
| Mistral Pixtral Large | json_prompt | Failed — 0% image support | Dropped |

### Running Comparisons

```bash
# Run all models in models.default.json in parallel
bin/run_models.sh -i emails/

# Run specific models
bin/run_models.sh -i emails/ -m us.amazon.nova-pro-v1:0 -m us.amazon.nova-lite-v1:0

# Results land in results/{model-short-name}/
# Use --save-responses to capture raw API responses for debugging
parmail process --model us.amazon.nova-pro-v1:0 --save-responses emails/
```

### Final Report Metrics

The comparison report will score each model on:

| Metric | Source | Description |
|--------|--------|-------------|
| address_resolved_pct | manifest | % of mail pieces with from_address.status = "resolved" |
| to_address_resolved_pct | manifest | % with to_address.status = "resolved" |
| confidence_avg | manifest | Mean confidence score across all pieces |
| mail_type_classified_pct | manifest | % with mail_type != "unknown" |
| full_text_present_pct | manifest | % of images with non-empty full_text |
| postmark_date_present_pct | manifest | % with postmark_date extracted |
| parse_error_rate | manifest | % of images that failed analysis entirely |
| input_tokens | Bedrock response | Total input tokens consumed |
| output_tokens | Bedrock response | Total output tokens consumed |
| cost_estimate | computed | (input_tokens × $/M) + (output_tokens × $/M) |
| cost_per_resolved_address | computed | cost_estimate / total_resolved_addresses |

Token counts must be captured from the Bedrock Converse API response metadata (TODO: wire into manifest or separate metrics file).

## Scripts (bin/)

```bash
bin/fetch_emails.sh -o emails/        # Bulk-download emails from Gmail via IMAP
bin/forward_emails.sh -r addr emails/ # Forward .eml files via SMTP
bin/report.sh -r all                  # Generate all CSV reports
```

Requires `~/.netrc` with Gmail app password credentials (no spaces in the password). Run any script with `-h` for options.

## Reports

```
Usage: bin/report.sh [-i data_dir] [-o output_dir] -r report [-r report]...

Options:
  -i DIR    Data directory containing manifest.json files (default: ./results)
  -o DIR    Write CSV files to this directory
  -r NAME   Report to generate (may specify multiple)

Reports:
  agreement       Cross-model agreement rates on from_name, street, city, mail_type, to_name
  dq              Address resolution rates (resolved/redacted/unreadable/not_analyzed)
  ds              Repeat senders (5+ times) - unsubscribe candidates
  export          Export all manifests to flat CSV (one row per mail piece)
  model_cost      Token usage, cost, and cost-per-resolved-address by model
  model_stats     Per-model parse and resolution rates
  mtb             Mail type breakdown (advertising/financial/personal/etc) with percentages
  ts              Top senders by frequency
  vot             Mail pieces received per day

  all             Run all reports
```

To add a new report, drop a `.jq` file in `bin/jq/` with `include "shared"`, `def describe:`, and `def execute:`. It auto-discovers.

## Output structure

```
data/
  79a5cc32ea4d10cf/
    manifest.json
    1080623812-052.jpg
    ra_0_1080623812-052.jpg
  0119894404aaf877/
    manifest.json
    mailer-1200542208.jpg
    content-1200542208.jpg
```

## Building

```bash
cargo build --release
```

## Docker

```bash
docker build -t parmail .
```

Produces a `FROM scratch` image with just the statically-linked binary and CA certificates.

The container defaults to `parmail lambda` for use as an AWS Lambda container image. To use it locally for batch processing, override the command:

```bash
# Run as Lambda (default)
docker run parmail

# Process local files (mount a volume)
docker run -v ./emails:/data/input -v ./output:/data/output \
  parmail process -s /data/output /data/input

# Process from S3 (pass AWS credentials)
docker run -e AWS_ACCESS_KEY_ID -e AWS_SECRET_ACCESS_KEY -e AWS_REGION \
  parmail process -s s3://my-bucket/output s3://my-bucket/emails/
```

### Deploying to Lambda

The Docker image is built and pushed to ECR automatically as part of `terraform apply`. The Lambda is triggered by S3 events when new emails land under the `emails/` prefix. It processes them and writes results to the `output/` prefix in the same bucket.

To manually rebuild and deploy outside Terraform:

```bash
aws ecr get-login-password | docker login --username AWS --password-stdin $ECR_URL
docker build -t $ECR_URL:latest .
docker push $ECR_URL:latest

aws lambda update-function-code \
  --function-name parmail \
  --image-uri $ECR_URL:latest
```

## Infrastructure

Terraform configs in `terraform/` provision:
- Subdomain hosted zone (`parmail.<your-domain>`) with NS delegation from parent
- Route53 MX record pointing to SES inbound
- SES domain identity with DNS verification
- SES receipt rule storing incoming mail to S3
- Single S3 bucket with key prefixes (`emails/` for incoming, `output/` for results)
- Docker image build and push to ECR (via kreuzwerker/docker provider)
- Lambda function (container image, arm64) triggered by S3 events under `emails/`
- ECR repository
- IAM roles with least-privilege access scoped to key prefixes

### Setup

Requires a domain you own with a Route53 hosted zone in the same account.

```bash
cd terraform
terraform init
terraform apply -var="parent_domain=yourdomain.com"
```

Or via environment variables:

```bash
export TF_VAR_parent_domain=yourdomain.com
terraform apply
```

### USPS Informed Delivery confirmation

After signing up for Informed Delivery with your new email address, USPS will send a verification email. Since there's no inbox (emails go straight to S3), retrieve it manually:

```bash
aws s3 ls s3://parmail-<account-id>/emails/ --recursive
aws s3 cp s3://parmail-<account-id>/emails/<key> - | grep -i "https.*confirm\|https.*verify"
```

### Registering a new domain (optional)

If you need a new domain, use the included registration script:

```bash
export FIRST_NAME=... LAST_NAME=... EMAIL=... PHONE="+1.555..." STATE=CA CITY=... ADDRESS=... ZIP=...
./terraform/register_domain.sh yourdomain.click
```

Then create a hosted zone for it before running `terraform apply`.

## Requirements

- Rust 1.85+
- AWS credentials with Bedrock access (Claude Sonnet)
- For S3 input: AWS credentials with S3 read access
- For email fetching: Gmail account with IMAP enabled and app password
