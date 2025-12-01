# HANDOFF: SPEC-KIT-102 V3 — Stage 0 Integration Phase 2

**Session**: P78 (2025-12-01)
**Prior Session**: P77 (V2 handoff - engine complete, adapters implemented)
**Status**: Phase 1 Complete — Pipeline wired, adapters functional

---

## Completed in This Session (P78)

### 1. MCP Adapters Created
- `tui/src/stage0_adapters.rs` — Full adapter implementations:
  - `LocalMemoryMcpAdapter` → wraps `mcp__local-memory__search`
  - `LlmStubAdapter` → heuristic fallback (no actual LLM calls in V1)
  - `Tier2McpAdapter` → wraps `mcp__notebooklm__ask_question`
  - `NoopTier2Client` → fallback when NotebookLM unavailable

### 2. Integration Module
- `tui/src/chatwidget/spec_kit/stage0_integration.rs`:
  - `run_stage0_for_spec()` — synchronous wrapper for pipeline
  - `write_task_brief_to_evidence()` — writes TASK_BRIEF.md
  - `build_stage0_context_prefix()` — (unused, for future use)
  - Dedicated single-threaded runtime (Stage0Engine not Send)

### 3. Pipeline Wiring
- `pipeline_coordinator.rs`: Stage0 runs after RunStart log, before advance_spec_auto
- `state.rs`: Added fields: `stage0_result`, `stage0_skip_reason`, `stage0_disabled`, `stage0_explain`
- `agent_orchestrator.rs`: Reads TASK_BRIEF.md from evidence/ into agent prompts

### 4. Build Status
- ✅ Clippy clean (only pre-existing dead_code warnings)
- ✅ All Stage0 crate tests passing (74 tests)

---

## Next Session Tasks (Phase 2)

### Task 1: CLI Flags
Add `--no-stage0` and `--stage0-explain` flags to `/speckit.auto`.

**Files to modify:**
- `tui/src/slash_command.rs` — Parse CLI flags
- `tui/src/chatwidget/spec_kit/pipeline_config.rs` — Add Stage0 config fields
- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` — Wire flags to state

**Implementation:**
```rust
// In slash_command.rs, extend SpecAutoArgs or similar:
pub struct SpecAutoFlags {
    pub no_stage0: bool,
    pub stage0_explain: bool,
}

// Parse from CLI: /speckit.auto SPEC-ID --no-stage0 --stage0-explain
```

### Task 2: ExecutionLogger Hooks
Add Stage0 events to execution_logger.rs for pipeline visibility.

**Events to add:**
```rust
pub enum ExecutionEvent {
    // ... existing events ...

    // SPEC-KIT-102: Stage 0 events
    Stage0Start {
        spec_id: String,
        run_id: String,
        timestamp: String,
    },
    Stage0Complete {
        spec_id: String,
        run_id: String,
        timestamp: String,
        outcome: String, // "success", "skipped", "error"
        tier2_used: bool,
        cache_hit: bool,
        memories_count: usize,
        latency_ms: u64,
        skip_reason: Option<String>,
    },
}
```

**Files to modify:**
- `tui/src/chatwidget/spec_kit/execution_logger.rs` — Add event variants
- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` — Emit events

### Task 3: Combined Context Injection
Use `Stage0Result.combined_context_md()` for full Divine Truth injection.

**Current approach:** Reads TASK_BRIEF.md from evidence/
**New approach:** Also include Divine Truth in prompts when available

**Files to modify:**
- `tui/src/chatwidget/spec_kit/agent_orchestrator.rs`:
```rust
// In build_individual_agent_prompt(), after TASK_BRIEF.md:

// Also inject Divine Truth if cached in state
// Option A: Read DIVINE_TRUTH.md from evidence/ (requires writing it)
// Option B: Pass Stage0Result through to function (requires signature change)

// Simplest: Write divine_truth.md to evidence/ alongside TASK_BRIEF.md
```

- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`:
```rust
// After successful Stage0, also write Divine Truth:
if let Err(e) = std::fs::write(
    evidence_dir.join("DIVINE_TRUTH.md"),
    &stage0_result.divine_truth.raw_markdown,
) {
    tracing::warn!("Failed to write DIVINE_TRUTH.md: {}", e);
}
```

### Task 4: Integration Tests
Create integration tests for Stage0 pipeline flow.

**Test file:** `tui/tests/stage0_integration_test.rs`

**Tests to implement:**
1. `test_stage0_disabled_skips_execution` — Verify --no-stage0 works
2. `test_stage0_runs_before_pipeline` — Verify Stage0 runs first
3. `test_stage0_writes_task_brief` — Verify TASK_BRIEF.md created
4. `test_stage0_context_in_prompts` — Verify agents receive context
5. `test_stage0_graceful_degradation` — Verify pipeline continues on Stage0 error

**Mock requirements:**
- Mock `McpConnectionManager` with local-memory responses
- Mock NotebookLM responses (or use NoopTier2Client)

---

## Architecture Reference

```
/speckit.auto SPEC-ID [--no-stage0] [--stage0-explain]
    ↓
handle_spec_auto()
    ├─ Load spec.md content
    ├─ Check stage0_disabled flag
    ↓
run_stage0_for_spec()           ← stage0_integration.rs
    ├─ Get MCP manager
    ├─ Create adapters (LocalMemory, LLM, Tier2)
    ├─ Stage0Engine::run_stage0()
    │     ├─ Build IQO (heuristic)
    │     ├─ Query local-memory
    │     ├─ Score & select memories
    │     ├─ Build TASK_BRIEF.md
    │     └─ Call Tier2 (NotebookLM) → Divine Truth
    ├─ Write TASK_BRIEF.md to evidence/
    ├─ Write DIVINE_TRUTH.md to evidence/  ← NEW
    └─ Log Stage0Complete event            ← NEW
    ↓
advance_spec_auto()
    ↓
build_individual_agent_prompt()   ← agent_orchestrator.rs
    ├─ Read TASK_BRIEF.md
    ├─ Read DIVINE_TRUTH.md        ← NEW
    └─ Build combined prompt
```

---

## File Checklist

| File | Status | Next Action |
|------|--------|-------------|
| `tui/src/stage0_adapters.rs` | ✅ Complete | — |
| `tui/src/chatwidget/spec_kit/stage0_integration.rs` | ✅ Complete | — |
| `tui/src/chatwidget/spec_kit/state.rs` | ✅ Fields added | — |
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | ✅ Wired | Add logging, Divine Truth write |
| `tui/src/chatwidget/spec_kit/agent_orchestrator.rs` | ✅ TASK_BRIEF | Add DIVINE_TRUTH read |
| `tui/src/slash_command.rs` | Pending | Add CLI flag parsing |
| `tui/src/chatwidget/spec_kit/execution_logger.rs` | Pending | Add Stage0 events |
| `tui/tests/stage0_integration_test.rs` | Pending | Create test file |

---

## Continuation Prompt

```
Continue SPEC-KIT-102 Stage 0 Integration Phase 2.

Prior session completed:
- MCP adapters (LocalMemory, Tier2, LLM stub)
- Stage0 pipeline wiring (runs before advance_spec_auto)
- TASK_BRIEF.md written to evidence/
- Agent prompts read TASK_BRIEF.md

This session implements:
1. CLI flags: --no-stage0, --stage0-explain (parse and wire to state)
2. ExecutionLogger: Add Stage0Start/Stage0Complete events
3. Combined context: Write DIVINE_TRUTH.md, read in agent prompts
4. Integration tests: 5 tests covering pipeline flow

Start with Task 1 (CLI flags) — modify slash_command.rs first.

Reference: docs/HANDOFF-SPEC-KIT-102-V3.md
```

---

## Session Context

- **Crate locations:**
  - Stage0 engine: `codex-rs/stage0/`
  - TUI integration: `codex-rs/tui/src/`
  - Adapters: `codex-rs/tui/src/stage0_adapters.rs`

- **Key types:**
  - `codex_stage0::Stage0Result` — main result type
  - `codex_stage0::Stage0Engine` — orchestrator
  - `crate::stage0_adapters::*` — MCP adapters

- **Test command:**
  ```bash
  cargo test -p codex-tui -- stage0
  cargo clippy -p codex-tui -p codex-stage0
  ```
