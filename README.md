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

All scripts support `-h` for detailed usage. Most require `~/.netrc` with Gmail app password credentials (no spaces in the password).

### fetch_emails.sh

Bulk-download emails from Gmail via IMAP.

```
Usage: bin/fetch_emails.sh [OPTIONS]

Options:
  -o DIR     Output directory (default: samples)
  -f FROM    Filter by sender (default: USPSInformeddelivery@email.informeddelivery.usps.com)
  -s SUBJ    Filter by subject (default: Your Daily Digest)
  -p NUM     Parallel downloads (default: 10)
  -m URL     IMAP mailbox URL (default: imaps://imap.gmail.com/%5BGmail%5D/All%20Mail)
  -n         Dry run: print message count and exit
  -h         Show this help

Examples:
  bin/fetch_emails.sh -o emails/               # Download all USPS emails
  bin/fetch_emails.sh -n                       # Count matching emails without downloading
  bin/fetch_emails.sh -f "sender@example.com"  # Download from specific sender
```

### forward_emails.sh

Forward .eml files via SMTP (useful for seeding S3 after infrastructure setup).

```
Usage: bin/forward_emails.sh -r RECIPIENT [OPTIONS] PATH [PATH...]

Forwards .eml files via SMTP. If PATH is a directory, all .eml files in it
are forwarded. If PATH is a file, it is forwarded directly.

Options:
  -r ADDR    Recipient email address (required)
  -d SECS    Delay between sends in seconds (default: 10)
  -h         Show this help

Examples:
  bin/forward_emails.sh -r mail@parmail.yourdomain.com emails/
  bin/forward_emails.sh -r mail@parmail.yourdomain.com -d 5 *.eml
```

### report.sh

Generate CSV reports from processed manifests.

```
Usage: bin/report.sh [options]

Options:
  -i DIR         Data directory containing manifest.json files (default: ./results)
  -o DIR         Output directory for CSV files (default: ./reports)
  -r REPORT      Report to generate (can specify multiple times, default: all)
  -h, --help     Show this help
  --completion   Show zsh completion setup
  --list-reports List available reports (one per line)

Available reports:
  addresses       Address frequency report with person names
  agreement       Cross-model agreement rates on from_name, street, city, mail_type, to_name
  dq              Address resolution rates (resolved/redacted/unreadable/not_analyzed)
  ds              Repeat senders (5+ times) - unsubscribe candidates
  errors          Per-model error counts grouped by error message
  export          Export all manifests to flat CSV (one row per mail piece)
  model_cost      Token usage, cost, and cost-per-resolved-address by model
  model_stats     Per-model parse and resolution rates
  mtb             Mail type breakdown (advertising/financial/personal/etc) with percentages
  ts              Top senders by frequency
  vot             Mail pieces received per day

Examples:
  bin/report.sh                        # Generate all reports
  bin/report.sh -r errors              # Generate errors report only
  bin/report.sh -r errors -r addresses # Generate errors and addresses reports
  bin/report.sh -o /tmp/reports        # Generate all reports to /tmp/reports
```

To add a new report, drop a `.jq` file in `bin/jq/` with `include "shared"`, `def describe:`, and `def execute:`. It auto-discovers.

### run_models.sh

Run multiple models against the same email corpus for comparison.

```
Usage: bin/run_models.sh [-m model]... [-f models_file] [-i input_dir] [-o output_dir] [--overwrite] [--save-responses]

Options:
  -m MODEL         Run a specific model (repeatable)
  -f FILE          Models config file (default: models.default.json)
  -i DIR           Input directory of .eml files (default: emails)
  -o DIR           Output base directory (default: results)
  --overwrite      Delete existing results before rerunning
  --save-responses Pass --save-responses to parmail for raw response capture

Examples:
  bin/run_models.sh                        # Run all models in models.default.json
  bin/run_models.sh -m us.amazon.nova-pro-v1:0
  bin/run_models.sh --overwrite            # Clear results and rerun all
```

### sync.sh

Sync local results to/from S3 and fill processing gaps.

```
Usage: bin/sync.sh [--reconcile] [--delete-remote] [--dry-run] [-m model] [-i results_dir] [-e emails_dir]

Sync local results to S3 and fill processing gaps.

Options:
  --reconcile      Pull emails from S3, process unprocessed ones, push results back
  --delete-remote  Delete all existing output/ in S3 before uploading
  --dry-run        Show what would be done without doing it
  -m MODEL         Model directory to sync (default: claude-haiku-4-5-20251001-v1)
  -i DIR           Local results base dir (default: ./results)
  -e DIR           Local emails dir (default: ./emails)
  -h               Show this help

Examples:
  bin/sync.sh                              # Sync default model results to S3
  bin/sync.sh --reconcile                  # Full reconciliation (download, process, upload)
  bin/sync.sh --delete-remote --reconcile  # Rebuild S3 output/ from scratch
  bin/sync.sh --dry-run                    # Preview without making changes
```

### reprocess_unparseable.sh

Find and reprocess emails that resulted in unparseable model responses.

**Note:** This script has no command-line options. It's hardcoded to reprocess with Claude Haiku 4.5. Edit the script to change the model or storage directory.

```bash
# Finds all manifests with "No parseable response from model" errors
# Deletes the manifest and reprocesses the email
bin/reprocess_unparseable.sh
```

The script:
1. Scans all manifests for parsing errors
2. Locates the original .eml file by Message-ID
3. Deletes the failed manifest
4. Reprocesses with the configured model (benefits from escape fix and disabled guardrails)

## Reports

See [report.sh](#reportsh) in the Scripts section for usage details.

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
