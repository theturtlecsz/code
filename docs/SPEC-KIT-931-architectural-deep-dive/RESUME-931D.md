# Session Resume: SPEC-931D - External Contracts & API Stability

**Copy-paste this to start next session:**

---

Begin SPEC-931D analysis: External Contracts and API Stability for agent orchestration system.

**Context**:
- SPEC-931A complete (component architecture, 10 findings)
- SPEC-931B complete (config/MCP, 5 findings + 4 decisions)
- SPEC-931C complete (error handling, 35 error types, SPEC-928 checklist)
- This is child spec 4/10 in systematic architectural deep dive
- Group B (Constraints): What CAN'T change

**Scope**: Single-session (1-2 hours), complete contract inventory and breaking change assessment

**Focus Areas**:
1. **User-Facing Contracts**: CLI commands, config format, file outputs, database schema
2. **System-Facing Contracts**: MCP protocol, agent APIs, evidence format, telemetry schema
3. **Breaking Change Impact**: Who/what breaks if we change X?
4. **Versioning Strategy**: How to evolve without breaking existing users/scripts

**Reference Documents**:
- docs/SPEC-KIT-931-architectural-deep-dive/spec.md (master index)
- docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931C-analysis.md (error handling analysis)
- codex-rs/tui/src/chatwidget/spec_kit/ (spec-kit implementation)
- docs/spec-kit/ (user-facing documentation)

**Key Files to Analyze**:
- .claude/commands/ (slash command contracts)
- codex-rs/core/src/protocol.rs (API definitions)
- codex-rs/core/src/mcp_connection_manager.rs (MCP protocol)
- codex-rs/tui/src/chatwidget/spec_kit/evidence.rs (evidence format)
- docs/SPEC-KIT-*/evidence/ (telemetry schema examples)

**Deliverables Expected**:
1. Contract inventory (user-facing vs system-facing)
2. Consumer analysis (who depends on each contract)
3. Breaking change impact matrix
4. Migration compatibility requirements
5. Versioning/deprecation strategy recommendations

**Output Format**:
- SPEC-931D-analysis.md (analysis document)
- Update spec.md with completion status
- Store critical findings to local-memory (importance ≥9)

**Success Criteria**:
- ✅ Every public API catalogued with consumers
- ✅ Breaking change impact quantified (users, scripts, integrations)
- ✅ Stability requirements defined for each contract
- ✅ Migration path documented for necessary breaking changes

**ultrathink** - Question everything, provide detailed analysis for critical contracts.

---

**Start command**: Read this file, then begin systematic contract analysis.
