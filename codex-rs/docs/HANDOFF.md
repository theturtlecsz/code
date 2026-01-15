# HANDOFF.md - SPEC-KIT-978 Reflex Implementation

**Session Date**: 2026-01-15
**Status**: Slices A & B Complete, Slice C Ready to Start

---

## What Was Completed This Session

### SPEC-KIT-978 Slice A: Health Check (978-A5) ✅

**CLI Commands:**
- `code reflex health` - Check server health + model availability
- `code reflex status` - Show reflex configuration
- `code reflex models` - List available models

**TUI Commands:**
- `/speckit.reflex health`
- `/speckit.reflex status`
- `/speckit.reflex models`

**Files Created:**
- `cli/src/reflex_cmd.rs` - CLI command implementation
- `stage0/src/reflex_config.rs` - Shared config module
- `tui/src/chatwidget/spec_kit/commands/reflex.rs` - TUI command struct
- `tui/src/chatwidget/spec_kit/reflex_router.rs` - Routing decision logic

### SPEC-KIT-978 Slice B: Routing Decision Events ✅

**Capsule Event Infrastructure:**
- Added `EventType::RoutingDecision` to capsule event types
- Added `RoutingMode` enum (Cloud, Reflex)
- Added `RoutingFallbackReason` enum (7 reasons)
- Added `RoutingDecisionPayload` struct
- Added `emit_routing_decision()` method to CapsuleHandle

**Routing Decision Logic:**
- `decide_implementer_routing()` - Makes routing decision
- `emit_routing_event()` - Emits to capsule
- Wired into `agent_orchestrator.rs` Implement stage dispatch

**Files Modified:**
- `tui/src/memvid_adapter/types.rs` - Event types
- `tui/src/memvid_adapter/capsule.rs` - Emit method
- `tui/src/memvid_adapter/mod.rs` - Re-exports
- `tui/src/chatwidget/spec_kit/agent_orchestrator.rs` - Wiring

### SPEC-KIT-971/977 Capsule Configuration Alignment ✅

**Problem Fixed:**
- `pipeline_coordinator.rs` used wrong path `.speckit/workspace.mv2` and `workspace_id: spec_id`
- Multiple files used `workspace_id: "workspace"` instead of `"default"`

**Solution:**
- Added `DEFAULT_CAPSULE_RELATIVE_PATH = ".speckit/memvid/workspace.mv2"`
- Added `DEFAULT_WORKSPACE_ID = "default"`
- Added `default_capsule_path()` and `default_capsule_config()` helpers
- Updated all write operations to use canonical config

**Files Modified:**
- `tui/src/memvid_adapter/mod.rs` - Constants and helpers
- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
- `tui/src/chatwidget/spec_kit/git_integration.rs`
- `tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
- `tui/src/chatwidget/spec_kit/stage0_integration.rs`
- `cli/src/speckit_cmd.rs`

---

## Commits This Session

1. `d2d41232e` - feat(reflex): SPEC-KIT-978 health check + routing decision events
2. (pending) - feat(capsule): SPEC-KIT-971/977 unified capsule configuration

---

## Next Session: SPEC-KIT-978 Slice C

### Priority 1: Bakeoff Metrics Collection

**Objective:** Track P95 latency, success rate, and JSON schema compliance for reflex vs cloud comparison.

**Tasks:**
1. Create `tui/src/chatwidget/spec_kit/reflex_metrics.rs`:
   - `BakeoffMetrics` struct (latency_samples, success_count, failure_count, json_compliance_count)
   - `record_reflex_attempt()` - Record latency + outcome
   - `record_cloud_attempt()` - Record cloud baseline
   - `compute_bakeoff_stats()` - Calculate P95, success %, compliance %

2. Add SQLite storage for metrics persistence:
   - Table: `reflex_bakeoff_metrics`
   - Columns: timestamp, mode, latency_ms, success, json_compliant, spec_id, run_id

3. Wire metrics recording into `emit_implementer_routing_decision()`

### Priority 2: Bakeoff CLI/TUI Commands

**Tasks:**
1. Add `code reflex bakeoff` CLI command:
   - Show P95 latency comparison (reflex vs cloud)
   - Show success rate comparison
   - Show JSON compliance rate
   - `--json` output support
   - `--since <duration>` filter (default: 24h)

2. Add `/speckit.reflex bakeoff` TUI command with same functionality

### Priority 3: Live Reflex Routing (Full Integration)

**Objective:** When reflex is healthy and thresholds met, actually route Implementer inference calls to local server.

**Tasks:**
1. Modify `spawn_regular_stage_agents_sequential()` in agent_orchestrator.rs:
   - Check routing decision mode
   - If `RoutingMode::Reflex`, configure agent to use reflex endpoint
   - Pass reflex endpoint/model to agent config

2. Add reflex client in `tui/src/chatwidget/spec_kit/reflex_client.rs`:
   - OpenAI-compatible chat completion client
   - Timeout handling (fallback to cloud if exceeded)
   - JSON schema enforcement check

3. Implement fallback mechanism:
   - If reflex request fails, emit fallback event
   - Retry with cloud automatically
   - Record both attempts in metrics

### Priority 4: Threshold Checking

**Tasks:**
1. Add threshold evaluation before routing:
   - Check `thresholds.p95_latency_ms` against recent metrics
   - Check `thresholds.success_parity_percent` against recent metrics
   - Check `thresholds.json_schema_compliance_percent` against recent metrics

2. Add `RoutingFallbackReason::ThresholdsNotMet` variants for each threshold

---

## Configuration Reference

**model_policy.toml structure:**
```toml
[routing.reflex]
enabled = true
endpoint = "http://127.0.0.1:3009/v1"
model = "qwen2.5-coder-7b-instruct"
timeout_ms = 1500
json_schema_required = true
fallback_to_cloud = true

[routing.reflex.thresholds]
p95_latency_ms = 2000
success_parity_percent = 85
json_schema_compliance_percent = 100
```

---

## Test Status

**All tests passing:**
- CLI reflex: 3 tests
- TUI reflex: 4 tests (command + router)
- Stage0 reflex_config: 2 tests
- Git integration: 5 tests (capsule checkpoints)

---

## Key Files to Reference

| File | Purpose |
|------|---------|
| `stage0/src/reflex_config.rs` | Shared config types |
| `tui/src/chatwidget/spec_kit/reflex_router.rs` | Routing decision logic |
| `tui/src/memvid_adapter/types.rs` | RoutingDecision event types |
| `tui/src/chatwidget/spec_kit/agent_orchestrator.rs:818-883` | Routing emit wiring |
| `cli/src/reflex_cmd.rs` | CLI command reference |

---

## Continuation Prompt

```
Continue SPEC-KIT-978 Slice C implementation. Reference docs/HANDOFF.md for context.

Session goals:
1. Implement bakeoff metrics collection (SQLite storage + recording)
2. Add `code reflex bakeoff` CLI and `/speckit.reflex bakeoff` TUI commands
3. Wire live reflex routing when healthy (full integration, not shadow mode)
4. Add threshold checking before routing decisions

Start with metrics infrastructure, then CLI/TUI commands, then live routing integration.

Key constraint: Reflex routing is ONLY valid for Implement stage. All other stages use cloud.
```
