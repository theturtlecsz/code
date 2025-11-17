# Agent Orchestration

Comprehensive guide to multi-agent coordination and execution.

---

## Overview

**Agent orchestration** coordinates multiple AI agents to produce validated consensus:

- **Agent selection**: ACE-based routing by capability and cost
- **Execution patterns**: Sequential pipeline vs parallel consensus
- **Response collection**: Async task management with timeouts
- **Retry logic**: Exponential backoff for transient failures
- **Degradation handling**: Continue with 2/3 agents if 1 fails
- **Lifecycle tracking**: From submission → execution → collection

**Performance**: 50ms parallel spawn, 8.7ms consensus synthesis

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

---

## Agent Lifecycle

### 5-Phase Lifecycle

```
Phase 1: Agent Selection (ACE routing)
    ↓
Phase 2: Agent Submission (async task spawn)
    ↓
Phase 3: Execution (parallel or sequential)
    ↓
Phase 4: Response Collection (timeout management)
    ↓
Phase 5: Consensus Synthesis (MCP integration)
```

**Total Time**: 3-12 minutes (depends on agent count and pattern)

---

### Phase 1: Agent Selection

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/ace_route_selector.rs:25-120`

```rust
pub struct AgentCapability {
    pub name: String,              // "gemini-flash"
    pub model: String,             // "gemini-1.5-flash-latest"
    pub reasoning_level: ReasoningLevel,  // Low/Medium/High/Specialist
    pub cost_per_1k_tokens: f64,   // 0.0002
    pub specialization: Vec<String>,  // ["analysis", "planning"]
    pub max_tokens: usize,         // 8192
}

pub fn select_agents_for_tier(
    tier: CommandTier,
    stage: &str,
) -> Vec<AgentCapability> {
    match tier {
        CommandTier::Tier1Single => {
            vec![AgentCapability {
                name: "gpt5-low".to_string(),
                model: "gpt-5-low".to_string(),
                reasoning_level: ReasoningLevel::Low,
                cost_per_1k_tokens: 0.0001,
                specialization: vec!["tasks".to_string()],
                max_tokens: 4096,
            }]
        }

        CommandTier::Tier2Multi => {
            if stage == "implement" {
                vec![
                    AgentCapability {
                        name: "gpt-5-codex".to_string(),
                        model: "gpt-5-codex-high".to_string(),
                        reasoning_level: ReasoningLevel::Specialist,
                        cost_per_1k_tokens: 0.0006,
                        specialization: vec!["code".to_string()],
                        max_tokens: 16384,
                    },
                    AgentCapability {
                        name: "claude-haiku".to_string(),
                        model: "claude-3-5-haiku-20241022".to_string(),
                        reasoning_level: ReasoningLevel::Medium,
                        cost_per_1k_tokens: 0.00025,
                        specialization: vec!["validator".to_string()],
                        max_tokens: 8192,
                    },
                ]
            } else {
                vec![
                    AgentCapability {
                        name: "gemini-flash".to_string(),
                        model: "gemini-1.5-flash-latest".to_string(),
                        reasoning_level: ReasoningLevel::Low,
                        cost_per_1k_tokens: 0.0002,
                        specialization: vec!["fast".to_string()],
                        max_tokens: 8192,
                    },
                    AgentCapability {
                        name: "claude-haiku".to_string(),
                        model: "claude-3-5-haiku-20241022".to_string(),
                        reasoning_level: ReasoningLevel::Medium,
                        cost_per_1k_tokens: 0.00025,
                        specialization: vec!["balanced".to_string()],
                        max_tokens: 8192,
                    },
                    AgentCapability {
                        name: "gpt5-medium".to_string(),
                        model: "gpt-5-medium".to_string(),
                        reasoning_level: ReasoningLevel::Medium,
                        cost_per_1k_tokens: 0.0005,
                        specialization: vec!["strategic".to_string()],
                        max_tokens: 8192,
                    },
                ]
            }
        }

        CommandTier::Tier3Premium => {
            vec![
                AgentCapability {
                    name: "gemini-pro".to_string(),
                    model: "gemini-1.5-pro-latest".to_string(),
                    reasoning_level: ReasoningLevel::High,
                    cost_per_1k_tokens: 0.0015,
                    specialization: vec!["reasoning".to_string()],
                    max_tokens: 32768,
                },
                AgentCapability {
                    name: "claude-sonnet".to_string(),
                    model: "claude-3-5-sonnet-20241022".to_string(),
                    reasoning_level: ReasoningLevel::High,
                    cost_per_1k_tokens: 0.003,
                    specialization: vec!["security".to_string()],
                    max_tokens: 16384,
                },
                AgentCapability {
                    name: "gpt5-high".to_string(),
                    model: "gpt-5-high".to_string(),
                    reasoning_level: ReasoningLevel::High,
                    cost_per_1k_tokens: 0.005,
                    specialization: vec!["critical".to_string()],
                    max_tokens: 16384,
                },
            ]
        }

        _ => vec![],
    }
}
```

**Selection Criteria**:
- **Tier**: Command complexity (simple → single, complex → multi, critical → premium)
- **Stage**: Special routing for code generation (implement)
- **Cost**: Prefer cheap models when reasoning quality not critical
- **Capability**: Match agent specialization to task requirements

---

### Phase 2: Agent Submission

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:100-200`

```rust
pub struct AgentSubmission {
    pub agent: AgentCapability,
    pub prompt: String,
    pub session_id: String,
    pub spec_id: String,
    pub stage: String,
    pub timeout: Duration,         // Default: 5 minutes
    pub retry_policy: RetryPolicy,
}

pub enum RetryPolicy {
    NoRetry,
    Fixed { attempts: usize, delay_ms: u64 },
    Exponential { max_attempts: usize, initial_delay_ms: u64, multiplier: f64 },
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy::Exponential {
            max_attempts: 3,
            initial_delay_ms: 100,
            multiplier: 2.0,  // 100ms → 200ms → 400ms
        }
    }
}
```

**Submission Flow**:

```rust
pub async fn submit_agent(
    submission: AgentSubmission,
) -> Result<AgentTask> {
    // Create async task
    let task_id = generate_task_id();

    // Spawn on Tokio runtime
    let handle = tokio::spawn(async move {
        execute_agent_with_retry(
            &submission.agent,
            &submission.prompt,
            submission.retry_policy,
        ).await
    });

    Ok(AgentTask {
        id: task_id,
        agent: submission.agent,
        handle,
        started_at: Instant::now(),
        timeout: submission.timeout,
    })
}
```

---

### Phase 3: Execution

#### Pattern A: Sequential Pipeline

**Use Cases**: Plan, Tasks, Implement (agents build on each other)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:439-576`

```rust
pub async fn execute_sequential_pipeline(
    agents: Vec<AgentCapability>,
    base_prompt: &str,
    spec_id: &str,
    stage: &str,
) -> Result<Vec<AgentOutput>> {
    let mut outputs = Vec::new();
    let mut previous_outputs = String::new();

    for (i, agent) in agents.iter().enumerate() {
        // Build prompt with previous outputs
        let prompt = if i == 0 {
            base_prompt.to_string()
        } else {
            base_prompt.replace("${PREVIOUS_OUTPUTS}", &previous_outputs)
        };

        // Submit agent
        let submission = AgentSubmission {
            agent: agent.clone(),
            prompt,
            session_id: generate_session_id(),
            spec_id: spec_id.to_string(),
            stage: stage.to_string(),
            timeout: Duration::from_secs(300),  // 5 minutes
            retry_policy: RetryPolicy::default(),
        };

        let task = submit_agent(submission).await?;

        // Wait for completion (blocking)
        let output = wait_for_agent(task).await?;

        // Accumulate outputs
        previous_outputs.push_str(&format!(
            "\n\n--- {} Output ---\n{}",
            agent.name,
            output.content
        ));

        outputs.push(output);
    }

    Ok(outputs)
}
```

**Example** (Plan stage):

```
Agent 1: gemini-flash
  Input: PRD + constitution
  Output: "Suggest modular architecture..."
  Duration: 8.5s

Agent 2: claude-haiku
  Input: PRD + constitution + gemini output
  Output: "Building on gemini's approach, I recommend..."
  Duration: 9.2s

Agent 3: gpt5-medium
  Input: PRD + constitution + gemini + claude outputs
  Output: "Synthesizing both perspectives, final plan is..."
  Duration: 10.5s

Total: 28.2s (sequential)
```

**Advantages**:
- ✅ Iterative refinement
- ✅ Each agent sees previous work
- ✅ Final agent synthesizes all inputs

**Disadvantages**:
- ❌ Slower (sequential, not parallel)
- ❌ Later agents potentially biased

---

#### Pattern B: Parallel Consensus

**Use Cases**: Validate, Audit, Unlock (independent perspectives)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:583-756`

```rust
pub async fn execute_parallel_consensus(
    agents: Vec<AgentCapability>,
    prompt: &str,
    spec_id: &str,
    stage: &str,
) -> Result<Vec<AgentOutput>> {
    // Spawn all agents in parallel
    let mut join_set = tokio::task::JoinSet::new();

    for agent in agents {
        let prompt = prompt.to_string();
        let spec_id = spec_id.to_string();
        let stage = stage.to_string();

        // Spawn async task for each agent
        join_set.spawn(async move {
            let submission = AgentSubmission {
                agent: agent.clone(),
                prompt,
                session_id: generate_session_id(),
                spec_id,
                stage,
                timeout: Duration::from_secs(600),  // 10 minutes
                retry_policy: RetryPolicy::default(),
            };

            let task = submit_agent(submission).await?;
            wait_for_agent(task).await
        });
    }

    // Collect all outputs (wait for all to complete)
    let mut outputs = Vec::new();

    while let Some(result) = join_set.join_next().await {
        match result? {
            Ok(output) => outputs.push(output),
            Err(e) => {
                // Log error, continue with other agents
                eprintln!("Agent failed: {}", e);
            }
        }
    }

    Ok(outputs)
}
```

**Example** (Validate stage):

```
Parallel Spawn (t=0s):
  gemini-flash   spawned (50ms overhead)
  claude-haiku   spawned
  gpt5-medium    spawned

Parallel Execution (t=0-10min):
  gemini-flash   → "Test coverage: 85%..." (9.0s)
  claude-haiku   → "Coverage adequate..." (9.5s)
  gpt5-medium    → "Coverage good..." (10.0s)

All Complete (t=10.0s):
  3 outputs ready simultaneously

Total: 10.0s + 50ms overhead = 10.05s
```

**Speedup**: 3× faster than sequential (28.2s → 10.05s)

**Advantages**:
- ✅ Fast (all agents run simultaneously)
- ✅ Independent perspectives (no bias)
- ✅ True consensus (2/3 quorum)

**Disadvantages**:
- ❌ No iterative refinement
- ❌ Potential conflicts (requires resolution)

---

### Phase 4: Response Collection

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:800-900`

```rust
pub struct AgentOutput {
    pub agent: AgentCapability,
    pub content: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost: f64,
    pub duration_ms: u64,
    pub status: AgentStatus,
}

pub enum AgentStatus {
    Success,
    Failed { reason: String },
    Timeout,
    Degraded { warning: String },
}

pub async fn wait_for_agent(task: AgentTask) -> Result<AgentOutput> {
    // Wait with timeout
    match timeout(task.timeout, task.handle).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(anyhow!("Agent execution failed: {}", e)),
        Err(_) => Err(anyhow!("Agent timeout after {:?}", task.timeout)),
    }
}
```

**Timeout Handling**:

```rust
pub async fn wait_for_agents_with_timeout(
    tasks: Vec<AgentTask>,
    global_timeout: Duration,
) -> Vec<Result<AgentOutput>> {
    // Create futures for all tasks
    let futures = tasks.into_iter().map(|task| {
        timeout(task.timeout, task.handle)
    }).collect::<Vec<_>>();

    // Wait for all with global timeout
    match timeout(global_timeout, join_all(futures)).await {
        Ok(results) => {
            results.into_iter().map(|r| {
                r.map_err(|_| anyhow!("Individual timeout"))
                    .and_then(|inner| inner.map_err(|e| anyhow!("Execution failed: {}", e)))
            }).collect()
        }
        Err(_) => {
            // Global timeout exceeded
            vec![Err(anyhow!("Global timeout after {:?}", global_timeout))]
        }
    }
}
```

**Timeouts**:
- **Per-agent**: 5-10 minutes (depends on stage)
- **Global**: 15 minutes (safety net for parallel execution)

---

### Phase 5: Consensus Synthesis

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs:47-98`

```rust
pub async fn synthesize_consensus(
    agent_outputs: Vec<AgentOutput>,
    spec_id: &str,
    stage: &str,
) -> Result<Consensus> {
    // Step 1: Validate outputs
    validate_agent_outputs(&agent_outputs)?;

    // Step 2: Call MCP for synthesis
    let synthesis_result = mcp_synthesize_consensus(
        &agent_outputs,
        spec_id,
        stage,
    ).await?;

    // Step 3: Compute verdict
    let verdict = compute_verdict(&agent_outputs, &synthesis_result)?;

    Ok(Consensus {
        synthesized_output: synthesis_result.output,
        verdict,
        agent_outputs,
        cost: compute_total_cost(&agent_outputs),
        duration_ms: synthesis_result.duration_ms,
    })
}
```

**MCP Synthesis**:

```rust
async fn mcp_synthesize_consensus(
    agent_outputs: &[AgentOutput],
    spec_id: &str,
    stage: &str,
) -> Result<SynthesisResult> {
    // Build synthesis prompt
    let prompt = format!(
        "Synthesize consensus from {} agent outputs:\n\n{}",
        agent_outputs.len(),
        format_agent_outputs(agent_outputs)
    );

    // Call MCP local-memory server
    let result = mcp_client
        .call_tool("synthesize_consensus", json!({
            "prompt": prompt,
            "spec_id": spec_id,
            "stage": stage,
        }))
        .await?;

    Ok(SynthesisResult {
        output: result["synthesized"].as_str().unwrap().to_string(),
        duration_ms: result["duration_ms"].as_u64().unwrap(),
    })
}
```

**Verdict Computation**:

```rust
fn compute_verdict(
    agent_outputs: &[AgentOutput],
    synthesis: &SynthesisResult,
) -> Result<ConsensusVerdict> {
    // Count present agents
    let present_agents: Vec<_> = agent_outputs
        .iter()
        .filter(|o| o.status == AgentStatus::Success)
        .map(|o| o.agent.name.clone())
        .collect();

    // Check for conflicts
    let conflicts = detect_conflicts(agent_outputs)?;

    // Determine status
    let status = if !conflicts.is_empty() {
        VerdictStatus::Conflict
    } else if present_agents.len() == agent_outputs.len() {
        VerdictStatus::Ok
    } else if present_agents.len() >= (agent_outputs.len() * 2) / 3 {
        VerdictStatus::Degraded
    } else {
        VerdictStatus::Unknown
    };

    Ok(ConsensusVerdict {
        status,
        present_agents,
        missing_agents: find_missing_agents(agent_outputs),
        conflicts,
        degraded: status == VerdictStatus::Degraded,
    })
}
```

---

## Retry Logic

### Exponential Backoff

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:850-920`

```rust
async fn execute_agent_with_retry(
    agent: &AgentCapability,
    prompt: &str,
    retry_policy: RetryPolicy,
) -> Result<AgentOutput> {
    match retry_policy {
        RetryPolicy::NoRetry => {
            execute_agent_once(agent, prompt).await
        }

        RetryPolicy::Exponential { max_attempts, initial_delay_ms, multiplier } => {
            let mut delay_ms = initial_delay_ms;

            for attempt in 0..max_attempts {
                match execute_agent_once(agent, prompt).await {
                    Ok(output) => return Ok(output),
                    Err(e) if attempt < max_attempts - 1 => {
                        eprintln!(
                            "Agent {} failed (attempt {}/{}): {}",
                            agent.name,
                            attempt + 1,
                            max_attempts,
                            e
                        );

                        // Wait before retry
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                        // Increase delay
                        delay_ms = (delay_ms as f64 * multiplier) as u64;
                    }
                    Err(e) => {
                        // Final attempt failed
                        return Err(anyhow!(
                            "Agent {} failed after {} attempts: {}",
                            agent.name,
                            max_attempts,
                            e
                        ));
                    }
                }
            }

            unreachable!()
        }

        _ => Err(anyhow!("Unsupported retry policy")),
    }
}
```

**Retry Schedule** (default):

| Attempt | Delay | Total Time |
|---------|-------|------------|
| 1 | 0ms | 0ms |
| 2 (retry) | 100ms | 100ms |
| 3 (retry) | 200ms | 300ms |

**Max Overhead**: 300ms per agent (negligible vs 3-10 min execution)

---

## Degradation Handling

### 2/3 Quorum Rule

**Principle**: Valid consensus requires at least 2/3 agents (if no conflicts)

**Implementation**:

```rust
pub fn is_valid_consensus(
    present: usize,
    expected: usize,
    conflicts: &[Conflict],
) -> bool {
    // No conflicts required for validity
    if !conflicts.is_empty() {
        return false;
    }

    // 2/3 quorum
    present >= (expected * 2) / 3
}
```

**Example** (3 agents):

| Scenario | Present | Missing | Status | Valid? |
|----------|---------|---------|--------|--------|
| All succeed | 3 | 0 | Ok | ✅ Yes |
| 1 fails | 2 | 1 | Degraded | ✅ Yes (2/3 quorum) |
| 2 fail | 1 | 2 | Unknown | ❌ No (< 2/3) |

**Degraded Consensus**:

```rust
pub fn handle_degraded_consensus(
    ctx: &mut impl SpecKitContext,
    verdict: &ConsensusVerdict,
) -> Result<()> {
    if verdict.degraded {
        // Log warning
        ctx.push_background(
            format!(
                "Degraded consensus: {} of {} agents succeeded. Missing: {:?}",
                verdict.present_agents.len(),
                verdict.present_agents.len() + verdict.missing_agents.len(),
                verdict.missing_agents
            ),
            BackgroundPlacement::Bottom,
        );

        // Store degradation in evidence
        record_degradation(
            ctx,
            &verdict.present_agents,
            &verdict.missing_agents,
        )?;

        // Schedule follow-up (optional)
        schedule_agent_rerun(ctx, &verdict.missing_agents)?;
    }

    Ok(())
}
```

---

## Performance Optimization

### Parallel Agent Spawning (SPEC-933)

**Before** (sequential spawn):
```
Agent 1: submit → wait 50ms
Agent 2: submit → wait 50ms
Agent 3: submit → wait 50ms
Total: 150ms
```

**After** (parallel spawn):
```
All agents: submit simultaneously → wait 50ms
Total: 50ms
```

**Speedup**: 3× faster spawn time

**Implementation**:

```rust
// Old: sequential
for agent in agents {
    let task = submit_agent(agent).await?;
    tasks.push(task);
}

// New: parallel
let tasks = agents.into_iter().map(|agent| {
    submit_agent(agent)  // Returns future, not awaited yet
}).collect::<Vec<_>>();

let tasks = join_all(tasks).await;  // Await all at once
```

---

### Response Caching

**Purpose**: Avoid redundant MCP calls

**Implementation**:

```rust
lazy_static! {
    static ref AGENT_CACHE: RwLock<HashMap<String, AgentOutput>> = RwLock::new(HashMap::new());
}

pub async fn get_agent_output_cached(
    agent: &AgentCapability,
    prompt: &str,
) -> Result<AgentOutput> {
    // Compute cache key (hash of agent + prompt)
    let cache_key = compute_cache_key(agent, prompt);

    // Check cache
    if let Some(cached) = AGENT_CACHE.read().unwrap().get(&cache_key) {
        return Ok(cached.clone());
    }

    // Execute agent
    let output = execute_agent_once(agent, prompt).await?;

    // Cache result
    AGENT_CACHE.write().unwrap().insert(cache_key, output.clone());

    Ok(output)
}
```

**Cache Invalidation**: Cleared on pipeline completion

---

## Error Handling

### Error Categories

**1. Transient Errors** (retry-able):
- Network timeouts
- Model API rate limits
- Temporary service unavailability

**Recovery**: Exponential backoff (3 attempts max)

**2. Permanent Errors** (halt pipeline):
- Invalid API credentials
- Model not found
- Insufficient permissions

**Recovery**: User intervention required

**3. Degraded Errors** (continue with warnings):
- 1 of 3 agents failed (2/3 still valid)
- Slower-than-expected execution
- Model API warnings

**Recovery**: Automatic, log warning

---

### Error Flow Example

**Scenario**: gemini-flash times out during Plan stage

```
1. submit_agent(gemini-flash) → timeout after 5 minutes
2. Retry 1: execute_agent_with_retry → wait 100ms, retry
3. Retry 2: execute_agent_with_retry → wait 200ms, retry
4. Retry 3: execute_agent_with_retry → wait 400ms, retry
5. All retries failed → mark as failed
6. Collect other agents (claude-haiku, gpt5-medium)
7. Check 2/3 quorum: 2 of 3 present → degraded consensus ✅
8. Continue pipeline with warning
```

**If 2+ agents fail**:
```
1. Only 1 of 3 agents succeed
2. Check 2/3 quorum: 1 of 3 present → unknown status ❌
3. Halt pipeline, show error
4. User can retry: /speckit.plan SPEC-ID
```

---

## Monitoring & Observability

### Agent Execution Tracking

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_tracker.rs`

```rust
pub struct AgentExecutionTracker {
    pub active_agents: HashMap<String, AgentExecution>,
    pub completed_agents: Vec<AgentExecution>,
}

pub struct AgentExecution {
    pub task_id: String,
    pub agent_name: String,
    pub spec_id: String,
    pub stage: String,
    pub started_at: Instant,
    pub status: ExecutionStatus,
}

pub enum ExecutionStatus {
    Running,
    Success { duration_ms: u64, cost: f64 },
    Failed { reason: String },
    Timeout,
}
```

**Usage**:

```rust
// Start tracking
tracker.start_agent("task-123", "gemini-flash", "SPEC-KIT-070", "plan");

// Update status
tracker.update_status("task-123", ExecutionStatus::Running);

// Complete
tracker.complete_agent("task-123", ExecutionStatus::Success {
    duration_ms: 8500,
    cost: 0.12,
});
```

---

### Real-Time Progress Display

**TUI Status**:

```
┌──────────────────────────────────────────────────────────┐
│ SPEC-KIT-070 | Stage: plan (in progress)                │
│ Agents: 2/3 complete (gemini-flash ✅, claude-haiku ✅)  │
│ Waiting: gpt5-medium (5min 30s elapsed)                 │
│ Cost: $0.23 / $0.35 (66%)                                │
└──────────────────────────────────────────────────────────┘
```

**Detailed View** (`/speckit.status SPEC-ID`):

```
Agent Execution Status:

gemini-flash:
  Status: ✅ Complete
  Duration: 8.5s
  Cost: $0.12
  Tokens: 5,000 in / 1,500 out

claude-haiku:
  Status: ✅ Complete
  Duration: 9.2s
  Cost: $0.11
  Tokens: 6,000 in / 2,000 out

gpt5-medium:
  Status: 🔄 Running (5min 30s elapsed)
  Expected: ~10min total
  Estimated cost: $0.14
```

---

## Best Practices

### Agent Selection

**DO**:
- ✅ Use cheap agents (gemini-flash, claude-haiku) for non-critical stages
- ✅ Use premium agents (gemini-pro, claude-sonnet, gpt5-high) for security/compliance
- ✅ Use code specialist (gpt-5-codex) for implementation
- ✅ Match agent capability to task requirements

**DON'T**:
- ❌ Use premium agents for all stages (unnecessary cost)
- ❌ Use single agent when consensus needed (lower quality)
- ❌ Use general agents for code generation (specialist better)

---

### Execution Patterns

**DO**:
- ✅ Use sequential pipeline when agents should build on each other (plan, tasks, implement)
- ✅ Use parallel consensus for independent perspectives (validate, audit, unlock)
- ✅ Set appropriate timeouts (5min simple, 10min complex)

**DON'T**:
- ❌ Use sequential when parallel would work (slower)
- ❌ Use parallel when agents need previous context (lower quality)
- ❌ Set timeouts too short (premature failures)

---

### Error Handling

**DO**:
- ✅ Implement retry logic for transient failures
- ✅ Continue with 2/3 agents if 1 fails (degraded consensus)
- ✅ Log all errors with context (agent, stage, reason)
- ✅ Store error telemetry in evidence

**DON'T**:
- ❌ Fail entire pipeline on single agent failure (unless <2/3)
- ❌ Retry indefinitely (max 3 attempts)
- ❌ Ignore degraded consensus warnings (investigate later)

---

## Summary

**Agent Orchestration Highlights**:

1. **5-Phase Lifecycle**: Selection → Submission → Execution → Collection → Synthesis
2. **Dual Patterns**: Sequential pipeline (build on each other) vs parallel consensus (independent)
3. **ACE Routing**: Agent selection by capability, cost, and specialization
4. **Retry Logic**: Exponential backoff (100ms, 200ms, 400ms)
5. **Degradation Handling**: 2/3 quorum allows 1 agent failure
6. **Performance**: 50ms parallel spawn, 8.7ms consensus synthesis
7. **Observability**: Real-time tracking, status display, telemetry

**Next Steps**:
- [Template System](template-system.md) - PRD and document templates
- [Workflow Patterns](workflow-patterns.md) - Common usage scenarios

---

**File References**:
- Agent orchestrator: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:100-920`
- ACE selector: `codex-rs/tui/src/chatwidget/spec_kit/ace_route_selector.rs:25-120`
- Consensus coordinator: `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs:47-98`
- Agent tracker: `codex-rs/tui/src/chatwidget/spec_kit/agent_tracker.rs`
