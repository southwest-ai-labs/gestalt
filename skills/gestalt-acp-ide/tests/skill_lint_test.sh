#!/usr/bin/env bash
set -e

SKILL_FILE="skills/gestalt-acp-ide/SKILL.md"

if [ ! -f "$SKILL_FILE" ]; then
    echo "Error: $SKILL_FILE does not exist."
    exit 1
fi

# Check YAML frontmatter
if ! head -n 1 "$SKILL_FILE" | grep -q "^---$"; then
    echo "Error: Missing YAML frontmatter start in $SKILL_FILE"
    exit 1
fi

if ! grep -q "name: gestalt-acp-ide" "$SKILL_FILE"; then
    echo "Error: Missing or incorrect name in frontmatter of $SKILL_FILE"
    exit 1
fi

# Check IDE coverage
IDES=("Claude Code" "Cursor" "Zed" "VS Code")
for IDE in "${IDES[@]}"
do
    if ! grep -qi "$IDE" "$SKILL_FILE"; then
        echo "Error: Missing coverage for $IDE in $SKILL_FILE"
        exit 1
    fi
done

echo "Linting passed: $SKILL_FILE covers all required IDEs and has valid frontmatter."
exit 0
