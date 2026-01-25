# Architecture Reference

> **Version**: 1.0.0 (2026-01-21)
>
> **Purpose**: System architecture documentation for Planner TUI and Spec-Kit automation.
>
> **Supersedes**: `docs/TUI.md` (partial), `docs/architecture/*.md`, `docs/SPEC-KIT-900-ARCHITECTURE-ANALYSIS.md`

***

## Table of Contents

* [Part I: System Overview](#part-i-system-overview)
  * [1. High-Level Architecture](#1-high-level-architecture)
    * [Component Summary](#component-summary)
  * [2. Key Boundaries](#2-key-boundaries)
* [Part II: TUI Surface Architecture](#part-ii-tui-surface-architecture)
  * [3. Chatwidget Module Structure](#3-chatwidget-module-structure)
    * [Module Extraction Progress (MAINT-11)](#module-extraction-progress-maint-11)
  * [4. State Machine Design](#4-state-machine-design)
    * [SpecAutoPhase Enum](#specautophase-enum)
    * [SpecAutoState Struct](#specautostate-struct)
    * [State Transitions](#state-transitions)
* [Part III: Concurrency Model](#part-iii-concurrency-model)
  * [5. Async/Sync Boundaries](#5-asyncsync-boundaries)
    * [Why block\_on() is Acceptable](#why-block_on-is-acceptable)
    * [Core Conflict Resolution](#core-conflict-resolution)
  * [6. Blocking Bridge Pattern](#6-blocking-bridge-pattern)
    * [Blocking Hotspots](#blocking-hotspots)
  * [7. Performance Characteristics](#7-performance-characteristics)
    * [MCP Synthesis Calls](#mcp-synthesis-calls)
    * [Mitigations](#mitigations)
    * [Thread Safety](#thread-safety)
* [Part IV: Spec-Kit Pipeline Architecture](#part-iv-spec-kit-pipeline-architecture)
  * [8. Pipeline Components](#8-pipeline-components)
  * [9. Command Flow](#9-command-flow)
    * [`/speckit.auto SPEC-ID` Execution](#speckitauto-spec-id-execution)
  * [10. Synthesis System](#10-synthesis-system)
    * [Agent Roster by Tier](#agent-roster-by-tier)
    * [Synthesis Quorum](#synthesis-quorum)
    * [Synthesis Metadata](#synthesis-metadata)
* [Appendices](#appendices)
  * [A. Developer Guidelines](#a-developer-guidelines)
    * [When to Use block\_on()](#when-to-use-block_on)
    * [Code Pattern](#code-pattern)
    * [Performance Profiling](#performance-profiling)
  * [B. Related Documentation](#b-related-documentation)
  * [C. Change History](#c-change-history)

# Part I: System Overview

## 1. High-Level Architecture

Planner is the `code` binary with an interactive TUI. Spec-Kit is implemented as slash commands under `/speckit.*`.

```
User input (TUI)
  -> slash parsing (codex-rs/tui/src/slash_command.rs)
  -> dispatch (codex-rs/tui/src/app.rs)
  -> Spec-Kit routing/registry (codex-rs/tui/src/chatwidget/spec_kit/)
  -> native pipeline + guardrails + evidence (writes under docs/SPEC-OPS-004-.../evidence/)
  -> shared Spec-Kit crate (codex-rs/spec-kit/) for config/retry/types
```

### Component Summary

| Component      | Concurrency Model | Purpose                                                  |
| -------------- | ----------------- | -------------------------------------------------------- |
| **Ratatui**    | Synchronous       | Terminal I/O (crossterm), render loop                    |
| **Tokio**      | Asynchronous      | HTTP/MCP, agent coordination                             |
| **Codex Core** | Asynchronous      | Model API calls (SSE streaming), conversation state      |
| **Spec-Kit**   | Hybrid            | Orchestration (sync), MCP calls (async), evidence (sync) |

## 2. Key Boundaries

| Boundary           | Location                                | Description                                                                                |
| ------------------ | --------------------------------------- | ------------------------------------------------------------------------------------------ |
| UX + Orchestration | `codex-rs/tui/src/chatwidget/spec_kit/` | User-facing commands and pipeline coordination                                             |
| Shared Library     | `codex-rs/spec-kit/`                    | Config, retry logic, shared types                                                          |
| Templates          | `./templates/` (project-local)          | Prompt templates with embedded fallbacks                                                   |
| Evidence Store     | `docs/SPEC-OPS-004-.../evidence/`       | Telemetry, artifacts, synthesis data. **Note**: Capsule is the SOR; filesystem is derived. |

***

# Part II: TUI Surface Architecture

## 3. Chatwidget Module Structure

The TUI chatwidget is the core rendering component (\~20K LOC).

```
chatwidget/ (Core TUI)
├── mod.rs (~19,800 LOC) - Main widget logic
│
├── Extracted Modules (MAINT-11)
│   ├── agent_status.rs (~130 LOC)
│   ├── command_render.rs (~340 LOC)
│   ├── input_helpers.rs (~200 LOC)
│   ├── review_handlers.rs (~580 LOC)
│   ├── session_handlers.rs (~560 LOC)
│   └── submit_helpers.rs (~380 LOC)
│
└── Original Modules
    ├── agent_install.rs (~870 LOC)
    ├── exec_tools.rs (~1,000 LOC) - Guardrail completion handling
    ├── terminal.rs (~920 LOC)
    └── ... (15+ more modules)
```

### Module Extraction Progress (MAINT-11)

| Metric               | Status | Target  |
| -------------------- | ------ | ------- |
| mod.rs LOC           | 19,792 | <15,000 |
| Extracted modules    | 6      | \~10    |
| Cumulative reduction | -3,621 | -8,413  |

## 4. State Machine Design

### SpecAutoPhase Enum

Tracks the current phase within a stage:

```rust
enum SpecAutoPhase {
    Guardrail,          // Running guardrail validation
    ExecutingAgents,    // Agents are spawned and running
    CheckingConsensus,  // Collecting and validating results
}
```

### SpecAutoState Struct

Tracks pipeline progress across all stages:

```rust
struct SpecAutoState {
    spec_id: String,
    goal: String,
    stages: Vec<SpecStage>,      // [Plan, Tasks, Implement, Validate, Audit, Unlock]
    current_index: usize,         // Which stage we're on
    phase: SpecAutoPhase,         // Phase within current stage
    agent_responses_cache: Vec<AgentResponse>,
}
```

### State Transitions

```
Guardrail(pass) → ExecutingAgents
ExecutingAgents(all_complete) → CheckingConsensus
CheckingConsensus(consensus_ok) → Guardrail(next_stage)
CheckingConsensus(conflict) → Halt
Guardrail(fail) → Halt
```

***

# Part III: Concurrency Model

## 5. Async/Sync Boundaries

**Problem**: Ratatui TUI (synchronous) + Tokio runtime (asynchronous) + MCP calls (async) create impedance mismatch.

**Solution**: `Handle::block_on()` bridges async operations into sync TUI event loop.

### Why block\_on() is Acceptable

Spec-kit workflows are:

* **Infrequent**: User-initiated `/speckit.*` commands
* **Not time-critical**: 10-60 min pipelines, 8ms blocking is negligible
* **User-visible**: Progress shown, not background work

### Core Conflict Resolution

```rust
// TUI runs INSIDE tokio context
pub async fn run_main(cli: Cli) -> TokenUsage {
    let codex = Codex::spawn(config, auth).await?;
    let app = App::new(codex, mcp_manager).await?;
    run_event_loop(app).await  // Event loop in tokio context
}
```

**Key Point**: TUI runs **inside** tokio context, so `Handle::current()` is available.

## 6. Blocking Bridge Pattern

**Pattern**: Sync function needs async operation.

```rust
// tui/src/chatwidget/spec_kit/handler.rs:722
let consensus_result = match tokio::runtime::Handle::try_current() {
    Ok(handle) => {
        handle.block_on(run_consensus_with_retry(...))  // BLOCKING
    }
    Err(_) => {
        Err(SpecKitError::from_string("No tokio runtime"))
    }
};
```

**Why this is safe**:

* Called from TUI event loop (single-threaded)
* No risk of deadlock (not inside another async fn)
* User initiated (not background task)
* Progress shown in TUI

### Blocking Hotspots

| Location         | Operation                   | Typical | Worst Case      | Frequency                   |
| ---------------- | --------------------------- | ------- | --------------- | --------------------------- |
| `handler.rs:429` | Synthesis check (plan)      | 8.7ms   | 700ms (cold)    | Per stage (6x/pipeline)     |
| `handler.rs:722` | Synthesis check (implement) | 8.7ms   | 700ms (cold)    | Per stage                   |
| `evidence.rs`    | File lock acquisition       | <1ms    | Unbounded (HDD) | Per artifact (20x/pipeline) |

**Total pipeline blocking**: \~50ms typical, \~4s worst-case (6 stages x 700ms cold-start)

## 7. Performance Characteristics

### MCP Synthesis Calls

| Metric              | Value                       |
| ------------------- | --------------------------- |
| Subprocess baseline | 46ms                        |
| Native MCP          | 8.7ms                       |
| Improvement         | **5.3x faster**             |
| Cold-start penalty  | 500-700ms (first call only) |

### Mitigations

1. **App-level MCP manager spawn** (once at startup)
2. **Retry logic** (AR-2, AR-3): Exponential backoff 100ms → 200ms → 400ms
3. **File-based fallback** (ARCH-002): Load from evidence directory if MCP unavailable

### Thread Safety

| Component       | Mechanism                                       |
| --------------- | ----------------------------------------------- |
| TUI event loop  | Single-threaded (no race conditions)            |
| MCP connections | `Arc<Mutex<Option<Arc<McpConnectionManager>>>>` |
| Evidence writes | `fs2::FileExt` exclusive locks (per-SPEC)       |

***

# Part IV: Spec-Kit Pipeline Architecture

## 8. Pipeline Components

```
┌─────────────────────────────────────────────────────────────┐
│                  SPEC-KIT AUTOMATION SYSTEM                  │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────────┐  │
│  │ Guardrails  │ → │ Quality Gates│ → │ Regular Stages  │  │
│  │ (Validation)│   │ (Pre-checks) │   │ (Multi-Agent)   │  │
│  └─────────────┘   └──────────────┘   └─────────────────┘  │
│        │                  │                      │           │
│        ↓                  ↓                      ↓           │
│  Native Check      3 Agents            3-4 Agents           │
│  - spec-id         Individual          Parallel             │
│  - clean-tree      Prompts             Spawning             │
│  - files exist                                              │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              SQLite Synthesis Database                │  │
│  │  - agent_executions (spawn tracking)                  │  │
│  │  - consensus_artifacts (agent outputs)                │  │
│  │  - consensus_synthesis (final outputs)                │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## 9. Command Flow

### `/speckit.auto SPEC-ID` Execution

```
USER INPUT: /speckit.auto SPEC-KIT-900
  │
  ↓
[pipeline_coordinator.rs:28] handle_spec_auto()
  ├─ Validate config
  ├─ Check evidence size (<50MB)
  └─ Create SpecAutoState(spec_id, stages[Plan, Tasks, Implement, Validate, Audit, Unlock])
  │
  ↓
[pipeline_coordinator.rs:96] advance_spec_auto()
  │
  FOR EACH stage:
  │
  ├─ STEP 1: Guardrail Check
  │   ├─ [native_guardrail.rs:75] run_native_guardrail()
  │   ├─ Checks: spec-id, files, clean-tree, stage-ready
  │   └─ Pass → Continue | Fail → Halt
  │
  ├─ STEP 2: Quality Gate (if applicable)
  │   ├─ before-specify, after-specify, after-tasks
  │   ├─ [quality_gate_handler.rs] spawn_quality_gate_agents_native()
  │   │   └─ 3 agents with INDIVIDUAL prompts
  │   └─ Pass (2/3 or 3/3) → Continue | Fail → Halt
  │
  ├─ STEP 3: Regular Stage Execution
  │   ├─ [agent_orchestrator.rs] auto_submit_spec_stage_prompt()
  │   ├─ Spawn 3-4 agents in parallel
  │   ├─ Poll every 500ms (10min timeout)
  │   ├─ [agent_orchestrator.rs:646] on_spec_auto_agents_complete()
  │   └─ [pipeline_coordinator.rs:531] check_consensus_and_advance()
  │
  └─ REPEAT until all stages complete
  │
  ↓
OUTPUT: plan.md, tasks.md, implement.md, etc.
```

## 10. Synthesis System

### Agent Roster by Tier

| Tier   | Agents               | Use Case                    |
| ------ | -------------------- | --------------------------- |
| Tier 2 | gemini, claude, code | Quality gates, basic stages |
| Tier 3 | + gpt\_codex         | Implementation stages       |
| Tier 4 | Dynamic 3-5 agents   | Complex synthesis           |

### Synthesis Quorum

| Condition           | Result                                     |
| ------------------- | ------------------------------------------ |
| 3/3 agents complete | Synthesis OK, advance                      |
| 2/3 agents complete | Degraded quorum, advance with warning      |
| Conflict detected   | Halt, require resolution                   |
| Agent timeout       | Retry (AR-2), then continue with remaining |

### Synthesis Metadata

Stored automatically in local-memory:

* `agent`, `version`, `content` per agent
* `consensus_ok`, `degraded`, `missing_agents`, `conflicts[]` in synthesis

***

# Appendices

## A. Developer Guidelines

### When to Use block\_on()

**Good Use Cases**:

* User-initiated commands (`/speckit.*`)
* Infrequent operations (<10/min)
* Operations that must complete before proceeding
* Operations with visible progress

**Bad Use Cases**:

* Tight loops (event poll, render loop)
* Background tasks (use `tokio::spawn` instead)
* Nested async functions (deadlock risk)
* Time-critical operations (<10ms target)

### Code Pattern

```rust
// GOOD: Sync handler blocking on async
fn handle_user_command() {
    let result = tokio::runtime::Handle::current()
        .block_on(async_operation())?;
    display_result(result);
}

// BAD: Blocking in async function (deadlock risk)
async fn foo() {
    Handle::current().block_on(async { /* DEADLOCK */ });
}
```

### Performance Profiling

```bash
# Check consensus timing
grep "Consensus completed" ~/.code/logs/codex-tui.log

# Tokio console (advanced)
RUSTFLAGS="--cfg tokio_unstable" cargo run
tokio-console

# Flamegraph
cargo flamegraph --bin code
```

## B. Related Documentation

* **OPERATIONS.md** - How to run, troubleshoot, validate
* **CONTRIBUTING.md** - Fork workflow, rebase strategy
* **POLICY.md** - Model policy, gate policy, evidence policy
* **CLAUDE.md** - Build commands, project structure

## C. Change History

| Version | Date       | Changes                                                                                                                                             |
| ------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1.0.0   | 2026-01-21 | Consolidated from ARCHITECTURE.md, TUI.md (arch sections), async-sync-boundaries.md, chatwidget-structure.md, SPEC-KIT-900-ARCHITECTURE-ANALYSIS.md |

***

*Last Updated: 2026-01-21*
