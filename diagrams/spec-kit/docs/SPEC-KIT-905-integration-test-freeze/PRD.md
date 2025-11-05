# PRD: SPEC-KIT-905 - Add Compile-Time API Stability Tests

**Priority**: P0 (Critical)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

The spec-kit fork relies on specific integration points in upstream code, but there are no compile-time guarantees that these APIs remain stable. This creates significant risk during upstream sync:

**Critical Integration Points**:
1. `ChatWidget.spec_auto_state` field
2. `ChatWidget.handle_spec_kit_command()` method
3. `Config.agents`, `Config.ace`, `Config.subagent_commands` fields
4. `SlashCommand` enum variants (13 SpecKit* variants)
5. Protocol events: `AgentMessage`, `AgentMessageDelta`, `TaskStarted`, `TaskComplete`, `McpToolCallBegin`, `McpToolCallEnd`

**Risk Scenarios**:
- Upstream refactors `ChatWidget` → spec-kit integration breaks silently
- Upstream renames config fields → spec-kit fails at runtime
- Upstream changes protocol events → agent tracking breaks
- No detection until runtime failure or manual testing

Without compile-time checks, upstream merges could break spec-kit integration without immediate detection, leading to hours of debugging during sync.

---

## Goals

### Primary Goal
Add compile-time API stability tests that fail during build if upstream changes break spec-kit integration points, enabling early detection of breaking changes during sync.

### Secondary Goals
- Document stable API surface for fork maintainers
- Provide clear error messages when API changes detected
- Reduce debugging time during upstream sync from hours to minutes
- Prevent runtime failures from undetected API changes

---

## Requirements

### Functional Requirements

1. **API Stability Test Module**
   - Create `spec_kit/mod.rs::api_stability_tests` test module
   - Tests compile but never execute (type-level checks only)
   - One test per integration point
   - Clear error messages when tests fail to compile

2. **ChatWidget Integration Tests**
   - Verify `spec_auto_state: Option<SpecAutoState>` field exists
   - Verify `handle_spec_kit_command(&mut self, SlashCommand)` method exists
   - Verify `on_agent_complete(&mut self, &str)` callback exists
   - Use unimplemented!() functions to check signatures

3. **Config Field Tests**
   - Verify `Config.agents: Vec<AgentConfig>` field exists
   - Verify `Config.ace: AceConfig` field exists
   - Verify `Config.subagent_commands: Option<HashMap<...>>` field exists
   - Check field types match expectations

4. **Protocol Event Tests**
   - Verify `EventMsg::AgentMessage` variant exists
   - Verify `EventMsg::AgentMessageDelta` variant exists
   - Verify `EventMsg::TaskStarted`, `EventMsg::TaskComplete` variants exist
   - Verify `EventMsg::McpToolCallBegin`, `EventMsg::McpToolCallEnd` variants exist

5. **SlashCommand Enum Tests**
   - Verify all 13 `SpecKit*` variants exist
   - Check variant field types (e.g., `SpecKitAuto { spec_id: String, from_stage: Option<SpecStage> }`)

### Non-Functional Requirements

1. **Early Detection**
   - Tests fail during `cargo build` or `cargo test` immediately
   - No need to run full test suite to detect breakage
   - Clear compile errors pointing to missing/changed APIs

2. **Zero Runtime Overhead**
   - Tests never execute (compile-time only)
   - No performance impact on production code

3. **Maintainability**
   - Easy to add new stability checks as integration points evolve
   - Clear documentation linking tests to integration points

---

## Technical Approach

### Test Module Structure

```rust
// spec_kit/mod.rs
#[cfg(test)]
mod api_stability_tests {
    use super::*;
    use crate::chatwidget::ChatWidget;
    use codex_core::config::Config;
    use codex_core::config_types::{AgentConfig, AceConfig};
    use codex_protocol::{EventMsg, Op, SlashCommand, SpecStage};
    use std::collections::HashMap;

    /// Compile-time check: ChatWidget has spec_auto_state field
    #[test]
    fn chatwidget_has_spec_auto_state() {
        fn check_field(_widget: &ChatWidget) -> &Option<SpecAutoState> {
            unimplemented!("Compile-time check only")
        }
    }

    /// Compile-time check: ChatWidget has handle_spec_kit_command method
    #[test]
    fn chatwidget_has_handle_method() {
        fn check_method(_widget: &mut ChatWidget, _cmd: SlashCommand) {
            unimplemented!("Compile-time check only")
        }
    }

    /// Compile-time check: ChatWidget has on_agent_complete callback
    #[test]
    fn chatwidget_has_agent_callback() {
        fn check_callback(_widget: &mut ChatWidget, _agent_id: &str) {
            unimplemented!("Compile-time check only")
        }
    }

    /// Compile-time check: Config has agents field
    #[test]
    fn config_has_agents_field() {
        fn check_field(_config: &Config) -> &Vec<AgentConfig> {
            unimplemented!("Compile-time check only")
        }
    }

    /// Compile-time check: Config has ace field
    #[test]
    fn config_has_ace_field() {
        fn check_field(_config: &Config) -> &AceConfig {
            unimplemented!("Compile-time check only")
        }
    }

    /// Compile-time check: Config has subagent_commands field
    #[test]
    fn config_has_subagent_commands_field() {
        fn check_field(_config: &Config) -> &Option<HashMap<String, SubagentCommand>> {
            unimplemented!("Compile-time check only")
        }
    }

    /// Compile-time check: EventMsg has AgentMessage variant
    #[test]
    fn protocol_has_agent_message_event() {
        fn check_variant(_msg: EventMsg) {
            match _msg {
                EventMsg::AgentMessage { .. } => {},
                _ => unimplemented!("Compile-time check only")
            }
        }
    }

    /// Compile-time check: EventMsg has AgentMessageDelta variant
    #[test]
    fn protocol_has_agent_delta_event() {
        fn check_variant(_msg: EventMsg) {
            match _msg {
                EventMsg::AgentMessageDelta { .. } => {},
                _ => unimplemented!("Compile-time check only")
            }
        }
    }

    /// Compile-time check: EventMsg has TaskStarted variant
    #[test]
    fn protocol_has_task_started_event() {
        fn check_variant(_msg: EventMsg) {
            match _msg {
                EventMsg::TaskStarted { .. } => {},
                _ => unimplemented!("Compile-time check only")
            }
        }
    }

    /// Compile-time check: SlashCommand has SpecKitAuto variant with correct fields
    #[test]
    fn slashcommand_has_speckit_auto_variant() {
        fn check_variant(_cmd: SlashCommand) {
            match _cmd {
                SlashCommand::SpecKitAuto { spec_id, from_stage } => {
                    let _s: String = spec_id;
                    let _opt: Option<SpecStage> = from_stage;
                },
                _ => unimplemented!("Compile-time check only")
            }
        }
    }

    // Repeat for all 13 SpecKit* variants...
}
```

### Documentation

**spec-kit/PUBLIC_API.md** (created in parallel):
```markdown
# Spec-Kit Public API Contract (DO NOT BREAK)

This document defines the stable API surface that spec-kit relies on from upstream code.
Any breaking changes to these APIs require spec-kit updates.

**Enforced via**: `spec_kit/mod.rs::api_stability_tests` (compile-time checks)

## ChatWidget Integration Points

### Required Fields
- `spec_auto_state: Option<SpecAutoState>` - Pipeline state machine

### Required Methods
- `handle_spec_kit_command(&mut self, SlashCommand)` - Command routing
- `on_agent_complete(&mut self, agent_id: &str)` - Agent completion callback

## Config Fields (DO NOT RENAME)
- `agents: Vec<AgentConfig>` - Multi-agent tier definitions
- `ace: AceConfig` - ACE learning system configuration
- `subagent_commands: Option<HashMap<String, SubagentCommand>>` - Subagent routing

## SlashCommand Enum Variants (DO NOT REMOVE)
- SpecKitNew, SpecKitSpecify, SpecKitPlan, SpecKitTasks, SpecKitImplement,
  SpecKitValidate, SpecKitAudit, SpecKitUnlock, SpecKitAuto, SpecKitStatus,
  SpecKitClarify, SpecKitAnalyze, SpecKitChecklist

## Protocol Events Used (DO NOT BREAK)
- `AgentMessage`, `AgentMessageDelta` - Multi-agent output
- `TaskStarted`, `TaskComplete` - Pipeline tracking
- `McpToolCallBegin`, `McpToolCallEnd` - MCP integration tracking
```

### CI Integration

```bash
# .github/workflows/ci.yml (add to existing tests)
- name: Spec-Kit API Stability Checks
  run: |
    cd codex-rs
    cargo test -p codex-tui api_stability_tests --lib
  # Fails fast if upstream broke integration points
```

---

## Acceptance Criteria

- [ ] `api_stability_tests` module created in `spec_kit/mod.rs`
- [ ] ChatWidget integration tests implemented (3 tests: field, method, callback)
- [ ] Config field tests implemented (3 tests: agents, ace, subagent_commands)
- [ ] Protocol event tests implemented (6 tests: AgentMessage, AgentMessageDelta, TaskStarted, TaskComplete, McpToolCallBegin, McpToolCallEnd)
- [ ] SlashCommand variant tests implemented (13 tests, one per SpecKit* variant)
- [ ] All tests compile successfully on current codebase
- [ ] Documentation created: `spec-kit/PUBLIC_API.md`
- [ ] CI integration added (optional but recommended)
- [ ] Tests verified to fail when API changes simulated (negative test)

---

## Out of Scope

- **Runtime API checks**: Focus is compile-time only, no runtime validation
- **Exhaustive coverage**: Only critical integration points, not all spec-kit internals
- **Automated fixes**: Tests detect breakage, don't auto-fix it

---

## Success Metrics

1. **Early Detection**: Upstream API breaks detected within 1 minute of `cargo build`
2. **Clear Errors**: Compile errors point to specific missing/changed API
3. **Sync Confidence**: Upstream sync testing time reduced by 50% (from hours to minutes)
4. **Zero False Positives**: Tests only fail on actual API changes, not spurious failures

---

## Dependencies

### Prerequisites
- None (pure testing/documentation work)

### Downstream Dependencies
- Upstream sync process (SYNC-001) relies on these tests
- Future API changes easier to validate

---

## Estimated Effort

**30 minutes** (as per architecture review)

**Breakdown**:
- Test module creation: 15 min
- Write ~20 compile-time tests: 10 min
- Documentation (PUBLIC_API.md): 5 min

---

## Priority

**P0 (Critical)** - Must complete before upstream sync. Low effort, high value for sync confidence. Should be first task in pre-sync refactor plan.

---

## Related Documents

- Architecture Review: Section "Pre-Sync Refactor, Step 3"
- Upstream Sync Readiness: "Conflict Zones" section
- `spec_kit/mod.rs` - Integration point
- Future: `spec-kit/PUBLIC_API.md` (created by this SPEC)
