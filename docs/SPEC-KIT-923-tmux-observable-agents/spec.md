# SPEC-KIT-923: Observable Agent Execution via Tmux

**Status**: Draft
**Created**: 2025-11-10
**Priority**: P0 (Critical - enables debugging hung agents)
**Owner**: Code
**Dependencies**: SPEC-KIT-920 (tmux automation foundation)

---

## Problem Statement

**Black Box Problem**: Current agent execution provides zero visibility into what agents are doing when they hang or timeout.

**Observed Issues** (SPEC-900 validation):
- Claude agent timeout after 1200s (20 minutes) during Plan stage
- No way to see what the agent is doing (stuck? rate-limited? processing?)
- No streaming output to understand progress
- Can't attach to watch agent work in real-time
- Debugging requires post-mortem log analysis only

**Impact**:
- ❌ Cannot diagnose hung agents in real-time
- ❌ No progress visibility during long-running consensus
- ❌ Difficult to identify prompt issues causing hangs
- ❌ Timeout is only signal - no context about failure
- ❌ Forces blind retries without understanding root cause

**Example** (SPEC-900, 2025-11-10):
```
Spawned: claude agent for Plan stage (16:16:41)
Waited: 20 minutes (1200s timeout threshold)
Result: Timeout - no completion, no output, no diagnostic info
Action: Blind retry with zero additional context
```

---

## Success Criteria

### Primary Goals
1. **Real-time visibility**: `tmux attach` to watch agents execute in real-time
2. **Per-agent panes**: Each agent runs in isolated tmux pane
3. **Full output capture**: Complete stdout/stderr logged to evidence
4. **Progress monitoring**: See token streaming, reasoning steps, API calls
5. **Debugging support**: Attach to hung agent, see exact failure point

### Acceptance Criteria
- [ ] Agent spawning modified to use tmux panes
- [ ] Each agent runs in: `tmux-agent-<spec-id>-<stage>-<agent-name>`
- [ ] Output captured to: `evidence/agents/<spec-id>/<stage>/<agent>-output.txt`
- [ ] User can attach: `tmux attach -t tmux-agent-SPEC-KIT-900-plan-claude`
- [ ] Pane layout: split-window for parallel execution visibility
- [ ] Completion detection: Monitor pane for exit status
- [ ] Timeout handling: Kill hung panes after AR-1 threshold
- [ ] Evidence capture: Full agent logs preserved
- [ ] Backward compatible: Works with existing consensus system

### Non-Goals
- Interactive agent control (read-only observation)
- Agent output modification
- Multi-session support (one SPEC at a time for now)

---

## Technical Design

### Architecture Overview

**Current** (Black Box):
```
TUI → Agent Manager → Background Process (invisible)
                         ↓
                    Agent executes (no visibility)
                         ↓
                    Timeout or Complete (result only)
```

**New** (Observable via Tmux):
```
TUI → Agent Manager → Tmux Session Creation
                         ↓
                    Tmux Pane per Agent (visible, attachable)
                         ↓
                    Monitor pane for completion
                         ↓
                    Capture output → Evidence
                         ↓
                    Return result to consensus
```

### Tmux Session Structure

**Session naming**: `agents-<spec-id>-<stage>-<run-id>`

**Example** (Plan stage with 3 agents):
```bash
tmux new-session -d -s agents-SPEC-KIT-900-plan-a1b2c3

# Pane layout (3 agents, side-by-side):
┌─────────────┬─────────────┬─────────────┐
│   Gemini    │   Claude    │   GPT-Pro   │
│             │             │             │
│ Agent:      │ Agent:      │ Agent:      │
│ gemini      │ claude      │ gpt_pro     │
│             │             │             │
│ Status:     │ Status:     │ Status:     │
│ Running...  │ Timeout!    │ Waiting...  │
│             │             │             │
│ Output...   │ [stuck]     │ Output...   │
└─────────────┴─────────────┴─────────────┘

# Attach to watch: tmux attach -t agents-SPEC-KIT-900-plan-a1b2c3
# Detach without killing: Ctrl+b d
```

### Implementation Details

#### 1. Tmux Agent Spawner Module

**File**: `tui/src/chatwidget/spec_kit/tmux_agent_spawner.rs` (new)

```rust
//! Observable agent execution via tmux panes
//!
//! SPEC-KIT-923: Spawn agents in tmux panes for real-time visibility and debugging.
//!
//! Each agent runs in an isolated tmux pane where:
//! - Output is visible in real-time
//! - User can attach to watch progress
//! - Full logs captured to evidence
//! - Hung agents can be diagnosed
//!
//! Session structure:
//! - One tmux session per stage: agents-<spec-id>-<stage>-<run-id>
//! - One pane per agent: split horizontally/vertically
//! - Completion detection: monitor pane exit status
//! - Output capture: tee to evidence/agents/<spec-id>/<stage>/<agent>.log

use super::error::{Result, SpecKitError};
use crate::spec_prompts::SpecStage;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Configuration for tmux-based agent execution
pub struct TmuxAgentConfig {
    pub session_name: String,
    pub spec_id: String,
    pub stage: SpecStage,
    pub run_id: String,
    pub evidence_dir: PathBuf,
    pub timeout: Duration,
}

/// Spawn agents in tmux panes for observable execution
pub fn spawn_agents_in_tmux(
    config: &TmuxAgentConfig,
    agents: &[(String, String)], // (agent_name, prompt)
    cwd: &Path,
) -> Result<Vec<String>> {
    tracing::info!(
        "Spawning {} agents in tmux session: {}",
        agents.len(),
        config.session_name
    );

    // 1. Create tmux session
    create_tmux_session(&config.session_name, cwd)?;

    // 2. Create pane for each agent
    let mut pane_ids = Vec::new();
    for (idx, (agent_name, prompt)) in agents.iter().enumerate() {
        let pane_id = if idx == 0 {
            // First agent uses the initial pane (pane 0)
            format!("{}:0.0", config.session_name)
        } else {
            // Subsequent agents: split window
            split_tmux_window(&config.session_name, idx)?
        };

        // 3. Start agent execution in pane
        start_agent_in_pane(
            &pane_id,
            agent_name,
            prompt,
            &config.evidence_dir,
            &config.spec_id,
            config.stage,
        )?;

        pane_ids.push(pane_id);
    }

    tracing::info!("✅ All {} agents spawned in tmux panes", agents.len());
    tracing::info!("   Attach with: tmux attach -t {}", config.session_name);

    Ok(pane_ids)
}

/// Create tmux session for agent execution
fn create_tmux_session(session_name: &str, cwd: &Path) -> Result<()> {
    let output = Command::new("tmux")
        .args(&[
            "new-session",
            "-d",           // Detached
            "-s", session_name,
            "-c", cwd.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Failed to create tmux session: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SpecKitError::from_string(format!(
            "Tmux session creation failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Split tmux window to create new pane
fn split_tmux_window(session_name: &str, agent_index: usize) -> Result<String> {
    // Split horizontally for readability (panes stacked)
    let output = Command::new("tmux")
        .args(&[
            "split-window",
            "-t", session_name,
            "-h",  // Horizontal split
            "-P",  // Print pane ID
            "-F", "#{pane_id}",
        ])
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Failed to split window: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SpecKitError::from_string(format!("Tmux split failed: {}", stderr)));
    }

    let pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(pane_id)
}

/// Start agent execution in tmux pane
fn start_agent_in_pane(
    pane_id: &str,
    agent_name: &str,
    prompt: &str,
    evidence_dir: &Path,
    spec_id: &str,
    stage: SpecStage,
) -> Result<()> {
    // Create evidence directory for agent logs
    let agent_log_dir = evidence_dir.join(spec_id).join(stage.command_name());
    std::fs::create_dir_all(&agent_log_dir)
        .map_err(|e| SpecKitError::from_string(format!("Failed to create agent log dir: {}", e)))?;

    let log_file = agent_log_dir.join(format!("{}-output.log", agent_name));

    // Build agent command
    // TODO: This needs to match how agents are currently spawned
    // For now, placeholder showing the pattern
    let agent_cmd = format!(
        "echo 'Agent: {}' && \
         echo 'Stage: {}' && \
         echo 'Spec: {}' && \
         echo '---' && \
         code /agents --agent {} --prompt \"$PROMPT\" 2>&1 | tee {}",
        agent_name,
        stage.display_name(),
        spec_id,
        agent_name,
        log_file.display()
    );

    // Set pane title
    Command::new("tmux")
        .args(&[
            "select-pane",
            "-t", pane_id,
            "-T", &format!("{} ({})", agent_name, spec_id),
        ])
        .output()
        .ok(); // Non-fatal if title setting fails

    // Send prompt as environment variable (avoid shell escaping issues)
    Command::new("tmux")
        .args(&[
            "send-keys",
            "-t", pane_id,
            &format!("export PROMPT='{}'", prompt.replace('\'', "'\\''")),
            "Enter",
        ])
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Failed to set prompt: {}", e)))?;

    // Execute agent command
    Command::new("tmux")
        .args(&[
            "send-keys",
            "-t", pane_id,
            &agent_cmd,
            "Enter",
        ])
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Failed to start agent: {}", e)))?;

    tracing::info!("Started {} in pane {}", agent_name, pane_id);
    Ok(())
}

/// Monitor tmux panes for agent completion
pub fn monitor_agent_panes(
    session_name: &str,
    pane_ids: &[String],
    agent_names: &[String],
    timeout: Duration,
) -> Result<Vec<AgentResult>> {
    let start = Instant::now();
    let mut results = Vec::new();
    let mut pending_panes: Vec<(String, String)> = pane_ids
        .iter()
        .zip(agent_names.iter())
        .map(|(p, a)| (p.clone(), a.clone()))
        .collect();

    while !pending_panes.is_empty() {
        if start.elapsed() > timeout {
            // Timeout - kill remaining panes
            for (pane_id, agent_name) in &pending_panes {
                tracing::warn!("Agent {} timed out after {:?}", agent_name, timeout);
                kill_pane(pane_id).ok(); // Best effort
                results.push(AgentResult {
                    agent_name: agent_name.clone(),
                    status: AgentStatus::Timeout,
                    output: None,
                });
            }
            break;
        }

        // Check each pane for completion
        pending_panes.retain(|(pane_id, agent_name)| {
            match check_pane_status(pane_id) {
                Ok(PaneStatus::Running) => {
                    // Still running
                    true
                }
                Ok(PaneStatus::Exited(exit_code)) => {
                    tracing::info!("Agent {} completed with exit code {}", agent_name, exit_code);

                    // Capture output
                    let output = capture_pane_output(pane_id).ok();

                    results.push(AgentResult {
                        agent_name: agent_name.clone(),
                        status: if exit_code == 0 {
                            AgentStatus::Success
                        } else {
                            AgentStatus::Failed(exit_code)
                        },
                        output,
                    });

                    false // Remove from pending
                }
                Err(e) => {
                    tracing::error!("Failed to check pane {}: {}", pane_id, e);
                    results.push(AgentResult {
                        agent_name: agent_name.clone(),
                        status: AgentStatus::Error,
                        output: None,
                    });
                    false // Remove from pending
                }
            }
        });

        std::thread::sleep(Duration::from_millis(500)); // Poll every 500ms
    }

    // Cleanup session
    cleanup_tmux_session(session_name).ok();

    Ok(results)
}

/// Check if tmux pane is still running
fn check_pane_status(pane_id: &str) -> Result<PaneStatus> {
    // Use tmux display-message to check if pane exists
    let output = Command::new("tmux")
        .args(&["display-message", "-t", pane_id, "-p", "#{pane_pid}"])
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Failed to check pane: {}", e)))?;

    if !output.status.success() {
        // Pane doesn't exist - agent completed and pane closed
        return Ok(PaneStatus::Exited(0));
    }

    let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let pid: u32 = pid_str
        .parse()
        .map_err(|e| SpecKitError::from_string(format!("Invalid PID: {}", e)))?;

    // Check if process is still running
    if is_process_running(pid) {
        Ok(PaneStatus::Running)
    } else {
        // Process finished - check exit code from pane
        Ok(PaneStatus::Exited(0)) // TODO: Capture actual exit code
    }
}

/// Check if process with given PID is running
fn is_process_running(pid: u32) -> bool {
    std::fs::metadata(format!("/proc/{}", pid)).is_ok()
}

/// Capture output from tmux pane
fn capture_pane_output(pane_id: &str) -> Result<String> {
    let output = Command::new("tmux")
        .args(&["capture-pane", "-t", pane_id, "-p", "-S", "-"])
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Failed to capture pane: {}", e)))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Kill hung pane
fn kill_pane(pane_id: &str) -> Result<()> {
    Command::new("tmux")
        .args(&["kill-pane", "-t", pane_id])
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Failed to kill pane: {}", e)))?;

    Ok(())
}

/// Cleanup tmux session after agents complete
fn cleanup_tmux_session(session_name: &str) -> Result<()> {
    Command::new("tmux")
        .args(&["kill-session", "-t", session_name])
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Failed to kill session: {}", e)))?;

    Ok(())
}

#[derive(Debug)]
pub struct AgentResult {
    pub agent_name: String,
    pub status: AgentStatus,
    pub output: Option<String>,
}

#[derive(Debug)]
pub enum AgentStatus {
    Success,
    Failed(i32), // Exit code
    Timeout,
    Error,
}

enum PaneStatus {
    Running,
    Exited(i32), // Exit code
}
```

#### 2. Integration with Agent Orchestrator

**File**: `tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

**Modify**: `auto_submit_spec_stage_prompt()` function

```rust
// Current approach: Use agent manager (background, invisible)
// New approach: Use tmux spawner (observable, attachable)

pub fn auto_submit_spec_stage_prompt(
    widget: &mut ChatWidget,
    stage: SpecStage,
    spec_id: &str,
) {
    // ... existing preamble ...

    // NEW: Check if observable mode enabled
    let observable_mode = widget.config.spec_kit_observable_agents();

    if observable_mode {
        // Use tmux-based spawning
        spawn_agents_via_tmux(widget, stage, spec_id, agents);
    } else {
        // Use existing agent manager (backward compat)
        spawn_agents_via_manager(widget, stage, spec_id, agents);
    }
}

fn spawn_agents_via_tmux(
    widget: &mut ChatWidget,
    stage: SpecStage,
    spec_id: &str,
    agents: Vec<(String, String)>,
) {
    let config = super::tmux_agent_spawner::TmuxAgentConfig {
        session_name: format!("agents-{}-{}-{}", spec_id, stage.command_name(), &widget.spec_auto_state.as_ref().unwrap().run_id.as_ref().unwrap()[..8]),
        spec_id: spec_id.to_string(),
        stage,
        run_id: widget.spec_auto_state.as_ref().unwrap().run_id.clone().unwrap(),
        evidence_dir: widget.config.cwd.join("evidence/agents"),
        timeout: Duration::from_secs(1200), // AR-1 timeout
    };

    // Spawn agents in tmux
    match super::tmux_agent_spawner::spawn_agents_in_tmux(
        &config,
        &agents,
        &widget.config.cwd,
    ) {
        Ok(pane_ids) => {
            // Show user how to attach
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    ratatui::text::Line::from(format!(
                        "🔍 Observable mode: Agents running in tmux session '{}'",
                        config.session_name
                    )),
                    ratatui::text::Line::from(format!(
                        "   Attach: tmux attach -t {}",
                        config.session_name
                    )),
                    ratatui::text::Line::from(
                        "   Detach: Ctrl+b d"
                    ),
                ],
                HistoryCellType::Notice,
            ));

            // Monitor for completion
            let agent_names: Vec<String> = agents.iter().map(|(n, _)| n.clone()).collect();

            match super::tmux_agent_spawner::monitor_agent_panes(
                &config.session_name,
                &pane_ids,
                &agent_names,
                config.timeout,
            ) {
                Ok(results) => {
                    // Process results (same as existing agent manager flow)
                    process_agent_results(widget, stage, spec_id, results);
                }
                Err(e) => {
                    halt_spec_auto_with_error(
                        widget,
                        format!("Agent monitoring failed: {}", e),
                    );
                }
            }
        }
        Err(e) => {
            halt_spec_auto_with_error(
                widget,
                format!("Failed to spawn agents in tmux: {}", e),
            );
        }
    }
}
```

#### 3. Configuration

**File**: `tui/src/chatwidget/spec_kit/state.rs`

```rust
impl ChatWidget {
    /// Check if observable agent execution is enabled
    pub fn spec_kit_observable_agents(&self) -> bool {
        // Environment variable override
        if let Ok(val) = std::env::var("SPEC_KIT_OBSERVABLE_AGENTS") {
            return val == "1" || val.to_lowercase() == "true";
        }

        // Config file setting
        self.config
            .shell_environment_policy
            .set
            .get("SPEC_KIT_OBSERVABLE_AGENTS")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false) // Default: disabled (backward compat)
    }
}
```

**File**: `config.toml.example`

```toml
[shell_environment_policy.set]
# Enable observable agent execution via tmux panes
# When enabled, agents run in tmux panes where you can watch real-time progress
# Attach with: tmux attach -t agents-<spec-id>-<stage>-<run-id>
# SPEC_KIT_OBSERVABLE_AGENTS = "1"
```

---

## Migration Strategy

### Phase 1: Core Implementation (4 hours)
1. Create `tmux_agent_spawner.rs` module (agent spawning, monitoring)
2. Modify `agent_orchestrator.rs` (add tmux code path)
3. Add config support in `state.rs`
4. Update `mod.rs` exports

### Phase 2: Agent Command Integration (2 hours)
**Challenge**: Need to understand how current agent manager spawns agents

**Current flow** (needs investigation):
```
agent_orchestrator.rs → AgentManager → ??? → Agent executes
```

**Need to find**:
- How agents are invoked (CLI command? API call?)
- What format prompts are in
- How results are collected
- Integration with `codex-core` agent system

**Action**: Investigate `agent_orchestrator.rs` and agent manager code

### Phase 3: Testing (1 hour)
1. Unit tests for tmux operations
2. Integration test with mock agents
3. E2E test with SPEC-900 Plan stage

### Phase 4: Validation (1 hour)
1. Enable: `SPEC_KIT_OBSERVABLE_AGENTS=1`
2. Run: `/speckit.auto SPEC-KIT-900 --from spec-plan`
3. Attach: `tmux attach -t agents-SPEC-KIT-900-plan-...`
4. Watch: Claude agent in real-time to see where it hangs
5. Debug: Identify exact failure point

**Total**: ~8 hours

---

## Immediate Next Step

Before full implementation, let me **investigate current agent spawning** to understand:
1. How are agents currently invoked?
2. What's the agent manager interface?
3. Can we easily intercept and wrap with tmux?

Should I investigate the agent spawning mechanism now to design the correct integration?

