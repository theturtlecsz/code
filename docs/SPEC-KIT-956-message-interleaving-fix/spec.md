# SPEC-KIT-956: Message Interleaving Fix

**Created**: 2025-11-25
**Priority**: P2 - UX Bug
**Status**: Backlog
**Branch**: TBD

---

## Problem Statement

Messages are being inserted in incorrect order in the chat history. Responses appear above their corresponding questions instead of below.

### Symptoms
- User sends a message
- Response appears ABOVE the question in chat history
- Visual ordering does not match conversation flow

### Impact
- Confusing UX - conversation reads out of order
- Makes multi-turn conversations hard to follow
- Breaks expected chat UI paradigm

---

## Context

### Related Work
- **SPEC-954**: Session management polish (21 message_ordering tests added)
- **SPEC-947**: Multi-provider validation (where bug was observed)

### Test Status
- 21/21 `message_ordering` tests pass
- Bug manifests in live TUI but not in unit tests
- Suggests issue is in rendering/display layer, not message storage

---

## Investigation Areas

### Suspected Components
1. `tui/src/chatwidget/mod.rs` - History cell insertion logic
2. `tui/src/history_cell/` - Cell ordering and timestamps
3. `tui/src/app.rs` - Event handling for message display
4. Async timing between prompt submission and response arrival

### Key Questions
- Is the issue in message storage or just rendering?
- Does it affect all providers or specific ones?
- Is it timing-related (race condition)?

---

## Acceptance Criteria

- [ ] Root cause identified
- [ ] Fix implemented
- [ ] Manual validation: responses appear below questions
- [ ] No regression in message_ordering tests
- [ ] Works across all providers (Claude, ChatGPT, Gemini)

---

## Notes

Created during SPEC-947 Phase 2 validation. Deferred to focus on model preset updates and provider validation.
