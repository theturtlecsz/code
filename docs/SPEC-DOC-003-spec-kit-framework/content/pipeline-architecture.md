# Pipeline Architecture

Comprehensive guide to the Spec-Kit 6-stage automation pipeline.

---

## Overview

The **Spec-Kit pipeline** orchestrates a 6-stage workflow from PRD creation to production readiness:

```
Plan → Tasks → Implement → Validate → Audit → Unlock
```

**Key Characteristics**:
- **Auto-advancement**: Stages automatically progress on success
- **Quality gates**: 3 strategic checkpoints between stages
- **Resume capability**: Can restart from any stage
- **Single-flight guards**: Prevents duplicate agent spawns
- **Graceful degradation**: Continues with fewer agents if needed
- **Cost**: ~$2.70 total (down from $11, 75% reduction)
- **Time**: 45-50 minutes end-to-end

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/`

---

## Architecture Components

### Component Hierarchy

```
/speckit.auto (user command)
    ↓
pipeline_coordinator.rs (advancement loop)
    ↓
├── Guardrail Execution (stage validation)
├── Agent Orchestration (multi-agent consensus)
├── Consensus Coordination (MCP integration)
├── Quality Gate Handler (checkpoint validation)
└── State Persistence (SQLite + evidence files)
```

**Core Files**:

| File | LOC | Purpose |
|------|-----|---------|
| `state.rs` | 1,003 | State machine definition |
| `pipeline_coordinator.rs` | 1,495 | Stage advancement loop |
| `agent_orchestrator.rs` | 2,207 | Agent submission & response collection |
| `quality_gate_handler.rs` | 1,810 | Quality gate orchestration |
| `consensus_coordinator.rs` | 194 | MCP consensus with retry |
| `consensus_db.rs` | 915 | SQLite persistence |
| `validation_lifecycle.rs` | 158 | Validate deduplication state |

---

## State Machine

### SpecAutoPhase Enum

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/state.rs:15-45`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecAutoPhase {
    // Standard stage phases (loop for each stage)
    Guardrail,                    // Running guardrail validation
    ExecutingAgents,              // Agents actively running
    CheckingConsensus,            // Synthesizing consensus from MCP

    // Quality gate sub-phases (checkpoint validation)
    QualityGateExecuting,         // Quality gate agents spawning
    QualityGateProcessing,        // Classifying results
    QualityGateValidating,        // GPT-5 validation of answers
    QualityGateAwaitingHuman,     // Escalation for user decision

    // Terminal state
    Complete,                     // Pipeline finished
}
```

**Phase Transitions**:

```
Standard Stage Flow:
Guardrail → ExecutingAgents → CheckingConsensus → (check quality gates)
                                                    ↓
                                        Quality gate required?
                                                    ↓
                                        ┌──────────┴──────────┐
                                        NO                   YES
                                        ↓                     ↓
                               Next stage              QualityGateExecuting
                                                             ↓
                                                  QualityGateProcessing
                                                             ↓
                                                  QualityGateValidating
                                                             ↓
                                            Pass?    QualityGateAwaitingHuman
                                              ↓              ↓
                                         Next stage    (User decision)
```

---

### SpecAutoState Struct

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/state.rs:47-110`

```rust
pub struct SpecAutoState {
    // === Stage Tracking ===
    pub spec_id: String,              // e.g., "SPEC-KIT-070"
    pub current_index: usize,         // 0-5 (Plan, Tasks, Implement, Validate, Audit, Unlock)
    pub phase: SpecAutoPhase,         // Current phase within stage
    pub start_index: Option<usize>,   // Resume from specific stage (--from flag)

    // === Execution State ===
    pub logger: Option<SpecAutoExecutionLogger>,  // Execution metadata
    pub validate_lifecycle: Option<ValidateLifecycleState>, // Deduplication state
    pub active_agents: Vec<String>,   // Currently running agent IDs

    // === Quality Gates ===
    pub quality_gates_state: Option<QualityGatesState>,  // Checkpoint state
    pub completed_checkpoints: HashSet<String>,  // Memoization (skip if done)

    // === Agent Response Caching ===
    pub agent_response_cache: HashMap<String, CachedResponse>,  // Avoid redundant MCP calls

    // === Error Recovery ===
    pub retry_count: usize,           // Current retry attempt (max 3)
    pub degraded_agents: Vec<String>, // Agents that failed (still valid if 2/3 succeed)

    // === Telemetry ===
    pub stage_start_time: Option<Instant>,  // For duration tracking
    pub total_cost: f64,              // Accumulated cost across stages
}
```

**Memory Footprint**: ~10 KB (in-memory only during execution)

---

## 6-Stage Workflow

### Stage Overview

| Index | Stage | Tier | Agents | Cost | Time | Purpose |
|-------|-------|------|--------|------|------|---------|
| 0 | **Plan** | 2 (Multi) | 3 | ~$0.35 | 10-12min | Work breakdown |
| 1 | **Tasks** | 1 (Single) | 1 | ~$0.10 | 3-5min | Task decomposition |
| 2 | **Implement** | 2 (Code) | 2 | ~$0.11 | 8-12min | Code generation |
| 3 | **Validate** | 2 (Multi) | 3 | ~$0.35 | 10-12min | Test strategy |
| 4 | **Audit** | 3 (Premium) | 3 | ~$0.80 | 10-12min | Compliance check |
| 5 | **Unlock** | 3 (Premium) | 3 | ~$0.80 | 10-12min | Ship decision |

**Total**: ~$2.70, 45-50 minutes

---

### Stage 0: Plan

**Purpose**: Architectural planning with multi-agent consensus

**Agents**: 3 (gemini-flash, claude-haiku, gpt5-medium)

**Flow**:
1. **Guardrail**: Validate PRD exists, no implementation started
2. **ExecutingAgents**: Submit 3 agents with plan prompt
3. **CheckingConsensus**: MCP synthesis of 3 perspectives
4. **Output**: `docs/SPEC-{id}-{slug}/plan.md`

**Quality Gate** (Before Tasks): **AfterSpecify (Checklist)**
- Validates PRD + plan quality
- Checks: completeness, clarity, testability, consistency
- Must score ≥80/100 to proceed

---

### Stage 1: Tasks

**Purpose**: Task decomposition from plan

**Agents**: 1 (gpt5-low)

**Flow**:
1. **Guardrail**: Validate plan.md exists, structure valid
2. **ExecutingAgents**: Single agent for structured breakdown
3. **CheckingConsensus**: Direct output (no consensus needed)
4. **Output**: `docs/SPEC-{id}-{slug}/tasks.md` + SPEC.md update

**Quality Gate** (Before Implement): **AfterTasks (Analyze)**
- Consistency check (ID mismatches, coverage gaps)
- Must have 0 critical issues to proceed

---

### Stage 2: Implement

**Purpose**: Code generation with specialist model

**Agents**: 2 (gpt_codex HIGH, claude-haiku validator)

**Flow**:
1. **Guardrail**: Validate git tree clean, tasks.md exists
2. **ExecutingAgents**: gpt-5-codex for code, haiku for validation
3. **CheckingConsensus**: Synthesize implementation + review
4. **Post-validation**: `cargo fmt`, `cargo clippy`, build checks
5. **Output**: Source code changes + implementation notes

**No Quality Gate**: Code validation happens in Validate stage

---

### Stage 3: Validate

**Purpose**: Test strategy consensus

**Agents**: 3 (gemini-flash, claude-haiku, gpt5-medium)

**Special Features**:
- **Single-flight guard**: Prevents duplicate submissions
- **Deduplication**: Payload hash tracking
- **Lifecycle state**: Tracks attempt count per hash

**Flow**:
1. **Guardrail**: Validate implementation complete, tests defined
2. **ExecutingAgents**: 3 agents for test coverage analysis
3. **CheckingConsensus**: Synthesize test strategy
4. **Deduplication Check**: Hash payload, skip if duplicate
5. **Output**: Test plan + coverage requirements

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/validation_lifecycle.rs:15-80`

```rust
pub struct ValidateLifecycleState {
    pub attempts: HashMap<String, ValidateAttempt>,  // Hash → attempt info
}

pub struct ValidateAttempt {
    pub payload_hash: String,     // SHA-256 of inputs
    pub attempt_number: usize,    // 1st, 2nd, 3rd attempt
    pub timestamp: Instant,       // When submitted
}

pub enum ValidateBeginOutcome {
    Fresh,           // New hash, proceed
    Duplicate,       // Same hash, skip dispatch
    Retry,           // Different hash, increment counter
}
```

**No Quality Gate**: Audit stage validates compliance

---

### Stage 4: Audit

**Purpose**: Compliance and security validation

**Agents**: 3 premium (gemini-pro, claude-sonnet, gpt5-high)

**Flow**:
1. **Guardrail**: Validate tests passing, coverage met
2. **ExecutingAgents**: 3 premium agents for security analysis
3. **CheckingConsensus**: Synthesize compliance report
4. **Checks**: OWASP Top 10, dependency vulnerabilities, license compliance
5. **Output**: Audit report with pass/fail per check

**No Quality Gate**: Unlock stage is final decision point

---

### Stage 5: Unlock

**Purpose**: Final ship/no-ship decision

**Agents**: 3 premium (gemini-pro, claude-sonnet, gpt5-high)

**Flow**:
1. **Guardrail**: Validate all prior stages complete, audit passed
2. **ExecutingAgents**: 3 premium agents for production readiness
3. **CheckingConsensus**: Synthesize ship decision
4. **Decision**: Consensus must agree (2/3 minimum)
5. **Output**: Unlock approval or blockers

**Phase**: Complete (pipeline finished)

---

## Quality Gates

### 3 Strategic Checkpoints

**Design Philosophy**: "Fail fast, recover early"

```
BeforeSpecify (Clarify) → BEFORE PLAN
    ↓
AfterSpecify (Checklist) → BEFORE TASKS
    ↓
AfterTasks (Analyze) → BEFORE IMPLEMENT
```

**Why These Checkpoints?**
- **BeforeSpecify**: Catch PRD ambiguities before investing in planning
- **AfterSpecify**: Validate PRD + plan quality before task breakdown
- **AfterTasks**: Ensure consistency before code generation

**Note**: Quality gates check BEFORE stages (not after) to prevent wasted work

---

### Quality Gate Sub-State Machine

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs:50-150`

```rust
pub struct QualityGatesState {
    pub current_checkpoint: String,        // e.g., "AfterSpecify"
    pub gate_phase: QualityGatePhase,      // Sub-phase within gate
    pub agent_responses: Vec<String>,      // Raw agent outputs
    pub classification: Option<Classification>,  // Pass/Fail/Unclear
    pub validation_result: Option<bool>,   // GPT-5 final verdict
}

pub enum QualityGatePhase {
    Executing,       // Agents spawning
    Processing,      // Classifying results
    Validating,      // GPT-5 validation
    AwaitingHuman,   // Escalation
}
```

**5-Phase Flow**:

```
1. QualityGateExecuting
   - Spawn quality gate agents (2-3 agents)
   - Submit gate-specific prompts (clarify, checklist, analyze)
   - Phase transition on all agents complete

2. QualityGateProcessing
   - Collect agent responses
   - Classify each as: Pass, Fail, Unclear
   - Count votes: 2/3 Pass = likely pass, 2/3 Fail = likely fail

3. QualityGateValidating
   - If clear consensus (2/3 same): GPT-5 validation
   - GPT-5 reviews all responses + classification
   - Returns: true (proceed), false (block)

4. QualityGateAwaitingHuman (if unclear or GPT-5 rejects)
   - Show user all agent responses
   - User decision: proceed or fix issues
   - Manual override option available

5. Back to Guardrail
   - Quality gate complete
   - Resume normal stage advancement
```

**Single-Flight Guard**:

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs:200-240`

```rust
pub fn begin_quality_gate(
    ctx: &mut impl SpecKitContext,
    checkpoint: &str,
) -> Result<()> {
    // Check if already running
    if let Some(state) = &ctx.spec_auto_state().as_ref()?.quality_gates_state {
        if state.current_checkpoint == checkpoint {
            return Err(anyhow!(
                "Quality gate '{}' already in progress",
                checkpoint
            ));
        }
    }

    // Check if already completed (memoization)
    if ctx.spec_auto_state()
        .as_ref()?
        .completed_checkpoints
        .contains(checkpoint)
    {
        return Ok(()); // Skip, already passed
    }

    // Spawn gate agents
    let agents = get_gate_agents(checkpoint);
    for agent in agents {
        ctx.submit_operation(Op::SubmitAgent(agent));
    }

    // Set gate state
    ctx.spec_auto_state_mut().as_mut()?.quality_gates_state = Some(QualityGatesState {
        current_checkpoint: checkpoint.to_string(),
        gate_phase: QualityGatePhase::Executing,
        agent_responses: Vec::new(),
        classification: None,
        validation_result: None,
    });

    Ok(())
}
```

**Memoization**: Completed checkpoints stored in `completed_checkpoints` set, skipped on resume

---

## Auto-Advancement Logic

### Advancement Loop

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs:100-450`

```rust
pub fn advance_spec_auto(ctx: &mut impl SpecKitContext) -> Result<()> {
    let state = ctx.spec_auto_state_mut()
        .as_mut()
        .ok_or_else(|| anyhow!("No spec auto state"))?;

    match state.phase {
        SpecAutoPhase::Guardrail => {
            // Validate stage prerequisites
            let stage = current_stage(state.current_index)?;
            run_guardrail_validation(ctx, &stage)?;

            // Transition to ExecutingAgents
            state.phase = SpecAutoPhase::ExecutingAgents;
            spawn_stage_agents(ctx, &stage)?;
        }

        SpecAutoPhase::ExecutingAgents => {
            // Wait for all agents to complete
            if !all_agents_complete(state) {
                return Ok(()); // Still running
            }

            // Transition to CheckingConsensus
            state.phase = SpecAutoPhase::CheckingConsensus;
            initiate_consensus_check(ctx)?;
        }

        SpecAutoPhase::CheckingConsensus => {
            // Synthesize consensus from MCP
            let consensus = run_consensus_with_retry(ctx)?;

            // Check for quality gates
            if let Some(checkpoint) = next_quality_gate(state.current_index) {
                // Begin quality gate
                state.phase = SpecAutoPhase::QualityGateExecuting;
                begin_quality_gate(ctx, &checkpoint)?;
            } else {
                // No gate, proceed to next stage
                increment_stage_and_reset(state)?;

                // Recursive call for next stage
                advance_spec_auto(ctx)?;
            }
        }

        SpecAutoPhase::QualityGateExecuting => {
            // Wait for gate agents
            handle_quality_gate_execution(ctx)?;
        }

        SpecAutoPhase::QualityGateProcessing => {
            // Classify responses
            handle_quality_gate_processing(ctx)?;
        }

        SpecAutoPhase::QualityGateValidating => {
            // GPT-5 validation
            handle_quality_gate_validation(ctx)?;
        }

        SpecAutoPhase::QualityGateAwaitingHuman => {
            // User decision required
            // (Blocks until user responds)
            return Ok(());
        }

        SpecAutoPhase::Complete => {
            // Pipeline finished
            finalize_pipeline(ctx)?;
        }
    }

    Ok(())
}
```

**Recursive Advancement**: Calls itself after stage increment to immediately start next stage

---

### Consensus Coordination

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs:15-120`

```rust
pub fn run_consensus_with_retry(
    ctx: &impl SpecKitContext,
) -> Result<Consensus> {
    let state = ctx.spec_auto_state()
        .as_ref()
        .ok_or_else(|| anyhow!("No spec auto state"))?;

    // Check cache first (avoid redundant MCP calls)
    let cache_key = format!("{}:{}", state.spec_id, state.current_index);
    if let Some(cached) = state.agent_response_cache.get(&cache_key) {
        return Ok(cached.consensus.clone());
    }

    // MCP consensus with exponential backoff
    let mut retry_delay = Duration::from_millis(100);
    for attempt in 0..3 {
        match mcp_synthesize_consensus(ctx, &state.active_agents) {
            Ok(consensus) => {
                // Cache for future use
                cache_consensus(ctx, &cache_key, &consensus)?;
                return Ok(consensus);
            }
            Err(e) if attempt < 2 => {
                // Retry with backoff
                std::thread::sleep(retry_delay);
                retry_delay *= 2;  // 100ms → 200ms → 400ms
            }
            Err(e) => {
                // Final attempt failed
                return Err(e);
            }
        }
    }

    unreachable!()
}
```

**Retry Strategy**:
- **Max attempts**: 3
- **Backoff**: Exponential (100ms, 200ms, 400ms)
- **Caching**: Successful consensus cached to avoid redundant calls

---

## State Persistence

### 3-Layer Architecture

```
Layer 1: In-Memory (ChatWidget.spec_auto_state)
    ↓
Layer 2: SQLite Database (~/.code/consensus_artifacts.db)
    ↓
Layer 3: Evidence Files (docs/SPEC-OPS-004.../evidence/)
```

**Purpose of Each Layer**:
- **In-Memory**: Fast access, active pipeline state only
- **SQLite**: Agent execution history, consensus artifacts, queryable
- **Evidence Files**: Auditable logs, human-readable, version controlled

---

### Layer 1: In-Memory State

**Location**: `codex-rs/tui/src/chatwidget/mod.rs:53`

```rust
pub(crate) struct ChatWidget<'a> {
    // ... other fields ...

    spec_auto_state: Option<SpecAutoState>,  // 10KB, active only
}
```

**Lifecycle**:
1. **Creation**: `/speckit.auto` initializes `SpecAutoState`
2. **Updates**: Every phase transition modifies state
3. **Cleanup**: Set to `None` when pipeline completes

**Not Persisted**: Lost on application exit (intentional, evidence files preserve results)

---

### Layer 2: SQLite Database

**Location**: `~/.code/consensus_artifacts.db`

**Schema** (from `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs:50-150`):

```sql
-- Agent executions (quality gate vs regular)
CREATE TABLE agent_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    is_quality_gate BOOLEAN NOT NULL,  -- Distinguish gate agents
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL,  -- 'running', 'success', 'failed', 'degraded'
    cost REAL,
    output_hash TEXT,  -- SHA-256 for deduplication
    UNIQUE(spec_id, stage, agent_name)
);

-- Consensus runs (agent outputs per run)
CREATE TABLE consensus_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    run_id TEXT NOT NULL,  -- UUID for this consensus run
    agent_responses TEXT NOT NULL,  -- JSON array of responses
    synthesized_consensus TEXT,     -- Final consensus output
    created_at INTEGER NOT NULL,
    UNIQUE(spec_id, stage, run_id)
);

-- Indexes for fast lookups
CREATE INDEX idx_executions_spec_stage ON agent_executions(spec_id, stage);
CREATE INDEX idx_consensus_spec_stage ON consensus_runs(spec_id, stage);
```

**Write Pattern** (async, non-blocking):

```rust
pub fn record_agent_execution(
    spec_id: &str,
    stage: &str,
    agent: &AgentInfo,
    is_quality_gate: bool,
) -> Result<()> {
    let db = get_db_connection()?;

    // Async write (don't block UI)
    tokio::spawn(async move {
        db.execute(
            "INSERT INTO agent_executions (spec_id, stage, agent_name, is_quality_gate, started_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running')",
            params![spec_id, stage, agent.name, is_quality_gate, now()],
        )?;
        Ok::<(), anyhow::Error>(())
    });

    Ok(())
}
```

**Query Pattern** (for diagnostics):

```rust
pub fn get_stage_agents(spec_id: &str, stage: &str) -> Result<Vec<AgentExecution>> {
    let db = get_db_connection()?;

    let mut stmt = db.prepare(
        "SELECT * FROM agent_executions
         WHERE spec_id = ?1 AND stage = ?2
         ORDER BY started_at ASC"
    )?;

    let rows = stmt.query_map(params![spec_id, stage], |row| {
        Ok(AgentExecution {
            agent_name: row.get(2)?,
            is_quality_gate: row.get(3)?,
            status: row.get(6)?,
            cost: row.get(7)?,
        })
    })?;

    rows.collect()
}
```

**Retention**: No automatic cleanup (user can manually delete old entries)

---

### Layer 3: Evidence Files

**Location**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/{SPEC-ID}/`

**Files Created Per Stage**:

```
evidence/commands/SPEC-KIT-070/
├── plan/
│   ├── plan_execution.json       (10 KB, guardrail telemetry)
│   ├── agent_1_gemini.txt        (15 KB, agent output)
│   ├── agent_2_claude.txt        (15 KB, agent output)
│   ├── agent_3_gpt5.txt          (15 KB, agent output)
│   └── consensus.json            (5 KB, synthesized consensus)
├── tasks/
│   ├── tasks_execution.json      (8 KB)
│   ├── agent_1_gpt5.txt          (10 KB)
│   └── consensus.json            (3 KB)
├── validate/
│   ├── validate_execution.json   (12 KB)
│   ├── payload_hash_abc123.json  (2 KB, deduplication record)
│   └── ... (agent outputs)
└── quality_gates/
    ├── AfterSpecify_checkpoint.json  (5 KB)
    ├── gate_agent_1.txt              (8 KB)
    └── gpt5_validation.json          (2 KB)
```

**Total**: ~200-300 KB per SPEC (within 25 MB soft limit)

**Format Example** (`plan_execution.json`):

```json
{
  "command": "plan",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T14:32:00Z",
  "schemaVersion": "1.0",
  "baseline": {
    "mode": "file",
    "artifact": "docs/SPEC-KIT-070-cost-optimization/spec.md",
    "status": "exists"
  },
  "hooks": {
    "session": {
      "start": "passed"
    }
  },
  "artifacts": [
    "docs/SPEC-KIT-070-cost-optimization/plan.md"
  ],
  "agents": [
    {
      "name": "gemini-flash",
      "cost": 0.12,
      "duration_ms": 8500,
      "status": "success"
    }
  ],
  "total_cost": 0.35,
  "total_duration_ms": 11200
}
```

---

## Resume & Recovery

### Resume from Specific Stage

**Command**: `/speckit.auto SPEC-KIT-070 --from tasks`

**Implementation** (`codex-rs/tui/src/chatwidget/spec_kit/commands/auto.rs:30-60`):

```rust
pub fn handle_auto_command(
    ctx: &mut impl SpecKitContext,
    spec_id: &str,
    from_stage: Option<&str>,
) -> Result<()> {
    // Determine start index
    let start_index = if let Some(stage) = from_stage {
        stage_name_to_index(stage)?  // "tasks" → 1
    } else {
        0  // Start from Plan
    };

    // Initialize state with start_index
    let state = SpecAutoState {
        spec_id: spec_id.to_string(),
        current_index: start_index,
        phase: SpecAutoPhase::Guardrail,
        start_index: Some(start_index),
        // ... other fields ...
    };

    ctx.spec_auto_state_mut().replace(state);

    // Begin advancement from specified stage
    advance_spec_auto(ctx)?;

    Ok(())
}
```

**Stage Index Mapping**:

```rust
fn stage_name_to_index(name: &str) -> Result<usize> {
    match name.to_lowercase().as_str() {
        "plan" => Ok(0),
        "tasks" => Ok(1),
        "implement" => Ok(2),
        "validate" => Ok(3),
        "audit" => Ok(4),
        "unlock" => Ok(5),
        _ => Err(anyhow!("Unknown stage: {}", name)),
    }
}
```

**Use Cases**:
- **Development**: Test individual stages without running full pipeline
- **Recovery**: Restart from failed stage after fixing issues
- **Iteration**: Re-run specific stage with different inputs

---

### Validate Deduplication

**Problem**: Prevent duplicate validate submissions when user retries

**Solution**: Payload hashing with attempt tracking

**Implementation** (`codex-rs/tui/src/chatwidget/spec_kit/validation_lifecycle.rs:40-100`):

```rust
pub struct ValidateLifecycleState {
    pub attempts: HashMap<String, ValidateAttempt>,
}

pub struct ValidateAttempt {
    pub payload_hash: String,     // SHA-256 of inputs
    pub attempt_number: usize,
    pub timestamp: Instant,
}

pub fn begin_validate(
    ctx: &mut impl SpecKitContext,
    spec_id: &str,
) -> Result<ValidateBeginOutcome> {
    // Compute payload hash (spec.md + plan.md + tasks.md)
    let payload = collect_validate_inputs(spec_id)?;
    let hash = sha256(&payload);

    // Check existing attempts
    let lifecycle = ctx.spec_auto_state_mut()
        .as_mut()?
        .validate_lifecycle
        .get_or_insert_with(Default::default);

    match lifecycle.attempts.get(&hash) {
        Some(_attempt) => {
            // Same hash = duplicate submission
            Ok(ValidateBeginOutcome::Duplicate)
        }
        None => {
            // New hash = fresh attempt
            lifecycle.attempts.insert(hash.clone(), ValidateAttempt {
                payload_hash: hash,
                attempt_number: lifecycle.attempts.len() + 1,
                timestamp: Instant::now(),
            });

            Ok(ValidateBeginOutcome::Fresh)
        }
    }
}
```

**Behavior**:
- **Same hash**: Skip agent dispatch, show cached results
- **Different hash**: New attempt, increment counter
- **Evidence**: `evidence/validate/payload_hash_{hash}.json`

---

### Quality Checkpoint Memoization

**Problem**: Don't re-run passed quality gates on resume

**Solution**: Track completed checkpoints in `completed_checkpoints` set

**Implementation** (`codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs:250-280`):

```rust
pub fn should_run_quality_gate(
    ctx: &impl SpecKitContext,
    checkpoint: &str,
) -> Result<bool> {
    let state = ctx.spec_auto_state()
        .as_ref()
        .ok_or_else(|| anyhow!("No spec auto state"))?;

    // Check if already completed
    if state.completed_checkpoints.contains(checkpoint) {
        return Ok(false);  // Skip
    }

    Ok(true)  // Run gate
}

pub fn mark_quality_gate_complete(
    ctx: &mut impl SpecKitContext,
    checkpoint: &str,
) -> Result<()> {
    let state = ctx.spec_auto_state_mut()
        .as_mut()
        .ok_or_else(|| anyhow!("No spec auto state"))?;

    state.completed_checkpoints.insert(checkpoint.to_string());

    // Save to evidence
    save_checkpoint_completion(ctx, checkpoint)?;

    Ok(())
}
```

**Persistence**: `evidence/quality_gates/completed_checkpoints.json`

```json
{
  "spec_id": "SPEC-KIT-070",
  "completed": [
    "BeforeSpecify",
    "AfterSpecify"
  ],
  "last_updated": "2025-10-18T15:45:00Z"
}
```

---

### Graceful Degradation

**Problem**: What if 1 of 3 agents fails?

**Solution**: Continue with 2/3 agents (consensus still valid)

**Implementation** (`codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:150-220`):

```rust
pub fn collect_agent_responses(
    ctx: &impl SpecKitContext,
) -> Result<Vec<AgentResponse>> {
    let state = ctx.spec_auto_state()
        .as_ref()
        .ok_or_else(|| anyhow!("No spec auto state"))?;

    let mut responses = Vec::new();
    let mut failed_agents = Vec::new();

    for agent_id in &state.active_agents {
        match get_agent_output(agent_id) {
            Ok(output) => {
                responses.push(AgentResponse {
                    agent: agent_id.clone(),
                    output,
                    status: AgentStatus::Success,
                });
            }
            Err(e) => {
                // Mark as degraded, but continue
                failed_agents.push(agent_id.clone());
                ctx.push_background(
                    format!("Agent {} failed: {}", agent_id, e),
                    BackgroundPlacement::Bottom,
                );
            }
        }
    }

    // Require at least 2/3 agents for multi-agent stages
    let required = (state.active_agents.len() * 2) / 3;  // 2 if 3 agents
    if responses.len() < required {
        return Err(anyhow!(
            "Insufficient agents: {} of {} required (failed: {:?})",
            responses.len(),
            required,
            failed_agents
        ));
    }

    // Record degradation
    ctx.spec_auto_state_mut()
        .as_mut()?
        .degraded_agents
        .extend(failed_agents);

    Ok(responses)
}
```

**Behavior**:
- **3/3 agents**: Ideal consensus
- **2/3 agents**: Degraded but valid
- **1/3 agents**: Insufficient, halt pipeline

**Evidence**: Failed agents recorded in `degraded_agents` field + telemetry

---

## Design Patterns

### Pattern 1: Single-Flight Guard

**Purpose**: Prevent duplicate operations during concurrent requests

**Implementation**:

```rust
pub fn begin_operation(ctx: &mut impl SpecKitContext) -> Result<()> {
    // Check if already running
    if ctx.spec_auto_state()
        .as_ref()
        .map(|s| s.phase == SpecAutoPhase::ExecutingAgents)
        .unwrap_or(false)
    {
        return Err(anyhow!("Operation already in progress"));
    }

    // ... proceed with operation
}
```

**Use Cases**:
- Quality gate execution (prevent duplicate spawns)
- Validate submission (deduplication via hash)
- Consensus checking (avoid redundant MCP calls)

---

### Pattern 2: Exponential Backoff

**Purpose**: Retry transient failures with increasing delays

**Implementation**:

```rust
let mut retry_delay = Duration::from_millis(100);
for attempt in 0..3 {
    match operation() {
        Ok(result) => return Ok(result),
        Err(e) if attempt < 2 => {
            std::thread::sleep(retry_delay);
            retry_delay *= 2;  // 100ms → 200ms → 400ms
        }
        Err(e) => return Err(e),
    }
}
```

**Use Cases**:
- MCP consensus requests (network transient)
- SQLite writes (lock contention)
- Agent response polling (rate limits)

---

### Pattern 3: Response Caching

**Purpose**: Avoid redundant MCP consensus calls

**Implementation**:

```rust
// Check cache
let cache_key = format!("{}:{}", spec_id, stage);
if let Some(cached) = state.agent_response_cache.get(&cache_key) {
    return Ok(cached.consensus.clone());
}

// Fetch from MCP
let consensus = mcp_synthesize_consensus(ctx, agents)?;

// Cache for future
state.agent_response_cache.insert(cache_key, CachedResponse {
    consensus: consensus.clone(),
    timestamp: Instant::now(),
});
```

**Cache Invalidation**: Cleared on pipeline completion (in-memory only)

---

### Pattern 4: Recursive Advancement

**Purpose**: Automatically progress through stages without user interaction

**Implementation**:

```rust
pub fn advance_spec_auto(ctx: &mut impl SpecKitContext) -> Result<()> {
    // ... handle current phase ...

    // When stage complete, increment and recurse
    if current_stage_complete(ctx)? {
        increment_stage(ctx)?;

        // Recursive call for next stage (tail recursion)
        advance_spec_auto(ctx)?;
    }

    Ok(())
}
```

**Stack Depth**: Max 6 stages (Plan → Unlock), no overflow risk

---

## Performance Metrics

### Pipeline Duration Breakdown

**Total**: 45-50 minutes end-to-end

| Stage | Guardrail | Agents | Consensus | Quality Gate | Total |
|-------|-----------|--------|-----------|--------------|-------|
| **Plan** | 5s | 10min | 30s | 2min (AfterSpecify) | ~12.5min |
| **Tasks** | 5s | 3min | 10s | 1min (AfterTasks) | ~4min |
| **Implement** | 10s | 8min | 30s | - | ~9min |
| **Validate** | 5s | 10min | 30s | - | ~10.5min |
| **Audit** | 5s | 10min | 30s | - | ~10.5min |
| **Unlock** | 5s | 10min | 30s | - | ~10.5min |

**Quality Gates**: BeforeSpecify (1min), AfterSpecify (2min), AfterTasks (1min) = 4min total

**Grand Total**: ~57 minutes (includes quality gates)

**Note**: Times vary based on agent load, network latency, model response times

---

### Cost Breakdown

**Total**: ~$2.70 (down from $11, 75% reduction)

| Component | Cost | Savings Strategy |
|-----------|------|------------------|
| **Plan** (3 multi) | $0.35 | Cheap agents (gemini-flash, claude-haiku) + gpt5-medium |
| **Tasks** (1 single) | $0.10 | Single agent (gpt5-low) instead of 3 |
| **Implement** (2 code) | $0.11 | gpt-5-codex (HIGH) + cheap validator |
| **Validate** (3 multi) | $0.35 | Same as Plan |
| **Audit** (3 premium) | $0.80 | Premium justified (security critical) |
| **Unlock** (3 premium) | $0.80 | Premium justified (ship decision) |
| **Quality Gates** | $0.19 | Native heuristics (FREE) + GPT-5 validation ($0.05/gate) |

**Savings Breakdown** (from original $11):
- **Native operations**: $2.40 saved (clarify, analyze, checklist now FREE)
- **Single-agent tasks**: $0.25 saved (3 agents → 1 agent)
- **Cheap multi-agent**: $1.05 saved (premium → cheap for plan/validate)
- **Specialist code generation**: $0.69 saved (3 premium → gpt-5-codex + cheap validator)

---

### Database Performance

**Writes** (async, non-blocking):
- Agent execution record: ~0.9ms (p50)
- Consensus run record: ~1.2ms (p50)

**Reads** (diagnostic queries):
- Get stage agents: ~129µs (p50)
- Get consensus history: ~180µs (p50)

**Total Database Overhead**: <100ms per full pipeline

---

## Error Handling

### Error Categories

**1. Transient Errors** (retry-able):
- Network timeouts (MCP consensus)
- SQLite lock contention
- Model API rate limits
- Agent timeout (rare)

**Recovery**: Exponential backoff (3 attempts max)

**2. Permanent Errors** (halt pipeline):
- Missing prerequisite files (spec.md, plan.md)
- Git tree dirty (implementation stage)
- Insufficient agents (< 2/3 success)
- Quality gate failure (user decision required)

**Recovery**: User intervention required

**3. Degraded Errors** (continue with warnings):
- 1 of 3 agents failed (2/3 still valid)
- Evidence file write failed (non-critical)
- Cache miss (fetch from source)

**Recovery**: Automatic, log warning

---

### Error Flow Example

**Scenario**: MCP consensus request fails during Plan stage

```
1. advance_spec_auto() calls run_consensus_with_retry()
2. First attempt fails (network timeout)
3. Sleep 100ms, retry
4. Second attempt fails
5. Sleep 200ms, retry
6. Third attempt succeeds
7. Cache result, continue to quality gate
```

**If all 3 attempts fail**:
```
1. Return error to advance_spec_auto()
2. Pipeline halts at CheckingConsensus phase
3. Show error to user in TUI
4. User can:
   - Retry (/speckit.auto --from plan)
   - Manual intervention (fix network, retry)
   - Abort pipeline
```

---

## Summary

**Pipeline Architecture Highlights**:

1. **6-Stage Workflow**: Plan → Tasks → Implement → Validate → Audit → Unlock
2. **8-Phase State Machine**: Guardrail → ExecutingAgents → CheckingConsensus → (quality gates) → Complete
3. **3 Quality Gates**: BeforeSpecify, AfterSpecify, AfterTasks (fail fast, recover early)
4. **Auto-Advancement**: Recursive loop automatically progresses stages
5. **3-Layer Persistence**: In-memory (fast) → SQLite (queryable) → Evidence files (auditable)
6. **Resume & Recovery**: Restart from any stage, deduplication, checkpoint memoization, graceful degradation
7. **Cost Optimization**: ~$2.70 total (75% cheaper via strategic agent routing)
8. **Performance**: 45-50 minutes end-to-end, <100ms database overhead

**Next Steps**:
- [Consensus System](consensus-system.md) - Multi-agent consensus details
- [Quality Gates](quality-gates.md) - Checkpoint validation deep dive
- [Cost Tracking](cost-tracking.md) - Per-stage cost breakdown

---

**File References**:
- State machine: `codex-rs/tui/src/chatwidget/spec_kit/state.rs:15-110`
- Advancement loop: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs:100-450`
- Quality gates: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs:50-280`
- Consensus: `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs:15-120`
- Database: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs:50-150`
- Validation lifecycle: `codex-rs/tui/src/chatwidget/spec_kit/validation_lifecycle.rs:15-100`
