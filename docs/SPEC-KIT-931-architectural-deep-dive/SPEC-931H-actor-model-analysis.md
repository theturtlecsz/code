# SPEC-931H: Actor Model Feasibility Analysis

**Date**: 2025-11-13
**Status**: ULTRATHINK ANALYSIS IN PROGRESS
**Parent**: SPEC-931 Architectural Deep Dive
**Cross-References**: SPEC-931F (Event Sourcing NO-GO), SPEC-931G (Testing Gaps), SPEC-931E (Ratatui Constraints)

---

## Executive Summary

**Research Question**: Can the actor model pattern solve current orchestration problems (dual-write, concurrency, crash recovery)?

**Answer**: **PARTIALLY** - Actors improve code organization and crash isolation but **do NOT eliminate fundamental problems**:
- ❌ **Dual-write still exists** (Supervisor state + AGENT_MANAGER cache + SQLite) - same as event sourcing
- ✅ **Ratatui compatible** (tokio::select! officially supported)
- ✅ **Better isolation** (agents crash independently, supervisor coordinates)
- ❌ **Doesn't solve storage complexity** (still need 3 systems: actors, cache, persistence)
- ⏳ **Migration effort significant** (~1,100 LOC, 3-5 days)

**Recommendation**: **NO-GO for Phase 1** - Actors are a refactoring opportunity, not a solution to core problems. Prioritize simpler fixes (ACID transactions from SPEC-931F, testing from SPEC-931G).

---

## 1. Current Architecture (Baseline)

### 1.1 Agent Orchestration Flow

```
[TUI] --spawn--> [native_quality_gate_orchestrator.rs]
                       |
                       +--[AGENT_MANAGER.create_agent()]
                       |      |
                       |      +--[tokio::spawn(execute_agent)]
                       |
                       +--[wait_for_quality_gate_agents]  (polling loop, 500ms)
                       |
                       +--[quality_gate_broker.collect_results]
                       |      |
                       |      +--[scan filesystem for result.txt]
                       |      +--[read AGENT_MANAGER.result]
                       |
                       +--[apply_consensus_resolution]
                       |
                       +--[AGENT_MANAGER.update_result] + [SQLite.store]
```

**Problems**:
1. **Dual-write**: AGENT_MANAGER (HashMap) + SQLite, no transaction coordination
2. **Lock contention**: RwLock on AGENT_MANAGER for concurrent access
3. **No crash recovery**: Agent crash leaves inconsistent state
4. **Complex polling**: 500ms loop, check each agent status
5. **Filesystem dependency**: Broker scans `.code/agents/` for results

### 1.2 TUI Event Loop (Current)

```rust
// codex-rs/tui/src/app.rs:1012
pub(crate) fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
    'main: loop {
        let event = match self.next_event_priority() {  // SYNC POLLING
            Some(e) => e,
            None => break 'main,
        };
        match event {
            AppEvent::SpecKitQualityGateResults { broker_result } => { ... }
            AppEvent::RequestRedraw => { self.schedule_redraw(); }
            // ... 50+ event types
        }
    }
}
```

**Characteristics**:
- **Synchronous polling**: `next_event_priority()` blocks on channel recv
- **Event priority**: High (input) vs Bulk (streaming) receivers
- **33ms debounce**: Redraw scheduled, not immediate (30 FPS)
- **mpsc channels**: app_event_tx → app_event_rx (unbounded)

---

## 2. Actor Model Design (Proposed)

### 2.1 Actor Hierarchy

```
SupervisorActor (long-lived)
    |
    +--AgentActor(gemini)  (short-lived, per-execution)
    +--AgentActor(claude)
    +--AgentActor(code)
```

### 2.2 Message Protocol

```rust
// === Supervisor Messages ===

/// Commands sent TO the supervisor
#[derive(Debug, Clone)]
pub enum SupervisorCommand {
    /// Spawn 3 agents for quality gate
    SpawnQualityGate {
        spec_id: String,
        checkpoint: QualityCheckpoint,
        run_id: Option<String>,
    },
    /// Cancel a specific agent
    CancelAgent {
        agent_id: String,
    },
    /// Query current status
    QueryStatus {
        agent_id: String,
        response_tx: oneshot::Sender<AgentStatus>,
    },
    /// Shutdown supervisor gracefully
    Shutdown,
}

/// Messages FROM agents TO supervisor
#[derive(Debug)]
pub enum AgentMessage {
    Started {
        agent_id: String,
        model: String,
    },
    Progress {
        agent_id: String,
        message: String,
    },
    Completed {
        agent_id: String,
        result: String,
    },
    Failed {
        agent_id: String,
        error: String,
    },
}

/// Messages FROM supervisor TO TUI
#[derive(Debug, Clone)]
pub enum TUIUpdate {
    /// Existing AppEvent variant
    AgentStatusUpdate {
        agents: Vec<AgentInfo>,
    },
    /// Quality gate completed
    QualityGateResult {
        spec_id: String,
        consensus: ConsensusResult,
    },
}
```

### 2.3 Supervisor Actor Implementation

```rust
pub struct SupervisorActor {
    /// Receive commands from TUI
    cmd_rx: mpsc::UnboundedReceiver<SupervisorCommand>,
    /// Receive messages from agent actors
    agent_rx: mpsc::UnboundedReceiver<AgentMessage>,
    /// Send updates to TUI
    tui_tx: mpsc::UnboundedSender<AppEvent>,
    /// Track active agents
    active_agents: HashMap<String, AgentHandle>,
    /// Agent configuration
    agent_configs: Vec<AgentConfig>,
}

struct AgentHandle {
    agent_id: String,
    model: String,
    status: AgentStatus,
    join_handle: tokio::task::JoinHandle<()>,
}

impl SupervisorActor {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                // Process commands from TUI
                Some(cmd) = self.cmd_rx.recv() => {
                    self.handle_command(cmd).await;
                }
                // Process messages from agents
                Some(msg) = self.agent_rx.recv() => {
                    self.handle_agent_message(msg).await;
                }
                // Poll for completed agents (detect crashes)
                _ = self.poll_agent_completion() => {
                    // Detect panics, update status
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: SupervisorCommand) {
        match cmd {
            SupervisorCommand::SpawnQualityGate { spec_id, checkpoint, run_id } => {
                // Spawn 3 agent actors
                for (agent_name, config_name) in [("gemini", "gemini_flash"), ...] {
                    let agent_id = Uuid::new_v4().to_string();
                    let agent_tx = self.agent_rx.clone();

                    let handle = tokio::spawn(async move {
                        AgentActor::run(agent_id, config_name, prompt, agent_tx).await;
                    });

                    self.active_agents.insert(agent_id, AgentHandle { ... });
                }
            }
            SupervisorCommand::CancelAgent { agent_id } => {
                if let Some(handle) = self.active_agents.get(&agent_id) {
                    handle.join_handle.abort();
                }
            }
            SupervisorCommand::Shutdown => {
                // Cancel all active agents, exit loop
                break;
            }
        }
    }

    async fn handle_agent_message(&mut self, msg: AgentMessage) {
        match msg {
            AgentMessage::Completed { agent_id, result } => {
                // Update supervisor state
                if let Some(agent) = self.active_agents.get_mut(&agent_id) {
                    agent.status = AgentStatus::Completed;
                }

                // Update AGENT_MANAGER cache (for TUI reads)
                let mut manager = AGENT_MANAGER.write().await;
                manager.update_agent_result(&agent_id, Ok(result.clone())).await;
                drop(manager);

                // Store to SQLite (persistence)
                if let Ok(db) = ConsensusDb::init_default() {
                    let _ = db.record_agent_completion(&agent_id, &result);
                }

                // Send TUI update
                let _ = self.tui_tx.send(AppEvent::AgentStatusUpdate { ... });

                // Check if all 3 agents complete → apply consensus
                if self.all_agents_complete() {
                    let consensus = self.apply_consensus();
                    let _ = self.tui_tx.send(AppEvent::QualityGateResult { ... });
                }
            }
            AgentMessage::Failed { agent_id, error } => {
                // Similar to Completed, but mark as Failed
                // Consensus can proceed with 2/3 agents
            }
            _ => { /* Progress, Started */ }
        }
    }
}
```

### 2.4 Agent Actor Implementation

```rust
pub struct AgentActor {
    agent_id: String,
    config_name: String,
    prompt: String,
    supervisor_tx: mpsc::UnboundedSender<AgentMessage>,
}

impl AgentActor {
    pub async fn run(
        agent_id: String,
        config_name: String,
        prompt: String,
        supervisor_tx: mpsc::UnboundedSender<AgentMessage>,
    ) {
        // Notify supervisor we started
        let _ = supervisor_tx.send(AgentMessage::Started {
            agent_id: agent_id.clone(),
            model: config_name.clone(),
        });

        // Execute agent (existing execute_agent logic)
        let result = execute_agent_internal(config_name, prompt).await;

        // Send result to supervisor
        match result {
            Ok(output) => {
                let _ = supervisor_tx.send(AgentMessage::Completed {
                    agent_id,
                    result: output,
                });
            }
            Err(error) => {
                let _ = supervisor_tx.send(AgentMessage::Failed {
                    agent_id,
                    error,
                });
            }
        }
    }
}
```

### 2.5 TUI Integration (tokio::select! Event Loop)

```rust
// NEW: codex-rs/tui/src/app.rs
pub(crate) async fn run_async(&mut self, terminal: &mut tui::Tui) -> Result<()> {
    // Create supervisor
    let (supervisor_tx, supervisor_rx) = mpsc::unbounded_channel();
    let (agent_tx, agent_rx) = mpsc::unbounded_channel();

    let supervisor = SupervisorActor {
        cmd_rx: supervisor_rx,
        agent_rx,
        tui_tx: self.app_event_tx.clone(),
        active_agents: HashMap::new(),
        agent_configs: self.config.agents.clone(),
    };

    // Spawn supervisor in background
    let supervisor_handle = tokio::spawn(supervisor.run());

    // Event loop with tokio::select!
    let mut tick_interval = tokio::time::interval(Duration::from_millis(33)); // 30 FPS
    let mut event_reader = EventStream::new();

    loop {
        tokio::select! {
            // High-priority events (keyboard input)
            Some(event) = self.app_event_rx_high.recv() => {
                self.handle_event(event, terminal).await?;
            }
            // Bulk events (streaming)
            Some(event) = self.app_event_rx_bulk.recv() => {
                self.handle_event(event, terminal).await?;
            }
            // Periodic redraw tick
            _ = tick_interval.tick() => {
                if self.pending_redraw.load(Ordering::Acquire) {
                    self.draw_next_frame(terminal)?;
                }
            }
            // Crossterm keyboard/mouse events
            Some(Ok(crossterm_event)) = event_reader.next() => {
                self.handle_crossterm_event(crossterm_event)?;
            }
        }
    }
}
```

---

## 3. Comparison: Current vs Actor Model

| Aspect | Current | Actor Model | Winner |
|--------|---------|-------------|--------|
| **Agent Isolation** | Shared AGENT_MANAGER HashMap | Isolated actor tasks | ✅ Actor |
| **Crash Recovery** | No mechanism, state lost | Supervisor detects crash, updates status | ✅ Actor |
| **Concurrency** | RwLock contention | Message passing, no locks | ✅ Actor |
| **Storage Systems** | 3 systems (HashMap, SQLite, Filesystem) | 3 systems (Supervisor, AGENT_MANAGER cache, SQLite) | ⚖️ Same |
| **Dual-write Problem** | HashMap + SQLite, no ACID | Supervisor + AGENT_MANAGER + SQLite, no ACID | ⚖️ Same |
| **TUI Read Performance** | Direct HashMap access (~μs) | AGENT_MANAGER cache (~μs) | ⚖️ Same |
| **Code Complexity** | 300 LOC orchestrator | 600 LOC (supervisor + agent + messages) | ❌ Current |
| **Testability** | Mock AGENT_MANAGER, filesystem | Mock actors, message streams | ✅ Actor |
| **Observability** | Logs + telemetry | Message tracing + actor lifecycle | ✅ Actor |
| **Migration Effort** | N/A | ~1,100 LOC, 3-5 days | ❌ Current |

**Conclusion**: Actors improve isolation, testability, observability. They do NOT solve storage complexity or dual-write. Trade-off: more code for better structure.

---

## 4. Critical Findings

### 4.1 Dual-Write NOT Eliminated (Same as SPEC-931F)

**Claim (SPEC-930)**: "Actor model simplifies state management"

**Reality**: Actors reorganize state, don't eliminate storage systems.

**Evidence**:
1. **Supervisor state** (in-memory): Agent status, join handles, run state
2. **AGENT_MANAGER cache** (HashMap): TUI needs sync read access for rendering (60 FPS)
3. **SQLite persistence**: Survive crash, historical data, routing

**Why AGENT_MANAGER still needed**:
- TUI rendering loop: 60 FPS (16ms per frame)
- Actor query latency: ~milliseconds (message send → supervisor process → respond)
- Can't await in render path: Ratatui rendering is sync (draw() not async)
- Solution: AGENT_MANAGER = **read cache**, supervisor updates asynchronously

**Pattern**: Actor Supervisor = source of truth, AGENT_MANAGER = denormalized TUI projection

### 4.2 Ratatui Async Compatibility (VALIDATED)

**Claim (SPEC-930)**: "Ratatui async is complex"

**Reality**: Ratatui **officially supports** async via tokio::select!

**Evidence**:
- Tutorial: https://ratatui.rs/tutorials/counter-async-app/full-async-events/
- Template: https://github.com/ratatui/async-template (production-ready)
- Pattern: `tokio::select! { event = rx.recv() => ..., _ = tick.tick() => ... }`

**Current codex-rs**: Sync polling loop (`next_event_priority()`) with mpsc channels

**Migration**: Rewrite run() to async, add tokio::select! (~150 LOC)

**No blocker**: Ratatui is fully compatible with actor model

### 4.3 Migration Effort: 3-5 Days (~1,100 LOC)

**Component Breakdown**:
1. **SupervisorActor** (~300 LOC): Message handling, agent lifecycle, consensus
2. **AgentActor wrapper** (~200 LOC): Execute agent, send messages
3. **Message types** (~100 LOC): Enums, structs for protocol
4. **TUI integration** (~150 LOC): Convert run() to tokio::select!, add supervisor channel
5. **AGENT_MANAGER refactor** (~100 LOC): Make it a cache, supervisor updates
6. **SQLite integration** (~50 LOC): Store from supervisor, not agents
7. **Integration tests** (~200 LOC): Mock actors, crash scenarios, message protocol

**Time Estimate**: 3-5 days (assuming no blockers, 4-6 hours/day)

**Risk**: Medium (async debugging, message protocol bugs, state synchronization)

### 4.4 Does NOT Solve SPEC-928 Bugs

**SPEC-928 Regressions**:
1. Tmux stdout mixing (UTF-8 panic, output corruption)
2. Schema template false positive in validation
3. Concurrent agent execution detection

**Actor Model Impact**:
- ✅ **Concurrent execution**: Easier to detect (supervisor knows all active agents)
- ❌ **Tmux stdout**: Actors don't change tmux execution (still used for observability)
- ❌ **Validation logic**: Actors don't change JSON validation (same code)

**Conclusion**: Actors improve observability, don't fix root causes (tmux, validation logic)

---

## 5. Decision Matrix

### 5.1 GO Criteria

- [ ] **Solves dual-write** (AGENT_MANAGER + SQLite) → ❌ **NO** (same 3 systems)
- [ ] **Eliminates AGENT_MANAGER** → ❌ **NO** (still needed as TUI cache)
- [x] **Improves crash isolation** → ✅ **YES** (supervisor detects, agents isolated)
- [x] **Ratatui compatible** → ✅ **YES** (tokio::select! supported)
- [ ] **Simpler than current** → ❌ **NO** (600 LOC actors vs 300 LOC orchestrator)
- [x] **Better testability** → ✅ **YES** (mock actors, message protocol)
- [ ] **Worth 3-5 days effort** → ⏳ **MAYBE** (depends on priorities)

### 5.2 NO-GO Criteria

- [x] **Doesn't solve core problems** (dual-write, storage complexity) → ❌ **FAIL**
- [x] **Adds complexity** (message protocol, supervisor state) → ❌ **FAIL**
- [ ] **Blocks simpler solutions** (ACID transactions, testing) → ⏳ **UNKNOWN**
- [ ] **Ratatui incompatible** → ✅ **PASS** (compatible)

### 5.3 Recommendation: **NO-GO for Phase 1**

**Rationale**:
1. **Doesn't solve dual-write** (same problem as event sourcing SPEC-931F)
2. **Doesn't eliminate AGENT_MANAGER** (TUI needs sync cache)
3. **Adds code complexity** (600 LOC actors vs 300 LOC orchestrator)
4. **Better alternatives exist**:
   - ACID transactions (SPEC-931F): 2-3 days, solves actual dual-write
   - Testing improvements (SPEC-931G): Add transaction tests, concurrency tests
   - Observability: Add logging, telemetry (cheaper than actors)

**When to reconsider**:
- After ACID transactions implemented (Phase 2)
- If actor pattern solves other problems (rate limiting, queueing)
- If supervisor pattern enables features (circuit breakers, retries)

**Alternative**: **Defer to Phase 2** (refactoring opportunity, not urgent fix)

---

## 6. Cross-References

### 6.1 SPEC-931F (Event Sourcing): Same Root Cause

**Finding**: Event sourcing doesn't eliminate dual-write (AGENT_MANAGER + event_log)
**Reason**: TUI needs sync read access for rendering
**Pattern**: Event log = source of truth, AGENT_MANAGER = cache

**Parallel**: Actor model has same pattern
- Supervisor = source of truth (in-memory)
- AGENT_MANAGER = TUI cache (sync reads)
- SQLite = persistence

**Lesson**: Storage complexity is **product requirement** (TUI + persistence), not implementation artifact

### 6.2 SPEC-931G (Testing): Actors Help Testing

**Current Gaps**:
- No real concurrency tests
- No transaction tests
- No crash recovery tests

**Actor Model Benefits**:
- **MockAgentActor**: Simulate crashes, delays, failures
- **Message protocol**: Test supervisor logic without real agents
- **Isolated state**: No global HashMap, easier to reset

**Recommendation**: Write concurrency tests NOW (without actors) → easier migration later

### 6.3 SPEC-931E (Ratatui): No Async Blocker

**Constraint**: Ratatui rendering is synchronous (33ms/30 FPS)
**Concern**: Can't use tokio::select! with Ratatui

**Finding**: **FALSE** - Ratatui async tutorial uses tokio::select!
**Pattern**: Async event loop + sync rendering (draw called from async context)

**Implication**: Actor model is Ratatui-compatible (no impedance mismatch)

---

## 7. Open Questions (Added to MASTER-QUESTIONS.md)

See MASTER-QUESTIONS.md Q172-Q186 for complete list.

**CRITICAL**:
- Q172: Ratatui async compatibility → ✅ ANSWERED (YES, compatible)
- Q176: Does actor model solve dual-write? → ✅ ANSWERED (NO, same as event sourcing)
- Q180: Can we eliminate AGENT_MANAGER? → ✅ ANSWERED (NO, TUI needs cache)

**HIGH**:
- Q177: Supervisor pattern design → ⏳ PARTIAL (sketched in this doc, need validation)
- Q178: Message types → ⏳ PARTIAL (protocol defined, need review)
- Q179: Crash recovery → ❌ UNANSWERED (restart policy unclear)

**MEDIUM**:
- Q181: Migration complexity → ⏳ PARTIAL (~1,100 LOC, 3-5 days estimate)
- Q182: Test strategy → ❌ UNANSWERED (mock actors, what scenarios?)
- Q185: Observability → ❌ UNANSWERED (message tracing design?)
- Q186: Performance → ❌ UNANSWERED (benchmark needed)

---

## 8. Next Steps (If GO Decision)

### 8.1 Phase 1: Prototype (1 day)

1. **Spike**: Ratatui async template integration
   - Validate tokio::select! with ChatWidget
   - Measure render latency (30 FPS target)
   - Test event priority (high vs bulk)

2. **Minimal supervisor**: Spawn 1 agent, collect result
   - SupervisorActor skeleton
   - AgentActor wrapper
   - Message protocol (Started, Completed, Failed)

3. **Benchmark**: Message passing overhead
   - Spawn 10 agents simultaneously
   - Measure: spawn latency, message latency, throughput

### 8.2 Phase 2: Integration (2 days)

1. **Full supervisor**: 3-agent quality gates
   - Spawn gemini, claude, code
   - Collect results
   - Apply consensus resolution

2. **AGENT_MANAGER integration**: Make it a cache
   - Supervisor updates HashMap on completion
   - TUI reads directly (no await)
   - SQLite stores from supervisor

3. **TUI event loop**: Convert to tokio::select!
   - Merge app_event_rx + supervisor_rx
   - Preserve 33ms debounce
   - Test all 50+ AppEvent types

### 8.3 Phase 3: Testing & Migration (2 days)

1. **Integration tests**:
   - Agent panics (supervisor detects)
   - Supervisor panics (TUI handles)
   - Timeout scenarios
   - Message channel closes

2. **Validation**:
   - Quality gate end-to-end test
   - Concurrent execution (10 gates)
   - TUI responsiveness (30 FPS)

3. **Migration**:
   - Remove old orchestrator
   - Update documentation
   - Add telemetry

---

## 9. Conclusion

**DECISION**: **NO-GO for Phase 1** - Defer actor model to Phase 2 refactoring

**Summary**:
- ✅ **Ratatui compatible**: tokio::select! officially supported
- ❌ **Doesn't solve dual-write**: Same 3 storage systems (Supervisor, AGENT_MANAGER, SQLite)
- ✅ **Better isolation**: Agents crash independently, supervisor coordinates
- ❌ **Adds complexity**: 600 LOC actors vs 300 LOC orchestrator
- ⏳ **Migration effort**: 3-5 days, medium risk

**Recommendation**: Prioritize simpler fixes first (ACID transactions, testing), revisit actors in Phase 2 as refactoring opportunity.

**Alternatives**:
1. **ACID transactions** (SPEC-931F): Wrap HashMap + SQLite updates in transaction (2-3 days)
2. **Testing improvements** (SPEC-931G): Add concurrency tests, transaction tests (1-2 days)
3. **Observability**: Add message tracing, telemetry to current code (1 day)

**Value Proposition**: Actors are **code organization pattern**, not **problem solution**. Fix problems first, refactor later.

---

**Status**: Analysis complete, NO-GO decision recommended
**Next**: SPEC-931I (Storage Consolidation), SPEC-931J (Dead Code Elimination)
