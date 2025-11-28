**SPEC-ID**: SYNC-008
**Feature**: ASCII Animation Module
**Status**: Backlog
**Created**: 2025-11-28
**Branch**: feature/sync-008
**Owner**: Code

**Context**: Port `ascii_animation.rs` from upstream providing frame-based ASCII animations for loading states and visual feedback in the TUI. Enhances user experience during long-running operations (model inference, agent execution, file operations) with engaging visual indicators.

**Source**: `~/old/code/codex-rs/tui/src/ascii_animation.rs`

---

## User Scenarios

### P1: Loading State Indication

**Story**: As a user waiting for model responses, I want animated loading indicators so that I know the system is working and not frozen.

**Priority Rationale**: Long waits without visual feedback cause users to think the app is hung.

**Testability**: Trigger long operation and verify animation plays.

**Acceptance Scenarios**:
- Given model inference starts, when waiting, then animation plays in status area
- Given animation frame interval, when elapsed, then next frame renders
- Given operation completes, when done, then animation stops cleanly

### P2: Multiple Animation Variants

**Story**: As a developer, I want multiple animation styles so that different operations have contextually appropriate indicators.

**Priority Rationale**: Visual variety improves UX but is less critical than having any animation.

**Testability**: Trigger different operations and verify appropriate animation plays.

**Acceptance Scenarios**:
- Given file operation, when in progress, then file-appropriate animation plays
- Given network operation, when in progress, then network animation plays
- Given thinking/inference, when in progress, then brain/thinking animation plays

---

## Edge Cases

- Very fast operations (animation may not be visible - use minimum display time?)
- Terminal doesn't support Unicode (fallback to ASCII-only frames)
- Very narrow terminal (truncate or skip animation)
- Animation during user input (don't block typing)
- Multiple concurrent animations (queue or layer?)

---

## Requirements

### Functional Requirements

- **FR1**: Implement `AsciiAnimation` struct with frame array and timing
- **FR2**: Support configurable frame interval (default 100ms)
- **FR3**: Provide at least 3 animation variants (spinner, dots, braille)
- **FR4**: Integrate with Ratatui rendering pipeline
- **FR5**: Support graceful start/stop without visual artifacts

### Non-Functional Requirements

- **Performance**: Animation rendering <1ms per frame
- **Compatibility**: Work in all terminal emulators (Unicode + ASCII fallback)
- **Visual**: Smooth animation at 10fps (100ms intervals)

---

## Success Criteria

- Module compiles and integrates with TUI
- At least one animation variant works in app.rs
- Animation starts/stops cleanly without artifacts
- Works in common terminals (iTerm2, Terminal.app, Windows Terminal, Alacritty)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-tui
./target/debug/codex-tui
# Trigger a model call and observe animation
```

---

## Dependencies

- `ratatui` for rendering (existing)
- TUI app.rs integration point

---

## Notes

- Estimated 4-6h including TUI integration
- Fork's TUI structure may differ from upstream - verify integration points
- Consider making animation optional via config for minimal/accessibility mode
