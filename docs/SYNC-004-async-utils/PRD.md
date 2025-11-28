**SPEC-ID**: SYNC-004
**Feature**: Async Utils Crate
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-004
**Owner**: Code

**Context**: Port the `async-utils` crate from upstream providing the `.or_cancel()` extension trait for futures. This enables clean cancellation patterns using `tokio_util::sync::CancellationToken` without verbose `tokio::select!` boilerplate throughout the codebase.

**Source**: `~/old/code/codex-rs/async-utils/` (~90 LOC)

---

## User Scenarios

### P1: Clean Future Cancellation

**Story**: As a developer, I want a clean `.or_cancel()` extension so that I can write cancellable async code without repetitive select! patterns.

**Priority Rationale**: Reduces boilerplate and makes cancellation logic more readable and consistent across the codebase.

**Testability**: Unit tests for the trait implementation with various future types.

**Acceptance Scenarios**:
- Given a future and cancellation token, when `.or_cancel(&token)` is called, then it returns `Result<T, CancelErr>`
- Given the token is cancelled before future completes, when awaited, then `CancelErr` is returned
- Given the future completes before cancellation, when awaited, then `Ok(result)` is returned

---

## Edge Cases

- Token cancelled exactly as future completes (race condition - either result is acceptable)
- Future that never completes with cancellation (should return CancelErr)
- Nested `.or_cancel()` calls (should work correctly)
- Future panics during cancellation (panic should propagate)

---

## Requirements

### Functional Requirements

- **FR1**: Implement `OrCancelExt` trait with `or_cancel()` method for any `Future`
- **FR2**: Define `CancelErr` error type for cancellation indication
- **FR3**: Use `tokio::select!` internally with `biased` for deterministic behavior
- **FR4**: Integrate with `tokio_util::sync::CancellationToken`

### Non-Functional Requirements

- **Performance**: Zero overhead when not cancelled (just future polling)
- **Ergonomics**: Single import to use (`use async_utils::OrCancelExt`)
- **Compatibility**: Work with any `Future` type (generic implementation)

---

## Success Criteria

- Crate compiles and is added to workspace
- `OrCancelExt` trait is usable from other workspace crates
- Unit tests pass for cancellation and completion scenarios
- At least one usage example in codebase (optional, can be added later)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-async-utils
cd codex-rs && cargo test -p codex-async-utils
```

---

## Dependencies

- `tokio` with `sync` feature
- `tokio_util` with `sync` feature (for `CancellationToken`)

---

## Notes

- Small utility crate (~90 LOC) - quick to port
- Can be used to simplify existing ad-hoc `select!` patterns in DirectProcessExecutor
- No breaking changes to existing code - purely additive
