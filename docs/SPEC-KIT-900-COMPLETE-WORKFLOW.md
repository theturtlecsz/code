# SPEC-KIT-900 Complete Workflow Documentation

**Date**: 2025-11-03 (Session 3)
**Status**: Architecture Refactored - Individual Prompts Per Agent

---

## Complete Agent & Model Roster

### Quality Gates (All Checkpoints)

**Checkpoints**: before-specify, after-specify, after-tasks

| Agent | Model | Config Name | Role | Cost/Gate |
|-------|-------|-------------|------|-----------|
| Gemini | gemini-2.5-flash | `gemini_flash` | Ambiguity Detector / Consistency Checker | ~$0.02 |
| Claude | claude-3.5-haiku | `claude_haiku` | Ambiguity Resolver / Validator | ~$0.02 |
| Code | gpt-5 low reasoning | `gpt_low` | Implementation Validator | ~$0.03 |

**Total per Gate**: ~$0.07
**Execution**: Parallel with individual prompts ✅
**Consensus**: 3/3 or 2/3 acceptable

---

### Plan Stage (Tier 2)

**Output**: `plan.md`

| Agent | Model | Config Name | Role | Prompt Section |
|-------|-------|-------------|------|----------------|
| Gemini | gemini-2.5-flash | `gemini_flash` | Researcher | Survey SPEC, summarize acceptance criteria, find conflicts |
| Claude | claude-3.5-haiku | `claude_haiku` | Synthesizer | Produce work_breakdown, acceptance_mapping, risks |
| GPT_Pro | gpt-5 medium effort | `gpt_pro` | Executor & QA | Validate feasibility, build final_plan with consensus |

**Cost**: ~$0.35
**Duration**: 10-12 minutes
**Execution**: Parallel with individual prompts (after Session 3 fix)

---

### Tasks Stage (Tier 2)

**Output**: `tasks.md`

| Agent | Model | Config Name | Role | Prompt Section |
|-------|-------|-------------|------|----------------|
| Gemini | gemini-2.5-flash | `gemini_flash` | Researcher | Identify surfaces, dependencies, SPEC status |
| Claude | claude-3.5-haiku | `claude_haiku` | Synthesizer | Produce task list with validation steps |
| GPT_Pro | gpt-5 medium effort | `gpt_pro` | Executor & QA | Verify guardrails, build command plan |

**Cost**: ~$0.35
**Duration**: 10-12 minutes
**Execution**: Parallel with individual prompts

---

### Implement Stage (Tier 2 + Code Specialist)

**Output**: `implement.md`

| Agent | Model | Config Name | Role | Prompt Section |
|-------|-------|-------------|------|----------------|
| Gemini | gemini-2.5-flash | `gemini_flash` | Code Path Analyzer | Identify code paths, recent changes, edge cases, tests |
| Claude | claude-3.5-haiku | `claude_haiku` | Strategy | Outline implementation approach, operations, validation plan |
| **GPT_Codex** | **gpt-5 codex HIGH** | `gpt_codex` | Code Generator | Generate diff_proposals, test_commands, tool_calls |
| GPT_Pro | gpt-5 medium effort | `gpt_pro` | QA | Validate feasibility, build checklist, assess risks |

**Cost**: ~$0.11 (codex cheaper than expected)
**Duration**: 8-12 minutes
**Execution**: Parallel with individual prompts
**Note**: 4 agents (adds gpt_codex specialist)

---

### Validate Stage (Tier 2)

**Output**: `validate.md`

| Agent | Model | Config Name | Role | Prompt Section |
|-------|-------|-------------|------|----------------|
| Gemini | gemini-2.5-flash | `gemini_flash` | Test Strategy | Summarize telemetry, scenarios, evidence |
| Claude | claude-3.5-haiku | `claude_haiku` | Coverage Analyzer | Crosswalk acceptance vs results, remediation |
| GPT_Pro | gpt-5 medium effort | `gpt_pro` | Validator | Confirm outputs, final decision |

**Cost**: ~$0.35
**Duration**: 10-12 minutes
**Execution**: Parallel with individual prompts

---

### Audit Stage (Tier 3 - Premium)

**Output**: `audit.md`

| Agent | Model | Config Name | Role | Prompt Section |
|-------|-------|-------------|------|----------------|
| Gemini | gemini-2.5-pro | `gemini_pro` | Security Auditor | Collect audit artifacts, compliance flags |
| Claude | claude-3.5-sonnet | `claude_sonnet` | Compliance Reviewer | Draft audit memo, evidence, risks |
| GPT_Pro | gpt-5 high reasoning | `gpt_pro` | Final Reviewer | Verify guardrails, recommendation |

**Cost**: ~$0.80
**Duration**: 10-12 minutes
**Execution**: Parallel with individual prompts

---

### Unlock Stage (Tier 3 - Premium)

**Output**: `unlock.md`

| Agent | Model | Config Name | Role | Prompt Section |
|-------|-------|-------------|------|----------------|
| Gemini | gemini-2.5-pro | `gemini_pro` | Context Analyzer | Explain branch state, pending work, risks |
| Claude | claude-3.5-sonnet | `claude_sonnet` | Decision Writer | Write unlock justification, safeguards, followups |
| GPT_Pro | gpt-5 high reasoning | `gpt_pro` | Final Approver | Check compliance, make unlock decision |

**Cost**: ~$0.80
**Duration**: 10-12 minutes
**Execution**: Parallel with individual prompts

---

## Complete Pipeline Flow: `/speckit.auto SPEC-KIT-900`

### Stage 1: PLAN

```
1. Guardrail Check (native, FREE, <1s)
   ✓ spec-id-validation: SPEC ID 'SPEC-KIT-900' is valid
   ✓ spec-files: spec.md exists
   ✓ clean-tree: No unexpected changes
   ✓ plan-stage: Ready

2. Quality Gate: before-specify (3 agents, ~$0.07, 50s)
   Spawn: gemini_flash, claude_haiku, gpt_low
   Prompts: quality-gate-clarify (individual per agent)
   Check: Ambiguities in SPEC
   Consensus: 3/3 or 2/3 → PASS

3. Regular Stage: Plan (3 agents, ~$0.35, 4-5min)
   Spawn: gemini_flash, claude_haiku, gpt_pro
   Prompts: spec-plan (individual per agent) ← FIXED Session 3

   Agent Execution (Parallel):
   ├─ Gemini: Research SPEC, find conflicts
   ├─ Claude: Build work_breakdown, acceptance_mapping
   └─ GPT_Pro: Validate, build consensus

   Background Polling:
   ├─ Every 500ms, check agent status
   ├─ Wait for all 3 agents: Completed|Failed|Cancelled
   └─ Timeout: 600s (10 minutes)

   Completion:
   ├─ Send: RegularStageAgentsComplete event
   ├─ Collect: 3 agent responses
   ├─ Store: SQLite consensus_artifacts (3 rows)
   └─ Synthesize: Generate plan.md from 3 perspectives

   Output: docs/SPEC-KIT-900-generic-smoke/plan.md

4. Quality Gate: after-specify (3 agents, ~$0.07, 50s)
   Spawn: gemini_flash, claude_haiku, gpt_low
   Prompts: quality-gate-analyze (individual per agent)
   Check: Consistency between spec.md and plan.md
   Consensus: 3/3 or 2/3 → PASS

→ ADVANCE TO TASKS
```

### Stage 2: TASKS

```
1. Guardrail Check (native, FREE, <1s)
   ✓ spec-id-validation
   ✓ spec-files
   ✓ clean-tree: plan.md excluded (expected artifact)
   ✓ tasks-stage: plan.md exists

2. Regular Stage: Tasks (3 agents, ~$0.35, 4-5min)
   Spawn: gemini_flash, claude_haiku, gpt_pro
   Prompts: spec-tasks (individual per agent) ← FIXED Session 3

   Context Provided to Each Agent:
   ├─ spec.md (SPEC definition)
   └─ plan.md (from Plan stage)

   Agent Execution (Parallel):
   ├─ Gemini: Identify surfaces, dependencies
   ├─ Claude: Produce task list with validation steps
   └─ GPT_Pro: Verify guardrails, build command plan

   Output: docs/SPEC-KIT-900-generic-smoke/tasks.md

3. Quality Gate: after-tasks (3 agents, ~$0.07, 50s)
   Spawn: gemini_flash, claude_haiku, gpt_low
   Prompts: quality-gate-analyze (individual per agent)
   Check: Consistency between spec.md, plan.md, tasks.md
   Consensus: 3/3 or 2/3 → PASS

→ ADVANCE TO IMPLEMENT
```

### Stage 3: IMPLEMENT

```
1. Guardrail Check (native, FREE, <1s)
   ✓ spec-id-validation
   ✓ spec-files
   ✓ clean-tree: plan.md, tasks.md excluded
   ✓ implement-stage: plan.md and tasks.md exist

2. Regular Stage: Implement (4 agents, ~$0.11, 8-12min)
   Spawn: gemini_flash, claude_haiku, gpt_codex, gpt_pro
   Prompts: spec-implement (individual per agent) ← FIXED Session 3

   Context Provided to Each Agent:
   ├─ spec.md
   ├─ plan.md (from Plan stage)
   └─ tasks.md (from Tasks stage)

   Agent Execution (Parallel):
   ├─ Gemini: Map code paths, flag integration points
   ├─ Claude: Outline implementation strategy, operations
   ├─ GPT_Codex: Generate diff_proposals, test_commands ← CODE SPECIALIST
   └─ GPT_Pro: Validate feasibility, build checklist

   Output: docs/SPEC-KIT-900-generic-smoke/implement.md

→ ADVANCE TO VALIDATE
```

### Stage 4: VALIDATE

```
1. Guardrail Check (native, FREE, <1s)
   ✓ All validations

2. Regular Stage: Validate (3 agents, ~$0.35, 10-12min)
   Spawn: gemini_flash, claude_haiku, gpt_pro
   Prompts: spec-validate (individual per agent)

   Context:
   ├─ spec.md
   ├─ plan.md
   ├─ tasks.md
   └─ implement.md (from Implement stage)

   Output: docs/SPEC-KIT-900-generic-smoke/validate.md

→ ADVANCE TO AUDIT
```

### Stage 5: AUDIT

```
1. Guardrail Check (native, FREE, <1s)

2. Regular Stage: Audit (3 agents, ~$0.80, 10-12min)
   Spawn: gemini_pro, claude_sonnet, gpt_pro (HIGH reasoning)
   Prompts: spec-audit (individual per agent)

   Note: Premium models for security/compliance

   Output: docs/SPEC-KIT-900-generic-smoke/audit.md

→ ADVANCE TO UNLOCK
```

### Stage 6: UNLOCK

```
1. Guardrail Check (native, FREE, <1s)

2. Regular Stage: Unlock (3 agents, ~$0.80, 10-12min)
   Spawn: gemini_pro, claude_sonnet, gpt_pro (HIGH reasoning)
   Prompts: spec-unlock (individual per agent)

   Output: docs/SPEC-KIT-900-generic-smoke/unlock.md

✅ PIPELINE COMPLETE
```

---

## Complete Cost Breakdown: `/speckit.auto`

| Component | Cost | Duration |
|-----------|------|----------|
| **Guardrails** (6 stages × FREE) | $0.00 | <6s |
| **Quality Gates** (3 gates × $0.07) | $0.21 | ~150s |
| **Native Checks** (clarify, analyze, checklist) | $0.00 | <3s |
| **Plan** (Tier 2, 3 agents) | $0.35 | 10-12min |
| **Tasks** (Tier 2, 3 agents) | $0.35 | 10-12min |
| **Implement** (Tier 2, 4 agents) | $0.11 | 8-12min |
| **Validate** (Tier 2, 3 agents) | $0.35 | 10-12min |
| **Audit** (Tier 3, 3 premium) | $0.80 | 10-12min |
| **Unlock** (Tier 3, 3 premium) | $0.80 | 10-12min |
| **TOTAL** | **~$2.97** | **45-60min** |

---

## Agent Spawning Mechanism (After Session 3 Fix)

### Before Fix (Broken)

```rust
// spec_prompts.rs - Build mega-bundle
let bundle = format!(
    "## Gemini\n{}\n## Claude\n{}\n## GPT_Pro\n{}",
    gemini_prompt, claude_prompt, gpt_pro_prompt
);

// agent_orchestrator.rs - Send SAME bundle to all
for agent in [gemini, claude, gpt_pro] {
    spawn(agent, bundle.clone());  // ❌ Everyone gets same bundle
}
```

**Problem**: All agents see everyone's instructions. Must parse out their section.

---

### After Fix (Correct) ✅

```rust
// agent_orchestrator.rs:38-107 - Build individual prompt
async fn build_individual_agent_prompt(
    spec_id: &str,
    stage: SpecStage,
    agent_name: &str,  // "gemini" | "claude" | "gpt_codex" | "gpt_pro"
    cwd: &Path,
) -> Result<String, String> {
    // 1. Load prompts.json
    let prompts = load_prompts_json()?;

    // 2. Get THIS agent's template
    let template = prompts[stage.key()][agent_name]["prompt"];

    // 3. Build context with prior stage outputs
    let context = build_context(spec_id, stage, cwd)?;
    // - spec.md (always)
    // - plan.md (if stage > Plan)
    // - tasks.md (if stage >= Implement)

    // 4. Replace variables
    let prompt = template
        .replace("${SPEC_ID}", spec_id)
        .replace("${CONTEXT}", &context);

    Ok(prompt)  // UNIQUE prompt for this agent
}

// agent_orchestrator.rs:142-177 - Spawn with individual prompts
for agent_name in [gemini, claude, gpt_codex, gpt_pro] {
    // Build UNIQUE prompt for THIS agent
    let prompt = build_individual_agent_prompt(spec_id, stage, agent_name, cwd).await?;

    // Spawn with INDIVIDUAL prompt
    spawn(agent, prompt);  // ✅ Each gets tailored instructions
}
```

**Benefit**: Each agent sees only their instructions. Clean separation.

---

## Data Flow Through Stages

```
/speckit.auto SPEC-KIT-900
    │
    ├─ INPUT: spec.md (requirements)
    │
    ├─ PLAN STAGE
    │   ├─ Context: spec.md
    │   ├─ Agents: 3 (gemini, claude, gpt_pro)
    │   ├─ Output: plan.md
    │   └─ SQLite: 3 artifacts + 1 synthesis
    │
    ├─ TASKS STAGE
    │   ├─ Context: spec.md + plan.md
    │   ├─ Agents: 3 (gemini, claude, gpt_pro)
    │   ├─ Output: tasks.md
    │   └─ SQLite: 3 artifacts + 1 synthesis
    │
    ├─ IMPLEMENT STAGE
    │   ├─ Context: spec.md + plan.md + tasks.md
    │   ├─ Agents: 4 (gemini, claude, gpt_codex, gpt_pro)
    │   ├─ Output: implement.md
    │   └─ SQLite: 4 artifacts + 1 synthesis
    │
    ├─ VALIDATE STAGE
    │   ├─ Context: spec.md + plan.md + tasks.md + implement.md
    │   ├─ Agents: 3 (gemini, claude, gpt_pro)
    │   ├─ Output: validate.md
    │   └─ SQLite: 3 artifacts + 1 synthesis
    │
    ├─ AUDIT STAGE (Premium)
    │   ├─ Context: All previous + guardrail outputs
    │   ├─ Agents: 3 (gemini_pro, claude_sonnet, gpt_pro HIGH)
    │   ├─ Output: audit.md
    │   └─ SQLite: 3 artifacts + 1 synthesis
    │
    └─ UNLOCK STAGE (Premium)
        ├─ Context: All previous + audit results
        ├─ Agents: 3 (gemini_pro, claude_sonnet, gpt_pro HIGH)
        ├─ Output: unlock.md
        └─ SQLite: 3 artifacts + 1 synthesis
```

---

## Execution Timeline Example

**For `/speckit.auto SPEC-KIT-900`** (typical timing):

```
00:00 - Start
00:01 - Guardrail: Plan (native, <1s)
00:01 - Quality Gate: before-specify (3 agents, 50s)
00:51 - Regular Stage: Plan (3 agents spawn)
04:51 - Plan agents complete, synthesize plan.md
04:52 - Quality Gate: after-specify (3 agents, 50s)
05:42 - Guardrail: Tasks (native, <1s)
05:43 - Regular Stage: Tasks (3 agents spawn)
09:43 - Tasks agents complete, synthesize tasks.md
09:44 - Quality Gate: after-tasks (3 agents, 50s)
10:34 - Guardrail: Implement (native, <1s)
10:35 - Regular Stage: Implement (4 agents spawn) ← 4 AGENTS
18:35 - Implement agents complete, synthesize implement.md
18:36 - Guardrail: Validate (native, <1s)
18:37 - Regular Stage: Validate (3 agents spawn)
22:37 - Validate complete, synthesize validate.md
22:38 - Guardrail: Audit (native, <1s)
22:39 - Regular Stage: Audit (3 premium agents)
32:39 - Audit complete, synthesize audit.md
32:40 - Guardrail: Unlock (native, <1s)
32:41 - Regular Stage: Unlock (3 premium agents)
42:41 - Unlock complete, synthesize unlock.md
42:42 - ✅ PIPELINE COMPLETE

Total: ~42-45 minutes, ~$2.97
```

---

## SQLite Database State After Complete Run

### agent_executions Table

| agent_id | spec_id | stage | phase_type | agent_name | spawned_at | completed_at |
|----------|---------|-------|------------|------------|------------|--------------|
| uuid-1 | SPEC-KIT-900 | Plan | quality_gate | gemini | 00:01:00 | 00:01:50 |
| uuid-2 | SPEC-KIT-900 | Plan | quality_gate | claude | 00:01:00 | 00:01:50 |
| uuid-3 | SPEC-KIT-900 | Plan | quality_gate | code | 00:01:00 | 00:01:50 |
| uuid-4 | SPEC-KIT-900 | Plan | regular_stage | gemini | 00:51:00 | 04:51:00 |
| uuid-5 | SPEC-KIT-900 | Plan | regular_stage | claude | 00:51:00 | 04:51:00 |
| uuid-6 | SPEC-KIT-900 | Plan | regular_stage | gpt_pro | 00:51:00 | 04:51:00 |
| ... | ... | Tasks | ... | ... | ... | ... |
| uuid-N | SPEC-KIT-900 | Implement | regular_stage | gpt_codex | 10:35:00 | 18:35:00 |

**Total Rows**: ~36 agents (6 quality gates × 3 + 6 regular stages × 3-4)

### consensus_artifacts Table

| spec_id | stage | agent_name | structured_content | raw_response | created_at |
|---------|-------|------------|-------------------|--------------|------------|
| SPEC-KIT-900 | spec-plan | gemini | {...JSON...} | Full text | timestamp |
| SPEC-KIT-900 | spec-plan | claude | {...JSON...} | Full text | timestamp |
| SPEC-KIT-900 | spec-plan | gpt_pro | {...JSON...} | Full text | timestamp |
| SPEC-KIT-900 | spec-tasks | gemini | {...JSON...} | Full text | timestamp |
| ... | ... | ... | ... | ... | ... |

**Total Rows**: ~21 artifacts (6 stages × 3-4 agents)

### consensus_synthesis Table

| spec_id | stage | synthesized_output | output_path | agent_count | created_at |
|---------|-------|-------------------|-------------|-------------|------------|
| SPEC-KIT-900 | spec-plan | Full plan.md | docs/SPEC-*/plan.md | 3 | timestamp |
| SPEC-KIT-900 | spec-tasks | Full tasks.md | docs/SPEC-*/tasks.md | 3 | timestamp |
| SPEC-KIT-900 | spec-implement | Full implement.md | docs/SPEC-*/implement.md | 4 | timestamp |
| ... | ... | ... | ... | ... | ... |

**Total Rows**: 6 syntheses (one per stage)

---

## Session 3 Fixes Applied

### Critical Architecture Fix

**File**: `agent_orchestrator.rs`

**Changes**:
1. Added `build_individual_agent_prompt()` (lines 38-107)
   - Loads prompts.json
   - Extracts agent-specific template
   - Builds context with prior stage artifacts
   - Returns unique prompt per agent

2. Modified spawn loop (lines 150-177)
   - Build individual prompt for EACH agent
   - Spawn with tailored instructions
   - No more mega-bundle

**Impact**:
- ✅ Each agent gets only their instructions
- ✅ Proper context with prior stage outputs
- ✅ Matches proven quality gate pattern
- ✅ Enables future sequential execution

---

## What's Still Broken (Known Issues)

### Issue 1: Prompt Variable Substitution

**File**: `spec_prompts.rs:402-404`

```rust
// Current (placeholder text):
"PREVIOUS_OUTPUTS.gemini" → "Gemini findings stored in local-memory"

// Needed (actual output):
"PREVIOUS_OUTPUTS.gemini" → [actual gemini JSON output]
```

**Impact**: Prompts reference placeholder text, not actual agent outputs
**Mitigation**: Agents now reference file artifacts (plan.md, tasks.md exist on disk)
**Future Fix**: Implement true sequential execution with output injection

### Issue 2: Parallel vs Sequential

**Current**: All agents spawn in parallel (faster, ~4min per stage)
**Prompts Expect**: Sequential with output passing
**Tradeoff**: Speed vs collaborative refinement

**Decision**: Keep parallel for now (faster), but architecture now supports sequential

---

## Summary

**Session 3 Achievement**: Fixed critical architectural mismatch

**Before**:
- ❌ Mega-bundle to all agents
- ❌ Agents confused by mixed instructions
- ❌ No proper data flow

**After**:
- ✅ Individual prompts per agent
- ✅ Each agent sees only their task
- ✅ Proper context with prior artifacts
- ✅ Architecture matches quality gates

**Tree**: Clean
**Binary**: hash 9127c7aa
**Ready**: For full pipeline testing with proper agent isolation
