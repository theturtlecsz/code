# SPEC-KIT-927 Fix Session Prompt

**Copy this prompt to start a new session focused on fixing the premature output collection bug.**

---

## Objective

Fix SPEC-KIT-927: Premature Output Collection Bug in observable agent execution that causes:
- Agents marked "completed" before they finish executing
- Partial output collected (headers only, no actual JSON responses)
- Invalid consensus data stored (JSON schema templates instead of real data)
- Zombie processes left running
- Silent data corruption throughout the pipeline

**Priority**: P0 Critical - Blocks reliable multi-agent execution and production use.

---

## Context

**Full specification**: `docs/SPEC-KIT-927-premature-output-collection/spec.md` (read this first!)

**Bug discovered**: 2025-11-11 during SPEC-KIT-900 /speckit.auto run
**Evidence**: gpt_codex agent marked complete after 6 seconds, stored 391-byte schema template instead of real output, actual process ran for 6+ minutes as zombie (PID 4175310)

**Root cause**: `execute_in_pane()` in `codex-rs/core/src/tmux.rs` returns prematurely when:
1. Stale tmux session prevents completion marker detection (SPEC-925 contributes)
2. File polling reads output file before agent finishes writing
3. System collects initialization header + JSON schema prompt as "output"
4. Agent process orphaned, continues running as zombie

**Database evidence**:
```sql
-- Agent execution record shows suspiciously fast completion
agent_id: 6d5b3180-da80-4d61-b464-df0709a5dcc2
spawned_at: 2025-11-11 15:42:12
completed_at: 2025-11-11 15:42:18  -- Only 6 seconds!
response_text: 1161 bytes          -- Just header, no JSON

-- Consensus artifact stored literal schema from prompt
content_json: {
  "stage": "spec-implement",
  "diff_proposals": [ { "path": string, "change": string, ... } ],
  ...
}
-- This is the SCHEMA, not actual data!
```

---

## Implementation Tasks

### Phase 2: Core Fix (4-6 hours)

**Task 1: Fix completion detection** (`codex-rs/core/src/tmux.rs:324-400`)

Current broken logic:
```rust
loop {
    let capture = Command::new("tmux")
        .args(["capture-pane", "-t", pane_id, "-p", "-S", "-"])
        .output().await?;

    let pane_content = String::from_utf8_lossy(&capture.stdout);
    if pane_content.contains("___AGENT_COMPLETE___") {
        return Ok(read_output_file().await?);
    }
    // If marker never seen, eventually timeout and return partial output!
}
```

Required improvements:
1. **Monitor output file size stability**: File must stop growing for 2+ seconds before reading
2. **Validate file size threshold**: Must be >1000 bytes before considering complete
3. **Dual detection**: Require BOTH completion marker AND stable file size
4. **Better timeout handling**: Don't return partial data on timeout, return error

**Task 2: Add output validation** (`codex-rs/core/src/agent_tool.rs:598-601`)

After `execute_model_with_permissions()` returns, validate output:
```rust
let result = execute_model_with_permissions(...).await;

match result {
    Ok(output) => {
        // Validation 1: Size check
        if output.len() < 500 {
            return Err("Agent output too small (< 500 bytes)");
        }

        // Validation 2: Schema detection
        if output.contains("{ \"path\": string") ||
           output.contains("\"diff_proposals\": [ {") {
            return Err("Agent returned JSON schema instead of data");
        }

        // Validation 3: JSON parsing
        match serde_json::from_str::<serde_json::Value>(&output) {
            Ok(_) => {
                // Valid JSON, proceed
                manager.update_agent_result(&agent_id, Ok(output)).await;
            }
            Err(e) => {
                return Err(format!("Invalid JSON output: {}", e));
            }
        }
    }
    Err(e) => {
        // Kill zombie process before reporting error
        cleanup_agent_process(&agent_id, &pane_id).await;
        manager.update_agent_result(&agent_id, Err(e)).await;
    }
}
```

**Task 3: Implement process cleanup** (`codex-rs/core/src/agent_tool.rs` or `tmux.rs`)

Add function to kill orphaned agent processes:
```rust
async fn cleanup_agent_process(agent_id: &str, pane_id: &str) {
    // Send Ctrl+C to tmux pane
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, "C-c"])
        .status()
        .await;

    // Give process 2 seconds to exit gracefully
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Force kill if still running (find by agent_id in process args)
    // Note: May need to track PIDs in AGENT_MANAGER
}
```

**Task 4: Increase timeout for slow models** (`codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:571`)

Current timeout is 1200s (20 minutes) which is reasonable, but ensure it's enforced properly:
```rust
let (agent_id, agent_output) = spawn_and_wait_for_agent(
    agent_name,
    config_name,
    prompt,
    agent_configs,
    &batch_id,
    spec_id,
    stage,
    run_id.as_deref(),
    1200, // 20min - keep this, but ensure timeout actually errors
)
.await?;
```

### Phase 3: Monitoring & Detection (2 hours)

**Task 5: Add suspicious completion warnings** (`codex-rs/core/src/agent_tool.rs:598-601`)

```rust
let duration = start_time.elapsed();
if output.len() < 1000 && duration < Duration::from_secs(30) {
    tracing::warn!(
        "⚠️ SUSPICIOUS: {} completed in {}s with only {} bytes - possible premature collection",
        agent_name,
        duration.as_secs(),
        output.len()
    );
}
```

**Task 6: Add zombie detection before spawning** (`codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`)

Before spawning new agents, check for zombies from previous runs:
```rust
// Check for zombie processes before starting new agents
async fn check_and_cleanup_zombies(session_name: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(["list-panes", "-t", session_name, "-F", "#{pane_id}"])
        .output()
        .await?;

    if output.status.success() {
        let pane_ids = String::from_utf8_lossy(&output.stdout);
        for pane_id in pane_ids.lines() {
            // Check if pane has running process
            // Kill if zombie detected
        }
    }
    Ok(())
}
```

**Task 7: Add detailed output validation logging**

```rust
tracing::info!(
    "Agent output validation: agent={}, size={} bytes, valid_json={}, has_schema_markers={}, duration={}s",
    agent_name,
    output.len(),
    is_valid_json,
    contains_schema_template,
    duration.as_secs()
);
```

### Phase 4: Testing (3-4 hours)

**Task 8: Create unit tests** (`codex-rs/core/tests/agent_lifecycle_tests.rs`)

```rust
#[tokio::test]
async fn test_premature_completion_detection() {
    // Simulate agent that writes header quickly but output slowly
    // Verify system waits for stable file size
    // Verify completion marker is required
}

#[tokio::test]
async fn test_output_validation_rejects_schema() {
    // Test with literal schema output
    // Verify rejection with appropriate error
}

#[tokio::test]
async fn test_output_validation_rejects_small() {
    // Test with <500 byte output
    // Verify rejection
}

#[tokio::test]
async fn test_zombie_cleanup() {
    // Verify orphaned processes are killed
    // Verify pane cleanup
}
```

**Task 9: Create integration tests** (`codex-rs/tui/tests/agent_output_validation_tests.rs`)

```rust
#[tokio::test]
async fn test_full_pipeline_with_observable_agents() {
    // Run /speckit.plan SPEC-KIT-900 with SPEC_KIT_OBSERVABLE_AGENTS=1
    // Verify all agents produce valid output (>1KB, valid JSON)
    // Verify no schema templates in consensus
    // Verify no zombie processes after completion
}
```

**Task 10: Manual validation**

```bash
# Clean state
rm -f /tmp/tmux-agent-output-*.txt
tmux kill-session -t agents-gemini agents-claude agents-code 2>/dev/null
sqlite3 ~/.code/consensus_artifacts.db "DELETE FROM consensus_artifacts WHERE spec_id='SPEC-KIT-927-TEST'"

# Rebuild with fixes
cd codex-rs && cargo build --bin code --profile dev-fast

# Test run with enhanced logging
export SPEC_KIT_OBSERVABLE_AGENTS=1
export RUST_LOG=codex_core::agent_tool=debug,codex_core::tmux=trace
./target/dev-fast/code

# In TUI: /speckit.plan SPEC-KIT-927-TEST (create simple test SPEC)

# Verify results
sqlite3 ~/.code/consensus_artifacts.db "
SELECT
    agent_name,
    length(content_json) as size,
    content_json NOT LIKE '%{ \"path\": string%' as is_valid,
    content_json LIKE '%{%}%' as has_braces
FROM consensus_artifacts
WHERE spec_id='SPEC-KIT-927-TEST' AND stage='spec-plan'
ORDER BY created_at DESC"

# All agents should show: size > 1000, is_valid=1, has_braces=1

# Check for zombies
ps aux | grep "code exec" | grep -v grep
# Should return nothing

# Check agent execution times
sqlite3 ~/.code/consensus_artifacts.db "
SELECT
    agent_name,
    spawned_at,
    completed_at,
    CAST((julianday(completed_at) - julianday(spawned_at)) * 86400 AS INTEGER) as duration_secs
FROM agent_executions
WHERE spec_id='SPEC-KIT-927-TEST' AND stage='spec-plan'
ORDER BY spawned_at"

# Durations should be reasonable (gemini: 60-180s, others: 30-120s)
# Nothing should be < 10 seconds
```

---

## Success Criteria

### Must Have (Blocking)

- [ ] Agents never marked complete with partial output (header only)
- [ ] Output validation catches and rejects schema-only responses
- [ ] Invalid output returns error, not false success
- [ ] Zombie processes cleaned up automatically on timeout/error
- [ ] File size stability check (2+ seconds stable before reading)
- [ ] Completion marker detection working with fresh sessions (SPEC-925 fix active)

### Should Have (Important)

- [ ] Suspicious completion warnings in logs (<30s and <1KB)
- [ ] Output validation logging shows size, JSON validity, schema detection
- [ ] Zombie detection runs before each agent spawn
- [ ] Unit tests cover all validation scenarios
- [ ] Integration test validates full pipeline

### Nice to Have (Enhancement)

- [ ] Automatic retry on invalid output (1-2 retries)
- [ ] Output content heuristics (JSON complexity score, field count)
- [ ] Historical zombie rate tracking in telemetry

---

## Dependencies & Prerequisites

**Required before starting**:
1. ✅ SPEC-925 fix deployed (stale session detection, commit d34f68a6c)
2. ✅ SPEC-KIT-927 investigation complete (root cause identified)
3. ⚠️ Current SPEC-KIT-900 run should complete (provides more test data)

**Parallel work (can proceed)**:
- SPEC-KIT-926 (TUI visibility) - complementary monitoring
- Other bug fixes not touching agent execution paths

**Conflicts (coordinate if active)**:
- Changes to `core/src/agent_tool.rs` execute_agent()
- Changes to `core/src/tmux.rs` execute_in_pane()
- Changes to agent_orchestrator.rs spawn functions

---

## Files to Modify

**Primary**:
- `codex-rs/core/src/tmux.rs` (execute_in_pane completion detection)
- `codex-rs/core/src/agent_tool.rs` (output validation, cleanup)
- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` (zombie detection)

**Tests**:
- `codex-rs/core/tests/agent_lifecycle_tests.rs` (new file)
- `codex-rs/tui/tests/agent_output_validation_tests.rs` (new file)

**Documentation**:
- `docs/SPEC-KIT-927-premature-output-collection/IMPLEMENTATION.md` (track progress)

---

## Testing Strategy

**Incremental validation**:
1. After each task, run unit tests: `cd codex-rs && cargo test --lib agent_lifecycle`
2. After Phase 2 complete, run integration test: `cargo test --test agent_output_validation`
3. After Phase 3 complete, run manual validation (see Task 10)
4. Before declaring complete, run full /speckit.auto on SPEC-KIT-927-TEST

**Validation commands**:
```bash
# Build and test cycle
cd codex-rs
cargo build --bin code --profile dev-fast
cargo test --lib agent_lifecycle
cargo test --test agent_output_validation

# Manual smoke test
./target/dev-fast/code
# Run: /speckit.new Test SPEC-KIT-927 fix with observable agents
# Run: /speckit.plan SPEC-KIT-927-TEST

# Check results (must be valid)
sqlite3 ~/.code/consensus_artifacts.db \
  "SELECT agent_name, length(content_json),
          content_json NOT LIKE '%string%' as clean
   FROM consensus_artifacts
   WHERE spec_id='SPEC-KIT-927-TEST'
   ORDER BY created_at DESC LIMIT 5"
```

---

## Risk Mitigation

**Risk 1**: Fix breaks normal (non-tmux) agent execution
**Mitigation**:
- Test both `SPEC_KIT_OBSERVABLE_AGENTS=1` and `=0` modes
- Keep non-tmux code path unchanged initially
- Add feature flag if needed: `SPEC_KIT_STRICT_OUTPUT_VALIDATION`

**Risk 2**: Increased timeout causes slower pipelines
**Mitigation**:
- File size stability is 2 seconds, not minutes
- Only affects agents that complete quickly (rare)
- Monitor average completion times before/after

**Risk 3**: False positives on output validation
**Mitigation**:
- Test with real agent outputs from database
- Allow legitimate small outputs if valid JSON
- Log validation details for debugging

**Risk 4**: Zombie cleanup kills legitimate processes
**Mitigation**:
- Only kill processes in confirmed-zombie panes
- Send graceful Ctrl+C first, SIGKILL only after timeout
- Track PIDs explicitly in AGENT_MANAGER

---

## Related Work

**SPEC-KIT-925**: Stale Session Detection
- Already fixed and committed (d34f68a6c)
- Reduces but doesn't eliminate this bug
- Fresh sessions (<5min) should work correctly

**SPEC-KIT-923**: Observable Agent Execution
- Introduced tmux-based execution
- Created opportunity for this bug
- Needs hardening (this SPEC)

**SPEC-KIT-926**: TUI Progress Visibility
- Would help detect this bug earlier
- Shows suspicious completion times
- Complementary monitoring solution

---

## Questions to Answer During Implementation

1. **What's the right file size stability duration?** Currently proposing 2 seconds - test with real agents
2. **Should we retry on invalid output?** Or fail fast and let user retry?
3. **How do we track PIDs for cleanup?** Add to AGENT_MANAGER or query tmux?
4. **What's the minimum valid output size?** Currently proposing 500 bytes - validate against real outputs
5. **Should validation be configurable?** Feature flag vs always-on?

---

## Debugging Tips

**If fix doesn't work**:
1. Check RUST_LOG includes `codex_core::agent_tool=debug,codex_core::tmux=trace`
2. Look for "SUSPICIOUS" warnings in logs
3. Check tmux sessions: `tmux ls` and `tmux list-panes -a`
4. Inspect output files: `ls -lh /tmp/tmux-agent-output-*.txt`
5. Check database for size patterns: `SELECT agent_name, length(content_json) FROM consensus_artifacts WHERE spec_id='...' ORDER BY created_at DESC`

**Common issues**:
- Stale sessions: Run `tmux kill-server` to reset all sessions
- Permission issues: Check `/tmp` write permissions
- JSON parsing: Use `jq` to validate agent outputs manually
- Process cleanup: Use `pkill -f "code exec"` to kill all agent processes

---

## Commit Strategy

**Incremental commits**:
1. `fix(spec-927): add file size stability check to tmux completion detection`
2. `fix(spec-927): add output validation (size, schema detection, JSON parsing)`
3. `fix(spec-927): implement agent process cleanup on timeout/error`
4. `feat(spec-927): add suspicious completion warnings and zombie detection`
5. `test(spec-927): add unit tests for output validation and zombie cleanup`
6. `test(spec-927): add integration tests for full pipeline validation`
7. `docs(spec-927): document fix implementation and validation results`

**Final PR description**:
```
Fixes SPEC-KIT-927: Premature output collection causing data corruption

Problem: Agents marked "completed" before finishing execution, storing
partial output (headers + schema) instead of real JSON responses.

Solution:
- File size stability check (must be stable 2+ seconds)
- Output validation (size, schema detection, JSON parsing)
- Zombie process cleanup (Ctrl+C on timeout/error)
- Monitoring (suspicious completion warnings, zombie detection)
- Comprehensive test coverage

Impact: Prevents silent data corruption in multi-agent consensus

Tests: 10 unit tests, 2 integration tests, manual validation passed
```

---

## Estimated Timeline

**Phase 2** (Core fix): 4-6 hours
**Phase 3** (Monitoring): 2 hours
**Phase 4** (Testing): 3-4 hours
**Documentation**: 1 hour

**Total**: 10-13 hours from start to completion

**Breakpoints** (can pause/resume):
- After Task 4: Core fix complete, basic validation
- After Task 7: Monitoring complete, enhanced visibility
- After Task 10: All testing complete, ready for review

---

## Next Steps After Completion

1. **Merge to main** after PR review and approval
2. **Run full SPEC-KIT-900 validation** with fix deployed
3. **Monitor** 3-5 additional pipeline runs for zombie issues
4. **Update SPEC.md** with SPEC-KIT-927 completion status
5. **Consider enabling observable agents by default** if stable
6. **Close** related issues and update dependent SPECs

---

**Good luck! This is a critical fix for production reliability.**

---

## Quick Start Commands

```bash
# Start working
cd /home/thetu/code

# Read the full spec
cat docs/SPEC-KIT-927-premature-output-collection/spec.md

# Check current state
git status
sqlite3 ~/.code/consensus_artifacts.db "SELECT COUNT(*) FROM agent_executions WHERE completed_at IS NULL"

# Start implementation
cd codex-rs
cargo build --bin code --profile dev-fast
# ... begin Task 1 ...
```
