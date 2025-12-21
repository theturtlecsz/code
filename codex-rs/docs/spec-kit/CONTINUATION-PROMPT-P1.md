# Continuation Prompt: SPEC-KIT-921 P1 Phase

**Date Created:** 2025-12-21
**Previous Commits:** `76e62675f` (P0 fixes), `81c974e58` (CI tests), `a779a8df6` (CLI MVP)
**Status:** Ready for P1 implementation

---

## Context Summary

The SPEC-KIT-921 Phase B extraction is complete through P0. The review command now:
- Reads from canonical evidence topology (`evidence/consensus/<SPEC-ID>/spec-<stage>_*.json`)
- Uses `review_evidence_count` for skip logic (not total artifacts)
- Has repo-relative paths in all outputs
- Passes policy via `ExecutionContext` (no env reads in executor)
- Has 12 CLI integration tests + 183 spec-kit unit tests passing

## P1 Scope (Architect Approved)

### P1-A: TUI Parity Lock (CRITICAL)
**Goal:** Route TUI `/spec-review` through executor, eliminate dual-truth risk.

**Current State:**
- `handle_spec_review()` in `tui/src/chatwidget/spec_kit/command_handlers.rs` already calls `SpeckitExecutor::execute()`
- Need to verify `/spec-consensus` is deprecated alias routing to same path
- Need to ensure no legacy review/gate logic remains callable outside executor

**Acceptance Criteria:**
- `/spec-review` is purely: parse → call executor → render
- `/spec-consensus` routes to same command (deprecated alias)
- No legacy review logic callable from TUI except via executor

### P1-B: CI Smoke Packet (SPEC-CI-001)
**Goal:** Create deterministic test spec that validates evidence topology + parsing + exit codes.

**Create:** `docs/SPEC-CI-001/` with:
- `spec.md` - Minimal spec definition
- `plan.md` - Required for plan stage artifacts

**Create:** `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-CI-001/` with:
1. `spec-plan_clean_20251201.json` - No conflicts (AutoApply case)
2. `spec-plan_conflict_20251202.json` - Has conflicts (Escalate case)
3. `spec-plan_malformed_20251203.json` - Invalid JSON (parse failure case)
4. `spec-tasks_clean_20251201.json` - Clean tasks (multi-stage coverage)

**CI Tests to Add:**
```bash
# Clean case → exit 0
code speckit review --spec SPEC-CI-001 --stage plan --json

# Missing evidence case → exit 0 (or 2 with --strict-artifacts)
code speckit review --spec SPEC-CI-001 --stage validate --strict-artifacts

# Parse failure + --strict-schema → exit 3
code speckit review --spec SPEC-CI-001 --stage plan --strict-schema
```

### P1-C: Add `--strict-schema` Flag
**Goal:** Prevent CI passing on corrupted evidence.

**Behavior:**
- Default (no flag): parse failures → advisory System signal, non-blocking
- With `--strict-schema`: parse failures → exit 3 (infra error, not human-approvable)

**Implementation:**
1. Add `--strict-schema` to `ReviewArgs` in `cli/src/speckit_cmd.rs`
2. Add `strict_schema: bool` to `ReviewOptions`
3. In `collect_signals_from_consensus()`:
   - If `strict_schema` and parse error → return `Err` (not advisory signal)
4. In CLI handler: catch error → exit 3

### P1-D: Evidence Root Config + Override
**Goal:** Make evidence root configurable (not hardcoded to SPEC-OPS-004).

**Implementation:**
1. Add config key: `spec_kit.evidence_root` (defaults to current hardcoded path)
2. Add CLI flag: `--evidence-root <PATH>`
3. CLI flag takes precedence over config
4. Pass resolved root via `ExecutionContext`

**Files to Update:**
- `spec-kit/src/executor/review.rs` - Remove hardcoded `EVIDENCE_ROOT`
- `spec-kit/src/executor/mod.rs` - Add `evidence_root` to `ExecutionContext`
- `cli/src/speckit_cmd.rs` - Add `--evidence-root` flag
- Config loader (if exists) or add to spec-kit config

---

## Implementation Order

1. **P1-A** (TUI parity) - Quick audit, likely already done
2. **P1-C** (`--strict-schema`) - Enables P1-B testing
3. **P1-B** (SPEC-CI-001) - Creates contract lock
4. **P1-D** (evidence root config) - Enables multi-project usage

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `cli/src/speckit_cmd.rs` | CLI adapter, add flags here |
| `cli/tests/speckit.rs` | CLI integration tests (12 tests) |
| `spec-kit/src/executor/mod.rs` | ExecutionContext, SpeckitExecutor |
| `spec-kit/src/executor/review.rs` | Review logic, signal collection |
| `tui/src/chatwidget/spec_kit/command_handlers.rs` | TUI adapter |
| `docs/spec-kit/REVIEW-CONTRACT.md` | Canonical behavior spec |

---

## Don't Regress Checklist

1. **Core never reads env/config** - all via ExecutionContext
2. **Skip based on review_evidence_count** - not artifacts_collected
3. **All paths repo-relative** - in signals and evidence refs
4. **Deterministic file selection** - lexicographic sort
5. **TUI and CLI same executor path** - no dual truth

---

## Commands for Next Session

```bash
# Build
cd /home/thetu/code/codex-rs
cargo build -p codex-spec-kit -p codex-cli -p codex-tui

# Test
cargo test -p codex-spec-kit
cargo test -p codex-cli --test speckit

# Manual verification
./target/debug/code speckit review --spec SPEC-KIT-921 --stage plan --json
```

---

## Prompt for Next Session

```
Continue SPEC-KIT-921 P1 implementation from docs/spec-kit/CONTINUATION-PROMPT-P1.md

Context:
- P0 fixes complete (commit 76e62675f)
- 12 CLI tests + 183 spec-kit tests passing
- Architect approved P1 scope with specific acceptance criteria

P1 Tasks (in order):
1. P1-A: Verify TUI parity (likely already done, audit only)
2. P1-C: Add --strict-schema flag (parse failures → exit 3)
3. P1-B: Create SPEC-CI-001 smoke packet with clean/conflict/malformed cases
4. P1-D: Add evidence_root config + CLI override

Start with P1-A audit, then proceed to P1-C implementation.
```
