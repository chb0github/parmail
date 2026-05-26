#!/usr/bin/env zsh
set -Eeuo pipefail

SCRIPT_DIR="${0:A:h}"
JQ_DIR="${SCRIPT_DIR}/jq"

usage() {
  echo "Usage: $0 [OPTIONS]" >&2
  echo "" >&2
  echo "Generate CSV reports from parmail manifest data." >&2
  echo "" >&2
  echo "Options:" >&2
  echo "  -s    Top senders by frequency" >&2
  echo "  -t    Mail type breakdown" >&2
  echo "  -v    Volume over time (monthly)" >&2
  echo "  -q    Data quality (address resolution rates)" >&2
  echo "  -d    Duplicate/repeat senders (unsubscribe candidates)" >&2
  echo "  -a    All reports" >&2
  echo "  -o    Output directory (default: ./reports)" >&2
  echo "  -i    Input data directory (default: ./data)" >&2
  echo "  -h    Show this help" >&2
  exit 0
}

run_report() {
  local data_dir=${1:?data_dir required}
  local report_name=${2:?report_name required}

  local jq_file="${JQ_DIR}/${report_name}.jq"
  if [[ ! -f "$jq_file" ]]; then
    echo "jq file not found: ${jq_file}" >&2
    return 1
  fi

  find "$data_dir" -name "manifest.json" -exec cat {} \; \
    | jq -sre -f "$jq_file"
}

top_senders() {
  run_report "$1" "top_senders"
}

mail_type_breakdown() {
  run_report "$1" "mail_type_breakdown"
}

volume_over_time() {
  run_report "$1" "volume_over_time"
}

data_quality() {
  run_report "$1" "data_quality"
}

duplicate_senders() {
  run_report "$1" "duplicate_senders"
}

data_dir="${PARMAIL_DATA:-./data}"
reports_dir="${PARMAIL_REPORTS:-./reports}"
reports=()

while getopts "stvqdi:o:ah" opt; do
  case $opt in
    s) reports+=(top_senders) ;;
    t) reports+=(mail_type_breakdown) ;;
    v) reports+=(volume_over_time) ;;
    q) reports+=(data_quality) ;;
    d) reports+=(duplicate_senders) ;;
    i) data_dir="$OPTARG" ;;
    o) reports_dir="$OPTARG" ;;
    a) reports=(top_senders mail_type_breakdown volume_over_time data_quality duplicate_senders) ;;
    h) usage ;;
    *) usage ;;
  esac
done

if [[ ${#reports[@]} -eq 0 ]]; then
  reports=(top_senders mail_type_breakdown volume_over_time data_quality duplicate_senders)
fi

mkdir -p "$reports_dir"

for report in "${reports[@]}"; do
  ($report "$data_dir" > "${reports_dir}/${report}.csv" && echo "${reports_dir}/${report}.csv") &
done
wait
