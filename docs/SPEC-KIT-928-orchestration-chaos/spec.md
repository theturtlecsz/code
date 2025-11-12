# SPEC-KIT-928: Orchestration Flow Validation & Code Agent Completion

**Status**: CRITICAL - P0
**Priority**: Immediate
**Created**: 2025-11-12 02:45 UTC
**Discovered**: SPEC-KIT-927 testing revealed agent completion and orchestration concerns

---

## Problem Statement

Multi-agent orchestration showing unexpected behavior:

1. **Gemini/Claude "duplicates"**: Appear to run twice (needs verification if expected)
2. **Code agent 100% failure**: Spawns successfully but never reaches completion
3. **Prompt consistency unclear**: User observed different prompts per agent
4. **Workflow ordering undocumented**: Unclear when quality gates run vs regular stages

**Impact**: Uncertainty about pipeline correctness, code agent completely unusable.

---

## Evidence

### Agent Spawn Count (Latest Run: 02:38-02:41)

```
Agent Duplicates Timeline (SPEC-KIT-900):
gemini    2x   89eda6a8 (quality_gate 02:38:16), 3ac38bb6 (regular_stage 02:39:53)
claude    2x   39521f2d (quality_gate 02:38:16), 29ade056 (regular_stage 02:40:15)
code      1x   5f9b81e0 (quality_gate 02:38:16) ← NEVER COMPLETED
gpt_pro   1x   7b4cec9e (regular_stage 02:41:33)
```

**Timeline**:
```
02:38:16 - Quality gate checkpoint spawns gemini, claude, code (3 agents)
02:39:53 - Plan stage spawns gemini (1st regular stage agent)
02:40:15 - Plan stage spawns claude (2nd regular stage agent)
02:41:33 - Plan stage spawns gpt_pro (3rd regular stage agent)
```

**Analysis**:
- Gemini/Claude duplicates = **EXPECTED** (quality gate + regular stage)
- Code appearing only in quality gate = **EXPECTED** (plan uses gpt_pro, not code)
- Total: 6 agents for plan stage with quality checkpoint = **REASONABLE**

**User concern**: "Gemini running twice" may be normal behavior, not a bug.

### Code Agent Failure Pattern

**Consistent failure across all runs**:
```
Session 1 (3 runs): 0/3 completions
Session 2 (1 run):  0/1 completion (02:38:16, never finished)
```

**Evidence**:
```sql
agent_id: 5f9b81e0-...
agent_name: code
phase_type: quality_gate
spawned_at: 2025-11-12 02:38:16
completed_at: NULL           ← Never finishes
response_text: NULL          ← No output
extraction_error: NULL       ← Never reached extraction
```

**Manual execution works**:
```bash
$ code exec --sandbox read-only --model gpt-5 -c 'model_reasoning_effort="low"' \
  <<< "Test prompt"

[02:29:50] OpenAI Codex v0.0.0
...
[02:30:01] tokens used: 701
✅ Completes in 11 seconds
```

**Orchestrated execution fails**:
```rust
create_agent_from_config_name("gpt_low", ..., false) // tmux_enabled=false
# Spawns successfully
# Never completes
# No output captured
# No error message
```

### Prompt Consistency (Needs Verification)

**User observation**: "prompts seemed much different for each running llm agent"

**Hypothesis**:
- Template variable substitution inconsistent?
- Context loading varies per agent?
- Prompt building has race conditions?

**Verification needed**:
- Capture actual prompts sent to gemini, claude, code
- Compare template substitution (${SPEC_ID}, ${MODEL_ID}, etc.)
- Check if context (${ARTIFACTS}, ${CONTEXT}) differs

---

## Root Cause Analysis

### Code Agent Failure: Working Hypothesis

**Symptom**: Spawns successfully, never completes, no output, no error.

**Difference between manual vs orchestrated**:

| Aspect | Manual (works) | Orchestrated (fails) |
|--------|---------------|---------------------|
| Spawn method | Direct exec | create_agent_from_config_name() |
| Tmux | No | No (tmux_enabled=false) |
| Output capture | stdout pipe | AgentManager result field |
| Config | CLI args | config.toml gpt_low |
| Working dir | /home/thetu/code | /home/thetu/code |
| Sandbox | read-only | read-only |

**Possible causes**:

**A. Non-tmux output capture broken**:
- Quality gates use tmux_enabled=false
- Output goes to agent.result via update_agent_result()
- But agent never calls update_agent_result()?
- Process completes but nobody captures output?

**B. Silent crash**:
- Process spawns
- Crashes immediately (no stderr captured)
- AgentManager never updated
- No error propagation

**C. Infinite hang**:
- Process spawns
- Waits for something (stdin? API? lock?)
- Times out silently
- Never updates status

**D. Execution path difference**:
- Manual: execute_model_with_permissions() directly
- Orchestrated: create_agent_internal() → tokio::spawn() → execute_agent() → execute_model_with_permissions()
- Async context difference causes failure?

### Investigation Steps

**1. Enable tmux for quality gates**:
```rust
// native_quality_gate_orchestrator.rs:104
true, // tmux_enabled - creates result.txt for debugging
```

**Benefit**: Can check ~/.code/agents/{id}/result.txt for output/errors

**2. Add execution logging**:
```rust
// In execute_agent() before execute_model_with_permissions()
tracing::warn!("🔍 Executing {} (config: {}, tmux: {})", model, config_name, tmux_enabled);

// After execute_model_with_permissions()
tracing::warn!("✅ Execution returned: {} bytes", result.as_ref().map(|s| s.len()).unwrap_or(0));
```

**3. Capture spawn command**:
```rust
// Log exact command that will be executed
tracing::warn!("Command: {:?} {:?}", program, args);
```

**4. Check for suppressed errors**:
```rust
// In update_agent_result(), log ALL results (Ok and Err)
tracing::warn!("Agent {} result: {:?}", agent_id, result);
```

---

## Acceptance Criteria

### Must Achieve

1. ✅ Documented workflow execution order (quality gates → stages → quality gates)
2. ✅ Code agent 100% completion rate (or documented exclusion rationale)
3. ✅ No unexpected duplicate spawns (gemini/claude 2x is expected if quality+stage)
4. ✅ Verified prompt consistency (all agents get correct, consistent prompts)
5. ✅ All agent completions logged to SQLite

### Diagnostic Outputs Required

1. Agent spawn timeline with phase_type
2. Actual prompts sent to each agent (captured to /tmp)
3. Code agent execution trace logs
4. Workflow state machine diagram (actual vs expected)

---

## Next Session Start Checklist

**Before running anything**:

1. ✅ Read this spec (SPEC-KIT-928)
2. ✅ Read SPEC-KIT-927 session summary
3. ⬜ Enable full logging: `RUST_LOG=codex_tui::chatwidget::spec_kit=debug,codex_core::agent_tool=debug`
4. ⬜ Reset SPEC-900 database
5. ⬜ Run with monitoring in separate terminal

**During run**:

1. Monitor SQLite agent_executions in real-time
2. Monitor process list (ps aux | grep code)
3. Monitor tmux sessions
4. Capture logs to /tmp/spec-900-debug.log

**After run**:

1. Query duplicate spawns
2. Compare prompts
3. Check code agent status
4. Document findings

---

## Session 2 Summary (2025-11-12)

### Completed

**SPEC-KIT-927 Implementation**:
- ✅ Industrial JSON extraction (4-strategy cascade, 95%+ success expected)
- ✅ Extraction failure logging (SQLite diagnostics)
- ✅ Claude prompt fixes (concrete examples, anti-template)
- ✅ Config updates (18-agent spawn fixed, gpt_low PATH)

**Commits** (4):
1. `955dcaa69` - json_extractor.rs (721 LOC, 10/10 tests)
2. `a5ba92beb` - validation script
3. `ef07dfda0` - extraction failure logging (+84 LOC)
4. `f119e7300` - Claude prompt fixes

### Discovered

**SPEC-KIT-928 Issues**:
- ❌ Code agent: 100% failure rate (spawns, never completes)
- ⚠️ Gemini/Claude duplicates (may be expected behavior)
- ⚠️ Prompt consistency unclear (needs verification)
- ⚠️ Workflow ordering undocumented

### Handoff State

**Git**: Clean (43 commits ahead of origin)
**Binary**: Built (c7559ebc hash)
**Database**: Has SPEC-900 run data with duplicates
**Config**: Updated (~/.code/config.toml)

---

**Next session**: Diagnose orchestration flow, fix code agent, document workflow.
