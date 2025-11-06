#!/bin/bash
# scripts/tui-session.sh
# Manage TUI sessions in tmux for background execution

set -e

# Get repo root relative to this script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SESSION_NAME="code-tui"
BINARY="$REPO_ROOT/codex-rs/target/dev-fast/code"

function usage() {
    cat <<EOF
Usage: $0 <command> [args...]

Commands:
  start <command>     Start new TUI session with command
  send <command>      Send command to running session
  attach              Attach to running session
  capture             Capture current output
  logs                Show captured output
  kill                Kill the session
  status              Check if session is running

Examples:
  $0 start "/speckit.plan SPEC-KIT-900"
  $0 send "/speckit.status SPEC-KIT-900"
  $0 capture > output.txt
  $0 attach
  $0 kill
EOF
    exit 1
}

function check_binary() {
    if [ ! -f "$BINARY" ]; then
        echo "Error: Binary not found at $BINARY"
        echo "Run: bash scripts/build-fast.sh"
        exit 1
    fi
}

function session_exists() {
    tmux has-session -t "$SESSION_NAME" 2>/dev/null
}

function cmd_start() {
    local command="$1"

    if [ -z "$command" ]; then
        echo "Error: No command provided"
        usage
    fi

    check_binary

    if session_exists; then
        echo "Session already running. Kill it first with: $0 kill"
        exit 1
    fi

    echo "Starting TUI session with command: $command"
    echo "Binary: $BINARY"
    echo "Working dir: $REPO_ROOT"

    # Start tmux session with TUI and initial command in repo root
    cd "$REPO_ROOT"
    tmux new-session -d -s "$SESSION_NAME" -c "$REPO_ROOT" "$BINARY '$command'"

    echo "Session started: $SESSION_NAME"
    echo "Attach with: $0 attach"
    echo "Capture output with: $0 capture"
}

function cmd_send() {
    local command="$1"

    if [ -z "$command" ]; then
        echo "Error: No command provided"
        usage
    fi

    if ! session_exists; then
        echo "Error: No session running. Start one with: $0 start"
        exit 1
    fi

    echo "Sending command: $command"
    tmux send-keys -t "$SESSION_NAME" "$command" C-m
}

function cmd_attach() {
    if ! session_exists; then
        echo "Error: No session running. Start one with: $0 start"
        exit 1
    fi

    echo "Attaching to session (Ctrl-b d to detach)..."
    tmux attach -t "$SESSION_NAME"
}

function cmd_capture() {
    if ! session_exists; then
        echo "Error: No session running"
        exit 1
    fi

    tmux capture-pane -t "$SESSION_NAME" -p
}

function cmd_logs() {
    if ! session_exists; then
        echo "Error: No session running"
        exit 1
    fi

    echo "=== TUI Output (last 50 lines) ==="
    tmux capture-pane -t "$SESSION_NAME" -p -S -50
}

function cmd_kill() {
    if ! session_exists; then
        echo "No session to kill"
        exit 0
    fi

    echo "Killing session: $SESSION_NAME"
    tmux kill-session -t "$SESSION_NAME"
    echo "Session killed"
}

function cmd_status() {
    if session_exists; then
        echo "Session '$SESSION_NAME' is RUNNING"
        echo "Attach with: $0 attach"
        exit 0
    else
        echo "Session '$SESSION_NAME' is NOT running"
        exit 1
    fi
}

# Main command dispatcher
case "${1:-}" in
    start)
        cmd_start "$2"
        ;;
    send)
        cmd_send "$2"
        ;;
    attach)
        cmd_attach
        ;;
    capture)
        cmd_capture
        ;;
    logs)
        cmd_logs
        ;;
    kill)
        cmd_kill
        ;;
    status)
        cmd_status
        ;;
    *)
        usage
        ;;
esac
