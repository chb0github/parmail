# Parmail

## Architecture

Inbound emails arrive via SES → SES drops raw .eml into S3 (`emails/` prefix) → S3 notification triggers the Lambda → Lambda processes them and writes output to S3 (`output/` prefix).

## AWS

All AWS CLI commands must use `--profile thetaone`.
Terraform commands must use `AWS_PROFILE=thetaone` prefix.

## GitHub

This project uses the personal GitHub identity `chb0github`. Switch before any gh/git operations:
```bash
gh auth switch --user chb0github
```
Switch back when done.
