#!/bin/bash
# SPEC-P0-TUI2-QUARANTINE: Prevent spec-kit drift into tui2
#
# Checks that tui2 source does not contain forbidden spec-kit imports.
# Run from repo root. Used by pre-commit hook and CI quality-gates.
#
# See: docs/adr/ADR-002-tui2-purpose-and-future.md
# See: docs/SPEC-P0-TUI2-QUARANTINE/spec.md

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

TUI2_SRC="${REPO_ROOT}/codex-rs/tui2/src"
TUI2_CARGO="${REPO_ROOT}/codex-rs/tui2/Cargo.toml"

if [ ! -d "$TUI2_SRC" ]; then
    echo "tui2 quarantine: codex-rs/tui2/src not found, skipping"
    exit 0
fi

VIOLATIONS=0

# Forbidden patterns in source files (excluding main.rs allowlist)
# main.rs contains ADR-002 informational warnings that reference spec-kit/speckit
# Those are safe: they tell users to use tui instead.
FORBIDDEN_PATTERNS=(
    "spec_kit::"           # Module-level use of spec_kit
    "codex_spec_kit"       # Crate import
    "chatwidget/spec_kit"  # Path-based import
    "codex_stage0"         # Stage0 crate import
)

for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
    # Search all .rs files except main.rs (which has allowlisted warnings)
    MATCHES=$(grep -rn --include="*.rs" "$pattern" "$TUI2_SRC" \
        | grep -v "tui2/src/main.rs" || true)
    if [ -n "$MATCHES" ]; then
        echo "QUARANTINE VIOLATION: Found '$pattern' in tui2 source:"
        echo "$MATCHES"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi
done

# Check Cargo.toml for forbidden direct dependencies
CARGO_FORBIDDEN=(
    "codex-spec-kit"
    "codex-stage0"
)

for pattern in "${CARGO_FORBIDDEN[@]}"; do
    # Check dependencies section (not comments)
    MATCHES=$(grep -n "^${pattern}\b\|^${pattern} " "$TUI2_CARGO" 2>/dev/null || true)
    if [ -n "$MATCHES" ]; then
        echo "QUARANTINE VIOLATION: Found '$pattern' dependency in tui2/Cargo.toml:"
        echo "$MATCHES"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi
done

# Check for /speckit. slash command implementations (not just references)
# Allowlist: main.rs warning messages are informational only
SPECKIT_IMPL=$(grep -rn --include="*.rs" '/speckit\.' "$TUI2_SRC" \
    | grep -v "tui2/src/main.rs" || true)
if [ -n "$SPECKIT_IMPL" ]; then
    echo "QUARANTINE VIOLATION: Found '/speckit.' slash command reference in tui2 source:"
    echo "$SPECKIT_IMPL"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

if [ "$VIOLATIONS" -gt 0 ]; then
    echo ""
    echo "tui2 quarantine check FAILED ($VIOLATIONS violation(s))"
    echo "tui2 must not contain spec-kit integration. See ADR-002."
    echo "If shared logic is needed, extract into a core crate first."
    exit 1
fi

echo "tui2 quarantine check passed"
exit 0
