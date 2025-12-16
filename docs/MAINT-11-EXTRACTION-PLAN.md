# MAINT-11: ChatWidget Extraction Plan

**Status**: IN PROGRESS
**Goal**: Reduce `chatwidget/mod.rs` from 23,413 LOC to <15,000 LOC
**Current**: 20,350 LOC (-3,063 cumulative, -13.1%)

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

**Subtotal**: -3,063 LOC

---

## Planned Phases

| Phase | Target Module | Est. LOC | Priority | Dependencies |
|-------|---------------|----------|----------|--------------|
| 8 | `session_handlers.rs` | ~800 | **P119** | None |
| 9 | `agents_terminal.rs` | ~300 | P120 | None |
| 10 | `history_handlers.rs` | ~600 | P121 | None |
| 11 | `event_handlers.rs` | ~1,000 | P122+ | Phases 8-10 |

**Projected**: Additional -2,700 LOC → mod.rs ~17,650 LOC

---

## Phase 7: Review/Merge Handlers (P118) ✅ COMPLETE

### Scope
Extracted PR review and code review functionality into `review_handlers.rs`.

### Functions Extracted
```rust
// mod.rs → review_handlers.rs (462 LOC with tests)
pub(crate) fn open_review_dialog(&mut self)
pub(crate) fn show_review_custom_prompt(&mut self)
pub(crate) fn show_review_commit_loading(&mut self)
pub(crate) fn present_review_commit_picker(&mut self, commits: Vec<CommitLogEntry>)
pub(crate) fn show_review_branch_loading(&mut self)
pub(crate) fn present_review_branch_picker(&mut self, current_branch: Option<String>, branches: Vec<String>)
pub(crate) fn handle_review_command(&mut self, args: String)
pub(crate) fn start_review_with_scope(&mut self, prompt: String, hint: String, ...)
pub(crate) fn is_review_flow_active(&self) -> bool
pub(crate) fn build_review_summary_cell(&self, hint: Option<&str>, ...) -> AssistantMarkdownCell
```

### Events Involved
- Event handlers remain in mod.rs (call extracted methods)
- `AppEvent::RunReviewCommand`
- `AppEvent::StartReviewCommitPicker`
- `AppEvent::StartReviewBranchPicker`

### Results
- mod.rs: 20,758 → 20,350 LOC (-408 LOC)
- review_handlers.rs: 462 LOC (includes 2 tests)
- All TUI tests pass, clippy clean

---

## Phase 8: Session Handlers (P119)

### Scope
Extract session save/load/resume functionality.

### Functions to Extract
```rust
pub(crate) fn save_session(&self)
pub(crate) fn load_session(&mut self, path: PathBuf)
pub(crate) fn resume_session(&mut self, rollout: PathBuf)
fn session_path(&self) -> PathBuf
fn serialize_session(&self) -> SessionData
fn deserialize_session(data: SessionData) -> Self
```

---

## Phase 9: Agents Terminal (P120)

### Scope
Extract `AgentsTerminalState` and related handlers.

### Types to Extract
```rust
struct AgentsTerminalState
enum AgentsTerminalFocus
fn toggle_agents_hud(&mut self)
fn render_agents_terminal(&self, ...)
```

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
| mod.rs LOC | 23,413 | 20,350 | <15,000 | 36% |
| Extracted modules | 0 | 6 | 10+ | 60% |
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

_Last Updated: 2025-12-16 (P118 complete - review_handlers.rs extracted)_
