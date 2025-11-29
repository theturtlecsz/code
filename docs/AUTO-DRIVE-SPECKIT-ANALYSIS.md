# Auto Drive Applicability Analysis for /speckit.auto

**Generated**: 2025-11-29
**Purpose**: Deep dive on Auto Drive patterns applicable to spec-kit pipeline
**Session**: P6 SYNC (pre-implementation analysis)

---

## Executive Summary

Auto Drive is upstream's autonomous code generation pipeline. While `/speckit.auto` is a different beast (multi-agent consensus vs single-model turns), several Auto Drive patterns are **highly applicable**:

| Pattern | Auto Drive | spec-kit Current | Applicability | Priority |
|---------|-----------|------------------|---------------|----------|
| **Decision Sequencing** | Exactly-once via seq/ACK | None | HIGH | P1 |
| **SessionMetrics** | Token/cost tracking | Partial (cost_tracker) | HIGH | P2 |
| **State Machine** | AutoRunPhase enum | SpecAutoPhase enum | MEDIUM | - |
| **Fault Injection** | dev-faults feature | None | HIGH | P3 |
| **Retry with Backoff** | retry.rs | P5-ported (strategy.rs) | DONE | - |
| **Cancellation Support** | CancellationToken | Basic (spec_auto_cancel) | MEDIUM | P4 |
| **History/Compaction** | AutoDriveHistory | None | LOW | - |

**Recommendation**: Port Decision Sequencing (P1), SessionMetrics (P2), and Fault Injection (P3) in P6/P7.

---

## Architecture Comparison

### Auto Drive (Upstream)

```
┌─────────────────────────────────────────────────────────────┐
│ AutoCoordinatorHandle                                        │
│   ├─ tx: Sender<AutoCoordinatorCommand>                     │
│   └─ cancel_token: CancellationToken                        │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ AutoCoordinator (113KB)                                      │
│   ├─ Decision sequencing (seq/ACK)                          │
│   ├─ Turn management (TurnMode, TurnComplexity)             │
│   ├─ Agent actions (CLI + sub-agents)                       │
│   ├─ Session metrics                                        │
│   └─ Auto-compaction                                        │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ AutoDriveController (35KB)                                   │
│   ├─ AutoRunPhase state machine                             │
│   ├─ Phase transitions                                      │
│   ├─ Review/diagnostics gates                               │
│   └─ Restart handling                                       │
└─────────────────────────────────────────────────────────────┘
```

### spec-kit /speckit.auto (Fork)

```
┌─────────────────────────────────────────────────────────────┐
│ ChatWidget.spec_auto_state                                   │
│   ├─ SpecAutoState (run_id, stages, current_stage_index)   │
│   └─ PipelineConfig (SPEC-948)                              │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ pipeline_coordinator.rs                                      │
│   ├─ handle_spec_auto() - initiation                        │
│   ├─ advance_spec_auto() - state machine                    │
│   └─ on_spec_auto_task_* - lifecycle hooks                  │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ consensus_coordinator.rs + agent_orchestrator.rs             │
│   ├─ run_consensus_with_retry() - MCP consensus             │
│   ├─ auto_submit_spec_stage_prompt() - agent dispatch       │
│   └─ Multi-agent coordination                               │
└─────────────────────────────────────────────────────────────┘
```

---

## Pattern Analysis

### 1. Decision Sequencing (HIGH PRIORITY)

**Auto Drive Implementation** (`auto_coordinator.rs:129-236`):

```rust
pub enum AutoCoordinatorEvent {
    Decision {
        seq: u64,  // Monotonically increasing sequence number
        status: AutoCoordinatorStatus,
        // ... other fields
    },
    // ...
    StopAck,  // Acknowledgment of stop
}

pub enum AutoCoordinatorCommand {
    AckDecision { seq: u64 },  // Acknowledge processed decision
    // ...
}

struct PendingDecision {
    seq: u64,
    // ... decision payload
}
```

**Why This Matters for spec-kit**:
- Multiple agents return responses that could arrive out of order
- Retry logic can produce duplicate responses
- Without sequencing, same agent response could be processed twice
- No guarantee consensus is evaluated in correct order

**Current spec-kit Gap**:
- No sequence numbers on agent responses
- No ACK mechanism before processing next response
- Duplicate detection relies on content hashing (fragile)

**Proposed Port** (P6 Phase 2):
```rust
// consensus_coordinator.rs additions
pub struct ConsensusSequence {
    decision_seq: AtomicU64,      // Next sequence to assign
    pending_ack_seq: AtomicU64,   // Awaiting acknowledgment
}

pub enum ConsensusEvent {
    AgentResponse {
        seq: u64,
        agent_id: String,
        content: String,
    },
    AckResponse { seq: u64 },
}
```

**Estimated Effort**: 4-6 hours
**Files Affected**: `consensus_coordinator.rs`, `agent_orchestrator.rs`

---

### 2. SessionMetrics (HIGH PRIORITY)

**Auto Drive Implementation** (`session_metrics.rs:1-125`):

```rust
pub struct SessionMetrics {
    running_total: TokenUsage,           // Cumulative tokens
    last_turn: TokenUsage,               // Most recent turn
    turn_count: u32,                     // Pipeline turn counter
    replay_updates: u32,                 // Replay/retry counter
    duplicate_items: u32,                // Dedup tracking
    recent_prompt_tokens: VecDeque<u64>, // Sliding window for prediction
    window: usize,
}

impl SessionMetrics {
    pub fn record_turn(&mut self, usage: &TokenUsage);
    pub fn estimated_next_prompt_tokens(&self) -> u64;  // Cost prediction!
    pub fn record_duplicate_items(&mut self, count: usize);
}
```

**Why This Matters for spec-kit**:
- `/speckit.auto` can cost $2.70+ per run
- No current way to estimate cost before running pipeline
- No visibility into token distribution across agents
- Duplicate detection not tracked centrally

**Current spec-kit State**:
- `cost_tracker.rs` exists but is stage-focused, not session-level
- No sliding window for prompt prediction
- No duplicate counting

**Proposed Port** (P6 Phase 3):
```rust
// New: spec-kit/src/telemetry/session_metrics.rs
pub struct PipelineMetrics {
    running_total: TokenUsage,
    per_agent: HashMap<String, TokenUsage>,
    duplicate_responses: u32,
    retry_count: u32,
    recent_prompts: VecDeque<u64>,
}

impl PipelineMetrics {
    pub fn estimated_cost(&self) -> f64;  // Pre-run cost estimate
    pub fn agent_breakdown(&self) -> Vec<(String, TokenUsage)>;
}
```

**Estimated Effort**: 2-3 hours
**Files Affected**: New `session_metrics.rs`, `pipeline_coordinator.rs`, `agent_orchestrator.rs`

---

### 3. Fault Injection (HIGH PRIORITY)

**Auto Drive Implementation** (`faults.rs:1-158`):

```rust
#![cfg(feature = "dev-faults")]

pub enum FaultScope {
    AutoDrive,
}

pub enum InjectedFault {
    Disconnect,
    RateLimit { reset_hint: Option<FaultReset> },
}

// Environment-driven: CODEX_FAULTS=disconnect:3,429:1
pub fn next_fault(scope: FaultScope) -> Option<InjectedFault>;
pub fn fault_to_error(fault: InjectedFault) -> anyhow::Error;
```

**Why This Matters for spec-kit**:
- No way to test retry paths without hitting real failures
- Can't verify agent failure handling deterministically
- Integration tests are flaky due to real API behavior
- Need to validate quality gate recovery paths

**Current spec-kit State**:
- `agent_retry.rs` has retry logic but no test injection
- Tests mock at high level, not at failure point
- No way to test "agent 2 of 3 times out" scenario

**Proposed Port** (P6 Phase 4):
```rust
// New: spec-kit/src/testing/faults.rs
#![cfg(feature = "dev-faults")]

pub enum FaultScope {
    SpecKit,
    Consensus,
    Agent(String),  // Per-agent faults
}

pub enum InjectedFault {
    AgentTimeout,
    ConsensusFailure,
    RateLimited { reset_hint: Option<Duration> },
    EmptyResponse,
}

// SPEC_KIT_FAULTS=agent_timeout:3,consensus_fail:1
// SPEC_KIT_FAULTS_AGENT=gemini-flash  // Target specific agent
```

**Estimated Effort**: 3-4 hours
**Files Affected**: New `faults.rs`, `Cargo.toml` (feature flag), `agent_retry.rs`

---

### 4. State Machine Patterns (MEDIUM PRIORITY)

**Auto Drive** (`controller.rs:63-165`):

```rust
pub enum AutoRunPhase {
    Idle,
    AwaitingGoalEntry,
    Launching,
    Active,
    PausedManual { resume_after_submit: bool, bypass_next_submit: bool },
    AwaitingCoordinator { prompt_ready: bool },
    AwaitingDiagnostics { coordinator_waiting: bool },
    AwaitingReview { diagnostics_pending: bool },
    TransientRecovery { backoff_ms: u64 },
}

pub enum PhaseTransition {
    BeginLaunch,
    LaunchSuccess,
    LaunchFailed,
    PauseForManualEdit { resume_after_submit: bool },
    TransientFailure { backoff_ms: u64 },
    RecoveryAttempt,
    Stop,
}
```

**Current spec-kit State** (`state.rs`):
```rust
pub enum SpecAutoPhase {
    Ready,
    Running,
    PausedForReview,
    CompletedSuccess,
    CompletedFailure,
    Cancelled,
}
```

**Analysis**:
- spec-kit's state machine is simpler (6 states vs 9)
- Auto Drive has richer "paused" states with metadata
- TransientRecovery state is valuable for retry visibility
- Payload states (diagnostics_pending, prompt_ready) useful for UI

**Recommendation**: Consider adding `TransientRecovery` and payload states, but not high priority. Current state machine is adequate.

---

### 5. Cancellation Support (MEDIUM PRIORITY)

**Auto Drive**:
```rust
// Uses tokio_util::sync::CancellationToken throughout
pub struct AutoCoordinatorHandle {
    pub tx: Sender<AutoCoordinatorCommand>,
    cancel_token: CancellationToken,
}

// In retry.rs
async fn wait_with_cancel(cancel: &CancellationToken, duration: Duration)
    -> Result<(), RetryError>
```

**Current spec-kit State**:
- Basic cancellation via `spec_auto_cancel` flag
- Not integrated with retry loops
- No graceful cleanup on cancel during agent spawn

**Proposed Enhancement**:
```rust
// Add to pipeline_coordinator.rs
pub struct PipelineHandle {
    cancel_token: CancellationToken,
    status_rx: Receiver<PipelineStatus>,
}

// Integrate with retry
pub async fn spawn_agent_with_retry_cancellable(
    agent_name: &str,
    operation: F,
    cancel: &CancellationToken,
) -> Result<...>
```

**Estimated Effort**: 2 hours
**Priority**: After core patterns (P4)

---

### 6. History/Compaction (LOW PRIORITY)

**Auto Drive** (`auto_compact.rs`, `auto_drive_history.rs`):
- Tracks conversation history for long runs
- Auto-compacts when context window fills
- Checkpoint summaries for recovery

**spec-kit Analysis**:
- Pipeline runs are bounded (6 stages max)
- No long-running conversation to compact
- History is stage-based, not turn-based
- Not applicable to current architecture

**Recommendation**: Skip - architectural mismatch.

---

## Patterns Already Ported (P5)

### Retry with Backoff

**P5 SYNC completed**:
- `codex-rs/spec-kit/src/retry/strategy.rs` - Core retry logic
- `codex-rs/spec-kit/src/retry/classifier.rs` - Error classification
- `codex-rs/spec-kit/src/retry/circuit_breaker.rs` - Circuit breaker (basic)

**Auto Drive Additions Not Yet Ported**:
- `RetryStatus` callbacks (partially done)
- Rate limit parsing with `reset_at` header
- Total elapsed timeout (added in P5, needs testing)

---

## Implementation Roadmap

### P6 Session (This Session)

| Phase | Pattern | Est. Hours | Files |
|-------|---------|------------|-------|
| 1 | Auth Diff Report | 2-3h | DONE |
| 2 | Decision Sequencing | 4-6h | consensus_coordinator.rs |
| 3 | SessionMetrics | 2-3h | New session_metrics.rs |
| 4 | Fault Injection | 3-4h | New faults.rs |
| 5 | Branch-aware Resume | 2-3h | session/ |

**Total P6 Estimate**: 13-19 hours (matches original plan)

### P7 Session (Future)

| Pattern | Est. Hours | Notes |
|---------|------------|-------|
| Cancellation Integration | 2h | Wire CancellationToken through pipeline |
| Enhanced State Machine | 2h | Add TransientRecovery, payload states |
| SYNC-016 Implementation | 3h | Device Code Auth (now unblocked) |
| SessionMetrics UI | 2h | `/speckit.status` cost estimates |

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Decision sequencing breaks existing consensus | Feature flag, extensive testing |
| SessionMetrics overhead | Benchmark, make optional |
| Fault injection leaks to prod | Strict `#[cfg(feature = "dev-faults")]` |
| Breaking changes to pipeline | Preserve existing API, add new fields |

---

## Key Takeaways

1. **Auto Drive is turn-based, spec-kit is stage-based** - Not a 1:1 port, but patterns transfer

2. **Decision Sequencing is critical** - Current lack of sequencing could cause duplicate processing in multi-agent consensus

3. **SessionMetrics enables cost prediction** - Currently flying blind on costs until pipeline completes

4. **Fault Injection unlocks testing** - Can't test retry paths without it; makes tests deterministic

5. **Skip History/Compaction** - Architectural mismatch; spec-kit doesn't have long conversations

6. **Cancellation is polish** - Current basic cancellation works; enhancement is nice-to-have

---

## Appendix: File Sizes for Context

### Auto Drive Core
| File | Size | LOC (est) |
|------|------|-----------|
| auto_coordinator.rs | 113KB | ~3000 |
| controller.rs | 35KB | ~900 |
| auto_compact.rs | 25KB | ~650 |
| auto_drive_history.rs | 19KB | ~500 |
| retry.rs | 6KB | ~170 |
| session_metrics.rs | 5KB | ~180 |
| faults.rs | 6KB | ~160 |

### spec-kit (relevant files)
| File | Size | LOC (est) |
|------|------|-----------|
| pipeline_coordinator.rs | - | ~400 |
| consensus_coordinator.rs | - | ~200 |
| agent_orchestrator.rs | - | ~300 |
| agent_retry.rs | - | ~280 |
| state.rs | - | ~150 |
| cost_tracker.rs | - | ~200 |

---

*Analysis complete. Ready for new session to begin implementation.*
