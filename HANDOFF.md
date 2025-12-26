# Session 31 Prompt - TUI Blocking UX Audit + Tier2 Validation

**Last updated:** 2025-12-26
**Status:** Session 30 Complete - Tier2 Fix Applied, UX Issue Identified
**Current SPEC:** SPEC-DOGFOOD-001

---

## Session 31 Scope (Decided)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Async Stage0 | **Audit + Design only** | Defer implementation to S32 |
| Blocking audit | **Full TUI audit** | All blocking patterns in `tui/src/` |
| Tier2 testing | **Manual validation** | No integration tests needed yet |

---

## Session 30 Summary

### What Was Fixed
1. **Root cause**: `stage0.toml` had notebook UUID instead of name - fixed to `code-project-docs`
2. **Runtime conflicts**: Multiple tokio runtime conflicts resolved:
   - `reqwest::blocking` creates its own tokio runtime
   - Stage0 runs inside TUI's tokio runtime
   - Solution: Tier2 HTTP runs in isolated `std::thread` inside `generate_divine_truth`
   - Outer wrapper: Stage0 itself runs in `std::thread::spawn` with immediate `.join()`

### Commits (Session 30)
```
42a627a16 - Add Tier2 decision tracing + fix notebook config
7560eead8 - Use blocking HTTP client + add progress feedback (attempt)
cafb1202a - Remove block_in_place - use blocking HTTP directly
88fa8a667 - Spawn separate thread for Tier2 HTTP to isolate runtimes
ada050090 - Simplify Stage0 to sync execution
b86435bd3 - Wrap Stage0 in std::thread to avoid nested runtime
```

### Major UX Issue Discovered
**TUI freezes during Stage0 execution** (~15-30 seconds for Tier2 query):
- `history_push` adds messages but TUI only redraws after event handlers return
- When `handle_spec_auto` blocks waiting for Stage0, no redraws occur
- User sees frozen TUI with no feedback

---

## Session 31 Tasks

### 1. Validate Tier2 Fix (Priority)
```bash
rm -f /tmp/speckit-trace.log
~/code/build-fast.sh run
# In TUI: /speckit.auto SPEC-DOGFOOD-001
# After completion:
cat /tmp/speckit-trace.log
```

**Expected trace entries:**
```
[HH:MM:SS] Tier2 THREAD START: spec_id=SPEC-DOGFOOD-001, url=.../api/ask
[HH:MM:SS] Tier2 THREAD SUCCESS: answer_len=XXXX  <-- Should be >1000
```

**Verify DIVINE_TRUTH.md:**
```bash
cat docs/SPEC-DOGFOOD-001/evidence/DIVINE_TRUTH.md | head -50
```
Should contain real NotebookLM synthesis, NOT "Tier2 unavailable" fallback.

### 2. Validate System Pointer (A4)
```bash
lm search "SPEC-DOGFOOD-001" --limit 5
```
Should show a `system:true` pointer entry.

### 3. TUI Blocking UX Audit (Major) — Full Scope

**Scope:** Identify ALL blocking patterns in `tui/src/` that freeze the interface.

**Search commands:**
```bash
cd codex-rs
grep -rn "\.join()" tui/src/ | grep -v test
grep -rn "block_on" tui/src/
grep -rn "recv()" tui/src/ | grep -v test
grep -rn "thread::sleep" tui/src/
grep -rn "\.await" tui/src/ | head -50  # Verify async patterns
```

**Known blocking areas to audit:**
| Area | File | Potential Blocking | Priority |
|------|------|-------------------|----------|
| Stage0 execution | `pipeline_coordinator.rs:274` | `handle.join()` blocks TUI | HIGH |
| Consensus checking | `consensus_coordinator.rs` | May have blocking queries | MEDIUM |
| Tier2 health check | `stage0_integration.rs` | HTTP call before pipeline | HIGH |
| Quality gate execution | `quality_gate_handler.rs` | Agent waits | MEDIUM |
| MCP queries | Various | May block on responses | LOW |
| File I/O | Various | Synchronous reads/writes | LOW |

**Deliverable:** Create `docs/SPEC-DOGFOOD-001/evidence/TUI_BLOCKING_AUDIT.md` with findings.

### 4. Design Async Stage0 State Machine

**Required architecture:**
1. Add `Stage0Pending { status: String }` phase to `SpecAutoPhase` enum
2. Add `stage0_pending: Option<Stage0PendingOperation>` field to ChatWidget
3. Modify `handle_spec_auto` to spawn Stage0 and return immediately
4. Add polling in `on_commit_tick` to check progress/completion
5. When complete, continue pipeline from where it left off

**Key files:**
- `tui/src/chatwidget/spec_kit/state.rs` - SpecAutoPhase enum
- `tui/src/chatwidget/spec_kit/stage0_integration.rs` - Stage0PendingOperation (already added but unused)
- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` - handle_spec_auto
- `tui/src/chatwidget/mod.rs` - on_commit_tick, widget fields

**Note:** `Stage0PendingOperation` and `spawn_stage0_async` are already implemented in `stage0_integration.rs` but not wired up.

---

## Acceptance Criteria Status

### SPEC-DOGFOOD-001
| ID | Criterion | Status | Notes |
|----|-----------|--------|-------|
| A0 | No Surprise Fan-Out | ✅ PASS | Verified S25 |
| A1 | Doctor Ready | ✅ PASS | Verified S25 |
| A2 | Tier2 Used | ❓ TEST | Config fixed, runtime fixed, needs validation |
| A3 | Evidence Exists | ⚠️ PARTIAL | Files exist, need real content |
| A4 | System Pointer | ❓ TEST | Code in place, needs validation |
| A5 | GR-001 Enforcement | ✅ PASS | Verified S25 |
| A6 | Slash Dispatch Single-Shot | ✅ PASS | Verified S25 |

### UX Improvements (New)
| ID | Criterion | Status | Session |
|----|-----------|--------|---------|
| UX1 | Blocking audit complete | ❌ PENDING | S31 |
| UX2 | Design doc for async Stage0 | ❌ PENDING | S31 |
| UX3 | Stage0Pending phase implemented | ❌ DEFERRED | S32 |

---

## Configuration Reference

**Stage0 config:** `~/.config/code/stage0.toml`
```toml
[tier2]
enabled = true
notebook = "code-project-docs"  # <-- FIXED from UUID
base_url = "http://127.0.0.1:3456"
cache_ttl_hours = 24
```

---

## Constraints
- Fix inside `codex-rs/` only
- Do NOT modify `localmemory-policy` or `notebooklm-mcp`
- Keep file-based tracing until Tier2 validated

---

## Session 31 Checklist

Copy this to track progress:

```
[ ] 1. Clear trace log: rm -f /tmp/speckit-trace.log
[ ] 2. Build and run: ~/code/build-fast.sh run
[ ] 3. Execute: /speckit.auto SPEC-DOGFOOD-001
[ ] 4. Verify trace: cat /tmp/speckit-trace.log (expect Tier2 SUCCESS)
[ ] 5. Verify DIVINE_TRUTH.md has real content (not fallback)
[ ] 6. Validate A4: lm search "SPEC-DOGFOOD-001" (expect system:true)
[ ] 7. Run blocking audit grep commands
[ ] 8. Create TUI_BLOCKING_AUDIT.md with findings
[ ] 9. Create ASYNC_STAGE0_DESIGN.md architecture doc
[ ] 10. Update HANDOFF.md for S32
```

---

## Quick Start for Session 31

```bash
# 1. Validate Tier2 fix first
rm -f /tmp/speckit-trace.log
~/code/build-fast.sh run
# In TUI: /speckit.auto SPEC-DOGFOOD-001
# After: cat /tmp/speckit-trace.log && cat docs/SPEC-DOGFOOD-001/evidence/DIVINE_TRUTH.md | head -50

# 2. If Tier2 works, proceed to audit
cd codex-rs && grep -rn "\.join()" tui/src/ | grep -v test
```
