#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JQ_DIR="${SCRIPT_DIR}/jq"
DATA_DIR="results"
OUTPUT_DIR=""
REPORTS=()

available_reports() {
  find "$JQ_DIR" -maxdepth 1 -name "*.jq" ! -name "shared.jq" -exec basename {} .jq \; | sort
}

usage() {
  echo "Usage: $0 [-i data_dir] [-o output_dir] -r report [-r report]..."
  echo ""
  echo "Options:"
  echo "  -i DIR    Data directory containing manifest.json files (default: ./results)"
  echo "  -o DIR    Write CSV files to this directory"
  echo "  -r NAME   Report to generate (may specify multiple)"
  echo "  -h        Show this help"
  echo ""
  echo "Reports:"
  for r in $(available_reports); do
    local desc
    desc=$(jq -nr "include \"$r\"; describe" -L "$JQ_DIR" --argjson prices '{}' 2>/dev/null || echo "")
    printf "  %-15s %s\n" "$r" "$desc"
  done
  echo ""
  echo "  all             Run all reports"
  exit 1
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -i) DATA_DIR="$2"; shift 2 ;;
      -o) OUTPUT_DIR="$2"; shift 2 ;;
      -r) REPORTS+=("$2"); shift 2 ;;
      -h|--help) usage ;;
      *) usage ;;
    esac
  done
}

run_report() {
  local report="${1:?run_report requires report name}"
  local data="$2"

  if [[ ! -f "${JQ_DIR}/${report}.jq" ]]; then
    echo "ERROR: unknown report '$report' (no ${JQ_DIR}/${report}.jq)" >&2
    return 1
  fi

  echo "$data" | jq -r \
    "include \"shared\";
     include \"$report\";
     execute | to_csv" \
    -L "$JQ_DIR" \
    --argjson prices "$(cat "$JQ_DIR/prices.json")"
}

parse_args "$@"

if [[ ${#REPORTS[@]} -eq 0 ]]; then
  usage
fi

# Expand "all" and deduplicate
expanded=()
for report in "${REPORTS[@]}"; do
  if [[ "$report" == "all" ]]; then
    while IFS= read -r r; do expanded+=("$r"); done < <(available_reports)
  else
    expanded+=("$report")
  fi
done
REPORTS=($(printf '%s\n' "${expanded[@]}" | sort -u))

# Load all manifests as a flat array
if [[ ! -d "$DATA_DIR" ]]; then
  echo "ERROR: data directory not found: $DATA_DIR" >&2
  exit 1
fi
DATA=$(find "$DATA_DIR" -name "manifest.json" -exec cat {} + | jq -s '.')

# Create output dir if needed
if [[ -n "$OUTPUT_DIR" ]]; then
  mkdir -p "$OUTPUT_DIR"
fi

# Run reports
if [[ -n "$OUTPUT_DIR" ]]; then
  for report in "${REPORTS[@]}"; do
    (run_report "$report" "$DATA" > "${OUTPUT_DIR}/${report}.csv" \
      && echo "${OUTPUT_DIR}/${report}.csv") &
  done
  wait
else
  for report in "${REPORTS[@]}"; do
    if [[ ${#REPORTS[@]} -gt 1 ]]; then
      echo "--- $report ---"
    fi
    run_report "$report" "$DATA"
    if [[ ${#REPORTS[@]} -gt 1 ]]; then
      echo ""
    fi
  done
fi
