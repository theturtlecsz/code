# Session Handoff — SPEC-DOGFOOD-001 Stage0 Fix

**Last updated:** 2025-12-26
**Status:** Session 29 Complete, Stage0 Executes but Tier2 Falls Back
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
| S29 | **Stage0 trace + Tier2 debug** | +350 | Stage0 executes, Tier2 health passes, but query fails |

**Total deleted (S17-S24):** ~5,422 LOC

---

## Session 29 Summary (Complete)

### Key Discovery

**Stage0 IS executing successfully** - confirmed via file-based tracing:
```
[17:33:50.923] handle_spec_auto ENTRY: spec_id=SPEC-DOGFOOD-001, stage0_disabled=false
[17:33:50.925] Stage0 CHECK: disabled=false, will_execute=true
[17:33:50.926] Stage0 SPEC LOADED: path=".../spec.md", len=4496
[17:33:50.926] Stage0 EXECUTING: calling run_stage0_for_spec...
[17:33:50.945] Tier2 HEALTH CHECK: url=http://127.0.0.1:3456/health
[17:33:50.963] Tier2 HEALTH RESULT: Ok(())
[17:33:51.014] Stage0 RETURNED: result.is_ok=true
[17:33:51.015] SYSTEM POINTER: entry for spec_id=SPEC-DOGFOOD-001, has_result=true
```

**Problem Identified:**
1. Tier2 **health check passes** (`Ok(())`)
2. But DIVINE_TRUTH.md shows **fallback content** ("Tier2 unavailable")
3. Timing suggests Tier2 query isn't actually running (only 50ms for full Stage0)
4. System pointer storage is attempted but not confirmed

### New Issues Found

1. **TUI history_push not displaying** - All DEBUG messages via `history_push` are invisible
   - File-based tracing (`/tmp/speckit-trace.log`) works as workaround
   - Need to investigate TUI rendering issue

2. **Tier2 query fails silently** - Health check passes but actual query to NotebookLM fails
   - Added comprehensive tracing to `Tier2HttpAdapter.generate_divine_truth`
   - Next run will show exact failure point

### Changes Made

| File | Change |
|------|--------|
| `pipeline_coordinator.rs` | File-based trace at entry, Stage0 decision, spec load, execution |
| `stage0_integration.rs` | File-based trace for Tier2 health check, system pointer storage |
| `stage0_adapters.rs` | File-based trace for generate_divine_truth HTTP call and response |
| `routing.rs` | DEBUG trace at registry dispatch entry |
| `commands/cancel.rs` | DEBUG trace at execute entry |

### Commits (Session 29)

| Hash | Description |
|------|-------------|
| `a72186fc7` | debug(spec-kit): Add comprehensive execution trace (SPEC-DOGFOOD-001 S29) |
| `4e15a1e4c` | debug(spec-kit): Add file-based trace to /tmp/speckit-trace.log (S29) |
| `713e5166b` | debug(stage0): Add Tier2 health and system pointer trace (S29) |
| `7148c33a6` | debug(stage0): Add Tier2 adapter tracing for generate_divine_truth (S29) |

---

## Session 30 Plan

### Immediate Task: Run Test with New Tracing

```bash
rm -f /tmp/speckit-trace.log
~/code/build-fast.sh run

# In TUI:
/speckit.auto SPEC-DOGFOOD-001

# Check trace:
cat /tmp/speckit-trace.log
```

**Expected new trace entries:**
- `Tier2 GENERATE: spec_id=..., url=.../api/ask, notebook=...` - Request made
- `Tier2 SUCCESS: answer_len=...` - If query works
- `Tier2 HTTP ERROR: ...` - Connection/network issue
- `Tier2 API ERROR: ...` - NotebookLM returned error
- `Tier2 JSON PARSE ERROR: ...` - Response format issue

### Priority Tasks

1. **Identify Tier2 failure point** from trace output
2. **Fix Tier2 query** based on identified error
3. **Verify system pointer storage** - check if it's actually stored
4. **Investigate TUI history_push issue** - why debug messages don't display
5. **Validate A2** - DIVINE_TRUTH.md with real Tier2 content
6. **Validate A4** - system:true pointer in local-memory
7. **Remove debug tracing** once issues are fixed

### Acceptance Criteria Status

| ID | Criterion | Status | Evidence |
|----|-----------|--------|----------|
| A0 | No Surprise Fan-Out | ✅ PASS | `quality_gate_handler.rs:1075-1088` |
| A1 | Doctor Ready | ✅ PASS | `code doctor` shows all [OK] |
| A2 | Tier2 Used | ❌ BLOCKED | Health passes, query fails (investigating) |
| A3 | Evidence Exists | ⚠️ PARTIAL | Files exist but fallback content |
| A4 | System Pointer | ❌ BLOCKED | Storage attempted, not confirmed |
| A5 | GR-001 Enforcement | ✅ PASS | `quality_gate_handler.rs:1206-1238` |
| A6 | Slash Dispatch Single-Shot | ✅ PASS | `quality_gate_handler.rs:28-71` |

**Score: 4/6 validated, 2/6 blocked by Tier2 issue**

---

## Key Files

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | handle_spec_auto + Stage0 execution |
| `tui/src/chatwidget/spec_kit/stage0_integration.rs` | Stage0 + Tier2 integration |
| `tui/src/stage0_adapters.rs` | Tier2HttpAdapter for NotebookLM HTTP calls |
| `tui/src/slash_command.rs` | ProcessedCommand::SpecAuto parsing |
| `tui/src/chatwidget/mod.rs` | SpecAuto routing, handle_spec_auto_command |

---

## Debug Trace Locations

All file-based traces write to `/tmp/speckit-trace.log`:

| Location | Trace Message |
|----------|---------------|
| pipeline_coordinator.rs:41 | `handle_spec_auto ENTRY` |
| pipeline_coordinator.rs:172 | `Stage0 CHECK` |
| pipeline_coordinator.rs:186 | `Stage0 SPEC LOADED` |
| pipeline_coordinator.rs:257 | `Stage0 EXECUTING` |
| pipeline_coordinator.rs:289 | `Stage0 RETURNED` |
| stage0_integration.rs:480 | `Tier2 HEALTH CHECK` |
| stage0_integration.rs:514 | `Tier2 HEALTH RESULT` |
| stage0_integration.rs:552 | `SYSTEM POINTER entry` |
| stage0_adapters.rs:192 | `Tier2 GENERATE` |
| stage0_adapters.rs:228 | `Tier2 HTTP ERROR` |
| stage0_adapters.rs:254 | `Tier2 HTTP STATUS ERROR` |
| stage0_adapters.rs:280 | `Tier2 JSON PARSE ERROR` |
| stage0_adapters.rs:300 | `Tier2 API ERROR` |
| stage0_adapters.rs:318 | `Tier2 SUCCESS` |

---

## Configuration

**Stage0 config:** `~/.config/code/stage0.toml`
```toml
enabled = true
store_system_pointers = true
db_path = "~/.config/code/local-memory-overlay.db"
phase1_gate_mode = "warn"

[tier2]
enabled = true
notebook = "4e80974f-789d-43bd-abe9-7b1e76839506"
base_url = "http://127.0.0.1:3456"
cache_ttl_hours = 24
call_timeout = "30s"
```

**NotebookLM service:** Running on port 3456, healthy, authenticated

---

## Non-Negotiable Constraints

- Fix must be inside `codex-rs/` only
- Do NOT modify `localmemory-policy` or `notebooklm-mcp`
- Keep changes minimal and targeted
- Keep file-based tracing until debugging complete
