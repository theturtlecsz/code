#!/bin/bash
# scripts/workflow-status.sh
# Comprehensive workflow status dashboard

set -e

SPEC_ID="${1:-}"
DB_PATH="${2:-$HOME/.code/consensus_artifacts.db}"

if [ -z "$SPEC_ID" ]; then
    cat <<EOF
Usage: $0 <SPEC-ID> [db-path]

Examples:
  $0 SPEC-KIT-900
  $0 SPEC-KIT-900 ~/.code/consensus_artifacts.db

Shows comprehensive status:
  - Stage completion (which stages done)
  - Agent participation per stage
  - Deliverable status
  - Evidence completeness
  - Cost tracking
  - Quality assessment
  - Next steps
EOF
    exit 1
fi

if ! command -v sqlite3 &> /dev/null; then
    echo "Error: sqlite3 not found"
    exit 1
fi

echo "╔═════════════════════════════════════════════════════╗"
echo "║         Spec-Kit Workflow Status Dashboard         ║"
echo "╚═════════════════════════════════════════════════════╝"
echo ""
echo "SPEC: $SPEC_ID"
echo "Time: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# 1. Stage completion overview
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 STAGE COMPLETION"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

STAGES=("plan" "tasks" "implement" "validate" "audit" "unlock")
COMPLETED=0

for stage in "${STAGES[@]}"; do
    STAGE_DB="spec-$stage"

    # Check database for artifacts
    HAS_ARTIFACTS=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB';" 2>/dev/null || echo "0")

    # Check for deliverable
    DELIVERABLE="docs/${SPEC_ID}-generic-smoke/${stage}.md"
    HAS_DELIVERABLE=$([ -f "$DELIVERABLE" ] && echo "yes" || echo "no")

    # Check for synthesis
    HAS_SYNTHESIS=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_synthesis WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB';" 2>/dev/null || echo "0")

    # Determine status
    if [ "$HAS_ARTIFACTS" -gt 0 ] && [ "$HAS_SYNTHESIS" -gt 0 ] && [ "$HAS_DELIVERABLE" = "yes" ]; then
        STATUS="✓ COMPLETE"
        COMPLETED=$((COMPLETED + 1))
    elif [ "$HAS_ARTIFACTS" -gt 0 ]; then
        STATUS="⚙ IN PROGRESS"
    else
        STATUS="○ PENDING"
    fi

    # Show stage with status
    printf "%-12s %s" "$stage" "$STATUS"

    # Show agent count if available
    if [ "$HAS_ARTIFACTS" -gt 0 ]; then
        AGENT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(DISTINCT agent_id) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB';" 2>/dev/null || echo "0")
        printf " (%d agents)" "$AGENT_COUNT"
    fi

    echo ""
done

echo ""
echo "Progress: $COMPLETED / 6 stages complete"
echo ""

# 2. Agent participation matrix
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🤖 AGENT PARTICIPATION"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

printf "%-12s " "Stage"
AGENTS=("gemini" "claude" "gpt_pro" "gpt_codex" "code")
for agent in "${AGENTS[@]}"; do
    printf "%-10s " "$agent"
done
echo ""
echo "$(printf '─%.0s' {1..65})"

for stage in "${STAGES[@]}"; do
    STAGE_DB="spec-$stage"
    printf "%-12s " "$stage"

    for agent in "${AGENTS[@]}"; do
        HAS_OUTPUT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND agent_id = '$agent';" 2>/dev/null || echo "0")

        if [ "$HAS_OUTPUT" -gt 0 ]; then
            printf "%-10s " "✓"
        else
            printf "%-10s " "·"
        fi
    done
    echo ""
done
echo ""

# 3. Deliverable status
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📄 DELIVERABLES"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

for stage in "${STAGES[@]}"; do
    DELIVERABLE="docs/${SPEC_ID}-generic-smoke/${stage}.md"

    if [ -f "$DELIVERABLE" ]; then
        SIZE=$(wc -c < "$DELIVERABLE")
        SIZE_KB=$(echo "scale=1; $SIZE / 1024" | bc 2>/dev/null || echo "?")

        if [ "$SIZE" -gt 2000 ]; then
            printf "%-12s ✓ %6s KB\n" "$stage.md" "$SIZE_KB"
        else
            printf "%-12s ⚠ %6s KB (small)\n" "$stage.md" "$SIZE_KB"
        fi
    else
        printf "%-12s ✗ missing\n" "$stage.md"
    fi
done
echo ""

# 4. Evidence completeness
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🗂️  EVIDENCE FILES"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

EVIDENCE_BASE="docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/$SPEC_ID"
EVIDENCE_COUNT=0
EXPECTED_COUNT=12  # 2 per stage * 6 stages

for stage in "${STAGES[@]}"; do
    SYNTHESIS="${EVIDENCE_BASE}/${stage}_synthesis.json"
    VERDICT="${EVIDENCE_BASE}/${stage}_verdict.json"

    SYNTHESIS_STATUS="✗"
    VERDICT_STATUS="✗"

    if [ -f "$SYNTHESIS" ]; then
        SYNTHESIS_STATUS="✓"
        EVIDENCE_COUNT=$((EVIDENCE_COUNT + 1))
    fi

    if [ -f "$VERDICT" ]; then
        VERDICT_STATUS="✓"
        EVIDENCE_COUNT=$((EVIDENCE_COUNT + 1))
    fi

    printf "%-12s synthesis:%s  verdict:%s\n" "$stage" "$SYNTHESIS_STATUS" "$VERDICT_STATUS"
done

echo ""
echo "Completeness: $EVIDENCE_COUNT / $EXPECTED_COUNT files"
echo ""

# 5. Cost tracking
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "💰 COST TRACKING"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

COST_FILE="docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/${SPEC_ID}_cost_summary.json"

if [ -f "$COST_FILE" ]; then
    if command -v jq &> /dev/null; then
        TOTAL_COST=$(jq -r '.total_cost // "N/A"' "$COST_FILE" 2>/dev/null || echo "N/A")
        echo "Total Cost: \$$TOTAL_COST"
        echo ""
        echo "Per-Stage:"
        jq -r '.per_stage | to_entries[] | "  \(.key): $\(.value)"' "$COST_FILE" 2>/dev/null || echo "  (parsing failed)"
    else
        echo "Cost file exists but jq not available for parsing"
        echo "Install with: apt-get install jq"
    fi
else
    echo "Cost summary not yet available"
fi
echo ""

# 6. Quality assessment
if [ -f "scripts/validate-deliverable.sh" ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "✨ QUALITY ASSESSMENT"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    for stage in "${STAGES[@]}"; do
        DELIVERABLE="docs/${SPEC_ID}-generic-smoke/${stage}.md"

        if [ -f "$DELIVERABLE" ]; then
            printf "%-12s " "$stage"

            # Run validation and capture result
            RESULT=$(bash scripts/validate-deliverable.sh "$SPEC_ID" "$stage" 2>&1 | tail -1 || echo "ERROR")

            if echo "$RESULT" | grep -q "ACCEPTABLE"; then
                echo "✓ PASS"
            elif echo "$RESULT" | grep -q "MARGINAL"; then
                echo "⚠ MARGINAL"
            elif echo "$RESULT" | grep -q "UNACCEPTABLE"; then
                echo "✗ FAIL"
            else
                echo "? UNKNOWN"
            fi
        fi
    done
    echo ""
fi

# 7. Database statistics
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 DATABASE STATISTICS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -f "$DB_PATH" ]; then
    ARTIFACT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "0")
    SYNTHESIS_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_synthesis WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "0")
    RUN_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(DISTINCT run_id) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "0")

    echo "Consensus artifacts: $ARTIFACT_COUNT"
    echo "Synthesis records: $SYNTHESIS_COUNT"
    echo "Distinct runs: $RUN_COUNT"

    if [ "$RUN_COUNT" -gt 0 ]; then
        echo ""
        echo "Run IDs:"
        sqlite3 "$DB_PATH" "SELECT DISTINCT run_id FROM consensus_artifacts WHERE spec_id = '$SPEC_ID';" 2>/dev/null | sed 's/^/  /' || echo "  (query failed)"
    fi
else
    echo "Database not found at $DB_PATH"
fi
echo ""

# 8. Next steps
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "➡️  NEXT STEPS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Determine next action based on progress
if [ "$COMPLETED" -eq 0 ]; then
    echo "▸ Start workflow: bash scripts/test-spec-kit.sh $SPEC_ID /speckit.plan"
elif [ "$COMPLETED" -eq 1 ]; then
    echo "▸ Continue workflow: bash scripts/test-spec-kit.sh $SPEC_ID /speckit.tasks"
elif [ "$COMPLETED" -eq 2 ]; then
    echo "▸ Continue workflow: bash scripts/test-spec-kit.sh $SPEC_ID /speckit.validate"
elif [ "$COMPLETED" -lt 6 ]; then
    echo "▸ Continue workflow: bash scripts/test-spec-kit.sh $SPEC_ID /speckit.auto"
else
    echo "▸ Workflow complete! Review deliverables:"
    echo "  ls -lh docs/${SPEC_ID}-generic-smoke/"
fi

echo ""
echo "▸ Debug consensus: bash scripts/debug-consensus.sh $SPEC_ID <stage>"
echo "▸ Monitor cost: bash scripts/monitor-cost.sh $SPEC_ID"
echo "▸ Audit evidence: bash scripts/audit-evidence.sh $SPEC_ID"
echo ""

echo "╔═════════════════════════════════════════════════════╗"
echo "║                 Dashboard Complete                  ║"
echo "╚═════════════════════════════════════════════════════╝"
