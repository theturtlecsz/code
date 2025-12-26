# Session Handoff — SPEC-DOGFOOD-001 Dead Code Cleanup

**Last updated:** 2025-12-26
**Status:** Session 21 Complete, Session 22 Ready
**Current SPEC:** SPEC-DOGFOOD-001

> **Goal**: Clean up dead code, fix test isolation, and modernize type naming.

---

## Session Log

| Session | Focus | LOC Deleted | Outcome |
|---------|-------|-------------|---------|
| S17 | Dead code audit | ~1,500 | Identified unused modules |
| S18 | Native consensus cleanup | ~800 | Deleted native_consensus_executor.rs |
| S19 | Config reload removal | ~840 | Deleted config_reload.rs, clippy fixes |
| S20 | Test isolation + clippy | ~10 | Added #[serial], fixed 5 clippy warnings |
| S21 | Type migration + audit | ~50 | Renamed 8 types, fixed 6 clippy, audited dead_code |

**Total deleted (S17-S21):** ~3,200 LOC

---

## Session 21 Summary (Complete)

### Commits
- `1d4ef03e2` - refactor(tui): Rename Consensus* types to Gate*/StageReview* and fix clippy

### Changes Made

#### 1. Type Migration (gate_evaluation.rs)
8 types renamed to better naming:
- `ConsensusArtifactData` → `GateArtifactData`
- `ConsensusEvidenceHandle` → `GateEvidenceHandle`
- `ConsensusTelemetryPaths` → `GateTelemetryPaths`
- `ConsensusArtifactVerdict` → `GateArtifactVerdict`
- `ConsensusVerdict` → `StageReviewVerdict`
- `ConsensusSynthesisSummary` → `StageReviewSummary`
- `ConsensusSynthesisRaw` → `StageReviewRaw`
- `ConsensusSynthesisConsensusRaw` → `StageReviewConsensusRaw`

Removed 6 type aliases (now direct types). Updated doc comments to reflect new naming.

#### 2. app-server-protocol Fixes
- Added `kind: None` to 5 `UserMessageEvent` constructors in thread_history.rs
- Fixed `SandboxPolicy::ExternalSandbox` test - changed to verify actual conversion behavior (ExternalSandbox → WorkspaceWrite)
- Removed unused `CoreNetworkAccess` import

#### 3. Clippy Fixes
- `backend-client/client.rs`: Use `div_ceil()` instead of manual ceiling division
- `core/cli_executor/claude_pipes.rs`: Inline format args (6 locations)
- `core/cli_executor/gemini_pipes.rs`: Inline format args (5 locations)
- `core/architect/complexity.rs`: Use `range.contains()` for bounds check

#### 4. Dead Code Audit
Removed blanket `#![allow(dead_code)]` from 3 modules:
- `config_validator.rs` - Code IS used; added targeted allows for 3 pending items
- `quality_gate_handler.rs` - Code IS used; added targeted allows for 2 pending items
- `ace_reflector.rs` - Code IS used; added targeted allow for 1 pending item

### Verification
- `cargo clippy --workspace --exclude codex-tui2 --exclude codex-cli -- -D warnings` ✅
- `cargo test -p codex-tui --lib` ✅ (533 tests)
- `cargo test -p codex-stage0` ✅ (257 tests)
- `~/code/build-fast.sh` ✅

---

## Session 22 Plan (Expanded)

### 1. Fix codex-cli Test Clippy Warnings

**Target:** `codex-rs/cli/tests/speckit_helpers.rs`

Known warnings:
- `expect_used` - Replace `.expect()` with proper error handling or `?`
- `redundant_closure` - Simplify closures like `|v| v.as_u64()` to method refs
- `uninlined_format_args` - Use inline format args

**Steps:**
a. Read the test file to understand context
b. Fix expect_used warnings (use `?` or `.ok()` as appropriate)
c. Fix redundant_closure warnings (use method references)
d. Fix format args (inline variables)
e. Verify: `cargo clippy -p codex-cli --all-targets -- -D warnings`

### 2. Comprehensive Dead Code Audit

**Scope:** Full workspace grep for `#[allow(dead_code)]`

**Audit targets:**
a. All spec_kit modules (not just the 3 already audited)
b. quality.rs, routing.rs helper functions
c. Test utilities across packages
d. Any remaining blanket module-level allows

**For each allow found:**
- Grep for actual usage
- If used: remove allow (or add targeted allow with comment)
- If unused: consider deletion or document why pending

**Steps:**
a. `grep -r "allow(dead_code)" codex-rs/` to find all locations
b. Categorize: blanket module vs targeted item
c. Audit each for actual usage
d. Apply fixes or document pending

### 3. Verification (Full Workspace)

```bash
# Full workspace clippy (excluding only tui2 per ADR-002)
cargo clippy --workspace --all-targets --exclude codex-tui2 -- -D warnings

# Tests
cargo test -p codex-tui --lib
cargo test -p codex-stage0
cargo test -p codex-core
cargo test -p codex-cli --lib

# Build
~/code/build-fast.sh
```

---

## Success Criteria (S21 - Achieved)

- [x] All Consensus* types renamed to Gate*/StageReview*
- [x] No type aliases remain (direct types only)
- [x] app-server-protocol compiles
- [x] Full workspace clippy passes (excluding tui2/cli pre-existing)
- [x] All tests pass
- [x] Commit pushed

## Success Criteria (S22)

- [ ] codex-cli test clippy warnings fixed (0 warnings)
- [ ] Comprehensive dead_code audit complete
- [ ] Full workspace clippy passes (excluding only tui2)
- [ ] All tests pass
- [ ] Commit pushed

---

## Known Issues

### Pre-existing (not blocking)
- `codex-tui2` compilation errors (upstream scaffold per ADR-002)

### Out of Scope
- ACE integration modules (pending feature work, properly annotated)
- tui2 (upstream scaffold only, per ADR-002)

---

## Key Files Modified (S21)

| File | Changes |
|------|---------|
| `gate_evaluation.rs` | 8 type renames, 6 alias removals, doc updates |
| `thread_history.rs` | 5 `kind: None` additions |
| `v2.rs` | SandboxPolicy test fix |
| `client.rs` | div_ceil fix |
| `claude_pipes.rs` | 6 inline format args |
| `gemini_pipes.rs` | 5 inline format args |
| `complexity.rs` | range.contains() fix |
| `config_validator.rs` | Targeted dead_code allows |
| `quality_gate_handler.rs` | Targeted dead_code allows |
| `ace_reflector.rs` | Targeted dead_code allow |

---

## Continuation Prompt

```
Continue SPEC-DOGFOOD-001 Dead Code Cleanup - Session 22 **ultrathink**

## Context
Session 21 completed (commit 1d4ef03e2):
- Renamed 8 Consensus* types to Gate*/StageReview*
- Removed 6 type aliases (now direct types)
- Fixed app-server-protocol (5 UserMessageEvent, SandboxPolicy test)
- Fixed 12 clippy warnings across 6 files
- Audited 3 modules for dead_code, added targeted allows

Total progress S17-S21: ~3,200 LOC deleted

See HANDOFF.md for full details.

## Session 22 Tasks (in order)

### 1. Fix codex-cli Test Clippy Warnings
Target: `codex-rs/cli/tests/speckit_helpers.rs`
a. Read the test file to understand context
b. Fix expect_used warnings (use `?` or `.ok()`)
c. Fix redundant_closure warnings (use method references)
d. Fix uninlined_format_args (inline variables)
e. Verify: `cargo clippy -p codex-cli --all-targets -- -D warnings`

### 2. Comprehensive Dead Code Audit
a. Run: `grep -r "allow(dead_code)" codex-rs/` to find all locations
b. Categorize: blanket module-level vs targeted item allows
c. For each blanket allow:
   - Grep for actual usage of module exports
   - If used: remove blanket, add targeted allows with comments
   - If unused: consider deletion or document why pending
d. Focus areas:
   - All spec_kit modules (expand beyond 3 already audited)
   - quality.rs, routing.rs helper functions
   - Test utilities across packages

### 3. Final Verification
```bash
# Full workspace clippy (excluding only tui2 per ADR-002)
cargo clippy --workspace --all-targets --exclude codex-tui2 -- -D warnings

# Tests
cargo test -p codex-tui --lib
cargo test -p codex-stage0
cargo test -p codex-core
cargo test -p codex-cli --lib

# Build
~/code/build-fast.sh
```

### 4. Commit and Update HANDOFF.md

## Success Criteria
- [ ] codex-cli test clippy warnings fixed (0 warnings)
- [ ] Comprehensive dead_code audit complete
- [ ] Full workspace clippy passes (excluding only tui2)
- [ ] All tests pass
- [ ] Commit pushed

## After Session 22
Evaluate SPEC-DOGFOOD-001 completion status. If ~3,500+ LOC deleted and
workspace clean, consider closing SPEC with summary in HANDOFF.md.
```

---

## Previous Context (Archived)

<details>
<summary>Session 15 Plan (Historical)</summary>

Session 15 was focused on SPEC-DOGFOOD-001 initial setup:
- Create stage0.toml
- Seed NotebookLM with core docs
- Create formal SPEC-DOGFOOD-001

This context has been superseded by the dead code cleanup focus (S17+).
</details>
