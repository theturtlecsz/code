# Session Resume: SPEC-931C - Error Handling & Recovery Analysis

**Copy-paste this to start next session:**

---

Begin SPEC-931C analysis: Error Handling & Recovery for agent orchestration.

**Context**:
- SPEC-931A complete (component architecture, 10 findings)
- SPEC-931B complete (config/MCP, 5 findings)
- This is child spec 3/10 in systematic architectural deep dive

**Scope**: Single-session (1-2 hours), complete error taxonomy and recovery assessment

**Focus Areas**:
1. **Error Taxonomy**: All error types at each orchestration step (spawn, execute, validate, store)
2. **Error Categorization**: Retryable vs permanent, transient vs systemic
3. **Recovery Mechanisms**: Current crash recovery (or lack thereof)
4. **SPEC-928 Regression**: Build checklist of 10 bugs that must not regress

**Reference Documents**:
- docs/SPEC-KIT-931-architectural-deep-dive/spec.md (master index)
- docs/SPEC-KIT-931-architectural-deep-dive/phase1-summary.md (931A findings)
- docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931B-analysis.md (931B findings)
- docs/SPEC-KIT-928-orchestration-chaos/spec.md (10 bugs fixed)

**Key Files to Analyze**:
- codex-rs/core/src/agent_tool.rs (error handling in execute_agent, validation)
- codex-rs/core/src/tmux.rs (timeout, completion detection errors)
- codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs (orchestration errors)
- codex-rs/tui/src/chatwidget/spec_kit/quality_gate_broker.rs (broker retry logic)

**Deliverables Expected**:
1. Error taxonomy (complete enumeration)
2. Error categorization matrix (retry strategy per error type)
3. SPEC-928 regression checklist (10 bugs with test criteria)
4. Recovery assessment (current vs needed)
5. Decision matrix (retry strategy, recovery requirements)

**Output Format**:
- SPEC-931C-analysis.md (analysis document)
- Update spec.md with completion status
- Store critical findings to local-memory (importance ≥9)

**Success Criteria**:
- ✅ Every error path documented with evidence (file:line)
- ✅ Each SPEC-928 bug mapped to current code location
- ✅ Retry strategy decision for each error category
- ✅ Crash recovery gaps identified with solutions

**ultrathink** - Question everything, provide detailed analysis for critical paths.

---

**Start command**: Read this file, then begin systematic error analysis.
