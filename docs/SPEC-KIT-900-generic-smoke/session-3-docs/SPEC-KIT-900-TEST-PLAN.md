# SPEC-KIT-900 Testing Plan
## Session 3 - Verification of Agent Collection Fix

**Date**: 2025-11-04
**Commit**: bf0d7afd4 (run_id tracking Part 1/3)
**Binary**: ./codex-rs/target/dev-fast/code (hash 8c1eb150)

---

## Pre-Test Verification

### 1. Check Binary Hash
```bash
shasum ./codex-rs/target/dev-fast/code
# Expected: d6e7539c... (or close to this)
```

### 2. Verify Database Schema
```bash
sqlite3 ~/.code/consensus_artifacts.db ".schema agent_executions"
# Should show: run_id TEXT column
```

### 3. Check Current State
```bash
ls -lh docs/SPEC-KIT-900-generic-smoke/*.md
# You should see: plan.md (116K), tasks.md (1.6M), implement.md (191 bytes OLD)
```

---

## Test Execution

### Option A: Clean Slate (Recommended)

**Purpose**: Test with fresh data to verify the fix

```bash
# 1. Archive old data
mkdir -p docs/SPEC-KIT-900-generic-smoke/archive-session-2
mv docs/SPEC-KIT-900-generic-smoke/implement.md docs/SPEC-KIT-900-generic-smoke/archive-session-2/

# 2. Clear old agent executions (optional - keeps history but not required)
# sqlite3 ~/.code/consensus_artifacts.db "DELETE FROM agent_executions WHERE spec_id='SPEC-KIT-900' AND stage='spec-implement';"

# 3. Run TUI
cd /home/thetu/code
./codex-rs/target/dev-fast/code

# 4. In TUI, execute:
/speckit.auto SPEC-KIT-900 --from spec-implement
```

**Expected Behavior**:
1. **Implement stage starts**: Shows "🚀 Launching 4 agents in sequential pipeline mode"
2. **Agent names**: gemini, claude, gpt_codex, gpt_pro (in that order)
3. **Sequential execution**: Each agent waits for previous to complete
4. **Intelligent extraction**: Small prompts (~600 chars each, not 2.4MB!)
5. **Synthesis**: Collects **only 4 agents** (not 23)
6. **Output file**: implement.md should be ~10-20KB (meaningful content)
7. **Auto-advance**: Pipeline automatically proceeds to Validate stage
8. **Parallel execution**: Validate/Audit/Unlock run in parallel

---

### Option B: Continue from Current State

**Purpose**: Test pipeline continuation (if you want to keep old data)

```bash
# Run TUI
cd /home/thetu/code
./codex-rs/target/dev-fast/code

# In TUI, execute:
/speckit.auto SPEC-KIT-900 --from spec-validate
```

**Expected**: Validate/Audit/Unlock stages should work with 3 agents each in parallel

---

## Verification Steps

### 1. Check implement.md Output
```bash
ls -lh docs/SPEC-KIT-900-generic-smoke/implement.md
# Should be: ~10-20KB (not 191 bytes!)

wc -l docs/SPEC-KIT-900-generic-smoke/implement.md
# Should be: ~200-500 lines (meaningful content)

head -20 docs/SPEC-KIT-900-generic-smoke/implement.md
# Should show: Proper synthesis header with 4 agents
```

### 2. Check SQLite Records
```bash
# Show recent implement stage agents
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_name, run_id, spawned_at, completed_at
FROM agent_executions
WHERE spec_id='SPEC-KIT-900'
  AND stage='spec-implement'
  AND spawned_at > datetime('now', '-1 hour')
ORDER BY spawned_at;"

# Expected: 4 rows (gemini, claude, gpt_codex, gpt_pro)
# All with SAME run_id (proves they're from same run)
```

### 3. Check Synthesis Record
```bash
sqlite3 ~/.code/consensus_artifacts.db "
SELECT
  stage,
  artifacts_count,
  LENGTH(output_markdown) as markdown_size,
  run_id,
  created_at
FROM consensus_synthesis
WHERE spec_id='SPEC-KIT-900'
  AND stage='spec-implement'
ORDER BY created_at DESC
LIMIT 1;"

# Expected:
# - artifacts_count: 4 (not 23!)
# - markdown_size: 10000-20000 (not 191!)
# - run_id: UUID (not NULL)
```

### 4. Check Pipeline Advancement
```bash
# Validate stage should start automatically
# Check for validate.md creation
ls -lh docs/SPEC-KIT-900-generic-smoke/validate.md

# Check validate stage agents
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_name, phase_type, spawned_at
FROM agent_executions
WHERE spec_id='SPEC-KIT-900'
  AND stage='spec-validate'
  AND spawned_at > datetime('now', '-1 hour')
ORDER BY spawned_at;"

# Expected: 3 rows (gemini, claude, gpt_codex) with phase_type='regular_stage'
```

---

## Success Criteria

### ✅ Core Fix Verification
- [ ] implement.md is **10-20KB** (not 191 bytes)
- [ ] Synthesis shows **4 agents** (not 23)
- [ ] All 4 agents have **same run_id** in database
- [ ] Implement stage outputs contain **meaningful content** (not just headers)

### ✅ Pipeline Automation
- [ ] Plan stage completes → automatically advances to Tasks
- [ ] Tasks stage completes → automatically advances to Implement
- [ ] Implement stage completes → automatically advances to Validate
- [ ] Validate/Audit/Unlock run in **parallel** (not sequential)

### ✅ Data Integrity
- [ ] Each stage's agents have distinct run_id
- [ ] No duplicate agent collection (no mixing of old runs)
- [ ] Quality gates don't interfere with regular stages
- [ ] File sizes reasonable (no 2.4MB prompts!)

---

## Troubleshooting

### If implement.md is Still Tiny
**Symptom**: implement.md is 191 bytes, shows 23 agents

**Diagnosis**: Old code or old data
```bash
# Check if fix is in binary
strings ./codex-rs/target/dev-fast/code | grep "specific_agent_ids"
# Should show: references to filtering by specific_agent_ids

# Rebuild if needed
cd codex-rs
cargo build --profile dev-fast
```

### If Pipeline Doesn't Advance
**Symptom**: Implement completes but doesn't start Validate

**Diagnosis**: Check logs for advancement logic
```bash
# Check last 100 lines of TUI output
# Look for: "DEBUG: Calling check_consensus_and_advance_spec_auto"
```

### If Agents Collect Wrong Data
**Symptom**: Agent outputs are huge (MB+) or truncated

**Diagnosis**: Check intelligent extraction
```bash
# Examine agent output in SQLite
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_name, LENGTH(content_json), LENGTH(response_text)
FROM consensus_artifacts
WHERE spec_id='SPEC-KIT-900'
  AND stage='spec-implement'
ORDER BY id DESC
LIMIT 4;"

# Expected:
# - content_json: ~500-2000 bytes (extracted JSON)
# - response_text: ~5000-50000 bytes (full response with metadata)
```

---

## Post-Test Analysis

### If Successful
```bash
# Document success
echo "✅ SPEC-KIT-900 Session 3: Agent collection fix VERIFIED" >> docs/SPEC-KIT-900-test-results.md
echo "- implement.md: $(wc -l < docs/SPEC-KIT-900-generic-smoke/implement.md) lines" >> docs/SPEC-KIT-900-test-results.md
echo "- Agents: 4 (gemini, claude, gpt_codex, gpt_pro)" >> docs/SPEC-KIT-900-test-results.md
echo "- run_id: $(sqlite3 ~/.code/consensus_artifacts.db "SELECT DISTINCT run_id FROM agent_executions WHERE spec_id='SPEC-KIT-900' AND stage='spec-implement' ORDER BY spawned_at DESC LIMIT 1;")" >> docs/SPEC-KIT-900-test-results.md
```

### If Issues Found
1. Capture exact error messages and symptoms
2. Check which specific agents failed or misbehaved
3. Review SQLite data for pattern analysis
4. Document in SPEC-KIT-900-AUDIT-INFRASTRUCTURE-TODO.md

---

## Next Steps After Successful Test

### Immediate (Complete Auditing - 2-3 hours)
1. **Quality Gate Completion Recording** (~15min)
   - Add completion timestamps for quality gate agents
   - Mirror regular stage completion logic

2. **Log Tagging with run_id** (~30min)
   - Prefix all logs with `[run:{uuid}]`
   - Enables `grep` filtering by specific run

3. **/speckit.verify Command** (~60min)
   - Display stage-by-stage execution timeline
   - Show agent timings and outputs
   - Validate completeness and file sizes

4. **Automated Verification** (~30min)
   - After Unlock completes, auto-run verification
   - Display: ✅ PASS or ⚠️ ISSUES FOUND

### Future Enhancements
- Historical run comparison (show improvements over time)
- Cost tracking per run_id
- Performance metrics (stage duration trends)
- Automated regression detection

---

## References

**Architecture**: docs/SPEC-KIT-900-ARCHITECTURE-ANALYSIS.md
**User Guide**: docs/SPEC-KIT-900-COMPLETE-WORKFLOW.md
**TODO**: docs/SPEC-KIT-900-AUDIT-INFRASTRUCTURE-TODO.md
**Session Notes**: (this conversation)

---

**Prepared**: 2025-11-04 (Session 3)
**Status**: Ready for execution
**Expected Duration**: 30-45 minutes for full pipeline
**Risk Level**: Low (clean git state, tested fixes)
