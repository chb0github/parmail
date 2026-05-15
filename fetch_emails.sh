#!/usr/bin/env zsh
set -Eeuo pipefail

main() {
  local outdir="samples"
  local mailbox="imaps://imap.gmail.com/%5BGmail%5D/All%20Mail"
  local search_from="USPSInformeddelivery@email.informeddelivery.usps.com"
  local parallel=10

  if [[ ! -f ~/.netrc ]] || ! grep -q "imap.gmail.com" ~/.netrc; then
    echo "Error: ~/.netrc missing or has no entry for imap.gmail.com" >&2
    echo "Add the following to ~/.netrc (chmod 600):" >&2
    echo "  machine imap.gmail.com" >&2
    echo "  login your-email@gmail.com" >&2
    echo "  password YOUR_APP_PASSWORD (no spaces)" >&2
    echo "" >&2
    echo "Create an app password at: https://myaccount.google.com/apppasswords" >&2
    exit 1
  fi

  while getopts "o:f:p:m:h" opt; do
    case $opt in
      o) outdir="$OPTARG" ;;
      f) search_from="$OPTARG" ;;
      p) parallel="$OPTARG" ;;
      m) mailbox="$OPTARG" ;;
      h) usage ;;
      *) usage ;;
    esac
  done

  mkdir -p "$outdir"

  echo "Searching for emails from $search_from..."
  local uids
  uids=$(curl -fns --url "$mailbox" \
    --request "UID SEARCH FROM \"$search_from\"" \
    | cut -d' ' -f3-)

  local count
  count=$(echo "$uids" | wc -w | tr -d ' ')
  echo "Found $count messages. Downloading with $parallel parallel connections..."

  echo "$uids" | xargs -n 1 -P "$parallel" -I {} \
    curl -fns --url "$mailbox;UID={}" -o "$outdir/{}.eml"

  echo "Done. Downloaded to $outdir/"
}

usage() {
  echo "Usage: $0 [OPTIONS]"
  echo ""
  echo "Options:"
  echo "  -o DIR     Output directory (default: samples)"
  echo "  -f FROM    Filter by sender (default: USPSInformeddelivery@email.informeddelivery.usps.com)"
  echo "  -p NUM     Parallel downloads (default: 10)"
  echo "  -m URL     IMAP mailbox URL (default: imaps://imap.gmail.com/%5BGmail%5D/All%20Mail)"
  echo "  -h         Show this help"
  exit 0
}

main "$@"
