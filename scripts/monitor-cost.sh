#!/bin/bash
# scripts/monitor-cost.sh
# Monitor cost and performance for spec-kit runs

set -e

SPEC_ID="${1:-}"
DB_PATH="${2:-$HOME/.code/consensus_artifacts.db}"

if [ -z "$SPEC_ID" ]; then
    cat <<EOF
Usage: $0 <SPEC-ID> [db-path]

Examples:
  $0 SPEC-KIT-900
  $0 SPEC-KIT-900 ~/.code/consensus_artifacts.db

Shows:
  - Total cost across all stages
  - Per-stage cost breakdown
  - Per-agent cost
  - Performance metrics (latency, token usage)
  - Cost efficiency analysis
EOF
    exit 1
fi

if ! command -v sqlite3 &> /dev/null; then
    echo "Error: sqlite3 not found"
    exit 1
fi

if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database not found at $DB_PATH"
    exit 1
fi

echo "==================================="
echo "Cost & Performance Monitor"
echo "==================================="
echo "SPEC: $SPEC_ID"
echo ""

# 1. Check for cost summary file
echo "1. Cost Summary File"
echo "-----------------------------------"
COST_FILE="docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/${SPEC_ID}_cost_summary.json"

if [ -f "$COST_FILE" ]; then
    echo "✓ Found: $COST_FILE"

    if command -v jq &> /dev/null; then
        echo ""
        TOTAL=$(jq -r '.total_cost // "N/A"' "$COST_FILE" 2>/dev/null || echo "N/A")
        echo "  Total Cost: \$$TOTAL"
        echo ""
        echo "  Per-Stage Breakdown:"
        jq -r '.per_stage | to_entries[] | "    \(.key): $\(.value)"' "$COST_FILE" 2>/dev/null || echo "    (parsing failed)"
    else
        echo "  (Install jq for detailed parsing)"
    fi
else
    echo "✗ Cost file not found (run may be incomplete)"
fi
echo ""

# 2. Agent participation and token usage
echo "2. Agent Participation"
echo "-----------------------------------"
sqlite3 "$DB_PATH" <<SQL
.mode column
.headers on
SELECT
    stage,
    agent_id,
    length(content) as content_length,
    datetime(created_at, 'unixepoch') as timestamp
FROM consensus_artifacts
WHERE spec_id = '$SPEC_ID'
ORDER BY stage, created_at;
SQL
echo ""

# 3. Stage timing analysis
echo "3. Stage Performance"
echo "-----------------------------------"
echo "Stage completion times:"
sqlite3 "$DB_PATH" <<SQL
.mode column
.headers on
SELECT
    stage,
    COUNT(DISTINCT agent_id) as agent_count,
    MIN(datetime(created_at, 'unixepoch')) as first_response,
    MAX(datetime(created_at, 'unixepoch')) as last_response,
    CAST((MAX(created_at) - MIN(created_at)) AS INTEGER) as duration_seconds
FROM consensus_artifacts
WHERE spec_id = '$SPEC_ID'
GROUP BY stage
ORDER BY MIN(created_at);
SQL
echo ""

# 4. Evidence footprint
echo "4. Evidence Footprint"
echo "-----------------------------------"
EVIDENCE_DIR="docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/$SPEC_ID"

if [ -d "$EVIDENCE_DIR" ]; then
    TOTAL_SIZE=$(du -sh "$EVIDENCE_DIR" 2>/dev/null | cut -f1 || echo "0")
    FILE_COUNT=$(find "$EVIDENCE_DIR" -type f 2>/dev/null | wc -l || echo "0")

    echo "Evidence directory: $EVIDENCE_DIR"
    echo "Total size: $TOTAL_SIZE"
    echo "File count: $FILE_COUNT"
    echo ""
    echo "Files by stage:"
    find "$EVIDENCE_DIR" -type f -exec ls -lh {} \; 2>/dev/null | awk '{print "  " $9 " (" $5 ")"}'
else
    echo "✗ Evidence directory not found"
fi
echo ""

# 5. Database record count
echo "5. Database Statistics"
echo "-----------------------------------"
ARTIFACT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "0")
SYNTHESIS_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_synthesis WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "0")

echo "Consensus artifacts: $ARTIFACT_COUNT"
echo "Synthesis records: $SYNTHESIS_COUNT"
echo ""

# 6. Cost efficiency analysis
echo "6. Cost Efficiency Analysis"
echo "-----------------------------------"

if [ -f "$COST_FILE" ] && command -v jq &> /dev/null; then
    TOTAL_COST=$(jq -r '.total_cost // 0' "$COST_FILE" 2>/dev/null || echo "0")

    # Compare to expected costs (from SPEC-KIT-070)
    echo "Cost comparison to targets:"
    echo "  Full pipeline target: \$2.70 (SPEC-KIT-070)"
    echo "  Your cost: \$$TOTAL_COST"

    # Use bc for float comparison if available
    if command -v bc &> /dev/null; then
        if (( $(echo "$TOTAL_COST > 2.70" | bc -l) )); then
            OVERAGE=$(echo "$TOTAL_COST - 2.70" | bc -l)
            echo "  ⚠ OVER BUDGET by \$$OVERAGE"
        else
            SAVINGS=$(echo "2.70 - $TOTAL_COST" | bc -l)
            echo "  ✓ Under budget by \$$SAVINGS"
        fi
    fi

    # Per-stage analysis
    echo ""
    echo "Stage cost analysis:"
    jq -r '.per_stage | to_entries[] |
        "\(.key): $\(.value) " +
        if .key == "plan" then "(target: ~$0.35)"
        elif .key == "tasks" then "(target: ~$0.35)"
        elif .key == "implement" then "(target: ~$0.11)"
        elif .key == "validate" then "(target: ~$0.35)"
        elif .key == "audit" then "(target: ~$0.80)"
        elif .key == "unlock" then "(target: ~$0.80)"
        else ""
        end
    ' "$COST_FILE" 2>/dev/null | sed 's/^/  /' || echo "  (parsing failed)"
fi
echo ""

# 7. Run tracking
echo "7. Run Tracking"
echo "-----------------------------------"
RUN_IDS=$(sqlite3 "$DB_PATH" "SELECT DISTINCT run_id FROM consensus_artifacts WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "")

if [ -n "$RUN_IDS" ]; then
    RUN_COUNT=$(echo "$RUN_IDS" | wc -l)
    echo "Total runs: $RUN_COUNT"
    echo ""
    echo "Run IDs:"
    echo "$RUN_IDS" | sed 's/^/  /'
else
    echo "No runs found"
fi
echo ""

echo "==================================="
echo "Cost Monitor Complete"
echo "==================================="
