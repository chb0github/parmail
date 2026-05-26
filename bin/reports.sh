#!/usr/bin/env zsh
set -Eeuo pipefail

DATA_DIR="${PARMAIL_DATA:-./data}"
REPORTS_DIR="${PARMAIL_REPORTS:-./reports}"
PARQUET="${DATA_DIR}/parmail.parquet"

usage() {
  echo "Usage: $0 [OPTIONS]" >&2
  echo "" >&2
  echo "Generate CSV reports from parmail data." >&2
  echo "Requires parquet file at ${PARQUET} (run: parmail export)" >&2
  echo "" >&2
  echo "Options:" >&2
  echo "  -s    Top senders by frequency" >&2
  echo "  -t    Mail type breakdown" >&2
  echo "  -v    Volume over time (monthly)" >&2
  echo "  -q    Data quality (address resolution rates)" >&2
  echo "  -d    Duplicate/repeat senders (unsubscribe candidates)" >&2
  echo "  -a    All reports" >&2
  echo "  -o    Output directory (default: ./reports)" >&2
  echo "  -h    Show this help" >&2
  exit 0
}

top_senders() {
  find "$DATA_DIR" -name "manifest.json" -exec cat {} \; \
    | jq -r '.mail_pieces[] | select(.from_address.status == "resolved") | .from_address.address | [.name // "unknown", .street // "", .city // "", .state // "", .zip // ""] | @csv' \
    | sort | uniq -c | sort -rn \
    | awk '{count=$1; $1=""; gsub(/^ /, ""); print count","$0}' \
    > "${REPORTS_DIR}/top_senders.csv"
  sed -i '' '1i\
count,name,street,city,state,zip
' "${REPORTS_DIR}/top_senders.csv"
  echo "  top_senders.csv ($(wc -l < "${REPORTS_DIR}/top_senders.csv") rows)"
}

mail_type_breakdown() {
  local total
  total=$(find "$DATA_DIR" -name "manifest.json" -exec cat {} \; | jq -r '.mail_pieces[].mail_type' | wc -l | tr -d ' ')

  find "$DATA_DIR" -name "manifest.json" -exec cat {} \; \
    | jq -r '.mail_pieces[].mail_type' \
    | sort | uniq -c | sort -rn \
    | awk -v total="$total" '{count=$1; type=$2; pct=sprintf("%.1f", count/total*100); print type","count","pct}' \
    > "${REPORTS_DIR}/mail_type_breakdown.csv"
  sed -i '' '1i\
mail_type,count,pct
' "${REPORTS_DIR}/mail_type_breakdown.csv"
  echo "  mail_type_breakdown.csv ($(wc -l < "${REPORTS_DIR}/mail_type_breakdown.csv") rows)"
}

volume_over_time() {
  find "$DATA_DIR" -name "manifest.json" -exec cat {} \; \
    | jq -r '.received_date' \
    | cut -c1-7 \
    | sort | uniq -c | sort -k2 \
    | awk '{print $2","$1}' \
    > "${REPORTS_DIR}/volume_over_time.csv"
  sed -i '' '1i\
month,email_count
' "${REPORTS_DIR}/volume_over_time.csv"
  echo "  volume_over_time.csv ($(wc -l < "${REPORTS_DIR}/volume_over_time.csv") rows)"
}

data_quality() {
  {
    echo "field,status,count"
    find "$DATA_DIR" -name "manifest.json" -exec cat {} \; \
      | jq -r '"to,"+ .to_address.status' \
      | sort | uniq -c | sort -rn \
      | awk '{print "to,"$2","$1}'
    find "$DATA_DIR" -name "manifest.json" -exec cat {} \; \
      | jq -r '.mail_pieces[] | "from,"+ .from_address.status' \
      | sort | uniq -c | sort -rn \
      | awk '{print "from,"$2","$1}'
  } > "${REPORTS_DIR}/data_quality.csv"
  echo "  data_quality.csv ($(wc -l < "${REPORTS_DIR}/data_quality.csv") rows)"
}

duplicate_senders() {
  find "$DATA_DIR" -name "manifest.json" -exec cat {} \; \
    | jq -r '
      .received_date as $date |
      .mail_pieces[] |
      select(.from_address.status == "resolved") |
      [.from_address.address.name // "unknown", .from_address.address.city // "", .from_address.address.state // "", .mail_type, $date] | @csv' \
    | sort -t, -k1,1 \
    | awk -F, '{
        key=$1","$2","$3","$4;
        count[key]++;
        if (!first[key]) first[key]=$5;
        last[key]=$5
      }
      END {
        for (k in count) if (count[k]>=5) print count[k]","k","first[k]","last[k]
      }' \
    | sort -t, -k1 -rn \
    > "${REPORTS_DIR}/duplicate_senders.csv"
  sed -i '' '1i\
count,name,city,state,mail_type,first_seen,last_seen
' "${REPORTS_DIR}/duplicate_senders.csv"
  echo "  duplicate_senders.csv ($(wc -l < "${REPORTS_DIR}/duplicate_senders.csv") rows)"
}

reports=()

while getopts "stvqdo:ah" opt; do
  case $opt in
    s) reports+=(top_senders) ;;
    t) reports+=(mail_type_breakdown) ;;
    v) reports+=(volume_over_time) ;;
    q) reports+=(data_quality) ;;
    d) reports+=(duplicate_senders) ;;
    o) REPORTS_DIR="$OPTARG" ;;
    a) reports=(top_senders mail_type_breakdown volume_over_time data_quality duplicate_senders) ;;
    h) usage ;;
    *) usage ;;
  esac
done

: ${reports[1]:?at least one report flag required (-s -t -v -q -d or -a)}

mkdir -p "$REPORTS_DIR"
echo "Generating reports in ${REPORTS_DIR}:"

for report in "${reports[@]}"; do
  $report
done
