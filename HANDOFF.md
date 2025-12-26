# Session Handoff — SPEC-DOGFOOD-001 Stage0 Fix

**Last updated:** 2025-12-26
**Status:** Session 26 Complete, Stage0 Trace Analysis Pending
**Current SPEC:** SPEC-DOGFOOD-001

> **Goal**: Fix Stage0 so it executes and produces artifacts before pipeline enters PLAN/TASKS/IMPLEMENT.

---

## Session Log

| Session | Focus | LOC Deleted | Outcome |
|---------|-------|-------------|---------|
| S17 | Dead code audit | ~1,500 | Identified unused modules |
| S18 | Native consensus cleanup | ~800 | Deleted native_consensus_executor.rs |
| S19 | Config reload removal | ~840 | Deleted config_reload.rs, clippy fixes |
| S20 | Test isolation + clippy | ~10 | Added #[serial], fixed 5 clippy warnings |
| S21 | Type migration + audit | ~50 | Renamed 8 types, fixed 6 clippy, audited dead_code |
| S22 | Clippy + dead_code docs | ~20 | Fixed 17 clippy warnings, documented 13 blanket allows |
| S23 | Config fix + module deletion | ~664 | Fixed xhigh parse error, deleted unified_exec |
| S24 | Orphaned module cleanup | ~1,538 | Deleted 4 orphaned TUI modules, verified A1 |
| S25 | Acceptance validation | 0 | 4/6 criteria validated, Stage0 routing bug found |
| S26 | Stage0 routing debug | +92 | Confirmed routing works, added comprehensive trace |

**Total deleted (S17-S24):** ~5,422 LOC

---

## Session 26 Summary (Complete)

### Commits
- `ed56cd960` - fix(stage0): Add panic detection and fallback output (SPEC-DOGFOOD-001)
- `eb9f507b1` - debug(stage0): Add file-based trace to diagnose routing (SPEC-DOGFOOD-001)
- `00b0228d7` - chore: Commit SPEC-DOGFOOD-001 pipeline artifacts
- `3e35fed3c` - debug(stage0): Add comprehensive trace throughout Stage0 execution path

### Critical Finding: Routing Works, Stage0 Block Entered

**Trace log evidence (`/tmp/stage0-trace.log`):**
```
[14:53:47] BEFORE Stage0 check: disabled=false
[14:53:47] INSIDE Stage0 block (not disabled)
[14:53:47] spec_path="/home/thetu/code/docs/SPEC-DOGFOOD-001/spec.md", content_len=4496
```

**What this proves:**
1. Command routing: `ProcessedCommand::SpecAuto` → `handle_spec_auto_command()` → `handle_spec_auto()` ✅
2. Stage0 not disabled: `stage0_config.disabled=false` ✅
3. Spec file found: `content_len=4496` (not empty) ✅
4. CWD correct: `/home/thetu/code` ✅

**What's missing:** The trace stops after loading spec content. We don't yet have trace showing:
- `ENTERING Stage0 execution (content not empty)`
- `BEFORE run_stage0_for_spec() call`
- `run_stage0_for_spec() ENTRY`
- `local-memory health check`
- `AFTER run_stage0_for_spec()`
- `Stage0 result: has_result=..., skip_reason=...`

### Trace Points Added (Comprehensive)

**pipeline_coordinator.rs:**
- Line 211-220: `ENTERING Stage0 execution (content not empty)`
- Line 253-262: `BEFORE run_stage0_for_spec() call`
- Line 276-286: `AFTER run_stage0_for_spec(): is_ok=...`
- Line 314-326: `Stage0 result: has_result=..., skip_reason=..., tier2_used=...`

**stage0_integration.rs:**
- Line 59-68: `run_stage0_for_spec() ENTRY: spec_id=..., cwd=..., disabled=...`
- Line 85-94: `Checking local-memory health...`
- Line 97-106: `local-memory UNHEALTHY - returning skip`
- Line 120-129: `local-memory HEALTHY`

### Services Verified Healthy
```bash
# local-memory daemon
curl -s http://localhost:3002/api/v1/health
# {"success":true,"message":"Server is healthy",...}

# NotebookLM service
curl -s http://127.0.0.1:3456/health/ready
# {"status":"ready","ready":true,...}
```

### Acceptance Criteria Status

| ID | Criterion | Status | Evidence |
|----|-----------|--------|----------|
| A0 | No Surprise Fan-Out | ✅ | `quality_gate_handler.rs:1075-1088` - default `false` when config absent |
| A1 | Doctor Ready | ✅ | `code doctor` shows all [OK] |
| A2 | Tier2 Used | ⚠️ BLOCKED | Stage0 not producing output |
| A3 | Evidence Exists | ⚠️ BLOCKED | TASK_BRIEF.md, DIVINE_TRUTH.md not generated |
| A4 | System Pointer | ⚠️ BLOCKED | Stage0 not storing system pointer |
| A5 | GR-001 Enforcement | ✅ | `quality_gate_handler.rs:1206-1238` |
| A6 | Slash Dispatch Single-Shot | ✅ | `quality_gate_handler.rs:28-71` |

**Score: 4/6 validated, 2/6 blocked by Stage0 execution issue**

---

## Session 27 Plan: Analyze Trace & Fix Stage0

### Priority: Stage0 fix ONLY (per user preference)

### Immediate Action: Run Pipeline & Capture Full Trace

```bash
# 1. Clear old trace
rm -f /tmp/stage0-trace.log

# 2. Build and run TUI
~/code/build-fast.sh run

# 3. Execute command
/speckit.auto SPEC-DOGFOOD-001

# 4. After first guardrail, exit TUI and check trace
cat /tmp/stage0-trace.log
```

### Expected Trace Output (Full Path)

If Stage0 executes correctly:
```
[HH:MM:SS] BEFORE Stage0 check: disabled=false
[HH:MM:SS] INSIDE Stage0 block (not disabled)
[HH:MM:SS] spec_path="...", content_len=4496
[HH:MM:SS] ENTERING Stage0 execution (content not empty)
[HH:MM:SS] BEFORE run_stage0_for_spec() call
[HH:MM:SS] run_stage0_for_spec() ENTRY: spec_id=SPEC-DOGFOOD-001, cwd=..., disabled=false
[HH:MM:SS] Checking local-memory health...
[HH:MM:SS] local-memory HEALTHY
[HH:MM:SS] AFTER run_stage0_for_spec(): is_ok=true
[HH:MM:SS] Stage0 result: has_result=true, skip_reason=None, tier2_used=true
```

### Failure Scenarios & Fixes

| Last Trace Line | Diagnosis | Fix |
|-----------------|-----------|-----|
| `spec_path=..., content_len=4496` | `if !spec_content.is_empty()` not entered | Check condition logic |
| `ENTERING Stage0 execution` | Stage0Start event logging issue | Check `state.run_id` |
| `BEFORE run_stage0_for_spec()` | Call hangs or panics | Check tokio/blocking interaction |
| `run_stage0_for_spec() ENTRY` | Entry but no health check | Check disabled flag inside function |
| `Checking local-memory health...` | Health check hangs | Check timeout, daemon |
| `local-memory UNHEALTHY` | Daemon not responding | Start daemon, check port |
| `local-memory HEALTHY` | Config load fails | Check Stage0Config::load() path |
| `AFTER run_stage0_for_spec(): is_ok=false` | Panic caught | Check panic message in result |
| `Stage0 result: has_result=false` | Skip occurred | Check `skip_reason` for cause |

### After Fix: Squash Debug Commits

Once Stage0 works, squash commits:
```bash
git rebase -i HEAD~4  # Squash: ed56cd960, eb9f507b1, 00b0228d7, 3e35fed3c
# New message: "fix(stage0): Ensure Stage0 executes before pipeline (SPEC-DOGFOOD-001)"
```

### Validation Commands

```bash
# Check artifacts exist
ls docs/SPEC-DOGFOOD-001/evidence/TASK_BRIEF.md
ls docs/SPEC-DOGFOOD-001/evidence/DIVINE_TRUTH.md

# Check system pointer
lm search "SPEC-DOGFOOD-001" --limit 5

# Verify TUI shows Stage0 output
# Look for: "Stage 0: Context compiled (X memories, tier2=yes/no, Xms)"
```

---

## Key Files

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/mod.rs:4464-4484` | ProcessedCommand::SpecAuto handler |
| `tui/src/chatwidget/mod.rs:12852-12910` | handle_spec_auto_command() |
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs:32-450` | handle_spec_auto() with Stage0 |
| `tui/src/chatwidget/spec_kit/stage0_integration.rs:52-230` | run_stage0_for_spec() |
| `/tmp/stage0-trace.log` | Runtime trace output |

---

## Continuation Prompt

```
Continue SPEC-DOGFOOD-001 - Session 27 **ultrathink**

## Context
Session 26 completed (commits ed56cd960..3e35fed3c):
- Confirmed routing works: ProcessedCommand::SpecAuto → handle_spec_auto() ✅
- Confirmed Stage0 block entered: disabled=false, content_len=4496 ✅
- Added comprehensive trace to /tmp/stage0-trace.log
- Trace stops after loading spec content - need to identify failure point

## Immediate Action Required
1. Run TUI: `~/code/build-fast.sh run`
2. Execute: `/speckit.auto SPEC-DOGFOOD-001`
3. Exit after first guardrail appears
4. Run: `cat /tmp/stage0-trace.log`
5. Share FULL trace output

## Expected Trace Lines (in order)
- BEFORE Stage0 check: disabled=false
- INSIDE Stage0 block (not disabled)
- spec_path=..., content_len=4496
- ENTERING Stage0 execution (content not empty)
- BEFORE run_stage0_for_spec() call
- run_stage0_for_spec() ENTRY: spec_id=..., cwd=..., disabled=...
- Checking local-memory health...
- local-memory HEALTHY (or UNHEALTHY)
- AFTER run_stage0_for_spec(): is_ok=...
- Stage0 result: has_result=..., skip_reason=..., tier2_used=...

## Diagnosis Guide
- If trace stops at "spec_path=..." → the if block isn't entered
- If trace stops at "BEFORE run_stage0_for_spec()" → function call hangs
- If trace shows "local-memory UNHEALTHY" → daemon issue
- If "has_result=false" → check skip_reason for cause

## Acceptance Criteria (Blocked)
| ID | Criterion | Status |
|----|-----------|--------|
| A2 | Tier2 Used | ⚠️ Needs Stage0 |
| A3 | Evidence Exists | ⚠️ Needs Stage0 |
| A4 | System Pointer | ⚠️ Needs Stage0 |

## After Fix
1. Squash debug commits (4 commits → 1)
2. Remove trace code OR leave for future debugging
3. Validate artifacts: TASK_BRIEF.md, DIVINE_TRUTH.md
4. Validate system pointer: `lm search "SPEC-DOGFOOD-001"`
5. Update SPEC status

## Key Files
- pipeline_coordinator.rs:210-326 - Stage0 execution with trace
- stage0_integration.rs:52-130 - run_stage0_for_spec with trace
- /tmp/stage0-trace.log - Runtime trace output
- HANDOFF.md - This file

## Non-Negotiable Constraints
- Fix must be inside codex-rs only
- Do NOT modify localmemory-policy or notebooklm-mcp
- Keep fix minimal and targeted
```

---

## Previous Sessions (Archived)

<details>
<summary>Sessions 17-25 Summary</summary>

| Session | Focus | Outcome |
|---------|-------|---------|
| S17-S19 | Dead code cleanup | ~3,140 LOC deleted |
| S20-S22 | Test isolation, clippy | All tests passing |
| S23 | Config fix | XHigh variant, unified_exec deleted |
| S24 | Orphaned modules | 4 modules deleted (~1,538 LOC) |
| S25 | Acceptance validation | 4/6 criteria passed, Stage0 bug found |

</details>
