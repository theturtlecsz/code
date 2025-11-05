#!/bin/bash
# scripts/run-tui-command.sh
# Run a command in the code TUI and capture output

set -e

COMMAND="$1"
BINARY="./codex-rs/target/release/code"
LOG_FILE="tui-output-$(date +%s).log"

if [ -z "$COMMAND" ]; then
    echo "Usage: $0 '<command>'"
    echo "Example: $0 '/speckit.plan SPEC-KIT-900'"
    exit 1
fi

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Binary not found at $BINARY"
    echo "Run: bash scripts/build-fast.sh"
    exit 1
fi

echo "Running command: $COMMAND"
echo "Log file: $LOG_FILE"
echo ""

# Run TUI with command and log output
# The TUI accepts the first argument as an initial prompt
"$BINARY" "$COMMAND" 2>&1 | tee "$LOG_FILE"

echo ""
echo "Output saved to: $LOG_FILE"
