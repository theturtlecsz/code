#!/bin/bash
# scripts/compare-runs.sh
# Compare two runs of the same SPEC (before/after prompt fix)

set -e

SPEC_ID="$1"
RUN1_ID="$2"
RUN2_ID="$3"
STAGE="${4:-plan}"
DB_PATH="${5:-$HOME/.code/consensus_artifacts.db}"

if [ -z "$SPEC_ID" ] || [ -z "$RUN1_ID" ] || [ -z "$RUN2_ID" ]; then
    cat <<EOF
Usage: $0 <SPEC-ID> <run1-id> <run2-id> [stage] [db-path]

Examples:
  $0 SPEC-KIT-900 before-run after-run plan
  $0 SPEC-KIT-900 abc123 def456 tasks

Compares:
  - Agent participation changes
  - Content length differences
  - Keyword presence (workload vs meta-analysis)
  - Cost differences
  - Quality improvements

Use to validate prompt fixes or model routing changes.
EOF
    exit 1
fi

if ! command -v sqlite3 &> /dev/null; then
    echo "Error: sqlite3 not found"
    exit 1
fi

STAGE_DB="spec-$STAGE"

echo "==================================="
echo "Run Comparison"
echo "==================================="
echo "SPEC: $SPEC_ID"
echo "Stage: $STAGE"
echo "Run 1: $RUN1_ID"
echo "Run 2: $RUN2_ID"
echo ""

# 1. Agent participation
echo "1. Agent Participation"
echo "-----------------------------------"
echo "Run 1 agents:"
sqlite3 "$DB_PATH" "SELECT agent_id FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND run_id = '$RUN1_ID';" 2>/dev/null | sed 's/^/  /' || echo "  (none)"

echo ""
echo "Run 2 agents:"
sqlite3 "$DB_PATH" "SELECT agent_id FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND run_id = '$RUN2_ID';" 2>/dev/null | sed 's/^/  /' || echo "  (none)"
echo ""

# 2. Content length comparison
echo "2. Content Length Comparison"
echo "-----------------------------------"
echo "Run 1:"
sqlite3 "$DB_PATH" <<SQL
.mode column
.headers on
SELECT agent_id, length(content) as content_length
FROM consensus_artifacts
WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND run_id = '$RUN1_ID'
ORDER BY agent_id;
SQL

echo ""
echo "Run 2:"
sqlite3 "$DB_PATH" <<SQL
.mode column
.headers on
SELECT agent_id, length(content) as content_length
FROM consensus_artifacts
WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND run_id = '$RUN2_ID'
ORDER BY agent_id;
SQL
echo ""

# 3. Keyword analysis (for SPEC-KIT-900 prompt fix validation)
if [ "$SPEC_ID" = "SPEC-KIT-900" ]; then
    echo "3. Keyword Analysis (Prompt Fix Validation)"
    echo "-----------------------------------"

    for run_id in "$RUN1_ID" "$RUN2_ID"; do
        echo ""
        echo "Run: $run_id"

        # Extract synthesis output
        SYNTHESIS=$(sqlite3 "$DB_PATH" "SELECT output FROM consensus_synthesis WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND run_id = '$run_id' LIMIT 1;" 2>/dev/null || echo "")

        if [ -n "$SYNTHESIS" ]; then
            # Count workload keywords
            REMINDER_COUNT=$(echo "$SYNTHESIS" | grep -io "reminder" | wc -l || echo 0)
            MICROSERVICE_COUNT=$(echo "$SYNTHESIS" | grep -io "microservice\|service" | wc -l || echo 0)
            SYNC_COUNT=$(echo "$SYNTHESIS" | grep -io "sync" | wc -l || echo 0)

            # Count meta-analysis keywords (bad)
            DEBUG_COUNT=$(echo "$SYNTHESIS" | grep -io "Debug:" | wc -l || echo 0)
            JSON_MODEL_COUNT=$(echo "$SYNTHESIS" | grep -io '"model":' | wc -l || echo 0)

            echo "  Workload keywords:"
            echo "    'reminder': $REMINDER_COUNT"
            echo "    'microservice/service': $MICROSERVICE_COUNT"
            echo "    'sync': $SYNC_COUNT"
            echo ""
            echo "  Anti-patterns:"
            echo "    'Debug:': $DEBUG_COUNT"
            echo "    JSON dumps: $JSON_MODEL_COUNT"

            # Verdict
            if [ "$REMINDER_COUNT" -gt 0 ] && [ "$DEBUG_COUNT" -eq 0 ]; then
                echo "  ✓ GOOD: Contains workload, no debug logs"
            elif [ "$REMINDER_COUNT" -eq 0 ]; then
                echo "  ✗ BAD: No workload keywords (meta-analysis?)"
            elif [ "$DEBUG_COUNT" -gt 0 ]; then
                echo "  ⚠ MIXED: Workload present but has debug logs"
            fi
        else
            echo "  (No synthesis output found)"
        fi
    done
    echo ""
fi

# 4. Synthesis file comparison
echo "4. Synthesis Output Comparison"
echo "-----------------------------------"

SYNTHESIS1_PATH=$(sqlite3 "$DB_PATH" "SELECT output_path FROM consensus_synthesis WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND run_id = '$RUN1_ID' LIMIT 1;" 2>/dev/null || echo "")
SYNTHESIS2_PATH=$(sqlite3 "$DB_PATH" "SELECT output_path FROM consensus_synthesis WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND run_id = '$RUN2_ID' LIMIT 1;" 2>/dev/null || echo "")

if [ -n "$SYNTHESIS1_PATH" ] && [ -f "$SYNTHESIS1_PATH" ]; then
    SIZE1=$(wc -c < "$SYNTHESIS1_PATH")
    echo "Run 1 output: $SYNTHESIS1_PATH ($SIZE1 bytes)"
else
    echo "Run 1 output: NOT FOUND"
fi

if [ -n "$SYNTHESIS2_PATH" ] && [ -f "$SYNTHESIS2_PATH" ]; then
    SIZE2=$(wc -c < "$SYNTHESIS2_PATH")
    echo "Run 2 output: $SYNTHESIS2_PATH ($SIZE2 bytes)"
else
    echo "Run 2 output: NOT FOUND"
fi

# Show diff if both exist
if [ -n "$SYNTHESIS1_PATH" ] && [ -f "$SYNTHESIS1_PATH" ] && [ -n "$SYNTHESIS2_PATH" ] && [ -f "$SYNTHESIS2_PATH" ]; then
    echo ""
    echo "Content Diff (first 50 lines of each):"
    echo "======================================="

    if command -v diff &> /dev/null; then
        diff -u <(head -50 "$SYNTHESIS1_PATH") <(head -50 "$SYNTHESIS2_PATH") || true
    else
        echo "(diff command not available)"
    fi
fi
echo ""

# 5. Quality comparison (if validation script exists)
if [ -f "scripts/validate-deliverable.sh" ]; then
    echo "5. Quality Validation"
    echo "-----------------------------------"

    # Note: This requires deliverable files to exist, which may not be the case for old runs
    SPEC_DIR="docs/${SPEC_ID}-generic-smoke"
    DELIVERABLE="$SPEC_DIR/${STAGE}.md"

    if [ -f "$DELIVERABLE" ]; then
        echo "Current deliverable (Run 2 assumed):"
        bash scripts/validate-deliverable.sh "$SPEC_ID" "$STAGE" 2>&1 | grep -E "^✓|^✗|^⚠|^Result:" || echo "  (validation failed)"
    else
        echo "  No deliverable file to validate"
    fi
    echo ""
fi

# Summary
echo "==================================="
echo "Comparison Summary"
echo "==================================="
echo ""
echo "To view full synthesis outputs:"
echo "  Run 1: cat '$SYNTHESIS1_PATH' 2>/dev/null | less"
echo "  Run 2: cat '$SYNTHESIS2_PATH' 2>/dev/null | less"
echo ""
echo "To extract agent outputs:"
echo "  sqlite3 $DB_PATH \"SELECT agent_id, content FROM consensus_artifacts WHERE spec_id='$SPEC_ID' AND stage='$STAGE_DB' AND run_id='$RUN1_ID';\""
echo ""
