#!/usr/bin/env bash
set -euo pipefail

# Remove filename and image fields from mailer/content in manifests

RESULTS_DIR="${1:-results}"

if [[ ! -d "$RESULTS_DIR" ]]; then
  echo "ERROR: results directory not found: $RESULTS_DIR" >&2
  exit 1
fi

echo "Removing filename/image fields from manifests in $RESULTS_DIR..."

find "$RESULTS_DIR" -name "manifest.json" -type f | while read manifest; do
  echo "Processing: $manifest"

  # Create backup
  cp "$manifest" "${manifest}.bak"

  # Remove filename and image fields
  jq '
    .mail_pieces |= map(
      (.mailer | del(.filename, .image)) as $m |
      (.content | del(.filename, .image)) as $c |
      .mailer = $m | .content = $c
    )
  ' "${manifest}.bak" > "$manifest"

  # Verify the new manifest is valid JSON
  if ! jq empty "$manifest" 2>/dev/null; then
    echo "ERROR: migration produced invalid JSON, restoring backup" >&2
    mv "${manifest}.bak" "$manifest"
    exit 1
  fi

  # Remove backup on success
  rm "${manifest}.bak"
done

echo "Migration complete!"
