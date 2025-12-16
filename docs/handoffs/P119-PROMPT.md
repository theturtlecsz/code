# P119 Session Prompt

```
load HANDOFF.md **ultrathink**
```

Continue P119: MAINT-11 Phase 8 (Session handlers extraction) + test fix + architecture diagram.

## Context
- P118 completed: review_handlers.rs extraction (-408 LOC)
- Commit: `5584713cb`
- mod.rs: 20,350 LOC remaining
- Cumulative MAINT-11: -3,063 LOC (-13.1%)

---

## Primary Task: MAINT-11 Phase 8 - Session Handlers

### Goal
Extract session save/load/resume functionality from `mod.rs` into `session_handlers.rs`.

### Phase 1: Investigation
Analyze session code boundaries:
```bash
cd codex-rs

# Find session-related functions
grep -n "fn.*session\|Session\|session_" tui/src/chatwidget/mod.rs | head -50

# Find save/load/resume functions
grep -n "save_session\|load_session\|resume\|rollout" tui/src/chatwidget/mod.rs | head -30

# Find session data structures
grep -n "SessionData\|RolloutPath\|session_path" tui/src/chatwidget/mod.rs | head -20

# Check for session-related imports
grep -n "use.*session\|use.*rollout" tui/src/chatwidget/mod.rs
```

### Phase 2: Define Module Boundaries
Expected `session_handlers.rs` contents (~800 LOC target):
- Session save/load functions
- Rollout path management
- Session resume logic
- Session data serialization
- Related helper functions

### Phase 3: Staged Extraction
1. Create `chatwidget/session_handlers.rs` with proper imports
2. Move functions one at a time, updating imports
3. Add `pub(crate)` visibility where needed
4. Update callers in mod.rs and app.rs
5. Add comprehensive tests

### Phase 4: Verification
```bash
cargo test -p codex-tui
cargo clippy -p codex-tui -- -D warnings
cargo test --workspace
```

---

## Secondary Task: Fix Pre-existing Test Failure

### Issue
`suite::exec_stream_events::test_aggregated_output_interleaves_in_order` fails in codex-core:
```
assertion `left == right` failed
  left: "O1\nO2\nE1\nE2\n"
 right: "O1\nE1\nO2\nE2\n"
```

### Investigation Steps
```bash
cd codex-rs

# Find the failing test
grep -rn "test_aggregated_output_interleaves_in_order" core/

# Read the test file
cat core/tests/suite/exec_stream_events.rs

# Check exec stream implementation
grep -rn "ExecStream\|aggregated_output" core/src/
```

### Expected Resolution
- Understand the expected vs actual behavior
- Determine if test expectation is wrong or implementation has a bug
- Fix the root cause (test or implementation)
- Ensure fix doesn't break other tests

---

## Tertiary Task: Architecture Diagram

### Goal
Create a Mermaid diagram showing chatwidget module structure.

### Diagram Contents
1. Module hierarchy (mod.rs + extracted modules)
2. Key dependencies between modules
3. External callers (app.rs)
4. Data flow for common operations

### Output Location
`docs/architecture/chatwidget-structure.md`

---

## Deliverables Checklist

### Required
- [ ] `session_handlers.rs` module created (~800 LOC)
- [ ] mod.rs reduced by ~800 LOC
- [ ] All TUI tests pass
- [ ] Clippy clean (0 warnings)
- [ ] MAINT-11-EXTRACTION-PLAN.md updated
- [ ] HANDOFF.md updated for P120

### Enhanced Testing
- [ ] Unit tests for session_handlers.rs
- [ ] Integration test for session save/load cycle
- [ ] Test for rollout path validation

### Test Fix
- [ ] `test_aggregated_output_interleaves_in_order` investigated
- [ ] Root cause identified and documented
- [ ] Fix implemented and verified
- [ ] No regressions in workspace tests

### Architecture
- [ ] Mermaid diagram created
- [ ] Module relationships documented
- [ ] Key data flows illustrated

---

## Expected Outcomes

| Deliverable | Target |
|-------------|--------|
| `session_handlers.rs` | ~800 LOC extracted |
| mod.rs reduction | 20,350 → ~19,550 LOC |
| Tests | All passing (including fixed test) |
| Clippy warnings | 0 |
| Architecture doc | Created with diagram |
| MAINT-11 progress | 36% → ~42% |

---

## Safety Checks

- [ ] Search for ALL usages before moving any function
- [ ] Check for trait implementations that depend on moved code
- [ ] Verify no circular imports after extraction
- [ ] Run clippy to catch dead code and unused imports
- [ ] All TUI tests pass
- [ ] Full workspace tests pass (including the fixed test)

---

## File References

| Component | File | Current Lines |
|-----------|------|---------------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 20,350 |
| Review handlers | `tui/src/chatwidget/review_handlers.rs` | 462 |
| Failing test | `core/tests/suite/exec_stream_events.rs` | ~100 |
| MAINT-11 Plan | `docs/MAINT-11-EXTRACTION-PLAN.md` | - |
| Extraction Guide | `docs/EXTRACTION-GUIDE.md` | - |

---

## Remaining Work Queue (Post-P119)

| Session | Target | Est. LOC | Complexity |
|---------|--------|----------|------------|
| P120 | agents_terminal.rs | ~300 | Low |
| P121 | history_handlers.rs | ~600 | Medium |
| P122+ | event_handlers.rs | ~1,000 | High |

---

_Generated: 2025-12-16 for P119 session_
