# PRD: SPEC-KIT-908 - Document Stable API Surface

**Priority**: P0 (Critical)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

The spec-kit fork relies on specific upstream APIs but has no formal documentation of these dependencies. This creates risk during upstream sync:

**Critical but Undocumented APIs**:
1. ChatWidget integration points (fields, methods, callbacks)
2. Config struct fields (agents, ace, subagent_commands)
3. SlashCommand enum variants (13 SpecKit* variants)
4. Protocol events (AgentMessage, TaskStarted, etc.)

Without explicit documentation:
- Upstream maintainers don't know which APIs spec-kit depends on
- Fork maintainers don't have clear API contract during sync
- Breaking changes are discovered at runtime, not design time
- No clear communication channel for API evolution

The fork has implicit assumptions about stable APIs but no formal contract defining them.

---

## Goals

### Primary Goal
Create comprehensive documentation (`spec-kit/PUBLIC_API.md`) defining the stable API surface that spec-kit relies on, serving as contract for both upstream and fork maintainers.

### Secondary Goals
- Establish clear communication about API dependencies
- Enable proactive notification of breaking changes
- Provide reference for upstream sync conflict resolution
- Foundation for compile-time API stability tests (SPEC-KIT-905)

---

## Requirements

### Functional Requirements

1. **Documentation File Creation**
   - Create `codex-rs/tui/src/chatwidget/spec_kit/PUBLIC_API.md`
   - Markdown format, GitHub-flavored
   - Versioned (track API contract evolution)

2. **ChatWidget API Documentation**
   - **Required Fields**: Document field name, type, purpose, stability guarantee
   - **Required Methods**: Document signature, behavior contract, stability guarantee
   - **Callbacks**: Document when called, expected behavior, stability guarantee
   - **Example**:
     ```markdown
     ### ChatWidget.spec_auto_state
     - **Type**: `Option<SpecAutoState>`
     - **Purpose**: Pipeline state machine for /speckit.auto
     - **Stability**: STABLE - Do not rename or remove
     - **Used by**: spec_kit::pipeline_coordinator
     ```

3. **Config API Documentation**
   - Document fork-specific config fields
   - Include: field name, type, default value, optional/required, stability
   - Group fork fields vs upstream fields clearly
   - Example:
     ```markdown
     ### Config.agents
     - **Type**: `Vec<AgentConfig>`
     - **Default**: `Vec::new()`
     - **Optional**: Yes (empty vec = no multi-agent)
     - **Stability**: STABLE - Do not rename or remove
     - **Used by**: spec_kit::agent_orchestrator
     ```

4. **SlashCommand API Documentation**
   - List all 13 SpecKit* enum variants
   - Include: variant name, fields, field types, purpose
   - Stability guarantee per variant
   - Example:
     ```markdown
     ### SlashCommand::SpecKitAuto
     - **Fields**: `spec_id: String`, `from_stage: Option<SpecStage>`
     - **Purpose**: Run full 6-stage pipeline
     - **Stability**: STABLE - Do not rename or remove
     - **Used by**: spec_kit::pipeline_coordinator::advance_spec_auto
     ```

5. **Protocol Event API Documentation**
   - Document events spec-kit subscribes to
   - Include: event name, fields used, purpose, stability
   - Example:
     ```markdown
     ### EventMsg::AgentMessage
     - **Fields Used**: `agent_id`, `content`, `turn_id`
     - **Purpose**: Track multi-agent completion for consensus
     - **Stability**: STABLE - Do not change field names/types
     - **Used by**: spec_kit::agent_orchestrator::on_agent_message
     ```

6. **File Location Documentation**
   - Document expected file paths for artifacts
   - Include: evidence directory, SPEC directories, templates
   - Example:
     ```markdown
     ### Evidence Repository
     - **Path**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`
     - **Purpose**: Telemetry and artifact storage
     - **Stability**: STABLE - Do not move without migration
     ```

7. **Stability Guarantees Legend**
   - **STABLE**: Breaking changes require spec-kit refactor coordination
   - **UNSTABLE**: May change, spec-kit adapts as needed
   - **DEPRECATED**: Planned removal, migration path provided

### Non-Functional Requirements

1. **Clarity**
   - Documentation readable by upstream maintainers
   - No spec-kit internals knowledge assumed
   - Clear examples for each API point

2. **Maintainability**
   - Easy to update as APIs evolve
   - Version tracked (API contract v1.0, v1.1, etc.)
   - Linked from main README

3. **Discoverability**
   - Prominent location (`spec_kit/PUBLIC_API.md`)
   - Linked from upstream sync documentation
   - Referenced in PR templates (future)

---

## Technical Approach

### Documentation Structure

**spec_kit/PUBLIC_API.md**:

```markdown
# Spec-Kit Public API Contract

**Version**: 1.0
**Last Updated**: 2025-10-30
**Enforced via**: Compile-time tests (SPEC-KIT-905)

---

## Overview

This document defines the stable API surface that spec-kit (fork-specific automation framework) relies on from upstream codex-rs code. Any breaking changes to these APIs require coordination with fork maintainers.

**Purpose**: Minimize merge conflicts during upstream sync by documenting explicit API dependencies.

**Stability Guarantees**:
- **STABLE**: Breaking changes require spec-kit refactor + coordination
- **UNSTABLE**: May change, spec-kit adapts as needed
- **DEPRECATED**: Planned removal within stated timeline

---

## ChatWidget Integration Points

### Required Fields

#### spec_auto_state
- **Type**: `Option<SpecAutoState>`
- **Purpose**: Pipeline state machine for /speckit.auto
- **Stability**: **STABLE**
- **Used by**: `spec_kit::pipeline_coordinator`
- **Breaking Change Impact**: Pipeline execution fails, no state tracking
- **Introduced**: Fork v0.1.0

#### ace_client
- **Type**: `Option<Arc<AceClient>>`
- **Purpose**: ACE learning system integration
- **Stability**: **STABLE**
- **Used by**: `spec_kit::ace_prompt_injector`
- **Breaking Change Impact**: ACE features disabled
- **Introduced**: SPEC-KIT-066

### Required Methods

#### handle_spec_kit_command
- **Signature**: `fn handle_spec_kit_command(&mut self, cmd: SlashCommand)`
- **Purpose**: Route /speckit.* commands to spec-kit dispatcher
- **Stability**: **STABLE**
- **Used by**: `spec_kit::routing::try_dispatch_spec_kit_command`
- **Breaking Change Impact**: All /speckit.* commands fail
- **Introduced**: Fork v0.1.0

### Callbacks (Must Be Preserved)

#### on_agent_complete
- **Signature**: `fn on_agent_complete(&mut self, agent_id: &str)`
- **Purpose**: Notify spec-kit when multi-agent completes
- **Stability**: **STABLE**
- **Used by**: `spec_kit::agent_orchestrator::check_consensus`
- **Breaking Change Impact**: Consensus never triggered, pipeline hangs
- **Introduced**: SPEC-KIT-045

---

## Config Fields (DO NOT RENAME)

### agents
- **Type**: `Vec<AgentConfig>`
- **Default**: `Vec::new()`
- **Optional**: Yes (empty vec = no multi-agent)
- **Stability**: **STABLE**
- **Used by**: `spec_kit::agent_orchestrator`
- **Breaking Change Impact**: Multi-agent orchestration fails
- **Introduced**: SPEC-KIT-045

### ace
- **Type**: `AceConfig`
- **Default**: `AceConfig::default()`
- **Optional**: No (but can be disabled via ace.enabled=false)
- **Stability**: **STABLE**
- **Used by**: `spec_kit::ace_*` modules
- **Breaking Change Impact**: ACE learning disabled
- **Introduced**: SPEC-KIT-066

### subagent_commands
- **Type**: `Option<HashMap<String, SubagentCommand>>`
- **Default**: `None`
- **Optional**: Yes
- **Stability**: **STABLE**
- **Used by**: `spec_kit::routing`
- **Breaking Change Impact**: Subagent routing fails
- **Introduced**: SPEC-KIT-070

---

## SlashCommand Enum Variants (DO NOT REMOVE)

All SpecKit* variants must be preserved. Breaking changes require coordinated refactor.

### SpecKitNew
- **Fields**: `description: String`
- **Purpose**: Create new SPEC with multi-agent PRD
- **Stability**: **STABLE**

### SpecKitAuto
- **Fields**: `spec_id: String`, `from_stage: Option<SpecStage>`
- **Purpose**: Run full 6-stage pipeline
- **Stability**: **STABLE**

### SpecKitStatus
- **Fields**: `spec_id: Option<String>`
- **Purpose**: Native dashboard for SPEC state
- **Stability**: **STABLE**

... (+ 10 more variants)

---

## Protocol Events Used (DO NOT BREAK)

### AgentMessage
- **Fields Used**: `agent_id: String`, `content: String`, `turn_id: String`
- **Purpose**: Track multi-agent output for consensus
- **Stability**: **STABLE**
- **Used by**: `spec_kit::consensus_coordinator`

### AgentMessageDelta
- **Fields Used**: `agent_id: String`, `delta: String`
- **Purpose**: Stream agent output incrementally
- **Stability**: **STABLE**
- **Used by**: `spec_kit::agent_orchestrator`

### TaskStarted / TaskComplete
- **Fields Used**: `task_id: String`, `status: String`
- **Purpose**: Track pipeline stage lifecycle
- **Stability**: **STABLE**
- **Used by**: `spec_kit::validation_lifecycle`

### McpToolCallBegin / McpToolCallEnd
- **Fields Used**: `tool_name: String`, `server: String`
- **Purpose**: Track MCP tool calls for telemetry
- **Stability**: **STABLE**
- **Used by**: `spec_kit::evidence`

---

## File Locations (DO NOT MOVE)

### Evidence Repository
- **Path**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`
- **Purpose**: Telemetry JSON and artifact storage
- **Stability**: **STABLE** - Requires migration if moved

### SPEC Directories
- **Path**: `docs/SPEC-<ID>-<slug>/`
- **Purpose**: Per-feature documentation and artifacts
- **Stability**: **STABLE** - Pattern enforced by tooling

### Constitution
- **Path**: `memory/constitution.md`
- **Purpose**: Project charter for policy validation
- **Stability**: **STABLE** - Guardrails depend on this path

---

## Deprecation Timeline

No current deprecations. Future API changes will be documented here with:
- **What**: API being deprecated
- **When**: Deprecation date + removal date
- **Why**: Reason for deprecation
- **How**: Migration path for spec-kit

---

## Contact

**Fork Maintainer**: theturtlecsz
**Questions**: Open issue in fork repository
**Breaking Changes**: Coordinate via GitHub discussions before merge

---

**End of API Contract**
```

---

## Acceptance Criteria

- [ ] `spec_kit/PUBLIC_API.md` created with complete API documentation
- [ ] ChatWidget integration points documented (3 fields, 2 methods, 1 callback)
- [ ] Config fields documented (3 fork fields: agents, ace, subagent_commands)
- [ ] SlashCommand variants documented (13 SpecKit* variants)
- [ ] Protocol events documented (6 events: AgentMessage, AgentMessageDelta, TaskStarted, TaskComplete, McpToolCallBegin, McpToolCallEnd)
- [ ] File locations documented (3 paths: evidence, SPEC directories, constitution)
- [ ] Stability guarantees defined (STABLE/UNSTABLE/DEPRECATED legend)
- [ ] Version tracked (API contract v1.0)
- [ ] Linked from `spec_kit/README.md` (if exists) or main README
- [ ] Referenced in upstream sync documentation (`docs/UPSTREAM-SYNC.md`)

---

## Out of Scope

- **Upstream changes**: This SPEC only documents, doesn't modify upstream code
- **Enforcement mechanisms**: Compile-time tests covered in SPEC-KIT-905
- **Internal spec-kit APIs**: Focus is fork→upstream dependencies, not internal modules

---

## Success Metrics

1. **Clarity**: Upstream maintainer can understand spec-kit dependencies without reading code
2. **Completeness**: All breaking-change-prone APIs documented (0 surprises during sync)
3. **Usability**: Sync conflict resolution time reduced by 30% (clear API contract reference)
4. **Maintenance**: API contract updated within 1 week of any dependency changes

---

## Dependencies

### Prerequisites
- Fork isolation audit (SPEC-KIT-907) identifies all touchpoints
- API stability tests (SPEC-KIT-905) validate documentation accuracy

### Downstream Dependencies
- Upstream sync process relies on this documentation
- Future API changes reference this contract for impact analysis

---

## Estimated Effort

**2 hours** (as per architecture review)

**Breakdown**:
- Structure definition: 30 min
- API documentation writing: 1 hour
- Examples and stability guarantees: 20 min
- Linking and integration: 10 min

---

## Priority

**P0 (Critical)** - Must complete before upstream sync. Second step in pre-sync refactor plan (after fork isolation audit). Low effort, high value for sync confidence.

---

## Related Documents

- Architecture Review: Section "Pre-Sync Refactor, Step 2"
- Upstream Sync Readiness: "Conflict Zones" section
- SPEC-KIT-905: API stability tests (enforcement mechanism)
- SPEC-KIT-907: Fork isolation audit (identifies APIs to document)
- Future: `spec_kit/PUBLIC_API.md` (created by this SPEC)
