# P4 Sync Continuation Session

**Generated**: 2025-11-29
**Previous Session**: P3 Sync (SYNC-009 footer ported, P3 items triaged)
**Priority**: Complete SYNC-009 footer integration, then expand scope

---

## Session Objective

Complete the footer module integration (SYNC-009) and establish tracking for deferred sync items. This session builds directly on the P3 work where `footer.rs` was ported but not yet wired into the chat_composer.

---

## Phase 1: SYNC-009 Footer Completion (Priority 1)

### Current State
- `tui/src/bottom_pane/footer.rs` exists (380 LOC, 6 tests)
- `FooterMode` enum, `FooterProps` struct, `render_footer()` implemented
- `prefix_lines` and `FOOTER_INDENT_COLS` ready in supporting modules
- **NOT WIRED**: chat_composer.rs still uses inline footer rendering

### Integration Tasks

1. **Add FooterMode state to ChatComposer**
   - File: `tui/src/bottom_pane/chat_composer.rs`
   - Add `footer_mode: FooterMode` field
   - Initialize to `FooterMode::ShortcutSummary`

2. **Replace inline footer rendering with footer module**
   - Location: Lines ~1750-1968 in chat_composer.rs (render section)
   - Replace `Line::from(line_spans)` construction with `render_footer()`
   - Wire `FooterProps` from ChatComposer state

3. **Wire "?" key to toggle FooterMode**
   - Add keyboard handler for `KeyCode::Char('?')`
   - Call `toggle_shortcut_mode()` from footer module

4. **Add integration tests with insta snapshots**
   - Test footer modes render correctly
   - Snapshot different FooterMode states

### Verification
```bash
cd ~/code/codex-rs && cargo build -p codex-tui
cargo test -p codex-tui --lib -- footer
cargo clippy -p codex-tui --lib -- -D warnings
```

---

## Phase 2: Documentation & Tracking

### Create P4 Deferred Tracker
Create `docs/SYNC-P4-DEFERRED.md` with:
- SYNC-010 Auto Drive Patterns (10-20h, architectural refactor)
- SYNC-016 Device Code Auth (3-5h, blocked on auth sync)
- Any other deferred items

### Update Existing Docs
- Archive `docs/NEXT-SESSION-P3-SYNC.md` or mark as complete
- Update CLAUDE.md sync status if applicable

---

## Phase 3: Optional Deep Dives

### 3.1 SYNC-010 Investigation
**Goal**: Document ToolOrchestrator pattern for potential future port
- Map upstream tools/ directory structure
- Document SandboxRetryData flow
- Identify minimal viable port approach
- Estimate effort with more precision

### 3.2 Auth Module Diff Report
**Goal**: Unblock SYNC-016 by understanding auth differences
- Compare `codex_core::auth` fork vs upstream
- Document: AuthCredentialsStoreMode, save_auth, ensure_workspace_allowed
- Identify which changes are breaking vs additive
- Create migration path

### 3.3 Footer Integration Tests
**Goal**: Ensure footer robustness
- Add insta snapshot tests for each FooterMode
- Test edge cases (narrow width, missing context %)
- Test mode transitions

---

## Local Memory Queries

```bash
# Check P3 session context
~/.claude/hooks/lm-search.sh "SYNC-009 footer"
~/.claude/hooks/lm-search.sh "SYNC-016 device code auth"
~/.claude/hooks/lm-search.sh "P3 sync session"
```

---

## Files to Load

1. `~/.claude/CLEARFRAME.md` - Operating mode
2. `docs/NEXT-SESSION-P4-SYNC.md` - This document
3. `tui/src/bottom_pane/footer.rs` - Footer module (needs integration)
4. `tui/src/bottom_pane/chat_composer.rs` - Target for footer wiring

---

## Success Criteria

- [ ] Footer module fully integrated into chat_composer
- [ ] "?" key toggles shortcut overlay
- [ ] All footer tests passing
- [ ] Clippy clean (no dead_code warnings for footer)
- [ ] P4 deferred tracker created
- [ ] Auth module diff report generated
- [ ] SYNC-010 architecture documented
