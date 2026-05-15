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
./target/release/parmail validate

# Process a single .eml file
./target/release/parmail process message.eml

# Process a directory of .eml files recursively
./target/release/parmail process samples/ --storage-dir ./data

# Run as HTTP server
./target/release/parmail serve --addr 0.0.0.0:3000

# Run as AWS Lambda (S3 event trigger)
./target/release/parmail lambda
```

## Fetching emails

Use the included script to bulk-download USPS Informed Delivery emails from Gmail via IMAP:

```bash
./fetch_emails.sh -o samples/
```

Requires `~/.netrc` with Gmail app password credentials. Run `./fetch_emails.sh -h` for options.

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

## Infrastructure

Terraform configs in `terraform/` provision:
- SES receipt rule for incoming email
- S3 buckets for email storage and images
- Lambda function (container image)
- ECR repository
- IAM roles with least-privilege access to S3 and Bedrock

## Requirements

- Rust 1.85+
- AWS credentials with Bedrock access (Claude Sonnet)
- For email fetching: Gmail account with IMAP enabled and app password
