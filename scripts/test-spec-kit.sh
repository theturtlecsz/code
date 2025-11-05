#!/bin/bash
# scripts/test-spec-kit.sh
# Quick workflow test script

set -e

SPEC_ID="${1:-SPEC-KIT-900}"
COMMAND="${2:-/speckit.plan}"

echo "==================================="
echo "Spec-Kit Workflow Test"
echo "==================================="
echo "SPEC: $SPEC_ID"
echo "Command: $COMMAND $SPEC_ID"
echo ""

# Step 1: Build
echo "Step 1: Building binary..."
if [ ! -f "scripts/build-fast.sh" ]; then
    echo "Error: build-fast.sh not found"
    exit 1
fi

bash scripts/build-fast.sh
echo "✓ Build complete"
echo ""

# Step 2: Kill any existing session
echo "Step 2: Cleaning up old sessions..."
bash scripts/tui-session.sh kill 2>/dev/null || true
echo "✓ Cleanup complete"
echo ""

# Step 3: Start TUI with command
echo "Step 3: Starting TUI session..."
bash scripts/tui-session.sh start "$COMMAND $SPEC_ID"
echo "✓ Session started"
echo ""

# Step 4: Wait for execution
echo "Step 4: Waiting for command to complete..."
echo "(This may take 3-10 minutes depending on the stage)"
echo ""

# Poll for completion or timeout after 15 minutes
timeout=900
elapsed=0
while [ $elapsed -lt $timeout ]; do
    sleep 10
    elapsed=$((elapsed + 10))

    # Capture output and look for completion signals
    output=$(bash scripts/tui-session.sh capture 2>/dev/null || echo "")

    if echo "$output" | grep -q "completed\|Error\|Failed"; then
        echo "✓ Command completed (after ${elapsed}s)"
        break
    fi

    if [ $((elapsed % 60)) -eq 0 ]; then
        echo "  ... still running (${elapsed}s elapsed)"
    fi
done

echo ""

# Step 5: Show output
echo "Step 5: Capturing output..."
echo "==================================="
bash scripts/tui-session.sh logs
echo "==================================="
echo ""

# Step 6: Check deliverable
if [ "$COMMAND" = "/speckit.plan" ]; then
    DELIVERABLE="docs/$SPEC_ID-generic-smoke/plan.md"
elif [ "$COMMAND" = "/speckit.tasks" ]; then
    DELIVERABLE="docs/$SPEC_ID-generic-smoke/tasks.md"
elif [ "$COMMAND" = "/speckit.validate" ]; then
    DELIVERABLE="docs/$SPEC_ID-generic-smoke/validate.md"
else
    DELIVERABLE=""
fi

if [ -n "$DELIVERABLE" ] && [ -f "$DELIVERABLE" ]; then
    echo "Step 6: Checking deliverable..."
    size=$(wc -c < "$DELIVERABLE")
    lines=$(wc -l < "$DELIVERABLE")
    echo "✓ Deliverable exists: $DELIVERABLE"
    echo "  Size: $size bytes, Lines: $lines"
    echo ""
    echo "Preview (first 30 lines):"
    echo "-----------------------------------"
    head -30 "$DELIVERABLE"
    echo "-----------------------------------"
fi

echo ""
echo "==================================="
echo "Test Complete!"
echo "==================================="
echo ""
echo "Next steps:"
echo "  View full output:  bash scripts/tui-session.sh capture"
echo "  Attach to session: bash scripts/tui-session.sh attach"
echo "  Send new command:  bash scripts/tui-session.sh send '/speckit.status $SPEC_ID'"
echo "  Kill session:      bash scripts/tui-session.sh kill"
