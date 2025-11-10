#!/usr/bin/env bash
# tmux-automation.sh - Send commands to codex-tui via tmux
#
# Usage:
#   ./tmux-automation.sh <spec-id> <command> [timeout]
#
# Examples:
#   ./tmux-automation.sh SPEC-KIT-070 "/speckit.status SPEC-KIT-070"
#   ./tmux-automation.sh SPEC-KIT-070 "/speckit.plan SPEC-KIT-070" 600

set -euo pipefail

# Color output for better readability
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PARENT_ROOT="$(cd "$PROJECT_ROOT/.." && pwd)"
BUILD_SCRIPT="$PARENT_ROOT/build-fast.sh"
TUI_BINARY="$PROJECT_ROOT/target/dev-fast/code"
EVIDENCE_BASE="$PROJECT_ROOT/evidence/tmux-automation"

# Show usage if no arguments
show_usage() {
    echo "Usage: $0 <spec-id> <command> [timeout]"
    echo ""
    echo "Arguments:"
    echo "  spec-id   SPEC identifier (e.g., SPEC-KIT-070)"
    echo "  command   Command to send to TUI (e.g., '/speckit.status SPEC-KIT-070')"
    echo "  timeout   Optional timeout in seconds (default: 300)"
    echo ""
    echo "Examples:"
    echo "  $0 SPEC-KIT-070 '/speckit.status SPEC-KIT-070'"
    echo "  $0 SPEC-KIT-070 '/speckit.plan SPEC-KIT-070' 600"
    exit 1
}

# Parse arguments
if [ $# -lt 2 ]; then
    echo "Error: Missing required arguments" >&2
    echo "" >&2
    show_usage
fi

SPEC_ID="$1"
COMMAND="$2"
TIMEOUT="${3:-300}"  # Default 5 minutes

# Session naming
SESSION="codex-automation-${SPEC_ID}-$$"

# Logging
log() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[$(date '+%H:%M:%S')] ✅${NC} $*"
}

log_error() {
    echo -e "${RED}[$(date '+%H:%M:%S')] ❌${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[$(date '+%H:%M:%S')] ⚠️${NC} $*"
}

# Cleanup function
cleanup() {
    local exit_code=$?
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        log "Cleaning up tmux session: $SESSION"
        tmux kill-session -t "$SESSION" 2>/dev/null || true
    fi
    exit $exit_code
}

trap cleanup EXIT INT TERM

# Initialize tmux session
init_session() {
    log "Initializing tmux session: $SESSION"

    # Check if tmux is available
    if ! command -v tmux &> /dev/null; then
        log_error "tmux is not installed. Please install it first."
        exit 1
    fi

    # Build TUI using build-fast.sh if binary doesn't exist
    if [ ! -x "$TUI_BINARY" ]; then
        log "Binary not found at $TUI_BINARY, building..."
        if [ -x "$BUILD_SCRIPT" ]; then
            log "Running $BUILD_SCRIPT"
            if ! "$BUILD_SCRIPT"; then
                log_error "Build failed"
                exit 1
            fi
        else
            log_error "Build script not found: $BUILD_SCRIPT"
            exit 1
        fi
    else
        log "Using existing binary: $TUI_BINARY"
    fi

    # Start TUI in tmux session
    if ! tmux new-session -d -s "$SESSION" "cd '$PROJECT_ROOT' && '$TUI_BINARY'"; then
        log_error "Failed to create tmux session"
        exit 1
    fi

    log "Waiting for TUI initialization..."
    sleep 12  # Allow time for TUI to fully startup before sending commands

    # Verify session still exists (TUI didn't crash immediately)
    if ! tmux has-session -t "$SESSION" 2>/dev/null; then
        log_error "TUI session terminated unexpectedly"
        return 1
    fi

    # Note: TUI uses alternate screen, so capture-pane may be empty initially
    # We'll verify successful startup by checking the session persists
    log_success "TUI session initialized"
    return 0
}

# Send command to TUI
send_command() {
    local cmd="$1"
    log "Sending command: $cmd"

    if ! tmux send-keys -t "$SESSION" "$cmd" Enter; then
        log_error "Failed to send command to tmux session"
        return 1
    fi

    # Brief pause to let command register
    sleep 1
    return 0
}

# Check if command completed
is_complete() {
    local output="$1"

    # Look for TUI ready indicators
    # The TUI shows "Ctrl+H help" at bottom when ready for input
    if echo "$output" | grep -qF "Ctrl+H help"; then
        # Make sure we're not in middle of processing
        if ! echo "$output" | grep -qE "(Processing\.\.\.|Running\.\.\.|Loading\.\.\.|Executing|⏳.*running)"; then
            return 0
        fi
    fi

    return 1
}

# Check for errors in output
has_error() {
    local output="$1"

    # Look for error indicators
    if echo "$output" | grep -qiE "(Error:|Failed:|Aborted|Panic|Fatal)"; then
        return 0
    fi

    return 1
}

# Wait for command completion
wait_for_completion() {
    local timeout="$1"
    local elapsed=0
    local check_interval=2
    local last_output=""
    local stable_count=0

    log "Waiting for command completion (timeout: ${timeout}s)..."

    while [ $elapsed -lt $timeout ]; do
        # Capture last 30 lines to check status
        local output
        output=$(tmux capture-pane -t "$SESSION" -p | tail -n 30)

        # Check for errors first
        if has_error "$output"; then
            log_error "Command failed - error detected in output"
            echo "$output" | tail -n 20
            return 1
        fi

        # Check for completion
        if is_complete "$output"; then
            # Verify output is stable (hasn't changed in 2 checks)
            if [ "$output" = "$last_output" ]; then
                stable_count=$((stable_count + 1))
                if [ $stable_count -ge 2 ]; then
                    log_success "Command completed after ${elapsed}s"
                    return 0
                fi
            else
                stable_count=0
            fi
            last_output="$output"
        else
            # Reset stability counter if still processing
            stable_count=0
            last_output="$output"
        fi

        sleep $check_interval
        elapsed=$((elapsed + check_interval))

        # Progress indicator
        if [ $((elapsed % 10)) -eq 0 ]; then
            echo -n "."
        fi
    done

    echo ""
    log_warning "Timeout reached after ${timeout}s"
    return 2
}

# Capture output to evidence file
capture_evidence() {
    local status="$1"
    local evidence_dir="$EVIDENCE_BASE/${SPEC_ID}"

    mkdir -p "$evidence_dir"

    local timestamp
    timestamp=$(date +%Y%m%d-%H%M%S)
    local output_file="${evidence_dir}/tmux-${status}-${timestamp}.txt"

    log "Capturing output to: $output_file"

    # Capture full scrollback history
    tmux capture-pane -t "$SESSION" -p -S -3000 > "$output_file"

    # Also create a recent-only file for quick inspection
    local recent_file="${evidence_dir}/tmux-${status}-recent-${timestamp}.txt"
    tail -n 100 "$output_file" > "$recent_file"

    log_success "Evidence captured ($(wc -l < "$output_file") lines)"
    echo "$output_file"
}

# Main execution flow
main() {
    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log "TMUX Automation for $SPEC_ID"
    log "Command: $COMMAND"
    log "Timeout: ${TIMEOUT}s"
    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Step 1: Initialize session
    if ! init_session; then
        log_error "Session initialization failed"
        capture_evidence "init-failed"
        exit 1
    fi

    # Step 2: Send command
    if ! send_command "$COMMAND"; then
        log_error "Failed to send command"
        capture_evidence "send-failed"
        exit 1
    fi

    # Step 3: Wait for completion
    local result=0
    if ! wait_for_completion "$TIMEOUT"; then
        result=$?
    fi

    # Step 4: Capture evidence
    case $result in
        0)
            local evidence_file
            evidence_file=$(capture_evidence "success")
            log_success "Automation completed successfully"
            log "Evidence: $evidence_file"
            ;;
        1)
            local evidence_file
            evidence_file=$(capture_evidence "error")
            log_error "Command execution failed"
            log "Evidence: $evidence_file"
            exit 1
            ;;
        2)
            local evidence_file
            evidence_file=$(capture_evidence "timeout")
            log_error "Command timed out"
            log "Evidence: $evidence_file"
            exit 2
            ;;
    esac

    return $result
}

# Run main function
main
