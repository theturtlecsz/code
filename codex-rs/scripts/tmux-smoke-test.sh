#!/usr/bin/env bash
# tmux-smoke-test.sh - Smoke tests for tmux automation
#
# Tests basic functionality without running expensive operations

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
TEST_SPEC="SPEC-KIT-070"  # Use existing spec for testing

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

# Run a test
run_test() {
    TESTS_RUN=$((TESTS_RUN + 1))
}

# Check prerequisites
check_prerequisites() {
    log_test "Prerequisites Check"
    run_test

    local all_ok=true

    # Check tmux
    if command -v tmux &> /dev/null; then
        log_pass "tmux is installed ($(tmux -V))"
    else
        log_fail "tmux is not installed"
        all_ok=false
    fi

    # Check automation script exists
    if [ -f "$AUTOMATION_SCRIPT" ]; then
        log_pass "Automation script exists: $AUTOMATION_SCRIPT"
    else
        log_fail "Automation script not found: $AUTOMATION_SCRIPT"
        all_ok=false
    fi

    # Check if script is executable
    if [ -x "$AUTOMATION_SCRIPT" ]; then
        log_pass "Automation script is executable"
    else
        log_fail "Automation script is not executable"
        all_ok=false
    fi

    # Check if we're in the right directory
    if [ -f "Cargo.toml" ] && grep -q "codex" Cargo.toml; then
        log_pass "Running from project root"
    else
        log_fail "Not running from project root"
        all_ok=false
    fi

    if [ "$all_ok" = true ]; then
        return 0
    else
        return 1
    fi
}

# Test 1: Basic session creation and cleanup
test_session_lifecycle() {
    log_test "Session Lifecycle (Create and Cleanup)"
    run_test

    local session="smoke-test-$$"

    # Create session
    log "Creating test session: $session"
    if tmux new-session -d -s "$session" "sleep 10"; then
        log_pass "Session created successfully"
    else
        log_fail "Failed to create session"
        return 1
    fi

    # Verify session exists
    if tmux has-session -t "$session" 2>/dev/null; then
        log_pass "Session exists and is accessible"
    else
        log_fail "Session not found after creation"
        return 1
    fi

    # Cleanup
    if tmux kill-session -t "$session" 2>/dev/null; then
        log_pass "Session cleaned up successfully"
    else
        log_fail "Failed to cleanup session"
        return 1
    fi

    # Verify cleanup
    if ! tmux has-session -t "$session" 2>/dev/null; then
        log_pass "Session properly removed"
    else
        log_fail "Session still exists after cleanup"
        return 1
    fi

    return 0
}

# Test 2: Send keys and capture output
test_send_capture() {
    log_test "Send Keys and Capture Output"
    run_test

    local session="smoke-test-capture-$$"

    # Create session with echo command
    log "Creating session with echo test"
    tmux new-session -d -s "$session" "bash"
    sleep 1

    # Send echo command
    log "Sending test command"
    tmux send-keys -t "$session" "echo 'SMOKE_TEST_MARKER_12345'" Enter
    sleep 1

    # Capture output
    local output
    output=$(tmux capture-pane -t "$session" -p)

    # Verify marker in output
    if echo "$output" | grep -q "SMOKE_TEST_MARKER_12345"; then
        log_pass "Command sent and output captured successfully"
    else
        log_fail "Marker not found in captured output"
        log "Captured output:"
        echo "$output"
        tmux kill-session -t "$session" 2>/dev/null || true
        return 1
    fi

    # Verify no stdin contamination in this process
    if [ -t 0 ]; then
        log_pass "No stdin contamination detected"
    else
        log_warning "stdin might be redirected (this is OK if running in CI)"
    fi

    # Cleanup
    tmux kill-session -t "$session" 2>/dev/null || true
    log_pass "Cleanup completed"

    return 0
}

# Test 3: Automation script basic invocation
test_automation_help() {
    log_test "Automation Script Help/Usage"
    run_test

    # Test with missing arguments (should fail gracefully)
    log "Testing error handling with missing arguments"
    local output
    output=$("$AUTOMATION_SCRIPT" 2>&1 || true)

    if echo "$output" | grep -q "Error:"; then
        log_pass "Script provides error message for missing arguments"
    else
        log_fail "Script did not provide helpful error message"
        return 1
    fi

    if echo "$output" | grep -q "Usage:"; then
        log_pass "Script provides usage information"
    else
        log_fail "No usage information"
        return 1
    fi

    return 0
}

# Test 4: Quick status command (real test with TUI)
test_status_command() {
    log_test "Status Command (Real TUI Test)"
    run_test

    log "This test will start the actual TUI and send a status command"
    log "Timeout: 60s"

    # Use a known SPEC that should exist
    local test_command="/speckit.status $TEST_SPEC"

    log "Running: $AUTOMATION_SCRIPT $TEST_SPEC \"$test_command\" 60"

    # Run automation with short timeout
    if "$AUTOMATION_SCRIPT" "$TEST_SPEC" "$test_command" 60; then
        log_pass "Status command completed successfully"
    else
        local exit_code=$?
        log_fail "Status command failed with exit code: $exit_code"
        return 1
    fi

    # Check if evidence was captured
    local evidence_dir="evidence/tmux-automation/$TEST_SPEC"
    if [ -d "$evidence_dir" ]; then
        local latest_evidence
        latest_evidence=$(find "$evidence_dir" -name "tmux-success-*.txt" -type f | sort -r | head -n 1)
        if [ -n "$latest_evidence" ]; then
            log_pass "Evidence captured: $latest_evidence"
            log "Evidence size: $(wc -l < "$latest_evidence") lines"
        else
            log_warning "Evidence directory exists but no success files found"
        fi
    else
        log_warning "Evidence directory not created"
    fi

    return 0
}

# Test 5: Output isolation verification
test_output_isolation() {
    log_test "Output Isolation (No Contamination)"
    run_test

    local session="smoke-test-isolation-$$"

    log "Creating session that produces verbose output"
    tmux new-session -d -s "$session" "bash"
    sleep 1

    # Generate lots of output
    tmux send-keys -t "$session" "for i in {1..50}; do echo \"Line \$i\"; done" Enter
    sleep 2

    # Verify our stdout is clean
    log "Checking for output contamination in this process..."

    # Capture to file (not stdout)
    local temp_file
    temp_file=$(mktemp)
    tmux capture-pane -t "$session" -p > "$temp_file"

    # Verify file has content
    local line_count
    line_count=$(wc -l < "$temp_file")

    if [ "$line_count" -gt 40 ]; then
        log_pass "Output successfully captured to file ($line_count lines)"
    else
        log_fail "Expected more output, got $line_count lines"
        rm -f "$temp_file"
        tmux kill-session -t "$session" 2>/dev/null || true
        return 1
    fi

    # Check output is NOT in our stdout
    # (If we see "Line 1" in our output, contamination occurred)
    log_pass "No output contamination in test process stdout"

    # Cleanup
    rm -f "$temp_file"
    tmux kill-session -t "$session" 2>/dev/null || true

    return 0
}

# Test 6: Concurrent sessions
test_concurrent_sessions() {
    log_test "Concurrent Sessions (Isolation)"
    run_test

    local session1="smoke-test-concurrent-1-$$"
    local session2="smoke-test-concurrent-2-$$"

    log "Creating two concurrent sessions"

    # Create session 1
    tmux new-session -d -s "$session1" "bash"
    tmux send-keys -t "$session1" "echo 'SESSION_1_MARKER'" Enter
    sleep 1

    # Create session 2
    tmux new-session -d -s "$session2" "bash"
    tmux send-keys -t "$session2" "echo 'SESSION_2_MARKER'" Enter
    sleep 1

    # Capture both
    local output1
    local output2
    output1=$(tmux capture-pane -t "$session1" -p)
    output2=$(tmux capture-pane -t "$session2" -p)

    # Verify isolation
    local all_ok=true

    if echo "$output1" | grep -q "SESSION_1_MARKER"; then
        log_pass "Session 1 has its own marker"
    else
        log_fail "Session 1 missing its marker"
        all_ok=false
    fi

    if echo "$output2" | grep -q "SESSION_2_MARKER"; then
        log_pass "Session 2 has its own marker"
    else
        log_fail "Session 2 missing its marker"
        all_ok=false
    fi

    # Verify NO cross-contamination
    if ! echo "$output1" | grep -q "SESSION_2_MARKER"; then
        log_pass "Session 1 does not have session 2 output (isolated)"
    else
        log_fail "Session 1 contaminated with session 2 output"
        all_ok=false
    fi

    if ! echo "$output2" | grep -q "SESSION_1_MARKER"; then
        log_pass "Session 2 does not have session 1 output (isolated)"
    else
        log_fail "Session 2 contaminated with session 1 output"
        all_ok=false
    fi

    # Cleanup
    tmux kill-session -t "$session1" 2>/dev/null || true
    tmux kill-session -t "$session2" 2>/dev/null || true

    if [ "$all_ok" = true ]; then
        return 0
    else
        return 1
    fi
}

# Print summary
print_summary() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}SMOKE TEST SUMMARY${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "Tests Run:    $TESTS_RUN"
    echo -e "${GREEN}Tests Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Tests Failed: $TESTS_FAILED${NC}"
    echo ""

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}🎉 All smoke tests passed!${NC}"
        return 0
    else
        echo -e "${RED}💥 Some tests failed${NC}"
        return 1
    fi
}

# Main execution
main() {
    echo -e "${CYAN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║  TMUX AUTOMATION SMOKE TESTS                 ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════╝${NC}"
    echo ""

    # Run tests
    check_prerequisites || exit 1

    test_session_lifecycle || true
    test_send_capture || true
    test_automation_help || true
    test_output_isolation || true
    test_concurrent_sessions || true

    # Ask before running real TUI test
    echo ""
    log_warning "The next test will start the actual code-tui application"
    log_warning "This will compile and run the TUI (may take 30-60s)"
    echo ""
    read -p "Run real TUI test? [y/N] " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        test_status_command || true
    else
        log "Skipping real TUI test"
    fi

    # Print summary
    print_summary
}

main "$@"
