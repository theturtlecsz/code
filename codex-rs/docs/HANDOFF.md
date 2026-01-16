# HANDOFF.md - SPEC-KIT-978 Reflex Implementation

**Session Date**: 2026-01-16
**Status**: Slice C Complete - Live Reflex Routing Implemented

---

## What Was Completed This Session

### SPEC-KIT-978 Slice C: Live Reflex Routing ✅

**Bakeoff Metrics Infrastructure:**
- Created `tui/src/chatwidget/spec_kit/reflex_metrics.rs`
  - SQLite table `reflex_bakeoff_metrics` for persistence
  - `record_reflex_attempt()` / `record_cloud_attempt()`
  - `compute_bakeoff_stats()` - P95, success %, JSON compliance %
  - `check_thresholds()` integration with routing decisions

**CLI/TUI Bakeoff Commands:**
- `code reflex bakeoff` - Show P95/success/compliance comparison
- `code reflex bakeoff --json` - Machine-readable output
- `code reflex bakeoff --since 1h` - Time-filtered stats
- `/speckit.reflex bakeoff` - TUI equivalent

**Live Reflex Routing (Full Integration):**
- Created `spawn_reflex_stage_agents_sequential()` in agent_orchestrator.rs
  - Uses `ReflexClient` for OpenAI-compatible local inference
  - Mirrors sequential agent execution pattern
  - Records metrics for each model call
- Modified Implement stage dispatch to branch on `RoutingMode`
- Added automatic fallback: reflex failure → cloud mode

**Reflex Client:**
- Created `tui/src/chatwidget/spec_kit/reflex_client.rs`
  - OpenAI-compatible chat completion client
  - Non-streaming and streaming support (SSE)
  - JSON schema enforcement
  - Timeout handling with configurable duration

**Threshold Checking:**
- P95 latency check against `thresholds.p95_latency_ms`
- Success rate check against `thresholds.success_parity_percent`
- JSON compliance check against `thresholds.json_schema_compliance_percent`
- Minimum sample requirement (10 samples) before enforcing thresholds

### Carry-over from Previous Session

**SPEC-KIT-977: Policy Drift Detection ✅**
- `latest_policy_ref_for_run()` - Derive policy from capsule events
- `restore_policy_from_events()` - Restore policy state on reopen
- `check_and_recapture_if_changed()` - Auto-restore before checking

**SPEC-KIT-977-A1: Deterministic Hash ✅**
- `compute_hash()` now uses BTreeMap for sorted keys
- `source_files` sorted before hashing
- Stable hash regardless of insertion order

**SPEC-KIT-971: Branch Isolation ✅**
- Added `branch_id: Option<String>` to `CheckpointMetadata`
- Added `branch_id: Option<String>` to `RunEventEnvelope`
- `list_checkpoints_filtered()` / `list_events_filtered()` support branch filtering

---

## Commits This Session

1. `a627cf573` - feat(reflex): SPEC-KIT-978 live reflex routing for Implement stage

Previous session commits:
- `20e9e62d8` - feat(capsule): SPEC-KIT-971/977 unified capsule configuration
- `d2d41232e` - feat(reflex): SPEC-KIT-978 health check + routing decision events

---

## Current Architecture

### Reflex Routing Flow

```
Implement Stage Entry
    ↓
emit_implementer_routing_decision()
    ├── Check: Is stage "implement"?
    ├── Check: Is reflex enabled in model_policy.toml?
    ├── Check: Is reflex server healthy? (GET /v1/models)
    ├── Check: Are bakeoff thresholds met?
    └── Return: RoutingDecision { mode, is_fallback, reason, config }
    ↓
match decision.mode
    ├── Reflex → spawn_reflex_stage_agents_sequential()
    │              ├── Use ReflexClient for inference
    │              ├── Record metrics on each call
    │              └── On failure → return Err() for fallback
    └── Cloud  → spawn_regular_stage_agents_sequential()
    ↓
If Reflex Err → fallback to Cloud automatically
```

### Key Files

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/spec_kit/reflex_router.rs` | Routing decision logic + health check |
| `tui/src/chatwidget/spec_kit/reflex_client.rs` | OpenAI-compatible inference client |
| `tui/src/chatwidget/spec_kit/reflex_metrics.rs` | SQLite bakeoff metrics |
| `tui/src/chatwidget/spec_kit/agent_orchestrator.rs:632-804` | Reflex spawner |
| `tui/src/chatwidget/spec_kit/agent_orchestrator.rs:998-1060` | Mode branching |
| `cli/src/speckit_cmd.rs` | CLI reflex commands |
| `stage0/src/reflex_config.rs` | Shared config types |

---

## Test Status

**All tests passing:**
```
6 reflex_router tests:
- test_routing_mode_string_representation
- test_routing_decision_mode_branching
- test_routing_not_implement_stage
- test_fallback_reason_variants
- test_routing_decision_payload_serialization
- test_routing_reflex_disabled
```

---

## Next Session Options

### Option 1: E2E Testing with Real Reflex Server

**Objective:** Validate the full routing flow with an actual local inference server.

**Tasks:**
1. Set up SGLang or vLLM with Qwen2.5-Coder model
2. Configure `model_policy.toml` to point to local server
3. Run `/speckit.auto` on a test SPEC
4. Verify routing events in capsule
5. Compare metrics via `code reflex bakeoff`

### Option 2: SPEC-KIT-975 Event Types Expansion

**Objective:** Expand capsule event types for richer audit trail.

**Event Types to Add (from types.rs comments):**
- `RetrievalRequest` / `RetrievalResponse`
- `ToolCall` / `ToolResult`
- `PatchApply`
- `GateDecision`
- `ErrorEvent`
- `ModelCallEnvelope`
- `BranchMerged`
- `CapsuleExported` / `CapsuleImported`

### Option 3: Streaming Support Activation

**Objective:** Enable streaming responses for reflex client (already stubbed).

**Tasks:**
1. Wire `chat_completion_streaming()` into spawn_reflex_stage_agents_sequential
2. Add progress indicators for streaming responses
3. Handle partial failures in streaming mode

### Option 4: Reliability Patterns

**Objective:** Add circuit breaker and rate limiting for production resilience.

**Tasks:**
1. Implement circuit breaker (open after N failures, half-open retry)
2. Add rate limiting per endpoint
3. Track circuit state in metrics

---

## Configuration Reference

**model_policy.toml structure:**
```toml
[routing.reflex]
enabled = true
endpoint = "http://127.0.0.1:3009/v1"
model = "qwen2.5-coder-7b-instruct"
timeout_ms = 30000
json_schema_required = true
fallback_to_cloud = true

[routing.reflex.thresholds]
p95_latency_ms = 2000
success_parity_percent = 85
json_schema_compliance_percent = 100
```

---

## Continuation Prompt

```
Continue SPEC-KIT development. Reference docs/HANDOFF.md for full context.

Completed:
- SPEC-KIT-978 Slices A, B, C (reflex health, routing events, live routing)
- SPEC-KIT-977 (policy drift detection, deterministic hash)
- SPEC-KIT-971 (branch isolation)

Choose focus for this session:
1. E2E Testing - Test reflex routing with actual local inference server
2. SPEC-KIT-975 - Expand capsule event types for richer audit trail
3. Streaming - Activate streaming responses for reflex client
4. Reliability - Add circuit breaker and rate limiting patterns

Start by reading the current state and confirming which focus area to pursue.
```
