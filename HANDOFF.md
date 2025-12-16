# P119 Session Handoff
Date: 2025-12-16
Scope: ChatWidget refactor (MAINT-11) - Post review handlers extraction

---

## 1. Executive Summary

Planner is a Rust workspace building the `code` binary with TUI focused on Spec-Kit workflows.

**P118 Completed**:
- Extracted review handlers from mod.rs into `review_handlers.rs` (462 LOC)
- 10 functions extracted: open_review_dialog, show_review_custom_prompt, etc.
- 2 tests added for review context metadata and request construction
- mod.rs: 20,758 → 20,350 LOC (-408 LOC)
- All TUI tests pass, clippy clean

**Commit**: (to be created after handoff update)

---

## 2. P119 Primary Task: MAINT-11 Phase 8 (Session Handlers)

### Goal
Extract session save/load/resume functionality from `mod.rs` into `session_handlers.rs`.

### Current State
- `mod.rs`: 20,350 LOC
- **Phase 7 (P118)**: `review_handlers.rs` (~408 LOC extracted, 462 LOC total with tests)

### Investigation Steps
```bash
cd codex-rs

# Find session-related functions
grep -n "fn.*session\|Session\|session_" tui/src/chatwidget/mod.rs | head -40

# Find save/load/resume functions
grep -n "save_session\|load_session\|resume_session\|rollout" tui/src/chatwidget/mod.rs | head -20

# Check session data structures
grep -n "SessionData\|session_path" tui/src/chatwidget/mod.rs
```

### Expected Extraction Targets
```rust
// Functions to potentially extract
pub(crate) fn save_session(&self)
pub(crate) fn load_session(&mut self, path: PathBuf)
pub(crate) fn resume_session(&mut self, rollout: PathBuf)
fn session_path(&self) -> PathBuf
fn serialize_session(&self) -> SessionData
fn deserialize_session(data: SessionData) -> Self
```

---

## 3. Architecture Context

### ChatWidget Module Structure (Post-P118)
```
chatwidget/
├── agent_install.rs      (24KB)
├── agent_status.rs       (5KB)  ← P113
├── agent.rs              (3KB)
├── command_render.rs     (10KB) ← P110
├── diff_handlers.rs      (6KB)
├── exec_tools.rs         (29KB)
├── gh_actions.rs         (10KB)
├── history_render.rs     (5KB)
├── input_helpers.rs      (6KB)  ← P116
├── interrupts.rs         (7KB)
├── layout_scroll.rs      (8KB)
├── limits_handlers.rs    (3KB)
├── limits_overlay.rs     (7KB)
├── perf.rs               (6KB)
├── rate_limit_refresh.rs (4KB)
├── review_handlers.rs    (19KB) ← P118 (NEW)
├── submit_helpers.rs     (11KB) ← P114
└── mod.rs                (830KB) ← 20,350 LOC
```

---

## 4. Verification Commands

```bash
# Full test suite
cd codex-rs && cargo test --workspace

# TUI-specific tests
cargo test -p codex-tui

# Clippy (should show 0 warnings)
cargo clippy -p codex-tui -- -D warnings

# Build
~/code/build-fast.sh
```

---

## 5. P118 Extraction Summary

| Component | LOC |
|-----------|-----|
| Functions extracted | 10 |
| Tests added | 2 |
| review_handlers.rs total | 462 |
| mod.rs reduction | -408 |

Extracted functions:
- `open_review_dialog()` - Show review options modal
- `show_review_custom_prompt()` - Custom prompt input
- `show_review_commit_loading()` - Commit loading indicator
- `present_review_commit_picker()` - Commit selection UI
- `show_review_branch_loading()` - Branch loading indicator
- `present_review_branch_picker()` - Branch selection UI
- `handle_review_command()` - Process /review command
- `start_review_with_scope()` - Core review submission
- `is_review_flow_active()` - Check review flow state
- `build_review_summary_cell()` - Build summary cell for history

---

## 6. Key File References

| Component | File | Lines |
|-----------|------|-------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 20,350 |
| Review Handlers | `tui/src/chatwidget/review_handlers.rs` | 462 |
| MAINT-11 Plan | `docs/MAINT-11-EXTRACTION-PLAN.md` | - |
| MAINT-11 Tracker | `SPEC.md:186` | - |

---

## 7. Session Summary

| Session | Commit | Key Deliverable |
|---------|--------|-----------------|
| P110 | - | command_render.rs extraction |
| P111 | 424990cc3 | MCP timeout_sec + per-model providers |
| P112 | c521f9c36 | Regression fix + HANDOFF.md |
| P113 | 09f78f6c9 | agent_status.rs + stage0 clippy |
| P114 | 83ae857d1 | submit_helpers.rs + core clippy |
| P115 | e82064d50 | dead_code cleanup (8→0 warnings) |
| P116 | d5c58634c | input_helpers.rs extraction |
| P117 | 15aa783a7 | Browser/chrome dead code removal |
| P118 | — | review_handlers.rs extraction |
| P119 | — | next: session_handlers.rs |

---

## 8. MAINT-11 Progress Summary

| Phase | Session | LOC Change | Total mod.rs |
|-------|---------|------------|--------------|
| 1 | P110 | -200 extracted | 23,213 |
| 2 | P113 | -65 extracted | 23,151 |
| 3 | P114 | -300 extracted | 22,911 |
| 4 | P115 | -5 removed | 22,906 |
| 5 | P116 | -54 extracted | 22,852 |
| 6 | P117 | -2,094 removed | 20,758 |
| 7 | P118 | -408 extracted | 20,350 |

**Cumulative**: 23,413 → 20,350 = **-3,063 LOC** (-13.1%)

---

## 9. Remaining Extraction Candidates

| Target | Est. LOC | Complexity | Session |
|--------|----------|------------|---------|
| Session handlers | ~800 | Medium | P119 |
| Agents terminal | ~300 | Low | P120 |
| History handlers | ~600 | Medium | P121 |
| Event handlers | ~1,000 | High | P122+ |

---

_Generated: 2025-12-16 after P118 review_handlers.rs extraction_
