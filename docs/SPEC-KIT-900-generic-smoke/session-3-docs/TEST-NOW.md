# 🚀 Quick Test Guide - SPEC-KIT-900

**Status**: ✅ Core fixes implemented, ready for end-to-end test
**Time**: ~30-45 minutes

---

## Quick Start (Clean Slate)

```bash
# 1. Archive old data
mkdir -p docs/SPEC-KIT-900-generic-smoke/archive-session-2
mv docs/SPEC-KIT-900-generic-smoke/implement.md docs/SPEC-KIT-900-generic-smoke/archive-session-2/

# 2. Run TUI (from repo root)
./codex-rs/target/dev-fast/code

# 3. In TUI:
/speckit.auto SPEC-KIT-900 --from spec-implement
```

---

## What to Watch For

### ✅ Good Signs
- "🚀 Launching 4 agents in sequential pipeline mode"
- Agent names: gemini → claude → gpt_codex → gpt_pro
- Small prompts (~600 chars each)
- Implement stage completes and shows "Synthesizing consensus..."
- **Automatically advances to Validate stage**
- Validate/Audit/Unlock run in parallel

### ⚠️ Red Flags
- Shows more than 4 agents
- Prompts are huge (MB+)
- implement.md stays 191 bytes
- Pipeline stalls (doesn't advance)

---

## Quick Verification

```bash
# Check output file
ls -lh docs/SPEC-KIT-900-generic-smoke/implement.md
# Should be: ~10-20KB

# Check synthesis
sqlite3 ~/.code/consensus_artifacts.db "
SELECT artifacts_count, LENGTH(output_markdown)
FROM consensus_synthesis
WHERE spec_id='SPEC-KIT-900' AND stage='spec-implement'
ORDER BY created_at DESC LIMIT 1;"
# Should show: 4 | 10000-20000 (not 23 | 191!)

# Check run_id tracking
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_name, run_id
FROM agent_executions
WHERE spec_id='SPEC-KIT-900' AND stage='spec-implement'
  AND spawned_at > datetime('now', '-1 hour');"
# Should show: 4 rows, all with SAME run_id
```

---

## Success = Move to Auditing

If test succeeds, next tasks:
1. Quality gate completion recording (~15min)
2. Log tagging with run_id (~30min)
3. /speckit.verify command (~60min)
4. Automated verification (~30min)

See: **SPEC-KIT-900-AUDIT-INFRASTRUCTURE-TODO.md**

---

## If Issues Found

Capture:
- Exact error messages
- TUI output (screenshots if possible)
- SQLite query results
- Log excerpts

Report in this conversation or update:
**SPEC-KIT-900-ARCHITECTURE-ANALYSIS.md**

---

**Full Details**: SPEC-KIT-900-TEST-PLAN.md
