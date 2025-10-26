# ACE Learning Integration Usage

## Overview

The ACE learning system collects execution outcomes (compile, tests, lints) and sends compact feedback to ACE for continuous learning and improvement.

**Important**: ACE is a **data-only** SQLite store accessed via MCP. It stores and retrieves playbook heuristics but does NOT call LLMs. The CODE orchestrator calls LLMs using your configured API keys.

## Integration Point

Add ACE learning hook after quality gate validation completes. Example location: `quality_gate_handler.rs` after validation results are processed.

## Usage Example

```rust
use super::ace_learning::{ExecutionFeedback, send_learning_feedback_sync};
use super::ace_route_selector::DiffStat;

// After validation completes, collect results
let feedback = ExecutionFeedback::new()
    .with_compile_ok(compile_success)
    .with_tests_passed(all_tests_passed)
    .with_failing_tests(failing_test_names)  // Vec<String> of names only
    .with_lint_issues(lint_error_count)
    .with_stack_traces(error_traces)         // Will be trimmed to 2KB
    .with_diff_stat(DiffStat::new(files_changed, insertions, deletions));

// Send learning feedback (async, non-blocking)
let repo_root = get_repo_root(&widget.config.cwd).unwrap_or_else(|| ".".to_string());
let branch = get_current_branch(&widget.config.cwd).unwrap_or_else(|| "main".to_string());
let scope = "implement";  // or "test" depending on stage
let task_title = "Add user authentication";

send_learning_feedback_sync(
    &widget.config.ace,
    repo_root,
    branch,
    scope,
    task_title,
    feedback,
    Some(diff_stat),
);
```

## Feedback Structure

The feedback sent to ACE is a compact JSON object:

```json
{
  "compile_ok": false,
  "tests_passed": false,
  "failing_tests": ["test_auth_flow", "test_login"],
  "lint_issues": 3,
  "stack_traces": [
    "Error: undefined variable\n  at auth.rs:42",
    "Error: type mismatch\n  at login.rs:15"
  ],
  "diff_stat": {
    "files": 5,
    "insertions": 150,
    "deletions": 20
  }
}
```

## Log Output

When learning completes, you'll see:

```
INFO ACE learn 127ms scope=implement added=2 demoted=1 promoted=3
```

- `127ms` - Time taken for ACE learning call
- `scope=implement` - Scope of learning (implement/test)
- `added=2` - New playbook bullets created
- `demoted=1` - Bullets marked as less helpful
- `promoted=3` - Bullets marked as more helpful

## Error Handling

The system gracefully handles all failure modes:

- **ACE disabled**: Silent skip, debug log only
- **ACE unavailable**: Logged at WARN level, doesn't block execution
- **Not on tokio runtime**: Silent skip, debug log only
- **Serialization failure**: Logged at WARN level, no learning sent

## Performance

- **Non-blocking**: Spawns async task, returns immediately
- **Compact**: Feedback capped at ~2-3KB JSON
- **Efficient**: Stack traces trimmed to 2KB total
- **Safe**: Fire-and-forget, never blocks validation flow

## Example Integration

```rust
// In quality_gate_handler.rs, after checkpoint validation completes:

pub fn on_checkpoint_validation_complete(
    widget: &mut ChatWidget,
    checkpoint: QualityCheckpoint,
    results: &ValidationResults,
) {
    // ... existing validation logic ...

    // Collect feedback for ACE learning
    let feedback = ExecutionFeedback::new()
        .with_compile_ok(results.compile_success)
        .with_tests_passed(results.tests_passed)
        .with_failing_tests(results.failing_test_names.clone())
        .with_lint_issues(results.lint_errors.len())
        .with_stack_traces(results.error_traces.clone())
        .with_diff_stat(results.diff_stat.clone());

    // Send to ACE (non-blocking)
    if let Some(state) = &widget.spec_auto_state {
        let repo_root = get_repo_root(&widget.config.cwd)
            .unwrap_or_else(|| ".".to_string());
        let branch = get_current_branch(&widget.config.cwd)
            .unwrap_or_else(|| "main".to_string());

        send_learning_feedback_sync(
            &widget.config.ace,
            repo_root,
            branch,
            "implement",
            &state.task_title,
            feedback,
            results.diff_stat.clone(),
        );
    }

    // ... continue with existing flow ...
}
```

## Testing

All learning logic is unit tested:

```bash
cargo test -p codex-tui --lib ace_learning::tests
```

Tests cover:
- Success/failure detection
- Feedback serialization
- Stack trace trimming
- Patch summary formatting
- Builder pattern
