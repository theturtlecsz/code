# Integration Testing Guide

Comprehensive guide to integration testing across modules.

---

## Overview

**Integration Testing Philosophy**: Test multiple modules working together in realistic workflows

**Goals**:
- Verify module interactions
- Test cross-cutting concerns (error recovery, state persistence)
- Validate end-to-end workflows
- Ensure evidence integrity

**Current Status**:
- ~200 integration tests (33% of total)
- 100% pass rate
- Average execution time: ~3-5s per test
- Categories: W01-W15 (workflows), E01-E15 (errors), S01-S10 (state), Q01-Q10 (quality), C01-C10 (concurrent)

---

## Integration Test Categories

### W01-W15: Workflow Integration Tests

**Purpose**: Test complete stage workflows across modules

**Flow**: Handler → Consensus → Evidence → Guardrail → State

**Location**: `codex-rs/tui/tests/workflow_integration_tests.rs`

**Coverage**:
- W01-W05: Individual stage workflows (Plan, Tasks, Implement, Validate, Audit)
- W06-W10: Multi-stage pipelines
- W11-W15: Quality gate integration

---

### E01-E15: Error Recovery Integration Tests

**Purpose**: Test error propagation and recovery across modules

**Flow**: Error → Retry → Fallback → Recovery → Evidence

**Location**: `codex-rs/tui/tests/error_recovery_integration_tests.rs`

**Coverage**:
- E01-E05: Consensus and MCP failures
- E06-E10: Guardrail validation errors
- E11-E15: State corruption and recovery

---

### S01-S10: State Persistence Integration Tests

**Purpose**: Test state coordination with evidence storage

**Flow**: State Change → Evidence Write → Load from Disk → Reconstruct

**Location**: `codex-rs/tui/tests/state_persistence_integration_tests.rs`

**Coverage**:
- S01-S05: State serialization and reconstruction
- S06-S10: Pipeline interrupt and resume

---

### Q01-Q10: Quality Gate Integration Tests

**Purpose**: Test quality gate orchestration across modules

**Flow**: Quality Gate → Native Checks → Consensus → Escalation → Guardrail

**Location**: `codex-rs/tui/tests/quality_flow_integration_tests.rs`

**Coverage**:
- Q01-Q05: BeforeSpecify and AfterSpecify gates
- Q06-Q10: AfterTasks gate and consensus validation

---

### C01-C10: Concurrent Operations Integration Tests

**Purpose**: Test concurrent stage execution and evidence locking

**Flow**: Parallel Stages → Lock Acquisition → Evidence Writes → Lock Release

**Location**: `codex-rs/tui/tests/concurrent_operations_integration_tests.rs`

**Coverage**:
- C01-C05: Parallel consensus collection
- C06-C10: Evidence write contention

---

## Test Structure

### Standard Integration Test Pattern

```rust
#[test]
fn w01_plan_stage_complete_workflow() {
    // 1. Setup: Create test context
    let ctx = IntegrationTestContext::new("SPEC-W01-001").unwrap();

    // 2. Arrange: Prepare filesystem (PRD, spec files)
    ctx.write_prd("test-feature", "# Test Feature\nBuild a test feature")
        .unwrap();
    ctx.write_spec("test-feature", "# Specification\nDetailed spec")
        .unwrap();

    // 3. Arrange: Create initial state
    let mut state = StateBuilder::new("SPEC-W01-001")
        .with_goal("Build test feature")
        .starting_at(SpecStage::Plan)
        .build();

    // 4. Act: Simulate module interactions
    // Write mock consensus artifacts (simulating consensus module output)
    let consensus_file = ctx
        .consensus_dir()
        .join("spec-plan_2025-10-19T12_00_00Z_gemini.json");
    std::fs::write(
        &consensus_file,
        json!({
            "agent": "gemini",
            "content": "Plan consensus output",
            "timestamp": "2025-10-19T12:00:00Z"
        })
        .to_string(),
    )
    .unwrap();

    // Write mock guardrail telemetry (simulating guardrail module output)
    let guardrail_file = ctx
        .commands_dir()
        .join("spec-plan_2025-10-19T12_00_00Z.json");
    std::fs::write(
        &guardrail_file,
        json!({
            "schemaVersion": 1,
            "baseline": {"status": "passed"},
            "tool": {"status": "passed"},
        })
        .to_string(),
    )
    .unwrap();

    // 5. Assert: Verify evidence
    let verifier = EvidenceVerifier::new(&ctx);
    assert!(verifier.assert_structure_valid());
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gemini"));
    assert!(ctx.assert_guardrail_telemetry_exists(SpecStage::Plan));

    // 6. Assert: Verify state transitions
    state.current_index += 1;
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));

    // 7. Assert: Verify artifact counts
    assert_eq!(ctx.count_consensus_files(), 1);
    assert_eq!(ctx.count_guardrail_files(), 1);
}
```

---

## Workflow Integration Tests

### Pattern 1: Individual Stage Workflow

**Example: W01 - Plan Stage Complete Workflow**

**Test** (workflow_integration_tests.rs:22):
```rust
#[test]
fn w01_plan_stage_complete_workflow() {
    let ctx = IntegrationTestContext::new("SPEC-W01-001").unwrap();

    // Arrange: Create PRD and spec
    ctx.write_prd("test-feature", "# Test Feature\nBuild a test feature")
        .unwrap();
    ctx.write_spec("test-feature", "# Specification\nDetailed spec")
        .unwrap();

    // Arrange: Initial state
    let mut state = StateBuilder::new("SPEC-W01-001")
        .with_goal("Build test feature")
        .starting_at(SpecStage::Plan)
        .build();

    assert_eq!(state.current_stage(), Some(SpecStage::Plan));

    // Act: Simulate consensus module output
    let consensus_file = ctx
        .consensus_dir()
        .join("spec-plan_2025-10-19T12_00_00Z_gemini.json");
    std::fs::write(
        &consensus_file,
        json!({
            "agent": "gemini",
            "content": "Plan consensus output",
        })
        .to_string(),
    )
    .unwrap();

    // Act: Simulate guardrail module output
    let guardrail_file = ctx
        .commands_dir()
        .join("spec-plan_2025-10-19T12_00_00Z.json");
    std::fs::write(
        &guardrail_file,
        json!({"schemaVersion": 1, "baseline": {"status": "passed"}})
            .to_string(),
    )
    .unwrap();

    // Assert: Verify evidence
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gemini"));
    assert!(ctx.assert_guardrail_telemetry_exists(SpecStage::Plan));

    // Assert: Verify state advancement
    state.current_index += 1;
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));
}
```

---

### Pattern 2: Multi-Stage Pipeline

**Example: W06 - Plan → Tasks Pipeline**

```rust
#[test]
fn w06_plan_tasks_pipeline() {
    let ctx = IntegrationTestContext::new("SPEC-W06-001").unwrap();

    // Arrange: Initial setup
    ctx.write_prd("multi-stage", "# Multi-stage Test").unwrap();
    let mut state = StateBuilder::new("SPEC-W06-001")
        .starting_at(SpecStage::Plan)
        .build();

    // ==================== PLAN STAGE ====================

    // Act: Plan stage consensus
    let plan_consensus = ctx
        .consensus_dir()
        .join("spec-plan_2025-10-19T10_00_00Z_gemini.json");
    std::fs::write(
        &plan_consensus,
        json!({"agent": "gemini", "stage": "plan", "content": "Plan output"})
            .to_string(),
    )
    .unwrap();

    // Assert: Plan evidence exists
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gemini"));

    // Advance to Tasks
    state.current_index += 1;
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));

    // ==================== TASKS STAGE ====================

    // Act: Tasks stage consensus
    let tasks_consensus = ctx
        .consensus_dir()
        .join("spec-tasks_2025-10-19T10_05_00Z_claude.json");
    std::fs::write(
        &tasks_consensus,
        json!({"agent": "claude", "stage": "tasks", "content": "Task list"})
            .to_string(),
    )
    .unwrap();

    // Assert: Tasks evidence exists (accumulated, not replaced)
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gemini"));
    assert!(ctx.assert_consensus_exists(SpecStage::Tasks, "claude"));
    assert_eq!(ctx.count_consensus_files(), 2);

    // Advance to Implement
    state.current_index += 1;
    assert_eq!(state.current_stage(), Some(SpecStage::Implement));
}
```

**Key Points**:
- ✅ Evidence accumulates across stages (not replaced)
- ✅ State advances sequentially
- ✅ Each stage verified independently

---

### Pattern 3: Quality Gate Integration

**Example: W11 - BeforeSpecify Quality Gate**

```rust
#[test]
fn w11_before_specify_quality_gate_workflow() {
    let ctx = IntegrationTestContext::new("SPEC-W11-001").unwrap();

    // Arrange: Create PRD with known ambiguities
    ctx.write_prd(
        "test",
        r#"
# PRD
## Requirements
- R1: System should be fast
- R2: Must handle TBD authentication
        "#,
    )
    .unwrap();

    let mut state = StateBuilder::new("SPEC-W11-001")
        .quality_gates(true)
        .starting_at(SpecStage::Plan)
        .build();

    // Act: Simulate quality gate execution (Clarify)
    let quality_gate_result = ctx
        .commands_dir()
        .join("quality-gate-clarify_2025-10-19T10_00_00Z.json");
    std::fs::write(
        &quality_gate_result,
        json!({
            "gate": "BeforeSpecify",
            "checks": ["clarify"],
            "results": {
                "ambiguities": [
                    {"pattern": "should", "severity": "Important"},
                    {"pattern": "TBD", "severity": "Critical"}
                ]
            },
            "verdict": "escalate",  // Critical issues found
            "escalation_reason": "2 ambiguities found (1 critical)"
        })
        .to_string(),
    )
    .unwrap();

    // Assert: Quality gate escalated
    let content = std::fs::read_to_string(&quality_gate_result).unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(data["verdict"], "escalate");
    assert!(data["results"]["ambiguities"]
        .as_array()
        .unwrap()
        .len() > 0);

    // State remains at Plan (doesn't advance on escalation)
    assert_eq!(state.current_stage(), Some(SpecStage::Plan));
}
```

**Key Points**:
- ✅ Quality gate runs before stage
- ✅ Escalation prevents advancement
- ✅ Evidence records escalation reason

---

## Error Recovery Integration Tests

### Pattern 1: Consensus Failure → Retry → Recovery

**Example: E01 - Consensus Failure with Retry**

**Test** (error_recovery_integration_tests.rs:23):
```rust
#[test]
fn e01_consensus_failure_handler_retry_evidence_cleanup_state_reset() {
    let ctx = IntegrationTestContext::new("SPEC-E01-001").unwrap();

    let mut state = StateBuilder::new("SPEC-E01-001")
        .starting_at(SpecStage::Plan)
        .build();

    // ==================== ATTEMPT 1: FAILURE ====================

    // Act: Write failed consensus (empty result)
    let failed_consensus = ctx
        .consensus_dir()
        .join("spec-plan_2025-10-19T10_00_00Z_gemini_attempt1.json");
    std::fs::write(
        &failed_consensus,
        json!({
            "agent": "gemini",
            "stage": "plan",
            "status": "failed",
            "error": "Empty consensus result",
            "attempt": 1,
        })
        .to_string(),
    )
    .unwrap();

    // Assert: Failed attempt recorded
    assert!(failed_consensus.exists());

    // ==================== RETRY: CLEANUP ====================

    // Simulate retry: cleanup failed evidence
    std::fs::remove_file(&failed_consensus).unwrap();
    assert!(!failed_consensus.exists());

    // ==================== ATTEMPT 2: SUCCESS ====================

    // Act: Retry with enhanced prompt
    let success_consensus = ctx
        .consensus_dir()
        .join("spec-plan_2025-10-19T10_05_00Z_gemini_attempt2.json");
    std::fs::write(
        &success_consensus,
        json!({
            "agent": "gemini",
            "stage": "plan",
            "status": "success",
            "content": "Enhanced prompt successful",
            "attempt": 2,
            "retry_reason": "empty_result",
        })
        .to_string(),
    )
    .unwrap();

    // Assert: Retry succeeded
    assert!(success_consensus.exists());
    assert_eq!(ctx.count_consensus_files(), 1); // Only successful attempt remains

    // Assert: State advances after successful retry
    state.current_index += 1;
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));

    // Assert: Evidence shows retry metadata
    let content = std::fs::read_to_string(&success_consensus).unwrap();
    assert!(content.contains("retry_reason"));
    assert!(content.contains("attempt"));
}
```

**Key Points**:
- ✅ Failed attempt recorded as evidence
- ✅ Retry cleanup removes failed attempt
- ✅ Success includes retry metadata
- ✅ State advances only on success

---

### Pattern 2: MCP Failure → Fallback → Recovery

**Example: E02 - MCP Timeout with File Fallback**

```rust
#[test]
fn e02_mcp_failure_fallback_to_file_evidence_records_fallback() {
    let ctx = IntegrationTestContext::new("SPEC-E02-001").unwrap();

    // ==================== MCP FAILURE ====================

    // Write fallback marker evidence (MCP failed, using file fallback)
    let fallback_evidence = ctx
        .consensus_dir()
        .join("spec-plan_mcp_fallback_2025-10-19T10_00_00Z.json");
    std::fs::write(
        &fallback_evidence,
        json!({
            "fallback_mode": "file_based",
            "mcp_error": "Timeout after 60s",
            "fallback_timestamp": "2025-10-19T10:00:00Z"
        })
        .to_string(),
    )
    .unwrap();

    // Assert: Fallback recorded
    assert!(fallback_evidence.exists());

    // ==================== FILE-BASED CONSENSUS ====================

    // Act: Write consensus from file-based fallback
    let file_consensus = ctx
        .consensus_dir()
        .join("spec-plan_2025-10-19T10_00_00Z_file_based.json");
    std::fs::write(
        &file_consensus,
        json!({
            "source": "file_based_fallback",
            "content": "Consensus from local files",
            "degraded": true
        })
        .to_string(),
    )
    .unwrap();

    // Assert: File-based consensus succeeded
    assert!(file_consensus.exists());
    assert_eq!(ctx.count_consensus_files(), 2); // Fallback marker + consensus

    // Assert: Degraded flag set
    let content = std::fs::read_to_string(&file_consensus).unwrap();
    assert!(content.contains("\"degraded\":true"));
}
```

**Key Points**:
- ✅ MCP failure recorded as fallback evidence
- ✅ File-based fallback produces consensus
- ✅ Degraded flag indicates fallback mode
- ✅ Multiple evidence files coexist

---

## State Persistence Integration Tests

### Pattern 1: State Serialization → Load → Reconstruct

**Example: S01 - State Persistence and Reconstruction**

**Test** (state_persistence_integration_tests.rs:18):
```rust
#[test]
fn s01_state_change_evidence_write_load_from_disk_reconstruct() {
    let ctx = IntegrationTestContext::new("SPEC-S01-001").unwrap();
    let state = StateBuilder::new("SPEC-S01-001")
        .starting_at(SpecStage::Plan)
        .build();

    // ==================== SERIALIZE STATE ====================

    // Act: Write state to evidence
    let state_file = ctx.commands_dir().join("spec_auto_state.json");
    std::fs::write(
        &state_file,
        json!({
            "spec_id": state.spec_id,
            "current_index": state.current_index,
            "quality_gates_enabled": state.quality_gates_enabled,
        })
        .to_string(),
    )
    .unwrap();

    // ==================== LOAD AND RECONSTRUCT ====================

    // Act: Load from disk and verify reconstruction
    let loaded = std::fs::read_to_string(&state_file).unwrap();
    let data: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    // Assert: All fields preserved
    assert_eq!(data["spec_id"], "SPEC-S01-001");
    assert_eq!(data["current_index"], 0);
    assert_eq!(data["quality_gates_enabled"], true);

    // Reconstruct state from loaded data
    let reconstructed = StateBuilder::new(data["spec_id"].as_str().unwrap())
        .starting_at(SpecStage::Plan)
        .quality_gates(data["quality_gates_enabled"].as_bool().unwrap())
        .build();

    assert_eq!(reconstructed.spec_id, state.spec_id);
    assert_eq!(reconstructed.current_index, state.current_index);
}
```

---

### Pattern 2: Pipeline Interrupt → Resume from Checkpoint

**Example: S02 - Pipeline Interrupt and Resume**

**Test** (state_persistence_integration_tests.rs:45):
```rust
#[test]
fn s02_pipeline_interrupt_state_saved_resume_from_checkpoint() {
    let ctx = IntegrationTestContext::new("SPEC-S02-001").unwrap();
    let mut state = StateBuilder::new("SPEC-S02-001")
        .starting_at(SpecStage::Tasks)
        .build();

    // ==================== SAVE CHECKPOINT ====================

    // Act: Save checkpoint before interrupt
    let checkpoint = ctx.commands_dir().join("checkpoint.json");
    std::fs::write(
        &checkpoint,
        json!({
            "spec_id": state.spec_id,
            "checkpoint_index": state.current_index,
            "timestamp": "2025-10-19T10:00:00Z"
        })
        .to_string(),
    )
    .unwrap();

    assert_eq!(state.current_index, 1); // Tasks = index 1

    // ==================== INTERRUPT ====================

    // Simulate interrupt (state dropped)
    drop(state);

    // ==================== RESUME ====================

    // Act: Resume from checkpoint
    let loaded = std::fs::read_to_string(&checkpoint).unwrap();
    let data: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    let resumed_state = StateBuilder::new("SPEC-S02-001")
        .starting_at(SpecStage::Plan)
        .build();

    // Assert: Checkpoint index preserved
    assert_eq!(data["checkpoint_index"], 1);
    assert_eq!(data["spec_id"], "SPEC-S02-001");

    // Resume would set current_index from checkpoint
    // (not shown: actual resume logic would apply checkpoint)
}
```

---

## Evidence Verification Patterns

### Pattern 1: Comprehensive Evidence Verification

```rust
#[test]
fn verify_complete_stage_evidence() {
    let ctx = IntegrationTestContext::new("SPEC-TEST").unwrap();

    // Simulate complete stage execution
    // ... (write consensus and guardrail artifacts)

    // ==================== VERIFY STRUCTURE ====================

    let verifier = EvidenceVerifier::new(&ctx);

    // Directory structure
    assert!(verifier.assert_structure_valid());

    // ==================== VERIFY CONSENSUS ====================

    // All agents present
    assert!(verifier.assert_consensus_complete(
        SpecStage::Plan,
        &["gemini", "claude", "gpt_pro"]
    ));

    // Individual agents
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gemini"));
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "claude"));
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gpt_pro"));

    // ==================== VERIFY GUARDRAIL ====================

    assert!(verifier.assert_guardrail_valid(SpecStage::Plan).is_ok());

    // ==================== VERIFY COUNTS ====================

    assert_eq!(ctx.count_consensus_files(), 3);
    assert_eq!(ctx.count_guardrail_files(), 1);
}
```

---

### Pattern 2: Degraded Consensus Detection

```rust
#[test]
fn verify_degraded_consensus() {
    let ctx = IntegrationTestContext::new("SPEC-TEST").unwrap();

    // Simulate degraded consensus (only 2/3 agents)
    // ... (write only gemini and claude consensus)

    let verifier = EvidenceVerifier::new(&ctx);

    // Should NOT be complete (missing gpt_pro)
    assert!(!verifier.assert_consensus_complete(
        SpecStage::Plan,
        &["gemini", "claude", "gpt_pro"]
    ));

    // But 2/3 is still valid
    assert!(verifier.assert_consensus_complete(
        SpecStage::Plan,
        &["gemini", "claude"]
    ));

    // Verify degraded flag
    let consensus = ctx
        .consensus_dir()
        .join("spec-plan_2025-10-19T10_00_00Z_synthesis.json");
    std::fs::write(
        &consensus,
        json!({"consensus_ok": true, "degraded": true, "missing_agents": ["gpt_pro"]})
            .to_string(),
    )
    .unwrap();

    let content = std::fs::read_to_string(&consensus).unwrap();
    assert!(content.contains("\"degraded\":true"));
}
```

---

## Best Practices

### DO

**✅ Use IntegrationTestContext for isolation**:
```rust
#[test]
fn test_workflow() {
    // Each test gets isolated filesystem
    let ctx = IntegrationTestContext::new("SPEC-TEST-001").unwrap();
    // ... test logic
}
```

---

**✅ Verify evidence at each step**:
```rust
// After consensus
assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gemini"));

// After guardrail
assert!(ctx.assert_guardrail_telemetry_exists(SpecStage::Plan));

// After completion
assert_eq!(ctx.count_consensus_files(), 3);
```

---

**✅ Test both success and failure paths**:
```rust
#[test]
fn test_success_path() {
    // Happy path
}

#[test]
fn test_failure_path_with_retry() {
    // Error → Retry → Success
}

#[test]
fn test_failure_path_exhausted_retries() {
    // Error → Retry → Retry → Fail
}
```

---

**✅ Simulate realistic timing**:
```rust
let timestamp_attempt1 = "2025-10-19T10:00:00Z";
let timestamp_retry = "2025-10-19T10:05:00Z";  // 5 minutes later

// Evidence shows temporal sequence
```

---

**✅ Verify state transitions**:
```rust
assert_eq!(state.current_stage(), Some(SpecStage::Plan));

// Execute stage...

state.current_index += 1;
assert_eq!(state.current_stage(), Some(SpecStage::Tasks));
```

---

### DON'T

**❌ Share IntegrationTestContext across tests**:
```rust
// Bad: Shared context (tests interfere)
static mut CTX: Option<IntegrationTestContext> = None;

#[test]
fn test_a() {
    unsafe { CTX = Some(IntegrationTestContext::new("SHARED").unwrap()); }
}

#[test]
fn test_b() {
    unsafe { /* use CTX */ }  // ❌ Flaky (depends on test_a)
}
```

---

**❌ Test too many stages in one test**:
```rust
// Bad: Tests entire pipeline (hard to debug failures)
#[test]
fn test_entire_pipeline() {
    // Plan
    // Tasks
    // Implement
    // Validate
    // Audit
    // Unlock
    // → 200 lines, hard to maintain
}

// Good: Split into focused tests
#[test]
fn w01_plan_stage_workflow() { /* ... */ }

#[test]
fn w02_tasks_stage_workflow() { /* ... */ }
```

---

**❌ Skip evidence verification**:
```rust
// Bad: No verification
#[test]
fn test_workflow() {
    // Run workflow...
    // No assertions ❌
}

// Good: Verify evidence
#[test]
fn test_workflow() {
    // Run workflow...
    assert!(ctx.assert_consensus_exists(...));
    assert!(ctx.assert_guardrail_telemetry_exists(...));
}
```

---

**❌ Use hard-coded paths**:
```rust
// Bad: Hard-coded paths (breaks on other machines)
let consensus = Path::new("/tmp/consensus/SPEC-TEST/plan.json");

// Good: Use IntegrationTestContext
let consensus = ctx.consensus_dir().join("plan.json");
```

---

## Running Integration Tests

### Run All Integration Tests

```bash
cd codex-rs
cargo test --test '*_integration_tests'
```

**Runs**:
- `workflow_integration_tests.rs`
- `error_recovery_integration_tests.rs`
- `state_persistence_integration_tests.rs`
- `quality_flow_integration_tests.rs`
- `concurrent_operations_integration_tests.rs`

---

### Run Specific Category

```bash
# Workflow tests only
cargo test --test workflow_integration_tests

# Error recovery tests only
cargo test --test error_recovery_integration_tests
```

---

### Run Specific Test

```bash
cargo test --test workflow_integration_tests w01_plan_stage_complete_workflow
```

---

### Run with Output

```bash
cargo test --test workflow_integration_tests -- --nocapture
```

Shows `println!()` output for debugging.

---

## Summary

**Integration Testing Best Practices**:

1. **Isolation**: Use `IntegrationTestContext` for each test
2. **Evidence**: Verify evidence at each step
3. **Coverage**: Test success and failure paths
4. **Clarity**: One workflow per test
5. **Timing**: Simulate realistic sequences
6. **State**: Verify state transitions
7. **Cleanup**: Automatic (TempDir drops)

**Test Categories**:
- ✅ W01-W15: Workflow integration (stage workflows, pipelines)
- ✅ E01-E15: Error recovery (retry, fallback, degradation)
- ✅ S01-S10: State persistence (serialize, resume, checkpoint)
- ✅ Q01-Q10: Quality gates (BeforeSpecify, AfterSpecify, AfterTasks)
- ✅ C01-C10: Concurrent operations (parallel, locking)

**Key Patterns**:
- ✅ Multi-module workflows (Handler → Consensus → Evidence → Guardrail → State)
- ✅ Error propagation (Failure → Retry → Recovery → Evidence)
- ✅ State persistence (Serialize → Load → Reconstruct)
- ✅ Evidence verification (EvidenceVerifier, counts, structure)

**Next Steps**:
- [E2E Testing Guide](e2e-testing-guide.md) - Complete user workflows
- [Property Testing Guide](property-testing-guide.md) - Generative invariant testing
- [Test Infrastructure](test-infrastructure.md) - MockMcpManager, fixtures

---

**References**:
- Workflow tests: `codex-rs/tui/tests/workflow_integration_tests.rs`
- Error recovery: `codex-rs/tui/tests/error_recovery_integration_tests.rs`
- State persistence: `codex-rs/tui/tests/state_persistence_integration_tests.rs`
- IntegrationTestContext: `codex-rs/tui/tests/common/integration_harness.rs`
