**SPEC-ID**: SYNC-009
**Feature**: Footer Improvements
**Status**: Backlog
**Created**: 2025-11-28
**Branch**: feature/sync-009
**Owner**: Code

**Context**: Extract and adapt footer enhancements from upstream's `footer.rs` including FooterMode enum, context window percentage display, and improved keyboard hints. The fork handles footer inline in `bottom_pane_view.rs`; this task adapts useful patterns without full file replacement.

**Source**: `~/old/code/codex-rs/tui/src/bottom_pane/footer.rs`

---

## User Scenarios

### P1: Context Window Usage Display

**Story**: As a user in long conversations, I want to see context window usage so that I know when I'm approaching limits.

**Priority Rationale**: Running out of context silently leads to degraded responses; visibility enables proactive management.

**Testability**: Start conversation, add messages, verify percentage updates.

**Acceptance Scenarios**:
- Given conversation with messages, when footer renders, then context % is shown
- Given context approaching 80%, when displayed, then warning color is used
- Given context exceeds 90%, when displayed, then critical color is used

### P2: Dynamic Footer Modes

**Story**: As a user, I want contextual footer hints so that I see relevant keyboard shortcuts for my current state.

**Priority Rationale**: Contextual hints reduce learning curve but aren't blocking.

**Testability**: Enter different states and verify footer hints change.

**Acceptance Scenarios**:
- Given input mode, when footer renders, then input-relevant shortcuts shown
- Given approval modal open, when footer renders, then approval shortcuts shown
- Given Ctrl+C pressed, when footer renders, then cancel reminder shown

### P3: Keyboard Hint Organization

**Story**: As a new user, I want organized keyboard hints so that I can discover available actions.

**Priority Rationale**: Discoverability is nice-to-have; users can refer to docs.

**Testability**: Verify hints are grouped logically and fit in footer space.

**Acceptance Scenarios**:
- Given footer width, when hints rendered, then most important hints visible
- Given narrow terminal, when hints rendered, then graceful truncation occurs

---

## Edge Cases

- Very narrow terminal (hide or truncate hints)
- Unknown context size (show "N/A" or estimate)
- Footer conflicts with input area (proper z-ordering)
- High message count affects percentage calculation accuracy

---

## Requirements

### Functional Requirements

- **FR1**: Implement `FooterMode` enum: CtrlCReminder, ShortcutSummary, EscHint, ContextOnly
- **FR2**: Add context window percentage calculation and display
- **FR3**: Color-code context percentage (green <70%, yellow 70-90%, red >90%)
- **FR4**: Implement mode-switching logic based on app state
- **FR5**: Adapt to fork's bottom_pane_view.rs structure

### Non-Functional Requirements

- **Performance**: Footer rendering <1ms
- **Usability**: Hints readable in terminals ≥80 columns
- **Compatibility**: Work with fork's existing bottom pane architecture

---

## Success Criteria

- Context percentage displays accurately in footer
- Footer mode changes based on app state
- No visual regressions in existing footer functionality
- Works with fork's bottom_pane_view.rs (not a separate footer.rs)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-tui
./target/debug/codex-tui
# Start conversation, add several messages
# Verify context % appears and updates
```

---

## Dependencies

- TUI bottom_pane_view.rs (existing)
- Token counting infrastructure (existing or needs addition)

---

## Notes

- Estimated 4-6h
- Fork doesn't have separate footer.rs - integrate into bottom_pane_view.rs
- Context percentage requires token counting - verify fork has this capability
- Consider config option to hide percentage for minimal UI
