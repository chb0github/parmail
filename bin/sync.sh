#!/usr/bin/env bash
set -euo pipefail

BUCKET="parmail-692140489268"
MODEL="claude-haiku-4-5-20251001-v1"
RESULTS_DIR="results"
EMAILS_DIR="emails"
DELETE_REMOTE=false
RECONCILE=false
DRY_RUN=false
AWS_ARGS="--profile thetaone"

usage() {
  echo "Usage: $0 [--reconcile] [--delete-remote] [--dry-run] [-m model] [-i results_dir] [-e emails_dir]"
  echo ""
  echo "Sync local results to S3 and fill processing gaps."
  echo ""
  echo "Options:"
  echo "  --reconcile      Pull emails from S3, process unprocessed ones, push results back"
  echo "  --delete-remote  Delete all existing output/ in S3 before uploading"
  echo "  --dry-run        Show what would be done without doing it"
  echo "  -m MODEL         Model directory to sync (default: $MODEL)"
  echo "  -i DIR           Local results base dir (default: ./results)"
  echo "  -e DIR           Local emails dir (default: ./emails)"
  echo "  -h               Show this help"
  exit 1
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -m) MODEL="$2"; shift 2 ;;
      -i) RESULTS_DIR="$2"; shift 2 ;;
      -e) EMAILS_DIR="$2"; shift 2 ;;
      --delete-remote) DELETE_REMOTE=true; shift ;;
      --reconcile) RECONCILE=true; shift ;;
      --dry-run) DRY_RUN=true; shift ;;
      -h|--help) usage ;;
      *) usage ;;
    esac
  done
}

sync_up() {
  local source="${RESULTS_DIR}/${MODEL}"

  if [[ ! -d "$source" ]]; then
    echo "ERROR: source directory not found: $source" >&2
    exit 1
  fi

  local manifest_count
  manifest_count=$(find "$source" -name "manifest.json" | wc -l | tr -d ' ')
  echo "Upload: $source ($manifest_count manifests) → s3://${BUCKET}/output/"

  if [[ "$DELETE_REMOTE" == "true" ]]; then
    echo "  Deleting remote output/..."
    if [[ "$DRY_RUN" == "true" ]]; then
      echo "  [dry-run] aws s3 rm s3://${BUCKET}/output/ --recursive"
    else
      aws s3 rm "s3://${BUCKET}/output/" --recursive $AWS_ARGS
    fi
  fi

  if [[ "$DRY_RUN" == "true" ]]; then
    aws s3 sync "$source" "s3://${BUCKET}/output/" $AWS_ARGS --dryrun
  else
    aws s3 sync "$source" "s3://${BUCKET}/output/" $AWS_ARGS
  fi
}

reconcile() {
  local source="${RESULTS_DIR}/${MODEL}"

  # 1. Pull all emails from S3
  echo "Pull: s3://${BUCKET}/emails/ → ${EMAILS_DIR}/"
  if [[ "$DRY_RUN" == "true" ]]; then
    aws s3 sync "s3://${BUCKET}/emails/" "$EMAILS_DIR/" $AWS_ARGS --dryrun
  else
    aws s3 sync "s3://${BUCKET}/emails/" "$EMAILS_DIR/" $AWS_ARGS
  fi

  local s3_emails local_manifests
  s3_emails=$(ls "$EMAILS_DIR" | wc -l | tr -d ' ')
  local_manifests=$(find "$source" -name "manifest.json" 2>/dev/null | wc -l | tr -d ' ')
  echo "  Emails: $s3_emails, Processed: $local_manifests, Gap: $((s3_emails - local_manifests))"

  # 2. Process unprocessed emails (resume logic skips existing valid manifests)
  if [[ "$DRY_RUN" == "true" ]]; then
    echo "  [dry-run] parmail process --model ... $EMAILS_DIR/"
  else
    echo "  Processing gaps..."
    ./target/release/parmail process \
      --model "us.anthropic.claude-haiku-4-5-20251001-v1:0" \
      --storage-dir "$source" \
      -c 2 "$EMAILS_DIR/"
  fi

  # 3. Push results back to S3
  local new_manifests
  new_manifests=$(find "$source" -name "manifest.json" | wc -l | tr -d ' ')
  echo "  Processed: $new_manifests total (was $local_manifests)"
  echo "Upload: $source → s3://${BUCKET}/output/"

  if [[ "$DRY_RUN" == "true" ]]; then
    aws s3 sync "$source" "s3://${BUCKET}/output/" $AWS_ARGS --dryrun
  else
    aws s3 sync "$source" "s3://${BUCKET}/output/" $AWS_ARGS
  fi
}

main() {
  if [[ "$RECONCILE" == "true" ]]; then
    reconcile
  else
    sync_up
  fi
  echo "Done."
}

parse_args "$@"
main
