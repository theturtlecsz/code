# HANDOFF.md - SPEC-KIT Session Continuity

**Session Date**: 2026-01-17
**Status**: Policy CLI/TUI + Capsule Merge Semantics Complete

---

## What Was Completed This Session

### SPEC-KIT-971: Run ID Normalization

**Problem:** Run branches were `run/run_SPEC-KIT-971_...` (redundant `run_` prefix).

**Solution:** Changed `generate_run_id()` format in `execution_logger.rs`:
```rust
// Before: format!("run_{}_{}_{}") → "run_SPEC-KIT-971_20260117_abc12345"
// After:  format!("{}_{}_{}")     → "SPEC-KIT-971_20260117_abc12345"
```

**Files Modified:**
- `tui/src/chatwidget/spec_kit/execution_logger.rs:28-33`

### Memvid Capsule Reopen Fix

**Problem:** After `CapsuleHandle::open()` on existing capsule, `resolve_uri(branch, as_of=None)` didn't preserve branch context.

**Solution:** Added `restore_entries_from_latest_snapshots()` to `UriIndex`:
- On `scan_and_rebuild()`, reconstruct `UriIndex.entries[branch]` from latest snapshot per branch
- Ensures "current state" queries work correctly after reopen

**Files Modified:**
- `tui/src/memvid_adapter/types.rs` - Added `restore_entries_from_latest_snapshots()`
- `tui/src/memvid_adapter/capsule.rs` - Updated `scan_and_rebuild()` to call restoration

### MergeMode Semantics (SPEC-KIT-971)

**Feature:** Curated vs Full merge modes for `CapsuleHandle::merge_branch()`.

**Curated Mode:**
- Promotes only stage artifacts + governance artifacts + baseline events
- Excludes debug-only class (e.g., `DebugTrace` events)

**Full Mode:**
- Promotes all artifacts and events regardless of classification

**Implementation:**
- Added `DebugTrace` to `EventType` enum
- Added `is_curated_eligible()` to `EventType` and `LogicalUri`
- Updated `merge_branch()` to accept `MergeMode` parameter
- Added `emit_event_on_branch()` for explicit branch targeting
- Updated `list_events_filtered()` to respect merge provenance via `BranchMerged` events

**Files Modified:**
- `tui/src/memvid_adapter/types.rs` - MergeMode enum, classification methods
- `tui/src/memvid_adapter/capsule.rs` - Merge logic, event emission
- `tui/src/memvid_adapter/tests.rs` - 5 new merge mode tests

### SPEC-KIT-977: Policy Diff Commands

**CLI Commands:**
```bash
code speckit policy diff <idA> <idB>        # Human-readable diff
code speckit policy diff <idA> <idB> --json  # Machine-parseable
code speckit policy validate [--path <path>] # Validate policy file
```

**TUI Command:**
```
/speckit.policy diff <idA> <idB>
```

**Implementation:**
- Added `PolicyFieldChange`, `ChangeCategory`, `PolicyDiff` structs
- `PolicyDiff::compute()` with deterministic output (sorted by path)
- Categories: Governance, ModelConfig, Weights, SourceFiles, Prompts, Schema

**Files Modified:**
- `stage0/src/policy.rs` - PolicyDiff implementation (lines 1049-1454)
- `stage0/src/lib.rs` - Added exports
- `cli/src/speckit_cmd.rs` - CLI command implementation
- `tui/src/chatwidget/spec_kit/commands/policy.rs` - TUI command

### SPEC-KIT-978: Reflex Bakeoff Run Command

**CLI Commands:**
```bash
code speckit reflex run-bakeoff [--trials N] [--json]  # Run actual bakeoff trials
code speckit reflex check --ci-gate                    # CI gate mode
```

**Implementation:**
- Added `ReflexSubcommand::RunBakeoff` with `--trials`, `--json`, `--fail-on-threshold`
- Added `--ci-gate` flag to `ReflexCheckArgs`
- CI gate fails when thresholds not met AND reflex is enabled in policy

**Files Modified:**
- `cli/src/speckit_cmd.rs` - Bakeoff run and CI gate
- `tui/src/lib.rs` - Re-export for `bakeoff_runner`

---

## Key Bug Fixes

### BranchMerged Event on Wrong Branch
**Problem:** `BranchMerged` event was emitted on run branch instead of main.
**Cause:** `emit_event()` used `self.current_branch()`.
**Fix:** Added `emit_event_on_branch(event, branch)` for explicit branch targeting.

### Merge Not Persisted After Reopen
**Problem:** In-memory event branch_id modifications during merge weren't persisted.
**Fix:** Instead of modifying event records, track merges via `BranchMerged` events. Updated `list_events_filtered()` to check `BranchMerged` events when filtering for main.

---

## Test Status

**All tests passing:**
```
50+ memvid tests including:
- test_curated_merge_excludes_debug_events
- test_full_merge_includes_debug_events
- test_curated_merge_persists_after_reopen
- test_event_type_curated_classification
- test_uri_curated_classification
- test_time_travel_survives_reopen

8 PolicyDiff tests:
- test_policy_diff_identical
- test_policy_diff_model_config_changes
- test_policy_diff_weights_changes
- test_policy_diff_governance_changes
- test_policy_diff_multiple_changes
- test_policy_diff_change_categories
- test_policy_diff_json_output
- test_policy_diff_text_output
```

---

## Architecture Summary

### Merge Mode Flow
```
merge_branch(from_branch, to_branch, mode)
    ↓
For each URI in from_branch:
    ├── MergeMode::Full → merge all
    └── MergeMode::Curated → check uri.is_curated_eligible()
    ↓
Emit BranchMerged event on to_branch
    ↓
list_events_filtered(branch=main) respects merge provenance
```

### Policy Diff Flow
```
PolicyDiff::compute(snapshot_a, snapshot_b)
    ↓
Compare: model_config, weights, governance, source_files, prompts, schema_version
    ↓
Group changes by ChangeCategory
    ↓
Output: to_text() or to_json()
```

---

## Next Session Priorities

### Priority 1: SPEC-KIT-975 Events (Foundation)

**Objective:** Expand event types for auditable replay - the gating unlock.

**Event Types to Add:**
- `ToolCall` / `ToolResult`
- `RetrievalRequest` / `RetrievalResult`
- `PatchProposed` / `PatchApplied`
- `LLMCall` (or split request/response) aligned to Policy capture mode
- `GateDecision`
- `ErrorEvent`

**Done When:**
- Event envelopes are branch-aware and reference only logical `mv2://` URIs
- Filters work: `code speckit capsule events --run --branch --type --since-checkpoint`

### Priority 2: E2E Testing (Keep 978 Honest)

**Objective:** Validate reflex routing with real local inference server.

**Tasks:**
1. Start SGLang/vLLM with Qwen2.5-Coder model
2. Run reflex routing scenario end-to-end
3. Assert:
   - Routing decision correctness
   - Fallback behavior correctness
   - Events emitted (RoutingDecision + LLMCall + etc.)
   - Policy capture mode respected

### Priority 3: Circuit Breaker (Phase 2 After E2E)

**Objective:** Add production resilience patterns based on E2E findings.

**Tasks:**
1. Circuit breaker state per endpoint/model
2. Clear fallback reason recorded in events
3. Tests for: consecutive failures → open → cooldown → half-open → recover

### Polish Items (Include with 975/E2E)

**SPEC-KIT-971:**
- Run ID normalization edge cases (URI/branch naming debt)

**SPEC-KIT-977:**
- Policy drift auto-recapture improvements
- Ensure event timelines bind to correct policy snapshot

---

## Documentation Updates Needed

| Document | Update |
|----------|--------|
| `SPEC.md` | Task statuses + gates for 971/977/978 |
| `SPEC-KIT-975` spec | Event schemas + acceptance tests |
| `SPEC-KIT-978` spec | Reference new event types from reflex calls |
| `OPERATIONAL-PLAYBOOK.md` | Add "how to run E2E reflex test" + event log interpretation |
| `MODEL-GUIDANCE.md` | Capture modes and what gets stored/omitted |

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/spec_kit/execution_logger.rs` | Run ID generation |
| `tui/src/memvid_adapter/types.rs` | UriIndex, BranchId, EventType, MergeMode |
| `tui/src/memvid_adapter/capsule.rs` | CapsuleHandle operations |
| `stage0/src/policy.rs` | PolicySnapshot, PolicyDiff |
| `cli/src/speckit_cmd.rs` | CLI commands |
| `tui/src/chatwidget/spec_kit/commands/policy.rs` | TUI policy commands |

---

## Continuation Prompt

```
Continue SPEC-KIT development. Reference docs/HANDOFF.md for full context.

This Session Completed:
- SPEC-KIT-971: Run ID normalization (removed redundant run_ prefix)
- SPEC-KIT-971: MergeMode semantics (Curated vs Full)
- SPEC-KIT-977: Policy diff CLI/TUI commands
- SPEC-KIT-978: Reflex bakeoff run command + CI gate
- Memvid reopen fix (branch context preservation)

Priorities for This Session:
1. SPEC-KIT-975 Events - Expand event types for auditable replay (foundation)
   - Add: ToolCall, RetrievalRequest/Result, PatchProposed/Applied, LLMCall, GateDecision
   - Ensure branch-aware envelopes with mv2:// URIs
   - Extend filters: --run, --branch, --type, --since-checkpoint

2. E2E Testing - Validate reflex routing with real local server
   - Start SGLang/vLLM, run end-to-end scenario
   - Assert: routing correctness, fallback behavior, events emitted, capture mode

3. Circuit Breaker (if time) - Add after E2E findings inform failure modes
   - Circuit state per endpoint, fallback reason in events
   - Tests: failures → open → cooldown → recover

4. Polish Items - Include alongside main work
   - 971 run ID edge cases
   - 977 policy drift auto-recapture

5. Documentation - Full update
   - SPEC.md gates, SPEC-KIT-975 event schemas, OPERATIONAL-PLAYBOOK E2E guide

Start by reading the current state and begin with SPEC-KIT-975 event type expansion.
```
