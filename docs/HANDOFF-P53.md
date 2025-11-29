# P53 Session Handoff - Integration Phase

## Current State (2025-11-29)
- **Commit**: `0832fa4e4` on main (pushed)
- **Tests**: 1836+ passing
- **Tree**: Clean

## Session Goal: Integration Work

The core SYNC crates are implemented but **not yet wired into the codebase**. This session integrates them.

## Integration Queue (Priority Order)

### Phase 1: async-utils Adoption (2-3h)
**Goal**: Replace ad-hoc `tokio::select!` cancellation with `.or_cancel()`

**Why**: Code consistency, fewer bugs, better readability

**Pattern to find and replace**:
```rust
// FIND: Manual select patterns
tokio::select! {
    _ = token.cancelled() => { ... }
    result = some_operation() => { result }
}

// REPLACE WITH:
some_operation().or_cancel(&token).await
```

**Files to search**:
```bash
grep -r "token.cancelled()" codex-rs/core/src/
```

**Validation**: `cargo test --workspace`, manual ctrl-c test in TUI

---

### Phase 2: keyring-store -> login (4-8h)
**Goal**: Move OAuth tokens from `~/.codex/device_code_tokens.json` to system keyring

**Why**: Security - encrypted at rest, no token leakage in backups/dotfiles

**Strategy**:
1. Add `codex-keyring-store` dependency to `login/Cargo.toml`
2. Modify `device_code_storage.rs`:
   - Add keyring storage alongside file storage
   - Load: try keyring first, fall back to file, migrate if found
   - Save: always save to keyring
3. Use `MockKeyringStore` in tests

**Files**:
- `login/Cargo.toml`
- `login/src/device_code_storage.rs`
- `login/src/lib.rs` (re-export if needed)

**Testing**: Unit tests with MockKeyringStore, manual login flow test

---

### Phase 3: feedback -> TUI (4-6h)
**Goal**: Add `/feedback` command for log capture and export

**Why**: Self-debugging, better bug reports, session forensics

**Strategy**:
1. Initialize `CodexFeedback` in `tui/src/main.rs` with tracing layer
2. Add `/feedback` slash command
3. Create feedback view (optional - can start with just file export)

**Files**:
- `tui/Cargo.toml` - add codex-feedback
- `tui/src/main.rs` - initialize with tracing
- `tui/src/slash_command.rs` - add /feedback
- `tui/src/bottom_pane/feedback_view.rs` (NEW, optional)

**Testing**: Manual test of /feedback command

---

## Reference Files

| Crate | Source | Key Types |
|-------|--------|-----------|
| async-utils | `codex-rs/async-utils/src/lib.rs` | `OrCancelExt`, `CancelErr` |
| keyring-store | `codex-rs/keyring-store/src/lib.rs` | `KeyringStore`, `MockKeyringStore` |
| feedback | `codex-rs/feedback/src/lib.rs` | `CodexFeedback`, `FeedbackMakeWriter` |

## Validation Checklist
After each phase:
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Manual TUI testing
- [ ] Commit with descriptive message

## To Start Session

```
load ~/code/docs/HANDOFF-P53.md
```

Then say: "Start Phase 1: async-utils adoption"
