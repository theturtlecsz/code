#!/bin/bash
# scripts/validate-deliverable.sh
# Validate spec-kit deliverable quality

set -e

SPEC_ID="$1"
STAGE="$2"

if [ -z "$SPEC_ID" ] || [ -z "$STAGE" ]; then
    cat <<EOF
Usage: $0 <SPEC-ID> <stage>

Stages: plan, tasks, validate, implement, audit, unlock

Examples:
  $0 SPEC-KIT-900 plan
  $0 SPEC-KIT-900 tasks

Checks:
  - File exists and has content
  - No debug logs or JSON dumps
  - Contains expected keywords
  - Meets minimum length requirements
  - Structure validation
EOF
    exit 1
fi

SPEC_DIR="docs/${SPEC_ID}-generic-smoke"
FILE="${SPEC_DIR}/${STAGE}.md"

echo "==================================="
echo "Deliverable Validation"
echo "==================================="
echo "SPEC: $SPEC_ID"
echo "Stage: $STAGE"
echo "File: $FILE"
echo ""

# Track validation results
PASS=0
FAIL=0
WARN=0

function check_pass() {
    echo "✓ PASS: $1"
    PASS=$((PASS + 1))
}

function check_fail() {
    echo "✗ FAIL: $1"
    FAIL=$((FAIL + 1))
}

function check_warn() {
    echo "⚠ WARN: $1"
    WARN=$((WARN + 1))
}

# 1. File exists
echo "1. Existence Checks"
echo "-----------------------------------"
if [ -f "$FILE" ]; then
    check_pass "File exists: $FILE"
else
    check_fail "File not found: $FILE"
    exit 1
fi

SIZE=$(wc -c < "$FILE")
LINES=$(wc -l < "$FILE")
echo "  Size: $SIZE bytes, Lines: $LINES"
echo ""

# 2. Minimum size check
echo "2. Size Validation"
echo "-----------------------------------"
MIN_SIZE=2000  # Substantial content expected

if [ "$SIZE" -gt "$MIN_SIZE" ]; then
    check_pass "File size $SIZE > $MIN_SIZE bytes"
else
    check_fail "File too small: $SIZE < $MIN_SIZE bytes (likely incomplete)"
fi
echo ""

# 3. Anti-patterns (should NOT contain)
echo "3. Anti-Pattern Detection"
echo "-----------------------------------"

# Check for debug logs
if grep -q "Debug:" "$FILE"; then
    check_fail "Contains 'Debug:' (agent debug logs present)"
else
    check_pass "No debug logs found"
fi

# Check for raw JSON (common in broken outputs)
JSON_LINES=$(grep -c '"model":' "$FILE" 2>/dev/null || echo 0)
if [ "$JSON_LINES" -gt 5 ]; then
    check_fail "Contains excessive JSON ($JSON_LINES lines with '\"model\":')"
else
    check_pass "No raw JSON dumps"
fi

# Check for very long lines (JSON dumps)
LONG_LINES=$(awk 'length > 10000' "$FILE" | wc -l)
if [ "$LONG_LINES" -gt 0 ]; then
    check_warn "$LONG_LINES lines exceed 10,000 chars (possible JSON dumps)"
else
    check_pass "No extremely long lines"
fi
echo ""

# 4. Content validation (stage-specific)
echo "4. Stage-Specific Content Validation"
echo "-----------------------------------"

case "$STAGE" in
    plan)
        # Plan should contain work breakdown, risks, acceptance criteria
        if grep -qi "work.*breakdown\|milestone\|timeline" "$FILE"; then
            check_pass "Contains work breakdown/milestones"
        else
            check_warn "Missing work breakdown/milestone section"
        fi

        if grep -qi "risk\|mitigation" "$FILE"; then
            check_pass "Contains risk analysis"
        else
            check_warn "Missing risk analysis"
        fi

        if grep -qi "acceptance\|criteria\|validation" "$FILE"; then
            check_pass "Contains acceptance criteria"
        else
            check_warn "Missing acceptance criteria"
        fi
        ;;

    tasks)
        # Tasks should contain task list, dependencies
        TASK_COUNT=$(grep -c "^#\{1,3\} T[0-9]\|Task [0-9]\|\- \*\*T[0-9]" "$FILE" 2>/dev/null || echo 0)
        if [ "$TASK_COUNT" -ge 5 ]; then
            check_pass "Contains $TASK_COUNT tasks (expected 8-12)"
        else
            check_fail "Only $TASK_COUNT tasks found (expected 8-12)"
        fi

        if grep -qi "dependenc\|order\|parallel" "$FILE"; then
            check_pass "Contains dependency information"
        else
            check_warn "Missing dependency analysis"
        fi
        ;;

    validate)
        # Validate should contain test scenarios, coverage
        if grep -qi "test.*scenario\|validation.*plan" "$FILE"; then
            check_pass "Contains test scenarios"
        else
            check_warn "Missing test scenarios"
        fi

        if grep -qi "coverage\|acceptance.*criteria" "$FILE"; then
            check_pass "Contains coverage information"
        else
            check_warn "Missing coverage analysis"
        fi

        if grep -qi "rollback\|monitoring\|kpi" "$FILE"; then
            check_pass "Contains rollback/monitoring plan"
        else
            check_warn "Missing operational concerns"
        fi
        ;;

    implement)
        # Implementation should contain code guidance
        if grep -qi "implementation\|code\|file.*path" "$FILE"; then
            check_pass "Contains implementation guidance"
        else
            check_warn "Missing implementation details"
        fi
        ;;

    audit)
        # Audit should contain checks and recommendations
        if grep -qi "audit\|compliance\|security" "$FILE"; then
            check_pass "Contains audit content"
        else
            check_warn "Missing audit analysis"
        fi
        ;;

    unlock)
        # Unlock should contain decision
        if grep -qi "unlock\|decision\|recommendation" "$FILE"; then
            check_pass "Contains unlock decision"
        else
            check_warn "Missing unlock decision"
        fi
        ;;
esac
echo ""

# 5. SPEC-KIT-900 specific checks (reminder microservice)
if [ "$SPEC_ID" = "SPEC-KIT-900" ]; then
    echo "5. SPEC-KIT-900 Workload Validation"
    echo "-----------------------------------"

    # Should contain reminder microservice keywords
    if grep -qi "reminder" "$FILE"; then
        check_pass "References 'reminder' (workload keyword)"
    else
        check_fail "Missing 'reminder' - NOT about the workload!"
    fi

    if grep -qi "microservice\|service\|api" "$FILE"; then
        check_pass "References microservice/API concepts"
    else
        check_warn "Missing microservice/API references"
    fi

    if grep -qi "sync\|device\|cross-device" "$FILE"; then
        check_pass "References sync functionality"
    else
        check_warn "Missing sync functionality references"
    fi

    # Should NOT be meta-analysis of SPEC-KIT-900 document
    if grep -qi "SPEC-KIT-900.*infrastructure\|multi-agent.*consensus.*SPEC-KIT-900" "$FILE"; then
        check_fail "Contains meta-analysis of SPEC-KIT-900 (PROMPT FIX NEEDED!)"
    else
        check_pass "Not meta-analyzing SPEC-KIT-900 document"
    fi
    echo ""
fi

# 6. Summary
echo "==================================="
echo "Validation Summary"
echo "==================================="
echo "✓ PASS: $PASS"
echo "⚠ WARN: $WARN"
echo "✗ FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "Result: ACCEPTABLE (${WARN} warnings)"
    exit 0
elif [ "$FAIL" -le 2 ]; then
    echo "Result: MARGINAL ($FAIL failures, needs review)"
    exit 1
else
    echo "Result: UNACCEPTABLE ($FAIL failures)"
    exit 2
fi
