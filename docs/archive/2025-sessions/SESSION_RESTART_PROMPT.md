# 🚀 ACE Integration - Session Restart Prompt

Use this to start your next session with full context.

---

## RESTART PROMPT (Copy/Paste to New Session)

```markdown
CONTEXT: ACE (Agentic Context Engine) Integration - Testing & Validation Phase

We completed full ACE framework integration (18 commits, 3,600 lines) in the previous session.
The code is complete and committed. Now we need to TEST and VALIDATE functionality.

## Current State (2025-10-26 20:15)

**Branch**: feature/spec-kit-069-complete
**Latest commit**: 5b7f228a8 (or later)
**Working directory**: /home/thetu/code/codex-rs
**Binary**: codex-rs/target/dev-fast/code (freshly built)

**ACE Database**: ~/.code/ace/playbooks_normalized.sqlite3
- Confirmed working: 8 bullets pinned (global:6, tasks:1, test:1)

**Code Status**:
- ✅ All tests passing (59 ACE + 604 total)
- ✅ Build successful
- ✅ Working tree clean
- ⏳ Real-world testing needed

## What Was Completed (Previous Session)

**Full ACE Framework Implementation**:
1. ✅ MCP client integration (playbook_slice, playbook_pin, learn)
2. ✅ Reflector module (LLM pattern extraction via Gemini Flash)
3. ✅ Curator module (strategic playbook updates via Gemini Flash)
4. ✅ Orchestrator (full reflection-curation cycle)
5. ✅ Constitution pinning (/speckit.constitution command)
6. ✅ Bullet injection (widget-level async, ALL 11 commands)
7. ✅ Learning hooks (after quality gate validation)
8. ✅ Status command (/speckit.ace-status)

**Issues Fixed**:
- SlashCommand enum (SpecKitConstitution, SpecKitAceStatus variants)
- Runtime panics (block_on → spawn, 4 locations)
- MCP tool names (ace.* prefix removed)
- FastMCP schema (input wrapping, scope param, bullet objects)
- Async architecture (widget.submit_prompt_with_ace method)
- UX feedback (detailed messages, status command)

**Documentation Created**:
- ACE_FULL_FRAMEWORK.md - Complete architecture
- ACE_ACTIVATION_GUIDE.md - Setup instructions
- ACE_QUICKSTART.md - User guide
- PROJECT_STATUS_FINAL.md - Current state
- SESSION_SUMMARY.md - Session overview

## TASKS FOR THIS SESSION

### Priority 1: Validate ACE Functionality ⚡

**Goal**: Verify all ACE components work as designed

**Tasks**:

1. **Test Constitution Command**
   ```bash
   code
   /speckit.constitution
   ```
   **Expected**:
   - Extract 7-8 bullets message
   - Scope breakdown shown
   - Success confirmation with database path
   - No errors

2. **Test Status Command**
   ```bash
   /speckit.ace-status
   ```
   **Expected**:
   - Table showing bullets by scope
   - Statistics (total, pinned, scores)
   - Database path displayed

3. **Test Bullet Injection**
   ```bash
   /speckit.implement SPEC-KIT-069
   ```
   **Expected**:
   - "⏳ Preparing prompt with ACE context..." message
   - Prompt includes bullets section before <task>
   - Check logs for: "Injected N ACE bullets for scope: implement"

4. **Verify Reflection/Curation**
   - Run a command that fails (compile error or test failure)
   - Check logs for Reflector analysis
   - Check logs for Curator decisions
   - Verify new bullets added to database

5. **Monitor Playbook Growth**
   ```bash
   sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 "
     SELECT scope, COUNT(*), AVG(score)
     FROM playbook_bullet
     GROUP BY scope;"
   ```
   **Expected**: After 5-10 runs, see growth from 8 → 15-25 bullets

**Success criteria**: All 5 tests pass, playbook grows, bullets are relevant

**Failure mode**: If issues found, debug or consider simplification

---

### Priority 2: Continue SPEC-KIT-070 (Cost Optimization)

**Status**: Phase 1 complete (40-50% reduction: $11 → $5.50-6.60)

**Tasks**:

1. **Validate GPT-4o** (rate limit should be reset)
   - Test GPT-4o in orchestrator
   - Verify cost reduction
   - Measure quality

2. **Add ACE Cost Tracking**
   - Integrate with cost_tracker.rs
   - Track reflection costs (~$0.05/call)
   - Track curation costs (~$0.03/call)
   - Report ACE overhead percentage

3. **Plan Phase 2**
   - Complexity routing (simple vs complex tasks)
   - /implement refactor for efficiency
   - Target: 70-80% total reduction

**Success criteria**: GPT-4o validated, costs tracked, Phase 2 planned

---

### Priority 3: ACE Value Assessment

**Goal**: Decide whether to keep full framework or simplify

**Measure**:

1. **Bullet Quality**
   - Are bullets relevant?
   - Are they actionable?
   - Do they improve prompts?

2. **Learning Effectiveness**
   - Does Reflector extract useful patterns?
   - Does Curator make good decisions?
   - Does playbook improve over time?

3. **Cost vs Benefit**
   - ACE overhead: ~$0.08/interesting outcome
   - Prompt quality improvement: measurable?
   - Time savings: quantifiable?

**Decision criteria** (end of week):
- **Keep** if bullets measurably improve prompts
- **Simplify** if value doesn't justify 3,600 lines

**Alternative**: 50-line constitution injector (same prompt enhancement, 1/60th code)

---

### Priority 4: Plan SPEC-KIT-071 (Memory Cleanup)

**Status**: Backlog, needs to start next week

**Preparation tasks**:

1. Review cleanup plan (574→300 memories, 552→90 tags)
2. Coordinate with ACE (keep separate or integrate?)
3. Schedule start date
4. Estimate effort (16-23 hours over 2 weeks)

**Dependencies**: ACE value decision (affects storage strategy)

---

## DEBUGGING GUIDE (If Issues Found)

### ACE Not Working

**Check initialization**:
```bash
tail ~/.code/logs/codex-tui.log | grep "ACE MCP client initialized"
```

**Check database**:
```bash
ls -lh ~/.code/ace/playbooks_normalized.sqlite3
sqlite3 ~/.code/ace/playbooks_normalized.sqlite3 ".tables"
```

**Check config**:
```bash
cat ~/.code/config.toml | grep -A 5 "\[ace\]"
cat ~/.code/config.toml | grep -A 3 "\[mcp_servers.ace\]"
```

### Common Issues

**"Unknown tool"**: Tool names wrong (should be: playbook_slice, playbook_pin, learn)
**"Validation error"**: Schema mismatch (check input wrapping, scope param)
**"No such table"**: Database not initialized (run /speckit.constitution)
**Silent failure**: Check logs with RUST_LOG=codex_tui=debug

### Files to Check

- `tui/src/chatwidget/spec_kit/ace_client.rs` - MCP integration
- `tui/src/chatwidget/spec_kit/commands/special.rs` - Constitution command
- `tui/src/chatwidget/mod.rs` - submit_prompt_with_ace method
- `tui/src/chatwidget/spec_kit/routing.rs` - Injection call
- `~/.code/config.toml` - ACE configuration

---

## RELEVANT CONTEXT

**Repository**: /home/thetu/code
**Rust workspace**: /home/thetu/code/codex-rs
**Build command**: /home/thetu/code/build-fast.sh
**Binary location**: codex-rs/target/dev-fast/code (in PATH as "code")

**ACE MCP Server**: /home/thetu/agentic-context-engine
- Command: /home/thetu/agentic-context-engine/.venv/bin/python -m ace_mcp_server
- Database: ~/.code/ace/playbooks_normalized.sqlite3
- Tools: playbook_slice, playbook_pin, learn

**Key Files**:
- Constitution: memory/constitution.md (7 short bullets + detailed versions)
- Config: ~/.code/config.toml ([ace] and [mcp_servers.ace] sections)
- SPEC tracker: SPEC.md (shows SPEC-KIT-070 in progress, 071 backlog)

**Outstanding SPECs**:
- SPEC-KIT-070: Cost optimization (Phase 1 done, Phase 2 pending)
- SPEC-KIT-071: Memory cleanup (analysis done, cleanup pending)
- SPEC-KIT-066: Native tools (low priority)

---

## SUCCESS METRICS

**This Week**:
- [ ] ACE commands work in TUI (visible output)
- [ ] Bullets inject into prompts
- [ ] Playbook grows from use
- [ ] SPEC-KIT-070 Phase 2 planned

**Next Week Decision**:
- Keep full ACE if valuable
- Simplify to basic injector if not

---

## IMMEDIATE FIRST STEPS

1. Verify build status: `cargo build -p codex-tui`
2. Check binary: `ls -lh codex-rs/target/dev-fast/code`
3. Test ACE status: `code` then `/speckit.ace-status`
4. Review logs: `tail ~/.code/logs/codex-tui.log | grep ACE`

---

## REFERENCES

Read these for full context:
- PROJECT_STATUS_FINAL.md - Complete status
- ACE_FULL_FRAMEWORK.md - Architecture
- DEBUGGING_STEPS.md - Troubleshooting

**Current state**: Framework complete, needs validation testing.
```

---

## END OF RESTART PROMPT

Copy everything between the ``` markers above to start your next session with full context!
