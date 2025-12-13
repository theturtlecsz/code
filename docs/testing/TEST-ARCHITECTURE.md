# Test Architecture Guide

**Status**: Current
**Created**: 2025-11-28 (SPEC-958 Session 12)

This document describes the test infrastructure for the codex-rs Rust codebase.

---

## Test Organization

### Directory Structure

```
codex-rs/
├── core/
│   ├── src/
│   │   └── codex.rs          # Contains inline unit tests (relocated from integration)
│   └── tests/
│       ├── all.rs            # Main test binary entry point
│       ├── common/           # Shared test utilities (library crate)
│       │   ├── lib.rs        # Re-exports all test support code
│       │   ├── responses.rs  # SSE response builders (wiremock)
│       │   └── test_codex.rs # TestCodex builder pattern
│       ├── fixtures/         # JSON fixture files
│       │   ├── completed_template.json
│       │   ├── incomplete_sse.json
│       │   └── ordered_responses_template.json
│       ├── suite/            # Integration tests (aggregated in mod.rs)
│       │   ├── mod.rs        # Aggregates all test modules
│       │   ├── client.rs
│       │   ├── compact.rs
│       │   ├── compact_resume_fork.rs
│       │   ├── fork_conversation.rs
│       │   ├── json_result.rs
│       │   ├── model_overrides.rs
│       │   ├── prompt_caching.rs
│       │   └── ... (18 total modules)
│       └── *.rs              # Standalone integration tests
├── exec/tests/suite/         # Exec crate tests
├── mcp-server/tests/suite/   # MCP server tests
└── tui/tests/                # TUI tests
```

### Test Naming Conventions

- **File names**: snake_case matching feature area (e.g., `fork_conversation.rs`)
- **Test functions**: Descriptive snake_case explaining behavior (e.g., `session_configured_event_contains_rollout_path`)
- **Module structure**: Tests grouped by feature in `suite/` subdirectory

---

## Test Categories

### 1. Unit Tests

**Location**: Inline `#[cfg(test)]` modules within source files

```rust
// In core/src/codex.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_specific_behavior() {
        // ...
    }
}
```

**Purpose**: Test internal implementation details that require private access

**Key files**:
- `core/src/codex.rs` - Contains relocated tests from integration suite
- `core/src/config.rs` - Configuration parsing tests

### 2. Integration Tests

**Location**: `core/tests/suite/*.rs`

**Entry point**: `core/tests/all.rs` imports `suite` module

**Pattern**:
```rust
// In core/tests/suite/mod.rs
mod client;
mod compact;
mod fork_conversation;
// ... 18 total modules

// In individual test files
use core_test_support::*;

#[tokio::test]
async fn test_feature_behavior() {
    let server = start_mock_server().await;
    let codex = test_codex().build(&server).await.unwrap();
    // ...
}
```

### 3. Stubbed Tests (Documented Gaps)

Tests removed but documented with comment stubs indicating the original API dependency:

```rust
// Stubbed: Needed Op::UserTurn which was removed from core API
// fn override_turn_context_does_not_persist_when_config_exists() {}
```

**Count**: 12 stubbed tests (see [SPEC-958-test-migration.md](../SPEC-958-test-migration.md))

### 4. Ignored Tests

Tests with `#[ignore]` attribute and documented blockers:

```rust
#[tokio::test]
#[ignore = "Token-based auto-compact not implemented (only error-message triggered)"]
async fn auto_compact_runs_after_token_limit_hit() {
    // ...
}
```

**Count**: 12 ignored tests with accurate blockers documented

---

## Mock Infrastructure

### wiremock Usage

All integration tests use [wiremock](https://crates.io/crates/wiremock) for HTTP mocking.

**Starting a mock server**:
```rust
use core_test_support::start_mock_server;

let server = start_mock_server().await;
// server.uri() returns "http://127.0.0.1:PORT"
```

**Mounting SSE responses**:
```rust
use core_test_support::responses::*;

// Build SSE body
let body = sse(vec![
    ev_assistant_message("msg_1", "Hello"),
    ev_completed("resp_1"),
]);

// Mount on mock server
mount_sse_once(&server, any(), body).await;
```

### SSE Response Builders (`core_test_support::responses`)

| Function | Purpose |
|----------|---------|
| `sse(events: Vec<Value>)` | Build SSE stream body from JSON events |
| `ev_completed(id)` | Completed response event |
| `ev_completed_with_tokens(id, tokens)` | Completed with token count |
| `ev_assistant_message(id, text)` | Assistant text output |
| `ev_function_call(call_id, name, args)` | Function call output |
| `ev_apply_patch_function_call(call_id, patch)` | apply_patch call |
| `sse_response(body)` | Build ResponseTemplate with SSE headers |
| `start_mock_server()` | Start wiremock server with body limit |

### Fixture Loading

```rust
use core_test_support::load_sse_fixture_with_id;

// Loads from tests/fixtures/, replaces "__ID__" with provided id
let events = load_sse_fixture_with_id("ordered_responses_template.json", "test_id")?;
let body = sse(events);
```

### TestCodex Builder

```rust
use core_test_support::test_codex;

let codex = test_codex()
    .with_config(|c| {
        c.model = "gpt-5".to_string();
        c.output_schema = Some(json!({"type": "object"}));
    })
    .build(&server)
    .await?;

// Access:
// codex.codex - Arc<CodexConversation>
// codex.session_configured - SessionConfiguredEvent
// codex.cwd - TempDir
// codex.home - TempDir
```

---

## Fork-Specific Testing Notes

See [FORK-DIVERGENCES.md](../FORK-DIVERGENCES.md) for complete documentation.

### Key Differences from Upstream

1. **Tools list**: Fork adds browser_*, agent_*, web_* tools
2. **Payload structure**: 5 messages vs upstream 3
3. **Role names**: `developer` changed to `user`
4. **Auto-compact**: Only error-message triggered, not token-count based

### Impact on Tests

- `prompt_tools_are_consistent_across_requests`: Updated expected tools for fork
- `compact_resume_fork` tests: Ignored due to payload structure evolution
- `auto_compact` tests: Ignored because feature not implemented

---

## Running Tests

### Basic Commands

```bash
# Run all core tests
cd codex-rs && cargo test -p codex-core

# Run specific test file
cargo test -p codex-core --test all -- suite::fork_conversation

# Run single test
cargo test -p codex-core --test all -- session_configured_event_contains_rollout_path

# Include ignored tests
cargo test -p codex-core -- --ignored

# Show test output
cargo test -p codex-core -- --nocapture
```

### Test Binary Entry Points

| Crate | Entry Point | Command |
|-------|-------------|---------|
| codex-core | `tests/all.rs` | `cargo test -p codex-core` |
| codex-exec | `tests/all.rs` | `cargo test -p codex-exec` |
| codex-mcp-server | `tests/all.rs` | `cargo test -p codex-mcp-server` |
| codex-tui | `tests/spec_status.rs` | `cargo test -p codex-tui` |

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` | Sandbox test control |
| `SPEC_OPS_HAL_SKIP=1` | Skip HAL secret validation |
| `RUST_LOG=debug` | Enable debug logging |

### CI Considerations

- Tests run with `cargo test --workspace --no-fail-fast`
- Ignored tests are not run in CI (require manual `--ignored` flag)
- Clippy warnings treated as errors: `cargo clippy -- -D warnings`

---

## Current Test Status

**As of SPEC-958 Session 12 (2025-11-28)**:

| Crate | Passing | Ignored | Notes |
|-------|---------|---------|-------|
| codex-core | 31 | 12 | See SPEC-958-test-migration.md |
| codex-exec | - | - | Use `cargo test -p codex-exec` |
| codex-mcp-server | - | - | Use `cargo test -p codex-mcp-server` |

**Ignored tests with blockers**:
- 3 auto_compact tests: Token-based trigger not implemented
- 2 compact_resume_fork tests: Payload structure evolved
- 6 per-turn context tests: Op::OverrideTurnContext partial
- 1 exec timeout test: ExecStream private fields

---

## Adding New Tests

### Integration Test

1. Create or edit file in `core/tests/suite/`
2. If new file, add module to `suite/mod.rs`
3. Use `core_test_support` helpers:

```rust
use core_test_support::*;
use core_test_support::responses::*;

#[tokio::test]
async fn my_new_test() {
    let server = start_mock_server().await;

    // Mount expected responses
    let body = sse(vec![
        ev_assistant_message("msg_1", "Response"),
        ev_completed("resp_1"),
    ]);
    mount_sse_once(&server, any(), body).await;

    // Build test instance
    let codex = test_codex()
        .with_config(|c| c.model = "test-model".to_string())
        .build(&server)
        .await
        .unwrap();

    // Run test
    // ...
}
```

### Unit Test (Internal Access)

Add to inline `#[cfg(test)]` module in source file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_helper() {
        // Can access private items
    }
}
```

---

## Related Documentation

- [SPEC-958-test-migration.md](../SPEC-958-test-migration.md) - Migration tracking and decisions
- [FORK-DIVERGENCES.md](../FORK-DIVERGENCES.md) - Fork vs upstream differences
- [CLAUDE.md](../../CLAUDE.md) - Project instructions (testing section)
