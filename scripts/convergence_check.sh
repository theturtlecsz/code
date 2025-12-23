#!/usr/bin/env bash
# Convergence Check Script
#
# CONVERGENCE: Validates that codex-rs adheres to convergence guardrails.
# Run in CI or pre-commit to catch policy violations early.
#
# Exit codes:
#   0 - All checks pass
#   1 - Violations found
#
# Usage: ./scripts/convergence_check.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

VIOLATIONS=0

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    VIOLATIONS=$((VIOLATIONS + 1))
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

echo "=== Convergence Check ==="
echo ""

# -----------------------------------------------------------------------------
# Check 1: No implicit notebook fallback behavior
# -----------------------------------------------------------------------------
echo "Checking: No implicit notebook fallback..."

# Search for patterns that suggest "general" or "default" notebook fallback
# Excludes: test files, doc comments (//!), regular comments (//)
FALLBACK_PATTERNS='general_notebook|default_notebook|fallback_notebook|use.*general.*notebook|create.*general.*notebook'
FALLBACK_MATCHES=$(grep -rE --include="*.rs" "$FALLBACK_PATTERNS" "${REPO_ROOT}/codex-rs" 2>/dev/null | grep -v "_test\.rs\|tests/" | grep -v "//!" | grep -v "// " || true)
if [ -n "$FALLBACK_MATCHES" ]; then
    log_fail "Found potential implicit notebook fallback patterns"
    echo "       Files with fallback patterns should not silently create or use a 'general' notebook"
    echo "$FALLBACK_MATCHES" | head -5 | sed 's/^/       /'
else
    log_pass "No implicit notebook fallback patterns found"
fi

# -----------------------------------------------------------------------------
# Check 2: Stage0 Tier2 requires explicit notebook mapping
# -----------------------------------------------------------------------------
echo "Checking: Tier2 requires explicit notebook mapping..."

# Look for Tier2 calls that don't check notebook configuration
TIER2_UNCHECKED=$(grep -rn --include="*.rs" "generate_divine_truth\|Tier2Client" "${REPO_ROOT}/codex-rs" 2>/dev/null | grep -v "test" | grep -v "trait" | grep -v "impl.*for" || true)
if echo "$TIER2_UNCHECKED" | grep -v "notebook\|NotebookLM\|noop\|mock" | grep -v "^$" > /dev/null 2>&1; then
    log_warn "Tier2 calls found - verify they check notebook configuration first"
else
    log_pass "Tier2 calls appear properly guarded"
fi

# -----------------------------------------------------------------------------
# Check 3: System pointer memories have required domain/tag schema
# -----------------------------------------------------------------------------
echo "Checking: System pointer memories have required schema..."

# Check that system pointer storage uses correct domain and tags
POINTER_IMPL="${REPO_ROOT}/codex-rs/stage0/src/system_memory.rs"
if [ -f "$POINTER_IMPL" ]; then
    # Check for spec-tracker domain
    if grep -q 'domain.*spec-tracker\|"spec-tracker"' "$POINTER_IMPL"; then
        log_pass "System pointer uses spec-tracker domain"
    else
        log_fail "System pointer does not use spec-tracker domain"
    fi

    # Check for system:true tag
    if grep -q 'system:true' "$POINTER_IMPL"; then
        log_pass "System pointer includes system:true tag"
    else
        log_fail "System pointer missing system:true tag"
    fi
else
    log_fail "System memory implementation not found at $POINTER_IMPL"
fi

# -----------------------------------------------------------------------------
# Check 4: Documentation uses 'code' not 'codex' for CLI commands
# -----------------------------------------------------------------------------
echo "Checking: Documentation uses 'code' not 'codex' for CLI..."

# Search for 'codex' command usage in markdown docs
CODEX_USAGE=$(grep -rn --include="*.md" '\bcodex\b' "${REPO_ROOT}/docs" 2>/dev/null | grep -v "codex-rs\|codex-core\|codex-tui\|codex-cli\|codex-exec\|codex_\|codex-stage0\|codex-common\|codex-protocol\|codex-git" | head -10 || true)
if [ -n "$CODEX_USAGE" ]; then
    log_warn "Found 'codex' in docs (should use 'code' for CLI commands):"
    echo "$CODEX_USAGE" | head -5 | sed 's/^/       /'
else
    log_pass "Documentation correctly uses 'code' for CLI"
fi

# -----------------------------------------------------------------------------
# Check 5: Convergence docs pointer exists
# -----------------------------------------------------------------------------
echo "Checking: Convergence docs pointer exists..."

if [ -f "${REPO_ROOT}/docs/convergence/README.md" ]; then
    # Check it contains the "do not fork" warning
    if grep -q "Do not fork" "${REPO_ROOT}/docs/convergence/README.md"; then
        log_pass "Convergence docs pointer exists with correct warning"
    else
        log_warn "Convergence docs pointer exists but missing 'Do not fork' warning"
    fi
else
    log_fail "Convergence docs pointer not found at docs/convergence/README.md"
fi

# -----------------------------------------------------------------------------
# Check 6: Stage0 doctor command exists
# -----------------------------------------------------------------------------
echo "Checking: Stage0 doctor command exists..."

if [ -f "${REPO_ROOT}/codex-rs/cli/src/stage0_cmd.rs" ]; then
    if grep -q "doctor\|Doctor" "${REPO_ROOT}/codex-rs/cli/src/stage0_cmd.rs"; then
        log_pass "Stage0 doctor command implemented"
    else
        log_fail "Stage0 doctor command not found in stage0_cmd.rs"
    fi
else
    log_fail "stage0_cmd.rs not found"
fi

# -----------------------------------------------------------------------------
# Check 7: Convergence acceptance tests exist
# -----------------------------------------------------------------------------
echo "Checking: Convergence acceptance tests exist..."

if [ -f "${REPO_ROOT}/codex-rs/stage0/tests/convergence_acceptance.rs" ]; then
    TEST_COUNT=$(grep -c "#\[test\]\|#\[tokio::test\]" "${REPO_ROOT}/codex-rs/stage0/tests/convergence_acceptance.rs" 2>/dev/null || echo "0")
    if [ "$TEST_COUNT" -ge 3 ]; then
        log_pass "Convergence acceptance tests exist (${TEST_COUNT} tests)"
    else
        log_warn "Only ${TEST_COUNT} convergence tests found (expected >= 3)"
    fi
else
    log_fail "Convergence acceptance tests not found"
fi

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "=== Summary ==="
if [ $VIOLATIONS -eq 0 ]; then
    echo -e "${GREEN}All convergence checks passed${NC}"
    exit 0
else
    echo -e "${RED}${VIOLATIONS} violation(s) found${NC}"
    echo "Fix the issues above before merging"
    exit 1
fi
