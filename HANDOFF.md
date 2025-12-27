# Session 32 Prompt - Async Stage0 Implementation

**Last updated:** 2025-12-26
**Status:** Session 31 Complete - Blocking Audit Done, Design Ready
**Current SPEC:** SPEC-DOGFOOD-001

---

## Session 31 Summary

### Completed
1. **TUI Blocking Audit** - Full audit of all blocking patterns in `tui/src/`
   - Created `docs/SPEC-DOGFOOD-001/evidence/TUI_BLOCKING_AUDIT.md`
   - Identified 3 critical `.join()` calls, 30+ `block_on` patterns
   - Root cause: `pipeline_coordinator.rs:285` blocks TUI during Stage0

2. **Async Stage0 Design** - Architecture document for S32 implementation
   - Created `docs/SPEC-DOGFOOD-001/evidence/ASYNC_STAGE0_DESIGN.md`
   - 6-phase implementation plan
   - Reuses existing `Stage0PendingOperation` infrastructure

3. **Build Preparation** - TUI built and ready for Tier2 validation

### Pending (Requires Manual Testing)
- **Tier2 Validation (A2, A3)** - User needs to run `/speckit.auto SPEC-DOGFOOD-001` in TUI
- **System Pointer (A4)** - Validate with `lm search "SPEC-DOGFOOD-001"`

---

## Session 32 Scope

| Task | Priority | Effort |
|------|----------|--------|
| Implement async Stage0 (6 phases) | HIGH | 2.5 hours |
| Manual testing of async behavior | HIGH | 30 min |
| Verify Tier2 still works after refactor | MEDIUM | 15 min |

---

## Session 32 Tasks

### 1. Validate Tier2 (If Not Done in S31)

```bash
rm -f /tmp/speckit-trace.log
~/code/build-fast.sh run
# In TUI: /speckit.auto SPEC-DOGFOOD-001
# After:
cat /tmp/speckit-trace.log
cat docs/SPEC-DOGFOOD-001/evidence/DIVINE_TRUTH.md | head -50
lm search "SPEC-DOGFOOD-001" --limit 5
```

### 2. Implement Async Stage0

Follow `docs/SPEC-DOGFOOD-001/evidence/ASYNC_STAGE0_DESIGN.md`:

**Phase 1:** Add `Stage0Pending` to `SpecAutoPhase` enum
- File: `tui/src/chatwidget/spec_kit/state.rs:276`

**Phase 2:** Add `stage0_pending` field to `ChatWidget`
- File: `tui/src/chatwidget/mod.rs`

**Phase 3:** Modify `handle_spec_auto` to use `spawn_stage0_async`
- File: `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs:274`

**Phase 4:** Add polling in `on_commit_tick`
- File: `tui/src/chatwidget/mod.rs`

**Phase 5:** Add helper methods (`update_stage0_status`, `continue_pipeline_after_stage0`)
- File: `tui/src/chatwidget/mod.rs`

**Phase 6:** Update status bar to show Stage0 progress
- File: Status bar rendering code

### 3. Testing

| Test | Expected Result |
|------|-----------------|
| Start `/speckit.auto` | TUI remains responsive |
| During Stage0 | Status bar shows progress |
| Scroll history | Immediate response |
| Wait for completion | Pipeline continues |
| Ctrl+C during Stage0 | Graceful cancellation |

---

## Key Files

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/spec_kit/state.rs` | SpecAutoPhase enum |
| `tui/src/chatwidget/spec_kit/stage0_integration.rs` | Existing async infrastructure |
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Pipeline entry point |
| `tui/src/chatwidget/mod.rs` | ChatWidget, on_commit_tick |

---

## Existing Async Infrastructure (Ready to Use)

```rust
// stage0_integration.rs:50-64
pub struct Stage0PendingOperation {
    pub progress_rx: mpsc::Receiver<Stage0Progress>,
    pub result_rx: mpsc::Receiver<Stage0ExecutionResult>,
    pub spec_id: String,
    pub spec_content: String,
    pub config: Stage0ExecutionConfig,
}

// stage0_integration.rs:66-101
pub fn spawn_stage0_async(...) -> Stage0PendingOperation
```

---

## Acceptance Criteria Status

### SPEC-DOGFOOD-001
| ID | Criterion | Status | Notes |
|----|-----------|--------|-------|
| A0 | No Surprise Fan-Out | ✅ PASS | Verified S25 |
| A1 | Doctor Ready | ✅ PASS | Verified S25 |
| A2 | Tier2 Used | ❓ TEST | Needs manual validation |
| A3 | Evidence Exists | ⚠️ PARTIAL | Needs real content verification |
| A4 | System Pointer | ❓ TEST | Needs manual validation |
| A5 | GR-001 Enforcement | ✅ PASS | Verified S25 |
| A6 | Slash Dispatch Single-Shot | ✅ PASS | Verified S25 |

### UX Improvements
| ID | Criterion | Status | Session |
|----|-----------|--------|---------|
| UX1 | Blocking audit complete | ✅ PASS | S31 |
| UX2 | Design doc for async Stage0 | ✅ PASS | S31 |
| UX3 | Stage0Pending phase implemented | ❌ PENDING | S32 |

---

## Configuration Reference

**Stage0 config:** `~/.config/code/stage0.toml`
```toml
[tier2]
enabled = true
notebook = "code-project-docs"
base_url = "http://127.0.0.1:3456"
cache_ttl_hours = 24
```

---

## Constraints
- Fix inside `codex-rs/` only
- Do NOT modify `localmemory-policy` or `notebooklm-mcp`
- Keep file-based tracing until Tier2 validated

---

## Session 32 Checklist

```
[ ] 1. Validate Tier2 if not done in S31
[ ] 2. Implement Phase 1: Add Stage0Pending to enum
[ ] 3. Implement Phase 2: Add stage0_pending field
[ ] 4. Implement Phase 3: Modify handle_spec_auto
[ ] 5. Implement Phase 4: Add polling in on_commit_tick
[ ] 6. Implement Phase 5: Add helper methods
[ ] 7. Implement Phase 6: Update status bar
[ ] 8. Build and test async behavior
[ ] 9. Verify Tier2 still works
[ ] 10. Update HANDOFF.md for S33
```

---

## Quick Start for Session 32

```bash
# Read the design doc first
cat docs/SPEC-DOGFOOD-001/evidence/ASYNC_STAGE0_DESIGN.md

# Start implementation
code codex-rs/tui/src/chatwidget/spec_kit/state.rs  # Phase 1
```
