#!/usr/bin/env bash
# map-repo.sh — Use Claude to generate a semantic code graph for any repository
#
# Usage:
#   ./map-repo.sh /path/to/repo              # outputs code-graph.json
#   ./map-repo.sh /path/to/repo -o out.json  # custom output path
#   ./map-repo.sh /path/to/repo --view       # generate + open in TUI immediately
#
# Requires: claude CLI (Claude Code)

set -euo pipefail

REPO_PATH="${1:-.}"
REPO_PATH="$(cd "$REPO_PATH" && pwd)"
OUTPUT="code-graph.json"
VIEW=false
DRY_RUN=false

shift || true
while [[ $# -gt 0 ]]; do
    case "$1" in
        -o|--output) OUTPUT="$2"; shift 2 ;;
        --view) VIEW=true; shift ;;
        --dry-run) DRY_RUN=true; shift ;;
        *) shift ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Collect file listing for the repo
echo "Scanning $REPO_PATH ..." >&2

FILE_LIST=$(find "$REPO_PATH/src" -type f \( \
    -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" \
    -o -name "*.py" -o -name "*.go" -o -name "*.rb" -o -name "*.java" -o -name "*.kt" \
    -o -name "*.swift" -o -name "*.c" -o -name "*.cpp" -o -name "*.h" \
    -o -name "*.cs" -o -name "*.ex" -o -name "*.exs" -o -name "*.clj" \
    \) ! -path "*/node_modules/*" ! -path "*/target/*" ! -path "*/.git/*" \
    ! -path "*/dist/*" ! -path "*/build/*" ! -name "*.d.ts" \
    2>/dev/null || find "$REPO_PATH" -maxdepth 4 -type f \( \
    -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" \
    -o -name "*.py" -o -name "*.go" -o -name "*.rb" -o -name "*.java" -o -name "*.kt" \
    \) ! -path "*/node_modules/*" ! -path "*/target/*" ! -path "*/.git/*" \
    ! -path "*/dist/*" ! -path "*/build/*" ! -name "*.d.ts" \
    2>/dev/null) || true

FILE_COUNT=$(echo "$FILE_LIST" | grep -c . || echo 0)
echo "Found $FILE_COUNT source files" >&2

if [ "$FILE_COUNT" -eq 0 ]; then
    echo "No source files found in $REPO_PATH" >&2
    exit 1
fi

# Build the prompt with file contents
PROMPT=$(cat <<'PROMPT_END'
Analyze this codebase and produce a JSON code graph. Read all the files I'm about to show you, then output ONLY valid JSON matching this schema:

```json
{
  "entities": [
    {
      "id": 0,
      "name": "EntityName",
      "owner": "ParentClass or null",
      "kind": "function|struct|enum|trait|module|component|hook|middleware|route|config|test|other",
      "file": "relative/path.rs",
      "line": 1,
      "end_line": 10,
      "description": "One sentence: what this entity does"
    }
  ],
  "relations": [
    {
      "from": 0,
      "to": 1,
      "kind": "calls|usestype|contains|implements|imports|configures|tests|validates|renders|handleserror|dependson",
      "label": "optional: why this relation exists"
    }
  ]
}
```

Rules:
- IDs must be sequential starting from 0
- Focus on the IMPORTANT entities — skip trivial helpers, getters/setters, re-exports
- Aim for 30-100 entities depending on codebase size
- Capture the architectural skeleton: main modules, key types, important functions
- Relations should show how the system fits together, not every single call
- Use semantic relation kinds: "renders" for UI components, "validates" for validation logic, "configures" for setup code, "tests" for test files, etc.
- Descriptions should explain the PURPOSE, not restate the name
- file paths should be relative to the repo root

Output ONLY the JSON object, no markdown fences, no explanation.

Here are the source files:

PROMPT_END
)

# Append file contents to prompt
while IFS= read -r file; do
    [ -z "$file" ] && continue
    REL_PATH="${file#$REPO_PATH/}"
    CONTENT=$(head -200 "$file" 2>/dev/null || true)  # Cap at 200 lines per file
    PROMPT="$PROMPT
--- $REL_PATH ---
$CONTENT
--- end ---
"
done <<< "$FILE_LIST"

if [ "$DRY_RUN" = true ]; then
    echo "$PROMPT"
    exit 0
fi

echo "Sending to Claude for analysis..." >&2

# Use claude CLI to generate the graph
RESULT=$(echo "$PROMPT" | claude -p 2>/dev/null)

# Extract JSON from response (handle potential markdown fences)
echo "$RESULT" | sed -n '/^{/,/^}/p' > "$OUTPUT"

# Validate
if [ ! -s "$OUTPUT" ]; then
    echo "Error: Claude did not produce valid JSON output" >&2
    echo "Raw output saved to /tmp/code-graph-raw.txt" >&2
    echo "$RESULT" > /tmp/code-graph-raw.txt
    exit 1
fi

ENTITY_COUNT=$(python3 -c "import json; d=json.load(open('$OUTPUT')); print(len(d['entities']))" 2>/dev/null || echo "?")
RELATION_COUNT=$(python3 -c "import json; d=json.load(open('$OUTPUT')); print(len(d['relations']))" 2>/dev/null || echo "?")

echo "Generated $OUTPUT: $ENTITY_COUNT entities, $RELATION_COUNT relations" >&2

if [ "$VIEW" = true ]; then
    cargo run --manifest-path "$SCRIPT_DIR/Cargo.toml" -- --from-json "$OUTPUT" "$REPO_PATH"
fi
