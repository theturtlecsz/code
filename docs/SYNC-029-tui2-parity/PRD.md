**SPEC-ID**: SYNC-029
**Feature**: TUI v2 parity pass (skills, /ps, bridge lane, keybindings)
**Status**: Backlog
**Created**: 2025-12-22
**Branch**: feature/sync-029
**Owner**: Code

**Context**: Make tui2 usable for day-to-day chat sessions by bringing over key UX affordances and ensuring the same backend contracts look identical.

---

## User Scenarios

### P1: Primary User Value

**Story**: As a user, I want tui v2 parity pass (skills, /ps, bridge lane, keybindings) so that the system becomes more capable without regressions.

**Priority Rationale**: Enables upstream parity and reduces future merge friction.

**Testability**: Validate via manual TUI run + targeted cargo tests.

**Acceptance Scenarios**:
- Tell a short story of the happy-path behavior.
- Include a negative-path behavior (feature disabled / unavailable).

### P2: Operator / Maintainer Value

**Story**: As a maintainer, I want clear boundaries and feature gating so that upstream syncs remain rebase-safe.

**Acceptance Scenarios**:
- Given feature is disabled, when code runs, then behavior is unchanged.
- Given feature is enabled, when code runs, then behavior is visible and testable.

### P3: Production / Edge Reliability

**Story**: As a user, I want failures to be contained so that the TUI stays responsive.

**Acceptance Scenarios**:
- Given a failure, when it occurs, then errors are surfaced without crashing.

---

## Edge Cases

- Feature disabled: must behave exactly as today.
- Partial config: default behavior must be safe.
- Cross-platform: must not assume Linux-only behavior unless feature is gated.

---

## Requirements

### Functional Requirements

- **FR1**: Implement TUI v2 parity pass (skills, /ps, bridge lane, keybindings) behind an explicit feature flag or config.
- **FR2**: Integrate with existing TUI/CLI flows without breaking stable paths.
- **FR3**: Provide a visible user-facing or debug-visible confirmation when enabled.

### Non-Functional Requirements

- **Performance**: No noticeable startup regression.
- **Reliability**: Feature failures degrade gracefully.
- **Rebase Safety**: Prefer additive modules; avoid invasive refactors.

---

## Success Criteria

- Builds cleanly in workspace.
- Feature is reachable and verifiable.
- No regression in existing flows when disabled.

---

## Evidence & Validation

**Validation Commands**:


**Evidence Path**:
- docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SYNC-029/

---

## Dependencies

- SYNC-028
- SYNC-020 (skills)
- SYNC-024 (/ps)
- SYNC-022 (bridge)

---

## Notes

Treat spec-kit as optional: keep spec-kit-first workflows on TUI1 until tui2 is stable.

---

## Model & Runtime (Spec Overrides)

**MODEL-POLICY.md Version**: 1.0.0

- **Default**: Local-first; do not introduce new cloud model requirements.
- **Single-owner pipeline**: No consensus/voting dependencies.
- **HR / Security**: If this feature expands sandbox/network surface area, require explicit approval in the PRD and add a safety checklist.
