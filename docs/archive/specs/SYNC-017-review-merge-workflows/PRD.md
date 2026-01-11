**SPEC-ID**: SYNC-017
**Feature**: /review and /merge Workflow Commands
**Status**: Backlog
**Created**: 2025-11-28
**Branch**: feature/sync-017
**Owner**: Code

**Context**: Port `/review` (uncommitted changes review) and `/merge` slash commands from upstream. These provide structured workflows for code review and merge operations, integrating with the fork's existing git-tooling module and slash command infrastructure.

**Source**: `~/old/code/codex-rs/tui/src/slash_command.rs` (review/merge commands)

---

## User Scenarios

### P1: Review Uncommitted Changes

**Story**: As a developer with uncommitted changes, I want to run `/review` so that I get AI-assisted code review before committing.

**Priority Rationale**: Pre-commit review catches issues early; high-value workflow.

**Testability**: Make changes, run /review, verify structured review output.

**Acceptance Scenarios**:
- Given uncommitted changes, when `/review` executed, then diff is analyzed
- Given review complete, when output shown, then issues are categorized (bugs, style, security)
- Given no uncommitted changes, when `/review` executed, then helpful message shown

### P2: Merge Workflow Assistance

**Story**: As a developer merging branches, I want `/merge` assistance so that I can handle conflicts with AI help.

**Priority Rationale**: Merge conflicts are common pain point; AI assistance valuable.

**Testability**: Create merge conflict, run /merge, verify conflict resolution suggestions.

**Acceptance Scenarios**:
- Given merge conflict, when `/merge` executed, then conflicts are analyzed
- Given conflict analysis, when suggestions shown, then resolution options provided
- Given clean merge possible, when `/merge` executed, then merge proceeds

### P3: Custom Review Presets

**Story**: As a team lead, I want custom review presets so that reviews follow team conventions.

**Priority Rationale**: Customization is valuable but default presets cover most needs.

**Testability**: Configure custom preset, run review, verify custom rules applied.

**Acceptance Scenarios**:
- Given custom preset configured, when `/review --preset security` executed, then security-focused review runs
- Given no preset specified, when `/review` executed, then default preset used

---

## Edge Cases

- Very large diffs (chunk and summarize)
- Binary file changes (skip or note)
- Staged vs unstaged changes (handle both)
- Submodule changes (handle or skip with note)
- Merge with multiple conflict files (process all)

---

## Requirements

### Functional Requirements

- **FR1**: Implement `/review` slash command for uncommitted changes
- **FR2**: Implement `/review --staged` variant for staged-only review
- **FR3**: Implement `/merge` slash command for merge assistance
- **FR4**: Integrate with fork's git-tooling module for diff extraction
- **FR5**: Support review presets (default, security, performance, style)
- **FR6**: Provide structured output with issue categorization

### Non-Functional Requirements

- **Performance**: Review should complete in <30s for typical diffs (<1000 lines)
- **Usability**: Clear output formatting with actionable suggestions
- **Integration**: Work with fork's existing slash command system

---

## Success Criteria

- `/review` command shows in `/help` output
- Uncommitted changes are analyzed with categorized feedback
- `/merge` provides useful conflict resolution suggestions
- Commands integrate with existing git-tooling module
- No conflicts with existing slash commands

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-tui
./target/debug/codex-tui

# In TUI:
# Make some changes to a file
/review
# Verify structured review output

# Create merge conflict scenario
/merge
# Verify conflict analysis
```

---

## Dependencies

- Slash command infrastructure (existing)
- Git-tooling module (existing)
- Diff parsing utilities (existing or add)

---

## Notes

- Estimated 6-8h
- Fork has existing slash command system - integrate, don't replace
- Review presets could be configured via CLAUDE.md or config file
- Consider integration with /commit command for review-then-commit workflow
