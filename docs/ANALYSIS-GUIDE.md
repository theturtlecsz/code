# Repository Analysis Guide - codex-rs (theturtlecsz/code fork)

**Purpose**: Guide for comprehensive end-to-end workflow analysis and product vision alignment
**Audience**: Claude Code (web) or external reviewers
**Repository**: https://github.com/theturtlecsz/code

---

## Repository Context

### What This Is

**Repository**: https://github.com/theturtlecsz/code (FORK)
**Upstream**: https://github.com/just-every/code (community fork of OpenAI Codex)
**NOT**: Anthropic's Claude Code (different product)

**Fork-Specific Additions**:
- **Spec-Kit**: Multi-agent PRD automation framework
- **Native MCP integration**: Consensus synthesis (5.3x faster than subprocess)
- **Quality gates**: Multi-tier validation framework
- **Evidence repository**: Telemetry and audit trail
- **SQLite consensus storage**: Replaces memory-based artifact storage

---

## Key Documents (Read First)

### Product Vision & Requirements
1. **product-requirements.md** - Canonical product scope and vision
2. **PLANNING.md** - High-level architecture, goals, constraints
3. **memory/constitution.md** - Non-negotiable project charter and guardrails

### Project Guide
4. **CLAUDE.md** - How Claude Code operates in this repo (operating procedures)
5. **SPEC.md** - Single source of truth for task tracking
6. **README.md** - Project overview and setup

### SPEC-KIT-900 (Multi-Agent Automation)
7. **docs/SPEC-KIT-900-generic-smoke/spec.md** - SPEC-KIT-900 specification
8. **docs/SPEC-KIT-900-generic-smoke/PRD.md** - Product requirements for multi-agent smoke testing
9. **docs/SPEC-KIT-900-generic-smoke/SESSION-3-COMPLETE.md** - Session 3 implementation summary

### Session 3 Implementation
10. **docs/SPEC-KIT-900-generic-smoke/session-3-docs/START-HERE.md** - Session 3 master index
11. **Session 3 root docs**:
    - SESSION-3-FINAL-STATUS.md
    - THREE-CRITICAL-BUGS-FIXED.md
    - AUTOMATIC-EVIDENCE-EXPORT.md
    - REFACTOR-DIRECT-RESULTS.md

---

## Analysis Focus Areas

### 1. Workflow Architecture

**Questions**:
- Does the multi-agent pipeline flow match the intended design?
- Are stages (Plan → Tasks → Implement → Validate → Audit → Unlock) properly sequenced?
- Is the distinction between sequential (Plan/Tasks/Implement) and parallel (Validate/Audit/Unlock) execution correct?

**Key Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
- `codex-rs/tui/src/chatwidget/spec_kit/state.rs`

### 2. Data Flow & Audit Trail

**Questions**:
- Is run_id tracking complete for all agent spawns?
- Are consensus artifacts properly stored to SQLite?
- Does evidence export happen automatically?
- Can we reconstruct complete execution history from database?

**Key Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs`
- `codex-rs/tui/src/chatwidget/spec_kit/evidence.rs`
- Database: `~/.code/consensus_artifacts.db`

### 3. Quality Gates

**Questions**:
- Do quality gates execute at correct checkpoints?
- Is the 3-agent consensus (gemini/claude/gpt) properly implemented?
- Are quality gate results used to gate progression?

**Key Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs`
- `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs`
- `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_broker.rs`

### 4. Evidence & Compliance

**Questions**:
- Does evidence structure match policy requirements?
- Are all required files auto-generated (consensus/, commands/, costs/)?
- Will `/speckit.checklist` pass with current implementation?

**Key Files**:
- `docs/spec-kit/evidence-policy.md`
- Evidence directory: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`
- Export logic: `codex-rs/tui/src/chatwidget/spec_kit/evidence.rs` (auto_export_stage_evidence)

### 5. Cost Optimization

**Questions**:
- Does the tiered model strategy (Tier 0-4) align with cost goals?
- Are native implementations (clarify/analyze/checklist/new) properly utilized?
- Is the cost reduction target ($11 → $2.71 per pipeline) achievable?

**Key Files**:
- `docs/spec-kit/cost-optimization-strategy.md`
- Native commands: `codex-rs/tui/src/chatwidget/spec_kit/{clarify,analyze,checklist,new}_native.rs`

### 6. Session 3 Objectives

**Questions**:
- Was the audit infrastructure properly implemented?
- Are all critical bugs (synthesis skip, agent mismatch, phase transition) fixed?
- Does the refactor (direct results) solve the collection issues permanently?

**Key Documents**:
- SESSION-3-FINAL-STATUS.md
- THREE-CRITICAL-BUGS-FIXED.md
- REFACTOR-DIRECT-RESULTS.md
- AUTOMATIC-EVIDENCE-EXPORT.md

---

## Analysis Methodology

### Step 1: Understand Product Vision
Read: product-requirements.md, PLANNING.md, memory/constitution.md

**Expected Understanding**:
- What is the vision for spec-kit automation?
- What problems does it solve?
- What are the non-negotiable requirements?

### Step 2: Trace Workflow Implementation
Read: SPEC-KIT-900 spec.md, PRD.md

**Expected Understanding**:
- How does `/speckit.auto` work end-to-end?
- What happens at each stage?
- How do agents collaborate?

### Step 3: Verify Architecture Matches Vision
Compare implementation to requirements

**Expected Outputs**:
- Alignment score (0-100%)
- Gaps or deviations
- Architecture strengths
- Improvement opportunities

### Step 4: Assess Data Integrity
Check: SQLite schema, evidence export, audit trail

**Expected Verification**:
- Can we trace every agent execution?
- Is evidence complete for compliance?
- Are costs tracked accurately?

### Step 5: Evaluate Quality & Reliability
Review: Bug fixes, testing, error handling

**Expected Assessment**:
- Are critical bugs resolved?
- Is the system production-ready?
- What are remaining risks?

---

## Database Analysis

### Schema Inspection
```sql
-- Agent executions (complete audit trail)
SELECT * FROM agent_executions WHERE spec_id='SPEC-KIT-900' LIMIT 5;

-- Consensus synthesis (stage outputs)
SELECT * FROM consensus_synthesis WHERE spec_id='SPEC-KIT-900';

-- Consensus artifacts (agent proposals)
SELECT * FROM consensus_artifacts WHERE spec_id='SPEC-KIT-900' AND stage='spec-implement';
```

**Location**: `~/.code/consensus_artifacts.db`

**Tables**:
- `agent_executions`: spawn/completion tracking with run_id
- `consensus_synthesis`: stage outputs with run_id
- `consensus_artifacts`: individual agent proposals with run_id

---

## Evidence Structure

### Required Directories
```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
├── commands/<SPEC-ID>/     # Guardrail telemetry JSON
├── consensus/<SPEC-ID>/    # Multi-agent consensus artifacts (auto-exported)
└── costs/                  # Cost summaries
```

### Auto-Generated Files (Per Stage)
- `<stage>_synthesis.json` - Combined consensus output
- `<stage>_verdict.json` - Individual agent proposals

**Generation**: Automatic after each synthesis (no manual export)

---

## Key Metrics

### Performance
- Pipeline duration: ~30-45 minutes (6 stages)
- Cost: ~$2.71 per full pipeline (vs $11 original)
- Reduction: 75% cost savings

### Reliability
- Agent completion rate: Should be ≥90%
- run_id coverage: 100% (all agents tracked)
- Evidence completeness: 100% (auto-exported)

### Quality
- Code quality: 0 compilation errors
- Test coverage: Integration tests for core flows
- Documentation: Comprehensive (>50 docs)

---

## Expected Findings

### Strengths
- ✅ Complete audit infrastructure (run_id, logs, verification)
- ✅ Automatic evidence export (permanent solution)
- ✅ Direct result passing (no race conditions)
- ✅ Quality gates framework
- ✅ Cost optimization (native implementations)

### Known Issues
- ⚠️ Agent prompt templates (returning placeholders, not content)
- ⚠️ Parallel stage collection (2-3 of 3 agents, active_agents timing)
- ⚠️ Synthesis output size (tiny files despite correct agent count)

### Gaps to Assess
- Integration with upstream (just-every/code)
- Testing coverage (unit vs integration)
- Error recovery and degraded mode
- Documentation completeness
- Production readiness

---

## Deliverables Expected

### Analysis Report Should Include:

1. **Executive Summary**
   - Alignment with product vision (%)
   - Critical findings (strengths/gaps)
   - Recommendation (proceed/revise/pivot)

2. **Workflow Analysis**
   - End-to-end flow diagram
   - Stage-by-stage verification
   - Data flow assessment

3. **Architecture Evaluation**
   - Design patterns assessment
   - Code quality review
   - Performance analysis

4. **Gap Analysis**
   - Missing functionality
   - Deviations from vision
   - Technical debt

5. **Recommendations**
   - Priority fixes
   - Enhancement opportunities
   - Long-term roadmap alignment

---

## How to Use This Guide

### For Claude Code (Web)
1. Clone: `git clone https://github.com/theturtlecsz/code.git`
2. Read: This document first
3. Review: Documents in order listed above
4. Analyze: Code in key files
5. Verify: Against product-requirements.md and PLANNING.md
6. Report: Findings using deliverables template

### For External Reviewers
1. Read: product-requirements.md, PLANNING.md
2. Understand: What spec-kit is supposed to do
3. Review: SPEC-KIT-900 as example implementation
4. Assess: Whether implementation matches vision
5. Recommend: Adjustments or confirm alignment

---

## Quick Start Analysis

**Minimal path** (2-3 hours):
1. Read: product-requirements.md (15 min)
2. Read: SPEC-KIT-900/PRD.md (10 min)
3. Read: SESSION-3-FINAL-STATUS.md (10 min)
4. Review: agent_orchestrator.rs, pipeline_coordinator.rs (60 min)
5. Check: Database schema and evidence structure (30 min)
6. Write: Executive summary (45 min)

**Comprehensive path** (1-2 days):
- All documents
- Full codebase review
- Database analysis
- Evidence verification
- Testing validation
- Detailed recommendations

---

**Prepared**: 2025-11-05
**Status**: Repository ready for analysis
**Commit**: b64cbeadd (Session 3 complete)
