#!/usr/bin/env zsh
set -Eeuo pipefail

SCRIPT_DIR="${0:A:h}"
JQ_DIR="${SCRIPT_DIR}/jq"

available_reports() {
  find "$JQ_DIR" -name "*.jq" -exec basename {} .jq \; | sort
}

usage() {
  echo "Usage: $0 [-r report] [-i data_dir] [-o output_dir]" >&2
  echo "" >&2
  echo "Generate CSV reports from parmail manifest data." >&2
  echo "Default: all reports." >&2
  echo "" >&2
  echo "Options:" >&2
  echo "  -r NAME   Report to generate (may specify multiple, duplicates ignored)" >&2
  echo "  -i DIR    Input data directory (default: ./data)" >&2
  echo "  -o DIR    Output directory (default: ./reports)" >&2
  echo "  -h        Show this help" >&2
  echo "" >&2
  echo "Available reports:" >&2
  for r in $(available_reports); do
    local desc=$(head -1 "${JQ_DIR}/${r}.jq" | sed 's/^# *//')
    printf "  %-6s %s\n" "$r" "$desc" >&2
  done
  exit 1
}

main() {
  local data_dir=${1:?data_dir required}
  local reports_dir=${2:?reports_dir required}
  shift 2

  mkdir -p "$reports_dir"

  for report in "$@"; do
    (find "$data_dir" -name "manifest.json" -exec cat {} \; \
      | jq -sre -f "${JQ_DIR}/${report}.jq" > "${reports_dir}/${report}.csv" \
      && echo "${reports_dir}/${report}.csv") &
  done
  wait
}

data_dir="${PARMAIL_DATA:-./data}"
reports_dir="${PARMAIL_REPORTS:-./reports}"
reports=()

while getopts "r:i:o:h" opt; do
  case $opt in
    r) reports+=("$OPTARG") ;;
    i) data_dir="$OPTARG" ;;
    o) reports_dir="$OPTARG" ;;
    h) usage ;;
    *) usage ;;
  esac
done

if [[ ${#reports[@]} -eq 0 ]]; then
  reports=($(available_reports))
fi

reports=($(printf '%s\n' "${reports[@]}" | sort -u))

validate_reports() {
  local invalid=()
  for report in "$@"; do
    if [[ ! -f "${JQ_DIR}/${report}.jq" ]]; then
      invalid+=("$report")
    fi
  done
  if [[ ${#invalid[@]} -gt 0 ]]; then
    echo "Unknown report(s): ${invalid[*]}" >&2
    echo "" >&2
    usage
  fi
}

validate_reports "${reports[@]}"

main "$data_dir" "$reports_dir" "${reports[@]}"
