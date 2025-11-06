#!/bin/bash
# scripts/test-speckit-auto.sh
# Full speckit.auto workflow integration test

set -e

SPEC_ID="${1:-SPEC-KIT-900}"

echo "=========================================="
echo "Spec-Kit Auto Workflow Integration Test"
echo "=========================================="
echo "SPEC: $SPEC_ID"
echo "Command: /speckit.auto $SPEC_ID"
echo "Testing: Full 6-stage pipeline automation"
echo ""

# Step 1: Build
echo "Step 1: Building binary..."
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ ! -f "$REPO_ROOT/build-fast.sh" ]; then
    echo "Error: build-fast.sh not found at $REPO_ROOT"
    exit 1
fi

cd "$REPO_ROOT"
bash build-fast.sh
echo "✓ Build complete"
echo ""

# Step 2: Kill any existing session
echo "Step 2: Cleaning up old sessions..."
bash "$REPO_ROOT/scripts/tui-session.sh" kill 2>/dev/null || true
echo "✓ Cleanup complete"
echo ""

# Step 3: Start TUI with command
echo "Step 3: Starting TUI session with /speckit.auto..."
bash "$REPO_ROOT/scripts/tui-session.sh" start "/speckit.auto $SPEC_ID"
echo "✓ Session started"
echo ""

# Step 4: Wait for execution
echo "Step 4: Waiting for full workflow to complete..."
echo "(This may take 45-50 minutes for complete 6-stage pipeline)"
echo "Stages: specify → plan → tasks → implement → validate → audit → unlock"
echo ""

# Poll for completion or timeout after 60 minutes (auto workflow is longer)
timeout=3600
elapsed=0
while [ $elapsed -lt $timeout ]; do
    sleep 10
    elapsed=$((elapsed + 10))

    # Capture output and look for completion signals
    output=$(bash "$REPO_ROOT/scripts/tui-session.sh" capture 2>/dev/null || echo "")

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
bash "$REPO_ROOT/scripts/tui-session.sh" logs
echo "==================================="
echo ""

# Step 6: Check deliverables from full workflow
echo "Step 6: Checking workflow deliverables..."
echo ""

SPEC_DIR="docs/$SPEC_ID-generic-smoke"
DELIVERABLES=(
    "PRD.md"
    "plan.md"
    "tasks.md"
    "validate.md"
    "audit.md"
)

echo "Checking deliverables in: $SPEC_DIR"
echo ""

found=0
missing=0

for file in "${DELIVERABLES[@]}"; do
    filepath="$SPEC_DIR/$file"
    if [ -f "$filepath" ]; then
        size=$(wc -c < "$filepath")
        lines=$(wc -l < "$filepath")
        echo "✓ $file - $lines lines, $size bytes"
        ((found++))
    else
        echo "✗ $file - MISSING"
        ((missing++))
    fi
done

echo ""
echo "Summary: $found found, $missing missing"
echo ""

# Show preview of plan.md if it exists
if [ -f "$SPEC_DIR/plan.md" ]; then
    echo "Preview of plan.md (first 30 lines):"
    echo "-----------------------------------"
    head -30 "$SPEC_DIR/plan.md"
    echo "-----------------------------------"
fi

echo ""
echo "=========================================="
echo "Spec-Kit Auto Workflow Test Complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  View full output:    bash "$REPO_ROOT/scripts/tui-session.sh" capture"
echo "  Attach to session:   bash "$REPO_ROOT/scripts/tui-session.sh" attach"
echo "  Check status:        bash "$REPO_ROOT/scripts/tui-session.sh" send '/speckit.status $SPEC_ID'"
echo "  View deliverables:   ls -lh $SPEC_DIR"
echo "  Kill session:        bash "$REPO_ROOT/scripts/tui-session.sh" kill"
echo ""
echo "Evidence location:"
echo "  SPEC artifacts:  $SPEC_DIR/"
echo "  Test logs:       Use 'bash "$REPO_ROOT/scripts/tui-session.sh" logs' to review"
