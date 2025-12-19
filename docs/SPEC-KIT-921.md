# SPEC-KIT-921: CLI Adapter + Shared SpeckitExecutor Core

## Status

| Field | Value |
|-------|-------|
| **Type** | Platform capability / Reliability / Automation |
| **Priority** | P1 (blocks CI automation + Claude Code iteration speed) |
| **Parent** | SPEC-KIT-920 (Automation MVP - superseded) |
| **Created** | 2025-12-19 |

## Problem Statement

We need a **non-TUI, automation-friendly execution surface** for Spec-Kit so that:

1. We can run end-to-end pipeline flows in CI without a PTY
2. Claude Code (and other non-TUI tooling) can execute specs and validate outcomes
3. TUI slash commands and CLI invocations use the **same command handlers** (no divergence)

Today, `tui/src/chatwidget/spec_kit/*` is tightly coupled to `ChatWidget`:

- Command implementations call UI methods (`history_push`, `request_redraw`, `submit_prompt`, etc.)
- Orchestration lives inside TUI layer (`pipeline_coordinator` invoked through widget handlers)

This prevents reuse in CLI and makes automation brittle.

### Why not PTY-based TUI automation?

SPEC-KIT-920 attempted `--initial-command` + `--exit-on-complete` flags, but:

- Requires PTY/terminal (fails in Proxmox/CI with "No such device or address")
- Even with PTY hacks, it's fragile and not deterministic
- CI should validate in the **most deterministic execution mode possible**

**The fix is not PTY tricks. The fix is ports/adapters architecture.**

## Goals

### G1 - Shared application core

Introduce a shared "application core" executor that:

- Accepts typed commands (`SpeckitCommand`)
- Runs pipeline operations (plan/tasks/implement/validate/audit/unlock/auto/status/review)
- Emits typed events (`SpeckitEvent`) to an event sink
- Uses ports/traits for IO (not `ChatWidget`)

### G2 - CLI surface (non-interactive)

Implement a CLI that:

- Does **not** initialize ratatui or require PTY
- Can be used in CI
- Returns deterministic exit codes
- Outputs progress as text and/or JSONL event stream

### G3 - TUI parity

Refactor TUI dispatch so:

- Slash commands map to the same `SpeckitCommand`
- TUI calls the same executor
- TUI renders the same event stream (instead of "core logic in widget methods")

### G4 - Deterministic "non-interactive" behavior

When running in CLI/CI:

- Escalation prompts never hang
- Escalation exits non-zero with machine-readable reason

## Non-goals

- Not building "headless TUI"
- Not redesigning gate policy / router (already stabilized)
- Not migrating DB schema or moving evidence dirs
- Not rewriting Stage0 in this spec (though `doctor` may check Stage0 reachability)

---

## Current Dispatch Map

Entry: `tui/src/app.rs:2066-2090` dispatches to ChatWidget handlers which delegate to `tui/src/chatwidget/spec_kit/*`, but those functions take `&mut ChatWidget`, mixing UI with orchestration.

### Current Files

| File | Purpose |
|------|---------|
| `tui/src/slash_command.rs` | `SlashCommand` enum + parsing |
| `tui/src/app.rs:2066-2090` | Dispatch in main event loop |
| `tui/src/chatwidget/mod.rs:13000-14500` | Handler wrappers + pipeline logic |
| `tui/src/chatwidget/spec_kit/` | All command implementations |
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Pipeline state machine |

**Cut line:** Extract `tui/src/chatwidget/spec_kit/*` logic into a shared executor crate/module and replace "call widget methods" with "emit events + use ports."

---

## Proposed Architecture

### 1) Command model (shared)

Define `SpeckitCommand` in a shared crate/module (not in TUI):

```rust
enum SpeckitCommand {
    Doctor { format: OutputFormat },
    Run {
        spec_id: String,
        from_stage: Option<Stage>,
        to_stage: Option<Stage>,
        non_interactive: bool,
        local_only: bool,
        format: OutputFormat,
    },
    Status { spec_id: String, format: OutputFormat },
    Review { spec_id: String, format: OutputFormat },
    Exec { raw: String }, // Bridge command for slash strings
}
```

**Adapter mapping:**
- TUI slash parsing -> `SpeckitCommand`
- CLI clap parsing -> `SpeckitCommand`

### 2) Event model (shared)

Define `SpeckitEvent` as the only output channel from the executor:

```rust
enum SpeckitEvent {
    RunStarted { spec_id: String, run_id: String },
    StageStarted { stage: Stage },
    StageProgress { stage: Stage, message: String, percent: Option<u8> },
    WorkerSelected { role: Role, worker_id: String },
    ArtifactWritten { kind: ArtifactKind, path: PathBuf },
    GateEvaluated {
        checkpoint: QualityCheckpoint,
        verdict: GateVerdict,
        blocking_signals: Vec<Signal>,
        advisory_signals: Vec<Signal>,
    },
    EscalationRequired { target: String, reason: String },
    RunFinished { outcome: Outcome },
}
```

**Why:** This makes it impossible for core logic to depend on UI primitives.

### 3) Executor (shared)

Introduce `SpeckitExecutor` with a single entrypoint:

```rust
impl SpeckitExecutor {
    pub fn execute(
        command: SpeckitCommand,
        context: ExecutionContext,
        ports: &dyn Ports,
        event_sink: &dyn EventSink,
    ) -> Result<Outcome, SpeckitError>;
}
```

Where:
- `context` includes spec id, repo root, config snapshot, policy toggles
- `ports` includes things that touch IO or services
- `event_sink` is implemented by TUI (renders) and CLI (prints)

### 4) Ports (traits)

Minimal set for MVP:

```rust
trait Ports {
    fn artifact_store(&self) -> &dyn ArtifactStore;
    fn tool_executor(&self) -> &dyn ToolExecutor;
    fn model_invoker(&self) -> Option<&dyn ModelInvoker>;
    fn router(&self) -> &dyn Router;
    fn policy_toggles(&self) -> &PolicyToggles;
    fn clock(&self) -> &dyn Clock;
    fn escalation_handler(&self) -> &dyn EscalationHandler;
}
```

`EscalationHandler` implementations:
- **CLI:** returns `Outcome::Escalated`
- **TUI:** opens UI prompt / handles interactive escalation

**Key rule:** Executor owns orchestration; adapters own rendering and interactive UX.

---

## CLI MVP

### Commands

1. `code speckit doctor [--format json|text]`
2. `code speckit run --spec SPEC-ID [--non-interactive] [--local-only] [--format jsonl|text] [--from <stage>] [--to <stage>]`
3. `code speckit status --spec SPEC-ID [--format json|text]`
4. `code speckit review --spec SPEC-ID [--format json|text]`
5. `code speckit exec "<slash string>"` (optional bridge)

### Exit codes (contract)

| Code | Meaning |
|------|---------|
| `0` | Success |
| `2` | Escalation required (non-interactive / fail-on-escalation) |
| `3` | Tool-truth failure (compile/tests) |
| `10` | Config invalid |
| `11` | Missing tool / permission |
| `12` | Dependency service unreachable |

---

## Migration Plan (incremental, low drama)

### Phase A - Extract core types + executor skeleton

- Create shared command/event/executor modules
- Provide no-op adapters initially

### Phase B - Migrate read-only paths first (safe)

- `status` + `review` (gate evaluation) moved behind executor
- TUI and CLI both call them
- Event stream is validated in tests

**Why start here:** Minimal mutation, deterministic outputs, prove the ports/events pattern works, immediately usable in CI.

### Phase C - Migrate guardrail stage ops

- plan/tasks/implement/validate/audit/unlock into executor
- Replace widget calls with emitted events and port calls

### Phase D - Migrate `/speckit.auto` pipeline coordinator

- `advance_pipeline` becomes core executor flow
- TUI becomes just an adapter (dispatch + render)

### Phase E - CI wiring + smoke spec

- Add a small "smoke spec" runnable via CLI (no Stage0 required)
- CI runs: `doctor` + `run --non-interactive --local-only`

---

## Acceptance Criteria

### A1 - No PTY required

CLI commands run in Proxmox/CI without ratatui initialization and without a TTY.

### A2 - Single codepath for business logic

For the migrated commands:
- TUI slash command calls `SpeckitExecutor`
- CLI command calls `SpeckitExecutor`
- No duplicate "core logic" implementations remain in TUI.

### A3 - Deterministic non-interactive behavior

- `--non-interactive` never hangs
- Escalations return exit code `2` and emit `EscalationRequired` event.

### A4 - Evidence compatibility preserved

- No changes to evidence directory layout or wire JSON keys in this spec.
- Existing golden evidence tests continue to pass.

### A5 - Regression proof

Add parity tests:
- Slash string -> parsed command equals CLI command parse for the same action (at least for `status`, `review`, `run`).

---

## Risks & Mitigations

### R1 - "ChatWidget owns too much"

**Mitigation:**
- Introduce a TUI adapter implementing ports/event sink
- Keep a minimal "TUI state" layer that consumes events

### R2 - Divergence between slash and CLI

**Mitigation:**
- Shared `SpeckitCommand` model + parity tests
- Optional `exec "<slash>"` bridge enforces shared parsing

### R3 - Scope creep

**Mitigation:**
- Migrate read-only commands first
- Ship CLI MVP early (doctor/status/review) before full pipeline.

---

## Tasks

- [ ] Create SPEC-KIT-921 doc (this spec)
- [ ] Add shared `SpeckitCommand`, `SpeckitEvent`, `SpeckitExecutor` modules
- [ ] Implement CLI MVP (doctor/status/review/run)
- [ ] Refactor TUI dispatch to call executor for status/review
- [ ] Extract guardrail stage ops behind executor
- [ ] Extract `/speckit.auto` orchestration behind executor
- [ ] Add CI job invoking CLI runner + smoke spec

---

## Files to Create/Modify

### New Files

| Path | Purpose |
|------|---------|
| `spec-kit/src/executor/mod.rs` | SpeckitExecutor trait + implementation |
| `spec-kit/src/executor/command.rs` | SpeckitCommand enum |
| `spec-kit/src/executor/event.rs` | SpeckitEvent enum |
| `spec-kit/src/executor/ports.rs` | Port traits (ArtifactStore, ToolExecutor, etc.) |
| `spec-kit/src/executor/context.rs` | ExecutionContext |
| `cli/src/speckit.rs` | CLI subcommand implementation |

### Modified Files (Phase B)

| Path | Change |
|------|--------|
| `tui/src/chatwidget/spec_kit/commands/status.rs` | Delegate to executor |
| `tui/src/chatwidget/spec_kit/gate_evaluation.rs` | Delegate to executor |
| `tui/src/app.rs` | Keep dispatch, but call executor |

---

## Relationship to SPEC-KIT-920

SPEC-KIT-920 implemented `--initial-command` and `--exit-on-complete` CLI flags. This work is **not wasted**:

- The flags are correctly wired through App struct
- Once CLI mode exists, these flags can trigger CLI execution instead of TUI
- The completion detection logic (`should_exit_on_automation_complete`) provides a pattern for the executor's `Outcome` handling

SPEC-KIT-921 **supersedes** SPEC-KIT-920's approach but builds on its foundation.

---

## Next Session

Recommended starting point for Phase B:

1. Map `handle_spec_status()` and `gate_evaluation.rs` call graphs
2. Identify "pure core" vs "UI adapter only" code
3. Define minimal port traits required for status + review
4. Extract surgically rather than full refactor spiral

---

## References

- `docs/HANDOFF.md` - Gate policy alignment history
- `docs/spec-kit/GATE_POLICY.md` - Canonical vocabulary
- `docs/MODEL-POLICY.md` - Role -> worker mapping
- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` - Current pipeline logic
