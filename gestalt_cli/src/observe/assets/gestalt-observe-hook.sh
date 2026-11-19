#!/bin/sh
# Gestalt Observation Hook
# Does POST to $GESTALT_OBSERVE_URL (default: http://127.0.0.1:8081/api/event)
# with 10s timeout, fail-open, exit 0.

AGENT="${1:-unknown-agent}"
EVENT_TYPE="${2:-run_started}"
SUMMARY="${3:-Agent execution event}"

# Fail-open: verify curl is installed
if ! command -v curl >/dev/null 2>&1; then
  exit 0
fi

URL="${GESTALT_OBSERVE_URL:-http://127.0.0.1:8081/api/event}"

# URL validation: non-loopback URLs require https://
is_valid_url() {
  case "$1" in
    https://*)
      return 0
      ;;
    http://*)
      rest="${1#http://}"
      host_port="${rest%%/*}"
      case "$host_port" in
        "["*"]"*)
          host="${host_port%%\]*}]"
          ;;
        *:*)
          host="${host_port%%:*}"
          ;;
        *)
          host="$host_port"
          ;;
      esac
      case "$host" in
        localhost|"[""::1""]")
          return 0
          ;;
        127.*)
          o1="${host%%.*}"
          r1="${host#*.}"
          o2="${r1%%.*}"
          r2="${r1#*.}"
          o3="${r2%%.*}"
          o4="${r2#*.}"
          case "$o1" in *[!0-9]*|"") return 1;; esac
          case "$o2" in *[!0-9]*|"") return 1;; esac
          case "$o3" in *[!0-9]*|"") return 1;; esac
          case "$o4" in *[!0-9]*|"") return 1;; esac
          return 0
          ;;
        *)
          return 1
          ;;
      esac
      ;;
    *)
      return 1
      ;;
  esac
}

if ! is_valid_url "$URL"; then
  exit 0
fi

# Build JSON payload safely without raw string interpolation
if command -v jq >/dev/null 2>&1; then
  PAYLOAD=$(jq -c -n \
    --arg agent "$AGENT" \
    --arg event_type "$EVENT_TYPE" \
    --arg summary "$SUMMARY" \
    '{"agent": $agent, "event_type": $event_type, "summary": $summary}')
elif command -v python3 >/dev/null 2>&1; then
  PAYLOAD=$(python3 -c 'import json, sys; sys.stdout.write(json.dumps({"agent": sys.argv[1], "event_type": sys.argv[2], "summary": sys.argv[3]}))' "$AGENT" "$EVENT_TYPE" "$SUMMARY")
elif command -v python >/dev/null 2>&1; then
  PAYLOAD=$(python -c 'import json, sys; sys.stdout.write(json.dumps({"agent": sys.argv[1], "event_type": sys.argv[2], "summary": sys.argv[3]}))' "$AGENT" "$EVENT_TYPE" "$SUMMARY")
elif command -v awk >/dev/null 2>&1; then
  escape_posix() {
    printf '%s' "$1" | awk '{
      if (NR > 1) printf "\\n"
      gsub(/\\/, "\\\\")
      gsub(/"/, "\\\"")
      gsub(/\r/, "\\r")
      gsub(/\t/, "\\t")
      printf "%s", $0
    }'
  }
  ESC_A=$(escape_posix "$AGENT")
  ESC_E=$(escape_posix "$EVENT_TYPE")
  ESC_S=$(escape_posix "$SUMMARY")
  PAYLOAD=$(printf '{"agent":"%s","event_type":"%s","summary":"%s"}' "$ESC_A" "$ESC_E" "$ESC_S")
else
  exit 0
fi

if [ -z "$PAYLOAD" ]; then
  exit 0
fi

# Optional GESTALT_OBSERVE_TOKEN sent via Authorization: Bearer <token>
# Pass curl config via stdin (-K -) to avoid leaking token in ps output
CURL_CFG="header = \"Content-Type: application/json\"
"
if [ -n "$GESTALT_OBSERVE_TOKEN" ]; then
  ESCAPED_TOKEN=$(printf '%s' "$GESTALT_OBSERVE_TOKEN" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g')
  CURL_CFG="${CURL_CFG}header = \"Authorization: Bearer ${ESCAPED_TOKEN}\"
"
fi

# Fail-open POST using curl with --max-time 10 and silent failure
printf '%s' "$CURL_CFG" | curl -s -X POST \
  -K - \
  -d "$PAYLOAD" \
  --max-time 10 \
  "$URL" >/dev/null 2>&1 || :

exit 0
