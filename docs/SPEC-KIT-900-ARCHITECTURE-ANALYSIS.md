# SPEC-KIT-900 Complete Architecture Analysis & Critical Bug Report

**Date**: 2025-11-03 (Session 3)
**Severity**: 🔴 **CRITICAL** - Architectural Mismatch
**Status**: Under Investigation
**Impact**: All regular stages (Plan, Tasks, Implement, Validate, Audit, Unlock)

---

## Executive Summary

The spec-kit system has a **fundamental architectural bug** where the prompt building strategy (mega-bundle for orchestrator) doesn't match the agent spawning strategy (direct parallel spawning).

**Root Cause**: Regular stages were designed for an orchestrator agent pattern but were migrated to direct spawning without fixing the prompts.

**Impact**: Agents receive a mega-bundle containing ALL agent prompts and must parse out their section. This causes:
- Variable substitution failures (`${PREVIOUS_OUTPUTS.gemini}` → placeholder text)
- Agents confused by seeing other agents' instructions
- No actual data flow between agents despite prompts expecting it

**Quality Gates**: ✅ Work correctly (individual prompts per agent)
**Regular Stages**: ❌ Broken (same bundle to all agents)

---

## Complete Workflow Documentation

### 1. System Components

```
┌─────────────────────────────────────────────────────────────┐
│                  SPEC-KIT AUTOMATION SYSTEM                  │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────────┐  │
│  │ Guardrails  │ → │ Quality Gates│ → │ Regular Stages  │  │
│  │ (Validation)│   │ (Pre-checks) │   │ (Multi-Agent)   │  │
│  └─────────────┘   └──────────────┘   └─────────────────┘  │
│        │                  │                      │           │
│        ↓                  ↓                      ↓           │
│  Native Check      3 Agents (✅)        3-4 Agents (❌)     │
│  - spec-id         Individual          Same Bundle         │
│  - clean-tree      Prompts             to All              │
│  - files exist                                              │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              SQLite Consensus Database                │  │
│  │  - agent_executions (spawn tracking)                  │  │
│  │  - consensus_artifacts (agent outputs)                │  │
│  │  - consensus_synthesis (final outputs)                │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 2. Command Flow: `/speckit.auto SPEC-KIT-900`

```
USER INPUT
  │
  ├─ /speckit.auto SPEC-KIT-900
  │
  ↓
[pipeline_coordinator.rs:28] handle_spec_auto()
  │
  ├─ Validate config
  ├─ Check evidence size (<50MB)
  ├─ Create SpecAutoState(spec_id, goal, stages[])
  │    stages = [Plan, Tasks, Implement, Validate, Audit, Unlock]
  │
  ↓
[pipeline_coordinator.rs:96] advance_spec_auto()
  │
  ├─ FOR EACH stage in stages:
  │   │
  │   ├─ STEP 1: Guardrail Check
  │   │   ├─ [native_guardrail.rs:75] run_native_guardrail()
  │   │   ├─ Checks: spec-id, files, clean-tree, stage-ready
  │   │   └─ Pass → Continue | Fail → Halt
  │   │
  │   ├─ STEP 2: Quality Gate (if applicable)
  │   │   ├─ before-specify (before Plan)
  │   │   ├─ after-specify (after Plan, before Tasks)
  │   │   ├─ after-tasks (after Tasks, before Implement)
  │   │   │
  │   │   ├─ [quality_gate_handler.rs:1034] spawn_quality_gate_agents_native()
  │   │   │   ├─ FOR agent in [gemini, claude, code]:
  │   │   │   │   ├─ Get INDIVIDUAL prompt from prompts.json ✅
  │   │   │   │   ├─ Build prompt with SPEC context ✅
  │   │   │   │   ├─ Spawn with UNIQUE prompt ✅
  │   │   │   │   └─ Record: phase_type="quality_gate"
  │   │   │   │
  │   │   │   └─ Poll → Collect → Store to SQLite
  │   │   │
  │   │   └─ Pass (3/3 or 2/3) → Continue | Fail → Halt
  │   │
  │   ├─ STEP 3: Regular Stage Execution
  │   │   │
  │   │   ├─ [agent_orchestrator.rs:185] auto_submit_spec_stage_prompt()
  │   │   │   │
  │   │   │   ├─ [spec_prompts.rs:460-549] build_stage_prompt_with_mcp()
  │   │   │   │   ├─ Build MEGA-BUNDLE: ❌ ARCHITECTURAL ISSUE
  │   │   │   │   │   ├─ Header: "run the agents below in parallel"
  │   │   │   │   │   ├─ ## Gemini Ultra — Research
  │   │   │   │   │   │   └─ Gemini's prompt (references local-memory)
  │   │   │   │   │   ├─ ## Claude Sonnet — Synthesis
  │   │   │   │   │   │   └─ Claude's prompt (references ${PREVIOUS_OUTPUTS.gemini})
  │   │   │   │   │   ├─ ## GPT-5 Codex — Code Diff
  │   │   │   │   │   │   └─ GPT_Codex's prompt
  │   │   │   │   │   └─ ## GPT-5 — Arbiter
  │   │   │   │   │       └─ GPT_Pro's prompt (references all previous)
  │   │   │   │   │
  │   │   │   │   └─ Variable substitution:
  │   │   │   │       ├─ ${PREVIOUS_OUTPUTS.gemini} → "Gemini findings stored in local-memory" ❌
  │   │   │   │       └─ NOT actual Gemini output!
  │   │   │   │
  │   │   │   ├─ [agent_orchestrator.rs:481-488] spawn_regular_stage_agents_native()
  │   │   │   │   │
  │   │   │   │   ├─ FOR agent in expected_agents:  # 3 for Plan, 4 for Implement
  │   │   │   │   │   ├─ Map: canonical_name → config_name
  │   │   │   │   │   ├─ Spawn: create_agent_from_config_name(
  │   │   │   │   │   │            config_name,
  │   │   │   │   │   │            prompt.to_string(),  ← SAME BUNDLE FOR ALL! ❌
  │   │   │   │   │   │         )
  │   │   │   │   │   └─ SQLite: phase_type="regular_stage"
  │   │   │   │   │
  │   │   │   │   └─ Return: Vec<AgentSpawnInfo> (3-4 agents)
  │   │   │   │
  │   │   │   ├─ [agent_orchestrator.rs:512] Background polling:
  │   │   │   │   ├─ Poll every 500ms (10min timeout)
  │   │   │   │   ├─ Check: All agents in terminal state?
  │   │   │   │   └─ Send: RegularStageAgentsComplete event
  │   │   │   │
  │   │   │   └─ [app.rs:2728] Event handler:
  │   │   │       └─ Call: on_spec_auto_agents_complete()
  │   │   │
  │   │   ├─ [agent_orchestrator.rs:646] on_spec_auto_agents_complete()
  │   │   │   ├─ Collect: All agent responses
  │   │   │   ├─ Store: SQLite consensus_artifacts (one per agent)
  │   │   │   ├─ Cache: state.agent_responses_cache
  │   │   │   └─ Trigger: check_consensus_and_advance_spec_auto()
  │   │   │
  │   │   ├─ [pipeline_coordinator.rs:531] check_consensus_and_advance_spec_auto()
  │   │   │   │
  │   │   │   ├─ [pipeline_coordinator.rs:913] synthesize_from_cached_responses()
  │   │   │   │   ├─ Input: agent_responses[] (3-4 responses)
  │   │   │   │   ├─ Parse: Extract JSON from each response
  │   │   │   │   ├─ Build: plan.md / tasks.md / implement.md
  │   │   │   │   └─ Store: SQLite consensus_synthesis
  │   │   │   │
  │   │   │   └─ Advance: state.current_index++ (next stage)
  │   │   │
  │   │   └─ REPEAT for next stage
  │   │
  │   └─ FINAL: All stages complete
  │
  └─ OUTPUT: plan.md, tasks.md, implement.md, etc.
```

---

## 🔴 Critical Bug: Prompt Distribution Mismatch

### How Quality Gates Work (✅ CORRECT)

**File**: `native_quality_gate_orchestrator.rs:76-105`

```rust
// FOR EACH agent:
for (agent_name, config_name) in [("gemini", "gemini_flash"), ("claude", "claude_haiku"), ("code", "gpt_low")] {
    // Step 1: Get INDIVIDUAL prompt for this agent
    let prompt_template = gate_prompts.get(agent_name)  // ← "gemini" section only
        .get("prompt")
        .as_str();

    // Step 2: Build prompt with context for THIS agent
    let prompt = build_quality_gate_prompt(spec_id, gate, prompt_template, cwd).await;

    // Step 3: Spawn THIS agent with ITS prompt
    manager.create_agent_from_config_name(
        config_name,
        agent_configs,
        prompt,  // ← UNIQUE prompt for this agent
        true,
        batch_id,
    );
}
```

**Result**: Each agent gets its own tailored prompt ✅

---

### How Regular Stages Work (❌ BROKEN)

**File**: `spec_prompts.rs:460-549`

```rust
// Build ONE mega-bundle
let mut bundle = String::new();
bundle.push_str("# /spec-plan — SPEC-KIT-900\n\n");
bundle.push_str("run the agents below in parallel using these prompts\n\n");

// Append ALL agent prompts to bundle
bundle.push_str("## Gemini Ultra — Research\n");
bundle.push_str(&gemini_prompt);  // Full prompt with "Output JSON: {...}"

bundle.push_str("## Claude Sonnet — Synthesis\n");
bundle.push_str(&claude_prompt);  // References ${PREVIOUS_OUTPUTS.gemini}

bundle.push_str("## GPT-5 Codex — Code Diff\n");
bundle.push_str(&gpt_codex_prompt);

bundle.push_str("## GPT-5 — Arbiter\n");
bundle.push_str(&gpt_pro_prompt);  // References all previous outputs

return Ok(bundle);  // ONE GIANT STRING
```

**File**: `agent_orchestrator.rs:481-488`

```rust
// Get the bundle (ONE prompt containing all agent sections)
let prompt_owned = prompt.clone();

let spawn_result = spawn_regular_stage_agents_native(
    &cwd, &spec_id_owned, stage,
    &prompt_owned,  // ← THE BUNDLE
    &expected_agents_owned,
    &agent_configs_owned,
).await;

// Inside spawn function (lines 68-87):
for agent_name in expected_agents {
    manager.create_agent_from_config_name(
        config_name,
        agent_configs,
        prompt.to_string(),  // ← SAME BUNDLE to gemini, claude, gpt_codex, gpt_pro!
        false,
        batch_id,
    );
}
```

**Result**: All agents get the SAME mega-bundle containing everyone's instructions ❌

---

## Variable Substitution Reality

**File**: `spec_prompts.rs:399-448`

### What Prompts Expect

```json
// Claude's prompt (prompts.json:38)
"Inputs: ... Gemini analysis (${PREVIOUS_OUTPUTS.gemini})"
```

### What Actually Happens

```rust
// Line 402-404: Implement stage
replacements.push((
    "PREVIOUS_OUTPUTS.gemini".into(),
    "Gemini Ultra findings stored in local-memory (spec-tracker domain).".into(),
    // ↑ GENERIC PLACEHOLDER, not actual Gemini output!
));

// Line 423-426: Implement stage
replacements.push((
    "PREVIOUS_OUTPUTS.tasks".into(),
    "Latest /spec-tasks consensus stored in docs/SPEC-*/tasks.md and local-memory.".into(),
    // ↑ Points to file, but doesn't inject content!
));
```

**Reality**: Variables are replaced with **placeholder instructions**, not actual agent outputs!

---

## Complete Stage-by-Stage Analysis

### PLAN STAGE

**Prompts** (prompts.json:2-16):
```
Gemini (Research):
  - Input: SPEC packet, template
  - Output: research_summary, questions
  - Dependencies: NONE ✅

Claude (Synthesis):
  - Input: SPEC, Gemini research (${PREVIOUS_OUTPUTS.gemini})
  - Output: work_breakdown, acceptance_mapping, risks
  - Dependencies: Gemini ❌

GPT_Pro (Arbiter):
  - Input: SPEC, Gemini + Claude outputs (${PREVIOUS_OUTPUTS})
  - Output: feasibility_notes, final_plan, consensus
  - Dependencies: Gemini + Claude ❌
```

**Intended Flow**: Gemini → Claude → GPT_Pro (SEQUENTIAL)
**Actual Flow**: All spawn in parallel with same bundle (PARALLEL)
**Mismatch**: ❌ YES

---

### TASKS STAGE

**Prompts** (prompts.json:17-31):
```
Gemini (Researcher):
  - Input: Template, context
  - Dependencies: NONE ✅

Claude (Synthesizer):
  - Input: SPEC, Gemini analysis (${PREVIOUS_OUTPUTS.gemini}), Plan (${PREVIOUS_OUTPUTS.plan})
  - Dependencies: Gemini + Plan file ❌

GPT_Pro (Executor):
  - Input: Gemini/Claude JSON outputs
  - Dependencies: Gemini + Claude ❌
```

**Intended Flow**: Gemini → Claude → GPT_Pro (SEQUENTIAL)
**Actual Flow**: All spawn in parallel (PARALLEL)
**Mismatch**: ❌ YES

---

### IMPLEMENT STAGE

**Prompts** (prompts.json:32-46):
```
Gemini (Code Path Analyzer):
  - Input: CONTEXT, task list (${PREVIOUS_OUTPUTS.tasks})
  - Dependencies: tasks.md file ⚠️

Claude (Strategy):
  - Input: Template, context
  - Note: Prompt says "Reference Gemini findings" but no variable
  - Dependencies: Implicit Gemini ❌

GPT_Codex (Code Generator):
  - Input: Template, context
  - Dependencies: NONE (but design implies using others)

GPT_Pro (QA):
  - Input: Template, validate feasibility
  - Dependencies: All previous agents ❌
```

**Intended Flow**: Parallel-ish with shared context
**Actual Flow**: Parallel with same bundle
**Mismatch**: ⚠️ PARTIAL (design unclear)

---

### VALIDATE, AUDIT, UNLOCK STAGES

**Prompts**: Similar pattern
- Gemini: Independent research
- Claude: Synthesis with ${PREVIOUS_OUTPUTS}
- GPT_Pro: Arbiter with all previous outputs

**Mismatch**: ❌ YES (same issue across all stages)

---

## Quality Gates (Working Correctly)

### CLARIFY, ANALYZE, CHECKLIST

**Code**: `native_quality_gate_orchestrator.rs:76-105`

```rust
// For each agent:
for (agent_name, config_name) in agents {
    // Get THIS agent's prompt ONLY
    let prompt_template = gate_prompts
        .get(agent_name)  // ← "gemini" or "claude" or "code"
        .get("prompt")
        .as_str();

    // Build custom prompt for this agent
    let prompt = build_quality_gate_prompt(spec_id, gate, prompt_template, cwd);

    // Spawn with UNIQUE prompt
    manager.create_agent_from_config_name(config_name, agents, prompt, ...);
}
```

**Prompts** (prompts.json:136-180):
```
quality-gate-clarify:
  gemini: Independent (analyze SPEC for ambiguities)
  claude: Independent (resolve ambiguities)
  code: Independent (implementation perspective)

  NO dependencies on each other! ✅
```

**Why it works**:
1. Prompts are truly independent (no ${PREVIOUS_OUTPUTS})
2. Each agent gets customized prompt
3. Parallel execution matches prompt design
4. Consensus happens in synthesis, not during execution

---

## The Design Intent vs Implementation Gap

### Original Design (Inferred from Prompts)

**Option A: Orchestrator Agent Pattern** (Likely Original)
```
User → Spawn Orchestrator Agent
     → Orchestrator reads mega-bundle
     → Orchestrator spawns Gemini → waits → collects
     → Orchestrator spawns Claude (injects Gemini output) → waits
     → Orchestrator spawns GPT_Codex → waits
     → Orchestrator spawns GPT_Pro (injects all outputs) → waits
     → Orchestrator synthesizes → produces plan.md
```

**Evidence**:
- spec_prompts.rs:462: "run the agents below in parallel using these prompts"
- This instruction is FOR an orchestrator agent!
- Prompts.json agent sections are meant to be read BY the orchestrator

### Current Implementation (SPEC-KIT-900 Migration)

**Option B: Direct Spawning** (Current Broken State)
```
User → Build mega-bundle
     → Spawn all agents IN PARALLEL
     → Send SAME bundle to each agent
     → Each agent must parse out their section
     → Hope they ignore other agents' sections
     → Collect responses → synthesize
```

**Why it's broken**:
- Agents aren't orchestrators - they can't spawn sub-agents
- Bundle contains conflicting instructions for different agents
- Variables like ${PREVIOUS_OUTPUTS} have placeholder text
- No actual data flow between agents

---

## Evidence: The Mega-Bundle Contents

**Example for Plan Stage** (spec_prompts.rs:460-549):

```
# /spec-plan — SPEC-KIT-900

Leverage local-memory before starting, then run the agents below in parallel using these prompts.
Record outputs back into local-memory (spec-tracker, impl-notes, docs-ops).

Goal: [user goal]

Prompt version: 20251002-plan-a

## Local-memory context
- [entries from local-memory MCP]

## HTTP MCP (HAL)
- [HAL instructions]

## Gemini Ultra — Research
Context:
- Template: ~/.code/templates/plan-template.md (reference structure)
[SPEC content...]

Task:
Survey SPEC SPEC-KIT-900. Summarize:
1. Acceptance criteria and evidence requirements.
2. Conflicts, gaps, stale telemetry, blocked tasks.
[...]

Output JSON:
{
  "stage": "spec-plan",
  "agent": "gemini",
  [...]
}

## Claude Sonnet 4.5 — Synthesis
Inputs:
- SPEC packet
- Gemini research (Gemini Ultra findings stored in local-memory (spec-tracker domain).)
- Template: ~/.code/templates/plan-template.md

Produce JSON:
{
  "stage": "spec-plan",
  "agent": "claude",
  [...]
}

## GPT-5 — Arbiter & QA
Inputs:
- SPEC packet
- Gemini and Claude outputs (Refer to Gemini + Claude outputs captured in local-memory for consensus notes.)
- Template: ~/.code/templates/plan-template.md

Validate feasibility and emit JSON:
{
  "stage": "spec-plan",
  "agent": "gpt_pro",
  [...]
}
```

**This ENTIRE bundle is sent to**:
- Gemini (who sees Claude and GPT_Pro instructions too)
- Claude (who sees everyone's instructions)
- GPT_Pro (who sees everyone's instructions)

---

## Why The System "Works" Despite Being Broken

### Agent Behavior Adaptation

**Agents appear to**:
1. Scan the bundle for their name/role header
2. Parse out their specific section
3. Ignore other agents' sections
4. Execute their instructions
5. Output JSON with correct "agent": field

**Evidence**:
- Consensus artifacts ARE being created
- plan.md, tasks.md ARE being generated
- Each agent outputs correct JSON with proper "agent" field
- Synthesis successfully combines outputs

**But we lose**:
- Actual collaborative refinement
- Claude can't use Gemini's actual findings
- GPT_Pro can't arbitrate actual conflicts
- It's parallel work, not sequential synthesis

---

## Fix Strategies

### OPTION 1: Fix Prompts (Quick, Low Risk) ⭐ RECOMMENDED

**Make prompts match current parallel execution**:

```json
// prompts.json - Implement stage
"claude": {
  "prompt": "Outline implementation strategy as JSON.

  Context available:
  - SPEC packet (provided above)
  - Previous stage artifacts in docs/SPEC-*/plan.md, tasks.md
  - Local-memory entries (search for relevant patterns)

  DO NOT expect other agent outputs - you are running in parallel.

  Output JSON: {...}"
}
```

**Changes needed**:
- Remove all `${PREVIOUS_OUTPUTS.agent_name}` references
- Update prompts to be independent
- Keep file-based context (plan.md, tasks.md exist on disk)
- Keep local-memory search instructions

**Benefits**:
- ✅ Aligns prompts with current code
- ✅ No code changes required
- ✅ Maintains parallel execution (faster)
- ✅ Low risk

**Drawbacks**:
- ⚠️ Loses intended collaborative refinement
- ⚠️ Agents work in isolation

---

### OPTION 2: Fix Code (Medium Risk)

**Make code match sequential prompt design**:

**File**: `agent_orchestrator.rs` - Create new function:

```rust
async fn spawn_regular_stage_agents_sequential(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    base_context: &str,
    expected_agents: &[String],
    agent_configs: &[AgentConfig],
) -> Result<Vec<(String, String)>, String> {
    let mut agent_outputs = Vec::new();

    for agent_name in expected_agents {
        // Build prompt for THIS agent using previous outputs
        let prompt = build_individual_agent_prompt(
            stage,
            agent_name,
            base_context,
            &agent_outputs,  // Previous agent outputs
        )?;

        // Spawn and WAIT for completion
        let agent_id = spawn_and_wait_for_agent(config_name, prompt).await?;

        // Collect output
        let output = get_agent_result(agent_id).await?;
        agent_outputs.push((agent_name.clone(), output));

        // Now NEXT agent can use this output
    }

    Ok(agent_outputs)
}
```

**Benefits**:
- ✅ True sequential execution
- ✅ Real data flow between agents
- ✅ Claude can use actual Gemini output
- ✅ Matches prompt design intent

**Drawbacks**:
- ⚠️ Slower (sequential vs parallel)
- ⚠️ Medium implementation complexity
- ⚠️ Need to build individual prompts per agent

---

### OPTION 3: Hybrid (Best of Both)

**Parallel execution with proper prompt distribution**:

**File**: `agent_orchestrator.rs` - Modify spawn function:

```rust
async fn spawn_regular_stage_agents_native(...) {
    // Get individual agent prompts from prompts.json (like quality gates)
    let prompts_json = load_prompts_json(cwd)?;
    let stage_prompts = prompts_json.get(stage.key())?;

    for agent_name in expected_agents {
        // Get THIS agent's prompt
        let agent_prompt_template = stage_prompts
            .get(agent_name)
            .get("prompt")
            .as_str()?;

        // Build prompt with context for THIS agent
        let prompt = build_individual_prompt(
            spec_id,
            stage,
            agent_name,
            agent_prompt_template,
            cwd,
        ).await?;

        // Spawn with UNIQUE prompt
        manager.create_agent_from_config_name(
            config_name,
            agent_configs,
            prompt,  // ← INDIVIDUAL prompt
            false,
            batch_id,
        );
    }
}
```

**Benefits**:
- ✅ Each agent gets tailored prompt
- ✅ Can make prompts independent (parallel)
- ✅ OR support sequential with output passing
- ✅ Matches quality gate pattern (proven to work)
- ✅ Flexible for future enhancements

**Drawbacks**:
- ⚠️ Moderate code changes
- ⚠️ Need to update prompts to be independent OR implement output passing

---

## Immediate Recommendations

### Phase 1: Quick Fix (This Session)

**Update prompts.json to remove ${PREVIOUS_OUTPUTS} dependencies**:

1. Make all agent prompts independent
2. Reference file artifacts (plan.md, tasks.md exist on disk)
3. Use local-memory search for context (not direct variable injection)

**Files to modify**: `docs/spec-kit/prompts.json`
**Estimated time**: 30-60 minutes
**Risk**: Low (only affects prompt text, not code)

### Phase 2: Proper Architecture (Next Session)

**Refactor to match quality gate pattern**:

1. Create `build_individual_agent_prompt()` function
2. Modify `spawn_regular_stage_agents_native()` to get individual prompts
3. Remove `build_stage_prompt_with_mcp()` mega-bundle approach
4. Test each stage independently

**Files to modify**:
- `agent_orchestrator.rs`
- `spec_prompts.rs`
- `pipeline_coordinator.rs` (if needed)

**Estimated time**: 4-6 hours
**Risk**: Medium (touches core orchestration)

---

## Impact on Current Operations

### What's Working Despite Bug

**Consensus IS being generated** because:
1. Agents parse their section from bundle (by header)
2. Each outputs correct JSON with "agent" field
3. Synthesis collects all outputs and merges them
4. File artifacts (plan.md, tasks.md) ARE created

### What's NOT Working

**Collaborative refinement is broken**:
1. Claude doesn't see Gemini's actual findings
2. GPT_Pro doesn't see actual conflicts to arbitrate
3. Agents work in isolation despite prompts suggesting collaboration
4. Quality lower than intended (no iterative refinement)

### Performance Impact

**Actually FASTER because parallel**:
- 3-4 agents run simultaneously (~4-5 minutes total)
- vs Sequential would be 3-4x longer (~12-20 minutes)

**But quality suffers**:
- No building on insights
- No conflict resolution
- No progressive refinement

---

## Conclusion

**Critical Findings**:

1. ❌ **Prompt Distribution Bug**: All agents get same mega-bundle
2. ❌ **Variable Substitution Bug**: ${PREVIOUS_OUTPUTS} → placeholder text
3. ❌ **Architecture Mismatch**: Prompts expect sequential, code does parallel
4. ✅ **Quality Gates**: Work correctly (individual prompts)
5. ⚠️ **System Functional**: Works despite bugs (agents adapt)

**Recommended Action**:

**IMMEDIATE**: Fix prompts.json to remove sequential dependencies (Option 1)
**NEXT SESSION**: Refactor to match quality gate pattern (Option 3)

**This completes the comprehensive architecture investigation.**
