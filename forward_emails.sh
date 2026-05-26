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
  echo "Usage: $0 -r RECIPIENT [OPTIONS] PATH [PATH...]" >&2
  echo "" >&2
  echo "Forwards .eml files via SMTP. If PATH is a directory, all .eml files in it" >&2
  echo "are forwarded. If PATH is a file, it is forwarded directly." >&2
  echo "" >&2
  echo "Options:" >&2
  echo "  -r ADDR    Recipient email address (required)" >&2
  echo "  -d SECS    Delay between sends in seconds (default: 10)" >&2
  echo "  -h         Show this help" >&2
  exit 0
}

check_netrc() {
  if [[ ! -f ~/.netrc ]] || ! grep -q "smtp.gmail.com" ~/.netrc; then
    echo "Error: ~/.netrc missing or has no entry for smtp.gmail.com" >&2
    echo "Add the following to ~/.netrc (chmod 600):" >&2
    echo "  machine smtp.gmail.com" >&2
    echo "  login your-email@gmail.com" >&2
    echo "  password YOUR_APP_PASSWORD (no spaces)" >&2
    echo "" >&2
    echo "Create an app password at: https://myaccount.google.com/apppasswords" >&2
    return 1
  fi
}

get_from_email() {
  awk '/machine imap.gmail.com/{found=1} found && /login/{print $2; exit}' ~/.netrc
}


collect_files() {
  : ${1:?at least one path required}
  for path in "$@"; do
    if [[ -d "$path" ]]; then
      /usr/bin/find "$path" -name "*.eml" -type f -print0
    else
      printf '%s\0' "$path"
    fi
  done
}

main() {
  local delay=${1:?delay required}
  local recipient=${2:?recipient required}
  : ${3:?at least one path required}
  shift 2

  check_netrc

  local from
  from=$(get_from_email)

  while IFS= read -r -d '' file; do
    if curl -ns $CURL_VERBOSE \
      --url "smtps://smtp.gmail.com:465" \
      --mail-from "$from" \
      --mail-rcpt "$recipient" \
      --upload-file "$file"; then
      echo "$file"
    else
      echo "$file" >&2
    fi
    sleep "$delay"
  done < <(collect_files "$@")
}

delay=10
recipient=""

while getopts "r:d:h" opt; do
  case $opt in
    r) recipient="$OPTARG" ;;
    d) delay="$OPTARG" ;;
    h) usage ;;
    *) usage ;;
  esac
done
shift $((OPTIND - 1))

main "$delay" "$recipient" "$@"
