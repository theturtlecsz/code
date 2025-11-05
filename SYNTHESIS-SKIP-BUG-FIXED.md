# 🔧 CRITICAL FIX: Synthesis File Skip Bug

**Issue**: Pipeline synthesis skipped writing output files if they already existed

**Impact**: Stale implement.md (191 bytes) despite 4 agents completing successfully

**Status**: ✅ Fixed and built

---

## Root Cause

### The Bug

**Code** (pipeline_coordinator.rs:1103-1107):
```rust
// Don't overwrite if file already exists (prevents quality gates from overwriting stage output)
if output_file.exists() {
    tracing::warn!("{} ⚠️  SYNTHESIS SKIP: {} already exists, returning existing file", run_tag, output_filename);
    return Ok(output_file);  // ← RETURNS OLD FILE!
}
```

### What Happened in Recent Run

**Timeline**:
1. 20:15-20:24: 4 implement agents executed ✅
2. All agents completed successfully ✅
3. Synthesis function called ✅
4. Checked: `implement.md` exists? → **YES** (old 191-byte file from 02:23)
5. **SKIPPED writing new synthesis** ❌
6. Returned path to OLD file
7. TUI displayed: "Output: implement.md" (but it's the stale 191-byte file!)

**Result**: User sees "success" but gets stale 191-byte output instead of fresh ~10-20KB synthesis

---

## Impact Analysis

### What This Broke

**Every synthesis after the first run**:
- ❌ plan.md never updated (stuck at Nov 2 version)
- ❌ tasks.md never updated (stuck at Nov 3 version)
- ❌ implement.md never updated (stuck at Nov 4 02:23 version)

**Database impact**:
- ❌ No synthesis records created for new runs
- ❌ run_id not stored in consensus_synthesis
- ❌ Cannot track synthesis history

**Evidence impact**:
- ❌ Evidence exports show old data
- ❌ Cannot verify latest consensus
- ❌ Checklist sees stale artifacts

### Data From Recent Run (20:07-20:24)

**Agents**: 19 completed ✅
**Artifacts**: 9 stored (partial) ⚠️
**Synthesis**: SKIPPED (file exists) ❌
**Output**: Stale implement.md (191 bytes) ❌
**DB Record**: None (synthesis never wrote to DB) ❌

---

## The Fix

### Before (BROKEN)
```rust
// Don't overwrite if file already exists
if output_file.exists() {
    tracing::warn!("⚠️  SYNTHESIS SKIP: {} already exists", output_filename);
    return Ok(output_file);  // Returns OLD file!
}

fs::write(&output_file, &output)?;  // Never reached!
```

### After (FIXED)
```rust
// SPEC-KIT-900: Always write synthesis output to update with latest run
// Previous skip logic prevented updates, causing stale output files
tracing::warn!("{}   💾 Writing {} to disk (overwrite={})...",
    run_tag, output_filename, output_file.exists());

fs::write(&output_file, &output)?;  // Always writes!
```

**Change**: Removed skip logic entirely, always write synthesis output

---

## Why This Wasn't Caught Earlier

### Design Intent
Original comment: "prevents quality gates from overwriting stage output"

**Intended**: Prevent quality gate consensus from overwriting plan.md
**Actual**: Prevented ALL synthesis from updating ANY file

### Testing Gap
- First run works fine (no existing files)
- Second run silently returns stale files
- Appears to "succeed" in TUI output
- Database never updated (no synthesis record)

### Our Session 3 Testing
- Used old data (implement.md from 02:23)
- Planned to test with fresh run
- User tested before we could
- Discovered the bug in production!

---

## Related Issues

### Issue 1: Only 3/4 Artifacts Stored

**Evidence**:
- agent_executions: gemini, claude, gpt_codex, gpt_pro (4 agents)
- consensus_artifacts: gemini, code, claude (3 artifacts)

**Hypothesis**:
- "code" is how gpt_codex or gpt_pro reported
- OR only first 3 got stored

**Needs Investigation**: agent_orchestrator.rs:1384-1388 storage loop

### Issue 2: 6.1MB Response

**Evidence**:
- "code" agent has 6,128,160 byte response_text
- Expected: ~5-50KB after intelligent extraction
- 100x too large!

**Hypothesis**:
- Intelligent extraction failed for this agent
- Got full response with metadata/debug output
- extract_json_from_agent_response returned full text

**Needs Investigation**: pipeline_coordinator.rs:1371 extraction logic

---

## Build Status

```
Finished `dev-fast` profile [optimized + debuginfo] target(s) in 12.97s
✅ 0 errors, 133 warnings
```

**Binary**: Updated with fix

---

## Next Steps

### Immediate
1. ✅ Skip logic removed
2. ✅ Build successful
3. ⏳ Commit fix
4. ⏳ Test with fresh run

### To Investigate
1. **Why only 3/4 artifacts stored?**
   - Check storage loop in agent_orchestrator.rs
   - Verify all agent_responses collected

2. **Why is "code" agent 6.1MB?**
   - Check intelligent extraction
   - Verify JSON extraction working

3. **Quality gate artifacts?**
   - Should 9 QG agents create artifacts?
   - Or intentionally excluded?

---

## Testing Plan

### After This Fix
```bash
# Delete old files to force fresh synthesis
rm docs/SPEC-KIT-900-generic-smoke/{plan,tasks,implement}.md

# Run pipeline
./codex-rs/target/dev-fast/code
/speckit.auto SPEC-KIT-900 --from spec-plan

# Verify:
# - New files created with proper sizes
# - Synthesis records in SQLite with run_id
# - All 4 implement agents stored
```

---

## Status

**Critical Bug**: ✅ Fixed (synthesis skip removed)
**Build**: ✅ Success
**Related Issues**: 2 (artifact count, extraction size)
**Ready For**: Commit + test

---

**File**: pipeline_coordinator.rs (1 line removed, 2 lines modified)
**Impact**: HIGH (blocked all synthesis updates)
**Priority**: CRITICAL (must commit immediately)
