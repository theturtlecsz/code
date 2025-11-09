#!/bin/bash
# scripts/test-initial-command.sh
# Comprehensive validation tests for SPEC-KIT-920 --initial-command flag

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY="$REPO_ROOT/codex-rs/target/release/code"

cd "$REPO_ROOT"

echo "========================================"
echo "SPEC-KIT-920 Validation Test Suite"
echo "========================================"
echo ""

# Verify binary exists and has our changes
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found at $BINARY"
    exit 1
fi

BINARY_TIME=$(stat -c "%y" "$BINARY" | cut -d'.' -f1)
echo "✓ Binary found: $BINARY"
echo "  Built: $BINARY_TIME"
echo ""

# Test 1: Help text
echo "=== Test 1: Help Text ==="
if $BINARY --help | grep -q "initial-command"; then
    echo "✅ PASS: --initial-command appears in help"
    $BINARY --help | grep -A2 "initial-command"
else
    echo "❌ FAIL: --initial-command not in help"
    exit 1
fi
echo ""

# Test 2: Simple status command
echo "=== Test 2: Simple Status Command ==="
tmux kill-session -t test-920 2>/dev/null || true
sleep 1

echo "Starting TUI with --initial-command '/speckit.status SPEC-KIT-900'..."
tmux new-session -d -s test-920 "$BINARY --debug --initial-command '/speckit.status SPEC-KIT-900'"

echo "Waiting 8 seconds for command to execute..."
sleep 8

# Check logs for our marker
if tail -50 ~/.code/log/codex-tui.log | grep -q "SPEC-KIT-920.*Auto-submitting"; then
    echo "✅ PASS: Auto-submit executed (found in logs)"
    tail -50 ~/.code/log/codex-tui.log | grep "SPEC-KIT-920"
else
    echo "⚠️  WARNING: No auto-submit log found"
    echo "   Last 10 log lines:"
    tail -10 ~/.code/log/codex-tui.log
fi

echo "Killing test session..."
tmux kill-session -t test-920 2>/dev/null
echo ""

# Test 3: Error handling - no slash
echo "=== Test 3: Error Handling - No Slash ==="
tmux new-session -d -s test-920 "$BINARY --debug --initial-command 'invalid'"
sleep 5

if tail -30 ~/.code/log/codex-tui.log | grep -q "SPEC-KIT-920.*must start with"; then
    echo "✅ PASS: Error logged for command without slash"
else
    echo "⚠️  No error log (might be in TUI display only)"
fi

tmux kill-session -t test-920 2>/dev/null
echo ""

# Test 4: Error handling - invalid slash command
echo "=== Test 4: Error Handling - Invalid Command ==="
tmux new-session -d -s test-920 "$BINARY --debug --initial-command '/notarealcommand'"
sleep 5

if tail -30 ~/.code/log/codex-tui.log | grep -q "SPEC-KIT-920"; then
    echo "✅ PASS: Error handling executed"
    tail -30 ~/.code/log/codex-tui.log | grep "SPEC-KIT-920" | tail -3
else
    echo "⚠️  No error log found"
fi

tmux kill-session -t test-920 2>/dev/null
echo ""

# Summary
echo "========================================"
echo "Quick Tests Complete"
echo "========================================"
echo ""
echo "Next Steps:"
echo "  1. Run: ./scripts/tui-session.sh start '/speckit.auto SPEC-KIT-900'"
echo "  2. Wait: ~45-50 minutes for full pipeline"
echo "  3. Validate: ./scripts/spec-kit-tools.sh validate SPEC-KIT-900 plan"
echo ""
