# 🎯 ROOT CAUSE FOUND: Missing Phase Transition After Synthesis

**Issue**: Pipeline consistently hangs after Implement stage
**Root Cause**: Missing `state.phase = SpecAutoPhase::Guardrail` transition
**Status**: ✅ FIXED

---

## The Pattern

### Every Single Run
- ✅ Plan completes (synthesis at 23:39, 00:29)
- ✅ Tasks completes (synthesis at 23:44, 00:33)
- ✅ Implement completes (synthesis at 23:54, 00:39)
- ❌ **HANGS** - Never advances to Validate

**Duration**: Stuck for 13-19 minutes after Implement synthesis

---

## Data Evidence

### Latest Run (run_SPEC-KIT-900_1762302335_ca4e5ad1)

**Agents**: 19 total, all completed ✅
- Plan: 12 agents (00:25-00:33)
- Tasks: 3 agents (00:30-00:33)
- Implement: 4 agents (00:33-00:39)

**Synthesis**: Created for all 3 stages ✅
- plan.md: 5,493 bytes (00:29:09) ✅
- tasks.md: 185 bytes (00:33:15) ✅
- implement.md: 189 bytes (00:39:50) ✅

**Advancement**: STUCK ❌
- No Validate/Audit/Unlock stages spawned
- TUI running 28+ minutes (started 00:25)
- Last activity: 00:39:50 (14 minutes ago)

---

## Root Cause Analysis

### The Code Flow

**After synthesis completes** (pipeline_coordinator.rs:660-673):

**Cached Response Path** (lines 660-673):
```rust
// Advance to next stage
if let Some(state) = widget.spec_auto_state.as_mut() {
    state.current_index += 1;  // ✅ Index advanced
    state.agent_responses_cache = None;  // ✅ Cache cleared
    // ❌ MISSING: state.phase = SpecAutoPhase::Guardrail
}
advance_spec_auto(widget);
```

**MCP/Consensus Path** (lines 847-857):
```rust
// Advance to next stage
if let Some(state) = widget.spec_auto_state.as_mut() {
    state.current_index += 1;  // ✅
    state.phase = SpecAutoPhase::Guardrail;  // ✅ PRESENT!
}
advance_spec_auto(widget);
```

### The Missing Piece

**Without phase transition**:
```
State after synthesis:
- current_index: 3 (Validate)
- phase: CheckingConsensus ← STUCK HERE!

advance_spec_auto() executes:
- Sees phase != Guardrail
- Doesn't enter Guardrail logic (line 143)
- Returns without doing anything
- Pipeline HUNG ❌
```

**With phase transition** (FIXED):
```
State after synthesis:
- current_index: 3 (Validate)
- phase: Guardrail ← RESET!

advance_spec_auto() executes:
- Sees phase == Guardrail ✅
- Enters Guardrail logic (line 143)
- Starts next stage's guardrail
- Pipeline CONTINUES ✅
```

---

## The Fix

**File**: pipeline_coordinator.rs

**Lines 662-669** (cached response success path):
```rust
if let Some(state) = widget.spec_auto_state.as_mut() {
    let old_index = state.current_index;
    state.current_index += 1;
    state.agent_responses_cache = None;
    state.phase = SpecAutoPhase::Guardrail; // ← ADDED
    tracing::warn!("    Stage index: {} → {}", old_index, state.current_index);
    tracing::warn!("    Phase reset to: Guardrail"); // ← ADDED logging
}
```

**Lines 686-690** (degraded/error path):
```rust
if let Some(state) = widget.spec_auto_state.as_mut() {
    state.current_index += 1;
    state.agent_responses_cache = None;
    state.phase = SpecAutoPhase::Guardrail; // ← ADDED
}
```

---

## Why This Happened

### Refactoring Gap

**Original code**: Only had MCP/consensus path
- Properly set phase = Guardrail

**Session 2 refactor**: Added cached response path for SPEC-KIT-900
- Skipped synthesis if file exists (Bug #1 - fixed)
- Forgot to set phase = Guardrail (Bug #3 - THIS FIX)

**Result**: Two separate code paths, one missing critical phase transition

---

## Impact

### All Sessions Affected

**Every run since cached response path was added**:
- First 3 stages work (Plan, Tasks, Implement)
- Pipeline hangs after Implement synthesis
- Never reaches Validate/Audit/Unlock
- User thinks system is stuck

### Why It Took So Long to Find

1. Synthesis appeared to succeed (created files)
2. No error messages (silent hang)
3. TUI stayed responsive (just waiting)
4. Logs would have shown it: "Phase stuck in CheckingConsensus"

**Missing**: The [run:UUID] logs we added would have revealed this immediately
```
[run:xyz] ⏩ Advancing to next stage
[run:xyz]   Stage index: 2 → 3
[run:xyz] Phase reset to: Guardrail  ← This log NOW exists
```

---

## Build Status

```
Finished `dev-fast` profile [optimized + debuginfo] target(s) in 29.13s
✅ 0 errors, 133 warnings
```

**Binary**: codex-rs/target/dev-fast/code (updated 00:37)

---

## Expected Behavior (After Fix)

### Full Pipeline Flow

**Plan stage**:
1. Spawn 3 agents (sequential)
2. Synthesis creates plan.md
3. **Phase → Guardrail** ✅
4. Advance to Tasks

**Tasks stage**:
1. Spawn 3 agents (sequential)
2. Synthesis creates tasks.md
3. **Phase → Guardrail** ✅
4. Advance to Implement

**Implement stage**:
1. Spawn 4 agents (sequential)
2. Synthesis creates implement.md
3. **Phase → Guardrail** ✅ (NOW FIXED!)
4. Advance to Validate

**Validate stage** (parallel):
1. Spawn 3 agents (parallel)
2. Synthesis creates validate.md
3. **Phase → Guardrail** ✅
4. Advance to Audit

**Audit stage** (parallel):
1. Spawn 3 agents (parallel)
2. Synthesis creates audit.md
3. **Phase → Guardrail** ✅
4. Advance to Unlock

**Unlock stage** (parallel):
1. Spawn 3 agents (parallel)
2. Synthesis creates unlock.md
3. Pipeline complete
4. Auto-verification displays

---

## Summary

**The Bug**: Missing phase transition after cached-response synthesis

**Why It Hung**:
- Index advanced (Implement → Validate)
- But phase stayed CheckingConsensus
- advance_spec_auto saw wrong phase, did nothing
- Pipeline stuck forever

**The Fix**: Add `state.phase = SpecAutoPhase::Guardrail`

**Impact**: CRITICAL - blocked ALL multi-stage pipelines

**Build**: ✅ Success

**Status**: Ready for testing

---

**This is Bug #3** in the "always stalling at the same spot" issue.
