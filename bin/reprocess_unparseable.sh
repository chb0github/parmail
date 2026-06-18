#!/usr/bin/env bash
set -euo pipefail

MODEL="us.anthropic.claude-haiku-4-5-20251001-v1:0"
STORAGE_DIR="results/claude-haiku-4-5-20251001-v1"

# Find all emails with unparseable errors
echo "Finding emails with parsing errors..."
find results/*/*/manifest.json -exec grep -l "No parseable response from model" {} \; 2>/dev/null | while read manifest; do
  hash=$(basename "$(dirname "$manifest")")
  msg_id=$(jq -r '.email_message_id' "$manifest" 2>/dev/null)

  # Find the .eml file
  eml=$(find emails/ -name "*.eml" -exec grep -l "Message-ID:.*$msg_id" {} \; 2>/dev/null | head -1)

  if [[ -n "$eml" && -f "$eml" ]]; then
    echo "Reprocessing: $hash ($msg_id)"
    # Delete old manifest
    rm -rf "results/claude-haiku-4-5-20251001-v1/$hash"

    # Reprocess
    ./target/release/parmail process \
      --model "$MODEL" \
      --storage-dir "$STORAGE_DIR" \
      "$eml" 2>&1 | grep -E "^(OK|ERROR)"
  fi
done

echo "Done!"
