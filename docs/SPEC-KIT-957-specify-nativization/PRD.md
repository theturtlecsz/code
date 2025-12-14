# SPEC-KIT-957: Nativize speckit.specify - Complete SPEC-KIT-902

**Status**: Draft
**Created**: 2025-11-29
**Depends On**: SPEC-KIT-902 (stage command nativization)
**Blocks**: SPEC-KIT-956 full cleanup (subagent config removal)

---

## Problem Statement

SPEC-KIT-902 refactored 6 stage commands (plan/tasks/implement/validate/audit/unlock) to use direct agent spawning, eliminating the orchestrator pattern. However, `speckit.specify` was missed and remains the **only** speckit command still dependent on config.toml `[[subagents.commands]]`.

This creates:
1. **Architectural inconsistency**: 6 commands use native Rust routing, 1 uses config-driven orchestration
2. **Config debt**: Cannot fully remove `[[subagents.commands]]` section (~200 lines)
3. **Maintenance burden**: Two different code paths for similar functionality

---

## Evidence

### Current Implementation (special.rs:157-166)
```rust
fn execute(&self, widget: &mut ChatWidget, args: String) {
    // Routed to subagent orchestrators  <-- LEGACY PATTERN
    let formatted = codex_core::slash_commands::format_subagent_command(
        "speckit.specify",
        &args,
        Some(&widget.config.agents),
        Some(&widget.config.subagent_commands),  // <-- CONFIG DEPENDENCY
    );
    widget.submit_prompt_with_display(args, formatted.prompt);
}
```

### Nativized Commands (plan.rs:30-32)
```rust
fn execute(&self, widget: &mut ChatWidget, args: String) {
    execute_stage_command(widget, args, SpecStage::Plan, "speckit.plan");
    // Uses auto_submit_spec_stage_prompt() -> decide_stage_routing() -> hardcoded Rust
}
```

### SpecStage Enum Gap
```rust
pub enum SpecStage {
    Plan, Tasks, Implement, Validate, Audit, Unlock,  // Main pipeline
    Clarify, Analyze, Checklist,                       // Quality commands
    // NO Specify variant!
}
```

---

## Scope

### In Scope
1. Add `Specify` variant to `SpecStage` enum
2. Implement prompt building for Specify stage
3. Add Tier 1 routing (single agent, gpt5-low)
4. Modify `SpecKitSpecifyCommand::execute()` to use direct execution
5. Remove `speckit.specify` from config.toml `[[subagents.commands]]`
6. Remove ALL remaining speckit.* subagent configs (plan/tasks/implement/validate/audit/unlock)

### Out of Scope
- Generic subagent commands (/plan, /solve, /code) - separate feature
- Agent definition cleanup (`[[agents]]`) - still needed
- Quality gate config (`[quality_gates]`) - still needed

---

## Requirements

### R1: Add SpecStage::Specify Variant

**File**: `codex-rs/tui/src/spec_prompts.rs`

```rust
pub enum SpecStage {
    // Pre-pipeline
    Specify,  // NEW
    // Main 6-stage pipeline
    Plan,
    Tasks,
    Implement,
    Validate,
    Audit,
    Unlock,
    // Quality commands
    Clarify,
    Analyze,
    Checklist,
}
```

Update all match arms:
- `key()` → `"spec-specify"`
- `command_name()` → `"specify"`
- `display_name()` → `"Specify"`
- `is_quality_command()` → `false` (it's a pipeline command)
- `all()` → Consider whether to include (it precedes main pipeline)

### R2: Implement Specify Prompt Building

**File**: `codex-rs/tui/src/spec_prompts.rs`

Add prompt template for Specify stage. Reference existing `orchestrator-instructions` from config.toml:

```
Single high-reasoning GPT-5 Codex session with staged context.

## Phase 1: Context Gathering
Read and summarize:
1. PRD.md (if exists) - current state
2. spec.md (if exists) - requirements
3. SPEC.md - tracker entry for this SPEC
4. memory/constitution.md - project principles

## Phase 2: PRD Generation/Refinement
Based on context and user description:
1. Problem Statement - what problem does this solve?
2. Goals - measurable objectives
3. Requirements - specific, testable requirements
4. Acceptance Criteria - how to verify completion
5. Risks & Unknowns - what could go wrong?

## Output
Write complete PRD to docs/SPEC-{id}/PRD.md
Update SPEC.md status if needed
```

### R3: Add Tier 1 Routing for Specify

**File**: `codex-rs/tui/src/chatwidget/spec_kit/ace_route_selector.rs`

Specify uses **Tier 1** (single agent, minimal aggregation):

```rust
pub fn decide_stage_routing(stage: SpecStage, ...) -> StageRoutingDecision {
    let mut effort = match stage {
        SpecStage::Specify => AggregatorEffort::Minimal,  // Tier 1: single agent
        SpecStage::Validate | SpecStage::Unlock => AggregatorEffort::Minimal,
        // ... rest unchanged
    };
}
```

### R4: Modify SpecKitSpecifyCommand to Direct Execution

**File**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`

```rust
impl SpecKitCommand for SpecKitSpecifyCommand {
    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // SPEC-KIT-957: Direct execution (matches SPEC-KIT-902 pattern)
        execute_stage_command(widget, args, SpecStage::Specify, "speckit.specify");
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None  // SPEC-KIT-957: No longer uses orchestrator pattern
    }
}
```

Or call `auto_submit_spec_stage_prompt` directly if Specify needs different handling.

### R5: Remove ALL Speckit Subagent Configs

**File**: `~/.code/config.toml`

Delete these `[[subagents.commands]]` entries:
- `speckit.specify` (was in use, now nativized)
- `speckit.plan` (already dead)
- `speckit.tasks` (already dead)
- `speckit.implement` (already dead)
- `speckit.validate` (already dead)
- `speckit.audit` (already dead)
- `speckit.unlock` (already dead)

**Estimated removal**: ~200 lines

### R6: Update ACE Integration

**File**: `codex-rs/tui/src/chatwidget/spec_kit/ace_prompt_injector.rs`

Add Specify to ACE scope mapping if appropriate:
```rust
fn command_to_scope(command: &str) -> Option<&'static str> {
    match command {
        "speckit.specify" => Some("specify"),  // NEW
        "speckit.plan" => Some("plan"),
        // ...
    }
}
```

---

## Acceptance Criteria

- [ ] `SpecStage::Specify` exists in enum with all match arms
- [ ] `/speckit.specify SPEC-ID` works via direct execution (no config lookup)
- [ ] Prompt builds correctly with context gathering
- [ ] Single-agent routing (Tier 1) applies to Specify
- [ ] ALL `[[subagents.commands]]` entries for speckit.* removed from config.toml
- [ ] TUI builds with 0 warnings
- [ ] Existing tests pass
- [ ] New test: `test_specify_direct_execution`

---

## Implementation Plan

### Phase 1: SpecStage Enum (30 min)
1. Add `Specify` variant to enum
2. Update all match arms (key, command_name, display_name, etc.)
3. Decide if `all()` should include Specify or create `all_with_specify()`

### Phase 2: Prompt Building (45 min)
1. Add `build_specify_prompt()` function
2. Port orchestrator-instructions from config to Rust
3. Add context gathering (PRD, spec, constitution)

### Phase 3: Routing Integration (30 min)
1. Update `decide_stage_routing()` for Specify
2. Add ACE scope mapping if needed
3. Verify Tier 1 (single agent) behavior

### Phase 4: Command Refactor (30 min)
1. Modify `SpecKitSpecifyCommand::execute()`
2. Add `expand_prompt() -> None`
3. Remove config dependency

### Phase 5: Config Cleanup (15 min)
1. Remove all speckit.* `[[subagents.commands]]` entries
2. Add comment explaining native commands
3. Verify TUI builds

### Phase 6: Testing (30 min)
1. Manual: `/speckit.specify SPEC-TEST-001`
2. Unit test: prompt building
3. Integration: verify single-agent routing

**Total Estimate**: ~3 hours

---

## Risks

1. **Prompt quality regression**: Config orchestrator-instructions may have nuances not captured in Rust port
   - **Mitigation**: Carefully port instructions, test with real SPEC

2. **ACE integration gap**: Specify may need different ACE handling
   - **Mitigation**: Check if ACE bullets apply to PRD generation

3. **Breaking change**: Users with customized speckit.specify config will lose customizations
   - **Mitigation**: Document in release notes, native behavior should be better

---

## Dependencies

- **SPEC-KIT-902**: Provides the direct execution pattern to follow
- **SPEC-KIT-956**: Blocked until this completes (full config cleanup)

---

## Success Metrics

- Config.toml reduced by ~200 lines (subagent commands section)
- All 7 speckit stage commands use consistent native routing
- Zero config.toml dependencies for speckit.* commands
- No runtime behavior change for users

---

Back to [Key Docs](../KEY_DOCS.md)
