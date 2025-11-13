# RESUME: PRD Generation for SPEC-936/938/939/940/941

**Session**: SPEC-932 Implementation Planning & Decision Framework
**Date**: 2025-11-13
**Status**: Question review COMPLETE, 2/7 PRDs generated, 5 PRDs pending
**Next Step**: Generate remaining PRD documents using question-based synthesis

---

## Session Accomplishments

### ✅ Question Review Complete
- **Reviewed**: 37/222 questions (CRITICAL + HIGH + MEDIUM priority)
- **Consolidated**: 222 questions → 135 actionable (39% reduction)
- **Method**: Analyzed SPEC-931F/G/H/I/J child spec research for existing answers
- **Output**: `QUESTION-CONSOLIDATION-ANALYSIS.md` (holistic answer synthesis)

### ✅ 7 SPECs Scoped (167-239h total effort)
1. **SPEC-933**: Database Integrity & Hygiene (65-96h) - ✅ PRD COMPLETE
2. **SPEC-934**: Storage Consolidation (10-13h) - ✅ PRD COMPLETE
3. **SPEC-936**: Tmux Elimination (45-65h) - ⏳ SCOPED, PRD PENDING
4. **SPEC-938**: Enhanced Agent Retry (4-6h) - ⏳ SCOPED, PRD PENDING
5. **SPEC-939**: Configuration Management (22-32h) - ⏳ SCOPED, PRD PENDING
6. **SPEC-940**: Performance Instrumentation (12-16h) - ⏳ SCOPED, PRD PENDING
7. **SPEC-941**: Automated Policy Compliance (8-10h) - ⏳ SCOPED, PRD PENDING

### ✅ Knowledge Stored to local-memory
- Question consolidation methodology (memory ID: 7bf543fc)
- 7 SPEC scopes + phased implementation (memory ID: 15de14ca)
- Critical NO-GO decisions (memory ID: 2c50e8a2)
- Storage architecture pattern (memory ID: 8dc5840e)

---

## Context for Next Session

### What We Know

**Session Goal**: SPEC-932 creates implementation backlog from SPEC-931 architectural research (222 questions across 10 child specs A-J).

**Key Discovery**: Most questions already answered in child spec analyses! Consolidation revealed:
- 52 questions answered in 931F/G/H/I/J research
- 47 questions resolved via our SPEC decisions
- 10 questions consolidated into holistic answer blocks
- Result: 222 → 135 questions (only LOW priority contracts/rate limits remain)

**Major Decisions Made**:
- ✅ **GO**: ACID transactions (933), Storage consolidation (934), Tmux elimination (936), Enhanced retry (938), Config management (939)
- ❌ **NO-GO**: Event sourcing (150-180h saved), Actor model (80-120h saved), Schema optimizations (YAGNI)

**PRDs Created** (use as templates):
- `docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md` (6,026 words, comprehensive)
- `docs/SPEC-KIT-934-storage-consolidation/PRD.md` (4,600 words, comprehensive)
- Template reference: `docs/SPEC-KIT-071-memory-system-optimization/PRD.md`

---

## Task: Generate Remaining 5 PRDs

### SPEC-936: Tmux Elimination & Async Orchestration (45-65h)

**Source Questions**: Q3, Q10, Q11, Q25, Q39, Q42, Q71-Q74 (answered in session)

**Scope Summary**:
- Remove tmux pane management (65× speedup target: 6.5s → 0.1s)
- Direct async API calls to provider CLIs (eliminate tmux overhead)
- Filesystem collection cleanup (+5h, remove legacy fallback)
- OAuth2 investigation (device code flows, non-interactive execution)
- Measurement gap noted: Claims are ESTIMATED not MEASURED (proceed anyway)

**Key Sections for PRD**:
1. **Problem Statement**:
   - Issue #1: Tmux overhead (ESTIMATED 93% of 7s orchestration time)
   - Issue #2: Filesystem collection duplication (native vs legacy paths)
   - Issue #3: Observable execution trade-off (lose tmux panes for debugging)

2. **Proposed Solution**:
   - Component 1: Direct async API calls (eliminate tmux session/pane creation)
   - Component 2: Filesystem cleanup (remove legacy collection path)
   - Component 3: OAuth2 flow investigation (device code, non-interactive)
   - Component 4: Fallback strategy (if direct API fails, how to debug?)

3. **Evidence Gaps** (acknowledge but proceed):
   - No Instant::now() timing instrumentation (add in SPEC-940)
   - Claims lack statistical rigor (n≥10 runs, mean±stddev)
   - Even if 50% wrong, still 3-4s savings (acceptable risk)

4. **Dependencies**: None (can proceed before SPEC-933/934)

5. **Risks**:
   - OAuth2 may require interactive prompts (device code)
   - Lose observable execution (tmux attach for debugging)
   - Provider CLI changes break direct invocation

---

### SPEC-938: Enhanced Agent Retry Logic (4-6h)

**Source Questions**: Q43 (answered in session)

**Scope Summary**:
- Retryable error detection (timeout, 429 rate limit, 503 service unavailable)
- Exponential backoff (1s, 2s, 4s, max 3 retries per agent)
- Quality gate integration (retry within gate, not re-spawn gate)
- Beyond AR-2/3/4 implementation (AR tasks added basic retry, enhance it)

**Key Sections for PRD**:
1. **Problem Statement**:
   - Issue #1: Transient failures (timeout, rate limit) treated as permanent
   - Issue #2: AR-2/3/4 retry exists but lacks sophistication (no backoff, limited error detection)
   - Issue #3: Quality gates don't retry, just mark as failed (2/3 consensus still works but suboptimal)

2. **Proposed Solution**:
   - Component 1: Error classification (retryable vs permanent)
   - Component 2: Exponential backoff with jitter (avoid thundering herd)
   - Component 3: Max retry limits (3 attempts, then fail)
   - Component 4: Telemetry (log retry attempts, success rate)

3. **Integration Points**:
   - quality_gate_handler.rs (agent failure detection)
   - agent_orchestrator.rs (retry loop)
   - Error types (classify 4xx/5xx, timeout, network errors)

4. **Testing**: Simulate transient errors (rate limit, timeout), verify retry behavior

---

### SPEC-939: Configuration Management (22-32h)

**Source Questions**: Q7, Q44, Q80-Q81, Q84-Q87 (answered in session)

**Scope Summary**:
- Hot-reload config when idle (3-4h, SPEC-931B D3 approved)
- Canonical name field (2h, SPEC-931B D4 approved)
- Configurable quality gate agents (8-12h, per-checkpoint agent selection)
- Pluggable validation layers (2-3h, per-agent validation depth)
- Startup config validation (3-4h, catch typos early)
- Config error messages (1-2h, user-friendly errors)
- JSON Schema documentation (2-3h, IDE autocomplete)
- API key naming docs (1-2h, GOOGLE_API_KEY vs GEMINI_API_KEY clarity)

**Key Sections for PRD**:
1. **Problem Statement**:
   - Issue #1: Config changes require TUI restart (lose session state)
   - Issue #2: Agent naming confusion (3-4 names per agent: config, command, model, agent_name)
   - Issue #3: Typos discovered late (during execution, not startup)
   - Issue #4: Hardcoded quality gate agents (can't experiment with cost/quality tradeoffs)

2. **Proposed Solution** (8 components, group logically):
   - **Core** (D3+D4): Hot-reload + canonical_name field (5-6h)
   - **Flexibility** (Q7+Q14): Configurable agents + pluggable validation (10-15h)
   - **Quality** (Q85+Q86+Q87): Startup validation + error messages + schema docs (6-9h)
   - **Documentation** (Q84): API key naming guide (1-2h)

3. **Implementation Strategy**: Group by dependency (hot-reload first, enables rapid testing of other features)

---

### SPEC-940: Performance Instrumentation (12-16h)

**Source Questions**: Q72-Q74, Q89, Q91 (measurement gaps identified in session)

**Scope Summary**:
- Add Instant::now() timing measurements throughout orchestration
- Validate performance claims (5× MCP→SQLite, 65× tmux elimination, 3× parallel spawning)
- Statistical rigor (n≥10 runs, report as "X±Yms over n runs")
- Instrumentation locations: spawn, tmux, MCP, SQLite, config parsing, agent execution

**Key Sections for PRD**:
1. **Problem Statement**:
   - Issue #1: Performance claims are ESTIMATED not MEASURED (no Instant::now() in code)
   - Issue #2: No statistical validation (single runs, no variance)
   - Issue #3: Can't identify actual bottlenecks (optimization based on guesses)
   - Issue #4: Post-implementation can't prove gains (SPEC-936 claims 65×, how to verify?)

2. **Proposed Solution**:
   - Component 1: Timing infrastructure (tracing::info! macros with elapsed time)
   - Component 2: Benchmark harness (run operations n≥10 times, collect stats)
   - Component 3: Statistical reporting (mean, stddev, min/max, percentiles)
   - Component 4: Pre/post validation (measure before SPEC-936, after, prove delta)

3. **Instrumentation Points** (prioritized):
   - P0: Tmux operations (session creation, pane creation, stability polling)
   - P0: Agent spawning (total time, breakdown by phase)
   - P1: MCP vs SQLite (consensus storage, retrieval)
   - P1: Config operations (parse, validation, hot-reload)
   - P2: Prompt building, template substitution

4. **Deliverables**: Performance baseline report, post-SPEC-936 validation report

---

### SPEC-941: Automated Policy Compliance (8-10h)

**Source Questions**: Q92 (policy violations identified in session)

**Scope Summary**:
- CI lint rules for SPEC-KIT-072 violations (workflow→SQLite, knowledge→MCP)
- Static analysis for storage separation (grep, AST analysis)
- Automated checks in pre-commit hooks (prevent policy drift)
- Policy compliance dashboard (visual report of violations)

**Key Sections for PRD**:
1. **Problem Statement**:
   - Issue #1: SPEC-KIT-072 policy violated (quality_gate_handler.rs:1775 stores consensus to MCP)
   - Issue #2: No automated enforcement (violations discovered manually during research)
   - Issue #3: Risk of regression (after SPEC-934 fixes violation, could break again)
   - Issue #4: Multiple policy dimensions (storage, memory importance, tag schema)

2. **Proposed Solution**:
   - Component 1: Storage separation validator (bash script: grep for "mcp.*consensus" in spec_kit/)
   - Component 2: CI integration (.github/workflows/ci.yml step)
   - Component 3: Pre-commit hook (optional, faster feedback)
   - Component 4: Policy dashboard (summarize compliance status)

3. **Policy Rules** (expand beyond storage):
   - Rule 1: Consensus artifacts → SQLite only (not MCP)
   - Rule 2: MCP importance threshold (≥8 for storage, prevent bloat)
   - Rule 3: Tag schema compliance (namespaced, no dates, no task IDs)
   - Rule 4: (Future) Memory retention policy

4. **Integration**: Minimal friction (fast checks <5s, clear error messages, auto-fix suggestions)

---

## Methodology: Question-Based PRD Generation

### Template Structure (from SPEC-933/934)

```markdown
# PRD: [Title]

**SPEC-ID**: SPEC-KIT-XXX
**Created**: 2025-11-13
**Status**: Draft - **[PRIORITY] PRIORITY**
**Priority**: **P[0-3]** ([Category])
**Owner**: Code
**Estimated Effort**: [X-Y hours] ([N days/weeks])
**Dependencies**: [List or None]
**Blocks**: [What this enables]

---

## 🔥 Executive Summary

**Current State**: [1-2 sentences describing problem]

**Proposed State**: [1-2 sentences describing solution]

**Impact**: [Bulleted list of key benefits with ✅ checkmarks]

**Source**: [SPEC-931X reference + question numbers]

**Alternative Rejected**: [If applicable, NO-GO decision reference]

---

## 1. Problem Statement

### Issue #1: [Primary Problem] ([CRITICAL/HIGH/MEDIUM])
[2-3 paragraphs with evidence, code examples, impact]

### Issue #2: [Secondary Problem]
[Repeat pattern]

---

## 2. Proposed Solution

### Component 1: [Main Feature] ([CRITICAL/HIGH - Xh])
[Implementation details, code examples, benefits]

### Component 2: [Supporting Feature]
[Repeat pattern]

---

## 3. Acceptance Criteria

### AC1: [Category] ✅
- [ ] Checklist item
- [ ] Checklist item

---

## 4. Technical Implementation

### Phase 1: [First Week] (Xh)
- Files to modify
- New code (~X LOC)
- Testing approach

---

## 5. Success Metrics
[Performance, correctness, operational metrics]

---

## 6. Risk Analysis

### Risk 1: [Description] ([HIGH/MEDIUM/LOW])
**Scenario**: [What could go wrong]
**Mitigation**: [How to prevent/handle]
**Likelihood**: [Low/Medium/High]

---

## 7. Open Questions
[Unresolved decisions, need input]

---

## 8. Implementation Strategy
[Week-by-week breakdown with hours]

---

## 9. Deliverables
[Code, scripts, docs, tests]

---

## 10. Validation Plan
[Test counts, categories]

---

## 11. Conclusion
[Summary, benefits, next steps]
```

### Key Principles

1. **Evidence-Based**: Reference SPEC-931 analyses, question numbers, code line numbers
2. **Comprehensive**: Cover problem, solution, risks, testing, metrics
3. **Actionable**: Clear acceptance criteria, implementation phases, deliverables
4. **Cross-Referenced**: Link to dependencies, blockers, related SPECs

---

## Key Files & References

### Source Documents (Read These First)
- `docs/SPEC-KIT-931-architectural-deep-dive/QUESTION-CONSOLIDATION-ANALYSIS.md` - Holistic answers
- `docs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` - All 222 questions catalogued
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931F-event-sourcing-feasibility.md` - Event sourcing NO-GO
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931H-actor-model-analysis.md` - Actor model NO-GO
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931I-storage-consolidation-analysis.md` - Storage GO decision
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931J-dead-code-elimination-analysis.md` - Dead code GO decision

### Templates (Use These)
- `docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md` - Comprehensive example (6K words)
- `docs/SPEC-KIT-934-storage-consolidation/PRD.md` - Comprehensive example (4.6K words)
- `docs/SPEC-KIT-071-memory-system-optimization/PRD.md` - Original template

### Session Artifacts
- `docs/SPEC-KIT-932-implementation-planning/` - This directory
- Local-memory: 4 memories stored (question consolidation, SPEC scopes, NO-GO decisions, storage architecture)

---

## Ultrathink Prompt for Next Session

```
TASK: Generate PRD documents for SPEC-936, 938, 939, 940, 941

CONTEXT:
You are continuing SPEC-932 implementation planning session. We reviewed 37 questions, analyzed 10 child spec reports (931A-J), consolidated 222 questions to 135 (39% reduction), and scoped 7 SPECs (167-239h total effort). 2 PRDs already complete (933, 934). You need to generate remaining 5 PRDs using question-based synthesis methodology.

INPUT FILES:
1. Read: docs/SPEC-KIT-932-implementation-planning/RESUME-PRD-GENERATION.md (THIS FILE)
2. Read: docs/SPEC-KIT-931-architectural-deep-dive/QUESTION-CONSOLIDATION-ANALYSIS.md (holistic answers)
3. Read: docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md (template reference)
4. Read: docs/SPEC-KIT-934-storage-consolidation/PRD.md (template reference)

SPECS TO GENERATE (in order):
1. SPEC-936: Tmux Elimination & Async Orchestration (45-65h)
   - Questions: Q3, Q10, Q11, Q25, Q39, Q42, Q71-Q74
   - Key: Acknowledge measurement gaps but proceed, 65× speedup target
   - Sections: Remove tmux, filesystem cleanup, OAuth2 investigation, observable execution trade-off

2. SPEC-938: Enhanced Agent Retry Logic (4-6h)
   - Questions: Q43
   - Key: Beyond AR-2/3/4, exponential backoff, error classification
   - Sections: Retryable errors, backoff strategy, quality gate integration

3. SPEC-939: Configuration Management (22-32h)
   - Questions: Q7, Q44, Q80-Q81, Q84-Q87
   - Key: 8 components grouped into Core/Flexibility/Quality/Documentation
   - Sections: Hot-reload, canonical names, configurable agents, validation, schema docs

4. SPEC-940: Performance Instrumentation (12-16h)
   - Questions: Q72-Q74, Q89, Q91
   - Key: Validate SPEC-933/934/936 performance claims, statistical rigor
   - Sections: Timing infrastructure, benchmark harness, pre/post validation

5. SPEC-941: Automated Policy Compliance (8-10h)
   - Questions: Q92
   - Key: CI checks for SPEC-KIT-072 violations, prevent regression
   - Sections: Storage separation validator, CI integration, policy dashboard

METHOD:
1. For each SPEC, extract relevant questions from QUESTION-CONSOLIDATION-ANALYSIS.md
2. Use consolidated answers as "Problem Statement" section evidence
3. Follow SPEC-933/934 template structure (11 sections)
4. Include code examples, file references, LOC estimates
5. Cross-reference dependencies (e.g., SPEC-936 enables filesystem cleanup in SPEC-934)
6. Acknowledge evidence gaps where identified (e.g., SPEC-936 measurement gap)

OUTPUT:
- Create 5 PRD files: docs/SPEC-KIT-{936,938,939,940,941}-[slug]/PRD.md
- Estimated 2-3 hours for all 5 PRDs (streamlined but comprehensive)
- Each PRD: 3,000-4,500 words (comprehensive but not exhaustive)

VALIDATION:
- All 11 sections present (Executive Summary through Conclusion)
- Acceptance Criteria clear and testable
- Dependencies/Blockers documented
- Cross-references to SPEC-931 analyses
- Evidence-based (code line numbers, question references)

After generating all 5 PRDs, update SPEC.md with new SPEC entries (7 total: 933-941).
```

---

## Success Criteria for Next Session

### ✅ Completion Checklist
- [ ] SPEC-936 PRD created (3,000-4,500 words)
- [ ] SPEC-938 PRD created (3,000-4,500 words)
- [ ] SPEC-939 PRD created (3,000-4,500 words)
- [ ] SPEC-940 PRD created (3,000-4,500 words)
- [ ] SPEC-941 PRD created (3,000-4,500 words)
- [ ] SPEC.md updated with 7 new entries (933-941)
- [ ] Local-memory updated with PRD completion milestone

### 📊 Deliverables
- **5 PRD files**: ~15,000-20,000 words total
- **SPEC.md update**: 7 new rows in Active Tasks table
- **Total documentation**: ~30,000 words across both sessions (planning + PRDs)

### 🚀 Ready for Implementation
After next session, you will have:
- ✅ Complete implementation backlog (7 SPECs, 167-239h)
- ✅ Comprehensive PRDs for all SPECs
- ✅ Phased implementation plan (Phase 1-4)
- ✅ Evidence-based decisions documented
- ✅ NO-GO decisions preventing wasted effort

---

## Notes for Continuity

**Time Investment This Session**: ~3.5 hours (question review, consolidation, 2 PRDs, knowledge storage)

**Estimated Next Session**: 2-3 hours (5 PRDs + SPEC.md update)

**Total SPEC-932 Effort**: ~5.5-6.5 hours to produce 167-239h implementation backlog (44× ROI on planning)

**Pattern Established**: Question-based synthesis + child spec research analysis = comprehensive PRDs without redundant research

**Key Insight**: Most questions already answered in prior research! Consolidation methodology enabled 39% reduction and holistic answer blocks. Future SPEC planning should start with "what research already exists?" before generating new questions.
