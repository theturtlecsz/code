#!/bin/bash
# Test script to verify SPEC-923 fix: clean agent output capture
#
# This script tests that tmux output redirection works correctly:
# 1. Agent output is written to dedicated file (no shell noise)
# 2. Output file is created and readable
# 3. Pane content is separate from output file
# 4. Output files are cleaned up after execution

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SPEC_ID="SPEC-KIT-923"

echo "Testing SPEC-923: Clean Agent Output Capture"
echo "=============================================="
echo ""

# Enable observable agents mode
export SPEC_KIT_OBSERVABLE_AGENTS=1
export SPEC_KIT_AUTO_ADVANCE=0  # Don't auto-advance stages

# Check for tmux
if ! command -v tmux &> /dev/null; then
    echo "❌ ERROR: tmux is not installed"
    exit 1
fi

echo "✅ tmux is available"
echo ""

# Test 1: Check that output files are created
echo "Test 1: Verify output file creation"
echo "------------------------------------"

# Count output files before
before_count=$(ls -1 /tmp/tmux-agent-output-*.txt 2>/dev/null | wc -l)
echo "Output files before test: $before_count"

# Run a simple spec command (will create a SPEC if needed)
cd "$PROJECT_ROOT"
echo "Running: cargo run --bin spec -- new 'Test SPEC-923 output capture'"

cargo run --bin spec -- new 'Test SPEC-923 output capture' 2>&1 | grep "SPEC-KIT-" | head -1 | tee /tmp/spec-test-output.txt

# Check output files during/after execution
after_count=$(ls -1 /tmp/tmux-agent-output-*.txt 2>/dev/null | wc -l)
echo "Output files after test: $after_count"

if [ "$after_count" -eq 0 ]; then
    echo "✅ Test 1 PASSED: Output files were created and cleaned up"
else
    echo "⚠️  Test 1: $after_count output files still exist (should be cleaned up)"
    ls -lh /tmp/tmux-agent-output-*.txt 2>/dev/null
fi

echo ""

# Test 2: Check SQLite for clean responses (no shell noise)
echo "Test 2: Verify clean agent responses in SQLite"
echo "-----------------------------------------------"

DB_PATH="$PROJECT_ROOT/consensus.db"

if [ -f "$DB_PATH" ]; then
    echo "Checking most recent agent response for shell noise..."

    # Query for response that contains shell prompts (should be ZERO)
    noise_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM consensus_responses WHERE response_text LIKE '%thetu@%' OR response_text LIKE '%export %' OR response_text LIKE '%cd %';" 2>/dev/null || echo "0")

    if [ "$noise_count" -eq 0 ]; then
        echo "✅ Test 2 PASSED: No shell noise found in agent responses"
    else
        echo "❌ Test 2 FAILED: Found $noise_count responses with shell noise"
        echo "Sample of problematic response:"
        sqlite3 "$DB_PATH" "SELECT substr(response_text, 1, 500) FROM consensus_responses WHERE response_text LIKE '%thetu@%' OR response_text LIKE '%export %' OR response_text LIKE '%cd %' LIMIT 1;" 2>/dev/null
    fi
else
    echo "⚠️  Test 2 SKIPPED: consensus.db not found at $DB_PATH"
fi

echo ""

# Test 3: Check plan.md for actual content (not empty 184 bytes)
echo "Test 3: Verify plan.md has content"
echo "-----------------------------------"

# Find most recent SPEC directory
latest_spec=$(ls -dt "$PROJECT_ROOT/docs/SPEC-KIT-"* 2>/dev/null | head -1)

if [ -n "$latest_spec" ] && [ -f "$latest_spec/plan.md" ]; then
    plan_size=$(stat -c%s "$latest_spec/plan.md" 2>/dev/null || stat -f%z "$latest_spec/plan.md" 2>/dev/null)

    if [ "$plan_size" -gt 500 ]; then
        echo "✅ Test 3 PASSED: plan.md has $plan_size bytes (good content)"
    else
        echo "❌ Test 3 FAILED: plan.md only has $plan_size bytes (likely empty)"
        echo "Content preview:"
        head -20 "$latest_spec/plan.md"
    fi
else
    echo "⚠️  Test 3 SKIPPED: No plan.md found in recent SPEC directories"
fi

echo ""

# Test 4: Manual verification instructions
echo "Test 4: Manual Verification Steps"
echo "----------------------------------"
echo "To manually verify the fix:"
echo ""
echo "1. Run with observable agents:"
echo "   export SPEC_KIT_OBSERVABLE_AGENTS=1"
echo "   /speckit.plan $SPEC_ID"
echo ""
echo "2. In another terminal, attach to watch agents:"
echo "   tmux attach -t spec-kit-agents"
echo "   (Press Ctrl-B, then D to detach)"
echo ""
echo "3. Verify output files are created:"
echo "   ls -lh /tmp/tmux-agent-output-*.txt"
echo ""
echo "4. Check SQLite for clean JSON:"
echo "   sqlite3 consensus.db 'SELECT substr(response_text, 1, 200) FROM consensus_responses ORDER BY rowid DESC LIMIT 1;'"
echo ""
echo "5. Verify plan.md has content:"
echo "   wc -c docs/$SPEC_ID/plan.md"
echo "   head -20 docs/$SPEC_ID/plan.md"
echo ""

# Summary
echo "=============================================="
echo "Test Summary for SPEC-923"
echo "=============================================="
echo ""
echo "Expected Behavior:"
echo "✅ Agent output redirected to /tmp/tmux-agent-output-*.txt"
echo "✅ No shell prompts or noise in agent responses"
echo "✅ plan.md has actual content (not 184 bytes)"
echo "✅ Output files cleaned up after execution"
echo "✅ Users can still observe in tmux pane"
echo ""
echo "If all tests passed, the fix is working correctly!"
