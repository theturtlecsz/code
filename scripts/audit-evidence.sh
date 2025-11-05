#!/bin/bash
# scripts/audit-evidence.sh
# Audit evidence completeness and integrity

set -e

SPEC_ID="${1:-}"
RUN_ID="${2:-}"
DB_PATH="${3:-$HOME/.code/consensus_artifacts.db}"

if [ -z "$SPEC_ID" ]; then
    cat <<EOF
Usage: $0 <SPEC-ID> [run-id] [db-path]

Examples:
  $0 SPEC-KIT-900
  $0 SPEC-KIT-900 abc123 ~/.code/consensus_artifacts.db

Checks:
  - All expected consensus files present (12 files for 6-stage run)
  - Database integrity (run_id propagation, no orphans)
  - Evidence schema compliance
  - File sizes reasonable
  - No missing agent outputs
EOF
    exit 1
fi

if ! command -v sqlite3 &> /dev/null; then
    echo "Error: sqlite3 not found"
    exit 1
fi

PASS=0
FAIL=0
WARN=0

function check_pass() {
    echo "✓ $1"
    PASS=$((PASS + 1))
}

function check_fail() {
    echo "✗ $1"
    FAIL=$((FAIL + 1))
}

function check_warn() {
    echo "⚠ $1"
    WARN=$((WARN + 1))
}

echo "==================================="
echo "Evidence Audit: $SPEC_ID"
echo "==================================="
echo ""

# 1. Consensus file audit
echo "1. Consensus Files (Evidence Repository)"
echo "-----------------------------------"
EVIDENCE_BASE="docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/$SPEC_ID"

EXPECTED_STAGES=("plan" "tasks" "implement" "validate" "audit" "unlock")
TOTAL_EXPECTED=12  # 2 files per stage * 6 stages

FOUND_COUNT=0

for stage in "${EXPECTED_STAGES[@]}"; do
    SYNTHESIS="${EVIDENCE_BASE}/${stage}_synthesis.json"
    VERDICT="${EVIDENCE_BASE}/${stage}_verdict.json"

    if [ -f "$SYNTHESIS" ]; then
        SIZE=$(wc -c < "$SYNTHESIS")
        if [ "$SIZE" -gt 100 ]; then
            check_pass "${stage}_synthesis.json ($SIZE bytes)"
            FOUND_COUNT=$((FOUND_COUNT + 1))
        else
            check_warn "${stage}_synthesis.json exists but suspiciously small ($SIZE bytes)"
            FOUND_COUNT=$((FOUND_COUNT + 1))
        fi
    else
        check_fail "${stage}_synthesis.json MISSING"
    fi

    if [ -f "$VERDICT" ]; then
        SIZE=$(wc -c < "$VERDICT")
        if [ "$SIZE" -gt 100 ]; then
            check_pass "${stage}_verdict.json ($SIZE bytes)"
            FOUND_COUNT=$((FOUND_COUNT + 1))
        else
            check_warn "${stage}_verdict.json exists but suspiciously small ($SIZE bytes)"
            FOUND_COUNT=$((FOUND_COUNT + 1))
        fi
    else
        check_fail "${stage}_verdict.json MISSING"
    fi
done

echo ""
echo "File completeness: $FOUND_COUNT / $TOTAL_EXPECTED"

if [ "$FOUND_COUNT" -eq "$TOTAL_EXPECTED" ]; then
    check_pass "All consensus files present"
elif [ "$FOUND_COUNT" -ge 8 ]; then
    check_warn "Partial run: $FOUND_COUNT / $TOTAL_EXPECTED files"
else
    check_fail "Incomplete evidence: $FOUND_COUNT / $TOTAL_EXPECTED files"
fi
echo ""

# 2. Database integrity
echo "2. Database Integrity"
echo "-----------------------------------"

if [ ! -f "$DB_PATH" ]; then
    check_fail "Database not found at $DB_PATH"
else
    check_pass "Database exists"

    # Check for artifacts
    ARTIFACT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "0")
    if [ "$ARTIFACT_COUNT" -gt 0 ]; then
        check_pass "Found $ARTIFACT_COUNT agent artifacts in database"
    else
        check_fail "No artifacts found in database"
    fi

    # Check for synthesis records
    SYNTHESIS_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_synthesis WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "0")
    if [ "$SYNTHESIS_COUNT" -gt 0 ]; then
        check_pass "Found $SYNTHESIS_COUNT synthesis records in database"
    else
        check_fail "No synthesis records found in database"
    fi

    # Check for orphaned artifacts (artifacts without synthesis)
    ORPHANS=$(sqlite3 "$DB_PATH" "
        SELECT COUNT(DISTINCT a.stage)
        FROM consensus_artifacts a
        LEFT JOIN consensus_synthesis s ON a.spec_id = s.spec_id AND a.stage = s.stage
        WHERE a.spec_id = '$SPEC_ID' AND s.synthesis_id IS NULL;
    " 2>/dev/null || echo "0")

    if [ "$ORPHANS" -eq 0 ]; then
        check_pass "No orphaned artifacts (all have synthesis)"
    else
        check_warn "$ORPHANS stages have artifacts but no synthesis"
    fi
fi
echo ""

# 3. Agent participation audit
echo "3. Agent Participation"
echo "-----------------------------------"

for stage in plan tasks implement validate audit unlock; do
    STAGE_DB="spec-$stage"
    AGENT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(DISTINCT agent_id) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB';" 2>/dev/null || echo "0")

    EXPECTED_AGENTS=3
    if [ "$stage" = "implement" ]; then
        EXPECTED_AGENTS=4  # gemini, claude, gpt_codex, gpt_pro
    fi

    if [ "$AGENT_COUNT" -eq "$EXPECTED_AGENTS" ]; then
        check_pass "$stage: $AGENT_COUNT/$EXPECTED_AGENTS agents"
    elif [ "$AGENT_COUNT" -gt 0 ]; then
        check_warn "$stage: $AGENT_COUNT/$EXPECTED_AGENTS agents (degraded mode)"
    else
        # Stage may not have run yet, don't fail
        echo "  $stage: 0/$EXPECTED_AGENTS agents (not started)"
    fi
done
echo ""

# 4. run_id propagation
echo "4. Run ID Propagation"
echo "-----------------------------------"

if [ -n "$RUN_ID" ]; then
    # Check specific run_id
    ARTIFACTS_WITH_RUN=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND run_id = '$RUN_ID';" 2>/dev/null || echo "0")
    SYNTHESIS_WITH_RUN=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_synthesis WHERE spec_id = '$SPEC_ID' AND run_id = '$RUN_ID';" 2>/dev/null || echo "0")

    if [ "$ARTIFACTS_WITH_RUN" -gt 0 ]; then
        check_pass "Run $RUN_ID: $ARTIFACTS_WITH_RUN artifacts"
    else
        check_fail "Run $RUN_ID: No artifacts found"
    fi

    if [ "$SYNTHESIS_WITH_RUN" -gt 0 ]; then
        check_pass "Run $RUN_ID: $SYNTHESIS_WITH_RUN synthesis records"
    else
        check_fail "Run $RUN_ID: No synthesis records"
    fi
else
    # Check all runs
    RUN_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(DISTINCT run_id) FROM consensus_artifacts WHERE spec_id = '$SPEC_ID';" 2>/dev/null || echo "0")
    if [ "$RUN_COUNT" -gt 0 ]; then
        check_pass "Found $RUN_COUNT distinct run(s)"
    else
        check_fail "No runs found"
    fi
fi
echo ""

# 5. Evidence footprint policy (25 MB soft limit)
echo "5. Evidence Footprint Policy"
echo "-----------------------------------"
SOFT_LIMIT_MB=25

if [ -d "$EVIDENCE_BASE" ]; then
    TOTAL_KB=$(du -sk "$EVIDENCE_BASE" 2>/dev/null | cut -f1 || echo "0")
    TOTAL_MB=$(echo "scale=2; $TOTAL_KB / 1024" | bc 2>/dev/null || echo "0")

    echo "Evidence size: ${TOTAL_MB} MB"

    if command -v bc &> /dev/null; then
        if (( $(echo "$TOTAL_MB < 15" | bc -l) )); then
            check_pass "Well under limit (<15 MB)"
        elif (( $(echo "$TOTAL_MB < $SOFT_LIMIT_MB" | bc -l) )); then
            check_warn "Approaching limit (${TOTAL_MB}/${SOFT_LIMIT_MB} MB)"
        else
            check_fail "Exceeds soft limit (${TOTAL_MB} > ${SOFT_LIMIT_MB} MB)"
        fi
    fi
else
    check_warn "Evidence directory not found (run may be incomplete)"
fi
echo ""

# 6. Schema validation (sampling)
echo "6. Schema Validation (Sampling)"
echo "-----------------------------------"

if command -v jq &> /dev/null; then
    # Check one verdict file for expected fields
    SAMPLE_VERDICT="${EVIDENCE_BASE}/plan_verdict.json"
    if [ -f "$SAMPLE_VERDICT" ]; then
        HAS_CONSENSUS=$(jq -e '.consensus_ok' "$SAMPLE_VERDICT" >/dev/null 2>&1 && echo "yes" || echo "no")
        HAS_AGENTS=$(jq -e '.agents' "$SAMPLE_VERDICT" >/dev/null 2>&1 && echo "yes" || echo "no")

        if [ "$HAS_CONSENSUS" = "yes" ]; then
            check_pass "Verdict schema: has 'consensus_ok' field"
        else
            check_warn "Verdict schema: missing 'consensus_ok' field"
        fi

        if [ "$HAS_AGENTS" = "yes" ]; then
            check_pass "Verdict schema: has 'agents' field"
        else
            check_warn "Verdict schema: missing 'agents' field"
        fi
    else
        echo "  (No verdict file to sample)"
    fi
else
    echo "  (Install jq for JSON schema validation)"
fi
echo ""

# 7. Deliverable files
echo "7. Deliverable Files"
echo "-----------------------------------"
SPEC_DIR="docs/${SPEC_ID}-generic-smoke"

for file in plan.md tasks.md implement.md validate.md audit.md unlock.md; do
    FULL_PATH="$SPEC_DIR/$file"
    if [ -f "$FULL_PATH" ]; then
        SIZE=$(wc -c < "$FULL_PATH")
        if [ "$SIZE" -gt 1000 ]; then
            check_pass "$file ($SIZE bytes)"
        else
            check_warn "$file exists but very small ($SIZE bytes)"
        fi
    else
        echo "  $file: not present (stage may not have run)"
    fi
done
echo ""

# Summary
echo "==================================="
echo "Audit Summary"
echo "==================================="
echo "✓ PASS: $PASS"
echo "⚠ WARN: $WARN"
echo "✗ FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ] && [ "$WARN" -eq 0 ]; then
    echo "Result: EXCELLENT - Full compliance"
    exit 0
elif [ "$FAIL" -eq 0 ]; then
    echo "Result: GOOD - Minor issues ($WARN warnings)"
    exit 0
elif [ "$FAIL" -le 3 ]; then
    echo "Result: MARGINAL - Needs attention ($FAIL failures)"
    exit 1
else
    echo "Result: POOR - Significant issues ($FAIL failures)"
    exit 2
fi
