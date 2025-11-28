**SPEC-ID**: SYNC-014
**Feature**: Prompt Management UI
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-014
**Owner**: Code

**Context**: Port prompt save/reload and alias autocomplete functionality from upstream's `custom_prompt_view.rs` and `prompt_args.rs`. This enables users to save frequently used prompts, create aliases, and quickly access them via autocomplete. Consider integration with ACE (Agentic Context Engine) for persistence.

**Source**: `~/old/code/codex-rs/tui/src/bottom_pane/custom_prompt_view.rs`, `prompt_args.rs`

---

## User Scenarios

### P1: Save and Reload Prompts

**Story**: As a power user, I want to save prompts so that I can reuse complex instructions without retyping.

**Priority Rationale**: Prompt reuse is essential for productivity with complex workflows.

**Testability**: Save prompt, restart TUI, verify prompt is still available.

**Acceptance Scenarios**:
- Given a complex prompt, when I save it with a name, then it persists across sessions
- Given saved prompts, when I open prompt manager, then I see list of saved prompts
- Given saved prompt, when I select it, then it's loaded into input field

### P2: Prompt Aliases and Autocomplete

**Story**: As a user, I want prompt aliases so that I can quickly invoke saved prompts with short commands.

**Priority Rationale**: Autocomplete reduces friction for frequent operations.

**Testability**: Type alias prefix and verify autocomplete suggestions.

**Acceptance Scenarios**:
- Given prompt with alias "review", when I type "/rev", then autocomplete suggests "/review"
- Given alias selected, when I press Tab, then full prompt is expanded
- Given multiple matching aliases, when typing, then all matches are shown

### P3: Prompt Organization

**Story**: As a user with many prompts, I want to organize them into categories so that I can find them easily.

**Priority Rationale**: Organization becomes important as prompt library grows.

**Testability**: Create categories, assign prompts, filter by category.

**Acceptance Scenarios**:
- Given prompt, when I assign category, then it appears in that category
- Given category filter, when selected, then only matching prompts are shown
- Given prompt search, when typing, then prompts are filtered by name and content

---

## Edge Cases

- Very long prompts (truncation in list view, full display on select)
- Duplicate alias names (error or auto-suffix)
- Invalid characters in alias (sanitization)
- Migration from file-based storage to ACE
- Concurrent edits from multiple sessions

---

## Requirements

### Functional Requirements

- **FR1**: Implement prompt save dialog in TUI (name, alias, category, content)
- **FR2**: Implement prompt list view with search and filtering
- **FR3**: Implement alias autocomplete in input field (triggered by "/" prefix)
- **FR4**: Persist prompts to ACE or file-based storage
- **FR5**: Support prompt import/export for sharing
- **FR6**: Implement prompt edit and delete operations

### Non-Functional Requirements

- **Performance**: Autocomplete suggestions <50ms
- **Usability**: Keyboard-navigable prompt list
- **Storage**: Support hundreds of saved prompts without performance degradation
- **Migration**: Provide path from file storage to ACE

---

## Success Criteria

- Prompts can be saved with name and alias
- Saved prompts persist across TUI restarts
- Autocomplete works for aliases
- Prompt list is searchable and filterable
- Integration with ACE documented (if implemented)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-tui
./target/debug/codex-tui

# In TUI:
# 1. Type complex prompt
# 2. Save with /save-prompt command
# 3. Restart TUI
# 4. Verify prompt available via autocomplete
```

---

## Dependencies

- TUI bottom_pane module (existing)
- ACE for persistence (optional, can use file storage initially)
- Slash command infrastructure (existing)

---

## Notes

- 6-10h estimated - significant TUI integration work
- Upstream has `custom_prompt_view.rs` but fork's TUI structure differs
- Consider ACE integration for prompt versioning and sharing
- Fork has existing slash command system - prompts could be implemented as dynamic slash commands
- May want to support prompt templates with variable substitution
