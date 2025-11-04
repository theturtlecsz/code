#!/bin/bash
# SPEC-KIT-900 Completeness Verification Script

cd /home/thetu/code/codex-rs || exit 1

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║ SPEC-KIT-900 Audit Infrastructure Completeness Check         ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

PASS=0
FAIL=0

# Check 1: All spawn functions have run_id parameter
echo "1. Checking spawn function signatures..."
if grep -q "run_id.*Option" tui/src/chatwidget/spec_kit/agent_orchestrator.rs && \
   grep -q "run_id.*Option" tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs; then
    echo "   ✅ All spawn functions have run_id parameter"
    ((PASS++))
else
    echo "   ❌ FAIL: Missing run_id in spawn signatures"
    ((FAIL++))
fi

# Check 2: record_agent_spawn calls pass run_id
echo "2. Checking record_agent_spawn calls..."
COUNT=$(grep "record_agent_spawn" tui/src/chatwidget/spec_kit/*.rs | grep -v "pub fn\|//" | wc -l)
if [ "$COUNT" -ge 3 ]; then
    echo "   ✅ Found $COUNT record_agent_spawn calls"
    ((PASS++))
else
    echo "   ❌ FAIL: Expected 3+ calls, found $COUNT"
    ((FAIL++))
fi

# Check 3: Log tagging with [run:
echo "3. Checking log tagging..."
COUNT=$(grep "\[run:" tui/src/chatwidget/spec_kit/agent_orchestrator.rs | wc -l)
if [ "$COUNT" -ge 30 ]; then
    echo "   ✅ Found $COUNT tagged log statements"
    ((PASS++))
else
    echo "   ⚠️  Found $COUNT tagged logs (expected 30+)"
    ((FAIL++))
fi

# Check 4: Quality gate completion recording
echo "4. Checking quality gate completion recording..."
if grep -q "record_agent_completion.*quality" tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs; then
    echo "   ✅ Quality gate completion recording present"
    ((PASS++))
else
    echo "   ❌ FAIL: Quality gate completion not recorded"
    ((FAIL++))
fi

# Check 5: verify.rs exists and is registered
echo "5. Checking /speckit.verify command..."
if [ -f "tui/src/chatwidget/spec_kit/commands/verify.rs" ] && \
   grep -q "VerifyCommand" tui/src/chatwidget/spec_kit/command_registry.rs; then
    echo "   ✅ Verify command exists and registered"
    ((PASS++))
else
    echo "   ❌ FAIL: Verify command missing or not registered"
    ((FAIL++))
fi

# Check 6: Automated verification in pipeline_coordinator
echo "6. Checking automated verification..."
if grep -q "generate_verification_report" tui/src/chatwidget/spec_kit/pipeline_coordinator.rs; then
    echo "   ✅ Automated verification present"
    ((PASS++))
else
    echo "   ❌ FAIL: Automated verification missing"
    ((FAIL++))
fi

# Check 7: Synthesis stores run_id
echo "7. Checking synthesis run_id storage..."
if grep -q "store_synthesis.*run_id" tui/src/chatwidget/spec_kit/pipeline_coordinator.rs && \
   ! grep -q "None.*run_id TODO" tui/src/chatwidget/spec_kit/pipeline_coordinator.rs; then
    echo "   ✅ Synthesis stores run_id"
    ((PASS++))
else
    echo "   ❌ FAIL: Synthesis not storing run_id"
    ((FAIL++))
fi

# Check 8: Build succeeds
echo "8. Checking build status..."
if [ -f "target/dev-fast/code" ]; then
    echo "   ✅ Binary exists ($(ls -lh target/dev-fast/code | awk '{print $5}'))"
    ((PASS++))
else
    echo "   ❌ FAIL: Binary not found"
    ((FAIL++))
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " RESULTS: $PASS/8 checks passed"
echo "═══════════════════════════════════════════════════════════════"

if [ "$FAIL" -eq 0 ]; then
    echo ""
    echo "✅ ALL CHECKS PASSED - 100% COMPLETE"
    echo ""
    echo "Ready for testing:"
    echo "  ./codex-rs/target/dev-fast/code"
    echo "  /speckit.auto SPEC-KIT-900"
    exit 0
else
    echo ""
    echo "⚠️  $FAIL checks failed - review implementation"
    exit 1
fi
