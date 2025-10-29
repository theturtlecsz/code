# Next Steps - Action Plan (2025-10-26)

## ✅ Just Completed (This Session)

### 1. Full ACE Framework Integration
- **Commit**: 3b0d47fc2
- **Content**: 8 modules, 3,195 lines, Reflector/Curator
- **Status**: Ready for testing

### 2. SPEC-KIT-069 Completion
- **Commits**:
  - bb972d789 (tests)
  - 7d0161c97 (infrastructure)
  - 2577a9de6 (ValidateLifecycle)
- **Content**: Quality gates, validate lifecycle, 79 files
- **Status**: Complete, all tests passing

### 3. Working Tree Cleanup
- **Before**: 56 uncommitted files
- **After**: 3 untracked docs only
- **Status**: Clean ✅

---

## 🎯 Immediate Next Steps (Today)

### Step 1: Test ACE Initialization

```bash
# Build is running...
# When complete, test ACE:

cd /home/thetu/code
./codex-rs/target/release/code-tui

# Expected in logs:
# INFO ACE MCP client initialized successfully
```

**Check logs**:
```bash
tail -f ~/.code/logs/codex-tui.log | grep ACE
```

**Expected output**:
```
INFO ACE MCP client initialized successfully
```

---

### Step 2: Seed ACE Playbook

**In CODE TUI**:
```
/speckit.constitution
```

**Expected output**:
```
Extracted 8 bullets from constitution, pinning to ACE...
Successfully pinned 8 bullets to ACE playbook (global + phase scopes)
```

**Verify in logs**:
```bash
grep "ACE pin" ~/.code/logs/codex-tui.log
# Should see: INFO ACE pin 145ms pinned=8 bullets
```

**Verify database**:
```bash
ls -lh ~/.code/ace/playbooks_normalized.sqlite3
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "SELECT COUNT(*) FROM bullets;"
# Should show: 8
```

---

### Step 3: Test ACE Injection

**Run a spec-kit command**:
```
/speckit.implement SPEC-KIT-069
```

**Watch logs** (in another terminal):
```bash
tail -f ~/.code/logs/codex-tui.log | grep -E "ACE|Reflector|Curator"
```

**Expected flow**:
```
DEBUG ACE playbook_slice: repo=/home/thetu/code, branch=..., scope=implement, k=8
DEBUG Injected 6 ACE bullets for scope: implement
... (execution happens) ...
INFO ACE Reflector: Analyzing execution outcome...
INFO ACE Reflector: Discovered 3 patterns (2 helpful, 1 harmful)
INFO ACE Curator: Deciding playbook updates...
INFO ACE Curator: +2 bullets, -1 deprecated, 1 adjustments
INFO ACE: Pinned 2 new bullets to playbook
INFO ACE cycle complete: 1850ms, 3 patterns, +2 bullets
```

---

## 📅 This Week's Plan

### Monday (Today)
- [x] Clean working tree
- [x] Commit SPEC-KIT-069
- [ ] Test ACE initialization
- [ ] Run `/speckit.constitution`
- [ ] Monitor 1-2 spec-kit runs with ACE

### Tuesday-Wednesday
- [ ] Run 5 more spec-kit commands with ACE
- [ ] Check playbook growth: `sqlite3 ~/.code/ace/playbooks_normalized.sqlite3`
- [ ] Assess bullet quality
- [ ] Monitor costs in logs

### Thursday-Friday
- [ ] SPEC-KIT-070: Validate GPT-4o
- [ ] Add ACE cost tracking to cost_tracker.rs
- [ ] Plan Phase 2 (complexity routing)

### Weekend
- [ ] Review ACE value (1 week of data)
- [ ] Decision: Keep full framework or simplify
- [ ] Plan SPEC-KIT-071 start

---

## 🔍 ACE Validation Checklist

### Success Indicators

✅ **Initialization works**:
```bash
grep "ACE MCP client initialized" ~/.code/logs/codex-tui.log
```

✅ **Constitution pinning works**:
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "SELECT text FROM bullets LIMIT 5;"
```

✅ **Injection works**:
```bash
grep "Injected.*ACE bullets" ~/.code/logs/codex-tui.log
```

✅ **Reflection triggers**:
```bash
grep "ACE Reflector" ~/.code/logs/codex-tui.log
```

✅ **Curation happens**:
```bash
grep "ACE Curator" ~/.code/logs/codex-tui.log
```

✅ **Playbook grows**:
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
SELECT scope, COUNT(*) as bullets, AVG(score) as avg_score
FROM bullets
GROUP BY scope;"
```

---

## 📊 Measurement Plan

### Day 1 (Today)
- Baseline: 0 bullets (fresh start)
- After constitution: 8 bullets
- After 1-2 runs: Check for growth

### Day 3
- Expected: 12-15 bullets
- Check reflection frequency
- Review bullet text quality

### Day 7
- Expected: 20-30 bullets
- Measure actual value
- Decision point: Keep or simplify

---

## 🎯 Decision Criteria (End of Week)

### Keep Full ACE Framework If:
- ✅ Bullets are relevant and actionable
- ✅ Playbook grows with quality patterns
- ✅ Reflection insights are valuable
- ✅ Measurable improvement in prompts
- ✅ Cost justified by quality gains

### Simplify to 50-Line Injector If:
- ❌ Bullets are generic/unhelpful
- ❌ Playbook doesn't grow meaningfully
- ❌ Reflection doesn't extract useful patterns
- ❌ No measurable improvements
- ❌ Complexity not justified

---

## 🚀 Outstanding Work (After ACE Testing)

### SPEC-KIT-070 (In Progress)
- **Current**: 40-50% cost reduction
- **Next**: GPT-4o validation, cost tracking, Phase 2
- **Target**: 70-80% reduction

### SPEC-KIT-071 (Start Next Week)
- **Problem**: 574 memories, 552 tags (chaos)
- **Target**: 300 memories, 90 tags (organized)
- **Priority**: HIGH (analysis tools broken)

### SPEC-KIT-066 (Lower Priority)
- **Issue**: Orchestrator uses bash/Python scripts
- **Fix**: Migrate to native tools
- **Timeline**: After 070/071

---

## 📝 Summary

**Status**: Clean working tree, ACE committed, ready to test

**Next**: Test ACE (waiting for build to complete)

**This Week**: Validate ACE, continue SPEC-KIT-070

**Next Week**: Decide on ACE, start SPEC-KIT-071

**Philosophy**: Validate before adding more complexity
