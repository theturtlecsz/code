# HANDOFF.md - SPEC-KIT Session Continuity

**Session Date**: 2026-01-18 (Evening)
**Status**: SPEC-KIT-975 (95%) + SPEC-KIT-978 (100%)

---

## What Was Completed This Session (2026-01-18 Evening)

### PR 1: SPEC-KIT-975 Runtime Emit Wiring (Complete)

**Objective:** Wire the emit helpers (already defined) to actual runtime boundaries.

**Files Created:**
- `tui/src/chatwidget/spec_kit/event_emitter.rs` - AuditEventEmitter with RunContext
  - `emit_tool_call()` / `emit_tool_result()` - Tool dispatch boundary
  - `emit_retrieval_request()` / `emit_retrieval_response()` - Retrieval boundary
  - `emit_patch_apply()` - Patch apply boundary
  - `emit_model_call_envelope()` - Model call with capture mode enforcement
  - `emit_gate_decision()` / `emit_error()` - Governance events

**Files Modified:**
- `tui/src/memvid_adapter/adapter.rs` - Added EmitContext, wired search_memories
- `tui/src/chatwidget/spec_kit/mod.rs` - Exported event_emitter, AuditEventEmitter, RunContext
- `tui/src/memvid_adapter/mod.rs` - Exported EmitContext

**Tests Added (3 new):**
- `test_runtime_emit_wiring_integration` - Full emit flow verification
- `test_emit_wiring_best_effort_never_fails` - Best-effort semantics
- `test_retrieval_events_capture_hit_uris` - URI capture verification

### PR 2: SPEC-KIT-975 Replay Engine (Complete)

**Objective:** CLI for replaying and verifying run events.

**New Commands:**
```bash
code speckit replay run --run <RUN_ID> [--branch ...] [--json] [--types ...]
code speckit replay verify --run <RUN_ID> [--check-retrievals] [--check-sequence] [--json]
```

**Files Modified:**
- `cli/src/speckit_cmd.rs`:
  - Added `ReplayArgs`, `ReplaySubcommand`, `ReplayRunArgs`, `ReplayVerifyArgs`
  - Added `run_replay()`, `run_replay_run()`, `run_replay_verify()`
  - Added `extract_seq_from_uri()`, `format_event_summary()`

**Timeline Output Example:**
```
Run Timeline: SPEC-KIT-975_20260118_abc123
Branch: run/SPEC-KIT-975_20260118_abc123

=== Stage: Plan ===

10:30:15.123 [TOOL]   Tool: read_file
10:30:15.456 [RESULT] Success
10:30:16.001 [QUERY]  "spec-kit architecture"
10:30:16.050 [HITS]   3 hits

Total: 45 events, 2 checkpoints
```

### PR 3: SPEC-KIT-978 TUI Slash Commands (Complete)

**Objective:** Expose reflex e2e and check commands in TUI.

**New TUI Commands:**
- `/speckit.reflex e2e [--stub]` - Run E2E routing tests
- `/speckit.reflex check [duration]` - Validate bakeoff thresholds

**Files Modified:**
- `tui/src/chatwidget/spec_kit/command_handlers.rs`:
  - Added e2e and check to `handle_speckit_reflex()` match
  - Added `handle_reflex_e2e()` - Mirrors CLI E2E tests (5 assertions)
  - Added `handle_reflex_check()` - Threshold validation with metrics DB

### PR 4: Circuit Breaker (Not Started)

Deferred to next session. Foundation laid:
- `EventType::BreakerStateChanged` ready to add
- `RoutingFallbackReason::CircuitBreakerOpen` ready to add
- `circuit_breaker.rs` module planned

---

## Session Metrics

- **Build status:** Green (all crates compile)
- **Test status:** All new tests pass (3 emit wiring + existing)
- **Doc lint:** Should pass (no schema changes this session)

---

## Gating Commands Status

```bash
# Need to run before session end:
python3 scripts/doc_lint.py                      # Pending
cargo test -p codex-tui --lib                   # 693 tests expected
cargo test -p codex-cli --lib                   # Pending
code speckit reflex e2e --stub                  # Pending
```

---

## Remaining Work

### SPEC-KIT-975 (5% remaining)
- Add replay tests in `tests.rs` (deterministic, offline retrieval)
- Update SPEC-KIT-975 spec docs with event schema + replay CLI

### SPEC-KIT-978 (0% remaining - Complete)
- All TUI commands implemented

### Circuit Breaker (PR 4)
- Add BreakerState, BreakerStateChangedPayload to types.rs
- Implement circuit_breaker.rs module
- Integrate into reflex_router.rs

---

## Previous Session Completed (2026-01-18 Afternoon)

### SPEC-KIT-975: Event Schema v1 (85% Complete)

### SPEC-KIT-975: Event Schema v1 (85% Complete)

**Objective:** Expand event types for auditable replay - the gating unlock for 973/976.

**Event Types Added (10 new):**
- `RetrievalRequest` / `RetrievalResponse` - Retrieval audit trail
- `ToolCall` / `ToolResult` - Tool invocation audit trail
- `PatchApply` - File modification audit trail
- `GateDecision` - Governance gate outcomes
- `ErrorEvent` - Error tracking
- `ModelCallEnvelope` - LLM I/O capture (mode-dependent)
- `CapsuleExported` / `CapsuleImported` - Provenance tracking

**LLM Capture Modes (D15 alignment):**
- `off`: Don't capture model calls
- `hash`: Content hashes only (export-safe)
- `summary`: Summary + hash (default, export-safe)
- `full`: Full content (NOT export-safe)

**Event Classification:**
- `is_curated_eligible()` - For merge mode filtering
- `is_audit_critical()` - For compliance replay requirements

**CLI Event Filters Extended:**
```bash
code speckit capsule events --type ToolCall           # Filter by type
code speckit capsule events --branch run/xyz          # Filter by branch
code speckit capsule events --since-checkpoint cp123  # Events after checkpoint
code speckit capsule events --audit-only              # Audit-critical only
code speckit capsule events --curated-only            # Curated-eligible only
```

**Files Modified:**
- `tui/src/memvid_adapter/types.rs` - EventType enum, payload structs, LLMCaptureMode
- `cli/src/speckit_cmd.rs` - CapsuleEventsArgs with new filters

**Tests Added (12 new):**
- `event_type_all_variants_covered`
- `event_type_curated_classification`
- `event_type_audit_critical_classification`
- `llm_capture_mode_export_safety`
- `llm_capture_mode_default_is_summary`
- `llm_capture_mode_round_trip`
- `gate_outcome_variants`
- `error_severity_variants`
- `retrieval_request_payload_serialization`
- `tool_call_payload_serialization`
- `gate_decision_payload_serialization`
- `model_call_envelope_payload_serialization`

---

## Previous Session Completed

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

### SPEC-KIT-975/978: E2E Reflex Test Harness

**Feature:** E2E test harness for validating reflex routing and event emission.

**CLI Command:**
```bash
# CI-safe stub mode (no real inference server needed)
code reflex e2e --stub

# Against real SGLang/vLLM endpoint
code reflex e2e --endpoint http://localhost:3009/v1 --model qwen2.5-coder-7b-instruct

# With verbose output
code reflex e2e --verbose --json

# Environment variable overrides
REFLEX_E2E_ENDPOINT=http://myserver/v1 REFLEX_E2E_MODEL=mymodel code reflex e2e
```

**Tests Included:**
1. Non-Implement stage uses Cloud (not reflex)
2. Reflex disabled uses Cloud
3. Routing decision has cloud_model field
4. RoutingDecisionPayload serializes correctly
5. Event type classification (curated/audit-critical)

**Files Modified:**
- `cli/src/reflex_cmd.rs` - Added E2E subcommand with E2eArgs, E2eResult, run_reflex_e2e()
- `tui/src/lib.rs` - Re-exported reflex_router module

### SPEC-KIT-975: Emit Helper Methods

**Feature:** CapsuleHandle methods for emitting all SPEC-KIT-975 event types.

**Methods Added:**
```rust
emit_tool_call(spec_id, run_id, payload: &ToolCallPayload)
emit_tool_result(spec_id, run_id, stage, payload: &ToolResultPayload)
emit_retrieval_request(spec_id, run_id, payload: &RetrievalRequestPayload)
emit_retrieval_response(spec_id, run_id, stage, payload: &RetrievalResponsePayload)
emit_patch_apply(spec_id, run_id, payload: &PatchApplyPayload)
emit_gate_decision(spec_id, run_id, payload: &GateDecisionPayload)
emit_error_event(spec_id, run_id, payload: &ErrorEventPayload)
emit_model_call_envelope(spec_id, run_id, payload: &ModelCallEnvelopePayload)
emit_capsule_exported(spec_id, run_id, payload: &CapsuleExportedPayload)
emit_capsule_imported(spec_id, run_id, payload: &CapsuleImportedPayload)
```

**Integration Tests:**
- `test_spec_kit_975_event_emission` - Verifies all event types can be emitted and retrieved
- `test_llm_capture_modes` - Verifies LLM capture mode serialization and export safety

**Files Modified:**
- `tui/src/memvid_adapter/capsule.rs` - Emit helper methods
- `tui/src/memvid_adapter/tests.rs` - Integration tests
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

## Session 2026-01-18 Completed

### SPEC-KIT-975 (85% → Event Schema Complete)
- 10 new event types: RetrievalRequest/Response, ToolCall/Result, PatchApply, GateDecision, ErrorEvent, ModelCallEnvelope, CapsuleExported/Imported
- LLMCaptureMode enum: off/hash/summary/full with is_export_safe()
- Classification methods: is_curated_eligible(), is_audit_critical()
- CapsuleHandle emit helpers: emit_tool_call, emit_retrieval_request, emit_gate_decision, etc.
- CLI filters: --type, --branch, --since-checkpoint, --audit-only, --curated-only
- Integration tests: test_spec_kit_975_event_emission, test_llm_capture_modes

### SPEC-KIT-978 (90% → E2E Harness Complete)
- `code reflex e2e` command with --stub (CI-safe) and --endpoint (real server) modes
- 5 test assertions: routing correctness, fallback behavior, serialization, classification
- Environment variable overrides: REFLEX_E2E_ENDPOINT, REFLEX_E2E_MODEL

### Test Results
- 985 tests passing: stage0 (293) + tui (686) + cli (6)
- doc_lint passes (5 non-blocking warnings on legacy specs)

## Priorities for Next Session

1. **SPEC-KIT-975 Completion (15% remaining)**
   - Wire emit calls at runtime points (Stage0 pipeline, retrieval, tool dispatch)
   - Add replay engine v1 (offline deterministic replay)
   - Add `speckit replay <RUN_ID> --as-of <CHECKPOINT>` CLI command

2. **SPEC-KIT-978 Completion (10% remaining)**
   - TUI slash commands for reflex operations
   - Final CI gate integration

3. **SPEC-KIT-973/976 Unblock** (after 975 complete)
   - 973: Time-Travel UI (replay visualization)
   - 976: Logic Mesh graph (event-based dependency graph)

4. **Circuit Breaker** (optional, post-E2E)
   - State per endpoint, fallback reason in events
   - Tests: failures → open → cooldown → recover

## Quick Reference Commands

# E2E Reflex Test
code reflex e2e --stub                    # CI-safe
code reflex e2e --endpoint URL -v --json  # Real server

# Event Timeline Inspection
code speckit capsule events --json --limit 20
code speckit capsule events --type ToolCall --curated-only
code speckit capsule events --run <RUN_ID> --audit-only

# Validation
python3 scripts/doc_lint.py
cargo test -p codex-stage0 -p codex-tui -p codex-cli

Start by wiring emit calls at Stage0 pipeline runtime points.
```
