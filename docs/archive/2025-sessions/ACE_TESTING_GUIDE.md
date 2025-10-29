# 🧪 ACE Integration - Testing & Validation Guide

## Current Status (2025-10-26 21:05)

✅ **ACE Framework**: Complete (18 commits, 3,600 lines, 59 tests passing)
✅ **Database**: 8 bullets confirmed in ~/.code/ace/playbooks_normalized.sqlite3
✅ **Binary**: Fresh build at codex-rs/target/dev-fast/code (Oct 26 20:15)
✅ **Config**: Fixed (playbooks_normalized.sqlite3)
⏳ **Testing**: Ready to begin

---

## Pre-Flight Checks

### 1. Verify Setup
```bash
# Binary exists and is recent
ls -lh codex-rs/target/dev-fast/code
# Expected: Oct 26 20:15, 339M

# Database exists
ls -lh ~/.code/ace/playbooks_normalized.sqlite3
# Expected: 68K (will grow with use)

# Check bullet count
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT scope, COUNT(*) as count
FROM playbook_bullet
GROUP BY scope;"
# Expected:
# global|6
# tasks|1
# test|1
```

### 2. View Current Bullets
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT b.text, pb.scope, pb.score, pb.pinned
FROM playbook_bullet pb
JOIN bullet b ON pb.bullet_id = b.id
ORDER BY pb.scope, pb.score DESC;"
```

**Expected**: 8 constitution bullets, all pinned, score 0.0

---

## Testing Plan

### Test 1: `/speckit.ace-status` Command ✅

**Purpose**: Verify ACE status display works

**Steps**:
1. Open terminal in codex-rs:
   ```bash
   cd /home/thetu/code/codex-rs
   code
   ```

2. In the TUI, run:
   ```
   /speckit.ace-status
   ```

**Expected Output**:
```
ACE Playbook Status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Scope: global
  6 total bullets (6 pinned, avg score: 0.0)

Scope: tasks
  1 total bullets (1 pinned, avg score: 0.0)

Scope: test
  1 total bullets (1 pinned, avg score: 0.0)

Database: ~/.code/ace/playbooks_normalized.sqlite3
```

**Success Criteria**:
- ✅ Command runs without errors
- ✅ Shows 6 global, 1 tasks, 1 test bullets
- ✅ All pinned, scores at 0.0
- ✅ Correct database path displayed

**Troubleshooting**:
If command fails:
```bash
# Check logs
tail -100 ~/.code/logs/codex-tui.log | grep -i "ace\|error"

# Check database
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 ".tables"
# Should show: bullet, playbook_bullet, repo
```

---

### Test 2: `/speckit.constitution` Command ⚡

**Purpose**: Verify constitution bullet pinning (with improved UX)

**Steps**:
1. In the TUI:
   ```
   /speckit.constitution
   ```

**Expected Output** (improved UX from commit 5edd0ee):
```
⏳ Reading constitution from memory/constitution.md...
📝 Extracted 7 bullets from constitution

   Scope breakdown:
   • global: 6 bullets
   • tasks: 1 bullet
   • test: 1 bullet

⏳ Pinning bullets to ACE playbook...
✅ Successfully pinned 8 bullets to ACE playbook
   Database: ~/.code/ace/playbooks_normalized.sqlite3
   Use /speckit.ace-status to view playbook
```

**Success Criteria**:
- ✅ Shows extraction message with count
- ✅ Shows scope breakdown
- ✅ Shows success confirmation
- ✅ Database path displayed
- ✅ No errors in output

**Verification**:
```bash
# Check bullet count hasn't changed (they're already pinned)
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT COUNT(*) FROM playbook_bullet;"
# Should still be 8 (re-pinning doesn't duplicate)
```

---

### Test 3: Bullet Injection in Prompts 🎯

**Purpose**: Verify ACE bullets inject into command prompts

**Setup**:
1. Enable debug logging:
   ```bash
   RUST_LOG=codex_tui=debug code
   ```

2. Open second terminal to watch logs:
   ```bash
   tail -f ~/.code/logs/codex-tui.log | grep -E "ACE|bullet|inject"
   ```

**Test Commands**:
```
/speckit.plan SPEC-KIT-070
```
or
```
/speckit.tasks SPEC-KIT-070
```

**Expected in Logs** (Terminal 2):
```
DEBUG ACE: Fetching bullets for scope=plan, k=8
DEBUG ACE: Retrieved 6 bullets from playbook
INFO Injected 6 ACE bullets for scope: plan
```

**Expected in Prompt** (check orchestrator prompt):
Look for section like:
```markdown
### Project heuristics learned (ACE)
- [helpful] Keep tooling in this repo, project configs in product repos
- [helpful] Follow: Wrap shell scripts with TUI slash commands
- [harmful] Use MCP/LLM tooling; avoid bespoke shell scripts
...
```

**Success Criteria**:
- ✅ Logs show bullet fetching
- ✅ Logs confirm injection count
- ✅ Prompt contains ACE section before `<task>`
- ✅ Bullets are relevant to scope
- ✅ No errors or panics

**Troubleshooting**:
```bash
# Check if ACE MCP client initialized
grep "ACE MCP client" ~/.code/logs/codex-tui.log | tail -5

# Expected: "INFO ACE MCP client initialized successfully"

# If not found:
cat ~/.code/config.toml | grep -A 5 "\[mcp_servers.ace\]"
# Verify command path is correct
```

---

### Test 4: Reflector/Curator Learning Cycle 🧠

**Purpose**: Test the full ACE intelligence cycle (Reflector → Curator → Playbook update)

**Background**:
- Reflector: Analyzes outcomes, extracts patterns (Gemini Flash ~$0.05)
- Curator: Decides playbook updates (Gemini Flash ~$0.03)
- Triggers on: failures, large changes, lint issues

**Test Scenario A**: Success (Simple Scoring Only)
```
/speckit.implement SPEC-KIT-069
```

**Expected** (if already passing):
- ✅ Quality gate passes
- ✅ Simple scoring: +1.0 to used bullets
- ⚠️ No Reflector/Curator (routine success)

**Test Scenario B**: Interesting Outcome (Full Cycle)

To trigger reflection, we need a failure or large change. Options:

1. **Run on incomplete SPEC**:
   ```
   /speckit.implement SPEC-KIT-066
   ```
   If it has compile errors or test failures, Reflector will trigger.

2. **Check existing failures**:
   ```bash
   grep -l "FAILED\|ERROR" docs/SPEC-*/evidence/*.json | head -5
   ```
   Re-run a SPEC that had issues.

**Expected Logs** (if Reflector triggers):
```
INFO Quality gate validation completed
INFO ACE: Outcome is interesting, triggering Reflector...
DEBUG ACE Reflector: Analyzing execution outcome
INFO ACE Reflector: Discovered 3 patterns (2 helpful, 1 harmful)
DEBUG ACE Curator: Deciding playbook updates
INFO ACE Curator: +2 bullets, -1 deprecated, 1 adjustments
INFO ACE: Pinned 2 new bullets to playbook
INFO ACE cycle complete: 1850ms, 3 patterns, +2 bullets
```

**Verification**:
```bash
# Check playbook grew
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT scope, COUNT(*) as count
FROM playbook_bullet
GROUP BY scope;"

# Should see increase from 8 → 10-12 bullets
```

**Success Criteria**:
- ✅ Reflector analyzes interesting outcomes
- ✅ Curator makes strategic decisions
- ✅ New bullets added to playbook
- ✅ Logs show timing and counts
- ✅ No errors or panics

---

### Test 5: Playbook Growth Monitoring 📈

**Purpose**: Measure ACE learning over multiple runs

**Method**: Run 5-10 spec-kit commands and monitor growth

**Baseline** (before runs):
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT scope, COUNT(*) as total,
       SUM(CASE WHEN score > 0 THEN 1 ELSE 0 END) as promoted,
       AVG(score) as avg_score
FROM playbook_bullet
GROUP BY scope;"
```

**Example output**:
```
global|6|0|0.0
tasks|1|0|0.0
test|1|0|0.0
```

**Run Commands**:
```bash
# In TUI, run 5-10 commands:
/speckit.plan SPEC-KIT-070
/speckit.tasks SPEC-KIT-070
/speckit.implement SPEC-KIT-069
/speckit.validate SPEC-KIT-069
/speckit.implement SPEC-KIT-066
```

**After Runs** (check growth):
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT scope, COUNT(*) as total,
       SUM(CASE WHEN score > 0 THEN 1 ELSE 0 END) as promoted,
       AVG(score) as avg_score
FROM playbook_bullet
GROUP BY scope;"
```

**Expected Changes**:
- Bullet count: 8 → 10-15 (if reflection triggered)
- Scores: Some bullets > 0.0 (successful uses)
- Average score: 0.0 → 0.3-0.8 (depends on outcomes)

**Detailed View**:
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT b.text, pb.scope, pb.score, pb.successes, pb.failures
FROM playbook_bullet pb
JOIN bullet b ON pb.bullet_id = b.id
WHERE pb.score != 0.0
ORDER BY pb.score DESC
LIMIT 10;"
```

**Success Criteria**:
- ✅ Bullets used get score increases (+1.0 per success)
- ✅ New bullets added if reflection triggered
- ✅ Scores reflect actual usage
- ✅ Playbook evolves over time

---

## ACE Value Assessment Criteria

### Week 1: Baseline Validation

**Measure**:
- [ ] All commands work without errors
- [ ] Bullets inject into prompts correctly
- [ ] Playbook grows from use (8 → 15-25)
- [ ] Logs show Reflector/Curator activity

**Decision**: If basics work, continue testing

---

### Week 2: Quality Assessment

**Measure**:
1. **Bullet Relevance**:
   - Are bullets actionable?
   - Do they match the scope (plan vs implement)?
   - Are they specific to this codebase?

2. **Learning Effectiveness**:
   - Does Reflector extract useful patterns?
   - Does Curator make good decisions?
   - Do high-scoring bullets deserve their scores?

3. **Prompt Improvement**:
   - Subjective: Do prompts feel better?
   - Objective: Compare outcomes with/without ACE

**Method**: Review top bullets:
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT b.text, pb.scope, pb.score
FROM playbook_bullet pb
JOIN bullet b ON pb.bullet_id = b.id
WHERE pb.scope = 'implement'
ORDER BY pb.score DESC
LIMIT 10;"
```

Ask: "Would I write these bullets in a guide?"

---

### End of Week 2: Cost vs Value

**Costs**:
- ACE overhead: ~$0.08 per interesting outcome
- Monthly (30 reflections): ~$2.40
- vs Total costs: $200/month → 1.2% overhead

**Benefits** (to measure):
- Fewer repeated mistakes?
- Faster development?
- Better code quality?
- Compounding improvements?

**Decision Criteria**:
- **Keep Full Framework** if:
  - ✅ Bullets measurably improve prompts
  - ✅ Playbook quality is high
  - ✅ Learning is effective
  - ✅ Value justifies 3,600 lines of code

- **Simplify to 50-line Injector** if:
  - ❌ Bullets are generic/unhelpful
  - ❌ No measurable improvements
  - ❌ Reflection doesn't add value
  - ❌ Complexity not justified

**Alternative**: Static constitution injector
- Same prompt enhancement
- Zero cost, zero learning
- 50 lines vs 3,600 lines
- Use if learning doesn't prove valuable

---

## Troubleshooting Guide

### Issue: ACE Not Initializing

**Symptoms**: No ACE logs, bullets not injecting

**Check**:
```bash
# 1. Config enabled
cat ~/.code/config.toml | grep "enabled = true"

# 2. MCP server exists
ls -lh /home/thetu/agentic-context-engine/.venv/bin/python

# 3. Check logs
grep "ACE" ~/.code/logs/codex-tui.log | head -20

# 4. Test MCP server manually
/home/thetu/agentic-context-engine/.venv/bin/python -m ace_mcp_server
# Send: {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

**Fix**: Verify MCP server installation and config paths

---

### Issue: Database Locked

**Symptoms**: SQLite error "database is locked"

**Cause**: Multiple processes accessing database

**Fix**:
```bash
# Find processes
lsof ~/.code/ace/playbooks_normalized.sqlite3

# Kill MCP server if needed
pkill -f ace_mcp_server

# Restart TUI
code
```

---

### Issue: No Bullets in Playbook

**Symptoms**: /speckit.ace-status shows 0 bullets

**Fix**:
```bash
# Re-pin constitution
code
/speckit.constitution

# Verify
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "SELECT COUNT(*) FROM bullet;"
# Should be 8
```

---

### Issue: Reflector Not Triggering

**Symptoms**: Only simple scoring, no pattern extraction

**Cause**: No interesting outcomes (all successes)

**Trigger Manually**:
- Run command on failing SPEC
- Introduce compile error
- Run large refactor (>5 files)

**Verify**:
```bash
tail -f ~/.code/logs/codex-tui.log | grep "Reflector\|Curator"
```

---

## Success Metrics Summary

### Immediate (This Week):
- [x] Config fixed (playbooks_normalized.sqlite3)
- [ ] `/speckit.ace-status` works
- [ ] `/speckit.constitution` works with improved UX
- [ ] Bullets inject into prompts
- [ ] Logs show ACE activity
- [ ] Playbook starts growing

### Short-term (Week 2):
- [ ] 10+ spec-kit runs completed
- [ ] Playbook: 8 → 20-30 bullets
- [ ] Reflector triggered 3-5 times
- [ ] High-scoring bullets are relevant
- [ ] No errors or panics

### Value Decision (End of Week 2):
- [ ] Bullets are actionable and specific
- [ ] Learning improves over time
- [ ] Measurable prompt quality improvement
- [ ] Cost justified (<2% overhead acceptable)

**Final Call**: Keep full ACE or simplify to 50-line injector

---

## Next Steps

1. **Today**: Run Tests 1-3 (status, constitution, injection)
2. **This Week**: Run Tests 4-5 (learning, growth)
3. **Next Week**: Value assessment and decision
4. **Parallel**: Continue SPEC-KIT-070 (cost optimization)

Ready to test! Start with `/speckit.ace-status` in the TUI. 🚀
