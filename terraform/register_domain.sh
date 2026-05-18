#!/usr/bin/env bash
set -euo pipefail

DOMAIN="${1:-${PARENT_DOMAIN:-}}"

if [[ -z "$DOMAIN" ]]; then
  echo "Error: domain is required." >&2
  echo "Usage: $0 <domain>" >&2
  echo "  or: PARENT_DOMAIN=example.com $0" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEMPLATE="$SCRIPT_DIR/contact_template.json"

if [[ ! -f "$TEMPLATE" ]]; then
  echo "Error: contact_template.json not found at $TEMPLATE" >&2
  exit 1
fi

CONTACT=$(jq \
  --arg first "${FIRST_NAME:-}" \
  --arg last "${LAST_NAME:-}" \
  --arg email "${EMAIL:-}" \
  --arg phone "${PHONE:-}" \
  --arg state "${STATE:-}" \
  --arg city "${CITY:-}" \
  --arg address "${ADDRESS:-}" \
  --arg zip "${ZIP:-}" \
  '.FirstName = $first | .LastName = $last | .Email = $email | .PhoneNumber = $phone | .State = $state | .City = $city | .AddressLine1 = $address | .ZipCode = $zip' \
  "$TEMPLATE")

echo "Registering domain: $DOMAIN"
echo "Contact info:"
echo "$CONTACT" | jq .

aws route53domains register-domain \
  --domain-name "$DOMAIN" \
  --duration-in-years 1 \
  --auto-renew \
  --admin-contact "$CONTACT" \
  --registrant-contact "$CONTACT" \
  --tech-contact "$CONTACT"

echo "Domain registration submitted for: $DOMAIN"
echo "Check status: aws route53domains get-domain-detail --domain-name $DOMAIN"
