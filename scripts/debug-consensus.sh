#!/bin/bash
# scripts/debug-consensus.sh
# Debug multi-agent consensus for a SPEC+stage

set -e

SPEC_ID="$1"
STAGE="$2"
DB_PATH="${3:-$HOME/.code/consensus_artifacts.db}"

if [ -z "$SPEC_ID" ] || [ -z "$STAGE" ]; then
    cat <<EOF
Usage: $0 <SPEC-ID> <stage> [db-path]

Stages: spec-plan, spec-tasks, spec-implement, spec-validate, spec-audit, spec-unlock

Examples:
  $0 SPEC-KIT-900 spec-plan
  $0 SPEC-KIT-900 spec-tasks ~/.code/consensus_artifacts.db

Shows:
  - All agent outputs (gemini, claude, gpt_pro)
  - Output lengths and timestamps
  - Synthesis summary (consensus/conflicts)
  - Verdict file location
EOF
    exit 1
fi

if ! command -v sqlite3 &> /dev/null; then
    echo "Error: sqlite3 not found. Install with: apt-get install sqlite3"
    exit 1
fi

if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database not found at $DB_PATH"
    exit 1
fi

echo "==================================="
echo "Consensus Debug: $SPEC_ID / $STAGE"
echo "==================================="
echo ""

# 1. Show all agent artifacts
echo "1. Agent Artifacts"
echo "-----------------------------------"
sqlite3 "$DB_PATH" <<SQL
.mode column
.headers on
SELECT
    agent_id,
    length(content) as content_length,
    datetime(created_at, 'unixepoch') as timestamp,
    run_id
FROM consensus_artifacts
WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE'
ORDER BY created_at;
SQL
echo ""

# 2. Show synthesis record
echo "2. Synthesis Record"
echo "-----------------------------------"
sqlite3 "$DB_PATH" <<SQL
.mode column
.headers on
SELECT
    synthesis_id,
    run_id,
    length(output) as output_length,
    output_path,
    datetime(created_at, 'unixepoch') as timestamp
FROM consensus_synthesis
WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE'
ORDER BY created_at DESC
LIMIT 1;
SQL
echo ""

# 3. Get synthesis output path and show preview
SYNTHESIS_PATH=$(sqlite3 "$DB_PATH" "SELECT output_path FROM consensus_synthesis WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE' ORDER BY created_at DESC LIMIT 1" 2>/dev/null || echo "")

if [ -n "$SYNTHESIS_PATH" ] && [ -f "$SYNTHESIS_PATH" ]; then
    echo "3. Synthesis Output Preview"
    echo "-----------------------------------"
    echo "Path: $SYNTHESIS_PATH"
    SIZE=$(wc -c < "$SYNTHESIS_PATH")
    echo "Size: $SIZE bytes"
    echo ""
    echo "Content (first 50 lines):"
    head -50 "$SYNTHESIS_PATH"
    echo ""
    echo "Content (last 20 lines):"
    tail -20 "$SYNTHESIS_PATH"
else
    echo "3. Synthesis output file not found"
fi
echo ""

# 4. Show agent outputs side-by-side (summary)
echo "4. Agent Responses (First 500 chars each)"
echo "-----------------------------------"
for agent in gemini claude gpt_pro gpt_codex code; do
    CONTENT=$(sqlite3 "$DB_PATH" "SELECT substr(content, 1, 500) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE' AND agent_id = '$agent' ORDER BY created_at DESC LIMIT 1" 2>/dev/null || echo "")

    if [ -n "$CONTENT" ]; then
        echo ""
        echo ">>> $agent <<<"
        echo "$CONTENT"
        echo "..."
    fi
done
echo ""

# 5. Check for verdict file
echo "5. Evidence Files"
echo "-----------------------------------"
EVIDENCE_BASE="docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/$SPEC_ID"
STAGE_SHORT="${STAGE#spec-}"  # Remove 'spec-' prefix

if [ -f "${EVIDENCE_BASE}/${STAGE_SHORT}_synthesis.json" ]; then
    echo "✓ Synthesis: ${EVIDENCE_BASE}/${STAGE_SHORT}_synthesis.json"
    SIZE=$(wc -c < "${EVIDENCE_BASE}/${STAGE_SHORT}_synthesis.json")
    echo "  Size: $SIZE bytes"
fi

if [ -f "${EVIDENCE_BASE}/${STAGE_SHORT}_verdict.json" ]; then
    echo "✓ Verdict: ${EVIDENCE_BASE}/${STAGE_SHORT}_verdict.json"
    SIZE=$(wc -c < "${EVIDENCE_BASE}/${STAGE_SHORT}_verdict.json")
    echo "  Size: $SIZE bytes"

    # Parse verdict for key info
    if command -v jq &> /dev/null; then
        echo ""
        echo "  Verdict Summary:"
        jq -r '
            if .consensus_ok != null then
                "    Consensus: " + (.consensus_ok | tostring)
            else empty end,
            if .degraded != null then
                "    Degraded: " + (.degraded | tostring)
            else empty end,
            if .missing_agents != null and (.missing_agents | length) > 0 then
                "    Missing: " + (.missing_agents | join(", "))
            else empty end
        ' "${EVIDENCE_BASE}/${STAGE_SHORT}_verdict.json" 2>/dev/null || echo "    (parse failed)"
    fi
else
    echo "✗ Verdict not found"
fi
echo ""

echo "==================================="
echo "Debug Summary"
echo "==================================="
echo ""
echo "To view full agent outputs:"
echo "  sqlite3 $DB_PATH \"SELECT agent_id, content FROM consensus_artifacts WHERE spec_id='$SPEC_ID' AND stage='$STAGE';\""
echo ""
echo "To extract synthesis:"
echo "  sqlite3 $DB_PATH \"SELECT output FROM consensus_synthesis WHERE spec_id='$SPEC_ID' AND stage='$STAGE' ORDER BY created_at DESC LIMIT 1;\""
echo ""
