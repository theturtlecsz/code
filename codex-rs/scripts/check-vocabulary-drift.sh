#!/usr/bin/env bash
# check-vocabulary-drift.sh - CI check for "consensus" vocabulary migration
#
# PURPOSE: Prevent NEW code locations from introducing "consensus" vocabulary.
# The term is deeply embedded in existing code and will migrate gradually.
# This script acts as a canary for drift into new directories/crates.
#
# CURRENT STATE (PR7): "consensus" vocabulary exists throughout the codebase.
# Full migration to "gate/review" vocabulary is a future multi-PR effort.
# This script allows existing code but blocks new introductions.
#
# Usage: ./scripts/check-vocabulary-drift.sh
# Exit: 0 = pass, 1 = violations found (new location introducing "consensus")

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Allowlist patterns (grep -E extended regex)
#
# STRATEGY: Allow existing legacy code, block NEW user-facing uses.
# The "consensus" vocabulary is deeply embedded in the codebase.
# This script prevents NEW drift, not immediate forced migration.
#
# Categories:
# 1. Wire types / DB schema - intentionally preserved for compatibility
# 2. Existing implementation code - will migrate gradually
# 3. Documentation - historical references are OK
# 4. Tests - existing fixtures are OK
#
ALLOWLIST=(
    # === WIRE TYPES & DB (intentionally preserved) ===
    "tui/src/chatwidget/spec_kit/gate_evaluation.rs"
    "tui/src/chatwidget/spec_kit/consensus_db.rs"
    "tui/src/chatwidget/spec_kit/consensus_coordinator.rs"
    "core/src/db/"
    "migrations/"
    "schema\\.sql"
    "benches/"

    # === EXISTING IMPLEMENTATION (gradual migration) ===
    # These directories contain deeply embedded "consensus" vocabulary.
    # Full migration requires significant refactoring; allow for now.
    "tui/src/"
    "spec-kit/src/"
    "stage0/src/"
    "core/src/"
    "mcp-server/src/"
    "cli/src/"

    # === DOCUMENTATION (historical references OK) ===
    "docs/"
    "HANDOFF.md"
    "REVIEW.md"
    "MEMORY-POLICY.md"
    "\\.code/"
    "\\.serena/"
    "CHANGELOG"
    "\\.md$"

    # === TESTS (existing fixtures OK) ===
    "_test\\.rs$"
    "tests/"

    # === META ===
    "scripts/check-vocabulary-drift.sh"
    "\\.git/"
)

echo "Checking for 'consensus' vocabulary drift..."
echo "Repo root: $REPO_ROOT"
echo ""

# Build grep exclusion pattern
EXCLUDE_PATTERN=""
for pattern in "${ALLOWLIST[@]}"; do
    if [[ -n "$EXCLUDE_PATTERN" ]]; then
        EXCLUDE_PATTERN="$EXCLUDE_PATTERN|$pattern"
    else
        EXCLUDE_PATTERN="$pattern"
    fi
done

# Find violations: case-insensitive search for "consensus" in Rust/Markdown files
# excluding allowlisted paths
cd "$REPO_ROOT"

VIOLATIONS=$(grep -rniE "consensus" \
    --include="*.rs" \
    --include="*.md" \
    --include="*.toml" \
    . 2>/dev/null | \
    grep -vE "$EXCLUDE_PATTERN" | \
    grep -vE "^Binary file" || true)

if [[ -n "$VIOLATIONS" ]]; then
    echo "FAILED: Found 'consensus' vocabulary in non-allowlisted locations:"
    echo ""
    echo "$VIOLATIONS"
    echo ""
    echo "If these are intentional, add the file path to the allowlist in:"
    echo "  scripts/check-vocabulary-drift.sh"
    echo ""
    echo "Migration guide:"
    echo "  - User-facing: use 'review' or 'gate' instead"
    echo "  - Wire types: keep in gate_evaluation.rs with serde aliases"
    echo "  - Config: use min_confidence_for_auto_apply (not consensus_threshold)"
    exit 1
else
    echo "PASSED: No vocabulary drift detected"
    echo ""
    echo "Allowlisted locations (where 'consensus' is permitted):"
    for pattern in "${ALLOWLIST[@]}"; do
        echo "  - $pattern"
    done
    exit 0
fi
