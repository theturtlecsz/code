# Tasks: SPEC-936 Tmux Elimination

**Created**: 2025-11-15
**Phase**: 1 Complete → Phase 2 Ready
**Total Estimated**: 27-35 hours (across Phases 2-6)

---

## Task Overview

| Phase | Description | Tasks | Hours | Dependencies |
|-------|-------------|-------|-------|--------------|
| **Phase 1** | Analysis & Design | ✅ 1.1-1.5 | 10h | COMPLETE |
| **Phase 2** | Core Async Infrastructure | T2.1-T2.4 | 8-10h | Phase 1 |
| **Phase 3** | Agent Tool Integration | T3.1-T3.5 | 6-8h | Phase 2 |
| **Phase 4** | Orchestrator Cleanup | T4.1-T4.4 | 3-4h | Phase 3 |
| **Phase 5** | Testing & Validation | T5.1-T5.6 | 8-10h | Phase 4 |
| **Phase 6** | Documentation & Evidence | T6.1-T6.4 | 2-3h | Phase 5 |

---

## Phase 1: Analysis & Design ✅ COMPLETE

### Component 1.1: Read PRD ✅
- **Status**: Complete (Session 1)
- **Hours**: 1h
- **Deliverable**: Understanding of 45-65h project scope

### Component 1.2: Tmux Inventory ✅
- **Status**: Complete (Session 1)
- **Hours**: 3h
- **Deliverable**: docs/SPEC-KIT-936-tmux-elimination/tmux-inventory.md
- **Findings**:
  - 7 call sites across 2 files
  - 851 LOC tmux.rs module
  - 93% latency overhead (6.5s/7s total)
  - MEDIUM complexity (11-15h removal estimate)

### Component 1.3: Async Architecture Design ✅
- **Status**: Complete (Session 2)
- **Hours**: 3h
- **Deliverable**: AsyncAgentExecutor trait + DirectProcessExecutor design in plan.md
- **Key Decisions**:
  - Exit codes for completion (not polling)
  - stdin for large prompts (not heredoc)
  - Streaming I/O (not file redirection)
  - 99.8% latency improvement (6.5s → <10ms)

### Component 1.4: OAuth2 Research ✅
- **Status**: Complete (Session 2)
- **Hours**: 2h
- **Deliverable**: OAuth2 authentication strategy in plan.md
- **Findings**:
  - Anthropic: Simple API key (0.5-1h)
  - OpenAI: API key or experimental device code (1h)
  - Google: gcloud --no-browser (2-3h)
  - Recommendation: Error detection only (no active flow)

### Component 1.5: Task Breakdown ✅
- **Status**: Complete (Session 2)
- **Hours**: 1h
- **Deliverable**: This file (tasks.md)

**Phase 1 Total**: 10 hours ✅

---

## Phase 2: Core Async Infrastructure (8-10h)

**Objective**: Create AsyncAgentExecutor trait and DirectProcessExecutor implementation with full async I/O streaming, timeout handling, and error detection.

---

### T2.1: Create AsyncAgentExecutor Trait

**File**: `codex-rs/core/src/async_agent_executor.rs` (new file)

**Estimated**: 2h

**Description**: Implement trait interface for async agent execution without tmux dependency.

**Acceptance Criteria**:
- ✅ `AgentOutput` struct defined (stdout, stderr, exit_code, duration, timed_out)
- ✅ `AgentExecutionError` enum defined (CommandNotFound, Timeout, ProcessCrash, OAuth2Required, IoError, OutputCaptureFailed)
- ✅ `AsyncAgentExecutor` trait defined with `execute()` method
- ✅ Full documentation with examples
- ✅ Compiles without errors

**Implementation**:
```rust
// codex-rs/core/src/async_agent_executor.rs
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
    pub timed_out: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentExecutionError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Execution timeout after {0}s")]
    Timeout(u64),

    #[error("Process crashed: {0}")]
    ProcessCrash(String),

    #[error("OAuth2 authentication required: {0}")]
    OAuth2Required(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Output capture failed: {0}")]
    OutputCaptureFailed(String),
}

#[async_trait::async_trait]
pub trait AsyncAgentExecutor: Send + Sync {
    async fn execute(
        &self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        working_dir: Option<&Path>,
        timeout_secs: u64,
        large_input: Option<&str>,
    ) -> Result<AgentOutput, AgentExecutionError>;
}
```

**Dependencies**: thiserror crate (already in Cargo.toml), async-trait crate

---

### T2.2: Implement DirectProcessExecutor

**File**: `codex-rs/core/src/async_agent_executor.rs`

**Estimated**: 4-5h

**Description**: Implement DirectProcessExecutor with streaming I/O, timeout handling, and error detection.

**Acceptance Criteria**:
- ✅ Spawns tokio::process::Command correctly
- ✅ Handles large_input via stdin pipe
- ✅ Streams stdout/stderr with tokio::spawn tasks
- ✅ Implements timeout with tokio::time::timeout
- ✅ Detects OAuth2 errors (pattern matching on stderr)
- ✅ Returns AgentOutput with all fields populated
- ✅ Properly handles process cleanup (kill_on_drop)
- ✅ Compiles without errors or warnings

**Implementation Pattern** (based on exec.rs:332-368):
```rust
pub struct DirectProcessExecutor;

#[async_trait::async_trait]
impl AsyncAgentExecutor for DirectProcessExecutor {
    async fn execute(...) -> Result<AgentOutput, AgentExecutionError> {
        let start = std::time::Instant::now();

        // Spawn process
        let mut child = Command::new(command)
            .args(args)
            .envs(env.iter())
            .current_dir(working_dir.unwrap_or_else(|| Path::new(".")))
            .stdin(if large_input.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        // Send large input via stdin
        // ... (see plan.md:1.3.2)

        // Spawn streaming tasks
        let stdout_handle = tokio::spawn(...);
        let stderr_handle = tokio::spawn(...);

        // Wait with timeout
        let (exit_status, timed_out) = match tokio::time::timeout(...).await { ... };

        // Collect outputs
        let stdout = stdout_handle.await??;
        let stderr = stderr_handle.await??;

        // Detect OAuth2 errors
        if stderr.contains("ANTHROPIC_API_KEY") || ... { ... }

        Ok(AgentOutput { ... })
    }
}
```

**Dependencies**: tokio::process, tokio::io::BufReader, tokio::time::timeout

---

### T2.3: Add Unit Tests

**File**: `codex-rs/core/src/async_agent_executor.rs` (tests module)

**Estimated**: 1.5-2h

**Description**: Comprehensive unit tests for DirectProcessExecutor.

**Test Cases**:
1. ✅ `test_successful_execution`: Execute `echo "test"` → verify stdout
2. ✅ `test_large_input_via_stdin`: Send >1KB input → verify complete output
3. ✅ `test_timeout`: Execute `sleep 10` with 1s timeout → verify Timeout error
4. ✅ `test_command_not_found`: Execute non-existent command → verify CommandNotFound
5. ✅ `test_oauth2_detection`: Mock stderr with "ANTHROPIC_API_KEY" → verify OAuth2Required
6. ✅ `test_exit_code_propagation`: Execute command with exit code 42 → verify exit_code field
7. ✅ `test_stderr_capture`: Execute command that writes to stderr → verify stderr field
8. ✅ `test_concurrent_execution`: Spawn 3 agents in parallel → verify all complete

**Acceptance Criteria**:
- ✅ All 8 tests pass
- ✅ `cargo test async_agent_executor` succeeds
- ✅ No warnings or clippy violations

**Example Test**:
```rust
#[tokio::test]
async fn test_successful_execution() {
    let executor = DirectProcessExecutor;
    let result = executor.execute(
        "echo",
        &vec!["test".to_string()],
        &HashMap::new(),
        None,
        60,
        None,
    ).await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.stdout.trim(), "test");
    assert_eq!(output.exit_code, 0);
    assert!(!output.timed_out);
}
```

---

### T2.4: Integration Tests with Real CLIs

**File**: `codex-rs/core/tests/async_agent_integration.rs` (new file)

**Estimated**: 0.5-1h

**Description**: Integration tests with actual AI CLI tools (if available).

**Test Cases** (conditional on CLI availability):
1. ✅ `test_claude_cli_execution`: Execute `claude -p "test"` with API key → verify output
2. ✅ `test_gemini_cli_execution`: Execute `gemini -p "test"` → verify output
3. ✅ `test_large_prompt_handling`: Execute with >50KB prompt → verify no truncation
4. ✅ `test_oauth2_error_detection`: Execute without API key → verify error message

**Acceptance Criteria**:
- ✅ Tests skip gracefully if CLI not available (use `#[ignore]` attribute)
- ✅ Tests pass when CLIs are installed and configured
- ✅ `cargo test --test async_agent_integration` succeeds

**Example**:
```rust
#[tokio::test]
#[ignore = "Requires claude CLI and ANTHROPIC_API_KEY"]
async fn test_claude_cli_execution() {
    let executor = DirectProcessExecutor;
    let mut env = HashMap::new();
    env.insert("ANTHROPIC_API_KEY".to_string(),
               std::env::var("ANTHROPIC_API_KEY").unwrap());

    let result = executor.execute(
        "claude",
        &vec!["-p".to_string(), "Say: test".to_string()],
        &env,
        None,
        60,
        None,
    ).await;

    assert!(result.is_ok());
}
```

---

**Phase 2 Complete Criteria**:
- ✅ async_agent_executor.rs module exists with trait + implementation
- ✅ All unit tests pass (8/8)
- ✅ Integration tests implemented (skip if CLIs unavailable)
- ✅ `cargo build --workspace` succeeds
- ✅ `cargo clippy --workspace -- -D warnings` succeeds
- ✅ Ready to integrate into agent_tool.rs

**Phase 2 Total**: 8-10 hours

---

## Phase 3: Agent Tool Integration (6-8h)

**Objective**: Replace tmux execution path with DirectProcessExecutor in agent_tool.rs.

---

### T3.1: Remove Tmux If-Block

**File**: `codex-rs/core/src/agent_tool.rs`

**Estimated**: 1h

**Description**: Delete lines 1228-1369 (tmux execution path) while preserving fallback logic.

**Steps**:
1. Backup current agent_tool.rs
2. Delete lines 1228-1369 (tmux if-block)
3. Verify direct execution path starts at line 1370 (now becomes primary path)
4. Verify compilation succeeds

**Acceptance Criteria**:
- ✅ Lines 1228-1369 deleted (tmux if-block removed)
- ✅ Direct execution path (formerly lines 1370+) becomes sole execution path
- ✅ `cargo build -p codex-core` succeeds
- ✅ No references to `crate::tmux::*` in agent_tool.rs

**Verification**:
```bash
# Before: 7 tmux function calls
grep -c "crate::tmux::" codex-rs/core/src/agent_tool.rs  # Should be 0 after

# Before: Lines 1228-1369 exist
wc -l codex-rs/core/src/agent_tool.rs  # Should be ~140 lines shorter
```

---

### T3.2: Integrate DirectProcessExecutor

**File**: `codex-rs/core/src/agent_tool.rs`

**Estimated**: 2-3h

**Description**: Replace current `child.wait_with_output()` with DirectProcessExecutor for async streaming and timeout.

**Changes**:
```rust
// OLD (line 1535):
child.wait_with_output().await
    .map_err(|e| format!("Failed to read output: {}", e))?

// NEW:
use crate::async_agent_executor::{DirectProcessExecutor, AsyncAgentExecutor};

let executor = DirectProcessExecutor;
let large_input = if prompt.len() > 1000 { Some(prompt) } else { None };

match executor.execute(
    &command,
    &args,
    &env,
    working_dir.as_deref(),
    600,  // 10 minute timeout
    large_input,
).await {
    Ok(output) => {
        if !output.exit_code == 0 {
            return Err(format!("Agent failed with exit code {}: {}",
                output.exit_code, output.stderr));
        }
        Ok(output.stdout)
    }
    Err(AgentExecutionError::OAuth2Required(msg)) => {
        Err(format!("Authentication required: {}", msg))
    }
    Err(AgentExecutionError::Timeout(secs)) => {
        Err(format!("Agent timed out after {}s", secs))
    }
    Err(e) => Err(e.to_string()),
}
```

**Acceptance Criteria**:
- ✅ DirectProcessExecutor imported and used
- ✅ Timeout handling integrated (600s default)
- ✅ OAuth2 error detection working
- ✅ Exit code validation (0 = success)
- ✅ `cargo build -p codex-core` succeeds
- ✅ `cargo test -p codex-core` passes

---

### T3.3: Add Large Prompt Handling

**File**: `codex-rs/core/src/agent_tool.rs`

**Estimated**: 1-1.5h

**Description**: Detect large prompts (>1KB) and send via stdin instead of command-line args.

**Changes**:
```rust
// Detect large prompt
const LARGE_PROMPT_THRESHOLD: usize = 1000;
let large_input = if prompt.len() > LARGE_PROMPT_THRESHOLD {
    Some(prompt)
} else {
    None
};

// Adjust args for large prompts
let mut args: Vec<String> = Vec::new();
// ... (existing arg building)

// For large prompts, use "-" to read from stdin
if large_input.is_some() {
    args.push("-".to_string());  // Most CLIs support this
} else {
    args.push(prompt.to_string());
}

// Execute with large_input
executor.execute(command, &args, &env, working_dir, 600, large_input).await?
```

**Acceptance Criteria**:
- ✅ Prompts >1KB sent via stdin (large_input parameter)
- ✅ Prompts ≤1KB sent as command-line arg (existing behavior)
- ✅ Test with 50KB prompt → no truncation
- ✅ `cargo test -p codex-core` passes

---

### T3.4: Remove tmux_enabled Field

**File**: `codex-rs/core/src/agent_tool.rs`

**Estimated**: 1h

**Description**: Remove tmux_enabled field from Agent struct and all references.

**Files to Modify**:
1. `codex-rs/core/src/agent_tool.rs`:
   - Remove line 58: `pub tmux_enabled: bool` field
   - Remove line 242: default `false` in create_agent_with_config()
   - Remove line 695: `tmux_enabled` extraction in execute_agent()
   - Remove line 1067: `use_tmux` parameter in execute_model_with_permissions()

2. `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`:
   - Remove lines 286-288: `SPEC_KIT_OBSERVABLE_AGENTS` env var check
   - Remove line 347: `tmux_enabled` parameter in create_agent_from_config_name()
   - Remove line 674: `tmux_enabled` parameter

3. `codex-rs/tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs`:
   - Remove line 128: `tmux_enabled: true`

4. `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs`:
   - Remove line 956: `tmux_enabled: false`

**Acceptance Criteria**:
- ✅ `Agent` struct no longer has `tmux_enabled` field
- ✅ All Agent constructors updated (no tmux_enabled parameter)
- ✅ execute_model_with_permissions() no longer has `use_tmux` parameter
- ✅ `cargo build --workspace` succeeds
- ✅ `cargo clippy --workspace -- -D warnings` succeeds

**Verification**:
```bash
grep -r "tmux_enabled" codex-rs/  # Should return 0 matches
grep -r "use_tmux" codex-rs/core/src/agent_tool.rs  # Should return 0 matches
```

---

### T3.5: Update Agent Constructors

**File**: `codex-rs/core/src/agent_tool.rs`

**Estimated**: 0.5-1h

**Description**: Update create_agent_from_config_name() and create_agent_internal() to remove tmux_enabled parameter.

**Changes**:
```rust
// OLD:
pub async fn create_agent_from_config_name(
    name: &str,
    tmux_enabled: bool,  // REMOVE THIS
) -> Result<Agent, String> { ... }

// NEW:
pub async fn create_agent_from_config_name(
    name: &str,
) -> Result<Agent, String> { ... }
```

**Acceptance Criteria**:
- ✅ All call sites updated across workspace
- ✅ `cargo build --workspace` succeeds
- ✅ `cargo test --workspace` passes (maintain 100% pass rate)

---

**Phase 3 Complete Criteria**:
- ✅ Tmux if-block removed (agent_tool.rs:1228-1369)
- ✅ DirectProcessExecutor integrated
- ✅ Large prompt handling (>1KB → stdin)
- ✅ tmux_enabled field removed from Agent struct
- ✅ All Agent constructors updated
- ✅ Zero references to tmux in agent execution path
- ✅ All tests pass (100% pass rate maintained)

**Phase 3 Total**: 6-8 hours

---

## Phase 4: Orchestrator Cleanup (3-4h)

**Objective**: Remove zombie cleanup logic and tmux.rs module.

---

### T4.1: Remove Zombie Cleanup

**File**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

**Estimated**: 0.5h

**Description**: Delete zombie pane detection and cleanup calls (no longer needed without tmux).

**Lines to Remove**:
- Lines 290-310: Zombie cleanup logic in spawn_and_wait_for_agent()
- Line 296: `crate::tmux::check_zombie_panes()` call
- Line 304: `crate::tmux::cleanup_zombie_panes()` call

**Acceptance Criteria**:
- ✅ Zombie cleanup code removed
- ✅ No references to `check_zombie_panes` or `cleanup_zombie_panes`
- ✅ `cargo build -p codex-tui` succeeds

---

### T4.2: Remove SPEC_KIT_OBSERVABLE_AGENTS Env Var

**File**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

**Estimated**: 0.5h

**Description**: Remove environment variable check for tmux mode (no longer relevant).

**Lines to Remove**:
- Lines 286-288: `SPEC_KIT_OBSERVABLE_AGENTS` env var check
- Line 619-623: Similar env var check (if exists)

**Acceptance Criteria**:
- ✅ No references to `SPEC_KIT_OBSERVABLE_AGENTS` in codebase
- ✅ `cargo build --workspace` succeeds

**Verification**:
```bash
grep -r "SPEC_KIT_OBSERVABLE_AGENTS" codex-rs/  # Should return 0 matches
```

---

### T4.3: Update Quality Gate Orchestrators

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs`
- `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs`

**Estimated**: 0.5h

**Description**: Remove tmux_enabled parameters from quality gate agent creation.

**Changes**:
1. native_quality_gate_orchestrator.rs:128 - Remove `tmux_enabled: true`
2. quality_gate_handler.rs:956 - Remove `tmux_enabled: false`

**Acceptance Criteria**:
- ✅ Quality gate orchestrators updated
- ✅ `cargo build -p codex-tui` succeeds
- ✅ Quality gate tests pass

---

### T4.4: Delete tmux.rs Module

**Files**:
- `codex-rs/core/src/tmux.rs` (DELETE)
- `codex-rs/core/src/lib.rs` (update)

**Estimated**: 1.5-2h

**Description**: Delete entire tmux.rs module (851 LOC) and remove from lib.rs exports.

**Steps**:
1. Verify no remaining references to `crate::tmux::*`
2. Delete codex-rs/core/src/tmux.rs
3. Remove `pub mod tmux;` from codex-rs/core/src/lib.rs (line 91)
4. Verify compilation succeeds

**Acceptance Criteria**:
- ✅ tmux.rs file deleted
- ✅ `pub mod tmux;` removed from lib.rs
- ✅ `grep -r "crate::tmux" codex-rs/` returns 0 matches
- ✅ `cargo build --workspace` succeeds
- ✅ `cargo test --workspace` maintains 100% pass rate

**Verification**:
```bash
# Verify no references
rg "crate::tmux" codex-rs/  # Should return nothing
rg "use.*tmux" codex-rs/    # Should return nothing (except test comments)

# Verify file deleted
ls codex-rs/core/src/tmux.rs  # Should error: No such file

# Verify build succeeds
cd codex-rs && cargo build --workspace --all-features
```

---

**Phase 4 Complete Criteria**:
- ✅ Zombie cleanup removed
- ✅ SPEC_KIT_OBSERVABLE_AGENTS env var removed
- ✅ Quality gate orchestrators updated
- ✅ tmux.rs module deleted (851 LOC removed)
- ✅ Zero references to tmux in codebase
- ✅ All tests pass

**Phase 4 Total**: 3-4 hours

---

## Phase 5: Testing & Validation (8-10h)

**Objective**: Comprehensive testing to ensure no regressions and validate performance improvements.

---

### T5.1: Full Test Suite Execution

**Command**: `cargo test --workspace --all-features`

**Estimated**: 2-3h (includes debugging time for any failures)

**Acceptance Criteria**:
- ✅ 604 tests pass (100% pass rate)
- ✅ 0 test failures
- ✅ No new warnings or errors
- ✅ Test output logged to evidence file

**Evidence**:
```bash
cd codex-rs
cargo test --workspace --all-features 2>&1 | tee ../docs/SPEC-KIT-936-tmux-elimination/evidence/test-results.log
```

---

### T5.2: Performance Benchmarking

**Estimated**: 2h

**Description**: Measure agent spawn latency before (tmux) vs after (direct).

**Test Cases**:
1. ✅ Single agent execution time
2. ✅ 3 agents parallel execution time
3. ✅ 10 agents sequential execution time

**Baseline** (from tmux-inventory.md):
- Single agent: 6.5s (tmux overhead)
- 3 agents parallel: 6.5s (shared session)

**Target**:
- Single agent: <50ms (99.2% improvement)
- 3 agents parallel: <50ms (99.2% improvement)

**Benchmark Script**:
```rust
// codex-rs/core/benches/agent_execution_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_agent_execution(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("single_agent_echo", |b| {
        b.to_async(&rt).iter(|| async {
            let executor = DirectProcessExecutor;
            executor.execute(
                "echo",
                &vec!["test".to_string()],
                &HashMap::new(),
                None,
                60,
                None,
            ).await.unwrap()
        });
    });

    c.bench_function("three_agents_parallel", |b| {
        b.to_async(&rt).iter(|| async {
            let executor = DirectProcessExecutor;
            let handles = (0..3).map(|i| {
                let executor = executor.clone();
                tokio::spawn(async move {
                    executor.execute(...).await
                })
            }).collect::<Vec<_>>();

            for handle in handles {
                handle.await.unwrap().unwrap();
            }
        });
    });
}

criterion_group!(benches, benchmark_agent_execution);
criterion_main!(benches);
```

**Acceptance Criteria**:
- ✅ Benchmark results logged to evidence file
- ✅ Single agent: <50ms (meets target)
- ✅ 3 agents parallel: <200ms total (meets target)
- ✅ Performance improvement documented (6.5s → <50ms = 130x faster)

---

### T5.3: Large Prompt Testing

**Estimated**: 1h

**Description**: Verify no truncation for prompts >50KB.

**Test Cases**:
1. ✅ 10KB prompt → full output
2. ✅ 50KB prompt → full output
3. ✅ 100KB prompt → full output (stress test)

**Test Implementation**:
```rust
#[tokio::test]
async fn test_large_prompt_no_truncation() {
    let executor = DirectProcessExecutor;

    // Generate 50KB prompt
    let large_prompt = "A".repeat(50_000);

    let result = executor.execute(
        "cat",  // Echo back via stdin
        &vec!["-".to_string()],  // Read from stdin
        &HashMap::new(),
        None,
        60,
        Some(&large_prompt),
    ).await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.stdout.len(), 50_000, "Output truncated!");
}
```

**Acceptance Criteria**:
- ✅ All large prompt tests pass
- ✅ No truncation observed
- ✅ Logged to evidence file

---

### T5.4: Concurrent Execution Testing

**Estimated**: 1h

**Description**: Verify 3+ agents can execute in parallel without interference.

**Test Cases**:
1. ✅ 3 agents parallel → all complete successfully
2. ✅ 10 agents parallel → all complete successfully
3. ✅ Verify no resource contention or race conditions

**Test Implementation**:
```rust
#[tokio::test]
async fn test_concurrent_agent_execution() {
    let executor = Arc::new(DirectProcessExecutor);

    let handles: Vec<_> = (0..10).map(|i| {
        let executor = executor.clone();
        tokio::spawn(async move {
            executor.execute(
                "echo",
                &vec![format!("agent-{}", i)],
                &HashMap::new(),
                None,
                60,
                None,
            ).await
        })
    }).collect();

    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap().unwrap();
        assert_eq!(result.stdout.trim(), format!("agent-{}", i));
    }
}
```

**Acceptance Criteria**:
- ✅ All concurrent tests pass
- ✅ No deadlocks or race conditions
- ✅ Logged to evidence file

---

### T5.5: Timeout and Error Handling Testing

**Estimated**: 1h

**Description**: Verify timeout handling and error detection work correctly.

**Test Cases**:
1. ✅ Timeout: `sleep 10` with 1s timeout → Timeout error
2. ✅ Command not found: `nonexistent_command` → CommandNotFound error
3. ✅ OAuth2 detection: Mock stderr with "ANTHROPIC_API_KEY" → OAuth2Required error
4. ✅ Non-zero exit code: Command exits with code 42 → AgentOutput.exit_code == 42

**Acceptance Criteria**:
- ✅ All error handling tests pass
- ✅ Errors are actionable (include recovery instructions)
- ✅ Logged to evidence file

---

### T5.6: OAuth2 Error Detection Testing

**Estimated**: 1h

**Description**: Verify OAuth2 error patterns are detected and surfaced with actionable messages.

**Test Cases**:
1. ✅ Anthropic: stderr contains "ANTHROPIC_API_KEY" → "Set ANTHROPIC_API_KEY environment variable"
2. ✅ Google: stderr contains "gcloud auth" → "Run: gcloud auth login --no-browser"
3. ✅ OpenAI: stderr contains "OPENAI_API_KEY" → "Set OPENAI_API_KEY or run: codex login --experimental_use-device-code"

**Acceptance Criteria**:
- ✅ All OAuth2 detection tests pass
- ✅ Error messages are provider-specific and actionable
- ✅ Logged to evidence file

---

**Phase 5 Complete Criteria**:
- ✅ Full test suite passes (604/604, 100%)
- ✅ Performance targets met (<50ms single agent)
- ✅ Large prompts tested (>50KB, no truncation)
- ✅ Concurrent execution tested (10 agents parallel)
- ✅ Timeout and error handling validated
- ✅ OAuth2 error detection validated
- ✅ All evidence logged to docs/SPEC-KIT-936-tmux-elimination/evidence/

**Phase 5 Total**: 8-10 hours

---

## Phase 6: Documentation & Evidence (2-3h)

**Objective**: Update documentation and capture evidence for SPEC-936 completion.

---

### T6.1: Update timing.rs Documentation

**File**: `codex-rs/spec-kit/src/timing.rs`

**Estimated**: 0.5h

**Description**: Update documentation examples to use async process spawning instead of tmux.

**Changes**:
```rust
// OLD (lines 38, 43):
/// Example: Measuring tmux pane creation time
/// ```
/// let start = Instant::now();
/// tmux::create_pane(&session, &title, false).await?;
/// timing_record!(create_pane_ms, start.elapsed().as_millis() as u64);
/// ```

// NEW:
/// Example: Measuring async agent execution time
/// ```
/// let start = Instant::now();
/// executor.execute(command, args, env, dir, 600, None).await?;
/// timing_record!(agent_execution_ms, start.elapsed().as_millis() as u64);
/// ```
```

**Acceptance Criteria**:
- ✅ Documentation examples updated
- ✅ No references to tmux in examples
- ✅ `cargo doc --workspace` succeeds

---

### T6.2: Create Migration Guide

**File**: `docs/SPEC-KIT-936-tmux-elimination/migration-guide.md` (new)

**Estimated**: 1h

**Description**: Document migration from tmux-based to direct execution for users and contributors.

**Contents**:
1. Overview of changes
2. Breaking changes (SPEC_KIT_OBSERVABLE_AGENTS env var removed)
3. Performance improvements (6.5s → <50ms)
4. Troubleshooting guide (OAuth2 errors, timeout issues)
5. Examples of new error messages

**Acceptance Criteria**:
- ✅ Migration guide created
- ✅ Covers all breaking changes
- ✅ Includes troubleshooting section

---

### T6.3: Capture Performance Metrics

**File**: `docs/SPEC-KIT-936-tmux-elimination/evidence/performance-metrics.md` (new)

**Estimated**: 0.5h

**Description**: Document before/after performance metrics.

**Contents**:
| Metric | Before (tmux) | After (direct) | Improvement |
|--------|---------------|----------------|-------------|
| Single agent spawn | 6500ms | <50ms | 99.2% |
| 3 agents parallel | 6500ms | <200ms | 96.9% |
| 10 agents sequential | 65000ms | <500ms | 99.2% |
| Memory per agent | 4MB (tmux) | 0MB | 100% |
| Temp files per agent | 4 files | 0 files | 100% |

**Acceptance Criteria**:
- ✅ Performance metrics documented
- ✅ Evidence files referenced
- ✅ Improvement percentages calculated

---

### T6.4: Update CLAUDE.md and SPEC.md

**Files**:
- `CLAUDE.md`
- `SPEC.md`

**Estimated**: 0.5-1h

**Description**: Update project documentation to reflect tmux removal.

**Changes to CLAUDE.md**:
- Remove references to tmux session management
- Update agent execution flow description
- Note performance improvements

**Changes to SPEC.md**:
- Mark SPEC-936 as COMPLETE
- Update status to "Done"
- Link to evidence files

**Acceptance Criteria**:
- ✅ CLAUDE.md updated
- ✅ SPEC.md updated (SPEC-936 row marked Done)
- ✅ Evidence links added

---

**Phase 6 Complete Criteria**:
- ✅ timing.rs examples updated
- ✅ Migration guide created
- ✅ Performance metrics documented
- ✅ CLAUDE.md and SPEC.md updated
- ✅ All documentation accurate and complete

**Phase 6 Total**: 2-3 hours

---

## Success Criteria Summary

**Phase 1** ✅:
- [x] Components 1.1-1.5 complete
- [x] plan.md exists with AsyncAgentExecutor design
- [x] OAuth2 research complete
- [x] tasks.md created (this file)

**Phase 2** (Ready):
- [ ] AsyncAgentExecutor trait defined
- [ ] DirectProcessExecutor implemented
- [ ] Unit tests pass (8/8)
- [ ] Integration tests implemented

**Phase 3** (Pending Phase 2):
- [ ] Tmux if-block removed
- [ ] DirectProcessExecutor integrated
- [ ] Large prompt handling added
- [ ] tmux_enabled field removed

**Phase 4** (Pending Phase 3):
- [ ] Zombie cleanup removed
- [ ] SPEC_KIT_OBSERVABLE_AGENTS removed
- [ ] tmux.rs deleted (851 LOC)
- [ ] Zero tmux references in codebase

**Phase 5** (Pending Phase 4):
- [ ] Full test suite passes (604/604)
- [ ] Performance targets met (<50ms)
- [ ] Large prompts tested (>50KB)
- [ ] Concurrent execution validated

**Phase 6** (Pending Phase 5):
- [ ] Documentation updated
- [ ] Migration guide created
- [ ] Performance metrics documented
- [ ] SPEC.md marked Done

---

## Risk Mitigation

| Risk | Mitigation | Status |
|------|------------|--------|
| CLI stdin compatibility | Fallback to temp file wrapper | Low (tested) |
| OAuth2 flow complexity | Error detection only (no active flow) | Resolved |
| Test failures after removal | Comprehensive test suite + manual validation | Medium |
| Performance regressions | Benchmark before/after | Low |
| Zombie processes | Use kill_on_drop + PDEATHSIG | Low |

---

## Evidence Repository Structure

```
docs/SPEC-KIT-936-tmux-elimination/
├── spec.md (PRD - to be created)
├── plan.md (Architecture - Session 2 ✅)
├── tasks.md (This file - Session 2 ✅)
├── tmux-inventory.md (Session 1 ✅)
├── migration-guide.md (Phase 6)
└── evidence/
    ├── test-results.log (Phase 5.1)
    ├── performance-metrics.md (Phase 6.3)
    ├── benchmark-results.json (Phase 5.2)
    ├── large-prompt-tests.log (Phase 5.3)
    ├── concurrent-execution-tests.log (Phase 5.4)
    ├── timeout-error-tests.log (Phase 5.5)
    └── oauth2-detection-tests.log (Phase 5.6)
```

---

**Next Session**: Phase 2 (Core Async Infrastructure) - Start with T2.1 (AsyncAgentExecutor trait definition).
