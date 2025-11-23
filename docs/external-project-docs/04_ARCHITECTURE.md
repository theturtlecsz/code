# Architecture: Spec-Kit Technical Design

## System Overview

Spec-Kit is built as a Rust workspace integrated into a Terminal User Interface (TUI). The architecture emphasizes:
- **Modularity**: Extracted spec-kit crate for reuse
- **Performance**: Native MCP integration (5.3x faster than subprocess)
- **Reliability**: Circuit breakers, retry logic, graceful degradation
- **Observability**: Full telemetry and evidence collection

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Terminal User Interface                   │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                   ChatWidget                         │    │
│  │  ┌───────────────────────────────────────────────┐  │    │
│  │  │              Spec-Kit Module                   │  │    │
│  │  │  ┌─────────┐ ┌─────────┐ ┌─────────────────┐  │  │    │
│  │  │  │ Routing │→│ Handler │→│ Pipeline Coord  │  │  │    │
│  │  │  └─────────┘ └─────────┘ └────────┬────────┘  │  │    │
│  │  │                                    ↓          │  │    │
│  │  │  ┌────────────────────────────────────────┐   │  │    │
│  │  │  │        Agent Orchestrator              │   │  │    │
│  │  │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐  │   │  │    │
│  │  │  │  │ Gemini  │ │ Claude  │ │  GPT-5  │  │   │  │    │
│  │  │  │  └────┬────┘ └────┬────┘ └────┬────┘  │   │  │    │
│  │  │  └───────│───────────│───────────│───────┘   │  │    │
│  │  └──────────│───────────│───────────│───────────┘  │    │
│  └─────────────│───────────│───────────│──────────────┘    │
└────────────────│───────────│───────────│───────────────────┘
                 ↓           ↓           ↓
         ┌───────────────────────────────────────┐
         │              MCP Layer                │
         │   ┌─────────────┐  ┌──────────────┐  │
         │   │local-memory │  │ consensus_db │  │
         │   └─────────────┘  └──────────────┘  │
         └───────────────────────────────────────┘
```

## Workspace Structure

### Crates

| Crate | Purpose | Key Dependencies |
|-------|---------|------------------|
| `spec-kit` | Core library (extracted) | config, retry, types |
| `tui` | Terminal UI + spec-kit integration | ratatui, crossterm |
| `core` | Client library, model providers | reqwest, tokio |
| `mcp-server/client/types` | MCP protocol implementation | serde, sqlx |

### Spec-Kit Module Layout

```
codex-rs/tui/src/chatwidget/spec_kit/
├── mod.rs                  # Module exports
├── routing.rs              # Command dispatch
├── handler.rs              # Main orchestrator
├── command_registry.rs     # Dynamic command registration
│
├── Pipeline
│   ├── pipeline_coordinator.rs   # Multi-stage orchestration
│   ├── pipeline_config.rs        # Configuration loading
│   └── state.rs                  # State machine
│
├── Agent Orchestration
│   ├── agent_orchestrator.rs     # Multi-agent spawning
│   ├── consensus.rs              # Result synthesis
│   └── ace_*.rs                  # Learning system
│
├── Quality Gates
│   ├── quality_gate_handler.rs   # Gate orchestration
│   ├── clarify_native.rs         # Ambiguity detection
│   ├── analyze_native.rs         # Consistency checking
│   └── checklist_native.rs       # Rubric scoring
│
├── Infrastructure
│   ├── consensus_db.rs           # SQLite storage
│   ├── evidence.rs               # Telemetry collection
│   └── cost_tracker.rs           # Budget management
│
└── Utilities
    ├── json_extractor.rs         # LLM output parsing
    ├── spec_id_generator.rs      # Native ID generation
    └── error.rs                  # Error types
```

## Core Components

### 1. Command Routing (`routing.rs`)

Receives commands from TUI and dispatches to appropriate handlers.

```rust
pub fn route_command(command: &str) -> Result<CommandResult> {
    let (name, args) = parse_command(command)?;

    // Check spec-kit registry first
    if let Some(handler) = SPEC_KIT_REGISTRY.get(&name) {
        return handler.execute(args);
    }

    // Fall through to upstream
    upstream_dispatch(command)
}
```

### 2. Command Registry (`command_registry.rs`)

Dynamic registration of 22 commands with 38 aliases.

```rust
lazy_static! {
    static ref SPEC_KIT_REGISTRY: HashMap<String, Box<dyn CommandHandler>> = {
        let mut m = HashMap::new();

        // Primary commands
        m.insert("/speckit.new", Box::new(SpecKitNewCommand));
        m.insert("/speckit.plan", Box::new(SpecKitPlanCommand));

        // Aliases
        m.insert("/new-spec", Box::new(SpecKitNewCommand));
        m.insert("/spec-plan", Box::new(SpecKitPlanCommand));

        m
    };
}
```

### 3. Pipeline Coordinator (`pipeline_coordinator.rs`)

Orchestrates multi-stage workflows with quality gates.

```rust
pub struct PipelineCoordinator {
    spec_id: String,
    config: PipelineConfig,
    state: SpecAutoState,
    cost_tracker: CostTracker,
}

impl PipelineCoordinator {
    pub async fn run(&mut self) -> Result<()> {
        // Quality Gate: Pre-Planning
        self.run_clarify_gate().await?;
        self.run_checklist_gate().await?;

        // Stage: Plan
        self.advance_to(SpecStage::Plan)?;
        self.execute_stage(SpecStage::Plan).await?;

        // Quality Gate: Post-Plan
        self.run_analyze_gate().await?;

        // Continue through stages...
        for stage in self.config.enabled_stages() {
            self.advance_to(stage)?;
            self.execute_stage(stage).await?;
        }

        Ok(())
    }
}
```

### 4. Agent Orchestrator (`agent_orchestrator.rs`)

Spawns and coordinates multiple AI agents.

```rust
pub struct AgentOrchestrator {
    agents: Vec<SpecAgent>,
    timeout: Duration,
    retry_config: RetryConfig,
}

impl AgentOrchestrator {
    pub async fn spawn_consensus(&self, spec_id: &str, stage: SpecStage) -> Result<ConsensusResult> {
        let mut handles = Vec::new();

        // Spawn agents in parallel
        for agent in &self.agents {
            let handle = tokio::spawn(async move {
                self.execute_agent(agent, spec_id, stage).await
            });
            handles.push(handle);
        }

        // Collect results
        let results = join_all(handles).await;

        // Synthesize consensus
        self.synthesize(results)
    }
}
```

### 5. Consensus System (`consensus.rs`)

Synthesizes multiple agent outputs into unified result.

```rust
pub struct ConsensusChecker {
    db: ConsensusDb,
    memory: LocalMemoryClient,
}

impl ConsensusChecker {
    pub async fn check(&self, results: Vec<AgentResult>) -> Result<Synthesis> {
        // Validate each result
        let valid_results: Vec<_> = results
            .into_iter()
            .filter(|r| r.is_valid())
            .collect();

        // Check for majority agreement
        let agreement = self.calculate_agreement(&valid_results)?;

        // Synthesize final output
        let synthesis = match agreement {
            Agreement::Unanimous => self.blend_unanimous(valid_results),
            Agreement::Majority => self.blend_majority(valid_results),
            Agreement::NoConsensus => self.escalate_to_user(valid_results),
        };

        // Store in database
        self.db.store_synthesis(&synthesis).await?;

        Ok(synthesis)
    }
}
```

### 6. Quality Gate Handler (`quality_gate_handler.rs`)

Executes quality checks and handles auto-resolution.

```rust
pub struct QualityGateHandler {
    gates: Vec<QualityGate>,
}

impl QualityGateHandler {
    pub async fn run_checkpoint(&self, checkpoint: Checkpoint) -> Result<GateResult> {
        let mut issues = Vec::new();

        for gate in &checkpoint.gates {
            let gate_issues = gate.execute().await?;
            issues.extend(gate_issues);
        }

        // Classify and resolve
        for issue in &mut issues {
            match issue.resolvability {
                Resolvability::AutoFix => {
                    self.apply_fix(issue).await?;
                }
                Resolvability::SuggestFix => {
                    self.suggest_to_user(issue);
                }
                Resolvability::NeedHuman => {
                    self.escalate(issue);
                }
            }
        }

        Ok(GateResult { issues })
    }
}
```

## Data Flow

### Command Execution Flow

```
User Input
    ↓
[Routing] → Parse command, extract args
    ↓
[Registry] → Find handler by command name
    ↓
[Handler] → Execute command logic
    ↓
[Orchestrator] → Spawn agents if needed
    ↓
[Consensus] → Synthesize results
    ↓
[Evidence] → Store telemetry
    ↓
Output to TUI
```

### Pipeline Flow

```
/speckit.auto SPEC-ID
    ↓
┌─────────────────────────┐
│ Pipeline Coordinator    │
│                         │
│ ┌─────────────────────┐ │
│ │ Quality Gate:       │ │
│ │ Clarify → Checklist │ │
│ └──────────┬──────────┘ │
│            ↓            │
│ ┌─────────────────────┐ │
│ │ Stage: Plan         │ │
│ │ (3 agents parallel) │ │
│ └──────────┬──────────┘ │
│            ↓            │
│ ┌─────────────────────┐ │
│ │ Quality Gate:       │ │
│ │ Analyze             │ │
│ └──────────┬──────────┘ │
│            ↓            │
│ [Tasks → Implement →    │
│  Validate → Audit →     │
│  Unlock]                │
└─────────────────────────┘
```

## Key Design Patterns

### 1. Context Trait (`context.rs`)

Abstracts TUI integration for testability.

```rust
pub trait SpecKitContext: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<String>;
    async fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    async fn submit_prompt(&self, prompt: &str) -> Result<String>;
    async fn execute_command(&self, cmd: &str) -> Result<()>;
}

// Real implementation
impl SpecKitContext for ChatWidget { ... }

// Test implementation
impl SpecKitContext for MockContext { ... }
```

### 2. State Machine (`state.rs`)

Tracks pipeline progress with explicit phases.

```rust
pub enum PipelinePhase {
    Guardrail,
    ExecutingAgents,
    CheckingConsensus,
    QualityGateExecuting,
    Completed,
    Failed,
}

pub struct SpecAutoState {
    current_stage: SpecStage,
    phase: PipelinePhase,
    active_agents: Vec<SpecAgent>,
    completed_stages: HashSet<SpecStage>,
}
```

### 3. Retry with Circuit Breaker

```rust
pub struct RetryConfig {
    max_retries: u32,
    backoff: ExponentialBackoff,
    circuit_breaker: CircuitBreaker,
}

impl AgentExecutor {
    async fn execute_with_retry(&self) -> Result<AgentResult> {
        let mut attempt = 0;

        loop {
            match self.execute().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < self.config.max_retries => {
                    attempt += 1;
                    let delay = self.config.backoff.next_delay(attempt);
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    self.config.circuit_breaker.record_failure();
                    return Err(e);
                }
            }
        }
    }
}
```

## Storage Architecture

### Consensus Database (SQLite)

```sql
CREATE TABLE consensus_results (
    id INTEGER PRIMARY KEY,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agent TEXT NOT NULL,
    model TEXT NOT NULL,
    content TEXT NOT NULL,
    tokens_in INTEGER,
    tokens_out INTEGER,
    cost_usd REAL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE synthesis_results (
    id INTEGER PRIMARY KEY,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agreement TEXT NOT NULL,
    content TEXT NOT NULL,
    agent_ids TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Evidence Storage (Filesystem)

```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
├── commands/
│   └── SPEC-KIT-065/
│       ├── plan-gemini-2025-10-27T10:15:00Z.json
│       ├── plan-claude-2025-10-27T10:15:00Z.json
│       ├── plan-gpt_pro-2025-10-27T10:16:00Z.json
│       ├── plan-synthesis-2025-10-27T10:18:00Z.json
│       └── telemetry.json
└── consensus/
    └── SPEC-KIT-065/
        └── plan.json
```

## Performance Considerations

### Native MCP vs Subprocess

**Before (Subprocess)**:
- Shell out to external tools
- Parse stdout/stderr
- ~50ms per operation

**After (Native MCP)**:
- Direct Rust FFI
- Structured data exchange
- ~8.7ms per operation (5.3x faster)

### Parallel Execution

```rust
// Good: Parallel agent execution
let handles: Vec<_> = agents
    .iter()
    .map(|a| tokio::spawn(execute_agent(a)))
    .collect();
let results = join_all(handles).await;

// Bad: Sequential execution
let mut results = Vec::new();
for agent in agents {
    results.push(execute_agent(agent).await);
}
```

### Memory Management

- Agent outputs buffered in memory until synthesis
- Large artifacts streamed to disk
- Evidence pruned after 90 days (configurable)

## Testing Infrastructure

### Test Coverage

- 604 tests, 100% pass rate
- 42-48% estimated code coverage
- Property-based testing with proptest

### Test Fixtures

Real consensus artifacts in `fixtures/` directory:
- `fixtures/consensus/` - Agent outputs
- `fixtures/specs/` - Example SPEC directories
- `fixtures/evidence/` - Telemetry examples

### MockContext

```rust
pub struct MockContext {
    files: HashMap<PathBuf, String>,
    commands: Vec<String>,
    prompts: Vec<String>,
}

impl SpecKitContext for MockContext {
    async fn read_file(&self, path: &Path) -> Result<String> {
        self.files.get(path).cloned().ok_or(Error::NotFound)
    }
    // ...
}
```

## Error Handling

### Error Types

```rust
pub enum SpecKitError {
    // User errors
    InvalidSpecId(String),
    SpecNotFound(String),
    MissingArtifact(String),

    // Agent errors
    AgentTimeout(SpecAgent),
    AgentFailure(SpecAgent, String),
    ConsensusFailed(String),

    // System errors
    IoError(std::io::Error),
    DatabaseError(sqlx::Error),
    McpError(mcp_client::Error),
}
```

### Recovery Strategies

| Error Type | Strategy |
|------------|----------|
| Agent Timeout | Retry 3x with backoff |
| Empty Result | Re-prompt with guidance |
| 1/3 Agent Failure | Continue with 2/3 |
| 2/3 Agent Failure | Escalate to user |
| MCP Error | Retry, fallback to filesystem |
