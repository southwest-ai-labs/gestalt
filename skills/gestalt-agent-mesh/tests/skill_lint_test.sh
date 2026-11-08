#!/bin/bash
set -euo pipefail

SKILL_FILE="skills/gestalt-agent-mesh/SKILL.md"

if [ ! -f "$SKILL_FILE" ]; then
  echo "Error: $SKILL_FILE does not exist."
  exit 1
fi

# 1. Verify YAML frontmatter exists
if ! head -n 1 "$SKILL_FILE" | grep -q "^---$"; then
  echo "Error: Missing YAML frontmatter start in $SKILL_FILE"
  exit 1
fi

# 2. Character count >= 2500
CHAR_COUNT=$(wc -m < "$SKILL_FILE")
if [ "$CHAR_COUNT" -lt 2500 ]; then
  echo "Error: $SKILL_FILE is only $CHAR_COUNT characters. Expected >= 2500."
  exit 1
fi

# 3. Document 5 core endpoints
ENDPOINTS=("/api/event" "/api/events" "/api/task" "/healthz" "/api/health/providers")
for ENDPOINT in "${ENDPOINTS[@]}"; do
  if ! grep -q "$ENDPOINT" "$SKILL_FILE"; then
    echo "Error: $SKILL_FILE must document the $ENDPOINT endpoint."
    exit 1
  fi
done

echo "OK: $SKILL_FILE passed lint checks."
exit 0
