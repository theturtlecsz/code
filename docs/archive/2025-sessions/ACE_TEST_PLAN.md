# ACE Testing Plan - Validation Session

**Date**: 2025-10-26
**Status**: Framework complete, needs real-world validation
**Database**: ~/.code/ace/playbooks_normalized.sqlite3 (8 bullets confirmed)

---

## Pre-Test Verification ✅

**Binary**: codex-rs/target/dev-fast/code (Oct 26 20:15)
**Database**: 8 bullets (global:6, tasks:1, test:1, all score 0.0)
**Config**: ACE enabled, MCP server configured

---

## Test 1: ACE Status Command

**Command**:
```bash
cd /home/thetu/code/codex-rs
code
/speckit.ace-status
```

**Expected Output**:
```
📊 ACE Playbook Status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Scope      Bullets  Pinned  Avg Score
───────────────────────────────────────
global          6       6       0.00
tasks           1       1       0.00
test            1       1       0.00
───────────────────────────────────────
TOTAL           8       8       0.00

Database: ~/.code/ace/playbooks_normalized.sqlite3
```

**Success Criteria**:
- [ ] Command runs without errors
- [ ] Table displays correctly
- [ ] Shows 8 total bullets
- [ ] Database path shown

**Failure Modes**:
- "Unknown command" → SlashCommand enum issue
- "MCP initialization failed" → Check MCP server
- Empty table → Database query issue

---

## Test 2: Constitution Command

**Command**:
```bash
/speckit.constitution
```

**Expected Output**:
```
⏳ Extracting bullets from constitution...

📋 Constitution Analysis
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Extracted 7 short bullets from memory/constitution.md

Scope Distribution:
  global: 6 bullets
  tasks:  1 bullet
  test:   1 bullet

✅ Successfully pinned 7 bullets to ACE playbook

Database: ~/.code/ace/playbooks_normalized.sqlite3

💡 Use /speckit.ace-status to view the playbook
```

**Success Criteria**:
- [ ] Extracts 7 bullets
- [ ] Shows scope breakdown
- [ ] Success confirmation
- [ ] Helpful next-step message

**Verification**:
```bash
# After running, check status again:
/speckit.ace-status

# Should show same 8 bullets (constitution already pinned)
```

**Failure Modes**:
- "Constitution not found" → Path issue
- "MCP error" → Tool call failed
- Silent failure → Check logs

---

## Test 3: Bullet Injection

**Command**:
```bash
/speckit.plan SPEC-KIT-069
```

**Expected Output**:
```
⏳ Preparing prompt with ACE context...
⏳ Fetching ACE bullets for scope: plan...
✅ Loaded 8 bullets from ACE playbook
⏳ Submitting prompt to LLM...
```

**Success Criteria**:
- [ ] "Preparing prompt with ACE context..." shown
- [ ] Bullet fetch message appears
- [ ] Bullet count displayed (should be ≤8)
- [ ] Prompt proceeds to LLM

**Verification (Check Logs)**:
```bash
tail -50 ~/.code/logs/codex-tui.log | grep -i ace

# Look for:
# - "Fetching ACE bullets for scope: plan"
# - "Injected N ACE bullets"
# - "Successfully loaded bullets from playbook"
```

**Failure Modes**:
- No "Preparing..." message → Injection not called
- "0 bullets" → Scope mismatch or query failed
- Error in logs → MCP communication issue

---

## Test 4: Reflector/Curator Learning

**Scenario**: Run a command that triggers quality gate validation

**Command**:
```bash
# Option A: Use existing SPEC
/speckit.implement SPEC-KIT-069

# Option B: Create test SPEC
/speckit.new Test ACE learning with simple feature
/speckit.auto SPEC-KIT-XXX
```

**Expected Behavior**:
1. Bullets inject at start
2. Command executes
3. Quality gate validates
4. Reflector analyzes outcome
5. Curator updates playbook

**Check Logs**:
```bash
tail -200 ~/.code/logs/codex-tui.log | grep -E "(Reflector|Curator|ACE)"

# Look for:
# - "Starting reflection on outcome..."
# - "Reflection completed, patterns extracted"
# - "Curator evaluating N patterns..."
# - "Created new bullet: ..."
```

**Verification (Database Growth)**:
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
  SELECT scope, COUNT(*) as count, AVG(score) as avg_score
  FROM playbook_bullet
  GROUP BY scope
  ORDER BY scope;
"

# Should show growth from 8 bullets
```

**Success Criteria**:
- [ ] Reflector extracts patterns (logs)
- [ ] Curator creates new bullets (logs)
- [ ] Database grows (8 → 10-15 bullets)
- [ ] Scores change based on outcome

---

## Test 5: Playbook Quality Assessment

**After 5-10 runs, check**:

**Bullet Quality**:
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
  SELECT id, scope, LEFT(text, 60) as bullet, score, pinned
  FROM playbook_bullet
  ORDER BY score DESC
  LIMIT 20;
"
```

**Questions**:
- [ ] Are bullets relevant to spec-kit tasks?
- [ ] Are they actionable (specific, not generic)?
- [ ] Do they capture useful patterns?
- [ ] Are they different from constitution bullets?

**Scope Distribution**:
```bash
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
  SELECT scope,
         COUNT(*) as total,
         SUM(CASE WHEN pinned THEN 1 ELSE 0 END) as pinned,
         COUNT(*) - SUM(CASE WHEN pinned THEN 1 ELSE 0 END) as learned,
         AVG(score) as avg_score,
         MAX(score) as max_score
  FROM playbook_bullet
  GROUP BY scope;
"
```

**Expected Evolution**:
- global: 6-10 bullets (foundational patterns)
- plan: 2-5 bullets (planning insights)
- tasks: 3-7 bullets (task decomposition)
- implement: 3-8 bullets (coding patterns)
- test: 2-5 bullets (validation strategies)

---

## Test 6: Cost Monitoring

**Check cost impact**:

**Before ACE** (from SPEC-KIT-070):
- Tier 2: ~$0.80/run
- Tier 3: ~$2.00/run

**With ACE**:
- Reflection: ~$0.05/interesting outcome
- Curation: ~$0.03/interesting outcome
- Total overhead: ~$0.08/run (1.2%)

**Verification**:
```bash
# Check recent runs in logs
grep -A 5 "Cost breakdown" ~/.code/logs/codex-tui.log | tail -30

# Should show:
# - Base agent costs
# - ACE overhead (Gemini Flash calls)
# - Total cost
```

**Success Criteria**:
- [ ] ACE overhead <2% of total cost
- [ ] Reflection only on interesting outcomes
- [ ] No excessive API calls

---

## Success Metrics (End of Week)

**ACE Proves Valuable If**:
- ✅ Bullets are relevant and actionable
- ✅ Playbook grows with quality patterns (8 → 20-30 bullets)
- ✅ Reflection insights are useful (check logs)
- ✅ Measurable improvement in prompts
- ✅ Cost justified (<2% overhead)

**ACE Should Be Simplified If**:
- ❌ Bullets are generic/unhelpful
- ❌ Playbook doesn't grow meaningfully
- ❌ No measurable improvements
- ❌ Complexity not justified
- ❌ Cost overhead unacceptable (>5%)

**Decision Point**: End of next week

---

## Alternative: 50-Line Constitution Injector

**If ACE doesn't deliver**, replace with:
```rust
// Simple injector (no learning, just constitution bullets)
fn inject_constitution(prompt: &str, scope: &str) -> String {
    let bullets = extract_constitution_bullets();
    let filtered = bullets.iter()
        .filter(|b| b.scope == scope || b.scope == "global")
        .take(8)
        .collect();
    format!("{}\n\n{}", format_bullets(filtered), prompt)
}
```

**Savings**: Remove 3,600 lines, keep prompt enhancement

---

## Troubleshooting

**ACE not initializing**:
```bash
RUST_LOG=codex_tui=debug code
# Check logs for "ACE MCP client initialized"
```

**Database issues**:
```bash
ls -lh ~/.code/ace/playbooks_normalized.sqlite3
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 ".tables"
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 ".schema playbook_bullet"
```

**MCP server issues**:
```bash
# Test MCP server directly
/home/thetu/agentic-context-engine/.venv/bin/python -m ace_mcp_server

# Check config
cat ~/.code/config.toml | grep -A 5 "\[mcp_servers.ace\]"
```

**Prompt not including bullets**:
```bash
# Check routing
grep -r "submit_prompt_with_ace" codex-rs/tui/src/chatwidget/

# Check injection call
grep -r "ace_bullet_injection" codex-rs/tui/src/chatwidget/spec_kit/
```

---

## Next Steps After Testing

**If Tests Pass**:
1. Run 10 spec-kit commands over 2-3 days
2. Monitor playbook growth and quality
3. Assess value vs complexity
4. Decide: keep full framework or simplify

**If Tests Fail**:
1. Check error logs
2. Debug specific failure mode
3. Consider simplification immediately

**Either Way**:
- Continue SPEC-KIT-070 (cost optimization Phase 2)
- Plan SPEC-KIT-071 (memory cleanup)
- Document learnings

---

## Test Log Template

**Test Run**: [Date/Time]
**Command**: [Command used]
**Result**: [Pass/Fail]
**Notes**: [Observations]
**Logs**: [Relevant log excerpts]

---

Ready to test! Start with Test 1 (/speckit.ace-status) and work through systematically.
