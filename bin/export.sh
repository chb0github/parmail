#!/usr/bin/env zsh
set -Eeuo pipefail

SCRIPT_DIR="${0:A:h}"

usage() {
  echo "Usage: $0 [-i data_dir]" >&2
  echo "" >&2
  echo "Export all manifests to CSV (one row per mail piece)." >&2
  echo "Output goes to stdout." >&2
  echo "" >&2
  echo "Options:" >&2
  echo "  -i DIR    Input data directory (default: ./data)" >&2
  echo "  -h        Show this help" >&2
  exit 0
}

main() {
  local data_dir=${1:?data_dir required}

  find "$data_dir" -name "manifest.json" -exec cat {} \; \
    | jq -sre -f "${SCRIPT_DIR}/jq/export.jq"
}

data_dir="${PARMAIL_DATA:-./data}"

while getopts "i:h" opt; do
  case $opt in
    i) data_dir="$OPTARG" ;;
    h) usage ;;
    *) usage ;;
  esac
done

main "$data_dir"
