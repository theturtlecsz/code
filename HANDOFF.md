# Session Handoff — SPEC-DOGFOOD-001 Stage0 Fix

**Last updated:** 2025-12-26
**Status:** Session 28 Complete, Cancel Command Added, Stage0/Tier2 Investigation Pending
**Current SPEC:** SPEC-DOGFOOD-001

> **Goal**: Complete SPEC-DOGFOOD-001 dogfooding validation with working Stage0 + Tier2.

---

## Session Log

| Session | Focus | LOC Changed | Outcome |
|---------|-------|-------------|---------|
| S17 | Dead code audit | -1,500 | Identified unused modules |
| S18 | Native consensus cleanup | -800 | Deleted native_consensus_executor.rs |
| S19 | Config reload removal | -840 | Deleted config_reload.rs, clippy fixes |
| S20 | Test isolation + clippy | -10 | Added #[serial], fixed 5 clippy warnings |
| S21 | Type migration + audit | -50 | Renamed 8 types, fixed 6 clippy, audited dead_code |
| S22 | Clippy + dead_code docs | -20 | Fixed 17 clippy warnings, documented 13 blanket allows |
| S23 | Config fix + module deletion | -664 | Fixed xhigh parse error, deleted unified_exec |
| S24 | Orphaned module cleanup | -1,538 | Deleted 4 orphaned TUI modules, verified A1 |
| S25 | Acceptance validation | 0 | 4/6 criteria validated, Stage0 routing bug found |
| S26 | Stage0 routing debug | +92 | Confirmed routing works, added comprehensive trace |
| S27 | **Stage0 JSON fix** | -73 | **Fixed null results bug, Stage0 now works** |
| S28 | **Cancel command + enum fix** | +95 | Added /speckit.cancel, fixed SlashCommand routing |

**Total deleted (S17-S24):** ~5,422 LOC

---

## Session 28 Summary (Complete)

### Changes Made

1. **Added `/speckit.cancel` command**
   - `commands/cancel.rs`: New command implementation
   - Clears `spec_auto_state` and `spec_auto_metrics`
   - Registered in command registry (41 commands total)
   - Added to native commands list in routing.rs

2. **Fixed SlashCommand enum routing** (Critical Bug)
   - **Problem:** `/speckit.cancel` was sent to LLM instead of registry
   - **Root cause:** Command wasn't in `SlashCommand` enum
   - **Fix:** Added `SpecKitCancel` variant to:
     - `slash_command.rs`: Enum definition + description + is_spec_kit()
     - `app.rs`: Native command redirect list

### Files Changed

| File | Change |
|------|--------|
| `commands/cancel.rs` | NEW - Command implementation |
| `commands/mod.rs` | Added cancel module |
| `command_registry.rs` | Registered command, updated test count |
| `routing.rs` | Added to native commands list |
| `slash_command.rs` | Added `SpecKitCancel` enum variant |
| `app.rs` | Added to registry redirect list |

### Commits

| Hash | Description |
|------|-------------|
| `a39aa7b0f` | feat(spec-kit): Add /speckit.cancel command (SPEC-DOGFOOD-001) |
| (pending) | fix(spec-kit): Add SpecKitCancel to SlashCommand enum for routing |

### Investigation Findings

**Stage0 not regenerating evidence:**
- Pipeline stages 1-6 ran successfully (plan.md through unlock.md updated)
- BUT Stage0 evidence files (TASK_BRIEF.md, DIVINE_TRUTH.md) were NOT regenerated
- No DEBUG output appeared from `handle_spec_auto` function
- DIVINE_TRUTH.md still shows "Tier2 (NotebookLM) was unavailable"

**NotebookLM Status:**
- Service is healthy and authenticated (confirmed via `notebooklm health`)
- Deep health check passes: `curl http://127.0.0.1:3456/health/ready?deep=true`
- Auth fix applied (commit 6ad1259 in notebooklm-mcp)

**Open Question:** Why is Stage0 code path not being executed even though pipeline runs?

### Acceptance Criteria Status

| ID | Criterion | Status | Evidence |
|----|-----------|--------|----------|
| A0 | No Surprise Fan-Out | ✅ PASS | `quality_gate_handler.rs:1075-1088` |
| A1 | Doctor Ready | ✅ PASS | `code doctor` shows all [OK] |
| A2 | Tier2 Used | ❌ BLOCKED | Stage0 not executing, Tier2 not reached |
| A3 | Evidence Exists | ⚠️ PARTIAL | Files exist but stale (not regenerated) |
| A4 | System Pointer | ❌ BLOCKED | No `system:true` tag in search results |
| A5 | GR-001 Enforcement | ✅ PASS | `quality_gate_handler.rs:1206-1238` |
| A6 | Slash Dispatch Single-Shot | ✅ PASS | `quality_gate_handler.rs:28-71` |

**Score: 4/6 validated, 2/6 blocked by Stage0 issue**

---

## Session 29 Plan

### Primary Focus: Stage0 + Tier2 Fix

1. **Verify `/speckit.cancel` works**
   - Restart TUI to pick up new build
   - Run `/speckit.cancel` - should show notice message

2. **Debug Stage0 execution path**
   - Add explicit debug output at the START of `handle_spec_auto`
   - Trace why the DEBUG line at line 41 of pipeline_coordinator.rs isn't appearing
   - Check if command is going through `ProcessedCommand::SpecAuto` path vs registry path

3. **Verify Stage0 runs and regenerates evidence**
   - Run `/speckit.auto SPEC-DOGFOOD-001`
   - Check for Stage0 output messages
   - Verify TASK_BRIEF.md and DIVINE_TRUTH.md are regenerated

4. **Validate A2 and A4**
   - A2: DIVINE_TRUTH.md should show actual Tier2 content (not fallback)
   - A4: `lm search "SPEC-DOGFOOD-001"` should return entry with `system:true`

### Key Debugging Points

```rust
// pipeline_coordinator.rs:41-47 - Should appear on /speckit.auto
"🔍 DEBUG: handle_spec_auto(spec_id={}, stage0_disabled={})"

// pipeline_coordinator.rs:323-331 - Should appear if Stage0 succeeds
"Stage 0: Context compiled (N memories, tier2=..., Xms)"

// pipeline_coordinator.rs:366-372 - Should appear if Stage0 skipped
"Stage 0: Skipped (reason)"
```

### Hypothesis

The `/speckit.auto` command may be going through a different code path that skips `handle_spec_auto` entirely. Possible causes:
1. ProcessedCommand parsing might be failing silently
2. Another command handler might be intercepting
3. State check might be blocking re-entry

---

## Key Files

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs:32-450` | handle_spec_auto + Stage0 execution |
| `tui/src/chatwidget/spec_kit/stage0_integration.rs` | Stage0 + Tier2 integration |
| `tui/src/slash_command.rs:459-468` | ProcessedCommand::SpecAuto parsing |
| `tui/src/chatwidget/mod.rs:4464-4471` | SpecAuto routing |
| `tui/src/app.rs:2023-2044` | Native command registry redirect |

---

## Continuation Prompt

```
Continue SPEC-DOGFOOD-001 - Session 29 **ultrathink**

## Context
Session 28 completed (commit pending):
- ADDED: /speckit.cancel command with full routing fix
- FOUND: Stage0 code path not being executed (no DEBUG output)
- Pipeline stages 1-6 work, but Stage0 evidence not regenerated
- NotebookLM service is healthy and authenticated

## Session 29 Tasks (Priority Order)

### 1. Verify /speckit.cancel Works
Restart TUI and test:
```bash
~/code/build-fast.sh run
# In TUI:
/speckit.cancel
```
Expected: "✓ Pipeline state cleared" or "ℹ No active pipeline"

### 2. Debug Stage0 Execution Path
The DEBUG line at pipeline_coordinator.rs:41-47 should appear but doesn't.
- Add more visible debug at entry point
- Trace through ProcessedCommand::SpecAuto path
- Check if re-entry guard or state check is blocking

### 3. Fix Stage0 and Validate A2/A4
Once Stage0 executes:
- A2: Verify DIVINE_TRUTH.md has actual Tier2 content
- A4: Verify `lm search "SPEC-DOGFOOD-001"` returns system:true entry

## Key Files
- pipeline_coordinator.rs:32-450 (handle_spec_auto)
- slash_command.rs:459-468 (SpecAuto parsing)
- chatwidget/mod.rs:4464-4471 (SpecAuto routing)

## Non-Negotiable Constraints
- Fix must be inside codex-rs only
- Do NOT modify localmemory-policy or notebooklm-mcp
- Keep changes minimal and targeted
```
