# P110 Continuation Prompt - ChatWidget Refactor + Upstream Integration

**Session**: P110
**Created**: 2025-12-14
**Previous**: P109 (GR001-001 Policy Compliance)

---

## Pre-flight

```bash
git log -1 --oneline  # Expect commit after P109 (feat(consensus): implement GR-001)
./build-fast.sh
cargo test -p codex-tui --lib gr001 -- --test-threads=1  # Verify P109 tests
```

---

## Primary Goal: MAINT-11 ChatWidget Monolith Refactor

Break `codex-rs/tui/src/chatwidget/mod.rs` "gravity well" into smaller modules behind stable interfaces.

### Context

- **File Size**: mod.rs is ~18,000+ LOC (largest single file in codebase)
- **Pain Points**: Merge conflicts, cognitive load, slow IDE, hard to navigate
- **Strategy**: Extract behind stable interfaces while preserving behavior and tests

### Phase Order

1. **Analysis** - Read mod.rs, identify logical groupings (~30 min)
2. **Interface Design** - Define public API surface to preserve (~30 min)
3. **Input Submission** - Extract input handling to `input.rs` (~2 hrs)
4. **Slash Command Routing** - Extract to `slash_routing.rs` (~2 hrs)
5. **Unit Tests** - Validate extraction preserves behavior (~1 hr)
6. **Documentation** - Update SPEC.md MAINT-11 entry (~15 min)

### Extraction Candidates (Priority Order)

| Component | Est. LOC | Target Module | Dependencies |
|-----------|----------|---------------|--------------|
| Input submission | ~500 | `input.rs` | SubmissionMode, validate_input() |
| Slash command dispatch | ~800 | `slash_routing.rs` | handle_slash_command(), is_slash_command() |
| Agent lifecycle | ~600 | `agent_view.rs` | spawn_agent(), track_agent_status() |
| Message rendering | ~1200 | `message_view.rs` | format_message(), syntax_highlight() |
| Keyboard handling | ~400 | `keybindings.rs` | handle_key_event() |

### First Actions

1. Read `codex-rs/tui/src/chatwidget/mod.rs` (full file scan)
2. Identify top-level public functions and their dependencies
3. Create `input.rs` stub with interface
4. Migrate input submission logic incrementally

---

## Secondary Goal: SYNC-005 + SYNC-006 Integration

Complete upstream sync integration for keyring-store and feedback crates.

### SYNC-005: Keyring Integration (~2-3 hrs)

**Crate**: `codex-rs/keyring-store/`
**Status**: 241 LOC, KeyringStore + MockKeyringStore ready
**Integration Points**:
- TUI OAuth2 token storage (replace plaintext ~/.config/code/tokens)
- CLI credential caching

**Tasks**:
1. Add keyring-store dependency to tui/Cargo.toml
2. Create `credentials.rs` module in TUI
3. Migrate token storage from file to keyring
4. Add fallback to file storage (headless servers)
5. Unit tests for credential flow

### SYNC-006: Feedback Integration (~2-3 hrs)

**Crate**: `codex-rs/feedback/`
**Status**: 306 LOC, 6 tests, ring buffer + tracing ready
**Integration Points**:
- Error reporting collection
- User feedback capture
- Session telemetry aggregation

**Tasks**:
1. Add feedback dependency to tui/Cargo.toml
2. Create `feedback_collector.rs` module
3. Wire into error handlers
4. Add export command or API endpoint
5. Unit tests for feedback capture

---

## Testing Strategy

**Unit tests only** (per user selection). E2E validation deferred.

```bash
# After each extraction
cargo test -p codex-tui --lib input::  # Test new module
cargo test -p codex-tui --lib         # Full lib tests

# Keyring tests
cargo test -p keyring-store

# Feedback tests
cargo test -p feedback
```

---

## Session Tracking

| Task | Status | Est. Hours |
|------|--------|-----------|
| MAINT-11 Analysis | Pending | 0.5 |
| MAINT-11 Input Extraction | Pending | 2.0 |
| MAINT-11 Slash Routing | Pending | 2.0 |
| SYNC-005 Keyring Integration | Pending | 2.5 |
| SYNC-006 Feedback Integration | Pending | 2.5 |
| Unit Tests | Pending | 1.0 |
| Documentation | Pending | 0.5 |
| **Total** | | **11.0** |

---

## Tracking References

- **SPEC.md**: MAINT-11 (line 186), SYNC-005 (line 247), SYNC-006 (line 248)
- **Previous Session**: P109 (GR001-001 complete)
- **Related SPECs**: MAINT-10 (deferred), SPEC-KIT-926 (future)

---

## Success Criteria

1. mod.rs reduced by ~1,300+ LOC (input + slash routing)
2. New modules have stable public interfaces
3. All existing tests pass
4. Keyring-store integrated with fallback
5. Feedback crate wired to error handlers
6. SPEC.md updated with completion status

---

## Rollback Plan

If extraction causes issues:
```bash
git checkout HEAD -- codex-rs/tui/src/chatwidget/
./build-fast.sh
```

Keep extractions in separate commits for easy revert.
