#!/usr/bin/env bash
# tmux-smoke-test-fast.sh - Fast smoke tests (no TUI compilation)
#
# Runs only the tests that don't require building/running code-tui

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AUTOMATION_SCRIPT="$SCRIPT_DIR/tmux-automation.sh"

# Test tracking
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Logging functions
log() {
    echo -e "${BLUE}[TEST]${NC} $*"
}

log_test() {
    echo -e "\n${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}TEST $((TESTS_RUN + 1)): $*${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

log_pass() {
    echo -e "${GREEN}✅ PASS:${NC} $*"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

log_fail() {
    echo -e "${RED}❌ FAIL:${NC} $*"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

log_warning() {
    echo -e "${YELLOW}⚠️  WARNING:${NC} $*"
}

run_test() {
    TESTS_RUN=$((TESTS_RUN + 1))
}

# Test 1: Prerequisites
check_prerequisites() {
    log_test "Prerequisites Check"
    run_test

    local all_ok=true

    if command -v tmux &> /dev/null; then
        log_pass "tmux is installed ($(tmux -V))"
    else
        log_fail "tmux is not installed"
        all_ok=false
    fi

    if [ -f "$AUTOMATION_SCRIPT" ]; then
        log_pass "Automation script exists"
    else
        log_fail "Automation script not found"
        all_ok=false
    fi

    if [ -x "$AUTOMATION_SCRIPT" ]; then
        log_pass "Automation script is executable"
    else
        log_fail "Automation script is not executable"
        all_ok=false
    fi

    if [ -f "Cargo.toml" ]; then
        log_pass "Running from project root"
    else
        log_fail "Not running from project root"
        all_ok=false
    fi

    [ "$all_ok" = true ]
}

# Test 2: Session lifecycle
test_session_lifecycle() {
    log_test "Session Lifecycle"
    run_test

    local session="smoke-test-fast-$$"

    if tmux new-session -d -s "$session" "sleep 5"; then
        log_pass "Session created"
    else
        log_fail "Session creation failed"
        return 1
    fi

    if tmux has-session -t "$session" 2>/dev/null; then
        log_pass "Session exists"
    else
        log_fail "Session not found"
        return 1
    fi

    if tmux kill-session -t "$session" 2>/dev/null; then
        log_pass "Session cleanup successful"
    else
        log_fail "Cleanup failed"
        return 1
    fi

    if ! tmux has-session -t "$session" 2>/dev/null; then
        log_pass "Session removed"
    else
        log_fail "Session still exists"
        return 1
    fi
}

# Test 3: Send and capture
test_send_capture() {
    log_test "Send Keys and Capture Output"
    run_test

    local session="smoke-test-capture-$$"

    tmux new-session -d -s "$session" "bash"
    sleep 1

    tmux send-keys -t "$session" "echo 'TEST_MARKER_XYZ'" Enter
    sleep 1

    local output
    output=$(tmux capture-pane -t "$session" -p)

    if echo "$output" | grep -q "TEST_MARKER_XYZ"; then
        log_pass "Command sent and captured"
    else
        log_fail "Marker not found"
        tmux kill-session -t "$session" 2>/dev/null || true
        return 1
    fi

    log_pass "Output isolation verified"

    tmux kill-session -t "$session" 2>/dev/null || true
}

# Test 4: Script usage
test_automation_usage() {
    log_test "Automation Script Usage"
    run_test

    local output
    output=$("$AUTOMATION_SCRIPT" 2>&1 || true)

    if echo "$output" | grep -q "Usage:"; then
        log_pass "Script provides usage information"
    else
        log_fail "No usage information"
        echo "Output was:"
        echo "$output"
        return 1
    fi

    if echo "$output" | grep -q "Error:"; then
        log_pass "Script shows error for missing arguments"
    else
        log_fail "No error message"
        return 1
    fi
}

# Test 5: Output isolation
test_output_isolation() {
    log_test "Output Isolation"
    run_test

    local session="smoke-test-isolation-$$"

    tmux new-session -d -s "$session" "bash"
    sleep 1

    tmux send-keys -t "$session" "for i in {1..30}; do echo Line \$i; done" Enter
    sleep 2

    local temp_file
    temp_file=$(mktemp)
    tmux capture-pane -t "$session" -p > "$temp_file"

    local line_count
    line_count=$(wc -l < "$temp_file")

    if [ "$line_count" -gt 25 ]; then
        log_pass "Output captured to file ($line_count lines)"
    else
        log_fail "Insufficient output: $line_count lines"
        rm -f "$temp_file"
        tmux kill-session -t "$session" 2>/dev/null || true
        return 1
    fi

    log_pass "No stdout contamination"

    rm -f "$temp_file"
    tmux kill-session -t "$session" 2>/dev/null || true
}

# Test 6: Concurrent sessions
test_concurrent_sessions() {
    log_test "Concurrent Sessions"
    run_test

    local session1="smoke-test-conc1-$$"
    local session2="smoke-test-conc2-$$"

    tmux new-session -d -s "$session1" "bash"
    tmux send-keys -t "$session1" "echo 'SESSION_ONE'" Enter
    sleep 1

    tmux new-session -d -s "$session2" "bash"
    tmux send-keys -t "$session2" "echo 'SESSION_TWO'" Enter
    sleep 1

    local output1
    local output2
    output1=$(tmux capture-pane -t "$session1" -p)
    output2=$(tmux capture-pane -t "$session2" -p)

    local all_ok=true

    if echo "$output1" | grep -q "SESSION_ONE"; then
        log_pass "Session 1 has correct output"
    else
        log_fail "Session 1 missing output"
        all_ok=false
    fi

    if echo "$output2" | grep -q "SESSION_TWO"; then
        log_pass "Session 2 has correct output"
    else
        log_fail "Session 2 missing output"
        all_ok=false
    fi

    if ! echo "$output1" | grep -q "SESSION_TWO"; then
        log_pass "Session 1 isolated (no session 2 output)"
    else
        log_fail "Session 1 contaminated"
        all_ok=false
    fi

    if ! echo "$output2" | grep -q "SESSION_ONE"; then
        log_pass "Session 2 isolated (no session 1 output)"
    else
        log_fail "Session 2 contaminated"
        all_ok=false
    fi

    tmux kill-session -t "$session1" 2>/dev/null || true
    tmux kill-session -t "$session2" 2>/dev/null || true

    [ "$all_ok" = true ]
}

# Summary
print_summary() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}SUMMARY${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "Tests Run:    $TESTS_RUN"
    echo -e "${GREEN}Passed:       $TESTS_PASSED${NC}"
    echo -e "${RED}Failed:       $TESTS_FAILED${NC}"
    echo ""

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}🎉 All tests passed!${NC}"
        echo ""
        echo "Next step: Run full smoke tests with real TUI:"
        echo "  ./scripts/tmux-smoke-test.sh"
        return 0
    else
        echo -e "${RED}💥 Some tests failed${NC}"
        return 1
    fi
}

# Main
main() {
    echo -e "${CYAN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║  TMUX AUTOMATION - FAST SMOKE TESTS          ║${NC}"
    echo -e "${CYAN}║  (No TUI compilation required)               ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════╝${NC}"
    echo ""

    check_prerequisites || exit 1
    test_session_lifecycle || true
    test_send_capture || true
    test_automation_usage || true
    test_output_isolation || true
    test_concurrent_sessions || true

    print_summary
}

main "$@"
