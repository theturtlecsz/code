# SPEC-KIT-924: Template Variable Substitution in Agent Responses

**Status**: Draft
**Created**: 2025-11-11
**Priority**: P1 (High - blocks synthesis content generation)
**Owner**: Code
**Dependencies**: SPEC-KIT-923 (observable agents - completed)

---

## Problem Statement

**Synthesis Empty Content Problem**: Multi-agent consensus stages produce synthesis files (plan.md, tasks.md, implement.md) with unsubstituted template variables instead of actual agent analysis.

**Observed Issues** (SPEC-900 validation, 2025-11-11):
- Agent responses contain literal `${PROMPT_VERSION}`, `${MODEL_ID}`, `${MODEL_RELEASE}`, `${REASONING_MODE}`
- Synthesis extracts these unsubstituted values, producing minimal .md files
- Gemini/Claude sections show empty `**content**:` fields
- gpt_pro sections show prompt templates instead of actual responses

**Impact**:
- ❌ plan.md only 360-1.5KB (should be >5KB with work breakdown)
- ❌ tasks.md only 361-1.4KB (should be >5KB with task list)
- ❌ implement.md shows prompts, not actual code diffs
- ❌ Consensus synthesis fails to extract meaningful agent analysis
- ❌ Pipeline produces unusable artifacts

**Evidence Files**:
```
docs/SPEC-KIT-900/plan.md (1.5KB) - Contains template vars like "${PROMPT_VERSION}"
docs/SPEC-KIT-900/tasks.md (1.4KB) - Empty content fields for gemini/claude
docs/SPEC-KIT-900/implement.md (5.9KB) - Shows prompts instead of responses
```

**Example from plan.md** (2025-11-11 04:32):
```json
{
  "prompt_version": "${PROMPT_VERSION}",  // ❌ Should be "20251028-plan-a"
  "agent": "gpt_pro",
  "model": "${MODEL_ID}",                 // ❌ Should be "gpt-5"
  "model_release": "${MODEL_RELEASE}",    // ❌ Should be actual release
  "reasoning_mode": "${REASONING_MODE}",  // ❌ Should be "medium"
  "feasibility_notes": [ "string" ],      // ❌ Placeholder, not actual notes
  ...
}
```

---

## Success Criteria

### Primary Goals
1. **Template substitution**: All `${...}` variables replaced with actual values before agent execution
2. **Full responses**: Agents produce complete analysis (>3KB per agent)
3. **Proper extraction**: Synthesis extracts agent responses, not prompts
4. **Rich artifacts**: plan.md >5KB, tasks.md >5KB, implement.md >5KB with actual content

### Acceptance Criteria
- [ ] Agents receive prompts with substituted variables (no `${...}` in agent input)
- [ ] Agent responses contain actual values: `"model": "gpt-5"` not `"model": "${MODEL_ID}"`
- [ ] Synthesis produces plan.md with work breakdown, risks, consensus (>5KB)
- [ ] Synthesis produces tasks.md with actual task list and details (>5KB)
- [ ] Synthesis produces implement.md with code diffs and rationale (>5KB)
- [ ] All agent sections have content (no empty `**content**:` fields)
- [ ] Template variables properly scoped: PROMPT_VERSION, MODEL_ID, MODEL_RELEASE, REASONING_MODE

### Non-Goals
- Changing template structure or schema
- Modifying synthesis extraction logic (already has fallbacks from SPEC-923)
- Agent prompt redesign (only fix variable substitution)

---

## Technical Investigation

### Files to Investigate

**Primary** (Prompt Construction):
- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
  - Lines 400-550: `build_individual_agent_prompt` or similar
  - Look for template variable substitution logic
  - Check where `${PROMPT_VERSION}` etc. should be replaced

**Secondary** (Synthesis):
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
  - Lines 1095-1315: `synthesize_from_cached_responses`
  - Lines 1142-1174: `extract_json_from_agent_response`
  - Verify response vs prompt extraction

**Templates**:
- `~/.code/templates/plan-template.md`
- `~/.code/templates/tasks-template.md`
- `~/.code/templates/implement-template.md`
- Check what variables are used and expected substitution mechanism

### Investigation Questions

1. **Where should substitution happen?**
   - In prompt construction before sending to agent?
   - Agent receives template and substitutes internally?
   - Shell-level expansion before agent execution?

2. **What are the expected values?**
   - `${PROMPT_VERSION}`: From where? Hardcoded, computed, or config?
   - `${MODEL_ID}`: From agent config (gpt-5, gemini, claude-haiku-4-5)?
   - `${MODEL_RELEASE}`: From agent metadata?
   - `${REASONING_MODE}`: From agent config (low, medium, high)?

3. **Why are some responses empty?**
   - Gemini/claude show `**content**:` with nothing
   - Is extraction failing or are responses actually empty?
   - Are agents receiving malformed prompts?

4. **Why does gpt_pro show prompts instead of responses?**
   - Is synthesis extracting wrong field from agent output?
   - Are agents echoing input instead of generating output?

---

## Test Plan

### Phase 1: Diagnosis
1. Add logging to prompt construction to show before/after substitution
2. Log actual prompts sent to agents (first 500 chars)
3. Log agent responses received (first 500 chars)
4. Identify where substitution should happen but doesn't

### Phase 2: Fix Implementation
1. Implement proper template variable substitution
2. Ensure all four variables handled: PROMPT_VERSION, MODEL_ID, MODEL_RELEASE, REASONING_MODE
3. Test with single agent first (/speckit.plan SPEC-KIT-900)
4. Verify substituted values appear in agent response

### Phase 3: Validation
1. Run full pipeline: /speckit.auto SPEC-KIT-900
2. Verify plan.md >5KB with actual work breakdown
3. Verify tasks.md >5KB with actual task list
4. Verify implement.md >5KB with actual code diffs
5. Check all `${...}` variables are replaced
6. Verify synthesis extracts responses correctly

### Test Commands
```bash
# Clean state
rm -f docs/SPEC-KIT-900/plan.md docs/SPEC-KIT-900/tasks.md docs/SPEC-KIT-900/implement.md
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-900
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-900

# Single stage test
export SPEC_KIT_OBSERVABLE_AGENTS=1
./target/dev-fast/code
# In TUI: /speckit.plan SPEC-KIT-900

# Verify
cat docs/SPEC-KIT-900/plan.md  # Should have >5KB, no ${...} vars
grep -c '${' docs/SPEC-KIT-900/plan.md  # Should be 0
wc -c docs/SPEC-KIT-900/plan.md  # Should be >5000
```

---

## Related Context

### From SPEC-923 (Completed)
- ✅ Tmux observable agents working (commits 5aa9da7ee, 33615d652, 3f99017e7)
- ✅ Output files clean (no RUST_LOG pollution)
- ✅ Completion detection working
- ✅ Fresh panes created per agent
- ✅ File cleanup functioning

### Current System State
- **Repository**: /home/thetu/code
- **Branch**: main (22 commits ahead, clean tree)
- **Binary**: ./target/dev-fast/code (includes all SPEC-923 fixes)
- **Observable agents**: Enabled via `SPEC_KIT_OBSERVABLE_AGENTS=1`

### Known Good
- Agent execution via tmux: ✅ Working
- Output file creation: ✅ Working
- Output file cleanup: ✅ Working
- Synthesis extraction fallbacks: ✅ Working (from SPEC-923 Bug #3 fix)

### Known Bad
- Template variable substitution: ❌ Not working
- Agent response content: ❌ Empty or contains prompts
- Synthesis file size: ❌ Too small (<2KB instead of >5KB)

---

## Notes

This issue was discovered during SPEC-923 validation but is **NOT** a tmux/observable agents issue. It's a **prompt construction or agent response handling** bug that affects synthesis quality regardless of execution method (tmux or normal).

SPEC-923 fixes are complete and working. This is a separate issue requiring investigation into:
- Where template variables should be substituted
- Why agents are returning prompts instead of responses
- Why some agent responses are completely empty

Focus on **prompt construction flow** and **template variable handling**, not tmux execution mechanics.
