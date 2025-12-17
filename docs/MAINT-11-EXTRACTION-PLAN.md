# MAINT-11: ChatWidget Extraction Plan

**Status**: IN PROGRESS
**Goal**: Reduce `chatwidget/mod.rs` from 23,413 LOC to <15,000 LOC
**Current**: 19,073 LOC (-4,340 cumulative, -18.5%)

---

## Completed Phases

| Phase | Session | Module Created | LOC Extracted | Notes |
|-------|---------|----------------|---------------|-------|
| 1 | P110 | `command_render.rs` | ~200 | 8 tests, command output rendering |
| 2 | P113 | `agent_status.rs` | ~65 | 3 tests, agent status types/helpers |
| 3 | P114 | `submit_helpers.rs` | ~300 | 4 tests, message submission logic |
| 4 | P115 | (cleanup) | ~5 | Dead code removal, 8 warnings fixed |
| 5 | P116 | `input_helpers.rs` | ~54 (+175 new) | 5 tests, input normalization |
| 6 | P117 | (removal) | ~2,094 | Browser/chrome dead code deletion |
| 7 | P118 | `review_handlers.rs` | ~408 | 2 tests, review/code review functions |
| 8 | P119 | `session_handlers.rs` | ~558 | 6 tests, session save/load/resume |
| 9 | P120 | `agents_terminal.rs` | ~719 | Agents terminal overlay |

**Subtotal**: -4,340 LOC

---

## Planned Phases

| Phase | Target Module | Est. LOC | Priority | Dependencies |
|-------|---------------|----------|----------|--------------|
| 10 | `history_handlers.rs` | ~600 | P121 | None |
| 11 | `event_handlers.rs` | ~1,000 | P122+ | Phases 8-10 |

**Projected**: Additional -1,600 LOC → mod.rs ~17,500 LOC

---

## Phase 8: Session Handlers (P119) ✅ COMPLETE

### Scope
Extracted session save/load/resume functionality into `session_handlers.rs`.

### Functions Extracted
```rust
// mod.rs → session_handlers.rs (624 LOC with tests)
pub(crate) fn human_ago(ts: SystemTime) -> String
pub(crate) fn list_cli_sessions_impl(&self) -> impl Future
pub(crate) fn kill_cli_session_impl(&self, uuid: &str) -> impl Future
pub(crate) fn kill_all_cli_sessions_impl(&self) -> impl Future
pub(crate) fn handle_sessions_command(&mut self, args: String)
pub(crate) fn show_resume_picker(&mut self)
pub(crate) fn render_replay_item(&mut self, ...) -> ResponseItems
pub(crate) fn export_response_items(&self, ...) -> Vec<ResponseItem>
pub(crate) fn handle_feedback_command(&self, args: String)
pub(crate) fn export_transcript_lines_for_buffer(&self) -> Vec<String>
pub(crate) fn render_lines_for_terminal(&self, ...) -> String
```

### Results
- mod.rs: 20,350 → 19,792 LOC (-558 LOC)
- session_handlers.rs: 624 LOC (includes 6 tests)
- All TUI tests pass, clippy clean

---

## Phase 9: Agents Terminal (P120) ✅ COMPLETE

### Scope
Extracted agents terminal overlay state, types, and rendering into `agents_terminal.rs`.

### Types Extracted
```rust
// mod.rs → agents_terminal.rs (759 LOC)
pub(crate) struct AgentTerminalEntry
pub(crate) struct AgentsTerminalState
pub(crate) enum AgentsTerminalFocus
```

### Functions Extracted
```rust
pub(crate) fn update_agents_terminal_state(&mut self, ...)
pub(crate) fn enter_agents_terminal_mode(&mut self)
pub(crate) fn exit_agents_terminal_mode(&mut self)
pub(crate) fn toggle_agents_hud(&mut self)
pub(crate) fn record_current_agent_scroll(&mut self)
pub(crate) fn restore_selected_agent_scroll(&mut self)
pub(crate) fn navigate_agents_terminal_selection(&mut self, delta: isize)
pub(crate) fn render_agents_terminal_overlay(&self, ...)
```

### Results
- mod.rs: 19,792 → 19,073 LOC (-719 LOC)
- agents_terminal.rs: 759 LOC
- All TUI tests pass (541/544, 3 pre-existing spec-kit failures), clippy clean

---

## Phase 10: History Handlers (P121)

### Scope
Extract history cell management.

### Functions to Extract
```rust
fn history_push(&mut self, cell: Box<dyn HistoryCell>)
fn history_replace(&mut self, idx: usize, cell: Box<dyn HistoryCell>)
fn history_replace_and_maybe_merge(&mut self, ...)
fn history_maybe_merge_tool_with_previous(&mut self, ...)
fn history_clear(&mut self)
fn history_truncate(&mut self, len: usize)
```

---

## Extraction Pattern

### Step 1: Investigation
```bash
# Find all references to target functionality
grep -n "function_name\|TypeName" tui/src/chatwidget/mod.rs

# Check for external callers
grep -rn "function_name" tui/src/ --include="*.rs" | grep -v mod.rs
```

### Step 2: Create Module
```rust
// chatwidget/new_module.rs
use super::*;  // Start with glob, refine later

impl ChatWidget<'_> {
    pub(crate) fn extracted_function(&mut self) {
        // Move implementation here
    }
}
```

### Step 3: Update mod.rs
```rust
// Add to mod.rs
mod new_module;
pub(crate) use new_module::*;  // Re-export if needed
```

### Step 4: Clean Up
- Remove moved code from mod.rs
- Refine imports (remove glob)
- Add tests
- Run clippy

---

## Metrics

| Metric | Start | Current | Target | Progress |
|--------|-------|---------|--------|----------|
| mod.rs LOC | 23,413 | 19,073 | <15,000 | 52% |
| Extracted modules | 0 | 8 | 10+ | 80% |
| Test coverage | N/A | Passing | Passing | ✅ |
| Clippy warnings | 8 | 0 | 0 | ✅ |

---

## Risk Mitigation

### Circular Imports
- Keep `use super::*` initially
- Refine imports incrementally
- Use `pub(crate)` re-exports

### Breaking Changes
- Extract implementation, not API
- Maintain same function signatures
- Use forwarding methods if needed

### Test Breakage
- Run tests after each function move
- Add integration tests for extracted modules
- Maintain snapshot tests

---

_Last Updated: 2025-12-16 (P120 complete - agents_terminal.rs extracted)_
