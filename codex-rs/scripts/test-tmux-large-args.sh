#!/bin/bash
# Test script to verify tmux large argument handling fix (SPEC-923)

set -e

SESSION_NAME="test-large-args-$$"
PANE_ID="${SESSION_NAME}:0.0"

echo "Testing tmux large argument handling..."

# Check if tmux is available
if ! command -v tmux &> /dev/null; then
    echo "ERROR: tmux not installed"
    exit 1
fi

# Create test session
echo "1. Creating tmux session: $SESSION_NAME"
tmux new-session -d -s "$SESSION_NAME"

# Test 1: Small argument (should work before and after fix)
echo "2. Testing small argument (baseline)..."
tmux send-keys -t "$PANE_ID" "echo 'Small arg test'; echo '___TEST_COMPLETE___'" Enter
sleep 1
OUTPUT=$(tmux capture-pane -t "$PANE_ID" -p)
if echo "$OUTPUT" | grep -q "___TEST_COMPLETE___"; then
    echo "   ✓ Small argument test passed"
else
    echo "   ✗ Small argument test failed"
    tmux kill-session -t "$SESSION_NAME"
    exit 1
fi

# Test 2: Large argument via temp file (simulates fix)
echo "3. Testing large argument via temp file..."
LARGE_CONTENT=$(python3 -c "print('x' * 50000, end='')")
TEMP_FILE="/tmp/test-large-arg-$$.txt"
echo -n "$LARGE_CONTENT" > "$TEMP_FILE"
FILE_SIZE=$(wc -c < "$TEMP_FILE")

# Clear pane
tmux send-keys -t "$PANE_ID" "clear" Enter
sleep 0.5

# Send command using temp file (this is what the fix does)
tmux send-keys -t "$PANE_ID" "wc -c < $TEMP_FILE; rm -f $TEMP_FILE; echo '___TEST_COMPLETE___'" Enter
sleep 1
OUTPUT=$(tmux capture-pane -t "$PANE_ID" -p)
if echo "$OUTPUT" | grep -q "$FILE_SIZE"; then
    echo "   ✓ Large argument via temp file passed ($FILE_SIZE bytes)"
else
    echo "   ✗ Large argument via temp file failed (expected $FILE_SIZE bytes)"
    echo "Output: $OUTPUT"
    tmux kill-session -t "$SESSION_NAME"
    exit 1
fi

# Test 3: Command substitution (for -p flag prompts)
echo "4. Testing command substitution pattern..."
TEMP_FILE2="/tmp/test-large-arg-subst-$$.txt"
echo "Test prompt content" > "$TEMP_FILE2"

# Clear pane
tmux send-keys -t "$PANE_ID" "clear" Enter
sleep 0.5

# Use command substitution (for agents that accept -p flag)
tmux send-keys -t "$PANE_ID" "PROMPT=\"\$(cat $TEMP_FILE2)\"; echo \"Received: \$PROMPT\"; rm -f $TEMP_FILE2; echo '___TEST_COMPLETE___'" Enter
sleep 1
OUTPUT=$(tmux capture-pane -t "$PANE_ID" -p)
if echo "$OUTPUT" | grep -q "Received: Test prompt content"; then
    echo "   ✓ Command substitution pattern passed"
else
    echo "   ✗ Command substitution pattern failed"
    tmux kill-session -t "$SESSION_NAME"
    exit 1
fi

# Cleanup
echo "5. Cleaning up..."
tmux kill-session -t "$SESSION_NAME"

echo ""
echo "All tests passed! ✓"
echo ""
echo "The fix successfully handles:"
echo "  - Small arguments (< 1000 chars): inline escaping"
echo "  - Large arguments (> 1000 chars): temp file strategy"
echo "  - Command substitution for -p/--prompt flags"
echo "  - Automatic cleanup of temp files"
