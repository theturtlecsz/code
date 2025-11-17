# End-to-End Testing Guide

Comprehensive guide to end-to-end testing of complete user workflows.

---

## Overview

**End-to-End (E2E) Testing Philosophy**: Test complete user workflows from start to finish, simulating real-world usage

**Goals**:
- Validate critical user journeys
- Test system integration (TUI + backend + database + MCP)
- Verify error recovery and degradation
- Ensure configuration hot-reload works

**Current Status**:
- ~24 E2E tests (4% of total)
- 100% pass rate
- Average execution time: ~10-60s per test
- Categories: Pipeline automation, quality checkpoints, tmux sessions, config hot-reload

---

## E2E Test Categories

### Pipeline Automation Tests

**Purpose**: Test complete `/speckit.auto` pipeline (Plan → Tasks → Implement → Validate → Audit → Unlock)

**Location**: `codex-rs/tui/tests/spec_auto_e2e.rs`

**Coverage**:
- Pipeline state machine (initialization, transitions, resume)
- Quality checkpoint integration (PrePlanning, PostPlan, PostTasks)
- Stage progression (all 6 stages)
- Error handling and recovery

---

### Quality Checkpoint Tests

**Purpose**: Test quality gates at critical pipeline points

**Checkpoints**:
- **PrePlanning** (BeforeSpecify): Clarify ambiguities before plan
- **PostPlan** (AfterSpecify): Checklist quality scoring after plan
- **PostTasks** (AfterTasks): Analyze consistency after tasks

**Coverage**:
- Checkpoint triggering
- Modification tracking
- Escalation logic
- Human intervention

---

### Tmux Session Tests

**Purpose**: Test tmux integration for long-running operations

**Location**: `codex-rs/evidence/tmux-automation/`

**Coverage**:
- Session creation and lifecycle
- Agent spawning in background
- Session termination
- Evidence collection

---

### Config Hot-Reload Tests

**Purpose**: Test configuration changes without restart

**Location**: `codex-rs/tui/tests/config_reload_integration_tests.rs`

**Coverage**:
- Config file watching
- Hot-reload triggers
- Provider switching
- <100ms latency (p95)

---

## Pipeline E2E Tests

### Test Structure

**Standard Pattern**:

```rust
#[test]
fn test_spec_auto_state_initialization() {
    // 1. Create initial state
    let state = SpecAutoState::new(
        "SPEC-TEST-001".to_string(),
        "Test automation".to_string(),
        SpecStage::Plan,
        None,  // HAL mode
    );

    // 2. Assert initial conditions
    assert_eq!(state.spec_id, "SPEC-TEST-001");
    assert_eq!(state.goal, "Test automation");
    assert_eq!(state.current_index, 0);
    assert_eq!(state.stages.len(), 6);
    assert_eq!(state.current_stage(), Some(SpecStage::Plan));
    assert!(state.quality_gates_enabled);
    assert!(state.completed_checkpoints.is_empty());
}
```

---

### Pattern 1: Pipeline Initialization

**Example: spec_auto_e2e.rs:20**

```rust
#[test]
fn test_spec_auto_state_initialization() {
    let state = SpecAutoState::new(
        "SPEC-TEST-001".to_string(),
        "Test automation".to_string(),
        SpecStage::Plan,
        None,
    );

    // Verify initial state
    assert_eq!(state.spec_id, "SPEC-TEST-001");
    assert_eq!(state.goal, "Test automation");
    assert_eq!(state.current_index, 0);
    assert_eq!(state.current_stage(), Some(SpecStage::Plan));

    // Verify stages
    assert_eq!(state.stages.len(), 6);
    let expected = vec![
        SpecStage::Plan,
        SpecStage::Tasks,
        SpecStage::Implement,
        SpecStage::Validate,
        SpecStage::Audit,
        SpecStage::Unlock,
    ];
    assert_eq!(state.stages, expected);

    // Verify quality gates
    assert!(state.quality_gates_enabled);
    assert!(state.completed_checkpoints.is_empty());
}
```

**What This Tests**:
- ✅ State initialization
- ✅ Stage ordering (Plan → Tasks → Implement → Validate → Audit → Unlock)
- ✅ Quality gates enabled by default
- ✅ Checkpoint tracking initialized

---

### Pattern 2: Pipeline Stage Progression

```rust
#[test]
fn test_pipeline_full_progression() {
    let mut state = SpecAutoState::new(
        "SPEC-TEST-002".to_string(),
        "Full pipeline test".to_string(),
        SpecStage::Plan,
        None,
    );

    // ==================== PLAN STAGE ====================
    assert_eq!(state.current_stage(), Some(SpecStage::Plan));
    assert_eq!(state.current_index, 0);

    // Simulate plan completion
    state.current_index += 1;

    // ==================== TASKS STAGE ====================
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));
    assert_eq!(state.current_index, 1);

    state.current_index += 1;

    // ==================== IMPLEMENT STAGE ====================
    assert_eq!(state.current_stage(), Some(SpecStage::Implement));
    assert_eq!(state.current_index, 2);

    state.current_index += 1;

    // ==================== VALIDATE STAGE ====================
    assert_eq!(state.current_stage(), Some(SpecStage::Validate));
    assert_eq!(state.current_index, 3);

    state.current_index += 1;

    // ==================== AUDIT STAGE ====================
    assert_eq!(state.current_stage(), Some(SpecStage::Audit));
    assert_eq!(state.current_index, 4);

    state.current_index += 1;

    // ==================== UNLOCK STAGE ====================
    assert_eq!(state.current_stage(), Some(SpecStage::Unlock));
    assert_eq!(state.current_index, 5);

    // ==================== COMPLETION ====================
    state.current_index += 1;
    assert_eq!(state.current_stage(), None); // Pipeline complete
}
```

**What This Tests**:
- ✅ All 6 stages execute in order
- ✅ Index advances correctly
- ✅ State transitions deterministically
- ✅ Pipeline completion (stage = None)

---

### Pattern 3: Resume from Middle Stage

```rust
#[test]
fn test_resume_from_tasks_stage() {
    // Start from Tasks (not Plan)
    let state = SpecAutoState::new(
        "SPEC-TEST-003".to_string(),
        "Resume test".to_string(),
        SpecStage::Tasks,  // Resume from Tasks
        None,
    );

    // Verify resume point
    assert_eq!(state.current_index, 1); // Tasks is index 1
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));

    // Verify can still progress
    let mut state = state;
    state.current_index += 1;
    assert_eq!(state.current_stage(), Some(SpecStage::Implement));
}
```

**What This Tests**:
- ✅ Pipeline can resume from any stage
- ✅ Index calculated correctly for resume
- ✅ Progression continues normally

---

## Quality Checkpoint E2E Tests

### Pattern 1: Checkpoint Tracking

```rust
#[test]
fn test_quality_checkpoints_track_completion() {
    let mut state = SpecAutoState::new(
        "SPEC-TEST-006".to_string(),
        "Checkpoint tracking".to_string(),
        SpecStage::Plan,
        None,
    );

    // Initially no checkpoints completed
    assert!(state.completed_checkpoints.is_empty());

    // ==================== PRE-PLANNING CHECKPOINT ====================

    // Simulate PrePlanning checkpoint (Clarify)
    state.completed_checkpoints.insert(QualityCheckpoint::PrePlanning);

    assert!(state.completed_checkpoints.contains(&QualityCheckpoint::PrePlanning));
    assert!(!state.completed_checkpoints.contains(&QualityCheckpoint::PostPlan));
    assert_eq!(state.completed_checkpoints.len(), 1);

    // ==================== POST-PLAN CHECKPOINT ====================

    // Simulate PostPlan checkpoint (Checklist)
    state.completed_checkpoints.insert(QualityCheckpoint::PostPlan);

    assert!(state.completed_checkpoints.contains(&QualityCheckpoint::PrePlanning));
    assert!(state.completed_checkpoints.contains(&QualityCheckpoint::PostPlan));
    assert_eq!(state.completed_checkpoints.len(), 2);

    // ==================== POST-TASKS CHECKPOINT ====================

    // Simulate PostTasks checkpoint (Analyze)
    state.completed_checkpoints.insert(QualityCheckpoint::PostTasks);

    assert_eq!(state.completed_checkpoints.len(), 3);
    assert!(state.completed_checkpoints.contains(&QualityCheckpoint::PostTasks));
}
```

**What This Tests**:
- ✅ Checkpoint completion tracked
- ✅ Multiple checkpoints can coexist
- ✅ No duplicate checkpoints (Set semantics)

---

### Pattern 2: Quality Modifications Tracking

```rust
#[test]
fn test_quality_modifications_tracked() {
    let mut state = SpecAutoState::new(
        "SPEC-TEST-007".to_string(),
        "Modification tracking".to_string(),
        SpecStage::Plan,
        None,
    );

    // Initially no modifications
    assert!(state.quality_modifications.is_empty());

    // ==================== PREPLANNING MODIFICATIONS ====================

    // User fixes ambiguities in spec.md
    state.quality_modifications.push("spec.md".to_string());

    assert_eq!(state.quality_modifications.len(), 1);
    assert!(state.quality_modifications.contains(&"spec.md".to_string()));

    // ==================== POSTPLAN MODIFICATIONS ====================

    // User improves plan.md after checklist
    state.quality_modifications.push("plan.md".to_string());

    assert_eq!(state.quality_modifications.len(), 2);
    assert!(state.quality_modifications.contains(&"plan.md".to_string()));

    // ==================== POSTTASKS MODIFICATIONS ====================

    // User fixes tasks.md after analyze
    state.quality_modifications.push("tasks.md".to_string());

    assert_eq!(state.quality_modifications.len(), 3);
}
```

**What This Tests**:
- ✅ Modifications tracked across checkpoints
- ✅ Multiple files can be modified
- ✅ Modification history preserved

---

### Pattern 3: Quality Gates Can Be Disabled

```rust
#[test]
fn test_quality_gates_can_be_disabled() {
    let state = SpecAutoState::with_quality_gates(
        "SPEC-TEST-008".to_string(),
        "No quality gates".to_string(),
        SpecStage::Plan,
        None,
        false,  // Disable quality gates
    );

    // Verify quality gates disabled
    assert!(!state.quality_gates_enabled);

    // Pipeline should skip all checkpoints
    // (checkpoint logic would check quality_gates_enabled flag)
}
```

**What This Tests**:
- ✅ Quality gates can be disabled
- ✅ Flag persists in state
- ✅ Pipeline can run without checkpoints

---

## Real-World E2E Tests

### Pattern 1: Apply Command E2E

**Example: apply_command_e2e.rs:78**

```rust
#[tokio::test]
async fn test_apply_command_creates_fibonacci_file() {
    // ==================== SETUP: TEMP GIT REPO ====================

    let temp_repo = create_temp_git_repo()
        .await
        .expect("Failed to create temp git repo");
    let repo_path = temp_repo.path();

    // ==================== LOAD TASK FIXTURE ====================

    let task_response = mock_get_task_with_fixture()
        .await
        .expect("Failed to load fixture");

    // ==================== EXECUTE: APPLY DIFF ====================

    apply_diff_from_task(task_response, Some(repo_path.to_path_buf()))
        .await
        .expect("Failed to apply diff from task");

    // ==================== VERIFY: FILE CREATED ====================

    let fibonacci_path = repo_path.join("scripts/fibonacci.js");
    assert!(fibonacci_path.exists(), "fibonacci.js was not created");

    // ==================== VERIFY: FILE CONTENTS ====================

    let contents = std::fs::read_to_string(&fibonacci_path)
        .expect("Failed to read fibonacci.js");

    assert!(
        contents.contains("function fibonacci(n)"),
        "fibonacci.js doesn't contain expected function"
    );
}
```

**Helper: Create Temp Git Repo**:
```rust
async fn create_temp_git_repo() -> anyhow::Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let envs = vec![
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
    ];

    // Initialize git repo
    Command::new("git")
        .envs(envs.clone())
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .await?;

    // Configure user
    Command::new("git")
        .envs(envs.clone())
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .await?;

    Command::new("git")
        .envs(envs.clone())
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .await?;

    // Create initial commit
    std::fs::write(repo_path.join("README.md"), "# Test Repo\n")?;

    Command::new("git")
        .envs(envs.clone())
        .args(["add", "README.md"])
        .current_dir(repo_path)
        .output()
        .await?;

    Command::new("git")
        .envs(envs.clone())
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .await?;

    Ok(temp_dir)
}
```

**What This Tests**:
- ✅ Complete apply command workflow
- ✅ Git integration (temp repo, commits)
- ✅ File creation and modification
- ✅ Task fixture loading

---

### Pattern 2: Login Flow E2E

**Example: login_server_e2e.rs:79**

```rust
#[tokio::test]
async fn end_to_end_login_flow_persists_auth_json() -> Result<()> {
    // ==================== SETUP: MOCK OAuth ISSUER ====================

    let (issuer_addr, issuer_handle) = start_mock_issuer();
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());

    // ==================== SETUP: TEMP CODEX HOME ====================

    let tmp = tempdir()?;
    let codex_home = tmp.path().to_path_buf();

    // Seed auth.json with stale data (should be overwritten)
    let stale_auth = serde_json::json!({
        "OPENAI_API_KEY": "sk-stale",
        "tokens": {
            "id_token": "stale.header.payload",
            "access_token": "stale-access",
            "refresh_token": "stale-refresh",
        }
    });
    std::fs::write(
        codex_home.join("auth.json"),
        serde_json::to_string_pretty(&stale_auth)?,
    )?;

    // ==================== EXECUTE: LOGIN FLOW ====================

    let options = ServerOptions {
        issuer: issuer.clone(),
        redirect_uri: "http://localhost:8080/callback".to_string(),
        codex_home: codex_home.clone(),
        // ... other options
    };

    run_login_server(options).await?;

    // ==================== VERIFY: AUTH.JSON UPDATED ====================

    let updated_auth = std::fs::read_to_string(codex_home.join("auth.json"))?;
    let auth_data: serde_json::Value = serde_json::from_str(&updated_auth)?;

    // Verify tokens refreshed
    assert_ne!(auth_data["tokens"]["access_token"], "stale-access");
    assert_eq!(auth_data["tokens"]["access_token"], "access-123");

    // ==================== CLEANUP: SHUTDOWN MOCK ====================

    drop(issuer_handle);

    Ok(())
}
```

**Helper: Start Mock OAuth Issuer**:
```rust
fn start_mock_issuer() -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tiny_http::Server::from_listener(listener, None).unwrap();

    let handle = thread::spawn(move || {
        while let Ok(mut req) = server.recv() {
            let url = req.url().to_string();
            if url.starts_with("/oauth/token") {
                // Build minimal JWT
                let payload = serde_json::json!({
                    "email": "user@example.com",
                    "https://api.openai.com/auth": {
                        "chatgpt_plan_type": "pro",
                    }
                });

                let id_token = create_jwt(&payload);

                let tokens = serde_json::json!({
                    "id_token": id_token,
                    "access_token": "access-123",
                    "refresh_token": "refresh-123",
                });

                let resp = tiny_http::Response::from_data(
                    serde_json::to_vec(&tokens).unwrap()
                );
                let _ = req.respond(resp);
            }
        }
    });

    (addr, handle)
}
```

**What This Tests**:
- ✅ Complete login flow
- ✅ OAuth integration (mock issuer)
- ✅ Token persistence (auth.json)
- ✅ Stale token replacement

---

## E2E Test Setup Patterns

### Pattern 1: Temp Git Repository

```rust
async fn create_temp_git_repo() -> anyhow::Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // Disable global git config (isolation)
    let envs = vec![
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
    ];

    // Initialize repo
    run_git_command(repo_path, &envs, &["init"]).await?;

    // Configure user (required for commits)
    run_git_command(repo_path, &envs, &["config", "user.email", "test@example.com"]).await?;
    run_git_command(repo_path, &envs, &["config", "user.name", "Test User"]).await?;

    // Create initial commit
    std::fs::write(repo_path.join("README.md"), "# Test\n")?;
    run_git_command(repo_path, &envs, &["add", "."]).await?;
    run_git_command(repo_path, &envs, &["commit", "-m", "Initial commit"]).await?;

    Ok(temp_dir)
}

async fn run_git_command(
    repo_path: &Path,
    envs: &[(&str, &str)],
    args: &[&str],
) -> anyhow::Result<()> {
    let output = Command::new("git")
        .envs(envs.iter().copied())
        .args(args)
        .current_dir(repo_path)
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!(
            "Git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
```

**Benefits**:
- ✅ Isolated from global git config
- ✅ Auto-cleanup (TempDir)
- ✅ Reusable helper functions

---

### Pattern 2: Mock HTTP Server

```rust
fn start_mock_server() -> (SocketAddr, thread::JoinHandle<()>) {
    // Bind to random port
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tiny_http::Server::from_listener(listener, None).unwrap();

    let handle = thread::spawn(move || {
        while let Ok(req) = server.recv() {
            let url = req.url().to_string();

            let response = match url.as_str() {
                "/api/v1/endpoint" => {
                    serde_json::json!({"status": "ok"})
                }
                _ => {
                    serde_json::json!({"error": "not found"})
                }
            };

            let resp = tiny_http::Response::from_data(
                serde_json::to_vec(&response).unwrap()
            );
            let _ = req.respond(resp);
        }
    });

    (addr, handle)
}

#[tokio::test]
async fn test_with_mock_server() {
    let (addr, _handle) = start_mock_server();
    let base_url = format!("http://{}:{}", addr.ip(), addr.port());

    // Test code using base_url...
}
```

**Benefits**:
- ✅ No external dependencies
- ✅ Deterministic responses
- ✅ Fast (no network)

---

### Pattern 3: Fixture Loading

```rust
async fn load_fixture<T: serde::de::DeserializeOwned>(name: &str) -> anyhow::Result<T> {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{}.json", name));

    let contents = std::fs::read_to_string(fixture_path)?;
    let data: T = serde_json::from_str(&contents)?;

    Ok(data)
}

#[tokio::test]
async fn test_with_fixture() {
    let task: GetTaskResponse = load_fixture("task_turn_fixture")
        .await
        .expect("Failed to load fixture");

    // Use task...
}
```

**Benefits**:
- ✅ Realistic test data
- ✅ Reusable across tests
- ✅ Version controlled

---

## Best Practices

### DO

**✅ Test complete user workflows**:
```rust
// Good: Tests entire pipeline
#[test]
fn test_speckit_auto_full_pipeline() {
    // Create state
    // Run plan
    // Run tasks
    // ... all 6 stages
    // Verify completion
}
```

---

**✅ Use realistic test data**:
```rust
// Good: Load from fixture
let task = load_fixture("real_task_response").await?;

// Bad: Minimal mock data
let task = GetTaskResponse { id: "1", content: "test" };
```

---

**✅ Verify side effects**:
```rust
// Verify file created
assert!(fibonacci_path.exists());

// Verify contents correct
let contents = std::fs::read_to_string(&fibonacci_path)?;
assert!(contents.contains("function fibonacci"));

// Verify git commit
let log = run_git(&["log", "--oneline"]).await?;
assert!(log.contains("Add fibonacci.js"));
```

---

**✅ Test error recovery**:
```rust
#[tokio::test]
async fn test_pipeline_recovers_from_mcp_failure() {
    // Simulate MCP failure
    mock_mcp.fail_next_request();

    // Run pipeline
    let result = run_pipeline().await;

    // Verify fallback succeeded
    assert!(result.is_ok());
    assert!(result.unwrap().degraded);
}
```

---

**✅ Clean up resources**:
```rust
#[tokio::test]
async fn test_with_cleanup() {
    let temp_dir = TempDir::new()?;
    let (_addr, handle) = start_mock_server();

    // Test logic...

    // Cleanup
    drop(handle);  // Shutdown mock server
    drop(temp_dir);  // Delete temp files

    Ok(())
}
```

---

### DON'T

**❌ Test too many workflows in one test**:
```rust
// Bad: Tests multiple workflows (hard to debug)
#[test]
fn test_all_commands() {
    test_apply_command();
    test_login_flow();
    test_config_reload();
    test_tmux_session();
    // ... 500 lines
}
```

---

**❌ Rely on external services**:
```rust
// Bad: Depends on real OpenAI API
#[tokio::test]
async fn test_real_api() {
    let response = reqwest::get("https://api.openai.com/v1/models").await?;
    // ❌ Flaky, slow, costs money
}

// Good: Use mock server
#[tokio::test]
async fn test_with_mock() {
    let (addr, _handle) = start_mock_server();
    let base_url = format!("http://{}", addr);
    // ✅ Fast, deterministic, free
}
```

---

**❌ Skip verification**:
```rust
// Bad: No assertions
#[tokio::test]
async fn test_pipeline() {
    run_pipeline().await?;
    // ❌ No verification
}

// Good: Verify outcomes
#[tokio::test]
async fn test_pipeline() {
    let result = run_pipeline().await?;
    assert_eq!(result.stages_completed, 6);
    assert!(result.plan_file.exists());
}
```

---

## Running E2E Tests

### Run All E2E Tests

```bash
cd codex-rs
cargo test --test '*_e2e'
```

**Runs**:
- `spec_auto_e2e.rs`
- `apply_command_e2e.rs`
- `login_server_e2e.rs`

---

### Run Specific E2E Test

```bash
cargo test --test spec_auto_e2e test_spec_auto_state_initialization
```

---

### Run with Verbose Output

```bash
cargo test --test spec_auto_e2e -- --nocapture --test-threads=1
```

**Why `--test-threads=1`**:
- E2E tests may conflict (ports, files)
- Single-threaded ensures isolation

---

## Summary

**E2E Testing Best Practices**:

1. **Complete Workflows**: Test from start to finish
2. **Realistic Data**: Use fixtures from real usage
3. **Isolation**: Temp dirs, mock servers, disable global config
4. **Verification**: Check files, state, side effects
5. **Error Recovery**: Test fallback and degradation
6. **Cleanup**: Auto-cleanup with TempDir, handle drops

**Test Categories**:
- ✅ Pipeline automation (/speckit.auto, 6 stages)
- ✅ Quality checkpoints (PrePlanning, PostPlan, PostTasks)
- ✅ Real-world workflows (apply command, login flow)
- ✅ Configuration hot-reload

**Key Patterns**:
- ✅ Temp git repositories (isolated, auto-cleanup)
- ✅ Mock HTTP servers (tiny_http, deterministic)
- ✅ Fixture loading (realistic test data)
- ✅ State machine validation (initialization, progression, resume)

**Next Steps**:
- [Property Testing Guide](property-testing-guide.md) - Generative invariant testing
- [CI/CD Integration](ci-cd-integration.md) - Automated testing pipeline
- [Performance Testing](performance-testing.md) - Benchmarks and profiling

---

**References**:
- Pipeline E2E: `codex-rs/tui/tests/spec_auto_e2e.rs`
- Apply command: `codex-rs/chatgpt/tests/suite/apply_command_e2e.rs`
- Login flow: `codex-rs/login/tests/suite/login_server_e2e.rs`
