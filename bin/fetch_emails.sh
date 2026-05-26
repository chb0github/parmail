#!/usr/bin/env zsh
set -Eeuo pipefail

if [[ ${DEBUG:-0} -ge 1 ]]; then
  set -x;
fi
if [[ ${DEBUG:-0} -ge 2 ]]; then
  CURL_VERBOSE="-v";
  else CURL_VERBOSE="";
fi

usage() {
  echo "Usage: $0 [OPTIONS]" >&2
  echo "" >&2
  echo "Options:" >&2
  echo "  -o DIR     Output directory (default: samples)" >&2
  echo "  -f FROM    Filter by sender (default: USPSInformeddelivery@email.informeddelivery.usps.com)" >&2
  echo "  -s SUBJ    Filter by subject (default: Your Daily Digest)" >&2
  echo "  -p NUM     Parallel downloads (default: 10)" >&2
  echo "  -m URL     IMAP mailbox URL (default: imaps://imap.gmail.com/%5BGmail%5D/All%20Mail)" >&2
  echo "  -n         Dry run: print message count and exit" >&2
  echo "  -h         Show this help" >&2
  exit 0
}

check_netrc() {
  if [[ ! -f ~/.netrc ]] || ! grep -q "imap.gmail.com" ~/.netrc; then
    echo "Error: ~/.netrc missing or has no entry for imap.gmail.com" >&2
    echo "Add the following to ~/.netrc (chmod 600):" >&2
    echo "  machine imap.gmail.com" >&2
    echo "  login your-email@gmail.com" >&2
    echo "  password YOUR_APP_PASSWORD (no spaces)" >&2
    echo "" >&2
    echo "Create an app password at: https://myaccount.google.com/apppasswords" >&2
    return 1
  fi
}

search_uids() {
  local mailbox=$1
  local search_from=$2
  local search_subject=$3

  local uids
  uids=$(curl -fns $CURL_VERBOSE --url "$mailbox" \
    --request "UID SEARCH FROM \"$search_from\" SUBJECT \"$search_subject\"" \
    | cut -d' ' -f3-)

  local count
  count=$(echo "$uids" | wc -w | tr -d ' ')

  echo "$count $uids"
}

main() {
  local outdir=$1
  local mailbox=$2
  local search_from=$3
  local search_subject=$4
  local parallel=$5
  local dry_run=$6

  check_netrc

  mkdir -p "$outdir"

  local count uids
  read -r count uids <<< "$(search_uids "$mailbox" "$search_from" "$search_subject")"

  if $dry_run; then
    echo "$count"
    exit 0
  fi

  echo "$uids" | xargs -n 1 -P "$parallel" -I {} \
    curl -fns $CURL_VERBOSE --url "$mailbox;UID={}" -o "$outdir/{}.eml"
}

outdir="samples"
mailbox="imaps://imap.gmail.com/%5BGmail%5D/All%20Mail"
search_from="USPSInformeddelivery@email.informeddelivery.usps.com"
search_subject="Your Daily Digest"
parallel=10
dry_run=false

while getopts "o:f:s:p:m:nh" opt; do
  case $opt in
    o) outdir="$OPTARG" ;;
    f) search_from="$OPTARG" ;;
    s) search_subject="$OPTARG" ;;
    p) parallel="$OPTARG" ;;
    m) mailbox="$OPTARG" ;;
    n) dry_run=true ;;
    h) usage ;;
    *) usage ;;
  esac
done

main "$outdir" "$mailbox" "$search_from" "$search_subject" "$parallel" "$dry_run"
