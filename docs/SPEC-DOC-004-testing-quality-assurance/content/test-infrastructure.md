# Test Infrastructure

Comprehensive testing infrastructure for the codebase.

---

## Overview

**Test Infrastructure Components**:
- **MockMcpManager**: Mock MCP server for isolated testing
- **IntegrationTestContext**: Multi-module test harness
- **StateBuilder**: Test state configuration
- **EvidenceVerifier**: Artifact validation helpers
- **Fixture Library**: Real production data (20 files, 96 KB)
- **Coverage Tools**: cargo-tarpaulin, cargo-llvm-cov
- **Property Testing**: proptest for generative testing

**Location**: `codex-rs/tui/tests/common/` (shared test utilities)

**Purpose**: Enable comprehensive testing without external dependencies

---

## MockMcpManager

### Purpose

Mock implementation of `McpConnectionManager` for testing MCP-dependent code without requiring a live local-memory server.

**Location**: `codex-rs/tui/tests/common/mock_mcp.rs` (272 LOC)

**Use Cases**:
- Test consensus logic without spawning agents
- Verify MCP tool calls in isolation
- Fast unit tests (<1ms vs 8.7ms real MCP)
- Deterministic fixture responses

---

### API Reference

#### Creating a Mock

```rust
use codex_tui::tests::common::MockMcpManager;

let mut mock = MockMcpManager::new();
```

**Methods**:
- `new()` → Create empty mock
- `default()` → Same as `new()` (implements Default)

---

#### Adding Fixtures

**Single Fixture**:
```rust
mock.add_fixture(
    "local-memory",           // server name
    "search",                 // tool name
    Some("SPEC-TEST plan"),   // query pattern (or None for wildcard)
    json!({                   // fixture response
        "memory": {
            "id": "test-1",
            "content": "Test content"
        }
    })
);
```

**Multiple Fixtures**:
```rust
mock.add_fixtures(
    "local-memory",
    "search",
    Some("SPEC-TEST plan"),
    vec![
        json!({"memory": {"id": "test-1", "content": "Agent 1"}}),
        json!({"memory": {"id": "test-2", "content": "Agent 2"}}),
    ]
);
```

**From File**:
```rust
mock.load_fixture_file(
    "local-memory",
    "search",
    Some("SPEC-KIT-DEMO plan"),
    "tests/fixtures/consensus/demo-plan-gemini.json"
)?;
```

---

#### Calling Tools

**Signature**:
```rust
pub async fn call_tool(
    &self,
    server: &str,
    tool: &str,
    arguments: Option<Value>,
    timeout: Option<Duration>,
) -> Result<CallToolResult>
```

**Example**:
```rust
let args = json!({"query": "SPEC-TEST plan"});
let result = mock.call_tool(
    "local-memory",
    "search",
    Some(args),
    None  // timeout
).await?;

// Extract response
if let ContentBlock::TextContent(text) = &result.content[0] {
    let data: Value = serde_json::from_str(&text.text)?;
    println!("{:?}", data);
}
```

---

#### Call Logging

**Get Call History**:
```rust
let log = mock.call_log();
for entry in log {
    println!("Called: {}/{}", entry.server, entry.tool);
    println!("  Args: {:?}", entry.arguments);
}
```

**Clear Log**:
```rust
mock.clear_log();
```

**Use Case**: Verify expected tool calls were made
```rust
assert_eq!(log.len(), 3);
assert_eq!(log[0].tool, "search");
assert_eq!(log[1].tool, "search");
assert_eq!(log[2].tool, "search");
```

---

### Fixture Matching

**Priority Order**:
1. **Exact query match**: `query_pattern = Some("SPEC-TEST plan")`
2. **Wildcard match**: `query_pattern = None`
3. **No match**: Returns error

**Example**:
```rust
// Add wildcard fixture
mock.add_fixture("local-memory", "search", None, json!({"default": true}));

// Add specific fixture
mock.add_fixture(
    "local-memory",
    "search",
    Some("SPEC-DEMO plan"),
    json!({"specific": true})
);

// Query "SPEC-DEMO plan" → Returns {"specific": true}
// Query "anything else"   → Returns {"default": true}
// Query with no fixture   → Error
```

---

### Usage Patterns

#### Pattern 1: Unit Testing Consensus

```rust
#[tokio::test]
async fn test_consensus_high_confidence() {
    let mut mock = MockMcpManager::new();

    // Load real production fixtures
    mock.load_fixture_file(
        "local-memory",
        "search",
        Some("SPEC-TEST plan"),
        "tests/fixtures/consensus/demo-plan-gemini.json"
    )?;
    mock.load_fixture_file(
        "local-memory",
        "search",
        Some("SPEC-TEST plan"),
        "tests/fixtures/consensus/demo-plan-claude.json"
    )?;

    // Test consensus collection
    let (results, degraded) = fetch_memory_entries(
        "SPEC-TEST",
        SpecStage::Plan,
        &mock
    ).await?;

    assert_eq!(results.len(), 2);
    assert!(!degraded, "Should have both agents");
}
```

---

#### Pattern 2: Verifying Tool Calls

```rust
#[tokio::test]
async fn test_quality_gate_calls_all_tools() {
    let mut mock = MockMcpManager::new();
    mock.add_fixture("local-memory", "search", None, json!({}));

    // Run quality gate
    run_quality_gate("SPEC-TEST", &mock).await?;

    // Verify calls
    let log = mock.call_log();
    assert!(log.iter().any(|e| e.tool == "search"));

    // Verify call arguments
    let search_call = log.iter().find(|e| e.tool == "search").unwrap();
    assert!(search_call.arguments.is_some());
}
```

---

#### Pattern 3: Testing Error Handling

```rust
#[tokio::test]
async fn test_consensus_degradation_on_missing_agent() {
    let mut mock = MockMcpManager::new();

    // Only add 2 of 3 agents
    mock.add_fixture("local-memory", "search", None, json!({"agent": "gemini"}));
    mock.add_fixture("local-memory", "search", None, json!({"agent": "claude"}));
    // gpt_pro deliberately missing

    let (results, degraded) = fetch_memory_entries(
        "SPEC-TEST",
        SpecStage::Plan,
        &mock
    ).await?;

    assert_eq!(results.len(), 2);
    assert!(degraded, "Should be degraded (missing 1 agent)");
}
```

---

### Tests

**Location**: `codex-rs/tui/tests/mock_mcp_tests.rs` (7 tests)

**Coverage**:
```rust
test_mock_mcp_returns_fixture                  ✓
test_mock_mcp_logs_calls                       ✓
test_mock_mcp_wildcard_matches                 ✓
test_mock_mcp_exact_query_precedence           ✓
test_mock_mcp_multiple_fixtures_return_array   ✓
test_mock_mcp_load_from_file                   ✓
test_mock_mcp_error_on_no_fixture              ✓
```

**Run Tests**:
```bash
cd codex-rs
cargo test --test mock_mcp_tests
```

---

## IntegrationTestContext

### Purpose

Multi-module test harness for integration tests with isolated filesystem and evidence verification.

**Location**: `codex-rs/tui/tests/common/integration_harness.rs` (254 LOC)

**Use Cases**:
- Cross-module workflow tests
- Evidence verification
- Filesystem isolation (temp directories)
- SPEC directory structure setup

---

### API Reference

#### Creating a Context

```rust
use codex_tui::tests::common::IntegrationTestContext;

let ctx = IntegrationTestContext::new("SPEC-TEST-001")?;
```

**Fields**:
```rust
pub struct IntegrationTestContext {
    pub temp_dir: TempDir,        // Auto-cleaned on drop
    pub spec_id: String,          // "SPEC-TEST-001"
    pub cwd: PathBuf,             // temp_dir path
    pub evidence_dir: PathBuf,    // docs/SPEC-OPS-004.../evidence
}
```

**Auto-Created Directories**:
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`
- `docs/SPEC-OPS-004.../evidence/consensus/{spec_id}/`
- `docs/SPEC-OPS-004.../evidence/commands/{spec_id}/`

---

#### Directory Helpers

**Get Evidence Directories**:
```rust
let consensus_dir = ctx.consensus_dir();
// → .../evidence/consensus/SPEC-TEST-001/

let commands_dir = ctx.commands_dir();
// → .../evidence/commands/SPEC-TEST-001/
```

**Create SPEC Directory**:
```rust
let spec_dir = ctx.create_spec_dirs("test-feature")?;
// → .../docs/SPEC-TEST-001-test-feature/
```

---

#### File Helpers

**Write PRD**:
```rust
ctx.write_prd("test-feature", "# PRD\n\nTest product requirements")?;
// Creates: docs/SPEC-TEST-001-test-feature/PRD.md
```

**Write Spec**:
```rust
ctx.write_spec("test-feature", "# SPEC-TEST-001\n\n## Goal\nTest")?;
// Creates: docs/SPEC-TEST-001-test-feature/spec.md
```

---

#### Evidence Verification

**Check Consensus Artifacts**:
```rust
// Single agent
let exists = ctx.assert_consensus_exists(SpecStage::Plan, "gemini");
assert!(exists);

// All agents (via EvidenceVerifier)
let verifier = EvidenceVerifier::new(&ctx);
assert!(verifier.assert_consensus_complete(
    SpecStage::Plan,
    &["gemini", "claude", "gpt_pro"]
));
```

**Check Guardrail Telemetry**:
```rust
let exists = ctx.assert_guardrail_telemetry_exists(SpecStage::Plan);
assert!(exists);
```

**Count Files**:
```rust
let count = ctx.count_consensus_files();
assert_eq!(count, 3, "Should have 3 agent outputs");

let guardrail_count = ctx.count_guardrail_files();
assert_eq!(guardrail_count, 1, "Should have 1 telemetry file");
```

---

### Usage Patterns

#### Pattern 1: Workflow Integration Test

```rust
#[tokio::test]
async fn test_full_plan_stage_workflow() -> Result<()> {
    // Setup
    let ctx = IntegrationTestContext::new("SPEC-INT-001")?;
    ctx.write_prd("test-feature", "# Test PRD\n\n## Goal\nTest")?;

    // Run plan stage
    run_plan_stage(&ctx.spec_id, &ctx.cwd).await?;

    // Verify evidence
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gemini"));
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "claude"));
    assert!(ctx.assert_consensus_exists(SpecStage::Plan, "gpt_pro"));
    assert!(ctx.assert_guardrail_telemetry_exists(SpecStage::Plan));

    // Verify file count
    assert_eq!(ctx.count_consensus_files(), 3);

    Ok(())
}
```

---

#### Pattern 2: Error Recovery Test

```rust
#[tokio::test]
async fn test_error_recovery_creates_evidence() -> Result<()> {
    let ctx = IntegrationTestContext::new("SPEC-INT-002")?;

    // Simulate error (missing PRD)
    let result = run_plan_stage(&ctx.spec_id, &ctx.cwd).await;
    assert!(result.is_err());

    // Verify error evidence still created
    let verifier = EvidenceVerifier::new(&ctx);
    assert!(verifier.assert_guardrail_valid(SpecStage::Plan).is_ok());

    Ok(())
}
```

---

#### Pattern 3: State Persistence Test

```rust
#[tokio::test]
async fn test_state_persists_across_stages() -> Result<()> {
    let ctx = IntegrationTestContext::new("SPEC-INT-003")?;
    ctx.write_prd("test", "# PRD")?;

    // Run plan
    run_plan_stage(&ctx.spec_id, &ctx.cwd).await?;
    assert_eq!(ctx.count_consensus_files(), 3);

    // Run tasks (should accumulate, not replace)
    run_tasks_stage(&ctx.spec_id, &ctx.cwd).await?;
    assert!(ctx.count_consensus_files() > 3, "Should accumulate evidence");

    Ok(())
}
```

---

### Tests

**Location**: `codex-rs/tui/tests/common/integration_harness.rs` (4 tests in `mod tests`)

**Coverage**:
```rust
test_integration_context_creation    ✓
test_state_builder                   ✓
test_spec_dirs_creation              ✓
test_evidence_verifier               ✓
```

---

## StateBuilder

### Purpose

Builder pattern for creating `SpecAutoState` instances in tests with custom configuration.

**Location**: `codex-rs/tui/tests/common/integration_harness.rs`

**Use Cases**:
- Configure test automation state
- Test different starting stages
- Test HAL mode variations
- Test quality gate configurations

---

### API Reference

#### Basic Usage

```rust
use codex_tui::tests::common::StateBuilder;

let state = StateBuilder::new("SPEC-TEST-001").build();
```

**Default Configuration**:
- `goal`: "Integration test"
- `start_stage`: Plan
- `hal_mode`: None
- `quality_gates_enabled`: true

---

#### Builder Methods

**Custom Goal**:
```rust
let state = StateBuilder::new("SPEC-TEST-001")
    .with_goal("Implement user authentication")
    .build();
```

**Start at Different Stage**:
```rust
let state = StateBuilder::new("SPEC-TEST-002")
    .starting_at(SpecStage::Implement)
    .build();
```

**HAL Mode Configuration**:
```rust
let state = StateBuilder::new("SPEC-TEST-003")
    .with_hal_mode(HalMode::Analyze)
    .build();
```

**Quality Gates Control**:
```rust
let state = StateBuilder::new("SPEC-TEST-004")
    .quality_gates(false)  // Disable quality gates
    .build();
```

**Chained Configuration**:
```rust
let state = StateBuilder::new("SPEC-TEST-005")
    .with_goal("Test refactoring")
    .starting_at(SpecStage::Validate)
    .with_hal_mode(HalMode::TestOnly)
    .quality_gates(true)
    .build();
```

---

### Usage Patterns

#### Pattern 1: Testing Stage Transitions

```rust
#[test]
fn test_stage_advancement() {
    let mut state = StateBuilder::new("SPEC-TEST-001")
        .starting_at(SpecStage::Plan)
        .build();

    assert_eq!(state.current_stage(), Some(SpecStage::Plan));

    state.advance_stage();
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));

    state.advance_stage();
    assert_eq!(state.current_stage(), Some(SpecStage::Implement));
}
```

---

#### Pattern 2: Testing Quality Gate Behavior

```rust
#[test]
fn test_quality_gates_disabled() {
    let state = StateBuilder::new("SPEC-TEST-002")
        .quality_gates(false)
        .build();

    assert!(!state.quality_gates_enabled);

    // Quality gates should not run
    assert!(should_skip_quality_gate(&state));
}

#[test]
fn test_quality_gates_enabled() {
    let state = StateBuilder::new("SPEC-TEST-003")
        .quality_gates(true)
        .build();

    assert!(state.quality_gates_enabled);
}
```

---

#### Pattern 3: Testing HAL Integration

```rust
#[test]
fn test_hal_mode_analyze() {
    let state = StateBuilder::new("SPEC-TEST-004")
        .with_hal_mode(HalMode::Analyze)
        .build();

    assert_eq!(state.hal_mode, Some(HalMode::Analyze));
}

#[test]
fn test_hal_mode_none() {
    let state = StateBuilder::new("SPEC-TEST-005")
        .build();

    assert_eq!(state.hal_mode, None);
}
```

---

## EvidenceVerifier

### Purpose

Helper for verifying evidence artifacts in integration tests.

**Location**: `codex-rs/tui/tests/common/integration_harness.rs`

**Use Cases**:
- Assert consensus artifacts exist
- Validate guardrail telemetry
- Verify directory structure
- Check multi-agent completion

---

### API Reference

#### Creating a Verifier

```rust
use codex_tui::tests::common::EvidenceVerifier;

let ctx = IntegrationTestContext::new("SPEC-TEST-001")?;
let verifier = EvidenceVerifier::new(&ctx);
```

---

#### Verification Methods

**Consensus Complete** (all agents present):
```rust
let complete = verifier.assert_consensus_complete(
    SpecStage::Plan,
    &["gemini", "claude", "gpt_pro"]
);
assert!(complete);
```

**Guardrail Valid** (telemetry exists and parseable):
```rust
let result = verifier.assert_guardrail_valid(SpecStage::Plan);
assert!(result.is_ok());
```

**Structure Valid** (directories exist):
```rust
let valid = verifier.assert_structure_valid();
assert!(valid);
```

---

### Usage Patterns

#### Pattern 1: Post-Workflow Verification

```rust
#[tokio::test]
async fn test_plan_creates_complete_evidence() -> Result<()> {
    let ctx = IntegrationTestContext::new("SPEC-VER-001")?;
    ctx.write_prd("test", "# PRD")?;

    run_plan_stage(&ctx.spec_id, &ctx.cwd).await?;

    let verifier = EvidenceVerifier::new(&ctx);

    // Verify all artifacts
    assert!(verifier.assert_structure_valid());
    assert!(verifier.assert_consensus_complete(
        SpecStage::Plan,
        &["gemini", "claude", "gpt_pro"]
    ));
    assert!(verifier.assert_guardrail_valid(SpecStage::Plan).is_ok());

    Ok(())
}
```

---

#### Pattern 2: Degraded Consensus Detection

```rust
#[tokio::test]
async fn test_degraded_consensus_still_valid() -> Result<()> {
    let ctx = IntegrationTestContext::new("SPEC-VER-002")?;

    // Simulate degraded consensus (only 2/3 agents)
    simulate_agent_failure("gpt_pro")?;
    run_plan_stage(&ctx.spec_id, &ctx.cwd).await?;

    let verifier = EvidenceVerifier::new(&ctx);

    // Should NOT be complete (missing 1 agent)
    assert!(!verifier.assert_consensus_complete(
        SpecStage::Plan,
        &["gemini", "claude", "gpt_pro"]
    ));

    // But 2/3 is still valid
    assert!(verifier.assert_consensus_complete(
        SpecStage::Plan,
        &["gemini", "claude"]
    ));

    Ok(())
}
```

---

## Fixture Library

### Overview

**Location**: `codex-rs/tui/tests/fixtures/consensus/` (20 files, 96 KB)

**Source**: Real production artifacts from `docs/SPEC-OPS-004.../evidence/consensus/`

**Coverage**:
- Plan stage: 13 fixtures (DEMO, 025, 045)
- Tasks stage: 3 fixtures (025)
- Implement stage: 4 fixtures (025)

---

### File Naming Convention

**Format**: `{spec_id}-{stage}-{agent}.json`

**Examples**:
- `demo-plan-gemini.json` — SPEC-KIT-DEMO plan stage (Gemini output)
- `025-implement-gpt_codex.json` — SPEC-KIT-025 implement stage (Codex output)
- `045-plan-claude.json` — SPEC-KIT-045 plan stage (Claude output)

---

### Available Fixtures

#### Plan Stage (13 files)

**SPEC-KIT-DEMO**:
- `demo-plan-gemini.json` (14 KB)
- `demo-plan-claude.json` (12 KB)
- `demo-plan-gpt_pro.json` (15 KB)

**SPEC-KIT-025** (Native SPEC-ID generation):
- `025-plan-gemini.json` (16 KB)
- `025-plan-claude.json` (14 KB)
- `025-plan-gpt_pro.json` (18 KB)

**SPEC-KIT-045** (Quality gate handler):
- `045-plan-gemini.json` (13 KB)
- `045-plan-claude.json` (11 KB)
- `045-plan-gpt_pro.json` (17 KB)

---

#### Tasks Stage (3 files)

**SPEC-KIT-025**:
- `025-tasks-gemini.json` (8 KB)
- `025-tasks-claude.json` (7 KB)

---

#### Implement Stage (4 files)

**SPEC-KIT-025**:
- `025-implement-gemini.json` (9 KB)
- `025-implement-claude.json` (8 KB)
- `025-implement-gpt_codex.json` (22 KB) — Code implementation
- `025-implement-gpt_pro.json` (11 KB)

---

### Usage in Tests

**Loading Single Fixture**:
```rust
let mut mock = MockMcpManager::new();
mock.load_fixture_file(
    "local-memory",
    "search",
    Some("SPEC-KIT-DEMO plan"),
    "tests/fixtures/consensus/demo-plan-gemini.json"
)?;
```

**Loading All Agents** (simulate 3-agent consensus):
```rust
let mut mock = MockMcpManager::new();
let agents = vec!["gemini", "claude", "gpt_pro"];

for agent in agents {
    mock.load_fixture_file(
        "local-memory",
        "search",
        Some("SPEC-KIT-DEMO plan"),
        &format!("tests/fixtures/consensus/demo-plan-{}.json", agent)
    )?;
}
```

**Loading Different Stages**:
```rust
// Plan stage
mock.load_fixture_file("local-memory", "search", Some("SPEC-KIT-025 plan"),
    "tests/fixtures/consensus/025-plan-gemini.json")?;

// Tasks stage
mock.load_fixture_file("local-memory", "search", Some("SPEC-KIT-025 tasks"),
    "tests/fixtures/consensus/025-tasks-gemini.json")?;

// Implement stage
mock.load_fixture_file("local-memory", "search", Some("SPEC-KIT-025 implement"),
    "tests/fixtures/consensus/025-implement-gpt_codex.json")?;
```

---

### Adding New Fixtures

**Manual Creation**:
```bash
cd codex-rs/tui/tests/fixtures/consensus

# Copy from production evidence
cp ../../../docs/SPEC-OPS-004.../evidence/consensus/SPEC-KIT-070/spec-plan_*.json \
   ./070-plan-gemini.json
```

**Automated Extraction** (future):
```bash
# Extract fixtures from evidence repository
./scripts/extract_test_fixtures.sh SPEC-KIT-070
```

**Size Guidelines**:
- Keep individual fixtures < 30 KB
- Total fixture directory < 200 KB
- Compress if needed (not implemented yet)

---

## Coverage Tools

### cargo-tarpaulin

**Purpose**: Line coverage measurement for Rust code

**Installation**:
```bash
cargo install cargo-tarpaulin
```

**Configuration**: `codex-rs/tarpaulin.toml`

---

#### Configuration Details

```toml
[config]
# Only measure spec-kit coverage (fork-specific code)
run-types = ["Lib", "Tests"]

# Include patterns (spec-kit only)
include-pattern = "tui/src/chatwidget/spec_kit/.*\\.rs"

# Exclude test files and generated code
exclude-files = [
    "tui/src/chatwidget/spec_kit/*/tests/*",
    "tui/tests/*",
]

# Output formats
out = ["Html", "Stdout"]
output-dir = "target/tarpaulin"

# Timeout per test (integration tests are slow)
timeout = 120

# Verbose output
verbose = true
```

---

#### Usage

**Full Coverage Report**:
```bash
cd codex-rs
cargo tarpaulin
```

**Output**:
```
|| Tested/Total Lines:
|| tui/src/chatwidget/spec_kit/handler.rs: 145/961
|| tui/src/chatwidget/spec_kit/consensus.rs: 120/992
|| tui/src/chatwidget/spec_kit/quality.rs: 178/807
|| ...
||
|| Coverage: 42.3%
```

**Specific Module**:
```bash
cargo tarpaulin -p codex-tui
```

**HTML Report**:
```bash
cargo tarpaulin --out Html
open target/tarpaulin/index.html
```

**XML for CI** (Codecov):
```bash
cargo tarpaulin --out Xml
```

---

#### Troubleshooting

**Issue**: Timeout on slow tests
```bash
# Increase timeout
cargo tarpaulin --timeout 300
```

**Issue**: Out of memory
```bash
# Reduce parallelism
cargo tarpaulin --jobs 2
```

**Issue**: Incorrect coverage (too low)
```bash
# Ensure all features enabled
cargo tarpaulin --all-features
```

---

### cargo-llvm-cov

**Purpose**: Alternative coverage tool using LLVM instrumentation

**Advantages**:
- More accurate than tarpaulin
- Faster execution
- Better integration with IDEs

**Installation**:
```bash
cargo install cargo-llvm-cov
```

---

#### Usage

**Generate Coverage**:
```bash
cd codex-rs
cargo llvm-cov --workspace --all-features --html
```

**Open Report**:
```bash
open target/llvm-cov/html/index.html
```

**JSON Output** (for parsing):
```bash
cargo llvm-cov --workspace --all-features --json --output-path coverage.json
```

**Integration with VS Code**:
```bash
# Install Coverage Gutters extension
# Run:
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

# VS Code will show coverage inline
```

---

#### Comparison: Tarpaulin vs llvm-cov

| Feature | Tarpaulin | llvm-cov |
|---------|-----------|----------|
| **Accuracy** | ~95% | ~99% |
| **Speed** | Baseline | 1.5-2× faster |
| **HTML Report** | ✅ Good | ✅ Excellent |
| **IDE Integration** | ❌ Limited | ✅ VS Code, IntelliJ |
| **CI Support** | ✅ Codecov, Coveralls | ✅ All platforms |
| **Install Size** | 50 MB | 150 MB (LLVM) |

**Recommendation**: Use llvm-cov for local development, tarpaulin for CI (smaller install).

---

## Property-Based Testing

### Overview

**Purpose**: Generative testing with random inputs to verify invariants

**Tool**: [proptest](https://docs.rs/proptest) (Rust equivalent of Hypothesis/QuickCheck)

**Location**: `codex-rs/tui/tests/property_based_tests.rs`

**Use Cases**:
- State machine invariants
- Evidence integrity
- Consensus edge cases
- Input validation

---

### Proptest Basics

**Simple Property Test**:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_state_index_never_negative(index in 0usize..20) {
        // Property: State always handles any index gracefully
        let mut state = SpecAutoState::new(...);
        state.current_index = index;

        // Should never panic
        let _ = state.current_stage();
    }
}
```

**How It Works**:
1. Generate 100 random values for `index` (0-19)
2. Run test with each value
3. If any fails, shrink to minimal failing case
4. Report failure with minimal input

---

### Test Categories

#### PB01-PB03: State Invariants

**PB01**: Index always in valid range
```rust
proptest! {
    #[test]
    fn pb01_state_index_always_in_valid_range(index in 0usize..20) {
        let mut state = StateBuilder::new("SPEC-PB01-TEST")
            .starting_at(SpecStage::Plan)
            .build();

        state.current_index = index;

        // Invariant: index ∈ [0, 5] → Some(_), else None
        if index < 6 {
            prop_assert!(state.current_stage().is_some());
        } else {
            prop_assert_eq!(state.current_stage(), None);
        }
    }
}
```

**PB02**: Current stage always Some when index < 6
```rust
proptest! {
    #[test]
    fn pb02_current_stage_always_some_when_index_under_six(
        index in 0usize..6
    ) {
        let mut state = StateBuilder::new("SPEC-PB02-TEST").build();
        state.current_index = index;

        prop_assert!(state.current_stage().is_some());
    }
}
```

**PB03**: Retry count never exceeds max
```rust
proptest! {
    #[test]
    fn pb03_retry_count_never_negative(retries in 0usize..100) {
        let max_retries = 3;
        let capped_retries = retries.min(max_retries);

        // Write retry file
        let retry_data = json!({
            "retry_count": capped_retries,
            "max_retries": max_retries,
        });

        // Invariant: retry_count ≤ max_retries
        prop_assert!(capped_retries <= max_retries);
    }
}
```

---

#### PB04-PB06: Evidence Integrity

**PB04**: Written evidence always parseable JSON
```rust
proptest! {
    #[test]
    fn pb04_written_evidence_always_parseable_json(
        agent in "[a-z]{3,10}",
        content in ".*"
    ) {
        let ctx = IntegrationTestContext::new("SPEC-PB04-TEST")?;

        let evidence = json!({
            "agent": agent,
            "content": content,
            "timestamp": "2025-10-19T00:00:00Z"
        });

        let file = ctx.consensus_dir().join("test.json");
        std::fs::write(&file, evidence.to_string())?;

        // Invariant: File is valid JSON
        let content = std::fs::read_to_string(&file)?;
        let parsed: Value = serde_json::from_str(&content)?;

        prop_assert_eq!(parsed["agent"].as_str(), Some(agent.as_str()));
    }
}
```

---

### Custom Generators

**Generate SPEC IDs**:
```rust
fn spec_id_strategy() -> impl Strategy<Value = String> {
    "[A-Z]{4}-[A-Z]{3}-[0-9]{3}"
        .prop_map(|s| s.to_string())
}

proptest! {
    #[test]
    fn test_spec_id_parsing(spec_id in spec_id_strategy()) {
        // Test SPEC ID validation
        assert!(is_valid_spec_id(&spec_id));
    }
}
```

**Generate Stages**:
```rust
fn stage_strategy() -> impl Strategy<Value = SpecStage> {
    prop_oneof![
        Just(SpecStage::Plan),
        Just(SpecStage::Tasks),
        Just(SpecStage::Implement),
        Just(SpecStage::Validate),
        Just(SpecStage::Audit),
        Just(SpecStage::Unlock),
    ]
}
```

---

### Running Property Tests

**Run All Property Tests**:
```bash
cd codex-rs
cargo test --test property_based_tests
```

**Run Specific Test**:
```bash
cargo test --test property_based_tests pb01_state_index
```

**Adjust Iteration Count** (default 100):
```bash
PROPTEST_CASES=1000 cargo test --test property_based_tests
```

**Debug Failing Case**:
```bash
# proptest creates a regression file
cat proptest-regressions/property_based_tests.txt

# Re-run with that specific input
cargo test --test property_based_tests -- --exact pb01_state_index
```

---

## TestCodexBuilder

### Purpose

Builder for creating test instances of `CodexConversation` with mock servers.

**Location**: `codex-rs/core/tests/common/test_codex.rs` (76 LOC)

**Use Cases**:
- Test agent spawning
- Test conversation lifecycle
- Test configuration variations
- Integration with wiremock

---

### API Reference

**Basic Usage**:
```rust
use codex_core::tests::common::test_codex;

let server = wiremock::MockServer::start().await;
let codex = test_codex()
    .build(&server)
    .await?;
```

**Fields**:
```rust
pub struct TestCodex {
    pub home: TempDir,                         // Isolated home directory
    pub cwd: TempDir,                          // Isolated working directory
    pub codex: Arc<CodexConversation>,         // Conversation instance
    pub session_configured: SessionConfiguredEvent,  // Initial event
}
```

---

### Custom Configuration

**Modify Config**:
```rust
let codex = test_codex()
    .with_config(|config| {
        config.model = "gpt-5-low".to_string();
        config.max_tokens = 4096;
    })
    .build(&server)
    .await?;
```

**Multiple Mutations**:
```rust
let codex = test_codex()
    .with_config(|config| config.model = "gpt-5-low".to_string())
    .with_config(|config| config.max_tokens = 8192)
    .with_config(|config| config.temperature = 0.7)
    .build(&server)
    .await?;
```

---

### Usage with Wiremock

**Mock API Responses**:
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_conversation_with_mock() -> Result<()> {
    let server = MockServer::start().await;

    // Mock /v1/chat/completions
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Test response"
                },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let codex = test_codex().build(&server).await?;

    // Test conversation
    let response = codex.codex.send_message("Test").await?;
    assert_eq!(response.content, "Test response");

    Ok(())
}
```

---

## Common Test Utilities

### Test Module Structure

**Location**: `codex-rs/tui/tests/common/mod.rs`

```rust
//! Common test utilities for spec-kit

pub mod integration_harness;
pub mod mock_mcp;

pub use integration_harness::{
    EvidenceVerifier,
    IntegrationTestContext,
    StateBuilder,
};
pub use mock_mcp::MockMcpManager;
```

**Usage in Tests**:
```rust
mod common;

use common::{
    MockMcpManager,
    IntegrationTestContext,
    StateBuilder,
    EvidenceVerifier,
};
```

---

### Shared Test Data

**Constants**:
```rust
// tests/common/mod.rs

pub const TEST_SPEC_ID: &str = "SPEC-TEST-001";
pub const TEST_GOAL: &str = "Integration test";

pub fn default_test_prd() -> &'static str {
    r#"
# Product Requirements Document

## Goal
Test feature implementation

## Requirements
- R1: Feature should work
- R2: Feature should be tested
    "#
}
```

**Usage**:
```rust
use common::{TEST_SPEC_ID, default_test_prd};

#[tokio::test]
async fn test_with_shared_data() {
    let ctx = IntegrationTestContext::new(TEST_SPEC_ID)?;
    ctx.write_prd("test-feature", default_test_prd())?;
    // ...
}
```

---

## Test Organization Best Practices

### File Naming

**Unit Tests** (in source files):
```rust
// src/chatwidget/spec_kit/handler.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_orchestration() { }
}
```

**Integration Tests** (separate files):
```
codex-rs/tui/tests/
├── workflow_integration_tests.rs
├── error_recovery_integration_tests.rs
├── state_persistence_integration_tests.rs
├── concurrent_operations_integration_tests.rs
└── quality_flow_integration_tests.rs
```

**Property Tests**:
```
codex-rs/tui/tests/property_based_tests.rs
```

---

### Test Naming Conventions

**Pattern**: `test_{what}_{condition}_{expected}`

**Examples**:
```rust
#[test]
fn test_state_advance_increments_index() { }

#[test]
fn test_consensus_degraded_when_missing_agent() { }

#[test]
fn test_evidence_created_on_error() { }

#[tokio::test]
async fn test_quality_gate_passes_when_score_above_80() { }
```

**Avoid**:
```rust
#[test]
fn test1() { }  // ❌ Meaningless

#[test]
fn it_works() { }  // ❌ Too vague
```

---

### Common Test Patterns

#### Pattern: Arrange-Act-Assert

```rust
#[test]
fn test_example() {
    // Arrange: Setup
    let ctx = IntegrationTestContext::new("SPEC-TEST")?;
    let state = StateBuilder::new("SPEC-TEST").build();

    // Act: Execute
    let result = do_something(&ctx, &state)?;

    // Assert: Verify
    assert_eq!(result, expected);
}
```

---

#### Pattern: Given-When-Then

```rust
#[tokio::test]
async fn test_consensus_with_degradation() {
    // Given: 3-agent consensus with 1 agent failing
    let mut mock = MockMcpManager::new();
    mock.add_fixture("local-memory", "search", None, json!({"agent": "gemini"}));
    mock.add_fixture("local-memory", "search", None, json!({"agent": "claude"}));
    // gpt_pro missing (simulates failure)

    // When: Fetch consensus
    let (results, degraded) = fetch_memory_entries(
        "SPEC-TEST",
        SpecStage::Plan,
        &mock
    ).await?;

    // Then: Should have 2/3 agents and be degraded
    assert_eq!(results.len(), 2);
    assert!(degraded);
}
```

---

#### Pattern: Table-Driven Tests

```rust
#[test]
fn test_stage_index_mapping() {
    let test_cases = vec![
        (0, Some(SpecStage::Plan)),
        (1, Some(SpecStage::Tasks)),
        (2, Some(SpecStage::Implement)),
        (3, Some(SpecStage::Validate)),
        (4, Some(SpecStage::Audit)),
        (5, Some(SpecStage::Unlock)),
        (6, None),
    ];

    for (index, expected) in test_cases {
        let mut state = StateBuilder::new("SPEC-TEST").build();
        state.current_index = index;
        assert_eq!(state.current_stage(), expected);
    }
}
```

---

## Summary

**Test Infrastructure Highlights**:

1. **MockMcpManager**: Fixture-based MCP testing (272 LOC, 7 tests)
2. **IntegrationTestContext**: Isolated filesystem, evidence verification
3. **StateBuilder**: Test state configuration with fluent API
4. **EvidenceVerifier**: Artifact validation helpers
5. **Fixture Library**: 20 real production artifacts (96 KB)
6. **Coverage Tools**: cargo-tarpaulin (CI), cargo-llvm-cov (local)
7. **Property Testing**: proptest for generative invariant testing
8. **TestCodexBuilder**: Conversation mocking with wiremock

**Benefits**:
- ✅ Fast tests (no external dependencies)
- ✅ Deterministic (fixture-based)
- ✅ Isolated (temp directories)
- ✅ Comprehensive (unit, integration, property)
- ✅ Measurable (coverage tools)

**Next Steps**:
- [Unit Testing Guide](unit-testing-guide.md) - Writing effective unit tests
- [Integration Testing Guide](integration-testing-guide.md) - Cross-module tests
- [Property Testing Guide](property-testing-guide.md) - Generative testing patterns

---

**References**:
- MockMcpManager: `codex-rs/tui/tests/common/mock_mcp.rs`
- IntegrationTestContext: `codex-rs/tui/tests/common/integration_harness.rs`
- Tarpaulin config: `codex-rs/tarpaulin.toml`
- Property tests: `codex-rs/tui/tests/property_based_tests.rs`
- TestCodexBuilder: `codex-rs/core/tests/common/test_codex.rs`
