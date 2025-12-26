# Session Handoff — SPEC-DOGFOOD-001 Dead Code Cleanup

**Last updated:** 2025-12-26
**Status:** Session 20 Complete, Session 21 Ready
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
| S21 | Type migration | TBD | (next) |

**Total deleted (S17-S20):** ~3,150 LOC

---

## Session 20 Summary (Complete)

### Commits
- `8808ebd7b` - fix(tui): Fix test isolation with #[serial] and resolve clippy warnings

### Changes Made
1. **Test isolation** - Added `#[serial]` to 4 env-var-mutating critic tests
2. **Dead code** - Deleted `create_diff_summary` wrapper (~10 LOC)
3. **Clippy fixes** - Resolved 5 warnings across stage0 and tui

### Verification
- `cargo clippy -p codex-tui -p codex-stage0 --all-targets -- -D warnings` ✅
- `cargo test -p codex-tui --lib` ✅ (533 tests passing in parallel)

---

## Session 21 Plan

### 1. Type Alias Migration (Consensus* → Gate*)

**Files:** `codex-rs/tui/src/chatwidget/spec_kit/gate_evaluation.rs`

Rename these types and update all callsites:

| Old Name | New Name |
|----------|----------|
| `ConsensusArtifactData` | `GateArtifactData` |
| `ConsensusEvidenceHandle` | `GateEvidenceHandle` |
| `ConsensusTelemetryPaths` | `GateTelemetryPaths` |
| `ConsensusArtifactVerdict` | `GateArtifactVerdict` |
| `ConsensusVerdict` | `StageReviewVerdict` |
| `ConsensusSynthesisSummary` | `StageReviewSummary` |
| `ConsensusSynthesisRaw` | `StageReviewRaw` |
| `ConsensusSynthesisConsensusRaw` | `StageReviewConsensusRaw` |

**Steps:**
a. Rename struct definitions (remove `Consensus` prefix)
b. Update all callsites in gate_evaluation.rs
c. Update callsites in other files (grep for usage)
d. Remove type aliases (now direct types)
e. Verify: `cargo build -p codex-tui`

### 2. Fix app-server-protocol Compilation Errors

**Files:** `codex-rs/app-server-protocol/src/protocol/`

| Error | File | Fix |
|-------|------|-----|
| Missing `kind` field | thread_history.rs:223,236,306,350,360 | Add `kind: UserMessageKind::default()` |
| Missing `ExternalSandbox` variant | v2.rs:1964 | Add variant or update match |

**Steps:**
a. Read `UserMessageEvent` struct to understand `kind` field
b. Read `SandboxPolicy` enum to understand variants
c. Add missing fields/variants
d. Verify: `cargo clippy --workspace -- -D warnings`

### 3. Audit Module-Level #[allow(dead_code)]

**Target modules** (check if code is actually used):
- `config_validator.rs` - "Validation helpers pending integration"
- `quality_gate_handler.rs` - "Extended QA features pending"
- `ace_reflector.rs` - "ACE reflection pending full integration"

**Steps:**
a. For each module, grep for usage of exported items
b. If items are used, remove the module-level allow
c. If items are truly unused, consider deletion or keep allow with updated comment
d. Document findings

### 4. Verification

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p codex-tui --lib
cargo test -p codex-stage0
```

---

## Success Criteria

- [ ] All Consensus* types renamed to Gate*/StageReview*
- [ ] No type aliases remain (direct types only)
- [ ] app-server-protocol compiles
- [ ] Full workspace clippy passes (0 warnings)
- [ ] All tests pass
- [ ] Commit pushed

---

## Known Issues

### Pre-existing (not blocking)
- `app-server-protocol` compilation errors (will fix in S21)
- Many modules have `#[allow(dead_code)]` for pending features

### Out of Scope
- ACE integration modules (intentionally dead, pending feature work)
- tui2 (upstream scaffold only, per ADR-002)

---

## Key Files

| File | Purpose |
|------|---------|
| `gate_evaluation.rs` | Type definitions, consensus logic |
| `thread_history.rs` | UserMessageEvent construction |
| `v2.rs` | SandboxPolicy handling |
| `config_validator.rs` | Validation helpers (check dead_code) |

---

## Continuation Prompt

```
Continue SPEC-DOGFOOD-001 Dead Code Cleanup - Session 21 **ultrathink**

## Context
Session 20 completed:
- Fixed test isolation with #[serial] for 4 critic tests
- Deleted create_diff_summary dead code (~10 LOC)
- Fixed 5 clippy warnings
- Commit: 8808ebd7b

See HANDOFF.md for full details.

## Session 21 Tasks (in order)

### 1. Type Alias Migration (Consensus* → Gate*)
In gate_evaluation.rs:
a. Rename ConsensusArtifactData → GateArtifactData
b. Rename ConsensusEvidenceHandle → GateEvidenceHandle
c. Rename ConsensusTelemetryPaths → GateTelemetryPaths
d. Rename ConsensusArtifactVerdict → GateArtifactVerdict
e. Rename ConsensusVerdict → StageReviewVerdict
f. Rename ConsensusSynthesisSummary → StageReviewSummary
g. Rename ConsensusSynthesisRaw → StageReviewRaw
h. Rename ConsensusSynthesisConsensusRaw → StageReviewConsensusRaw
i. Update all callsites (grep for old names)
j. Remove now-unused type aliases

### 2. Fix app-server-protocol Errors
a. Add missing `kind` field to UserMessageEvent constructors (5 locations)
b. Fix missing ExternalSandbox variant in SandboxPolicy match

### 3. Audit #[allow(dead_code)]
Check these modules for unused code:
- config_validator.rs
- quality_gate_handler.rs
- ace_reflector.rs
Remove allows if code is used, or document why it's pending.

### 4. Final Verification
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test -p codex-tui --lib
- cargo test -p codex-stage0
- Commit and push

## Success Criteria
- All Consensus* types renamed to Gate*/StageReview*
- No type aliases remain
- Full workspace clippy passes (0 warnings)
- All tests pass
- Commit pushed

## After Session 21
Continue with Session 22: Review remaining dead code opportunities
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
