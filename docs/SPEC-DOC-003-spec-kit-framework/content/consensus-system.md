# Consensus System

Comprehensive guide to multi-agent consensus mechanics in Spec-Kit.

---

## Overview

The **Consensus System** orchestrates multiple AI agents to produce validated, high-quality outputs through:

- **Multi-agent collaboration**: 1-5 agents per stage
- **Native MCP integration**: 5.3× faster than subprocess (8.7ms typical)
- **Tier-based routing**: Strategic agent selection by cost/complexity
- **Consensus synthesis**: Automated validation and conflict resolution
- **Response caching**: SQLite persistence avoids redundant work
- **Graceful degradation**: 2/3 quorum allows partial success

**Performance**: 8.7ms consensus check, 50ms parallel agent spawn

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/`

---

## Architecture Layers

### 5-Layer Consensus Stack

```
Layer 1: Agent Selection & Routing (tier-based)
    ↓
Layer 2: Agent Orchestration (sequential vs parallel)
    ↓
Layer 3: MCP Consensus Coordination (retry + fallback)
    ↓
Layer 4: Consensus Synthesis (verdict computation)
    ↓
Layer 5: Response Caching (SQLite persistence)
```

**Core Files**:

| File | LOC | Purpose |
|------|-----|---------|
| `routing.rs` | 221 | Command dispatch & ACE routing |
| `agent_orchestrator.rs` | 2,208 | Sequential/parallel spawning, execution control |
| `consensus_coordinator.rs` | 194 | MCP retry logic, cost summary |
| `consensus.rs` | 1,160 | Artifact collection, verdict synthesis |
| `consensus_db.rs` | 600+ | SQLite storage with connection pooling |

---

## Layer 1: Agent Selection & Routing

### Tier-Based Routing

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/routing.rs:15-80`

```rust
pub enum CommandTier {
    Tier0Native,        // $0, <1s (native Rust)
    Tier1Single,        // ~$0.10, 3-5min (1 agent)
    Tier2Multi,         // ~$0.35, 8-12min (2-3 agents)
    Tier3Premium,       // ~$0.80, 10-12min (3 premium)
    Tier4Pipeline,      // ~$2.70, 45-50min (strategic routing)
}

pub fn get_command_tier(command: &str) -> CommandTier {
    match command {
        // Tier 0: Native (FREE)
        "new" | "clarify" | "analyze" | "checklist" | "status" => CommandTier::Tier0Native,

        // Tier 1: Single Agent
        "specify" | "tasks" => CommandTier::Tier1Single,

        // Tier 2: Multi-Agent
        "plan" | "validate" => CommandTier::Tier2Multi,
        "implement" => CommandTier::Tier2Multi,  // Special: code specialist

        // Tier 3: Premium
        "audit" | "unlock" => CommandTier::Tier3Premium,

        // Tier 4: Full Pipeline
        "auto" => CommandTier::Tier4Pipeline,

        _ => CommandTier::Tier1Single,  // Default
    }
}
```

---

### ACE-Based Agent Selection

**ACE Model** (Agent Capability Evaluation): Selects agents based on:
- **Reasoning ability**: Low/Medium/High (affects tier)
- **Cost**: Budget constraints per tier
- **Specialization**: Code generation, analysis, validation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/ace_route_selector.rs:25-120`

```rust
pub struct AgentCapability {
    pub name: String,              // e.g., "gemini-flash"
    pub reasoning_level: ReasoningLevel,  // Low/Medium/High
    pub cost_per_1k_tokens: f64,   // e.g., 0.0002 (gemini-flash)
    pub specialization: Vec<String>,  // ["analysis", "planning"]
    pub max_tokens: usize,         // e.g., 8192
}

pub enum ReasoningLevel {
    Low,       // gpt5-low, gemini-flash
    Medium,    // gpt5-medium, claude-haiku
    High,      // gpt5-high, gemini-pro, claude-sonnet
    Specialist, // gpt-5-codex (code generation)
}

pub fn select_agents_for_tier(
    tier: CommandTier,
    stage: &str,
) -> Vec<AgentCapability> {
    match tier {
        CommandTier::Tier1Single => {
            // Single agent, low reasoning
            vec![agent("gpt5-low")]
        }

        CommandTier::Tier2Multi => {
            if stage == "implement" {
                // Code specialist + cheap validator
                vec![
                    agent("gpt-5-codex"),   // HIGH reasoning, code specialist
                    agent("claude-haiku"),  // MEDIUM reasoning, validator
                ]
            } else {
                // Multi-agent consensus (plan, validate)
                vec![
                    agent("gemini-flash"),  // LOW cost
                    agent("claude-haiku"),  // MEDIUM cost
                    agent("gpt5-medium"),   // MEDIUM reasoning
                ]
            }
        }

        CommandTier::Tier3Premium => {
            // Premium agents (audit, unlock)
            vec![
                agent("gemini-pro"),     // HIGH reasoning
                agent("claude-sonnet"),  // HIGH reasoning
                agent("gpt5-high"),      // HIGH reasoning
            ]
        }

        _ => vec![],  // Native or pipeline (no agents)
    }
}
```

**Agent Cost Table**:

| Agent | Reasoning | Cost/1K Tokens | Use Case |
|-------|-----------|----------------|----------|
| `gpt5-low` | Low | $0.0001 | Simple tasks, single-agent |
| `gemini-flash` | Low | $0.0002 | Multi-agent, budget-conscious |
| `claude-haiku` | Medium | $0.00025 | Validation, analysis |
| `gpt5-medium` | Medium | $0.0005 | Strategic planning |
| `gpt-5-codex` | Specialist | $0.0006 | Code generation only |
| `gemini-pro` | High | $0.0015 | Critical decisions |
| `claude-sonnet` | High | $0.003 | Security, compliance |
| `gpt5-high` | High | $0.005 | Ship/no-ship decisions |

---

## Layer 2: Agent Orchestration

### Sequential vs Parallel Execution

**Two Patterns**:
1. **Sequential Pipeline**: Agents build on each other's outputs (Plan, Tasks, Implement)
2. **Parallel Consensus**: Independent agents provide diverse perspectives (Validate, Audit, Unlock)

---

### Pattern 1: Sequential Pipeline

**Use Case**: Plan, Tasks, Implement stages

**Flow**: Agent 1 → Agent 2 (uses Agent 1 output) → Agent 3 (uses both)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:439-576`

```rust
pub async fn execute_sequential_pipeline(
    agents: Vec<AgentCapability>,
    spec_id: &str,
    stage: &str,
) -> Result<Vec<AgentOutput>> {
    let mut outputs = Vec::new();
    let mut previous_outputs = String::new();

    for (i, agent) in agents.iter().enumerate() {
        // Build prompt with previous outputs
        let prompt = if i == 0 {
            // First agent: base prompt only
            get_stage_prompt(spec_id, stage, &agent.name)?
        } else {
            // Subsequent agents: include previous outputs
            let prompt = get_stage_prompt(spec_id, stage, &agent.name)?;
            prompt.replace("${PREVIOUS_OUTPUTS}", &previous_outputs)
        };

        // Submit agent
        let output = submit_agent_and_wait(agent, &prompt).await?;

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

**Example** (Plan stage with 3 agents):

```
Step 1: gemini-flash
  Input: PRD + constitution
  Output: "Suggest modular architecture with 3 layers..."

Step 2: claude-haiku
  Input: PRD + constitution + gemini-flash output
  Output: "Building on gemini's layered approach, I recommend..."

Step 3: gpt5-medium
  Input: PRD + constitution + gemini + claude outputs
  Output: "Synthesizing both perspectives, final plan is..."
```

**Advantages**:
- ✅ Iterative refinement
- ✅ Agents learn from previous perspectives
- ✅ Final agent can synthesize all inputs

**Disadvantages**:
- ❌ Sequential (slower, ~30 min for 3 agents)
- ❌ Later agents biased by earlier ones

---

### Pattern 2: Parallel Consensus

**Use Case**: Validate, Audit, Unlock stages

**Flow**: All agents spawn simultaneously → Collect outputs → Synthesize consensus

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:583-756`

```rust
pub async fn execute_parallel_consensus(
    agents: Vec<AgentCapability>,
    spec_id: &str,
    stage: &str,
) -> Result<Vec<AgentOutput>> {
    // Spawn all agents in parallel (SPEC-933)
    let mut join_set = tokio::task::JoinSet::new();

    for agent in agents {
        let prompt = get_stage_prompt(spec_id, stage, &agent.name)?;

        // Spawn async task for each agent
        join_set.spawn(async move {
            submit_agent_and_wait(&agent, &prompt).await
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

**Example** (Validate stage with 3 agents):

```
Parallel Spawn (t=0s):
  gemini-flash   → "Test coverage: 85%, needs integration tests"
  claude-haiku   → "Test coverage adequate, add edge case tests"
  gpt5-medium    → "Coverage good, recommend mutation testing"

Collect (t=10min):
  All 3 outputs ready simultaneously

Synthesize:
  MCP consensus: "Test coverage: 85% (adequate), recommendations: integration tests, edge cases, mutation testing"
```

**Advantages**:
- ✅ Fast (all agents run simultaneously)
- ✅ Independent perspectives (no bias)
- ✅ True consensus (2/3 quorum)

**Disadvantages**:
- ❌ No iterative refinement
- ❌ Potential conflicts (requires resolution)

**Performance** (SPEC-933):
- **Spawn time**: 50ms total (all agents)
- **Execution time**: 10 minutes (parallel, not sequential)
- **Speedup**: 3× faster than sequential (30 min → 10 min)

---

### Retry Logic (SPEC-938)

**Problem**: Agents can fail (network, rate limits, timeouts)

**Solution**: Exponential backoff with 3 attempts

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:850-920`

```rust
pub async fn submit_agent_and_wait(
    agent: &AgentCapability,
    prompt: &str,
) -> Result<AgentOutput> {
    let mut retry_delay = Duration::from_millis(100);

    for attempt in 0..3 {
        match submit_agent_internal(agent, prompt).await {
            Ok(output) => {
                // Success
                return Ok(output);
            }
            Err(e) if attempt < 2 => {
                // Retry with backoff
                eprintln!(
                    "Agent {} failed (attempt {}/3): {}",
                    agent.name,
                    attempt + 1,
                    e
                );

                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;  // 100ms → 200ms → 400ms
            }
            Err(e) => {
                // Final attempt failed
                return Err(anyhow!(
                    "Agent {} failed after 3 attempts: {}",
                    agent.name,
                    e
                ));
            }
        }
    }

    unreachable!()
}
```

**Retry Behavior**:

| Attempt | Delay | Total Time |
|---------|-------|------------|
| 1 | 0ms | 0ms |
| 2 (retry) | 100ms | 100ms |
| 3 (retry) | 200ms | 300ms |

**Total Overhead**: Max 300ms per agent (negligible vs 10 min execution)

---

## Layer 3: MCP Consensus Coordination

### Native MCP Integration

**Advantage**: 5.3× faster than subprocess (46ms → 8.7ms)

**Architecture**:

```
Spec-Kit (Rust)
    ↓
MCP Client (Native, codex-rs/mcp-client/)
    ↓
MCP Server (local-memory, stdio transport)
    ↓
SQLite Database (~/.code/consensus_artifacts.db)
```

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs:47-98`

---

### Consensus Synthesis via MCP

**MCP Tool**: `mcp__local-memory__synthesize_consensus`

**Input**: Array of agent outputs + synthesis instructions

**Output**: Consensus document + metadata (conflicts, missing agents, etc.)

```rust
pub async fn run_consensus_with_retry(
    spec_id: &str,
    stage: &str,
    agent_outputs: &[AgentOutput],
) -> Result<Consensus> {
    // Step 1: Collect artifacts from 3 sources (fallback chain)
    let artifacts = collect_consensus_artifacts(spec_id, stage).await?;

    // Step 2: MCP synthesis with retry
    let mut retry_delay = Duration::from_millis(100);

    for attempt in 0..3 {
        match mcp_synthesize_consensus(agent_outputs, &artifacts).await {
            Ok(consensus) => {
                // Cache to SQLite
                cache_consensus(spec_id, stage, &consensus).await?;
                return Ok(consensus);
            }
            Err(e) if attempt < 2 => {
                // Retry with backoff
                eprintln!(
                    "MCP consensus failed (attempt {}/3): {}",
                    attempt + 1,
                    e
                );

                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;  // 100ms → 200ms → 400ms
            }
            Err(e) => {
                // Final attempt failed, check cache
                if let Ok(cached) = get_cached_consensus(spec_id, stage).await {
                    return Ok(cached);
                }

                return Err(anyhow!(
                    "MCP consensus failed after 3 attempts: {}",
                    e
                ));
            }
        }
    }

    unreachable!()
}
```

---

### Artifact Collection (3-Source Fallback)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs:251-433`

```rust
pub async fn collect_consensus_artifacts(
    spec_id: &str,
    stage: &str,
) -> Result<Vec<Artifact>> {
    let mut artifacts = Vec::new();

    // Source 1: SQLite (PRIMARY, fastest)
    match query_sqlite_artifacts(spec_id, stage).await {
        Ok(mut db_artifacts) => {
            artifacts.append(&mut db_artifacts);
        }
        Err(e) => {
            eprintln!("SQLite query failed: {}", e);
            // Continue to fallback sources
        }
    }

    // Source 2: local-memory MCP (FALLBACK 1)
    if artifacts.is_empty() {
        match query_mcp_artifacts(spec_id, stage).await {
            Ok(mut mcp_artifacts) => {
                artifacts.append(&mut mcp_artifacts);
            }
            Err(e) => {
                eprintln!("MCP query failed: {}", e);
                // Continue to final fallback
            }
        }
    }

    // Source 3: Evidence files (FALLBACK 2, slowest but always works)
    if artifacts.is_empty() {
        let evidence_path = format!(
            "docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/{}/{}",
            spec_id,
            stage
        );

        artifacts = read_evidence_files(&evidence_path)?;
    }

    if artifacts.is_empty() {
        return Err(anyhow!(
            "No artifacts found for {} stage {}",
            spec_id,
            stage
        ));
    }

    Ok(artifacts)
}
```

**Performance**:

| Source | Typical Time | Failure Rate |
|--------|--------------|--------------|
| SQLite | 8.7ms | <0.1% (SQLITE_BUSY) |
| MCP | 15ms | <1% (network) |
| Evidence files | 50ms | 0% (always works) |

---

### Cost Summary

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs:150-180`

```rust
pub struct ConsensusCostSummary {
    pub total_cost: f64,
    pub agent_costs: Vec<AgentCost>,
    pub mcp_consensus_cost: f64,
}

pub struct AgentCost {
    pub agent: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost: f64,
}

pub fn compute_cost_summary(
    agent_outputs: &[AgentOutput],
) -> ConsensusCostSummary {
    let mut total_cost = 0.0;
    let mut agent_costs = Vec::new();

    for output in agent_outputs {
        let cost = (output.input_tokens as f64 * output.agent.cost_per_1k_tokens / 1000.0)
            + (output.output_tokens as f64 * output.agent.cost_per_1k_tokens / 1000.0);

        agent_costs.push(AgentCost {
            agent: output.agent.name.clone(),
            input_tokens: output.input_tokens,
            output_tokens: output.output_tokens,
            cost,
        });

        total_cost += cost;
    }

    // MCP consensus cost (GPT-5 validation)
    let mcp_consensus_cost = 0.05;  // Fixed cost per synthesis
    total_cost += mcp_consensus_cost;

    ConsensusCostSummary {
        total_cost,
        agent_costs,
        mcp_consensus_cost,
    }
}
```

**Example Output**:

```json
{
  "total_cost": 0.35,
  "agent_costs": [
    {
      "agent": "gemini-flash",
      "input_tokens": 5000,
      "output_tokens": 1500,
      "cost": 0.10
    },
    {
      "agent": "claude-haiku",
      "input_tokens": 6000,
      "output_tokens": 2000,
      "cost": 0.15
    },
    {
      "agent": "gpt5-medium",
      "input_tokens": 7000,
      "output_tokens": 2500,
      "cost": 0.15
    }
  ],
  "mcp_consensus_cost": 0.05
}
```

---

## Layer 4: Consensus Synthesis

### Verdict Computation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs:682-958`

```rust
pub struct ConsensusVerdict {
    pub status: VerdictStatus,
    pub present_agents: Vec<String>,
    pub missing_agents: Vec<String>,
    pub conflicts: Vec<Conflict>,
    pub degraded: bool,
}

pub enum VerdictStatus {
    Ok,          // All agents, no conflicts → proceed
    Degraded,    // 2/3+ agents, schedule follow-up → proceed with caution
    Conflict,    // Explicit disagreements → HALT
    Unknown,     // Insufficient data → HALT
}

pub struct Conflict {
    pub agent_a: String,
    pub agent_b: String,
    pub issue: String,       // What they disagree on
    pub severity: ConflictSeverity,
}

pub enum ConflictSeverity {
    Minor,       // Different wording, same intent
    Moderate,    // Different approach, both valid
    Critical,    // Fundamentally incompatible
}
```

---

### Verdict Algorithm

```rust
pub fn compute_verdict(
    agent_outputs: &[AgentOutput],
    expected_agents: &[AgentCapability],
) -> ConsensusVerdict {
    // Step 1: Identify present/missing agents
    let present: Vec<_> = agent_outputs.iter().map(|o| o.agent.name.clone()).collect();
    let missing: Vec<_> = expected_agents
        .iter()
        .filter(|a| !present.contains(&a.name))
        .map(|a| a.name.clone())
        .collect();

    // Step 2: Detect conflicts (compare pairwise)
    let mut conflicts = Vec::new();
    for i in 0..agent_outputs.len() {
        for j in (i + 1)..agent_outputs.len() {
            if let Some(conflict) = detect_conflict(
                &agent_outputs[i],
                &agent_outputs[j],
            ) {
                conflicts.push(conflict);
            }
        }
    }

    // Step 3: Determine status
    let status = if !conflicts.is_empty() {
        VerdictStatus::Conflict  // HALT
    } else if present.len() == expected_agents.len() {
        VerdictStatus::Ok  // Perfect consensus
    } else if present.len() >= (expected_agents.len() * 2) / 3 {
        VerdictStatus::Degraded  // Acceptable (2/3 quorum)
    } else {
        VerdictStatus::Unknown  // Insufficient agents
    };

    ConsensusVerdict {
        status,
        present_agents: present,
        missing_agents: missing,
        conflicts,
        degraded: status == VerdictStatus::Degraded,
    }
}
```

---

### Conflict Detection

**Strategy**: Use MCP to detect semantic conflicts (GPT-5 validation)

```rust
pub fn detect_conflict(
    output_a: &AgentOutput,
    output_b: &AgentOutput,
) -> Option<Conflict> {
    // Call MCP to compare outputs semantically
    let comparison = mcp_compare_outputs(
        &output_a.content,
        &output_b.content,
    )?;

    if comparison.has_conflict {
        Some(Conflict {
            agent_a: output_a.agent.name.clone(),
            agent_b: output_b.agent.name.clone(),
            issue: comparison.conflict_description,
            severity: comparison.severity,
        })
    } else {
        None
    }
}
```

**MCP Tool**: `mcp__local-memory__compare_consensus_outputs`

**Input**:
```json
{
  "output_a": "Recommend 3-layer architecture...",
  "output_b": "Suggest monolithic approach...",
  "aspect": "architecture"
}
```

**Output**:
```json
{
  "has_conflict": true,
  "conflict_description": "Agent A recommends layered, Agent B monolithic",
  "severity": "Critical"
}
```

---

### Conflict Resolution

**Strategy**: User decision required for Critical conflicts

```rust
pub async fn resolve_conflicts(
    ctx: &mut impl SpecKitContext,
    verdict: &ConsensusVerdict,
) -> Result<ConflictResolution> {
    if verdict.conflicts.is_empty() {
        return Ok(ConflictResolution::NoConflicts);
    }

    // Check severity
    let has_critical = verdict.conflicts.iter().any(|c| {
        matches!(c.severity, ConflictSeverity::Critical)
    });

    if has_critical {
        // Escalate to user
        ctx.push_background(
            format!(
                "Critical conflicts detected:\n{}",
                format_conflicts(&verdict.conflicts)
            ),
            BackgroundPlacement::Top,
        );

        // HALT pipeline, await user decision
        return Ok(ConflictResolution::AwaitingUser);
    }

    // Minor/moderate conflicts: auto-resolve via GPT-5
    let resolution = mcp_auto_resolve_conflicts(&verdict.conflicts).await?;

    Ok(ConflictResolution::AutoResolved(resolution))
}
```

**User Decision Prompt**:

```
Critical conflicts detected between agents:

1. gemini-flash vs claude-haiku:
   Issue: Architecture approach (layered vs monolithic)
   Severity: Critical

How would you like to proceed?
  [1] Use gemini-flash recommendation
  [2] Use claude-haiku recommendation
  [3] Provide manual resolution
  [4] Abort pipeline
```

---

## Layer 5: Response Caching

### SQLite Schema (SPEC-KIT-072)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs:50-150`

```sql
-- Agent outputs (primary cache)
CREATE TABLE agent_outputs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    run_id TEXT NOT NULL,        -- UUID for this execution
    agent_name TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cost REAL NOT NULL,
    content TEXT NOT NULL,       -- Agent output (full text)
    created_at INTEGER NOT NULL,
    UNIQUE(spec_id, stage, run_id, agent_name)
);

-- Consensus runs (synthesized results)
CREATE TABLE consensus_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    run_id TEXT NOT NULL,
    synthesized_consensus TEXT NOT NULL,  -- MCP synthesis result
    verdict_status TEXT NOT NULL,         -- 'ok', 'degraded', 'conflict', 'unknown'
    present_agents TEXT NOT NULL,         -- JSON array
    missing_agents TEXT NOT NULL,         -- JSON array
    conflicts TEXT,                       -- JSON array (if any)
    total_cost REAL NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(spec_id, stage, run_id)
);

-- Indexes for fast lookups
CREATE INDEX idx_outputs_spec_stage ON agent_outputs(spec_id, stage);
CREATE INDEX idx_outputs_run_id ON agent_outputs(run_id);
CREATE INDEX idx_consensus_spec_stage ON consensus_runs(spec_id, stage);
CREATE INDEX idx_consensus_run_id ON consensus_runs(run_id);
```

---

### Connection Pooling (SPEC-945C)

**Problem**: SQLite BUSY errors under concurrent load

**Solution**: R2D2 connection pool + WAL mode + retry logic

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs:156-250`

```rust
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

lazy_static! {
    static ref DB_POOL: Pool<SqliteConnectionManager> = {
        let manager = SqliteConnectionManager::file("~/.code/consensus_artifacts.db");

        Pool::builder()
            .max_size(10)               // 10 connections
            .connection_timeout(Duration::from_secs(5))
            .build(manager)
            .expect("Failed to create DB pool")
    };
}

pub fn get_connection() -> Result<PooledConnection<SqliteConnectionManager>> {
    DB_POOL.get()
        .map_err(|e| anyhow!("Failed to get DB connection: {}", e))
}

pub fn init_db() -> Result<()> {
    let conn = get_connection()?;

    // Enable WAL mode (6.6× read speedup)
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -32000;  -- 32MB cache
         PRAGMA mmap_size = 1073741824;"  -- 1GB memory-mapped I/O
    )?;

    // Create tables if not exist
    conn.execute_batch(include_str!("schema.sql"))?;

    Ok(())
}
```

---

### Write Pattern (with retry)

```rust
pub async fn cache_agent_output(
    spec_id: &str,
    stage: &str,
    run_id: &str,
    output: &AgentOutput,
) -> Result<()> {
    let mut retry_delay = Duration::from_millis(50);

    for attempt in 0..5 {
        let conn = get_connection()?;

        match conn.execute(
            "INSERT INTO agent_outputs (spec_id, stage, run_id, agent_name, input_tokens, output_tokens, cost, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                spec_id,
                stage,
                run_id,
                output.agent.name,
                output.input_tokens,
                output.output_tokens,
                output.cost,
                output.content,
                now(),
            ],
        ) {
            Ok(_) => return Ok(()),
            Err(e) if e.to_string().contains("SQLITE_BUSY") && attempt < 4 => {
                // Retry with backoff
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;  // 50ms → 100ms → 200ms → 400ms → 800ms
            }
            Err(e) => {
                return Err(anyhow!("SQLite insert failed after 5 attempts: {}", e));
            }
        }
    }

    unreachable!()
}
```

**Retry Backoff**: 50ms, 100ms, 200ms, 400ms, 800ms (max 1.55s total)

---

### Read Pattern (query cached consensus)

```rust
pub async fn get_cached_consensus(
    spec_id: &str,
    stage: &str,
) -> Result<Consensus> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare(
        "SELECT synthesized_consensus, verdict_status, present_agents, missing_agents, conflicts, total_cost
         FROM consensus_runs
         WHERE spec_id = ?1 AND stage = ?2
         ORDER BY created_at DESC
         LIMIT 1"
    )?;

    let row = stmt.query_row(params![spec_id, stage], |row| {
        Ok((
            row.get::<_, String>(0)?,  // synthesized_consensus
            row.get::<_, String>(1)?,  // verdict_status
            row.get::<_, String>(2)?,  // present_agents (JSON)
            row.get::<_, String>(3)?,  // missing_agents (JSON)
            row.get::<_, Option<String>>(4)?,  // conflicts (JSON, nullable)
            row.get::<_, f64>(5)?,     // total_cost
        ))
    })?;

    Ok(Consensus {
        synthesized: row.0,
        verdict: VerdictStatus::from_str(&row.1)?,
        present_agents: serde_json::from_str(&row.2)?,
        missing_agents: serde_json::from_str(&row.3)?,
        conflicts: row.4.map(|s| serde_json::from_str(&s).unwrap_or_default()).unwrap_or_default(),
        total_cost: row.5,
    })
}
```

**Performance**: ~8.7ms typical (with indexes)

---

## Degradation Handling

### 2/3 Quorum Rule

**Principle**: Valid consensus requires at least 2/3 agents (if no conflicts)

**Example** (3 agents expected):

| Scenario | Present | Missing | Status | Action |
|----------|---------|---------|--------|--------|
| All 3 agents | 3 | 0 | Ok | Proceed |
| 2 of 3 agents | 2 | 1 | Degraded | Proceed + log warning |
| 1 of 3 agents | 1 | 2 | Unknown | HALT |
| 0 of 3 agents | 0 | 3 | Unknown | HALT |

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

---

### Fallback Chain

**3-Level Fallback**:
1. **SQLite** (8.7ms, <0.1% failure)
2. **MCP local-memory** (15ms, <1% failure)
3. **Evidence files** (50ms, 0% failure)

```rust
pub async fn get_consensus_robust(
    spec_id: &str,
    stage: &str,
) -> Result<Consensus> {
    // Try SQLite first
    if let Ok(consensus) = get_cached_consensus(spec_id, stage).await {
        return Ok(consensus);
    }

    // Fallback to MCP
    if let Ok(consensus) = query_mcp_consensus(spec_id, stage).await {
        // Cache to SQLite for next time
        let _ = cache_consensus_to_sqlite(spec_id, stage, &consensus).await;
        return Ok(consensus);
    }

    // Final fallback: evidence files
    let consensus = read_consensus_from_evidence(spec_id, stage)?;

    // Cache to SQLite
    let _ = cache_consensus_to_sqlite(spec_id, stage, &consensus).await;

    Ok(consensus)
}
```

---

## Performance Metrics

### Consensus Check Latency

**Native MCP** (current):
- **Typical**: 8.7ms (p50)
- **95th percentile**: 15ms (p95)
- **99th percentile**: 25ms (p99)

**Subprocess MCP** (old):
- **Typical**: 46ms (p50)
- **95th percentile**: 80ms (p95)
- **99th percentile**: 120ms (p99)

**Speedup**: 5.3× faster (46ms → 8.7ms)

---

### Agent Spawn Latency

**Parallel Spawn** (SPEC-933):
- **3 agents**: 50ms total
- **5 agents**: 65ms total

**Sequential Spawn** (old):
- **3 agents**: 150ms total (50ms × 3)
- **5 agents**: 250ms total (50ms × 5)

**Speedup**: 3× faster for 3 agents

---

### Database Performance

**Writes** (async, non-blocking):
- Agent output: ~0.9ms (p50)
- Consensus run: ~1.2ms (p50)

**Reads** (cached queries):
- Get consensus: ~8.7ms (p50)
- Get stage agents: ~5.2ms (p50)

**Total Overhead**: <100ms per full pipeline (6 stages)

---

## End-to-End Example

### Validate Stage (3 agents, parallel)

**Step 1: Agent Selection**

```rust
let agents = select_agents_for_tier(CommandTier::Tier2Multi, "validate");
// Returns: [gemini-flash, claude-haiku, gpt5-medium]
```

**Step 2: Parallel Execution**

```rust
let outputs = execute_parallel_consensus(agents, "SPEC-KIT-070", "validate").await?;
// Spawns 3 agents in parallel (50ms spawn time)
// Waits ~10 minutes for all to complete
```

**Step 3: Artifact Collection**

```rust
let artifacts = collect_consensus_artifacts("SPEC-KIT-070", "validate").await?;
// Tries SQLite (8.7ms) → MCP (15ms) → files (50ms)
```

**Step 4: MCP Synthesis**

```rust
let consensus = mcp_synthesize_consensus(&outputs, &artifacts).await?;
// Calls local-memory MCP server
// GPT-5 validation of 3 agent outputs
// Returns synthesized consensus + verdict
```

**Step 5: Verdict Computation**

```rust
let verdict = compute_verdict(&outputs, &agents);
// Status: Ok (all 3 agents, no conflicts)
// Present: [gemini-flash, claude-haiku, gpt5-medium]
// Missing: []
// Conflicts: []
```

**Step 6: Cache to SQLite**

```rust
cache_consensus("SPEC-KIT-070", "validate", &consensus).await?;
// Stores in consensus_runs table
// Stores individual outputs in agent_outputs table
```

**Step 7: Evidence Files**

```rust
write_evidence_files("SPEC-KIT-070", "validate", &outputs, &consensus)?;
// Creates:
// - validate_execution.json (metadata)
// - agent_1_gemini.txt (output)
// - agent_2_claude.txt (output)
// - agent_3_gpt5.txt (output)
// - consensus.json (synthesized result)
```

**Total Time**: ~10 minutes (parallel agent execution dominates)

**Total Cost**: ~$0.35 (3 agents @ ~$0.12 each)

---

## Summary

**Consensus System Highlights**:

1. **Tier-Based Routing**: Strategic agent selection by cost/complexity (Tier 0-4)
2. **Dual Patterns**: Sequential pipeline (iterative) vs parallel consensus (fast)
3. **Native MCP**: 5.3× faster than subprocess (8.7ms typical)
4. **3-Source Fallback**: SQLite → MCP → evidence files (robust)
5. **Verdict Computation**: 2/3 quorum, conflict detection, GPT-5 validation
6. **Response Caching**: SQLite with connection pooling, WAL mode, retry logic
7. **Graceful Degradation**: Continue with 2/3 agents, halt on conflicts

**Next Steps**:
- [Quality Gates](quality-gates.md) - Checkpoint validation details
- [Native Operations](native-operations.md) - FREE Tier 0 commands
- [Cost Tracking](cost-tracking.md) - Per-stage cost breakdown

---

**File References**:
- Routing: `codex-rs/tui/src/chatwidget/spec_kit/routing.rs:15-80`
- ACE selection: `codex-rs/tui/src/chatwidget/spec_kit/ace_route_selector.rs:25-120`
- Agent orchestration: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:439-920`
- MCP coordinator: `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs:47-180`
- Consensus synthesis: `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs:251-958`
- Database caching: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs:50-250`
