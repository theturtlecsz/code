# SPEC-KIT-902: Nativize Guardrail Scripts

**Status**: Analysis Complete - ARCHITECTURE ISSUE IDENTIFIED
**Created**: 2025-11-29
**Updated**: 2025-11-29 (Session 2 - Deep Dive with ultrathink)
**Author**: Deep Dive Session

---

## Executive Summary

**CRITICAL FINDING**: The `/speckit.*` stage commands (`/speckit.plan`, `/speckit.tasks`, etc.) **still use the orchestrator pattern** when called standalone!

```
CURRENT (orchestrator pattern - BAD):
/speckit.plan SPEC-ID
  → expand_prompt() in commands/plan.rs
  → Submit expanded prompt as TEXT to "code" orchestrator agent
  → Orchestrator interprets prompt
  → Orchestrator spawns sub-agents (unpredictable)
  → Meta-agent spawning ("fetch results", etc.)

/speckit.auto SPEC-ID (CORRECT - direct spawning):
  → auto_submit_spec_stage_prompt()
  → spawn_regular_stage_agents_native()
  → DIRECTLY spawn gemini, claude, gpt_pro
  → Wait for completion (no orchestrator)
  → Synthesize results natively
```

**The direct spawning code EXISTS** - but it's only used by `/speckit.auto`. The standalone stage commands still use the old orchestrator pattern.

---

## The Problem

### Current `/speckit.plan` Flow (Orchestrator Pattern)

```
1. User types: /speckit.plan SPEC-KIT-123

2. commands/plan.rs:
   fn expand_prompt(&self, args: &str) -> Option<String> {
       Some(format_subagent_command("plan", args, None, None).prompt)
   }

3. mod.rs:5173-5176:
   message.ordered_items.clear();
   message.ordered_items.push(InputItem::Text { text: expanded });

4. Expanded prompt submitted to "code" orchestrator agent

5. Orchestrator interprets prompt, spawns agents unpredictably
```

### Desired Flow (Direct Spawning)

```
1. User types: /speckit.plan SPEC-KIT-123

2. commands/plan.rs:
   fn execute(&self, widget: &mut ChatWidget, args: String) {
       // Run native guardrail
       run_native_guardrail(&widget.config.cwd, spec_id, SpecStage::Plan, false);

       // Directly spawn agents
       spawn_regular_stage_agents_native(...)

       // Wait for completion, synthesize, display
   }

3. Agents spawn directly via AgentManager::create_agent_with_config()

4. Results synthesized natively in check_consensus_and_advance_spec_auto()
```

---

## What Already Exists (Native Infrastructure)

### Direct Spawning (Working in /speckit.auto)

| File | Function | Purpose |
|------|----------|---------|
| `agent_orchestrator.rs:751` | `spawn_regular_stage_agents_native()` | Routes to seq/parallel |
| `agent_orchestrator.rs:424` | `spawn_regular_stage_agents_sequential()` | Plan, Tasks, Implement |
| `agent_orchestrator.rs:570` | `spawn_regular_stage_agents_parallel()` | Validate, Audit, Unlock |
| `agent_orchestrator.rs:969` | `auto_submit_spec_stage_prompt()` | Entry point for /speckit.auto |

### Native Guardrails (Working)

| File | Function | Purpose |
|------|----------|---------|
| `native_guardrail.rs:77` | `run_native_guardrail()` | Validation checks |
| `guardrail.rs:765` | `handle_native_guardrail()` | /guardrail.* commands |

### Native Quality (Working)

| File | Function | Purpose |
|------|----------|---------|
| `clarify_native.rs` | `find_ambiguities()` | Pattern matching |
| `analyze_native.rs` | `check_consistency()` | Structural diff |
| `checklist_native.rs` | `score_quality()` | Rubric scoring |

---

## Scope: Make Stage Commands Use Direct Spawning

### Phase 1: Refactor Stage Commands (4-6h)

Modify each stage command to call direct spawning instead of prompt expansion:

**Files to modify:**
- `commands/plan.rs` - `/speckit.plan`
- `commands/plan.rs` - `/speckit.tasks` (same file)
- `commands/plan.rs` - `/speckit.implement`
- `commands/plan.rs` - `/speckit.validate`
- `commands/plan.rs` - `/speckit.audit`
- `commands/plan.rs` - `/speckit.unlock`

**Pattern:**
```rust
impl SpecKitCommand for SpecKitPlanCommand {
    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let spec_id = parse_spec_id(&args);

        // 1. Run native guardrail
        let result = run_native_guardrail(&widget.config.cwd, &spec_id, SpecStage::Plan, false);
        if !result.success {
            display_guardrail_failure(widget, result);
            return;
        }

        // 2. Spawn agents directly (like /speckit.auto does)
        auto_submit_spec_stage_prompt(widget, SpecStage::Plan, &spec_id);
    }

    // Remove expand_prompt() - no longer needed
}
```

### Phase 2: Delete Legacy Code (2-3h)

After stage commands use direct spawning:

1. **Delete orchestrator-related code:**
   - `queue_consensus_runner()` (mod.rs:18001-18049)
   - `parse_spec_stage_invocation()` (mod.rs:17909-17977)
   - Prompt expansion logic in mod.rs:5141-5176

2. **Delete shell scripts:**
   - `consensus_runner.sh` (456 LOC)
   - `common.sh` (423 LOC)
   - `baseline_audit.sh` (82 LOC)
   - `log_agent_runs.sh` (105 LOC)
   - `spec_ops_004/commands/*.sh` (~400 LOC)

3. **Deprecate legacy commands:**
   - Remove `/spec-plan`, `/spec-tasks`, etc.
   - Keep only `/speckit.*` namespace

---

## Architecture After Fix

```
/speckit.plan SPEC-ID
  │
  ├── 1. run_native_guardrail() ← NATIVE
  │     └── SPEC ID validation, clean tree, file structure
  │
  ├── 2. auto_submit_spec_stage_prompt() ← NATIVE
  │     │
  │     ├── build_individual_agent_prompt() for each agent
  │     │
  │     └── spawn_regular_stage_agents_native()
  │           │
  │           ├── spawn_regular_stage_agents_sequential() [Plan, Tasks, Implement]
  │           │     └── AgentManager::create_agent_with_config()
  │           │
  │           └── spawn_regular_stage_agents_parallel() [Validate, Audit, Unlock]
  │                 └── AgentManager::create_agent_with_config()
  │
  ├── 3. Wait for agents (polling or event-based)
  │
  ├── 4. check_consensus_and_advance_spec_auto() ← NATIVE
  │     └── Synthesize results, store to SQLite
  │
  └── 5. Display results in TUI
```

**Key benefits:**
- NO orchestrator agent
- Exactly N agents spawned (predictable)
- Direct control over agent lifecycle
- Immediate TUI visibility
- Native synthesis

---

## Effort Estimate

| Phase | Work | Effort |
|-------|------|--------|
| Phase 1 | Refactor 6 stage commands to use direct spawning | 4-6h |
| Phase 2 | Delete legacy code (~1,500 LOC) | 2-3h |
| **Total** | | **6-9h** |

---

## Verification Checklist

- [ ] `/speckit.plan SPEC-ID` spawns agents directly (no orchestrator)
- [ ] `/speckit.tasks SPEC-ID` spawns agents directly
- [ ] `/speckit.implement SPEC-ID` spawns agents directly
- [ ] `/speckit.validate SPEC-ID` spawns agents directly
- [ ] `/speckit.audit SPEC-ID` spawns agents directly
- [ ] `/speckit.unlock SPEC-ID` spawns agents directly
- [ ] Agent spawning visible in TUI immediately
- [ ] Results synthesized natively
- [ ] No shell scripts called
- [ ] Legacy `/spec-*` commands removed

---

## Files Summary

### Modify
- `commands/plan.rs` - All 6 stage commands

### Delete (Phase 2)
```
codex-rs/tui/src/chatwidget/mod.rs
  ├── queue_consensus_runner()      (lines 18001-18049)
  └── parse_spec_stage_invocation() (lines 17909-17977)

scripts/spec_ops_004/
  ├── consensus_runner.sh     (456 LOC)
  ├── common.sh               (423 LOC)
  ├── baseline_audit.sh       (82 LOC)
  ├── log_agent_runs.sh       (105 LOC)
  └── commands/*.sh           (~400 LOC)
```

### Keep (Evidence scripts)
```
scripts/spec_ops_004/
  ├── evidence_stats.sh       (called by pipeline)
  ├── evidence_archive.sh     (manual tool)
  └── evidence_cleanup.sh     (manual tool)
```

---

## References

- `agent_orchestrator.rs:424-568` - Sequential spawning (CORRECT pattern)
- `agent_orchestrator.rs:570-750` - Parallel spawning (CORRECT pattern)
- `agent_orchestrator.rs:969` - `auto_submit_spec_stage_prompt()` (entry point)
- `native_consensus_executor.rs:1-27` - Comments explaining the elimination of orchestrator
- `mod.rs:5173-5176` - Current orchestrator pattern (to be removed)
