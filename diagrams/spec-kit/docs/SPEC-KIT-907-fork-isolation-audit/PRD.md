# PRD: SPEC-KIT-907 - Audit Fork Isolation

**Priority**: P0 (Critical)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

The spec-kit fork claims 98.2% isolation from upstream code, but this hasn't been systematically verified. Before attempting upstream sync, we need concrete evidence that fork-specific code is truly isolated to minimize merge conflicts.

**Risk Areas**:
1. **Business logic leakage**: Spec-kit logic may have spread beyond `spec_kit/` module
2. **Touchpoint sprawl**: Integration hooks may be scattered across multiple files
3. **Undocumented dependencies**: Implicit coupling to upstream code not clearly marked
4. **Config field pollution**: Fork-specific config fields may be mixed with upstream fields

Without systematic audit, upstream sync could encounter unexpected conflicts in supposedly "clean" files.

---

## Goals

### Primary Goal
Systematically verify that spec-kit code is isolated to designated touchpoints, identifying any leakage into upstream code paths that would complicate sync.

### Secondary Goals
- Document all fork-specific markers for maintainer reference
- Identify any "gray zones" requiring refactoring before sync
- Provide evidence-based confidence for sync readiness
- Create audit script for future fork isolation verification

---

## Requirements

### Functional Requirements

1. **Automated Audit Script**
   - Create `scripts/fork-isolation-audit.sh`
   - Scan codebase for fork-specific markers
   - Output: Categorized list of touchpoints with file paths and line numbers

2. **Fork Marker Search Patterns**
   - `FORK-SPECIFIC` comments
   - `spec_kit::` imports
   - `SpecKit` type references
   - `spec-kit` crate references
   - Config fields: `agents`, `ace`, `subagent_commands`
   - SlashCommand variants: `SpecKit*`

3. **Expected Touchpoint Whitelist**
   - `tui/src/chatwidget/spec_kit/` - Isolated module (allowed)
   - `tui/src/chatwidget/mod.rs` - Minimal hooks (~10-15 lines, allowed)
   - `tui/src/slash_command.rs` - 13 enum variants (allowed)
   - `tui/src/spec_prompts.rs` - Prompt building (allowed)
   - `core/src/config.rs` - Config struct fields (allowed)
   - `core/src/config_types.rs` - Type definitions (allowed)

4. **Unexpected Touchpoint Detection**
   - Any fork markers outside whitelist → Flag for review
   - Business logic in core → Flag as leakage
   - Scattered integration hooks → Flag for consolidation

5. **Audit Report Output**
   - **Section 1**: Whitelisted touchpoints (expected)
   - **Section 2**: Unexpected touchpoints (require review)
   - **Section 3**: Statistics (% isolation, line count, files touched)
   - **Section 4**: Recommendations (refactor suggestions if needed)

### Non-Functional Requirements

1. **Completeness**
   - Audit must scan entire `codex-rs/` workspace
   - No false negatives (miss actual fork code)
   - Minimal false positives (flag non-fork code)

2. **Repeatability**
   - Script can be run at any time for verification
   - Deterministic output (same input → same output)
   - Fast execution (<10 seconds)

---

## Technical Approach

### Audit Script Implementation

```bash
#!/usr/bin/env bash
# scripts/fork-isolation-audit.sh

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$WORKSPACE_ROOT/codex-rs"

echo "=== Fork Isolation Audit ==="
echo "Scanning for fork-specific markers..."
echo

# Whitelist (expected touchpoints)
WHITELIST=(
    "tui/src/chatwidget/spec_kit/"
    "tui/src/chatwidget/mod.rs"
    "tui/src/slash_command.rs"
    "tui/src/spec_prompts.rs"
    "core/src/config.rs"
    "core/src/config_types.rs"
)

# Search patterns
PATTERNS=(
    "FORK-SPECIFIC"
    "spec_kit::"
    "SpecKit"
    "spec-kit"
    "SpecAutoState"
    "agents:"
    "ace:"
    "subagent_commands"
)

# Output files
EXPECTED_OUTPUT=$(mktemp)
UNEXPECTED_OUTPUT=$(mktemp)

# Scan for each pattern
for pattern in "${PATTERNS[@]}"; do
    rg "$pattern" --type rust --with-filename --line-number . 2>/dev/null | while read -r line; do
        file=$(echo "$line" | cut -d: -f1)

        # Check if file is whitelisted
        is_whitelisted=false
        for whitelist_path in "${WHITELIST[@]}"; do
            if [[ "$file" == "$whitelist_path"* ]]; then
                is_whitelisted=true
                break
            fi
        done

        if [ "$is_whitelisted" = true ]; then
            echo "$line" >> "$EXPECTED_OUTPUT"
        else
            echo "$line" >> "$UNEXPECTED_OUTPUT"
        fi
    done
done

# Generate report
echo "## Section 1: Expected Touchpoints (Whitelisted)"
echo "These are known fork integration points:"
echo
sort -u "$EXPECTED_OUTPUT" | head -20
echo
echo "... ($(wc -l < "$EXPECTED_OUTPUT") total matches)"
echo

echo "## Section 2: Unexpected Touchpoints (Requires Review)"
if [ -s "$UNEXPECTED_OUTPUT" ]; then
    echo "⚠️  Fork code found outside whitelisted locations:"
    echo
    sort -u "$UNEXPECTED_OUTPUT"
    echo
    echo "❌ ISOLATION BREACH: Fork code leaked into upstream paths"
    exit 1
else
    echo "✅ No unexpected fork markers found"
fi
echo

echo "## Section 3: Statistics"
expected_count=$(wc -l < "$EXPECTED_OUTPUT" || echo 0)
unexpected_count=$(wc -l < "$UNEXPECTED_OUTPUT" || echo 0)
total_count=$((expected_count + unexpected_count))

if [ "$total_count" -gt 0 ]; then
    isolation_pct=$(awk "BEGIN {printf \"%.1f\", ($expected_count / $total_count) * 100}")
else
    isolation_pct="100.0"
fi

echo "Total fork markers: $total_count"
echo "Whitelisted: $expected_count"
echo "Unexpected: $unexpected_count"
echo "Isolation: ${isolation_pct}% (target: 98%+)"
echo

if [ "$unexpected_count" -eq 0 ]; then
    echo "## Section 4: Recommendations"
    echo "✅ Fork isolation verified. Safe to proceed with upstream sync."
    exit 0
else
    echo "## Section 4: Recommendations"
    echo "❌ Refactor unexpected touchpoints before upstream sync:"
    sort -u "$UNEXPECTED_OUTPUT" | cut -d: -f1 | sort -u | while read -r file; do
        echo "   - Review $file"
    done
    exit 1
fi

# Cleanup
rm -f "$EXPECTED_OUTPUT" "$UNEXPECTED_OUTPUT"
```

### Manual Audit Checklist

**In addition to script, manually verify**:

1. **ChatWidget Integration** (`tui/src/chatwidget/mod.rs`):
   - Count fork-specific lines (target: ≤15 lines)
   - Verify all hooks are labeled with `// FORK-SPECIFIC` comments
   - Check no business logic, only delegation to `spec_kit/` module

2. **Config Struct** (`core/src/config.rs`):
   - Verify fork fields grouped together (not scattered)
   - Check all fork fields use `#[serde(default)]` for backward compat
   - Ensure no fork-specific validation logic in core config loading

3. **Protocol Layer** (`codex-protocol/`):
   - Verify zero fork-specific changes (pure upstream)
   - Check no custom event types added (only use existing events)

4. **MCP Integration** (`mcp-client/`, `core/src/mcp_connection_manager.rs`):
   - Verify native optimization is modular (can be disabled)
   - Check no spec-kit-specific logic in MCP client
   - Ensure fallback behavior is transparent

---

## Acceptance Criteria

- [ ] Fork isolation audit script created (`scripts/fork-isolation-audit.sh`)
- [ ] Script scans all fork marker patterns (8+ patterns)
- [ ] Whitelist of expected touchpoints defined (6 locations)
- [ ] Script outputs 4-section report (expected, unexpected, stats, recommendations)
- [ ] Audit run on current codebase shows 98%+ isolation
- [ ] Zero unexpected touchpoints found (or documented if found)
- [ ] Manual audit checklist completed for critical integration points
- [ ] Audit results documented in `docs/fork-isolation-audit-report.md`
- [ ] Script added to pre-sync checklist in `docs/UPSTREAM-SYNC.md`
- [ ] CI integration (optional): Audit runs on PRs to detect leakage

---

## Out of Scope

- **Refactoring work**: This SPEC only audits, doesn't fix leakage if found
- **Upstream comparison**: Not comparing against upstream codebase, only checking isolation
- **Performance analysis**: Focus is structural isolation, not performance

---

## Success Metrics

1. **Isolation Verification**: 98%+ of fork code in whitelisted locations
2. **Audit Speed**: Script completes in <10 seconds
3. **Confidence**: Zero surprises during upstream sync (all touchpoints known)
4. **Repeatability**: Script usable for future fork isolation checks

---

## Dependencies

### Prerequisites
- `ripgrep` installed (for fast searching)
- Bash shell available

### Downstream Dependencies
- Upstream sync (SYNC-001) depends on this audit
- Pre-sync refactor may require fixes if leakage found

---

## Estimated Effort

**1 hour** (as per architecture review)

**Breakdown**:
- Audit script creation: 30 min
- Run audit and analyze results: 15 min
- Manual checklist verification: 10 min
- Document results: 5 min

---

## Priority

**P0 (Critical)** - Must complete before upstream sync. First step in pre-sync refactor plan. Low effort, high value for sync confidence.

---

## Related Documents

- Architecture Review: Section "Pre-Sync Refactor, Step 1"
- Upstream Sync Readiness: "Conflict Zones" section
- `docs/UPSTREAM-SYNC.md` - Sync process documentation
- Future: `docs/fork-isolation-audit-report.md` (created by this SPEC)
