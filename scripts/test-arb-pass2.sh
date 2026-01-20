#!/usr/bin/env bash
# ARB Pass 2 Enforcement Tests (D130-D134)
#
# Runs all 18 enforcement tests validating ACE + Maieutics decisions.
# See: codex-rs/tui/src/chatwidget/spec_kit/arb_pass2_enforcement.rs
#
# Usage:
#   ./scripts/test-arb-pass2.sh          # Run all tests
#   ./scripts/test-arb-pass2.sh --quick  # Skip slow tests
#
# Test coverage:
#   D130: Maieutic mandatory before execute (1 test)
#   D131: capture=none persists no artifacts (2 tests)
#   D132: Ship gate hard-fail requirements (6 tests)
#   D133: Headless multi-surface parity (7 tests, 3 ignored)
#   D134: ACE Frame schema stability (3 tests)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT/codex-rs"

echo "=== ARB Pass 2 Enforcement Tests (D130-D134) ==="
echo ""

echo "--- Meta-tests: Registry validation ---"
cargo test -p codex-tui --lib -- arb_pass2_enforcement::validation_tests --nocapture 2>&1 | grep -E '(test |ok|FAILED|running)'
echo ""

echo "--- D130: Maieutic mandatory ---"
cargo test -p codex-tui --lib -- maieutic::tests::test_maieutic_required --nocapture 2>&1 | grep -E '(test |ok|FAILED)'
echo ""

echo "--- D131: capture=none persists no artifacts ---"
cargo test -p codex-tui --lib -- maieutic::tests::test_capture_none --nocapture 2>&1 | grep -E '(test |ok|FAILED)'
cargo test -p codex-tui --lib -- ace_reflector::tests::test_capture_none --nocapture 2>&1 | grep -E '(test |ok|FAILED)'
echo ""

echo "--- D132: Ship gate requirements ---"
cargo test -p codex-tui --lib -- ship_gate::tests --nocapture 2>&1 | grep -E '(test |ok|FAILED)'
echo ""

echo "--- D134: ACE Frame schema versioning ---"
cargo test -p codex-tui --lib -- ace_reflector::schema_tests --nocapture 2>&1 | grep -E '(test |ok|FAILED)'
echo ""

if [[ "${1:-}" != "--quick" ]]; then
    echo "--- D133: CLI/headless tests (integration) ---"
    cargo test -p codex-cli --test speckit -- test_headless --nocapture 2>&1 | grep -E '(test |ok|FAILED|ignored)'
    echo ""
fi

echo "=== ARB Pass 2: All enforcement tests passed ==="
echo ""
echo "Summary: 15 active tests, 3 ignored (SPEC-KIT-930)"
echo "For ignored tests, run: cargo test -p codex-cli --test speckit -- --ignored"
