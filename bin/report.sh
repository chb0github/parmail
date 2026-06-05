#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JQ_DIR="${SCRIPT_DIR}/jq"
DATA_DIR="results"
OUTPUT_DIR="reports"
REPORTS=()

available_reports() {
  find "$JQ_DIR" -maxdepth 1 -name "*.jq" ! -name "shared.jq" -exec basename {} .jq \; | sort
}

completion() {
  cat <<'EOF'
# Add to ~/.zshrc:
alias report='./bin/report.sh'
_report() { local -a reports; reports=($(./bin/report.sh --list-reports 2>/dev/null)); [[ ${words[CURRENT-1]} == "-r" ]] && compadd -a reports || _default }
compdef _report report
EOF
  exit 0
}

usage() {
  echo "Usage: $0 [options]"
  echo ""
  echo "Options:"
  echo "  -i DIR         Data directory containing manifest.json files (default: ./results)"
  echo "  -o DIR         Output directory for CSV files (default: ./reports)"
  echo "  -r REPORT      Report to generate (can specify multiple times, default: all)"
  echo "  -h, --help     Show this help"
  echo "  --completion   Show zsh completion setup"
  echo "  --list-reports List available reports (one per line)"
  echo ""
  echo "Available reports:"
  for r in $(available_reports); do
    local desc
    desc=$(jq -nr "include \"$r\"; describe" -L "$JQ_DIR" --argjson prices '{}' 2>/dev/null || echo "")
    printf "  %-15s %s\n" "$r" "$desc"
  done
  echo ""
  echo "Examples:"
  echo "  $0                        # Generate all reports"
  echo "  $0 -r errors              # Generate errors report only"
  echo "  $0 -r errors -r addresses # Generate errors and addresses reports"
  echo "  $0 -o /tmp/reports        # Generate all reports to /tmp/reports"
  exit 1
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -i) DATA_DIR="$2"; shift 2 ;;
      -o) OUTPUT_DIR="$2"; shift 2 ;;
      -r) REPORTS+=("$2"); shift 2 ;;
      -h|--help) usage ;;
      --completion) completion ;;
      --list-reports) available_reports; exit 0 ;;
      -*) echo "ERROR: unknown option $1" >&2; usage ;;
      *) usage ;;
    esac
  done
}

run_report() {
  local report="${1:?run_report requires report name}"
  local datafile="${2:?run_report requires datafile}"

  if [[ ! -f "${JQ_DIR}/${report}.jq" ]]; then
    echo "ERROR: unknown report '$report' (no ${JQ_DIR}/${report}.jq)" >&2
    return 1
  fi

  jq -r \
    "include \"shared\";
     include \"$report\";
     execute | to_csv" \
    -L "$JQ_DIR" \
    --argjson prices "$(cat "$JQ_DIR/prices.json")" \
    "$datafile"
}

main() {
  local data_dir="${1:?main requires data_dir}"
  local output_dir="${2:?main requires output_dir}"
  shift 2
  local reports=("$@")

  if [[ ! -d "$data_dir" ]]; then
    echo "ERROR: data directory not found: $data_dir" >&2
    exit 1
  fi

  local datafile
  datafile=$(mktemp)
  find "$data_dir" -name "manifest.json" -exec cat {} + | jq -s '.' > "$datafile"
  [[ "${DEBUG:-0}" == "1" ]] && echo "data: $datafile" >&2

  mkdir -p "$output_dir"

  for report in "${reports[@]}"; do
    (run_report "$report" "$datafile" > "${output_dir}/${report}.csv") &
  done
  wait

  rm -f "$datafile" || true

  for report in "${reports[@]}"; do
    echo "$(cd "$output_dir" && pwd)/${report}.csv"
  done
}

parse_args "$@"

if [[ ${#REPORTS[@]} -eq 0 ]]; then
  while IFS= read -r r; do
    REPORTS+=("$r");
  done < <(available_reports)
fi
mapfile -t REPORTS < <(printf '%s\n' "${REPORTS[@]}" | sort -u)

main "$DATA_DIR" "$OUTPUT_DIR" "${REPORTS[@]}"
