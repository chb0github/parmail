#!/usr/bin/env bash
set -euo pipefail

# Migrate old manifest format to new format:
# 1. Remove to_address from top level (it was always the same across all pieces anyway)
# 2. Flatten from_address.address -> from_address, add resolved field
# 3. Add to_address to each mail_piece (extract from full_text or set to null)
# 4. Add image field to mailer/content (copy filename)

RESULTS_DIR="${1:-results}"

if [[ ! -d "$RESULTS_DIR" ]]; then
  echo "ERROR: results directory not found: $RESULTS_DIR" >&2
  exit 1
fi

echo "Migrating manifests in $RESULTS_DIR..."

find "$RESULTS_DIR" -name "manifest.json" -type f | while read manifest; do
  echo "Migrating: $manifest"

  # Create backup
  cp "$manifest" "${manifest}.bak"

  # Apply migration with jq
  jq '
    # Remove top-level to_address
    del(.to_address) |

    # Transform mail_pieces
    .mail_pieces |= map(
      # Flatten from_address and add resolved field
      (.from_address.status == "resolved") as $from_resolved |
      if .from_address.address then
        .from_address = .from_address.address + {resolved: $from_resolved}
      else
        .from_address = {resolved: false}
      end |

      # Add to_address (null for now - models didnt extract it anyway)
      .to_address = null |

      # Remove filename and image from mailer/content (redundant with hash)
      if .mailer then
        .mailer = .mailer | del(.filename, .image)
      end |
      if .content then
        .content = .content | del(.filename, .image)
      end
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
