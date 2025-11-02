# Next Steps: Quality Gate JSON Extraction Fix

**Priority**: HIGH (Blocks full /speckit.auto automation)
**Status**: Plan stage works ✅, Quality gates block pipeline ❌
**Affected**: before-specify, after-specify checkpoints

---

## 🎯 Problem Summary

**Symptom**:
```
✖ Quality Gate: after-specify broker error — Only found 2/3 agents
✖ Quality gate after-specify failed – missing artefacts after 1 attempts
```

**Root Cause** (via debugger agent analysis):
- "code" agent wraps JSON in timestamp/metadata prefixes
- Output format: `[2025-11-02T21:09:09] OpenAI Codex v0.0.0 ... { "stage": ... }`
- All 6 extraction strategies fail (expect clean JSON or markdown fences)

**Impact**:
- gemini agent: ✅ Valid JSON (3022 chars)
- claude agent: ✅ Valid JSON (12298 chars)
- code agent: ❌ Extraction fails (8064 chars total, JSON buried inside)
- Result: 2/3 agents → Quality gate fails → Pipeline halts

---

## 🔍 Technical Analysis (From Debugger Agent)

### Extraction Logic Location
**File**: `quality_gate_broker.rs:677-832`
**Function**: `extract_json_from_content()`

**6 Current Strategies**:
1. Markdown code fence (```` ```json ... ``` ````)
2. Brace-depth tracking (finds `{ ... }` blocks)
3. Stage marker search (`"stage": "quality-gate-"`)
4. After `[codex]` marker
5. Simple fallback (first line starting with `{`)
6. Stage-marker hardcoded search (backwards from occurrences)

**All fail because**: Timestamp prefix `[2025-11-02T...]` appears before JSON

### Quality Gate Schema Requirements
**File**: `schemas.rs:11-73`

Required fields:
```json
{
  "stage": "quality-gate-clarify",  // Must start with "quality-gate-"
  "agent": "code",
  "issues": [ ... ]  // Array required
}
```

---

## 🔧 Solution Options

### Option A: Add Metadata Stripping Strategy (RECOMMENDED - Quick Win)

**Add Strategy 7** to `extract_json_from_content()` before trying other strategies:

```rust
// Strategy 0 (pre-process): Strip agent metadata/timestamps
fn strip_agent_metadata(content: &str) -> String {
    content
        .lines()
        .skip_while(|line| {
            let trimmed = line.trim();
            // Skip timestamp lines: [2025-11-02T21:09:09]
            // Skip version lines: OpenAI Codex v0.0.0
            // Skip metadata: workdir:, model:, provider:, etc.
            trimmed.starts_with('[') && trimmed.contains(']') && trimmed.len() < 30 ||
            trimmed.starts_with("OpenAI") || trimmed.starts_with("Codex") ||
            trimmed.starts_with("---") ||
            (trimmed.contains(':') && trimmed.len() < 100 &&
             (trimmed.starts_with("workdir") || trimmed.starts_with("model") ||
              trimmed.starts_with("provider") || trimmed.starts_with("sandbox")))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

**Then**:
```rust
fn extract_json_from_content(content: &str) -> Option<String> {
    // Pre-process: Strip metadata
    let cleaned = strip_agent_metadata(content);

    // Try existing strategies on cleaned content
    // Strategy 1: Markdown fence
    // ... (existing code)
}
```

**Pros**:
- ✅ Non-invasive (adds preprocessing layer)
- ✅ Fixes "code" agent immediately
- ✅ Makes extraction more robust for all agents
- ✅ No prompt changes needed

**Cons**:
- ⚠️ Hides symptoms (agent producing messy output)
- ⚠️ Adds processing overhead (minimal)

**Implementation**: 20-30 lines in `quality_gate_broker.rs`

---

### Option B: Fix "code" Agent Prompt (ROOT CAUSE)

**Update prompt** in `prompts.json` or `quality_gate_handler.rs`:

**Current** (assumed):
```
Score requirements in SPEC-KIT-900...
Output JSON: { "stage": "quality-gate-checklist", ... }
```

**Fixed**:
```
CRITICAL: Your response must be ONLY valid JSON.
No timestamps, no metadata, no explanations.
Start your response with { and end with }

BAD (will fail):
[2025-11-02T21:09:09] OpenAI Codex...
{ "stage": ... }

GOOD:
{ "stage": "quality-gate-checklist", ... }
```

**Pros**:
- ✅ Fixes root cause (clean output)
- ✅ No extraction hacks needed
- ✅ Better long-term solution

**Cons**:
- ⚠️ Requires prompt tuning/testing
- ⚠️ May not work (agent adds metadata automatically)
- ⚠️ Doesn't help if backend adds prefixes

**Implementation**: Update prompts.json, rebuild, test

---

### Option C: Make Quality Gates Tolerant (WORKAROUND)

**Change requirement**: 2/3 passing = proceed (instead of 3/3)

```rust
// In quality gate handler:
if stored_count >= 2 {  // Was: stored_count == expected_agents.len()
    // Proceed with degraded mode
    tracing::warn!("Quality gate passed with {}/{} agents (degraded)",
        stored_count, expected_agents.len());
}
```

**Pros**:
- ✅ Unblocks testing immediately
- ✅ Realistic (agents can fail)
- ✅ No extraction changes

**Cons**:
- ⚠️ Lowers quality bar
- ⚠️ Doesn't fix underlying issue
- ⚠️ May mask real problems

---

## 📋 Recommended Implementation Plan

### Phase 1: Quick Fix (Option A - 30 min)

1. Add `strip_agent_metadata()` helper function
2. Call it at start of `extract_json_from_content()`
3. Test with existing quality gate run
4. Verify "code" agent JSON extracts successfully

### Phase 2: Validate Fix (15 min)

```bash
# Clean environment
rm -f docs/SPEC-KIT-900-generic-smoke/plan.md ~/.code/consensus_artifacts.db

# Run full pipeline
/speckit.auto SPEC-KIT-900

# Expected: All quality gates pass, all stages complete
```

### Phase 3: Root Cause (Optional - 1 hour)

If time permits, also implement Option B:
- Update prompts to enforce JSON-only
- Test if it prevents metadata wrapping
- Commit as additional hardening

---

## 🚀 Expected Outcome

**After Option A fix**:
```
✓ Quality Gate: before-specify - 3/3 agents ✅
  Launching Gemini, Claude, and GPT Pro...
  Output: plan.md ✅

✓ Quality Gate: after-specify - 3/3 agents ✅
  Launching agents for Tasks...
  Output: tasks.md ✅

[Continue through all stages]
```

**Full /speckit.auto should complete** without manual intervention!

---

## 📂 Files to Modify

1. **quality_gate_broker.rs** (add metadata stripping):
   - Add `strip_agent_metadata()` function (lines ~670-676)
   - Call at start of `extract_json_from_content()` (line ~678)
   - Test with existing failed output

2. **Optional - prompts.json** (if pursuing Option B):
   - Update quality-gate-* prompts
   - Add "CRITICAL: JSON ONLY" instruction
   - Test prompt effectiveness

---

## ✅ Session 2 Complete - Ready for Quality Gates

**Achieved**:
- ✅ Plan stage architecture: COMPLETE
- ✅ Full audit trail: OPERATIONAL
- ✅ plan.md generation: VALIDATED
- ✅ SQLite tracking: WORKING

**Next**:
- 🔧 Fix quality gate JSON extraction
- 🚀 Enable full /speckit.auto automation
- 🎯 Validate end-to-end pipeline

**Current Branch**: debugging-session (69 commits)
**Status**: Clean tree, ready for next phase

Let's fix quality gates! 🎯
