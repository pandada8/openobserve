#!/bin/sh
set -eu
umask 077
AUTH_FILE=$(mktemp)
trap 'rm -f "$AUTH_FILE"' EXIT
AUTH=$(printf '%s:%s' "$ZO_ROOT_USER_EMAIL" "$ZO_ROOT_USER_PASSWORD" | base64 | tr -d '\n')
printf 'header = "Authorization: Basic %s"\n' "$AUTH" > "$AUTH_FILE"
BASE="${ZO_DRAIN_URL:-http://127.0.0.1:5080}"
curl --fail --silent --show-error --max-time 15 --config "$AUTH_FILE" -X PUT "$BASE/node/drain" >/dev/null
DEADLINE=$(( $(date +%s) + ${ZO_INGESTER_DRAIN_TIMEOUT_SECS:-1000} ))
while [ "$(date +%s)" -lt "$DEADLINE" ]; do
  STATUS=$(curl --fail --silent --show-error --max-time 10 --config "$AUTH_FILE" "$BASE/node/drain/status")
  if printf '%s' "$STATUS" | jq -e '.readyForShutdown == true' >/dev/null; then
    exit 0
  fi
  if printf '%s' "$STATUS" | jq -e '.error != null' >/dev/null; then
    printf '%s\n' "$STATUS" >&2
    exit 1
  fi
  sleep 2
done
echo 'Drain timed out: retain PVC; do not treat termination as successful upload' >&2
exit 1
