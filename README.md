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
parmail process samples/ --storage-dir ./data

# Mix files, directories, and S3 URIs
parmail process samples/ s3://my-bucket/emails/ extra.eml

# Process from S3 (lists all .eml files under prefix)
parmail process s3://my-bucket/incoming/

# Output to S3 instead of local disk
parmail process samples/ --storage-dir s3://my-bucket/output/

# S3 to S3
parmail process s3://input-bucket/emails/ --storage-dir s3://output-bucket/results/

# Run as AWS Lambda (S3 event trigger)
parmail lambda
```

### Verbosity

```bash
parmail -qq process samples/   # Silent (errors only)
parmail -q process samples/    # Spinner + percentage
parmail process samples/       # Progress bar + one-liner per file (default)
parmail -v process samples/    # All processing steps
parmail -vv process samples/   # All steps + JSON dump
```

## Fetching emails

Use the included script to bulk-download USPS Informed Delivery emails from Gmail via IMAP:

```bash
./fetch_emails.sh -o samples/
```

Requires `~/.netrc` with Gmail app password credentials (no spaces in the password). Run `./fetch_emails.sh -h` for options.

## Output structure

```
data/
  2025-07-25/
    your-daily-digest-for-3f7a0aa0/
      manifest.json
      1114971734-054.jpg
      1114971733-054.jpg
  2025-10-18/
    your-daily-digest-for-11414ccf/
      manifest.json
      1005799964-054.jpg
```

## Building

```bash
cargo build --release
```

## Docker

```bash
# Native architecture
docker build -t parmail .

# Multi-arch
docker buildx build --platform linux/amd64,linux/arm64 -t parmail .
```

Produces a `FROM scratch` image with just the statically-linked binary and CA certificates.

The container defaults to `parmail lambda` for use as an AWS Lambda container image. To use it locally for batch processing, override the command:

```bash
# Run as Lambda (default)
docker run parmail

# Process local files (mount a volume)
docker run -v ./samples:/data/input -v ./output:/data/output \
  parmail process -s /data/output /data/input

# Process from S3 (pass AWS credentials)
docker run -e AWS_ACCESS_KEY_ID -e AWS_SECRET_ACCESS_KEY -e AWS_REGION \
  parmail process -s s3://my-bucket/output s3://my-bucket/emails/
```

### Deploying to Lambda

```bash
# Build and push to ECR
aws ecr get-login-password | docker login --username AWS --password-stdin $ECR_URL
docker build -t $ECR_URL:latest .
docker push $ECR_URL:latest

# Update Lambda to use new image
aws lambda update-function-code \
  --function-name parmail \
  --image-uri $ECR_URL:latest
```

The Lambda is triggered by S3 events when new emails land under the `emails/` prefix. It processes them and writes results to the `output/` prefix in the same bucket.

## Infrastructure

Terraform configs in `terraform/` provision:
- Random `.click` domain via Route53 (for receiving email)
- Route53 hosted zone with MX record pointing to SES
- SES domain identity with DNS verification
- SES receipt rule storing incoming mail to S3 and forwarding via SNS
- SNS topic + email subscription for forwarding to your real inbox (needed for USPS confirmation)
- Single S3 bucket with key prefixes (`emails/` for incoming, `output/` for results)
- Lambda function (container image) triggered by S3 events under `emails/`
- ECR repository
- IAM roles with least-privilege access scoped to key prefixes

### Setup

```bash
cd terraform
terraform init
terraform apply -var="forward_email=your-real@gmail.com"
```

Note: Domain registration is not yet supported by Terraform. After the first apply, register the domain via CLI:

```bash
aws route53domains register-domain \
  --domain-name "$(terraform output -raw domain_name)" \
  --duration-in-years 1 --auto-renew \
  --admin-contact '{"FirstName":"...","LastName":"...","ContactType":"PERSON","Email":"...","PhoneNumber":"+1.555...","CountryCode":"US","State":"CA","City":"...","AddressLine1":"...","ZipCode":"..."}' \
  --registrant-contact <same> --tech-contact <same>
```

Then re-run `terraform apply` to configure DNS and SES on the new domain.

## Requirements

- Rust 1.85+
- AWS credentials with Bedrock access (Claude Sonnet)
- For S3 input: AWS credentials with S3 read access
- For email fetching: Gmail account with IMAP enabled and app password
