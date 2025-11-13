# Phase 1: Current System Inventory

**Date**: 2025-11-12
**Status**: In Progress
**Goal**: Comprehensive map of agent orchestration architecture

---

## Executive Summary

**Architecture Style**: Hybrid async/sync orchestration with dual storage (in-memory + SQLite)
**Primary Pattern**: tokio spawn → execute_agent() → tmux wrapper → output capture → validation → storage
**State Management**: AGENT_MANAGER (in-memory HashMap) + consensus_db (SQLite) - dual-write pattern
**Key Insight**: System is **not ACID-compliant** - dual writes without transactions create consistency risks

**Critical Files Analyzed**: 6 files, ~6,000 LOC
- agent_tool.rs (1,854 lines)
- tmux.rs (786 lines)
- consensus_db.rs (575 lines)
- native_quality_gate_orchestrator.rs (319 lines)
- quality_gate_handler.rs (1,791 lines)
- quality_gate_broker.rs (687 lines)

---

## 1. Component Inventory

### 1.1 Agent Execution Layer

**agent_tool.rs** (1,854 LOC) - Core agent lifecycle management

**Primary Responsibilities**:
- Agent state machine (Pending → Running → Completed/Failed/Cancelled)
- Agent spawning and lifecycle tracking
- Output validation (JSON extraction, schema checks, size validation)
- Result storage and error handling

**Key Structures**:
```rust
// Global singleton - in-memory HashMap
lazy_static! {
    pub static ref AGENT_MANAGER: Arc<RwLock<AgentManager>> = ...
}

struct AgentManager {
    agents: HashMap<String, Agent>,  // In-memory state
    handles: HashMap<String, JoinHandle<()>>,  // tokio task handles
    event_sender: Option<mpsc::UnboundedSender<AgentStatusUpdatePayload>>,
}

struct Agent {
    id: String,
    batch_id: Option<String>,
    model: String,
    prompt: String,
    status: AgentStatus,  // Enum: Pending/Running/Completed/Failed/Cancelled
    result: Option<String>,
    error: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    progress: Vec<String>,
    tmux_enabled: bool,  // SPEC-KIT-923 feature flag
    config: Option<AgentConfig>,
    // ... 16 fields total
}
```

**Critical Methods**:
- `create_agent_from_config_name()` - Spawn agent via config lookup (SPEC-KIT-900)
- `execute_agent()` - Main async execution function (spawned as tokio task)
- `execute_model_with_permissions()` - Actual model invocation (tmux or direct)
- `extract_json_from_mixed_output()` - Multi-strategy JSON extraction (SPEC-KIT-927+)
- `update_agent_result()` - Store completion/failure (SPEC-KIT-928: stores raw output on validation failure)

**SPEC-928 Improvements** (10 bugs fixed):
1. Validation failure now stores raw output (lines 404-423)
2. Duplicate spawn prevention via `check_concurrent_agents()` (lines 437-461)
3. JSON extractor strips Codex metadata (lines 560-667)
4. Extractor detects prompt schema vs real response (lines 605-657)
5. UTF-8 panic prevention via char-aware slicing (lines 562-567)
6. Fallback pane capture handles code agent pattern (lines 521-527)
7. Both Completed and Failed states recorded to SQLite (lines 261-263 in orchestrator)
8. Double completion marker fixed in tmux.rs wrapper (tmux.rs:293-297)
9. Wait status logging added (orchestrator lines 294-305)
10. Schema template false positive detection (lines 913-925)

**Validation Pipeline** (lines 837-1003):
```
1. Corruption detection (TUI text, conversation fragments)
2. Headers-only check (Codex initialization without response)
3. Minimum size check (>500 bytes required)
4. Schema template detection (": string" patterns)
5. JSON parsing validation (must parse as valid JSON)
```

**Product Questions**:
- **Q1**: Why dual-write AGENT_MANAGER + SQLite without transactions? (Lines 62-64, 141-143)
  - Risk: Crash between writes leaves inconsistent state
  - Evidence: No transaction coordination, separate update calls
- **Q2**: Why spawn tokio tasks directly instead of actor supervision? (Lines 288-294)
  - No automatic restart on crash
  - No parent-child lifecycle management
  - Manual handle tracking required

---

### 1.2 Observable Execution Layer

**tmux.rs** (786 LOC) - Tmux wrapper for observable agent runs

**Purpose**: Enable real-time agent monitoring and debugging via tmux panes

**Key Features**:
- Session management with 5-minute staleness detection (SPEC-KIT-925, lines 27-103)
- Wrapper script generation for large prompts (>1000 chars, lines 169-260)
- Completion marker polling (`___AGENT_COMPLETE___`)
- File stability detection (2s stable period, 1KB minimum, lines 351-423)
- Output file redirection (`/tmp/tmux-agent-output-{pid}-{pane}.txt`)

**Critical Functions**:
- `execute_in_pane()` - Main execution with output capture (lines 159-558)
- `ensure_session()` - Session lifecycle with staleness check (lines 27-103)
- `wait_for_completion()` - Poll loop with file stability (lines 346-557)

**SPEC-KIT-928 Fix: Double Completion Marker** (KEY BUG):
```rust
// Lines 293-297: Only add external marker for direct commands
let has_wrapper = !temp_files.is_empty();
if !has_wrapper {
    final_command.push_str("; echo '___AGENT_COMPLETE___'");
}
// Wrapper scripts already have marker internally (line 229)
```

**Problem**: Wrapper scripts had marker added TWICE:
- **Internal**: Inside wrapper after command finishes (77s typical)
- **External**: Appended to tmux command, fires immediately (1s)
- **Result**: Polling detected marker too early, captured only prompt (1,281 bytes instead of 23KB)

**Wrapper Script Approach** (lines 181-260):
```bash
#!/bin/bash
set -e

export KEY='value'
cd /path

command "$(cat <<'PROMPT_HEREDOC_EOF'
{large prompt content}
PROMPT_HEREDOC_EOF
)" > /tmp/output.txt 2>&1

echo '___AGENT_COMPLETE___'
```

**Benefits**:
- No command length limits (bash heredoc handles any size)
- Perfect content preservation (no escaping issues)
- Environment isolation
- Observable via `tmux attach`

**File Stability Detection** (lines 351-423):
```rust
// Track file size changes
last_file_size: Option<u64>
stable_since: Option<Instant>
min_stable_duration: 2 seconds
min_file_size: 1KB

// Only read when BOTH:
// 1. Completion marker present in pane
// 2. File size stable for 2+ seconds at >1KB
```

**Product Questions**:
- **Q3**: Can we eliminate tmux entirely with direct async API calls? (Referenced in SPEC-930)
  - Benefits: No pane management, faster spawn (<100ms vs ~1s), simpler code
  - Risks: Lose observable execution, harder debugging
  - Blocker: OAuth2 flows require interactive prompts (device code)
- **Q4**: Is 2-second stability window optimal? (Line 354)
  - Too short: Premature collection risk
  - Too long: Unnecessary latency
  - Current: Conservative (prevents SPEC-928 issues)

---

### 1.3 Persistence Layer

**consensus_db.rs** (575 LOC) - SQLite storage for consensus artifacts

**Purpose**: Separate workflow artifacts from curated knowledge (SPEC-KIT-072)

**Database Schema**:

```sql
-- Agent execution tracking (definitive routing)
CREATE TABLE agent_executions (
    agent_id TEXT PRIMARY KEY,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    phase_type TEXT NOT NULL,  -- "quality_gate" | "regular_stage"
    agent_name TEXT NOT NULL,
    run_id TEXT,
    spawned_at TEXT NOT NULL,
    completed_at TEXT,
    response_text TEXT,
    extraction_error TEXT  -- SPEC-KIT-927: stores raw output on failure
);

-- Consensus artifacts (agent outputs)
CREATE TABLE consensus_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    content_json TEXT NOT NULL,
    response_text TEXT,
    run_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Consensus synthesis (final outputs)
CREATE TABLE consensus_synthesis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    output_markdown TEXT NOT NULL,
    output_path TEXT,
    status TEXT NOT NULL,
    artifacts_count INTEGER,
    agreements TEXT,
    conflicts TEXT,
    degraded BOOLEAN DEFAULT 0,
    run_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

**Key Features**:
- Default location: `~/.code/consensus_artifacts.db` (line 40)
- Thread-safe via `Arc<Mutex<Connection>>` (lines 33-35)
- Agent spawn recording for definitive routing (lines 339-366)
- Extraction failure logging with raw output (lines 420-438)
- Cleanup operations for old executions (lines 474-482)

**Critical Methods**:
- `record_agent_spawn()` - Track agent launch for routing (lines 339-366)
- `get_agent_spawn_info()` - Retrieve phase_type for completion routing (lines 368-383)
- `record_agent_completion()` - Store successful output (lines 403-414)
- `record_extraction_failure()` - Store raw output on JSON extraction failure (lines 420-438)
- `query_extraction_failures()` - Debug failed extractions (lines 443-471)

**Usage Pattern**:
```rust
// Spawn: Record to DB
db.record_agent_spawn(agent_id, spec_id, stage, "quality_gate", "gemini", run_id)?;

// Completion: Update with output
db.record_agent_completion(agent_id, response_text)?;

// OR Failure: Store raw output
db.record_extraction_failure(agent_id, raw_output, error_msg)?;
```

**Product Questions**:
- **Q5**: Why separate SQLite DB instead of MCP local-memory? (Lines 1-11 comment)
  - Answer: Avoid polluting knowledge base with transient workflow data
  - Benefit: Proper lifecycle (delete rows vs memories)
  - Concern: Now have THREE storage systems (AGENT_MANAGER, SQLite, MCP)
- **Q6**: Is SQLite the right choice vs event log? (SPEC-930 suggests event sourcing)
  - Current: Direct state updates (UPDATE agent_executions SET ...)
  - Alternative: Event log (INSERT INTO event_log VALUES ...)
  - Trade-off: Simplicity vs ACID compliance + time-travel

---

### 1.4 Quality Gate System

**native_quality_gate_orchestrator.rs** (319 LOC) - Native orchestration (SPEC-KIT-900)

**Purpose**: Eliminate LLM orchestrator overhead for quality gate spawning

**Key Innovation**: Native Rust code spawns agents directly (no Python, no orchestrator LLM)

**Agent Configuration** (lines 68-72):
```rust
// SPEC-KIT-070 Tier 2: Cheapest models for quality gates
vec![
    ("gemini", "gemini_flash"),    // gemini-2.5-flash
    ("claude", "claude_haiku"),    // claude-haiku
    ("code", "gpt_low"),            // gpt-5 low reasoning
]
```

**Spawn Flow** (lines 99-189):
```rust
for (agent_name, config_name) in agent_spawn_configs {
    // 1. Load prompt template from prompts.json
    let prompt_template = gate_prompts[agent_name]["prompt"];

    // 2. Build prompt with SPEC context (spec.md + PRD.md)
    let prompt = build_quality_gate_prompt(spec_id, gate, prompt_template, cwd)?;

    // 3. Spawn via AGENT_MANAGER
    let agent_id = manager.create_agent_from_config_name(
        config_name, agent_configs, prompt, true, batch_id, true
    )?;

    // 4. Record spawn to SQLite for routing
    db.record_agent_spawn(agent_id, spec_id, stage, "quality_gate", agent_name, run_id)?;
}
```

**Wait Loop** (lines 236-314):
```rust
loop {
    // Check all agents for completion
    for agent_id in agent_ids {
        let agent = AGENT_MANAGER.read().await.get_agent(agent_id);
        match agent.status {
            Completed | Failed => {
                // Record to SQLite (lines 264-281)
                db.record_agent_completion(agent_id, result);
                recorded_completions.insert(agent_id);
            }
            _ => still_running.push(agent_id)
        }
    }

    // Log wait status every 10s (SPEC-KIT-928 fix #9)
    if !still_running.is_empty() && elapsed % 10 == 0 {
        log!("Waiting for {} agents: {}", still_running.len(), running_summary);
    }

    if all_done { return Ok(()); }
    sleep(500ms).await;
}
```

**SPEC-KIT-928 Duplicate Prevention** (lines 78-96, 179-187):
```rust
// Pre-spawn check
let running_agents = manager.get_running_agents();
log!("Pre-spawn: {} agents running", running_agents.len());

// Post-spawn check
let concurrent = manager.check_concurrent_agents();
for (model, count) in concurrent {
    if count > 1 {
        warn!("CONCURRENT: {} instances of '{}' running!", count, model);
    }
}
```

**Product Questions**:
- **Q7**: Why fixed 3-agent spawn instead of configurable? (Lines 68-72)
  - Hardcoded: gemini, claude, code
  - Alternative: Load from config.toml [[agents]]
  - Current rationale: Cost optimization (SPEC-KIT-070)
- **Q8**: Why 500ms poll interval? (Line 312)
  - Trade-off: Responsiveness vs CPU usage
  - Too fast: Wastes CPU checking repeatedly
  - Too slow: Adds latency to completion detection

---

**quality_gate_handler.rs** (1,791 LOC) - Event handlers and workflow

**Purpose**: TUI event handlers for quality gate lifecycle

**Key Phases**:
1. `on_quality_gate_agents_complete()` - Store artifacts to local-memory (lines 29-138)
2. `on_quality_gate_broker_result()` - Process collected artifacts (lines 140-264)
3. `process_quality_gate_agent_results()` - Parse and classify issues (lines 266-550)
4. `on_quality_gate_validation_result()` - GPT-5 validation handling (lines 648-724)
5. `on_quality_gate_answers()` - Human answers applied (lines 553-642)

**Artifact Storage** (STEP 3, lines 1540-1667):
```rust
// Synchronous storage to ensure completion before broker searches
fn store_quality_gate_artifacts_sync(widget, spec_id, checkpoint, gate_names) -> usize {
    // 1. Scan .code/agents/ for completed agents
    let completed_agents = get_completed_quality_gate_agents(widget);

    // 2. Read result.txt files
    let content = fs::read_to_string(&result_path)?;

    // 3. Extract and validate JSON (SPEC-KIT-927)
    let json = extract_and_validate_quality_gate(&content, agent_name)?;

    // 4. Store to local-memory via MCP (async tasks)
    let handle = tokio::spawn(store_artifact_async(mcp, spec_id, agent, json));

    // 5. Wait for all storage (block_in_place)
    block_in_place(|| handle.await)
}
```

**Issue Classification** (lines 363-395):
```rust
// ACE Framework integration
let ace_bullets = state.ace_bullets_cache;

for issue in merged_issues {
    if should_auto_resolve_with_ace(&issue, ace_bullets) {
        auto_resolvable.push(issue);  // High confidence + ACE match
    } else if issue.confidence == Medium {
        needs_validation.push(issue);  // GPT-5 validation required
    } else {
        escalate_to_human.push(issue);  // Low confidence or critical
    }
}
```

**GPT-5 Validation Spawn** (lines 889-996):
```rust
// SPEC-KIT-927: Direct spawn instead of LLM tool call
tokio::spawn(async move {
    let agent_id = AGENT_MANAGER.write().await.create_agent_from_config_name(
        "gpt5-medium",  // Not gpt_pro - medium reasoning for validation
        prompt,
        true,  // read_only
        batch_id,
        false  // No tmux for single agent
    )?;

    // Wait up to 5 minutes
    loop {
        if agent.status == Completed { break; }
        sleep(500ms);
    }
});
```

**Product Questions**:
- **Q9**: Why block_in_place for artifact storage? (Lines 1642-1664)
  - Benefit: Ensures storage completes before broker searches
  - Cost: Blocks tokio thread during MCP calls (~200ms delay acceptable)
  - Alternative: Async coordination via channels (more complex)
- **Q10**: Why scan filesystem instead of use widget.active_agents? (Lines 1676-1740)
  - Answer: Sub-agents spawned by orchestrator not tracked in widget
  - Consequence: Polling filesystem is only source of truth
  - Alternative: Track all agents globally (requires architecture change)

---

**quality_gate_broker.rs** (687 LOC) - Async artifact retrieval

**Purpose**: Off-load MCP calls from Ratatui UI thread to prevent blocking

**Architecture**:
```rust
struct QualityGateBroker {
    sender: mpsc::UnboundedSender<QualityGateCommand>
}

// Background task processes commands
tokio::spawn(async move {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            FetchAgentPayloads => {
                let result = fetch_agent_payloads(...).await;
                event_tx.send(AppEvent::QualityGateResults { result });
            }
            FetchValidationPayload => { ... }
        }
    }
});
```

**Two Collection Paths**:

**Path 1: From AGENT_MANAGER Memory** (lines 214-403, native orchestrator):
```rust
async fn fetch_agent_payloads_from_memory(spec_id, checkpoint, expected_agents, agent_ids) {
    let manager = AGENT_MANAGER.read().await;

    for agent_id in agent_ids {
        if let Some(agent) = manager.get_agent(agent_id) {
            if let Some(result_text) = &agent.result {
                // Extract JSON (SPEC-KIT-927 robust extraction)
                let extraction = extract_and_validate_quality_gate(result_text, agent_id)?;

                // Match agent name to expected list
                if expected_agents.contains(&extraction.json["agent"]) {
                    results_map.insert(agent_name, payload);
                }
            }
        }
    }

    // 2/3 consensus acceptable (degraded mode)
    let min_required = if expected >= 3 { 2 } else { expected.len() };
    let valid = results_map.len() >= min_required;
}
```

**Path 2: From Filesystem** (lines 405-573, legacy LLM orchestrator):
```rust
async fn fetch_agent_payloads_from_filesystem(spec_id, checkpoint, expected_agents) {
    let agents_dir = ".code/agents";

    for entry in read_dir(agents_dir)? {
        let result_path = entry.path().join("result.txt");

        // Only scan recent files (<1 hour old)
        if metadata.modified().elapsed() > 3600s { continue; }

        let content = fs::read_to_string(result_path)?;
        let extraction = extract_and_validate_quality_gate(&content, "filesystem")?;

        // Check if quality gate artifact
        if extraction.json["stage"].starts_with("quality-gate-") {
            results_map.insert(agent_name, payload);
        }
    }

    // Limit scan to 100 agents max (prevent stack overflow)
}
```

**Retry Strategy** (lines 20-22):
```rust
const RETRY_DELAYS_MS: [u64; 3] = [100, 200, 400];  // Exponential backoff
const MIN_PARTICIPATING_AGENTS: usize = 2;           // 2/3 consensus
```

**Product Questions**:
- **Q11**: Why two collection paths (memory + filesystem)? (Lines 214-573)
  - Native: Read from AGENT_MANAGER (fast, in-memory)
  - Legacy: Scan filesystem (slow, fallback for LLM orchestrator)
  - Duplication: Same extraction logic in both paths
  - Opportunity: Consolidate to single path after tmux removal
- **Q12**: Why 100 agent scan limit? (Line 436)
  - Protection: Prevent stack overflow with many agents
  - Assumption: Quality gates only spawn 3 agents
  - Risk: Legacy agents from other runs could cause issues

---

## 2. Data Flow Analysis

### 2.1 Agent Spawn Flow (Quality Gate)

```
User Invokes /speckit.auto SPEC-ID
    ↓
quality_gate_handler::execute_quality_checkpoint()
    ↓
native_quality_gate_orchestrator::spawn_quality_gate_agents_native()
    ├→ Load prompts from prompts.json
    ├→ Build prompt with SPEC context (spec.md + PRD.md)
    ├→ For each (gemini, claude, code):
    │   ├→ AGENT_MANAGER.create_agent_from_config_name()
    │   │   ├→ Create Agent struct (status: Pending)
    │   │   ├→ Insert into agents: HashMap<String, Agent>
    │   │   ├→ tokio::spawn(execute_agent(agent_id, config))
    │   │   └→ Store JoinHandle in handles: HashMap
    │   └→ consensus_db.record_agent_spawn()
    │       └→ INSERT INTO agent_executions (agent_id, phase_type="quality_gate")
    └→ Return Vec<AgentSpawnInfo>
```

**State Mutations**:
1. AGENT_MANAGER.agents[agent_id] = Agent { status: Pending }
2. AGENT_MANAGER.handles[agent_id] = JoinHandle<()>
3. SQLite agent_executions: INSERT agent_id, phase_type, spawned_at

**Product Question**:
- **Q13**: Why not use transaction to coordinate HashMap + SQLite writes?
  - Current: Separate operations, no rollback if second fails
  - Risk: Crash between writes leaves orphaned task or DB record
  - SPEC-930 Solution: Event sourcing eliminates dual-write

---

### 2.2 Agent Execution Flow

```
tokio::spawn(execute_agent(agent_id, config))
    ↓
AGENT_MANAGER.update_agent_status(agent_id, Running)
    ↓
execute_model_with_permissions(model, prompt, read_only, working_dir, config, tmux_enabled)
    ├→ IF tmux_enabled:
    │   ├→ tmux::ensure_session("agents-{model}")
    │   ├→ tmux::create_pane(session, model, false)  // SPEC-923: always split new
    │   └→ tmux::execute_in_pane(session, pane_id, command, args, env, cwd, 600s)
    │       ├→ IF prompt >1000 chars:
    │       │   ├→ Write wrapper script with heredoc
    │       │   ├→ chmod +x wrapper.sh
    │       │   └→ Execute: bash wrapper.sh
    │       │       ├→ Command output → /tmp/tmux-agent-output-{pid}-{pane}.txt
    │       │       └→ echo '___AGENT_COMPLETE___'  (internal marker)
    │       └→ Poll loop (500ms intervals):
    │           ├→ Check pane for ___AGENT_COMPLETE___ marker
    │           ├→ Check file size stability (2s stable, >1KB)
    │           └→ IF both true: read output file, return
    │
    └→ ELSE:
        └→ spawn_child_async() → wait_with_output()
    ↓
Validation Phase:
    ├→ extract_json_from_mixed_output(output, model)
    │   ├→ Pattern 1: Markdown fence ```json {...} ```
    │   ├→ Pattern 2: Codex headers with "] codex\n" marker
    │   └→ Pattern 3: Plain JSON (no extraction)
    ├→ Validate: Corruption detection (TUI text, headers-only)
    ├→ Validate: Minimum size (>500 bytes)
    ├→ Validate: Schema template detection (": string" patterns)
    └→ Validate: JSON parsing (serde_json::from_str)
    ↓
AGENT_MANAGER.update_agent_result(agent_id, result)
    ├→ IF Ok: agent.status = Completed, agent.result = Some(output)
    └→ IF Err: agent.status = Failed, agent.error = Some(error)
                agent.result = Some(raw_output)  // SPEC-KIT-928: store raw even on failure
```

**State Mutations**:
1. AGENT_MANAGER.agents[agent_id].status = Running
2. AGENT_MANAGER.agents[agent_id].started_at = Utc::now()
3. Tmux: Create pane, execute command, write output file
4. AGENT_MANAGER.agents[agent_id].status = Completed | Failed
5. AGENT_MANAGER.agents[agent_id].result = Some(output)
6. AGENT_MANAGER.agents[agent_id].completed_at = Utc::now()

**Critical Timing** (SPEC-KIT-928 double marker bug):
```
00:00 - Wrapper script starts
00:01 - External marker fires (wrong! from tmux send-keys)
00:27 - Polling detects marker, file=1,281 bytes (TOO EARLY)
00:27 - Reads file prematurely (only prompt schema)
00:77 - Code exec ACTUALLY finishes (23KB output ready)
```

**Fix**: Only add external marker for direct commands, not wrappers (tmux.rs:293-297)

---

### 2.3 Quality Gate Collection Flow

```
quality_gate_handler::on_quality_gate_agents_complete()
    ↓
store_quality_gate_artifacts_sync(widget, spec_id, checkpoint, gate_names)
    ├→ get_completed_quality_gate_agents(widget)
    │   └→ Scan .code/agents/ for result.txt files
    ├→ For each agent:
    │   ├→ Read result.txt
    │   ├→ extract_and_validate_quality_gate(content, agent_name)
    │   └→ tokio::spawn(store_artifact_async())
    │       └→ MCP: local-memory.store_memory(json, domain="spec-kit", importance=8)
    └→ block_in_place: Wait for all storage (timeout: 15s)
    ↓
quality_gate_broker.fetch_agent_payloads_from_memory(spec_id, checkpoint, expected, agent_ids)
    ├→ AGENT_MANAGER.read().await
    ├→ For each agent_id:
    │   ├→ Get agent.result
    │   ├→ extract_and_validate_quality_gate(result)
    │   └→ Match agent_name to expected_agents list
    ├→ Validate: 2/3 consensus (min_required)
    └→ AppEvent::QualityGateResults { broker_result }
    ↓
quality_gate_handler::on_quality_gate_broker_result(widget, broker_result)
    ↓
process_quality_gate_agent_results(widget, checkpoint, payloads)
    ├→ For each payload: parse_quality_issue_from_agent()
    ├→ Classify issues:
    │   ├→ High confidence + ACE match → auto_resolvable
    │   ├→ Medium confidence → needs_validation (GPT-5)
    │   └→ Low confidence | critical → escalate_to_human
    ├→ Apply auto-resolutions
    └→ IF needs_validation:
        ├→ submit_gpt5_validations()
        │   └→ Spawn gpt5-medium agent directly
        └→ quality_gate_broker.fetch_validation_payload()
```

**Storage Systems Used**:
1. **AGENT_MANAGER** (in-memory) - Agent state during execution
2. **SQLite consensus_db** - Agent spawn tracking + completion records
3. **Filesystem** (.code/agents/{id}/result.txt) - Temporary output files
4. **Local-memory MCP** - Persistent consensus artifacts (importance=8)

**Product Question**:
- **Q14**: Why 4 storage systems for one workflow?
  - AGENT_MANAGER: Volatile, fast, tokio coordination
  - SQLite: Persistent, queryable, survives restarts
  - Filesystem: Observable, debugging, tmux output capture
  - MCP: Searchable, tagged, domain="spec-kit"
  - Alternative: SPEC-930 proposes event log as single source of truth

---

## 3. Database Schema Analysis

### 3.1 Current Schema (consensus_artifacts.db)

**agent_executions** (definitive routing table):
```sql
CREATE TABLE agent_executions (
    agent_id TEXT PRIMARY KEY,           -- UUID
    spec_id TEXT NOT NULL,               -- SPEC-KIT-###
    stage TEXT NOT NULL,                 -- "plan" | "tasks" | "implement" | etc.
    phase_type TEXT NOT NULL,            -- "quality_gate" | "regular_stage"
    agent_name TEXT NOT NULL,            -- "gemini" | "claude" | "code"
    run_id TEXT,                         -- Session identifier
    spawned_at TEXT NOT NULL,            -- ISO timestamp
    completed_at TEXT,                   -- NULL until completion
    response_text TEXT,                  -- Agent output (may be large)
    extraction_error TEXT                -- Error message if extraction failed
);

CREATE INDEX idx_agent_executions_spec ON agent_executions(spec_id, stage);
CREATE INDEX idx_agent_executions_run ON agent_executions(run_id);
```

**Usage Pattern**:
```rust
// Spawn: Write phase_type for routing
INSERT INTO agent_executions (agent_id, phase_type, ...) VALUES (?, "quality_gate", ...);

// Completion: Check phase_type to route correctly
SELECT phase_type FROM agent_executions WHERE agent_id = ?;
// If phase_type == "quality_gate": Route to quality_gate_handler
// If phase_type == "regular_stage": Route to stage completion handler

// Completion: Store output
UPDATE agent_executions SET completed_at=now(), response_text=? WHERE agent_id=?;

// Failure: Store raw output + error
UPDATE agent_executions SET completed_at=now(), response_text=?, extraction_error=? WHERE agent_id=?;
```

**Product Questions**:
- **Q15**: Why store full response_text in SQLite?
  - Size: Quality gate responses can be 10-20KB
  - Usage: Queried for extraction failure debugging
  - Alternative: Store only in-memory, write to files on failure
- **Q16**: Why TEXT timestamps instead of INTEGER epoch?
  - Current: ISO strings like "2025-11-12 02:38:16"
  - Benefit: Human-readable in DB browser
  - Cost: Slower comparisons, larger storage

---

**consensus_artifacts** (agent outputs):
```sql
CREATE TABLE consensus_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,                 -- "plan" | "tasks" | etc.
    agent_name TEXT NOT NULL,            -- "gemini" | "claude" | "code"
    content_json TEXT NOT NULL,          -- Full agent response (JSON)
    response_text TEXT,                  -- Original text before extraction
    run_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_spec_stage ON consensus_artifacts(spec_id, stage);
```

**Usage Pattern**:
```rust
// Store artifact after consensus
INSERT INTO consensus_artifacts (spec_id, stage, agent_name, content_json, ...) VALUES (...);

// Query for consensus synthesis
SELECT content_json FROM consensus_artifacts WHERE spec_id=? AND stage=? ORDER BY created_at DESC;

// Cleanup after SPEC complete
DELETE FROM consensus_artifacts WHERE spec_id = ?;
```

**Product Questions**:
- **Q17**: Why separate agent_executions + consensus_artifacts tables?
  - agent_executions: Tracking (phase_type, routing)
  - consensus_artifacts: Results (content_json, synthesis)
  - Overlap: Both store response_text
  - Alternative: Single table with all data
- **Q18**: Why no foreign key from consensus_artifacts to agent_executions?
  - Current: agent_name (string) matching, no referential integrity
  - Risk: Orphaned artifacts if agent_executions deleted
  - Benefit: Looser coupling allows independent cleanup

---

**consensus_synthesis** (final outputs):
```sql
CREATE TABLE consensus_synthesis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    output_markdown TEXT NOT NULL,       -- Final synthesized output
    output_path TEXT,                    -- File path if written
    status TEXT NOT NULL,                -- "success" | "degraded" | "failed"
    artifacts_count INTEGER,             -- How many agents contributed
    agreements TEXT,                     -- JSON: shared points
    conflicts TEXT,                      -- JSON: disagreements
    degraded BOOLEAN DEFAULT 0,          -- 2/3 consensus vs 3/3
    run_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_synthesis_spec_stage ON consensus_synthesis(spec_id, stage);
```

**Usage Pattern**:
```rust
// Store final synthesis
INSERT INTO consensus_synthesis (spec_id, stage, output_markdown, status, degraded, ...) VALUES (...);

// Query latest synthesis for stage
SELECT output_markdown FROM consensus_synthesis
WHERE spec_id=? AND stage=? ORDER BY created_at DESC LIMIT 1;
```

**Product Questions**:
- **Q19**: Is consensus_synthesis actually used?
  - Usage: Store synthesis operation results
  - Queries: Fetch latest synthesis (lines 314-335)
  - Observation: No evidence of synthesis reads in orchestration flow
  - Alternative: Remove table if unused, or implement proper consensus synthesis
- **Q20**: Why TEXT for agreements/conflicts instead of separate table?
  - Current: JSON strings in TEXT columns
  - Benefit: Simple, single row per synthesis
  - Cost: Can't query "show all agreements for SPEC-ID"

---

### 3.2 Schema Gaps vs SPEC-930 Event Sourcing

**Current: Direct State Updates**
```sql
UPDATE agent_executions SET status='completed', response_text=? WHERE agent_id=?;
```

**SPEC-930: Event Log Pattern**
```sql
-- Immutable append-only log
CREATE TABLE event_log (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL,  -- 'AgentQueued' | 'AgentStarted' | 'AgentCompleted'
    event_data JSON NOT NULL,  -- Event-specific payload
    timestamp INTEGER NOT NULL,
    sequence_number INTEGER NOT NULL
);

-- Current state derived from events
CREATE TABLE agent_snapshots (
    agent_id TEXT PRIMARY KEY,
    state_json JSON NOT NULL,
    last_event_id INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (last_event_id) REFERENCES event_log(event_id)
);
```

**Benefits of Event Sourcing**:
1. **ACID compliance**: Events are immutable, no UPDATE races
2. **Time-travel debugging**: Replay to any point in history
3. **Audit trail**: Complete lifecycle record
4. **Crash recovery**: Replay events to rebuild state

**Costs**:
1. **Complexity**: Replay engine, snapshot management
2. **Performance**: Replay overhead vs direct state reads
3. **Storage**: Events + snapshots vs single state table

**Product Question**:
- **Q21**: Should we migrate to event sourcing?
  - Current pain: Dual-write, no transactions, state corruption risk
  - SPEC-930 recommendation: Event sourcing with snapshots
  - Decision criteria: Does time-travel justify complexity?
  - Scale consideration: At 500 agents/day, how large does event log grow?

---

## 4. Configuration Surface

### 4.1 Agent Configuration (config.toml)

```toml
[[agents]]
name = "gemini_flash"
command = "gemini"
enabled = true
args = ["-m", "gemini-2.5-flash"]
env = { GEMINI_API_KEY = "${GEMINI_API_KEY}" }

[[agents]]
name = "claude_haiku"
command = "claude"
enabled = true
args = ["-m", "claude-haiku-4-5"]
env = { ANTHROPIC_API_KEY = "${ANTHROPIC_API_KEY}" }

[[agents]]
name = "gpt_low"
command = "code"
enabled = true
args = ["exec", "--sandbox", "read-only", "--model", "gpt-5", "-c", "model_reasoning_effort=\"low\""]
```

**Usage**:
```rust
// Lookup config by name
manager.create_agent_from_config_name("gemini_flash", agent_configs, prompt, read_only, batch_id, tmux)
    ├→ Find config where config.name == "gemini_flash"
    ├→ Use config.command as base command ("gemini")
    ├→ Use config.args for execution
    └→ Use config.env for environment variables
```

**Product Questions**:
- **Q22**: Why separate config.command from agent_name?
  - Example: config.command="gemini" but agent spawned as "gemini-2.5-flash"
  - Confusion: Agent matching uses lowercase startsWith()
  - Alternative: Use config.name directly as agent identifier
- **Q23**: Can we dynamically add agents without restart?
  - Current: config.toml read at startup
  - Limitation: Adding new model requires restart
  - SPEC-930: Config should be hot-reloadable

---

### 4.2 Environment Variables

**Execution Control**:
- `SPEC_OPS_TELEMETRY_HAL` - Enable HAL telemetry collection
- `SPEC_OPS_ALLOW_DIRTY` - Allow guardrail runs with dirty tree
- `SPEC_OPS_HAL_SKIP` - Skip HAL validation if secrets unavailable
- `SPEC_OPS_CARGO_MANIFEST` - Cargo workspace location

**Logging**:
- `RUST_LOG` - Tracing level (filtered out in tmux env, line 1198-1203)
- `RUST_BACKTRACE` - Backtrace on panic (filtered out)

**API Keys** (via config.toml → environment):
- `GEMINI_API_KEY` / `GOOGLE_API_KEY` - Google Gemini
- `ANTHROPIC_API_KEY` / `CLAUDE_API_KEY` - Anthropic Claude
- `OPENAI_API_KEY` - OpenAI GPT
- `QWEN_API_KEY` / `DASHSCOPE_API_KEY` - Qwen/DashScope

**Key Mirroring** (agent_tool.rs:1332-1364):
```rust
// Convenience: map common key names
if let Some(google) = env.get("GOOGLE_API_KEY") {
    env.insert("GEMINI_API_KEY", google);  // Both work
}
if let Some(claude) = env.get("CLAUDE_API_KEY") {
    env.insert("ANTHROPIC_API_KEY", claude);
}
```

**Product Question**:
- **Q24**: Why mirror API keys instead of canonical names?
  - Benefit: Tools using different names "just work"
  - Cost: Confusion about which name is primary
  - Alternative: Document canonical names, require explicit aliases

---

## 5. Critical Path Summary

**Shortest Path to Agent Completion** (quality gate):

```
1. execute_quality_checkpoint() - Spawn decision [5ms]
2. spawn_quality_gate_agents_native() - Create 3 agents [50ms]
   ├→ AGENT_MANAGER.create_agent_from_config_name() - Insert HashMap [5ms each]
   ├→ tokio::spawn(execute_agent) - Launch 3 tasks [5ms each]
   └→ consensus_db.record_agent_spawn() - SQLite INSERT [10ms each]
3. execute_agent() - Main execution [60-120s per agent]
   ├→ execute_model_with_permissions() - Tmux or direct [60-120s]
   │   ├→ ensure_session() - Create tmux session [100ms]
   │   ├→ create_pane() - Split pane [50ms]
   │   ├→ execute_in_pane() - Run command [60-120s]
   │   │   ├→ Write wrapper script [10ms]
   │   │   ├→ Execute bash wrapper [60-120s]
   │   │   │   └→ Model API call [60-120s]
   │   │   └→ Poll for completion [500ms intervals, 2s stability]
   │   └→ Read output file [10ms]
   ├→ extract_json_from_mixed_output() - JSON extraction [10ms]
   ├→ Validation pipeline - 5 checks [10ms]
   └→ update_agent_result() - Store result [5ms]
4. wait_for_quality_gate_agents() - Poll until all done [60-120s]
   └→ record_agent_completion() - SQLite UPDATE [10ms each]
5. store_quality_gate_artifacts_sync() - MCP storage [200ms]
   └→ store_artifact_async() × 3 - Parallel MCP calls [150ms each]
6. fetch_agent_payloads_from_memory() - Collection [50ms]
   └→ extract_and_validate_quality_gate() × 3 [10ms each]
7. process_quality_gate_agent_results() - Classification [50ms]

Total: 60-120s (dominated by model API calls)
```

**Bottlenecks**:
1. **Model API calls** (60-120s) - Unavoidable, external dependency
2. **Tmux overhead** (~200ms per agent) - Can eliminate with direct API calls
3. **MCP storage** (200ms) - Can optimize with batch writes or remove
4. **File stability polling** (2s minimum) - Safety vs latency trade-off

**Product Question**:
- **Q25**: Can we achieve sub-100ms spawn latency? (SPEC-930 NFR-1 target)
  - Current: ~200ms (HashMap insert + tokio spawn + SQLite write + tmux setup)
  - SPEC-930 target: <100ms
  - Blocker: Tmux session/pane creation (~150ms)
  - Solution: Direct async API calls (no tmux)

---

## 6. Architectural Questions to Answer

### 6.1 State Management Questions

**Q26**: Why in-memory HashMap instead of actor supervision?
- Current: AGENT_MANAGER.agents: HashMap<String, Agent>
- SPEC-930: Supervisor actor with message passing
- Trade-off: Simple direct access vs isolated state + restart policies

**Q27**: Can we unify state storage?
- Current: 4 systems (AGENT_MANAGER, SQLite, Filesystem, MCP)
- SPEC-930: Single event log as source of truth
- Migration path: Parallel run old + new systems?

**Q28**: How to handle crash recovery?
- Current: SQLite records spawns, but in-flight agents lost
- SPEC-930: Replay events from log to rebuild state
- Question: Is crash recovery critical for quality gates? (Short-lived, re-runnable)

### 6.2 Execution Questions

**Q29**: Can tmux be safely removed?
- Benefits: Faster spawn, simpler code, no pane management
- Risks: Lose observable execution, debugging harder
- Blocker: OAuth2 device code flows (need user interaction?)
- Analysis needed: Can all provider CLIs work non-interactively?

**Q30**: Should we use actor model for agents?
- SPEC-930: Agent actors with message passing
- Benefit: Isolation, supervision, graceful shutdown
- Cost: Complexity, sync/async bridge with Ratatui TUI
- Question: Does actor model solve current pain points?

**Q31**: How to handle streaming responses?
- Current: Batch output collection (wait for completion)
- SPEC-930: Streaming via tokio channels
- Use case: Real-time progress updates in TUI
- Question: Do quality gates need streaming? (Short, deterministic tasks)

### 6.3 Consensus Questions

**Q32**: Is 2/3 consensus always acceptable?
- Current: Degraded mode if 1 agent fails
- Risk: Wrong majority answer if 2/3 happen to agree on incorrect
- Alternative: Require 3/3 and retry failed agents
- Trade-off: Reliability vs throughput

**Q33**: Should we implement conflict resolution?
- Current: Simple majority voting
- SPEC-930: LangGraph-style shared scratchpad for agent collaboration
- Use case: Agents discuss disagreements before final answer
- Question: Does quality benefit justify complexity?

**Q34**: How to handle extraction failures?
- Current: Store raw output to SQLite for debugging (SPEC-KIT-928)
- Alternative: Auto-retry extraction with different strategy
- Question: Should we re-run agent or fix extraction?

### 6.4 Quality Gate Questions

**Q35**: Are quality gates the right abstraction?
- Current: 3 gates (Clarify, Checklist, Analyze) at 3 checkpoints
- SPEC-930: General agent orchestration (quality gates as use case)
- Question: Should we generalize or keep specialized?

**Q36**: Should quality gates be synchronous or asynchronous?
- Current: Block /speckit.auto pipeline until gates complete
- Alternative: Run gates in background, continue pipeline, interrupt if issues found
- Trade-off: Fail-fast vs progress

**Q37**: How to handle GPT-5 validation spawning?
- Current: Direct spawn in async task (lines 889-996)
- Previous: LLM tool call spawned 18 agents (SPEC-KIT-927 bug)
- Question: Should validation be part of orchestration flow or separate?

### 6.5 Performance Questions

**Q38**: Can we parallelize agent spawning?
- Current: Sequential spawn (for loop)
- Alternative: tokio::spawn all 3 agents simultaneously
- Benefit: 3× faster spawn (~50ms vs ~150ms)
- Risk: Need coordination for SQLite writes

**Q39**: Should we pre-warm tmux sessions?
- Current: Create session per spawn (~100ms)
- Alternative: Keep persistent session, reuse panes
- SPEC-KIT-925: Sessions >5min killed as stale
- Question: Can we reduce session creation overhead?

**Q40**: Can we cache prompts?
- Current: Build prompt every spawn (load spec.md + PRD.md)
- Alternative: Cache prompts per SPEC-ID, invalidate on file change
- Benefit: 10-50ms saved per spawn
- Risk: Stale prompts if files modified

---

## 7. Next Steps (Phase 2)

**Constraint Identification** (see SPEC-931 Phase 2):
1. Map external contracts (user-facing, system-facing)
2. Identify technical constraints (Ratatui sync, SQLite single-writer)
3. Document current bugs and workarounds (SPEC-928 fixes)
4. Define what CANNOT change

**Pattern Validation** (see SPEC-931 Phase 3):
1. Test event sourcing feasibility (can we migrate agent_executions?)
2. Test actor model integration (how to bridge with Ratatui?)
3. Test rate limiting necessity (500 agents/day - do we hit limits?)
4. Test caching-based testing (can we cache OAuth2 responses?)
5. Test TUI dashboard feasibility (where does it fit?)

**Product Design Review** (see SPEC-931 Phase 4):
1. Question consensus_db necessity (right schema? right storage?)
2. Question MCP integration necessity (which operations are essential?)
3. Question evidence repository necessity (file-based vs SQLite?)
4. Question tmux removal impact (can direct API calls replace?)
5. Question quality gate architecture (over-engineered?)

---

## 8. References

**Source Files**:
- `/home/thetu/code/codex-rs/core/src/agent_tool.rs` (1,854 LOC)
- `/home/thetu/code/codex-rs/core/src/tmux.rs` (786 LOC)
- `/home/thetu/code/codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs` (575 LOC)
- `/home/thetu/code/codex-rs/tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs` (319 LOC)
- `/home/thetu/code/codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (1,791 LOC)
- `/home/thetu/code/codex-rs/tui/src/chatwidget/spec_kit/quality_gate_broker.rs` (687 LOC)

**SPEC Documents**:
- SPEC-KIT-928: Orchestration chaos - 10 bugs fixed
- SPEC-KIT-930: Comprehensive agent orchestration refactor (master research)
- SPEC-KIT-931: Architectural deep dive (this analysis)

**Database Location**:
- `~/.code/consensus_artifacts.db` (SQLite)
- Tables: agent_executions, consensus_artifacts, consensus_synthesis

**Configuration**:
- `~/.code/config.toml` - Agent definitions
- `docs/spec-kit/prompts.json` - Quality gate prompts
