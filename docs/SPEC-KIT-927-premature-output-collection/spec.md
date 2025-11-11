# SPEC-KIT-927: Premature Output Collection Bug (Zombie Agents)

**Status**: Critical Bug - Active Investigation
**Priority**: P0 - Blocks reliable multi-agent execution
**Created**: 2025-11-11
**Discovered**: SPEC-KIT-900 /speckit.auto run (zombie process 4175310)

---

## Problem Statement

Agents are marked as "completed" and their output is collected **before they finish executing**, resulting in:
1. **Partial output collected**: Only initialization headers, no actual response
2. **Invalid consensus stored**: JSON schema templates instead of real data
3. **Zombie processes**: Agent processes continue running after "completion"
4. **Pipeline corruption**: Later stages use empty/invalid data from previous stages

**Impact**: Multi-agent consensus produces garbage data while appearing to succeed.

---

## Evidence

### Timeline (SPEC-KIT-900 Implement Stage, 2025-11-11)

```
15:42:12 - gpt_codex agent spawns (agent_id: 6d5b3180...)
15:42:17 - Agent writes initialization header to output file (1161 bytes)
15:42:18 - System marks agent COMPLETED and stores output ❌ BUG OCCURS HERE
15:42:XX - Agent continues running, starts actual processing
15:48:XX - Agent still running as zombie process (PID 4175310)
```

### Database Evidence

**agent_executions table**:
```sql
agent_id: 6d5b3180-da80-4d61-b464-df0709a5dcc2
agent_name: gpt_codex
spawned_at: 2025-11-11 15:42:12
completed_at: 2025-11-11 15:42:18  ← 6 seconds! Way too fast
response_text: 1161 bytes           ← Only header, no real output
```

**Response text content** (truncated):
```
[2025-11-11T15:42:17] OpenAI Codex v0.0.0 (research preview)
--------
workdir: /home/thetu/.code/working/code/branches/code-code-template--code-templates-implement-templ-20251111-154212
model: gpt-5-codex
...
[2025-11-11T15:42:17] User instructions:
Template: ~/.code/templates/implement-template.md
Task: Generate code to implement...
Emit code diff proposals as JSON:
{
  "stage": "spec-implement",
  "prompt_version": "20251002-implement-a",
  ...
}
[END OF FILE - NO ACTUAL JSON OUTPUT]
```

**consensus_artifacts table**:
```sql
spec_id: SPEC-KIT-900
stage: spec-implement
agent_name: gpt_codex
created_at: 2025-11-11 15:43:05
content_json: 391 bytes           ← Just JSON schema template!
```

**Content stored**:
```json
{
  "stage": "spec-implement",
  "prompt_version": "20251002-implement-a",
  "agent": "gpt_codex",
  "model": "gpt-5-codex",
  "model_release": "2025-09-29",
  "reasoning_mode": "auto",
  "diff_proposals": [ { "path": string, "change": string (diff or summary), ... } ],
  "test_commands": [ string ],
  "tool_calls": [ string ],
  "risks": [ string ]
}
```

This is the **literal JSON schema** from the prompt, not actual output!

### Process Evidence

```bash
# Zombie process still running 6+ minutes after "completion"
$ ps aux | grep 4175310
thetu 4175310  0.5  0.1 1311012 70248 pts/12  Sl+  15:42   0:02 code exec \
  --sandbox workspace-write --skip-git-repo-check --model gpt-5-codex \
  -c model_reasoning_effort="high" Template: ~/.code/templates/implement-template.md ...
```

### Historical Pattern

**Previous runs with zombies** (from same SPEC-KIT-900):
```
22:49 run: gpt_pro zombie (completed_at NULL)
00:46 run: gpt_codex zombie (completed_at NULL)
01:36 run: gpt_pro zombie (completed_at NULL)
02:02 run: gpt_codex zombie (completed_at NULL)
```

**Current run** (15:40+):
- Database shows all 4 agents completed successfully
- But gpt_codex output is invalid (just header)
- Actual process was still running as zombie

---

## Root Cause Analysis

### Primary Cause: Premature File Reading in Tmux Mode

**Code Path**: `codex-rs/core/src/agent_tool.rs:771-892` (SPEC-KIT-923 tmux execution)

```rust
// Execute in tmux pane with 10 minute timeout
let timeout_secs = 600;
match crate::tmux::execute_in_pane(
    &session_name,
    &pane_id,
    &program,
    &args,
    &env,
    working_dir.as_deref(),
    timeout_secs,
)
.await
{
    Ok(output) => {
        return Ok(output); // ← This returns prematurely!
    }
```

**What should happen**:
1. Agent spawns in tmux pane
2. Agent writes initialization header
3. Agent processes prompt (this takes MINUTES for LLMs)
4. Agent writes JSON response
5. System detects completion marker: `___AGENT_COMPLETE___`
6. System reads output file
7. Returns complete output

**What actually happens**:
1. Agent spawns in tmux pane ✓
2. Agent writes initialization header ✓
3. System reads output file IMMEDIATELY ❌ (after ~6 seconds)
4. Finds only header + JSON schema
5. Returns incomplete output ❌
6. Agent continues processing (becomes zombie) ❌

### Contributing Factor: SPEC-925 Stale Session Bug

**Context**: This run started BEFORE the SPEC-925 fix was compiled/deployed

The stale tmux session bug (fixed in commit d34f68a6c) was causing:
- 10-hour-old sessions being reused
- Pane capture corruption
- Completion marker never detected
- Timeout or premature return

This explains why `execute_in_pane()` returned early - it couldn't detect completion properly due to stale session state.

### Bug Trigger Conditions

**Required**:
1. SPEC_KIT_OBSERVABLE_AGENTS=1 (tmux mode enabled)
2. Stale tmux session (>5 minutes old) OR corrupted pane state
3. Agent that takes >10 seconds to produce output

**Result**:
- execute_in_pane() fails to detect completion marker
- Falls back to timeout or premature return
- Partial output captured
- Agent process orphaned (becomes zombie)

---

## Impact Assessment

### Data Corruption

**Implement Stage** (current evidence):
- gpt_codex: Invalid output (schema only, 391 bytes)
- gemini: Valid output (4,853 bytes) ✓
- claude: Valid output (7,625 bytes) ✓
- gpt_pro: Valid output (363 bytes) ✓

**Result**: 3/4 consensus, synthesis proceeded with degraded data

### Cascade Effects

**Downstream stages** using corrupted implement data:
1. **Validate stage**: Cannot validate code that doesn't exist
2. **Audit stage**: Audit findings based on incomplete implementation
3. **Unlock stage**: Ship decision based on invalid data

**Severity**: Pipeline produces garbage but appears successful ✓✓✓✓✓

### Zombie Process Accumulation

**If not killed manually**:
- Zombies consume resources (memory, CPU)
- Multiple pipeline runs = multiple zombies
- System instability over time

---

## Fix Strategy

### Phase 1: Immediate Mitigation (DONE ✓)

**Completed**:
1. ✓ Killed zombie process 4175310
2. ✓ Identified root cause (premature output collection)
3. ✓ Documented evidence chain

### Phase 2: Core Fix (TODO)

**Required Changes**:

**A. Fix execute_in_pane() completion detection** (core/src/tmux.rs)

**Current logic** (broken):
```rust
loop {
    let capture = Command::new("tmux")
        .args(["capture-pane", "-t", pane_id, "-p", "-S", "-"])
        .output().await?;

    let pane_content = String::from_utf8_lossy(&capture.stdout);
    if pane_content.contains("___AGENT_COMPLETE___") {
        // Read output file and return
        return Ok(output);
    }

    // But what if we NEVER see the marker due to stale session?
    // Eventually timeout and return partial output!
}
```

**Fixed logic** (needed):
```rust
loop {
    // Check output file existence AND size growth
    let file_meta = tokio::fs::metadata(&output_file).await?;
    let current_size = file_meta.len();

    // File must exist AND be stable (not growing) for 2+ seconds
    if current_size > 1000 && current_size == last_size {
        stable_duration += poll_interval;

        if stable_duration > Duration::from_secs(2) {
            // Check for completion marker
            if pane_content.contains("___AGENT_COMPLETE___") {
                return Ok(read_output_file().await?);
            }
        }
    } else {
        stable_duration = Duration::ZERO;
        last_size = current_size;
    }

    // Timeout logic...
}
```

**B. Validate output before marking complete** (core/src/agent_tool.rs)

```rust
// After execute_model_with_permissions returns
let result = execute_model_with_permissions(...).await;

match result {
    Ok(output) => {
        // VALIDATE output is not just a schema/header
        if output.len() < 500 || output.contains("{ \"path\": string") {
            // This is just a schema template, not real output!
            return Err("Agent returned invalid output (schema only)");
        }

        // Try parsing as JSON
        if let Err(e) = serde_json::from_str::<serde_json::Value>(&output) {
            return Err(format!("Agent output is not valid JSON: {}", e));
        }

        // Output is valid
        manager.update_agent_result(&agent_id, Ok(output)).await;
    }
    Err(e) => {
        manager.update_agent_result(&agent_id, Err(e)).await;
    }
}
```

**C. Process cleanup on timeout** (core/src/agent_tool.rs)

```rust
// If agent times out or fails, kill the process
if let Err(e) = result {
    // Find and kill the agent process
    if use_tmux {
        // Send Ctrl+C to tmux pane
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &pane_id, "C-c"])
            .status()
            .await;
    }
    // ... rest of cleanup
}
```

### Phase 3: Monitoring & Detection (TODO)

**Add instrumentation**:

1. **Output size validation**:
   ```rust
   if output.len() < 1000 && duration < Duration::from_secs(30) {
       tracing::warn!("⚠️ Suspiciously fast agent completion: {} bytes in {}s",
                      output.len(), duration.as_secs());
   }
   ```

2. **Zombie detection**:
   ```rust
   // Before spawning new agent, check for zombies from previous run
   let zombie_count = check_zombie_processes(&session_name).await;
   if zombie_count > 0 {
       tracing::error!("❌ Found {} zombie agents, cleaning up", zombie_count);
       kill_zombie_agents(&session_name).await;
   }
   ```

3. **Output validation logging**:
   ```rust
   tracing::info!("Agent output validation: {} bytes, valid JSON: {}, has schema markers: {}",
                  output.len(), is_valid_json, contains_schema_template);
   ```

### Phase 4: Testing (TODO)

**Test scenarios**:

1. **Fast agent** (< 10s): Should complete normally
2. **Slow agent** (> 5min): Should not timeout prematurely
3. **Stale session**: Should be detected and killed
4. **Invalid output**: Should retry or fail explicitly
5. **Zombie cleanup**: Should kill orphaned processes

---

## Success Criteria

### Must Have

1. ✓ Agents never marked complete with partial output
2. ✓ Output validation catches schema-only responses
3. ✓ Zombie processes cleaned up automatically
4. ✓ Stale sessions killed (SPEC-925 fix deployed)

### Should Have

1. ✓ Output file size monitoring
2. ✓ Suspicious completion time warnings
3. ✓ Zombie detection before new runs

### Nice to Have

1. ○ Automatic retry on invalid output
2. ○ Output content heuristics (JSON complexity score)
3. ○ Historical zombie rate tracking

---

## Testing Plan

### Unit Tests

**Location**: `codex-rs/core/tests/agent_lifecycle_tests.rs`

**Scenarios**:
```rust
#[tokio::test]
async fn test_premature_completion_detection() {
    // Simulate agent that writes header quickly but output slowly
    // Verify system waits for full output
}

#[tokio::test]
async fn test_output_validation_rejects_schema() {
    // Verify schema-only output is rejected
}

#[tokio::test]
async fn test_zombie_cleanup() {
    // Verify orphaned processes are killed
}
```

### Integration Tests

**Location**: `codex-rs/tui/tests/agent_output_validation_tests.rs`

**Scenarios**:
1. Run full /speckit.plan with observable agents
2. Verify all agents produce valid output (>1KB, valid JSON)
3. Verify no zombie processes remain after completion
4. Verify stale sessions are recreated

### Manual Validation

```bash
# Clean state
rm -f /tmp/tmux-agent-output-*.txt
tmux kill-session -t agents-* 2>/dev/null

# Rebuild with fixes
cd codex-rs && cargo build --bin code --profile dev-fast

# Test run
export SPEC_KIT_OBSERVABLE_AGENTS=1
export RUST_LOG=codex_core::agent_tool=debug,codex_core::tmux=trace

./target/dev-fast/code
# Run: /speckit.plan SPEC-KIT-900

# Verify
sqlite3 ~/.code/consensus_artifacts.db \
  "SELECT agent_name, length(content_json),
          content_json NOT LIKE '%{ \"path\": string%' as is_valid
   FROM consensus_artifacts
   WHERE spec_id='SPEC-KIT-900' AND stage='spec-plan'
   ORDER BY created_at DESC LIMIT 3"

# All agents should show is_valid=1

# Check for zombies
ps aux | grep "code exec" | grep -v grep
# Should return nothing
```

---

## Related Work

**SPEC-KIT-925**: Agent Status Sync Failure (stale tmux sessions)
- Fixed session freshness checking (>5min = kill and recreate)
- Contributed to this bug (stale sessions caused completion detection failures)
- Fix deployed: commit d34f68a6c

**SPEC-KIT-923**: Observable Agent Execution (tmux integration)
- Introduced tmux-based execution for agent observability
- Created opportunity for this bug (file polling vs process completion)
- Needs hardening for production use

**SPEC-KIT-926**: TUI Progress Visibility
- Would help detect this bug earlier (show "Agent X completed suspiciously fast")
- Would expose zombie processes in status display
- Complementary monitoring solution

---

## Priority Justification

**P0 Critical** because:
1. **Silent data corruption**: Pipeline succeeds with garbage data
2. **Affects all stages**: Implement bug cascades to validate/audit/unlock
3. **Reproducible**: Happens consistently with observable agents enabled
4. **Resource leak**: Zombie processes accumulate over time
5. **Blocks production use**: Cannot trust multi-agent consensus results

**Must fix before**:
- SPEC-KIT-900 validation completion
- Any production deployment
- Observable agents become default mode

---

## Timeline

**Phase 1** (Immediate): ✓ Done (investigation complete)
**Phase 2** (Core fix): 4-6 hours implementation
**Phase 3** (Monitoring): 2 hours instrumentation
**Phase 4** (Testing): 3-4 hours validation

**Total**: ~10-12 hours to full resolution

---

## Open Questions

1. **Q**: Should we disable observable agents (SPEC_KIT_OBSERVABLE_AGENTS) until this is fixed?
   **A**: TBD - Current run completing, assess if bug reproduced

2. **Q**: Can we detect corrupted output automatically and retry?
   **A**: Yes - check output size, JSON validity, schema markers

3. **Q**: Should timeout be longer than 10 minutes for slow models?
   **A**: Yes - Gemini/GPT-5 can take 15-20 minutes on complex prompts

4. **Q**: How do we handle agents that legitimately produce small outputs?
   **A**: Combine size check + JSON validation + schema detection

---

## References

**User Report**:
> "i'm seeing it's in the unlock stage, yet I see a process that is using the implement template? 4175310 what is going on here"

**Key Insight**: Process zombie + stage mismatch = premature completion bug

**Debugging Command**:
```bash
ps aux | grep [PID]  # Check process details
sqlite3 ~/.code/consensus_artifacts.db "..."  # Check DB records
```

---

**End of SPEC-KIT-927**
