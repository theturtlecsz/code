# Phase 1: Data Flow Analysis

**Date**: 2025-11-12
**Status**: In Progress

---

## 1. End-to-End Quality Gate Flow

### 1.1 Success Path (Gemini Agent Example)

```
Time    Component                Action                                      State Mutation
───────────────────────────────────────────────────────────────────────────────────────────────────
00:00   User                     /speckit.auto SPEC-KIT-900                 -

00:01   handler.rs               advance_spec_auto()                        -
                                 ├→ Check for quality checkpoint
                                 └→ determine_quality_checkpoint(Plan)      → BeforeSpecify

00:02   quality_gate_handler     execute_quality_checkpoint(BeforeSpecify)  -
                                 └→ TUI: "Starting Quality Checkpoint: BeforeSpecify"

00:03   orchestrator             spawn_quality_gate_agents_native()         -
        (native)                 ├→ Load docs/spec-kit/prompts.json
                                 ├→ Read spec.md + PRD.md
                                 └→ For gemini, claude, code:

00:04   orchestrator             Process gemini:                            -
        (gemini spawn)           ├→ prompts["quality-gate-clarify"]["gemini"]["prompt"]
                                 ├→ build_quality_gate_prompt()
                                 │  └→ Replace ${SPEC_ID}, inject context
                                 └→ create_agent_from_config_name("gemini_flash", ...)

00:05   AGENT_MANAGER            create_agent_internal()                    [1] agents[uuid] = Agent {
        (HashMap write)          ├→ Generate UUID: 89eda6a8-...                 id: "89eda6a8",
                                 ├→ Create Agent struct                         model: "gemini",
                                 ├→ agents.insert(uuid, agent)                  status: Pending,
                                 └→ tokio::spawn(execute_agent(uuid))           created_at: 02:38:16
                                                                            }
                                                                            [2] handles[uuid] = JoinHandle

00:06   consensus_db             record_agent_spawn()                       [3] SQLite INSERT:
        (SQLite write)           └→ INSERT INTO agent_executions            agent_executions {
                                    (agent_id, phase_type, spawned_at)         agent_id: "89eda6a8",
                                                                                phase_type: "quality_gate",
                                                                                spawned_at: "2025-11-12 02:38:16"
                                                                            }

[Repeat 00:04-00:06 for claude and code agents - total 3 spawns in ~50ms]

00:07   orchestrator             wait_for_quality_gate_agents(agent_ids, 300s)
        (wait loop)              └→ Poll every 500ms

--- PARALLEL: All 3 agents execute simultaneously ---

00:08   execute_agent()          [Background tokio task for gemini]         [4] agents[89eda6a8].status = Running
        (gemini task)            ├→ AGENT_MANAGER.update_agent_status(Running)  agents[89eda6a8].started_at = now()
                                 └→ execute_model_with_permissions()

00:09   tmux.rs                  ensure_session("agents-gemini")            [5] Tmux session created
                                 └→ create_pane(session, "gemini", false)       Pane: %0 (split horizontal)

00:10   tmux.rs                  execute_in_pane()                          -
                                 ├→ Detect large prompt (>1000 chars)
                                 └→ Write wrapper script:
                                    /tmp/tmux-agent-wrapper-{pid}-{pane}.sh

00:11   tmux.rs                  Wrapper script content:                    [6] File created:
        (wrapper gen)            ```bash                                    /tmp/tmux-agent-wrapper-123-0.sh
                                 #!/bin/bash
                                 set -e
                                 export GEMINI_API_KEY='...'
                                 gemini -m gemini-2.5-flash "$(cat <<'EOF'
                                 {3KB prompt with spec.md + PRD.md}
                                 EOF
                                 )" > /tmp/tmux-agent-output-123-0.txt 2>&1
                                 echo '___AGENT_COMPLETE___'
                                 ```

00:12   tmux.rs                  chmod +x wrapper.sh                        -
                                 tmux send-keys "bash wrapper.sh" Enter     [7] Command sent to pane

00:13   Gemini API               HTTP POST to Gemini API                    -
        (external)               ├→ Authentication
                                 ├→ Streaming response
                                 └→ [35 seconds of external API processing]

00:48   Gemini API               Response complete                          [8] File written:
        (response)               └→ Write to /tmp/tmux-agent-output-123-0.txt   5,729 bytes

00:48   Wrapper script           echo '___AGENT_COMPLETE___'                [9] Completion marker in pane

00:48   tmux.rs poll             Loop iteration 70:                         -
        (500ms poll)             ├→ capture-pane: Detect marker
                                 ├→ Check file size: 5,729 bytes
                                 └→ stable_since = Some(now())

00:50   tmux.rs poll             Loop iteration 74:                         -
        (2s stability)           ├→ File still 5,729 bytes
                                 ├→ Stable for 2+ seconds ✓
                                 └→ Read output file

00:50   tmux.rs                  Read /tmp/tmux-agent-output-123-0.txt      [10] Output captured:
                                 ├→ 5,729 bytes                                 5,729 bytes JSON
                                 └→ Delete temp files

00:50   execute_agent()          extract_json_from_mixed_output()           -
                                 ├→ No markdown fence found
                                 ├→ No codex headers
                                 └→ Plain JSON (no extraction)

00:51   execute_agent()          Validation pipeline:                       -
                                 ├→ Corruption: No TUI text ✓
                                 ├→ Headers-only: Has JSON ✓
                                 ├→ Size: 5,729 > 500 ✓
                                 ├→ Schema: No template patterns ✓
                                 └→ JSON parse: Valid ✓

00:51   AGENT_MANAGER            update_agent_result(Ok(output))            [11] agents[89eda6a8] = {
        (success)                ├→ agent.result = Some(output)                 status: Completed,
                                 ├→ agent.status = Completed                    result: Some("..."),
                                 └→ agent.completed_at = now()                  completed_at: 02:38:51
                                                                            }

00:51   orchestrator             wait_for_quality_gate_agents()             [12] SQLite UPDATE:
        (completion)             └→ consensus_db.record_agent_completion()      agent_executions {
                                    UPDATE agent_executions SET                     completed_at: "02:38:51",
                                    completed_at=now(), response_text=?             response_text: "..."
                                    WHERE agent_id=?                            }

[Wait for claude and code agents - parallel execution]

02:30   orchestrator             All 3 agents complete                      -
                                 └→ Return Ok(())

02:30   quality_gate_handler     on_quality_gate_agents_complete()          -
                                 └→ store_quality_gate_artifacts_sync()

02:31   artifact storage         Scan .code/agents/*/result.txt             -
        (sync)                   ├→ Read gemini result: 5,729 bytes
                                 ├→ extract_and_validate_quality_gate()
                                 └→ tokio::spawn(store_artifact_async())    [13] Local-memory MCP:
                                    └→ MCP: local-memory.store_memory()         store_memory(
                                                                                    content: "{...}",
                                                                                    domain: "spec-kit",
                                                                                    importance: 8,
                                                                                    tags: ["quality-gate", ...]
                                                                                )

02:31   artifact storage         block_in_place: Wait for all 3 stores      -
        (wait)                   └→ 3/3 stored successfully (200ms)

02:31   quality_gate_broker      fetch_agent_payloads_from_memory()         -
                                 ├→ AGENT_MANAGER.read().await
                                 ├→ For each agent_id:
                                 │  ├→ agent.result
                                 │  └→ extract_and_validate_quality_gate()
                                 └→ Validate 2/3 consensus

02:32   quality_gate_handler     on_quality_gate_broker_result()            -
                                 └→ process_quality_gate_agent_results()

02:32   issue processing         parse_quality_issue_from_agent() × 3       -
                                 ├→ Merge agent issues
                                 └→ Classify:
                                    ├→ 5 auto-resolvable (high confidence + ACE)
                                    ├→ 2 need validation (medium confidence)
                                    └→ 1 escalate (low confidence)

02:33   auto-resolution          apply_auto_resolution() × 5                [14] Files modified:
                                 └→ Update spec.md, PRD.md                      docs/SPEC-KIT-900/spec.md
                                                                                docs/SPEC-KIT-900/PRD.md

02:34   GPT-5 validation         submit_gpt5_validations()                  [15] Spawn gpt5-medium agent
                                 └→ tokio::spawn(create_agent_from_config_name("gpt5-medium"))

[... GPT-5 validation executes in background ...]
```

**Total Duration**: ~2:30 (150 seconds) for 3-agent quality gate
- API calls: ~120s (parallel, external)
- Orchestration overhead: ~30s (spawn, poll, storage, classification)

**State Mutations Count**: 15 distinct state changes across 4 storage systems

---

### 1.2 Failure Path (Code Agent SPEC-KIT-928 Bug)

```
Time    Component                Action                                      State Mutation
───────────────────────────────────────────────────────────────────────────────────────────────────
00:00   orchestrator             Spawn code agent                           [1] AGENT_MANAGER.agents[5f9b81e0]
                                                                            [2] SQLite agent_executions

00:01   tmux.rs                  Write wrapper script                       [3] /tmp/tmux-agent-wrapper-{pid}.sh
                                 CRITICAL BUG: Double completion marker!
                                 ├→ Internal: echo '___AGENT_COMPLETE___' (line 229 in wrapper)
                                 └→ External: final_command += "echo ___" (line 296 - WRONG!)

00:01   tmux.rs                  tmux send-keys                             -
                                 "bash wrapper.sh; echo '___AGENT_COMPLETE___'"
                                      ^-- BUG: External marker fires immediately!

00:01   Wrapper execution        bash wrapper.sh starts                     -
                                 └→ code exec --model gpt-5 ... starts

00:02   tmux.rs poll             EXTERNAL marker fires!                     [4] Pane contains marker
        (iteration 2)            └→ Completion marker detected (TOO EARLY)      (1 second after start!)

00:02   tmux.rs poll             Check file size: 0 bytes                   -
                                 └→ stable_since = None (file too small)

[Poll loop continues - file is still being written]

00:15   File growth              Output file growing:                       [5] /tmp/output.txt
                                 └→ 1,281 bytes (just prompt echo!)            1,281 bytes (PARTIAL)

00:27   tmux.rs poll             File stable at 1,281 bytes for 2s          [6] PREMATURE READ
        (iteration 54)           ├→ Marker present ✓ (external, wrong)         Read 1,281 bytes
                                 ├→ File stable ✓ (but too early!)
                                 └→ Read output file NOW (WRONG!)

00:27   execute_agent()          extract_json_from_mixed_output()           -
                                 └→ Returns 1,281 bytes
                                    (contains prompt schema, not response!)

00:27   Validation               Check for schema template:                 -
                                 ├→ Contains "\"id\": string," ✓
                                 └→ VALIDATION FAILED: Schema template detected

00:27   AGENT_MANAGER            update_agent_result(Err("Schema template"))[7] agents[5f9b81e0] = {
                                                                                status: Failed,
                                                                                error: "Schema template",
                                                                                result: Some("...") (raw 1,281 bytes)
                                                                            }

00:77   Code exec                ACTUAL completion (inside wrapper)         [8] /tmp/output.txt
        (50s later!)             └→ 23KB JSON with 15 real issues              23,456 bytes (FULL)
                                    But nobody is listening anymore!

RESULT: Code agent shows Failed status with 1,281 byte partial output (prompt schema)
        Real 23KB output never collected, lost in /tmp
```

**Root Cause Analysis**:
1. **Dual completion markers**: External (tmux command) + Internal (wrapper script)
2. **External fires too early**: Immediately after wrapper starts, not after it finishes
3. **File stability not defensive enough**: 1,281 bytes met >1KB threshold, so stable period started
4. **Poll loop has no safeguard**: Trusted marker + stability, didn't verify content quality

**SPEC-KIT-928 Fix** (tmux.rs:293-297):
```rust
let has_wrapper = !temp_files.is_empty();
if !has_wrapper {
    final_command.push_str("; echo '___AGENT_COMPLETE___'");  // Only for direct commands
}
// Wrapper scripts already have marker at line 229 - DON'T ADD EXTERNAL
```

**Post-Fix Behavior**:
```
00:01 - Wrapper starts (no external marker)
00:77 - Code exec finishes, wrapper adds internal marker
00:79 - Poll detects marker, file=23,456 bytes, stable for 2s
00:81 - Read complete output ✓
```

**Critical Insight**: Marker placement determines when polling stops. Must be AFTER output write completes.

---

### 1.3 Claude Async Hang Path (SPEC-KIT-929)

```
Time    Component                Action                                      State Mutation
───────────────────────────────────────────────────────────────────────────────────────────────────
00:00   orchestrator             Spawn claude agent (quality_gate)          [1] AGENT_MANAGER + SQLite
00:01   execute_agent()          tokio task starts                          [2] status = Running
00:02   tmux.rs                  execute_in_pane() starts                   -
00:10   Tmux pane                Command finishes, output written           [3] Pane shows "zsh" (back to shell)
                                                                            [4] /tmp/output.txt complete
00:12   tmux.rs poll             Marker detected + file stable              [5] Read output (17KB)
00:12   tmux.rs                  execute_in_pane() returns Ok(output)       -

--- BUG: execute_agent() never progresses past this point ---

??:??   execute_agent()          Stuck somewhere between:                   NEVER REACHED:
        (hung task)              ├→ execute_model_with_permissions() returned  - extract_json_from_mixed_output()
                                 └→ update_agent_result() never called         - Validation
                                                                               - AGENT_MANAGER update
                                                                               - SQLite update

∞       AGENT_MANAGER            agents[uuid].status = Running (forever)    [6] Stale state:
        (stale state)            agents[uuid].result = None                     status: Running
                                 agents[uuid].completed_at = None               completed_at: NULL

∞       SQLite                   agent_executions WHERE agent_id=uuid       [7] Stale state:
        (stale state)            completed_at = NULL                            completed_at: NULL
                                 response_text = NULL                           response_text: NULL

∞       wait_for_quality_gate    Poll loop continues checking status        -
        (infinite wait)          └→ Agent never transitions to Completed
                                    → Timeout after 300s (5 minutes)
```

**Observations**:
1. **Tmux completes successfully**: Pane shows shell prompt, output file exists
2. **execute_agent() task hangs**: Never reaches update_agent_result()
3. **Only affects quality gates**: Claude works in regular stages (107s, 17KB response)
4. **Only affects Claude**: Gemini and Code work fine in quality gates

**Hypotheses** (from SPEC-928):
- **A. Async context issue**: Quality gate uses tmux_enabled=true, regular stages use false?
  - Evidence: Quality gates explicitly enable tmux (orchestrator.rs:125)
  - Counterpoint: Gemini/Code work fine with tmux in quality gates
- **B. Claude CLI specific**: Some Claude CLI behavior different in tmux context?
  - Evidence: Only Claude hangs, not Gemini/Code
  - Test: Run Claude CLI in tmux manually (works fine)
- **C. Lock deadlock**: execute_agent() waiting on AGENT_MANAGER lock?
  - Code inspection: Lock dropped before execute_model_with_permissions() (agent_tool.rs:697)
  - Unlikely: Would affect all agents, not just Claude

**Status**: Tracked in SPEC-929, deferred (P2). Workaround: Use Gemini + Code for 2/2 consensus.

---

## 2. Database Write Patterns

### 2.1 Dual-Write Problem (ACID Violation)

**Pattern**: Update in-memory HashMap AND SQLite in sequence without transaction

**Example** (agent spawn):
```rust
// Step 1: Update HashMap (agent_tool.rs:283)
self.agents.insert(agent_id.clone(), agent.clone());  // [WRITE 1]

// Step 2: Spawn tokio task (agent_tool.rs:289-294)
let handle = tokio::spawn(async move { ... });
self.handles.insert(agent_id.clone(), handle);  // [WRITE 2]

// Step 3: Update SQLite (orchestrator.rs:133-149)
db.record_agent_spawn(agent_id, spec_id, stage, phase_type, agent_name, run_id)?;  // [WRITE 3]
```

**Failure Scenarios**:

**Scenario A: Crash between HashMap and SQLite**
```
✓ agents.insert() succeeds
✓ handles.insert() succeeds
✗ CRASH before record_agent_spawn()

Result: In-memory agent exists, tokio task running, but no SQLite record
Impact: Agent completes, no routing info (phase_type unknown)
Recovery: None - agent completion handler can't route without SQLite record
```

**Scenario B: SQLite write fails**
```
✓ agents.insert() succeeds
✓ handles.insert() succeeds
✗ record_agent_spawn() fails (disk full, permission denied)

Result: In-memory agent exists, task running, but SQLite INSERT failed
Impact: Agent completes normally, routing works (in-memory), but no persistence
Recovery: Restart loses spawn info (can't resume)
```

**Scenario C: Concurrent spawns race**
```
Thread 1: agents.insert(agent_1)  [WRITE A1]
Thread 2: agents.insert(agent_2)  [WRITE A2]
Thread 1: db.record_agent_spawn(agent_1)  [WRITE B1]
Thread 2: db.record_agent_spawn(agent_2)  [WRITE B2]

If crash between B1 and B2:
- agent_1: Both writes ✓
- agent_2: HashMap ✓, SQLite ✗

Result: Inconsistent state across storage systems
```

**SPEC-930 Event Sourcing Solution**:
```rust
// Single atomic transaction
event_store.transaction(|tx| {
    // 1. Append event (immutable)
    tx.append_event(AgentEvent::AgentQueued { agent_id, ... })?;

    // 2. Update projection (derived state)
    tx.update_projection(agent_id, AgentState::Queued)?;

    // 3. Commit (ACID)
    tx.commit()?;

    // Event log is source of truth
    // Projection is cache (can rebuild from events)
})?;
```

**Benefits**:
- **Atomic**: Single transaction for all writes
- **Recoverable**: Rebuild projections from events on crash
- **Auditable**: Complete history preserved

**Costs**:
- **Complexity**: Replay engine, snapshot management
- **Performance**: Replay overhead on startup

---

### 2.2 Current Write Operations Inventory

**Agent Lifecycle Writes**:
```
Spawn:
├→ [W1] AGENT_MANAGER.agents.insert()        (in-memory HashMap)
├→ [W2] AGENT_MANAGER.handles.insert()       (in-memory HashMap)
└→ [W3] consensus_db.record_agent_spawn()    (SQLite INSERT)

Status Update (Running):
└→ [W4] AGENT_MANAGER.agents[id].status = Running  (in-memory)

Completion:
├→ [W5] AGENT_MANAGER.agents[id].status = Completed  (in-memory)
├→ [W6] AGENT_MANAGER.agents[id].result = Some(...)  (in-memory)
└→ [W7] consensus_db.record_agent_completion()       (SQLite UPDATE)

Failure:
├→ [W8] AGENT_MANAGER.agents[id].status = Failed     (in-memory)
├→ [W9] AGENT_MANAGER.agents[id].error = Some(...)   (in-memory)
└→ [W10] consensus_db.record_extraction_failure()    (SQLite UPDATE)
```

**Total Write Operations**: 10 separate writes for single agent lifecycle
- 7 in-memory (AGENT_MANAGER HashMap)
- 3 SQLite (consensus_db)
- 0 transactions coordinating them

**Failure Windows**: 3 critical windows where crash causes inconsistency
1. Between W1-W2 and W3 (spawn)
2. Between W5-W6 and W7 (completion)
3. Between W8-W9 and W10 (failure)

**Product Question**:
- **Q41**: Do we accept eventual consistency?
  - Current: In-memory updates happen first, SQLite eventually
  - Benefit: Fast in-memory reads for UI
  - Risk: Crash loses recent updates (not persisted)
  - Alternative: SQLite as source of truth, in-memory as cache

---

## 3. Read Operations Inventory

### 3.1 Hot Path Reads (during execution)

**Poll Loop** (quality_gate_orchestrator.rs:236-314):
```rust
loop {
    for agent_id in agent_ids {
        let agent = AGENT_MANAGER.read().await.get_agent(agent_id);  // [R1] HashMap read
        match agent.status {
            Completed | Failed => { /* record */ }
            _ => { /* still running */ }
        }
    }
    sleep(500ms);
}
```

**Frequency**: 2 reads/second × 3 agents × 120 seconds = ~720 reads per quality gate
**Contention**: RwLock allows concurrent reads (no blocking)

### 3.2 Collection Path Reads

**From Memory** (quality_gate_broker.rs:229-348):
```rust
let manager = AGENT_MANAGER.read().await;  // [R2] Acquire read lock
for agent_id in agent_ids {
    if let Some(agent) = manager.get_agent(agent_id) {  // [R3] HashMap get
        if let Some(result_text) = &agent.result {  // [R4] Field read
            // Extract and validate
        }
    }
}
```

**From Filesystem** (quality_gate_broker.rs:429-514):
```rust
for entry in read_dir(".code/agents")? {  // [R5] Directory scan
    let content = fs::read_to_string(result_path)?;  // [R6] File read (5-20KB)
    // Extract and validate
}
```

**From SQLite** (consensus_db.rs:369-383):
```rust
let (phase_type, stage) = conn.query_row(  // [R7] SQLite SELECT
    "SELECT phase_type, stage FROM agent_executions WHERE agent_id = ?",
    params![agent_id],
    |row| Ok((row.get(0)?, row.get(1)?))
)?;
```

**Read Frequency per Quality Gate**:
- AGENT_MANAGER HashMap: ~720 reads (poll loop)
- SQLite: ~6 reads (3 spawns + 3 completions for routing)
- Filesystem: ~1 scan per broker attempt (max 3 attempts)

**Product Question**:
- **Q42**: Is filesystem scan acceptable latency?
  - Current: Scan .code/agents/ (up to 100 entries) per broker attempt
  - Latency: ~50-200ms depending on directory size
  - Alternative: Index by agent_id in SQLite, direct lookup
  - SPEC-930: Eliminate filesystem, use memory or event log

---

## 4. Concurrency Patterns

### 4.1 Lock Acquisition Order

```rust
// Pattern 1: Short read locks (preferred)
{
    let manager = AGENT_MANAGER.read().await;  // Acquire read
    let agent = manager.get_agent(id);         // Fast HashMap lookup
}  // Release immediately

// Pattern 2: Write locks (mutation)
{
    let mut manager = AGENT_MANAGER.write().await;  // Acquire exclusive write
    manager.update_agent_status(id, Running);       // Mutate
}  // Release

// Pattern 3: Long-held write lock (ANTI-PATTERN)
let mut manager = AGENT_MANAGER.write().await;  // Acquire write
// ... do work ...
manager.create_agent_from_config_name(...).await;  // Holds lock during spawn
// ... more work ...
drop(manager);  // Finally release

// This blocks ALL readers and writers for entire spawn duration!
```

**Lock Contention Points**:
1. **Agent spawn**: Write lock held during config lookup + struct creation (agent_tool.rs:117-158)
2. **Status update**: Write lock for single HashMap update (agent_tool.rs:379-394)
3. **Poll loop**: Read lock acquired 2×/second × 3 agents = 6×/second

**Optimization Opportunities**:
- **Batch status updates**: Update all 3 agents in single write lock
- **Lock-free reads**: Use Arc<Agent> instead of HashMap cloning
- **Actor model**: No shared state, only message passing (SPEC-930)

---

### 4.2 Race Conditions

**Race 1: Completion handler vs poll loop**
```rust
// Thread 1: execute_agent() completes
manager.update_agent_result(id, Ok(output));  // Write status = Completed

// Thread 2: wait_for_quality_gate_agents() polling
let agent = manager.get_agent(id);  // Read status
if agent.status == Completed { /* found */ }
```

**Safe**: RwLock ensures either poll sees old status or new status, never torn read

**Race 2: Duplicate spawn check**
```rust
// Thread 1: Spawn gemini agent
manager.create_agent_from_config_name("gemini", ...);
// [Time window here]
let concurrent = manager.check_concurrent_agents();  // Check for duplicates

// Thread 2: Spawn another gemini (race)
manager.create_agent_from_config_name("gemini", ...);
```

**Unsafe**: Two spawns can both pass duplicate check if interleaved
**Mitigation**: SPEC-KIT-928 logs duplicates but doesn't prevent (lines 78-96, 179-187)

**Race 3: Filesystem vs AGENT_MANAGER**
```rust
// Thread 1: Agent completes, writes to AGENT_MANAGER
manager.update_agent_result(id, Ok(output));

// Thread 2: Broker scans filesystem
for entry in read_dir(".code/agents")? {
    // Might find file before AGENT_MANAGER updated (or vice versa)
}
```

**Current behavior**: Both paths eventually consistent
**Risk**: Broker might miss agent if timing is unlucky

**SPEC-930 Solution**: Single source of truth (event log) eliminates races

---

## 5. Error Propagation Paths

### 5.1 Spawn Errors

```
create_agent_from_config_name()
    ├→ Config not found → Err("Agent config 'X' not found")
    ├→ Agent disabled → Err("Agent 'X' is disabled")
    └→ Spawn succeeds → Ok(agent_id)

orchestrator.rs:117-128:
    match manager.create_agent_from_config_name() {
        Ok(agent_id) => spawn_infos.push(AgentSpawnInfo { ... }),
        Err(e) => return Err(format!("Failed to spawn {}: {}", config_name, e))
    }

quality_gate_handler.rs:1143-1203:
    match spawn_quality_gate_agents_native() {
        Ok(spawn_infos) => { /* wait for agents */ },
        Err(e) => { warn!("Failed to spawn: {}", e) }
    }
```

**Error Handling**: Propagates up to handler, logs warning, but pipeline CONTINUES
**Product Question**: Should spawn failure halt pipeline or degrade gracefully?

---

### 5.2 Execution Errors

```
execute_model_with_permissions()
    ├→ Command not found → Err("Required agent 'X' not installed")
    ├→ Tmux execution failed → Err("Tmux execution failed: {e}")
    ├→ Timeout → Err("Timeout waiting for agent completion after 600s")
    ├→ Command exit code != 0 → Err("Command failed: {stderr}")
    └→ Success → Ok(output)

execute_agent():
    match execute_model_with_permissions() {
        Ok(output) => {
            // Validation pipeline (5 checks)
            if validation_fails {
                Err("Validation failed: {reason}")
            } else {
                Ok(output)
            }
        }
        Err(e) => Err(e)  // Propagate execution error
    }

    match validated_result {
        Ok(output) => manager.update_agent_result(id, Ok(output)),
        Err(error) => {
            // SPEC-KIT-928: Store raw output even on validation failure
            if let Some(raw) = raw_output_for_storage {
                let output_with_error = format!("VALIDATION_FAILED: {}\n\n--- RAW OUTPUT ---\n{}", error, raw);
                manager.update_agent_result(id, Err(output_with_error));
            } else {
                manager.update_agent_result(id, Err(error));
            }
        }
    }
```

**Error Storage**:
- **agent.error**: Error message (why failed)
- **agent.result**: Raw output if available (SPEC-KIT-928 fix)
- **agent.status**: Failed

**Product Question**:
- **Q43**: Should we retry failed agents automatically?
  - Current: No retry, just mark as Failed
  - SPEC-930: Retry on transient errors (timeout, rate limit)
  - Decision: Which errors are retryable? (Timeout: yes, Invalid JSON: no)

---

### 5.3 Validation Errors (5-Layer Cascade)

```
Validation Layer 1: Corruption Detection
    ├→ Check for TUI text patterns
    ├→ Check for conversation fragments
    └→ Err("Output polluted with TUI conversation text")

Validation Layer 2: Headers-Only Detection
    ├→ Check for Codex headers without JSON
    └→ Err("Returned initialization headers without JSON output")

Validation Layer 3: Minimum Size
    ├→ Check if output.len() < 500 bytes
    └→ Err("Agent output too small (X bytes, minimum 500)")

Validation Layer 4: Schema Template Detection
    ├→ Check for unquoted types (": string", ": number")
    ├→ Check for diff proposal templates
    └→ Err("Returned JSON schema template instead of actual data")

Validation Layer 5: JSON Parsing
    ├→ serde_json::from_str::<Value>(output)?
    └→ Err("Agent output is not valid JSON: {parse_error}")

All pass → Ok(output)
```

**SPEC-KIT-928 Improvements**:
1. Layer 1 expanded: More corruption patterns (agent_tool.rs:874-887)
2. Layer 4 added: Schema template detection (agent_tool.rs:914-925)
3. Both layers NEW in SPEC-928 (found code agent returning prompt schema)

**Success Rate** (empirical):
- Before SPEC-928: Code agent 0% (0/15 runs)
- After SPEC-928: Code agent 100% (3/3 runs)

**Product Question**:
- **Q44**: Should validation be pluggable per agent type?
  - Current: Same 5-layer validation for all agents
  - Observation: Only code agent needed schema template detection
  - Alternative: Per-agent validation rules in config.toml

---

## 6. MCP Integration Points

### 6.1 Consensus Artifact Storage

**Operation**: `local-memory.store_memory` (quality_gate_handler.rs:1772-1780)

**Call Pattern**:
```rust
let args = json!({
    "content": json_content,      // Agent output JSON (5-20KB)
    "domain": "spec-kit",
    "importance": 8,
    "tags": [
        "quality-gate",
        "spec:{spec_id}",
        "checkpoint:{checkpoint}",
        "stage:{stage}",
        "agent:{agent_name}"
    ]
});

manager.call_tool("local-memory", "store_memory", Some(args), timeout_10s)?;
```

**Frequency**: 3 calls per quality checkpoint (gemini, claude, code)
**Purpose**: Persistent storage for consensus artifacts, searchable by tags
**Alternative**: Store in SQLite consensus_artifacts table (already exists!)

**Product Question**:
- **Q45**: Why duplicate storage (SQLite + MCP)?
  - SQLite consensus_artifacts: Has content_json column (line 67)
  - MCP local-memory: Same JSON stored with tags
  - Usage: Broker searches MCP, but could query SQLite
  - Overhead: 200ms MCP calls + memory database writes
  - SPEC-930: Eliminate MCP for workflow artifacts (use SQLite only)

---

### 6.2 Broker Collection (Legacy Path)

**Operation**: `local-memory.search` (quality_gate_broker.rs:594-613)

**Call Pattern**:
```rust
let args = json!({
    "query": format!("{} gpt5-validation", spec_id),
    "limit": 10,
    "tags": [
        "quality-gate",
        "spec:{spec_id}",
        "checkpoint:{checkpoint}",
        "stage:gpt5-validation"
    ],
    "search_type": "hybrid"
});

manager.call_tool("local-memory", "search", Some(args), timeout_10s)?;
```

**Frequency**: 3 retries max (100ms, 200ms, 400ms delays)
**Purpose**: Collect GPT-5 validation results
**Note**: Only used for validation, NOT for agent artifacts (uses memory path now)

**Product Question**:
- **Q46**: Why search MCP instead of read from AGENT_MANAGER?
  - Current: GPT-5 agent stores to MCP, broker searches MCP
  - Alternative: GPT-5 agent completes → AGENT_MANAGER.result → broker reads directly
  - Benefit: Eliminate MCP search, faster, simpler
  - Legacy: Originally all artifacts were in MCP, now transitioning to memory

---

## 7. Performance Profile

### 7.1 Latency Breakdown (Quality Gate)

**Operation**:               **Latency**:  **Count**: **Total**:
```
Spawn preparation            50ms          1          50ms
├─ Load prompts.json         10ms
├─ Read spec.md + PRD.md     20ms
└─ Build prompts × 3         20ms

Agent spawns (sequential)    50ms          3          150ms
├─ create_agent_internal     15ms each
├─ tokio::spawn              5ms each
└─ SQLite INSERT             10ms each

Tmux setup per agent         150ms         3          450ms
├─ ensure_session            100ms
├─ create_pane               50ms
└─ write wrapper script      10ms

Model API calls (parallel)   60-120s       3          60-120s
└─ External API latency      (varies)

Tmux polling overhead        2s            3          6s
└─ File stability wait       2s each       (after API completes)

Result collection            250ms         1          250ms
├─ Scan filesystem           50ms
├─ Extract JSON × 3          30ms
├─ Store to MCP × 3          150ms (parallel)
└─ Fetch from memory         20ms

Issue classification         50ms          1          50ms
└─ Parse + merge + classify

Total orchestration:         ~7s (7% of total time)
Total API latency:           60-120s (93% of total time)
Total elapsed:               67-127s
```

**Bottleneck**: Model API calls (external, unavoidable)
**Optimization target**: Reduce 7s orchestration to <1s (SPEC-930 goal)

---

### 7.2 Optimization Opportunities

**Opportunity 1: Eliminate tmux** (SPEC-930)
- Current: 450ms tmux setup + 6s stability wait = 6.45s
- Alternative: Direct async API calls with tokio
- Savings: ~6.5s per quality gate (97% of orchestration overhead)

**Opportunity 2: Parallel spawns**
- Current: Sequential spawn loop (~150ms)
- Alternative: tokio::join! all 3 spawns
- Savings: ~100ms (spawn 3× faster)

**Opportunity 3: Eliminate MCP storage**
- Current: 150ms for 3 parallel MCP stores
- Alternative: Use SQLite consensus_artifacts only
- Savings: ~150ms per quality gate

**Opportunity 4: Pre-warm tmux sessions**
- Current: Create session per spawn (100ms)
- Alternative: Keep persistent session, reuse
- Savings: ~100ms per quality gate
- Risk: SPEC-KIT-925 already tried this, sessions go stale

**Total potential savings**: 6.5s + 0.1s + 0.15s + 0.1s = **6.85s** (98% reduction in orchestration)
**New target**: ~150ms orchestration + 60-120s API = 60-120s total (vs current 67-127s)

---

## 8. State Machine Validation

### 8.1 Current State Transitions

**Valid Transitions** (agent_tool.rs:23-31):
```
AgentStatus enum:
- Pending     (created, not started)
- Running     (executing)
- Completed   (success)
- Failed      (error)
- Cancelled   (user abort)

State Graph:
Pending ──────┐
              ↓
         → Running ──┬→ Completed
         ↑          │
         │          ↓
    Cancelled ←── Failed
```

**Enforced Invariants** (agent_tool.rs:379-394):
```rust
// Can only set started_at when transitioning to Running
if agent.status == Running && agent.started_at.is_none() {
    agent.started_at = Some(Utc::now());
}

// completed_at set on terminal states
if matches!(agent.status, Completed | Failed | Cancelled) {
    agent.completed_at = Some(Utc::now());
}
```

**Missing Invariants**:
- ✗ No check preventing Completed → Running (invalid transition)
- ✗ No check preventing multiple Running → Completed (idempotency not enforced)
- ✗ No validation that result.is_some() when status == Completed

**SPEC-930 Improvement**:
```rust
impl AgentState {
    fn can_transition_to(&self, target: &AgentState) -> bool {
        match (self, target) {
            (Pending, Running | Cancelled) => true,
            (Running, Completed | Failed | Cancelled) => true,
            (Failed, Retrying) => true,  // SPEC-930: Retry support
            (Retrying, Queued) => true,
            _ => false  // All other transitions invalid
        }
    }
}
```

---

### 8.2 State Consistency Checks

**Question: Can agents have status=Completed but result=None?**

```sql
-- Query SQLite for inconsistent states
SELECT agent_id, agent_name, spawned_at, completed_at, response_text
FROM agent_executions
WHERE completed_at IS NOT NULL AND response_text IS NULL;
```

**Expected**: Should be impossible (update_agent_result sets both)
**Reality**: SPEC-928 fixed this - before fix, validation failures discarded output

**Product Question**:
- **Q47**: Should we add database constraints?
  ```sql
  -- Enforce: completed_at set → response_text OR extraction_error set
  CREATE CONSTRAINT check_completion_has_output
  CHECK (
      completed_at IS NULL
      OR response_text IS NOT NULL
      OR extraction_error IS NOT NULL
  );
  ```
  - Benefit: Database enforces invariants
  - Cost: Failed migrations if existing data violates

---

## 9. Critical Path Analysis

### 9.1 What Must Work for Quality Gates to Succeed?

**Critical Dependencies** (ranked by impact):

1. **AGENT_MANAGER write lock availability** [CRITICAL]
   - If locked: Agent spawn blocks indefinitely
   - Current: No timeout, no backpressure
   - Failure mode: Deadlock if lock holder panics

2. **SQLite writability** [CRITICAL]
   - If full: record_agent_spawn() fails, routing broken
   - If locked: Write blocks until timeout
   - Failure mode: Disk full, permission denied

3. **Tmux availability** [HIGH]
   - If missing: Falls back to direct execution
   - Fallback works: Tests run without tmux
   - Graceful degradation ✓

4. **Model API reachability** [HIGH]
   - If down: Agent fails with network error
   - No fallback: Pipeline halts on API failure
   - Failure mode: Network outage, rate limit

5. **Filesystem writability** [MEDIUM]
   - If /tmp full: Output file write fails
   - If .code/ missing: Result files can't be written
   - Failure mode: Disk full, permission denied

6. **MCP local-memory availability** [LOW]
   - If down: Artifact storage fails
   - Workaround: Broker can read from AGENT_MANAGER memory
   - Non-blocking: Quality gates still work

**Failure Recovery**:
- ✓ Tmux failure: Graceful fallback to direct execution
- ✓ MCP failure: Broker reads from memory instead
- ✗ SQLite failure: No fallback, pipeline halts
- ✗ AGENT_MANAGER lock: No timeout, potential deadlock
- ✗ API failure: No retry, no fallback provider

**SPEC-930 Improvements**:
- **Event store resilience**: Multiple storage backends (SQLite + disk + memory)
- **Rate limiting**: Queue requests to avoid hitting API limits
- **Circuit breaker**: Fallback to different provider on repeated failures
- **Actor supervision**: Automatic restart on crash

---

## 10. Data Flow Findings

### 10.1 Architecture Assessment

**Strengths**:
1. ✅ **Separated concerns**: Execution (agent_tool), observation (tmux), storage (consensus_db)
2. ✅ **Observable**: Tmux panes allow real-time monitoring
3. ✅ **Robust validation**: 5-layer cascade catches most issues (SPEC-928)
4. ✅ **Graceful degradation**: 2/3 consensus, tmux fallback

**Weaknesses**:
1. ❌ **No ACID compliance**: Dual-write without transactions (HashMap + SQLite)
2. ❌ **No retry logic**: Transient failures fail permanently
3. ❌ **No rate limiting**: Can hit API limits at scale
4. ❌ **Brittle timing**: File stability, polling intervals, marker detection
5. ❌ **Storage redundancy**: 4 systems (HashMap, SQLite, Filesystem, MCP) for same data

---

### 10.2 SPEC-930 Pattern Fit Analysis

**Event Sourcing** [GOOD FIT]:
- Current problem: Dual-write inconsistency
- SPEC-930 solution: Event log as single source of truth
- Migration path: Add event_log table, write events first, maintain projections
- Complexity: Medium (replay engine, snapshots)

**Actor Model** [QUESTIONABLE FIT]:
- Current problem: No crash recovery, no supervision
- SPEC-930 solution: Supervisor + agent actors
- Concern: Ratatui TUI is synchronous, actors are async (impedance mismatch)
- Complexity: High (actor lifecycle, message passing, TUI bridge)

**Rate Limiting** [LOW PRIORITY]:
- Current problem: No rate limiting (but also no evidence of hitting limits)
- Scale: Quality gates spawn 3 agents, ~10 quality gates/day = 30 agents/day
- OpenAI limit: 30,000 TPM (tokens per minute)
- Analysis: Nowhere near limits at current scale
- Decision: Defer until proven necessary

**Queue-Based Execution** [LOW PRIORITY]:
- Current: Spawn immediately, no queue
- Use case: When many agents spawn simultaneously
- Current load: 3 agents at a time (quality gate)
- Decision: Overkill for current scale

---

## 11. Key Findings

### Finding 1: Dual-Write is Root Architectural Weakness
**Evidence**: 10 separate write operations with no transaction coordination
**Impact**: State corruption risk on crash, inconsistency between HashMap and SQLite
**SPEC-930 Solution**: Event sourcing with single source of truth
**Recommendation**: HIGH PRIORITY to fix

### Finding 2: Tmux is 93% of Orchestration Overhead
**Evidence**: 6.5s tmux (setup + stability) vs 7s total orchestration
**Impact**: Slower spawn (<100ms target impossible with tmux)
**SPEC-930 Solution**: Direct async API calls
**Recommendation**: MEDIUM PRIORITY (works, but slow)

### Finding 3: 4 Storage Systems Create Confusion
**Evidence**: AGENT_MANAGER (volatile), SQLite (persistent), Filesystem (temp), MCP (searchable)
**Impact**: Unclear where to read from, redundant writes, synchronization complexity
**SPEC-930 Solution**: Event log + projections (2 systems instead of 4)
**Recommendation**: HIGH PRIORITY to simplify

### Finding 4: No Retry Logic
**Evidence**: Transient errors (timeout, rate limit) fail permanently
**Impact**: Low reliability, requires manual re-run
**SPEC-930 Solution**: Exponential backoff retry with error categorization
**Recommendation**: MEDIUM PRIORITY (mitigates transient failures)

### Finding 5: Actor Model May Not Fit
**Evidence**: Ratatui TUI is synchronous, requires sync/async bridge
**Concern**: SPEC-930 actor pattern assumes fully async architecture
**Analysis needed**: Can we make TUI fully async? (Ratatui async-template exists)
**Recommendation**: Phase 3 analysis required before commitment

---

## 12. Next Phase Inputs

**For Phase 2 (Constraint Identification)**:
- External contracts: /speckit.* commands, consensus_artifacts.db schema
- Technical constraints: Ratatui sync rendering, SQLite single-writer
- Bug inventory: 10 SPEC-928 bugs that must not regress

**For Phase 3 (Pattern Validation)**:
- Event sourcing migration path: Add event_log, parallel run, cutover
- Actor model TUI integration: Async Ratatui feasibility
- Rate limiting necessity: Current scale (30 agents/day) vs limits (30,000 TPM)

**For Phase 4 (Product Design Review)**:
- Consensus DB: Keep (simplify schema), Redesign (event log), or Remove (memory-only)
- MCP integration: Remove for workflow artifacts, keep for curated knowledge
- Tmux: Remove if direct API calls proven viable
