# Spec-Kit Pipeline Architecture Implementation

## Overview

The Spec-Kit pipeline is a 6-stage automated workflow orchestration system that coordinates multi-agent work through a sophisticated state machine. It automates the entire specification-to-implementation lifecycle with quality gates, consensus validation, and intelligent auto-advancement.

**Core Stages (6):**
1. **Plan** - Strategic architecture and work breakdown
2. **Tasks** - Detailed task decomposition  
3. **Implement** - Code generation with validation
4. **Validate** - Test strategy and coverage analysis
5. **Audit** - Security and compliance checking
6. **Unlock** - Final ship/no-ship decision

**Quality Stages (3, integrated):**
- **Clarify** - Ambiguity detection before planning
- **Checklist** - Quality scoring after planning
- **Analyze** - Consistency checking after tasks

---

## State Machine Architecture

### Phase Enumeration

The pipeline state machine has multiple phases (located in `/home/user/code/codex-rs/tui/src/chatwidget/spec_kit/state.rs`):

```rust
pub enum SpecAutoPhase {
    // Guardrail validation phase (always first)
    Guardrail,
    
    // Agent execution phases
    ExecutingAgents {
        expected_agents: Vec<String>,
        completed_agents: HashSet<String>,
    },
    CheckingConsensus,
    
    // Quality gate phases
    QualityGateExecuting {
        checkpoint: QualityCheckpoint,
        gates: Vec<QualityGateType>,
        active_gates: HashSet<QualityGateType>,
        expected_agents: Vec<String>,
        completed_agents: HashSet<String>,
        results: HashMap<String, Value>,
        native_agent_ids: Option<Vec<String>>,
    },
    QualityGateProcessing {
        checkpoint: QualityCheckpoint,
        auto_resolved: Vec<QualityIssue>,
        escalated: Vec<QualityIssue>,
    },
    QualityGateValidating {
        checkpoint: QualityCheckpoint,
        auto_resolved: Vec<QualityIssue>,
        pending_validations: Vec<(QualityIssue, String)>,
        completed_validations: HashMap<usize, GPT5ValidationResult>,
    },
    QualityGateAwaitingHuman {
        checkpoint: QualityCheckpoint,
        escalated_issues: Vec<QualityIssue>,
        escalated_questions: Vec<EscalatedQuestion>,
        answers: HashMap<String, String>,
    },
}
```

### State Container

The main state machine (`SpecAutoState`) contains:

```rust
pub struct SpecAutoState {
    pub spec_id: String,
    pub goal: String,
    pub stages: Vec<SpecStage>,           // [Plan, Tasks, Implement, Validate, Audit, Unlock]
    pub current_index: usize,              // Position in stages array
    pub phase: SpecAutoPhase,              // Current execution phase
    
    // Guardrail tracking
    pub waiting_guardrail: Option<GuardrailWait>,
    pub pending_prompt_summary: Option<String>,
    
    // Quality gate state
    pub quality_gates_enabled: bool,
    pub completed_checkpoints: HashSet<QualityCheckpoint>,
    pub quality_gate_processing: Option<QualityCheckpoint>,
    
    // Execution tracking
    pub execution_logger: Arc<ExecutionLogger>,
    pub run_id: Option<String>,
    pub validate_lifecycle: ValidateLifecycle,
    
    // Agent response cache
    pub agent_responses_cache: Option<Vec<(String, String)>>,
    
    // Cost tracking
    pub cost_recorded_agents: HashMap<SpecStage, HashSet<String>>,
}
```

---

## Pipeline Initiation

**Entry Point:** `handle_spec_auto()` in `pipeline_coordinator.rs` (lines 29-102)

```
/speckit.auto <spec_id> [goal] [--from <stage>]
      ↓
handle_spec_auto()
      ↓
1. Validate configuration (config_validator)
2. Check evidence size (<50MB hard limit)
3. Create SpecAutoState with resume_from stage
4. Initialize ValidateLifecycle
5. Log RunStart event to execution_logger
6. Call advance_spec_auto() to begin
```

### Resume Capability

The pipeline can resume from any stage via the `resume_from` parameter:

```rust
pub fn new(
    spec_id: String,
    goal: String,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
) -> Self {
    let stages = vec![
        SpecStage::Plan,
        SpecStage::Tasks,
        SpecStage::Implement,
        SpecStage::Validate,
        SpecStage::Audit,
        SpecStage::Unlock,
    ];
    let start_index = stages
        .iter()
        .position(|stage| *stage == resume_from)
        .unwrap_or(0);  // Default to Plan if invalid
    
    // state.current_index = start_index
    // Skips earlier stages, starts from specified point
}
```

---

## Stage Advancement Logic

**Core Loop:** `advance_spec_auto()` in `pipeline_coordinator.rs` (lines 105-356)

### Advancement Flow

```
advance_spec_auto()
├─ Check current_index < stages.len()
│  ├─ No: Go to PipelineComplete
│  └─ Yes: Proceed
│
├─ Check if quality gates still running (single-flight guard)
│  └─ If yes: Return (block advancement)
│
├─ Check if quality checkpoint needed before this stage
│  ├─ If yes: Call execute_quality_checkpoint() and return
│  └─ No: Proceed
│
└─ Switch on current phase:
   ├─ Guardrail: 
   │  ├─ Log StageStart event
   │  ├─ Mark waiting_guardrail
   │  └─ NextAction::RunGuardrail { command, args, hal_mode }
   │
   ├─ ExecutingAgents: Return (waiting for agents)
   ├─ CheckingConsensus: Return (waiting for consensus)
   └─ QualityGate*: Return (waiting for gates)

After NextAction determined:
├─ RunGuardrail:
│  └─ Call handle_spec_ops_command() (native guardrail or subprocess)
│
└─ PipelineComplete:
   ├─ Finalize quality gates if enabled
   ├─ Log RunComplete event
   ├─ Generate verification report
   └─ Clear spec_auto_state
```

### Quality Checkpoint Placement

Quality gates are strategically inserted BEFORE stages (not after):

```rust
fn determine_quality_checkpoint(
    stage: SpecStage,
    completed: &HashSet<QualityCheckpoint>,
) -> Option<QualityCheckpoint> {
    let checkpoint = match stage {
        SpecStage::Plan   => QualityCheckpoint::BeforeSpecify,  // Clarify before planning
        SpecStage::Tasks  => QualityCheckpoint::AfterSpecify,   // Checklist after plan
        SpecStage::Implement => QualityCheckpoint::AfterTasks,  // Analyze after tasks
        _ => return None,
    };
    
    if completed.contains(&checkpoint) {
        None  // Already ran this checkpoint
    } else {
        Some(checkpoint)  // Run it now
    }
}
```

This ensures:
- **BeforeSpecify (Clarify):** Runs before Plan to resolve PRD ambiguities early
- **AfterSpecify (Checklist):** Runs before Tasks to validate PRD+plan quality
- **AfterTasks (Analyze):** Runs before Implement to ensure consistency

---

## Guardrail Execution Phase

**Entry Point:** `on_spec_auto_task_complete()` in `pipeline_coordinator.rs` (lines 369-541)

### Guardrail Flow

```
on_spec_auto_task_complete(task_id)
    ↓
1. Extract spec_id and stage from waiting_guardrail
2. Call collect_guardrail_outcome() → GuardrailOutcome
3. If failed:
   - For Validate stage: Record as Failed, halt pipeline
   - For other stages: Cleanup with cancel
4. If succeeded:
   - Run consensus check via MCP (run_consensus_with_retry)
   - If consensus failed: Cleanup with cancel
5. If consensus succeeded:
   - Call auto_submit_spec_stage_prompt(widget, stage)
   - Transition to ExecutingAgents phase
```

### Native Guardrail Path

```
advance_spec_auto_after_native_guardrail(result)
    ↓
Same as on_spec_auto_task_complete() but:
- Takes GuardrailOutcome directly (no file re-read)
- Skips task_id verification
- More efficient for native guardrail sync execution
```

---

## Agent Execution Phase

**Entry Point:** `auto_submit_spec_stage_prompt()` in `agent_orchestrator.rs` (lines 974-1256)

### Agent Submission Workflow

```
auto_submit_spec_stage_prompt(stage, spec_id)
    ↓
1. Pre-fetch ACE playbook bullets (if enabled)
   ├─ Query ACE for stage-specific context
   └─ Cache in state.ace_bullets_cache
2. Build stage prompt with:
   - spec.md content
   - Prior stage outputs (plan.md, tasks.md)
   - ACE bullets injected
3. Determine routing via decide_stage_routing()
4. Apply aggregator_effort (node optimization)
5. For Validate stage: Begin validate_lifecycle run
6. Transition phase to ExecutingAgents
7. Spawn agents for the stage:
   - Plan stage: 1 agent (strategic planning)
   - Tasks stage: 1 agent (decomposition)
   - Implement stage: 2 agents (codex + validator)
   - Validate stage: 3 agents (parallel)
   - Audit stage: 3 agents (parallel)
   - Unlock stage: 3 agents (parallel)
8. Log AgentSpawn events for telemetry
```

### Agent Response Collection

**For Sequential Stages (Plan, Tasks):**
```
on_spec_auto_agents_complete_with_results(agent_results)
    ↓
1. Store agent responses to SQLite (consensus_db)
2. Cache responses in state.agent_responses_cache
3. Transition phase to CheckingConsensus
4. Call check_consensus_and_advance_spec_auto()
```

**For Parallel Stages (Validate, Audit, Unlock):**
```
on_spec_auto_agents_complete_with_ids(specific_agent_ids)
    ↓
1. Check which phase we're in
2. Collect responses from active_agents
3. Filter by specific_agent_ids (prevents collecting stale agents)
4. Determine if quality gate or regular stage agents
5. Store to SQLite
6. Transition to CheckingConsensus
7. Call check_consensus_and_advance_spec_auto()
```

---

## Consensus Checking & Synthesis

**Entry Point:** `check_consensus_and_advance_spec_auto()` in `pipeline_coordinator.rs` (lines 625-850)

### Consensus Flow

```
check_consensus_and_advance_spec_auto()
    ↓
1. For Validate stage: Mark as CheckingConsensus in lifecycle
2. Check if agent_responses_cache is populated
   ├─ If yes: Use cached responses (fast path)
   └─ If no: Query via MCP consensus
3. Call synthesize_from_cached_responses() or MCP synthesis
4. On success:
   ├─ Advance current_index++
   ├─ Reset phase to Guardrail
   ├─ Clear agent_responses_cache
   ├─ Persist cost summary
   └─ Call advance_spec_auto() (recursive)
5. On failure:
   ├─ Advance degraded (still increment stage)
   ├─ Log degradation
   └─ Call advance_spec_auto() (degraded mode)
```

### Consensus Retry Logic

**MCP Consensus with Exponential Backoff:**
```rust
run_consensus_with_retry(mcp, cwd, spec_id, stage)
    ↓
For attempt in 0..3:
  1. Try to acquire MCP manager lock
  2. If unavailable: Wait 100ms * 2^attempt and retry
  3. If available: Run spec_consensus(stage)
  4. On success: Return (consensus_lines, ok_bool)
  5. On error: Retry with exponential backoff
       Delays: 100ms, 200ms, 400ms
  6. Return last error after 3 attempts
```

This handles MCP initialization delays gracefully without blocking.

---

## Quality Gate Architecture

**Quality Gate Orchestration:** `execute_quality_checkpoint()` in `quality_gate_handler.rs` (lines 1053-1200+)

### Quality Gate Phases

```
execute_quality_checkpoint(checkpoint)
    ↓
1. Transition to QualityGateExecuting phase
   ├─ checkpoint: BeforeSpecify|AfterSpecify|AfterTasks
   ├─ gates: [Clarify]|[Checklist]|[Analyze]
   └─ expected_agents: [gemini, claude, code] (default)
2. Log QualityGateStart event
3. Spawn native quality gate agents
   ├─ Check for already-running agents (single-flight guard)
   ├─ Prevent duplicate spawns
   └─ Track agent IDs for later collection
4. Agents begin execution asynchronously

on_quality_gate_agents_complete()
    ↓
1. Mark processing active (prevent recursion)
2. Store agent artifacts to local-memory
3. Transition to QualityGateProcessing
4. Call quality_gate_broker.fetch_agent_payloads()
   ├─ For native orchestrator: Use memory-based collection
   └─ For legacy: Use filesystem collection

on_quality_gate_broker_result()
    ↓
1. Receive agent payloads (3 quality gates x 3 agents = 9 max)
2. Parse issues from agent responses
3. Classify issues by:
   ├─ Confidence: High (3/3), Medium (2/3), Low (0-1/3)
   ├─ Magnitude: Critical, Important, Minor
   └─ Resolvability: AutoFix, SuggestFix, NeedHuman
4. Transition to QualityGateValidating phase
5. For unanimous issues (3/3 agree):
   ├─ Auto-apply fix immediately
   └─ Record to quality_auto_resolved
6. For majority issues (2/3 agree):
   ├─ Send to GPT-5 validator (async)
   └─ Collect validation results
7. For disputed issues (no consensus):
   ├─ Escalate to human
   └─ Collect to escalated_questions

on_quality_gate_validation_result()
    ↓
1. Receive GPT-5 validation result
2. Update completed_validations map
3. Check if all validations done
4. If all done: Transition to QualityGateAwaitingHuman

on_quality_gate_answers()
    ↓
1. Receive human answers to escalated questions
2. Apply human-approved resolutions
3. Mark checkpoint as completed
4. Transition back to Guardrail phase
5. Call advance_spec_auto() to continue pipeline

finalize_quality_gates()
    ↓
1. After all checkpoints completed
2. Write quality_auto_resolved summary
3. Write quality_escalated summary
4. Store evidence artifacts
```

### Single-Flight Guard

Quality gate agents are protected by single-flight guard (SPEC-KIT-928):
```rust
// Check if agents already running for this checkpoint
let already_running = check_running_agents_for_checkpoint();
if !already_running.is_empty() {
    // Log warning, don't spawn duplicates
    return;  // Wait for existing run
}
```

This prevents:
- Accidental double-spawning
- Resource exhaustion
- Consensus drift from multiple runs

---

## State Persistence & Recovery

### State Location

**In-Memory:** `widget.spec_auto_state: Option<SpecAutoState>`
- Holds entire pipeline state during execution
- Cleared on completion or cancellation

**Evidence Artifacts:** `docs/SPEC-OPS-004.../evidence/`
- Guardrail telemetry per stage
- Quality gate results
- Validate lifecycle events
- Execution logs

**SQLite Database:** `~/.code/consensus_artifacts.db`
- Agent execution tracking (agent_executions table)
- Consensus artifacts (consensus_runs, agent_outputs)
- Validate lifecycle telemetry
- Spawn phase metadata (quality_gate vs regular)

### Validate Lifecycle State

Separate deduplication state for Validate stage:
```rust
pub struct ValidateLifecycle {
    spec_id: Arc<String>,
    inner: Arc<Mutex<ValidateLifecycleInner>>,
}

pub struct ValidateLifecycleInner {
    attempt: u32,                      // Attempt counter (increments on new run)
    active: Option<ActiveValidateRun>, // Currently running validation
    last_completion: Option<ValidateRunCompletion>, // Last terminal state
}
```

**Lifecycle Flow:**
```
begin(mode, payload_hash)
    ├─ No active run: Start new attempt
    │  └─ Generate run_id: validate-<spec>-<mode>-attempt-<N>-<uuid>
    ├─ Active with same hash: Duplicate (incr dedupe_count)
    └─ Active with different hash: Conflict

mark_dispatched(run_id) → Updates status: Queued → Dispatched
mark_checking_consensus(run_id) → Updates status: Dispatched → CheckingConsensus
complete(run_id, reason) → Terminal state (Completed|Failed|Cancelled|Reset)
reset_active(reason) → Force terminal state
```

**Deduplication Logic:**
```
If same payload_hash submitted twice:
├─ First: ValidateBeginOutcome::Started (attempt=1, run_id)
├─ Second: ValidateBeginOutcome::Duplicate (dedupe_count=1)
└─ Result: Skip dispatch, await existing run completion
```

### Resume Points

Pipeline can resume from:
1. **Any Stage:** Via `--from <stage>` parameter
2. **Within Stage:** Via validate lifecycle deduplication
3. **Quality Checkpoint:** Checkpoints skip if already completed

```rust
// Skip stage if already completed (in completed_checkpoints set)
if completed.contains(&checkpoint) {
    None  // Skip this checkpoint
} else {
    Some(checkpoint)  // Run it now
}
```

---

## Error Handling & Degradation

### Guardrail Failures

```
Guardrail returns !success
    ├─ For Validate stage:
    │  ├─ Record ValidateLifecycleEvent::Failed
    │  ├─ No retries (terminal)
    │  └─ Halt pipeline
    └─ For other stages:
       ├─ Cleanup with cancellation
       └─ Halt pipeline
```

### Consensus Failures

```
Consensus check returns !ok
    ├─ Log consensus_not_reached
    ├─ Cleanup with cancellation
    └─ Halt pipeline
```

### Agent Degradation (AR-2, AR-3)

```
if agent_fails():
    ├─ Retry up to 3 times
    ├─ Exponential backoff
    └─ If still fails: Continue with remaining agents
       (2/3 consensus still valid per SPEC-KIT-070)
```

### Quality Gate Degradation

```
if quality_gate_agents < expected:
    ├─ Record degradation_reason in quality_checkpoint_degradations
    ├─ Process available results (2 agents instead of 3)
    ├─ Record missing_agents
    ├─ Schedule degraded_follow_up checklist after stage
    └─ Continue pipeline (warning level, not error)
```

---

## Tiered Model Strategy (SPEC-KIT-070)

### Agent Selection by Stage

**Tier 0: Native** (0 agents, $0, <1s)
- Guardrails: Native bash scripts + native Rust validation
- Quality gates: Native pattern matching

**Tier 1: Single Agent** ($0.10, 3-5min)
- Plan: gpt5-low (strategic planning)
- Tasks: gpt5-low (decomposition)

**Tier 2: Multi-Agent** ($0.35, 8-12min)
- Implement: gpt-5-codex (code) + claude-haiku (validator)
- Validate: gemini-flash, claude-haiku, gpt5-medium (parallel)

**Tier 3: Premium** ($0.80, 10-12min)
- Audit: gemini-pro, claude-sonnet, gpt5-high
- Unlock: gemini-pro, claude-sonnet, gpt5-high

**Total Cost:** ~$2.70 for full pipeline (down from $11, 75% reduction)

---

## File Locations

### Core Pipeline Files

| File | Lines | Purpose |
|------|-------|---------|
| state.rs | 1003 | State machine, SpecAutoPhase, SpecAutoState |
| pipeline_coordinator.rs | 1495 | advance_spec_auto, stage transitions |
| agent_orchestrator.rs | 2207 | Agent submission, response collection |
| quality_gate_handler.rs | 1810 | Quality gate orchestration |
| consensus_coordinator.rs | 194 | MCP consensus with retry |
| consensus_db.rs | 915 | SQLite persistence layer |
| validation_lifecycle.rs | 158 | Validate deduplication state |
| execution_logger.rs | 726 | End-to-end execution telemetry |

### Supporting Systems

| File | Purpose |
|------|---------|
| agent_retry.rs | Exponential backoff for agent failures |
| ace_route_selector.rs | Intelligent agent routing by stage complexity |
| cost_tracker.rs | Budget tracking and cost attribution |
| evidence.rs | Evidence artifact collection |
| consensus.rs | Consensus checking algorithm |
| json_extractor.rs | Robust JSON extraction from LLM outputs |

---

## Execution Flow Visualization

```
START
  ↓
handle_spec_auto(spec_id, goal, resume_from)
  ├─ Validate config ✓
  ├─ Check evidence size ✓
  ├─ Create SpecAutoState (current_index = resume_from)
  └─ advance_spec_auto()
       ↓
   LOOP: while current_index < stages.len()
       ├─ Check if quality checkpoint needed
       │  └─ If yes: execute_quality_checkpoint() → [quality gate phase loop]
       ├─ Transition phase: Guardrail
       ├─ Run guardrail for stage
       │  └─ on_spec_auto_task_complete(task_id)
       │     ├─ Collect guardrail outcome
       │     ├─ Run consensus check
       │     └─ auto_submit_spec_stage_prompt()
       │        └─ Spawn agents (Tier varies by stage)
       ├─ Transition phase: ExecutingAgents
       ├─ Wait for agent completion
       │  └─ on_spec_auto_agents_complete_with_results|ids()
       │     └─ Transition phase: CheckingConsensus
       ├─ Check consensus
       │  └─ check_consensus_and_advance_spec_auto()
       │     ├─ Synthesize from cached responses
       │     ├─ current_index++
       │     ├─ phase = Guardrail
       │     └─ [Loop back to top]
       └─
   AFTER LOOP: Finalize
       ├─ Finalize quality gates
       ├─ Log RunComplete event
       ├─ Generate verification report
       └─ Clear spec_auto_state
END
```

---

## Key Design Patterns

### 1. **Phase Transition with Logging**
Each phase change is logged with trigger reason for debugging:
```rust
state.transition_phase(new_phase, "trigger_reason");
```

### 2. **Single-Flight Guard**
Prevents concurrent execution of same operation:
```rust
if state.quality_gate_processing == Some(checkpoint) {
    return;  // Already processing, skip
}
```

### 3. **Cached Response Path**
Avoids expensive MCP calls for cached data:
```rust
if has_cached_responses {
    // Use cached agent responses directly
    synthesize_from_cached_responses(cached, ...)
} else {
    // Fall back to MCP consensus
    run_consensus_with_retry(...)
}
```

### 4. **Validate Lifecycle Deduplication**
Payload hash prevents identical submits:
```rust
let hash = compute_validate_payload_hash(mode, stage, spec_id, payload);
match validate_lifecycle.begin(mode, hash) {
    ValidateBeginOutcome::Duplicate(_) => return,  // Skip
    ValidateBeginOutcome::Started(_) => continue,  // New run
}
```

### 5. **Degradation Strategy**
Continues with fewer agents if needed:
```rust
if missing_agents.len() > 0 {
    record_degradation(missing_agents);
    schedule_degraded_follow_up(stage);
    continue_pipeline();  // Don't fail
}
```

---

## Performance Characteristics

### Timeline

| Stage | Tier | Duration | Cost | Execution |
|-------|------|----------|------|-----------|
| Plan | 1 | 3-5min | $0.10 | 1 agent sequential |
| Tasks | 1 | 3-5min | $0.10 | 1 agent sequential |
| Implement | 2 | 8-12min | $0.15 | 2 agents (codex + haiku) |
| Validate | 2 | 8-12min | $0.35 | 3 agents parallel |
| Audit | 3 | 10-12min | $0.80 | 3 premium agents parallel |
| Unlock | 3 | 10-12min | $0.80 | 3 premium agents parallel |
| **Quality Gates (3x)** | 0 | <1s each | $0.00 | Native checks |
| **Total** | Varies | 45-50min | ~$2.70 | Mixed |

### Memory Usage

- **SpecAutoState:** ~10KB per active pipeline
- **ExecutionLogger:** ~100KB per run (logged asynchronously)
- **ConsensusDb:** ~1-2MB per stage (SQLite with WAL)
- **Agent Responses Cache:** Variable (typically 100-500KB)

### Database I/O

- **SQLite Writes:** Async via tokio::spawn (non-blocking)
- **Evidence Files:** Written per-stage (10-50KB per stage)
- **Local-Memory:** Minimal (only important observations)

---

## Summary

The Spec-Kit pipeline is a sophisticated state machine that:

1. **Orchestrates** multi-agent workflows across 6 sequential stages
2. **Validates** at quality checkpoints using native and multi-agent consensus
3. **Coordinates** agent responses with phase tracking and deduplication
4. **Persists** state across database/filesystem for recovery
5. **Degrades** gracefully when agents fail or are unavailable
6. **Advances** automatically through consensus validation
7. **Resumes** from any stage if interrupted
8. **Tracks** execution with comprehensive logging and telemetry

The architecture balances automation with safety through:
- Quality gates that catch issues early
- Consensus validation before advancement
- Single-flight guards preventing duplicates
- Graceful degradation under failures
- Deterministic state machines with clear phase transitions
