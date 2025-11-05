# Analysis Prompt for Claude Code (Web)

**Repository**: https://github.com/theturtlecsz/code
**Branch**: debugging-session (or main after merge)
**Analysis Date**: 2025-11-05

---

## Your Task

Perform a comprehensive end-to-end analysis of the **SPEC-KIT-900 multi-agent automation workflow** and verify it aligns with the product vision and intended architecture.

---

## Context

This is a **fork** of https://github.com/just-every/code (itself a community fork of OpenAI Codex). The fork adds:
- **Spec-Kit**: Multi-agent PRD automation framework
- **Native MCP integration**: For consensus synthesis
- **Quality gates**: Multi-tier validation
- **SQLite audit trail**: Complete execution tracking

**CRITICAL**: This is NOT Anthropic's Claude Code product. It's a TUI-based code assistant with multi-agent workflow capabilities.

---

## Analysis Scope

### Primary Objective
**Verify that SPEC-KIT-900 (multi-agent automation) implementation matches the product vision and architectural intent defined in:**
- `product-requirements.md` (product vision)
- `PLANNING.md` (architecture goals)
- `memory/constitution.md` (guardrails)
- `docs/SPEC-KIT-900-generic-smoke/spec.md` (SPEC requirements)
- `docs/SPEC-KIT-900-generic-smoke/PRD.md` (product requirements)

---

## Step-by-Step Analysis

### Phase 1: Understand Product Vision (30 min)

**Read** (in order):
1. `product-requirements.md` - What is the vision?
2. `PLANNING.md` - What are the architectural goals?
3. `memory/constitution.md` - What are the non-negotiables?

**Document**:
- Core vision statement
- Key success criteria
- Non-negotiable requirements

### Phase 2: Understand SPEC-KIT-900 (30 min)

**Read**:
1. `docs/SPEC-KIT-900-generic-smoke/spec.md` - What should it do?
2. `docs/SPEC-KIT-900-generic-smoke/PRD.md` - What are the requirements?
3. `SESSION-3-FINAL-STATUS.md` - What was implemented?

**Document**:
- SPEC-KIT-900 objectives
- Functional requirements (FR1-FR10)
- Acceptance criteria

### Phase 3: Trace Workflow End-to-End (2-3 hours)

**Start**: User runs `/speckit.auto SPEC-KIT-900`

**Trace**:
1. **Command routing** → `codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs`
2. **Pipeline initialization** → `pipeline_coordinator.rs::advance_spec_auto()`
3. **Per-stage flow**:
   - Guardrail validation → `native_guardrail.rs`
   - Quality gates → `native_quality_gate_orchestrator.rs`
   - Agent spawning → `agent_orchestrator.rs::spawn_regular_stage_agents_*`
   - Agent execution → Sequential (Plan/Tasks/Implement) or Parallel (Validate/Audit/Unlock)
   - Result collection → `on_spec_auto_agents_complete_with_results()` (sequential) or `on_spec_auto_agents_complete_with_ids()` (parallel)
   - Consensus synthesis → `pipeline_coordinator.rs::synthesize_from_cached_responses()`
   - Evidence export → `evidence.rs::auto_export_stage_evidence()`
   - Stage advancement → `advance_spec_auto()`
4. **Pipeline completion** → Automatic verification report

**Verify**:
- Does flow match product vision?
- Are all 6 stages executed?
- Is audit trail complete?

### Phase 4: Data Integrity Analysis (1 hour)

**Database**: `~/.code/consensus_artifacts.db`

**Query Examples**:
```sql
-- Check run_id coverage
SELECT COUNT(*) as total,
       SUM(CASE WHEN run_id IS NOT NULL THEN 1 ELSE 0 END) as with_run_id
FROM agent_executions;

-- Verify synthesis records
SELECT stage, artifacts_count, run_id, created_at
FROM consensus_synthesis
ORDER BY created_at DESC LIMIT 10;

-- Check agent completions
SELECT stage, COUNT(*),
       SUM(CASE WHEN completed_at IS NOT NULL THEN 1 ELSE 0 END) as completed
FROM agent_executions
GROUP BY stage;
```

**Verify**:
- 100% run_id coverage?
- All agents have completion timestamps?
- Synthesis records linked to runs?

### Phase 5: Evidence Compliance (1 hour)

**Check**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

**Required Structure** (per PRD):
```
evidence/
├── commands/<SPEC-ID>/     # Guardrail telemetry
├── consensus/<SPEC-ID>/    # Agent consensus (12 files: 6 synthesis + 6 verdict)
└── costs/                  # Cost summaries
```

**Verify**:
- All directories exist?
- Consensus files auto-generated?
- Cost summary matches schema v1?
- Checklist compliance requirements met?

### Phase 6: Bug Fix Verification (1 hour)

**Session 3 Fixed 4 Critical Bugs**:

1. **Synthesis file skip** (commit 2682bfe53)
   - Verify: Files update on every run (no skip logic)

2. **Agent name mismatch** (commit 23726fa69)
   - Verify: Database query for expected names used
   - Check: agent_orchestrator.rs::get_agent_name()

3. **Missing phase transition** (commit bffc93cf6)
   - Verify: state.phase = Guardrail after synthesis
   - Check: pipeline_coordinator.rs:666, 689

4. **Direct results refactor** (commit b64cbeadd)
   - Verify: Sequential uses agent_results from event
   - Check: on_spec_auto_agents_complete_with_results()

**Test**: Can you find the fixes in the code?

### Phase 7: Architecture Assessment (2 hours)

**Evaluate**:
1. **Separation of Concerns**
   - Is agent orchestration cleanly separated from pipeline coordination?
   - Are quality gates independent of regular stages?

2. **Error Handling**
   - Degraded mode support?
   - Non-blocking evidence export?
   - Retry logic?

3. **Scalability**
   - Can it handle 10+ stages?
   - Can it handle 20+ agents?
   - Database query performance?

4. **Maintainability**
   - Code organization clear?
   - Logging comprehensive?
   - Documentation sufficient?

**Compare**: Does architecture match PLANNING.md goals?

---

## Deliverable Template

### Executive Summary (1 page)

**Project**: codex-rs (theturtlecsz/code fork)
**Analysis Date**: [DATE]
**Analyst**: Claude Code (Web)

**Alignment Score**: [0-100%]

**Key Findings**:
1. [Finding 1]
2. [Finding 2]
3. [Finding 3]

**Recommendation**: [Proceed / Revise / Pivot]

---

### Detailed Analysis

#### 1. Product Vision Alignment

**Vision Statement** (from product-requirements.md):
[Quote vision]

**Implementation Assessment**:
- ✅ Aligned: [What matches]
- ⚠️ Partial: [What's close but needs work]
- ❌ Gaps: [What's missing]

**Score**: [0-100%]

#### 2. Workflow Correctness

**Expected Flow** (from SPEC-KIT-900/spec.md):
[Describe expected workflow]

**Actual Implementation**:
- ✅ Correct: [What works]
- ⚠️ Deviations: [What differs]
- ❌ Broken: [What's not working]

**Score**: [0-100%]

#### 3. Data Integrity

**Audit Trail Requirements**:
[From PRD and constitution]

**Implementation Status**:
- run_id coverage: [%]
- Completion tracking: [%]
- Evidence export: [Auto/Manual]
- Synthesis records: [Complete/Incomplete]

**Score**: [0-100%]

#### 4. Quality Gates

**Requirements**:
[From spec.md]

**Implementation**:
- Quality gate execution: [Working/Broken]
- Consensus mechanism: [Implemented/Missing]
- Gate decisions: [Enforced/Advisory]

**Score**: [0-100%]

#### 5. Evidence & Compliance

**Policy Requirements** (evidence-policy.md):
[List requirements]

**Implementation**:
- Structure: [Compliant/Non-compliant]
- Auto-generation: [Yes/No]
- Completeness: [%]

**Score**: [0-100%]

#### 6. Bug Fixes (Session 3)

**Four Critical Bugs Fixed**:
1. Synthesis skip: [Verified/Not Found]
2. Agent mismatch: [Verified/Not Found]
3. Phase transition: [Verified/Not Found]
4. Direct results: [Verified/Not Found]

**Assessment**: [All fixed / Partial / Issues remain]

---

### Gap Analysis

**Functional Gaps**:
1. [Gap 1]: [Description] - Priority: [High/Med/Low]
2. [Gap 2]: [Description] - Priority: [High/Med/Low]

**Architectural Gaps**:
1. [Gap 1]: [Description] - Impact: [High/Med/Low]
2. [Gap 2]: [Description] - Impact: [High/Med/Low]

**Documentation Gaps**:
1. [Gap 1]: [Description]
2. [Gap 2]: [Description]

---

### Recommendations

**Immediate** (must fix before production):
1. [Recommendation 1]
2. [Recommendation 2]

**Short-term** (next sprint):
1. [Recommendation 1]
2. [Recommendation 2]

**Long-term** (roadmap):
1. [Recommendation 1]
2. [Recommendation 2]

---

### Overall Assessment

**Product Vision Alignment**: [Score/100]
**Implementation Quality**: [Score/100]
**Production Readiness**: [Score/100]

**Final Recommendation**: [Detailed recommendation]

---

**Analyst**: Claude Code (Web)
**Date**: [DATE]
**Confidence**: [High/Medium/Low]

---

## Supporting Queries

### Useful SQL Queries for Analysis

```sql
-- Complete run analysis
SELECT
  run_id,
  COUNT(DISTINCT stage) as stages,
  COUNT(*) as total_agents,
  SUM(CASE WHEN completed_at IS NOT NULL THEN 1 ELSE 0 END) as completed,
  MIN(spawned_at) as start,
  MAX(completed_at) as end
FROM agent_executions
GROUP BY run_id
ORDER BY start DESC
LIMIT 5;

-- Artifact coverage by stage
SELECT
  stage,
  COUNT(DISTINCT run_id) as runs,
  COUNT(*) as artifacts,
  AVG(LENGTH(content_json)) as avg_size
FROM consensus_artifacts
GROUP BY stage;

-- Synthesis completeness
SELECT
  stage,
  COUNT(*) as synthesis_count,
  AVG(artifacts_count) as avg_agents,
  AVG(LENGTH(output_markdown)) as avg_output_size
FROM consensus_synthesis
GROUP BY stage;
```

---

## Analysis Checklist

**Before Starting**:
- [ ] Repository cloned
- [ ] Product vision documents read
- [ ] SPEC-KIT-900 context understood
- [ ] Database accessible (~/.code/consensus_artifacts.db)

**During Analysis**:
- [ ] Workflow traced end-to-end
- [ ] Code reviewed in key files
- [ ] Database queries executed
- [ ] Evidence structure verified
- [ ] Bug fixes confirmed

**Deliverable**:
- [ ] Executive summary written
- [ ] Detailed analysis complete
- [ ] Gap analysis provided
- [ ] Recommendations prioritized
- [ ] Overall assessment given

---

**Prepared**: 2025-11-05
**For**: Claude Code (web) comprehensive analysis
**Repository**: https://github.com/theturtlecsz/code
