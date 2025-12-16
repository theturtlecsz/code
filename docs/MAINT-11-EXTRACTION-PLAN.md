# MAINT-11: ChatWidget Extraction Plan

**Status**: IN PROGRESS
**Goal**: Reduce `chatwidget/mod.rs` from 23,413 LOC to <15,000 LOC
**Current**: 20,758 LOC (-2,655 cumulative, -11.3%)

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

**Subtotal**: -2,655 LOC

---

## Planned Phases

| Phase | Target Module | Est. LOC | Priority | Dependencies |
|-------|---------------|----------|----------|--------------|
| 7 | `review_handlers.rs` | ~500 | **P118** | None |
| 8 | `session_handlers.rs` | ~800 | P119 | None |
| 9 | `agents_terminal.rs` | ~300 | P120 | None |
| 10 | `history_handlers.rs` | ~600 | P121 | None |
| 11 | `event_handlers.rs` | ~1,000 | P122+ | Phases 7-10 |

**Projected**: Additional -3,200 LOC → mod.rs ~17,500 LOC

---

## Phase 7: Review/Merge Handlers (P118)

### Scope
Extract PR review and merge functionality into `review_handlers.rs`.

### Functions to Extract
```rust
// From mod.rs → review_handlers.rs
pub(crate) fn open_review_dialog(&mut self)
fn handle_review_mode_entered(&mut self, request: ReviewRequest)
fn handle_review_mode_exited(&mut self, output: ReviewOutputEvent)
fn start_review_commit_picker(&mut self)
fn start_review_branch_picker(&mut self)
fn handle_review_findings(&mut self, findings: Vec<ReviewFinding>)
```

### Events Involved
- `EventMsg::EnteredReviewMode`
- `EventMsg::ExitedReviewMode`
- `AppEvent::RunReviewCommand`
- `AppEvent::StartReviewCommitPicker`
- `AppEvent::StartReviewBranchPicker`

### Verification Steps
1. `cargo test -p codex-tui`
2. `cargo clippy -p codex-tui -- -D warnings`
3. `cargo test --workspace`
4. Manual: `/review` command still works

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
| mod.rs LOC | 23,413 | 20,758 | <15,000 | 31% |
| Extracted modules | 0 | 5 | 10+ | 50% |
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

_Last Updated: 2025-12-16 (P117 complete)_
