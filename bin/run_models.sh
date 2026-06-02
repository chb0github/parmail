#!/usr/bin/env bash
set -euo pipefail

MODELS_FILE="models.default.json"
MODELS=()
INPUT_DIR="emails"
OUTPUT_DIR="results"
OVERWRITE=false
SAVE_RESPONSES=false

usage() {
  echo "Usage: $0 [-m model]... [-f models_file] [-i input_dir] [-o output_dir] [--overwrite] [--save-responses]"
  echo ""
  echo "Options:"
  echo "  -m MODEL         Run a specific model (repeatable)"
  echo "  -f FILE          Models config file (default: models.default.json)"
  echo "  -i DIR           Input directory of .eml files (default: emails)"
  echo "  -o DIR           Output base directory (default: results)"
  echo "  --overwrite      Delete existing results before rerunning"
  echo "  --save-responses Pass --save-responses to parmail for raw response capture"
  exit 1
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -m) MODELS+=("$2"); shift 2 ;;
      -f) MODELS_FILE="$2"; shift 2 ;;
      -i) INPUT_DIR="$2"; shift 2 ;;
      -o) OUTPUT_DIR="$2"; shift 2 ;;
      --overwrite) OVERWRITE=true; shift ;;
      --save-responses) SAVE_RESPONSES=true; shift ;;
      -h|--help) usage ;;
      *) usage ;;
    esac
  done
}

short_name() {
  local model="${1:?short_name requires a model argument}"
  local short="${model##*.}"
  echo "${short%%:*}"
}

# Outputs model IDs to stdout, one per line
resolve_models() {
  local models_file="${1:?resolve_models requires models_file argument}"
  if [[ ! -f "$models_file" ]]; then
    echo "ERROR: models file not found: $models_file" >&2
    return 1
  fi
  jq -r 'keys[]' "$models_file"
}

# Outputs model IDs that have NOT been run, one per line
filter_completed() {
  local output_dir="${1:?filter_completed requires output_dir argument}"
  shift
  local models=("$@")

  for model in "${models[@]}"; do
    local dir
    dir="${output_dir}/$(short_name "$model")"
    if [[ "$OVERWRITE" == "true" || ! -d "$dir" ]]; then
      echo "$model"
    fi
  done
}

# Outputs model IDs that HAVE been run, one per line
filter_skipped() {
  local output_dir="${1:?filter_skipped requires output_dir argument}"
  shift
  local models=("$@")

  for model in "${models[@]}"; do
    local dir
    dir="${output_dir}/$(short_name "$model")"
    if [[ "$OVERWRITE" == "false" && -d "$dir" ]]; then
      echo "$model"
    fi
  done
}

run_models() {
  local output_dir="${1:?run_models requires output_dir argument}"
  local input_dir="${2:?run_models requires input_dir argument}"
  local models_file="${3:?run_models requires models_file argument}"
  local save_responses="${4:?run_models requires save_responses argument}"
  local overwrite="${5:-false}"
  shift 5
  local models=("$@")
  if [[ ! -d "$input_dir" ]]; then
    echo "ERROR: input directory not found: $input_dir" >&2
    return 1
  fi
  if [[ ${#models[@]} -eq 0 ]]; then
    return 0
  fi

  local extra_args=""
  if [[ "$save_responses" == "true" ]]; then
    extra_args="--save-responses"
  fi

  mkdir -p "${output_dir}/.logs"

  for model in "${models[@]}"; do
    local short
    short=$(short_name "$model")
    if [[ "$overwrite" == "true" ]]; then
      rm -rf "${output_dir:?}/${short:?}"
    fi
    echo "  Starting: ${short} (${model})"
    ./target/release/parmail process --model "$model" --models-file "$models_file" \
      $extra_args --storage-dir "${output_dir}/${short}" "$input_dir" \
      > "${output_dir}/.logs/${short}.log" 2>&1 &
  done

  wait || true

  for model in "${models[@]}"; do
    local short
    short=$(short_name "$model")
    local log="${output_dir}/.logs/${short}.log"
    local ok err
    ok=$(grep -c "^OK" "$log" 2>/dev/null || echo 0)
    err=$(grep -c "^ERROR" "$log" 2>/dev/null || echo 0)
    echo "  Done: ${short} (${ok} ok, ${err} errors)"
  done
}

# --- main ---

parse_args "$@"

if [[ ${#MODELS[@]} -eq 0 ]]; then
  readarray -t MODELS < <(resolve_models "$MODELS_FILE")
fi
if [[ ${#MODELS[@]} -eq 0 ]]; then
  echo "ERROR: no models to run" >&2
  exit 1
fi

readarray -t SKIPPED < <(filter_skipped "$OUTPUT_DIR" "${MODELS[@]}")
readarray -t TO_RUN < <(filter_completed "$OUTPUT_DIR" "${MODELS[@]}")

for model in "${SKIPPED[@]}"; do
  [[ -n "$model" ]] || break
  echo "  Skipping: $(short_name "$model") (results exist, use --overwrite to rerun)"
done

if [[ ${#TO_RUN[@]} -eq 0 ]]; then
  echo "Nothing to run. All models already have results."
  exit 0
fi

echo "Running ${#TO_RUN[@]} models against ${INPUT_DIR} → ${OUTPUT_DIR} (${#SKIPPED[@]} skipped)..."
run_models "$OUTPUT_DIR" "$INPUT_DIR" "$MODELS_FILE" "$SAVE_RESPONSES" "$OVERWRITE" "${TO_RUN[@]}"
