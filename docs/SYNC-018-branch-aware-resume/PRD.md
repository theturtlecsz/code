**SPEC-ID**: SYNC-018
**Feature**: Branch-Aware Session Resume
**Status**: Backlog
**Created**: 2025-11-28
**Branch**: feature/sync-018
**Owner**: Code

**Context**: Port branch filtering to the session resume picker from upstream. When resuming sessions in large workspaces with many branches, this filters the session list to show only sessions from the current git branch, reducing clutter and improving findability.

**Source**: `~/old/code/codex-rs/tui/src/` (session resume with branch filtering)

---

## User Scenarios

### P1: Filter Sessions by Current Branch

**Story**: As a developer working on multiple features, I want session resume filtered by branch so that I quickly find relevant sessions.

**Priority Rationale**: Large workspaces accumulate many sessions; branch filtering is a quality-of-life improvement.

**Testability**: Create sessions on different branches, switch branches, verify filtered list.

**Acceptance Scenarios**:
- Given sessions on branches A, B, C, when on branch A, then only branch A sessions shown by default
- Given filtered view, when user toggles filter, then all sessions shown
- Given session selected, when resumed, then conversation continues correctly

### P2: Show Branch in Session List

**Story**: As a developer, I want to see which branch each session is from so that I can identify sessions even with filter off.

**Priority Rationale**: Branch visibility aids identification; complements filtering.

**Testability**: View session list, verify branch names displayed.

**Acceptance Scenarios**:
- Given session list, when displayed, then branch name shown for each session
- Given orphan session (branch deleted), when displayed, then shows "(deleted branch)"

---

## Edge Cases

- Session from deleted branch (show with "(deleted)" indicator)
- Not in a git repository (disable filtering, show all)
- Detached HEAD state (filter by commit or show all)
- Branch renamed since session created (handle gracefully)
- Very long branch names (truncate in display)

---

## Requirements

### Functional Requirements

- **FR1**: Store git branch name with session metadata at creation
- **FR2**: Filter session resume picker by current branch (default on)
- **FR3**: Provide toggle to show all sessions regardless of branch
- **FR4**: Display branch name in session list view
- **FR5**: Handle edge cases (no git, deleted branch, detached HEAD)

### Non-Functional Requirements

- **Performance**: Branch detection <10ms
- **Usability**: Clear indication when filter is active
- **Compatibility**: Graceful degradation for non-git directories

---

## Success Criteria

- Session metadata includes branch name
- Resume picker filters by current branch by default
- Toggle available to show all sessions
- Branch name visible in session list
- Works correctly outside git repos (shows all)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-tui

# Create sessions on different branches
git checkout -b feature-a
./target/debug/codex-tui  # Start session, exit
git checkout -b feature-b
./target/debug/codex-tui  # Start session, exit

# Resume on feature-a
git checkout feature-a
./target/debug/codex-tui
# Session picker should show only feature-a session by default
```

---

## Dependencies

- Session management (existing rollout module)
- Git info utilities (existing git_info module)
- TUI session picker (existing)

---

## Notes

- Estimated 2-3h - small, focused QoL improvement
- Session metadata schema may need migration for existing sessions
- Consider keyboard shortcut to toggle filter quickly
- May want to persist filter preference in config
