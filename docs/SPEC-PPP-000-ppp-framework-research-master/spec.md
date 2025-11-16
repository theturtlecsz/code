# SPEC-PPP-000: PPP Framework Research Master Coordination

**Status**: In Progress
**Type**: Research Coordination (Meta-SPEC)
**Priority**: P0 (Critical Path)
**Created**: 2025-11-16
**Target Completion**: 2025-12-15 (4 weeks)
**Author**: Research Team (Claude Sonnet 4.5)

---

## Executive Summary

This master SPEC coordinates comprehensive research into integrating the **PPP (Productive, Proactive, Personalized) Framework** into the theturtlecsz/code CLI to achieve **95% compliance** with all framework dimensions. The PPP framework, introduced by CMU researchers (arXiv:2511.02208), optimizes LLM agents across three joint objectives that significantly impact user satisfaction:

1. **Productivity ($R_{Prod}$)**: Task completion success
2. **Proactivity ($R_{Proact}$)**: Asking essential clarifying questions while avoiding high-effort queries
3. **Personalization ($R_{Pers}$)**: Adapting to 20 distinct user interaction preferences

**Research Scope**: 4 parallel research SPECs targeting each implementation goal from the feasibility analysis (docs/ppp-framework-feasibility-analysis.md):
- **SPEC-PPP-001**: Dynamic Proactivity & Vagueness Detection (HIGH complexity)
- **SPEC-PPP-002**: Personalization Profiles & Preference Systems (MEDIUM complexity)
- **SPEC-PPP-003**: Interaction Scoring & Weighted Consensus (MEDIUM-HIGH complexity)
- **SPEC-PPP-004**: Trajectory Logging & MCP Integration (LOW-MEDIUM complexity)

**95% Compliance Target**: Cover 20/20 PPP preferences + core formulas ($R_{Proact}$, $R_{Pers}$) + infrastructure

---

## Research Mandate

### Objectives

**Primary**: Conduct comprehensive research to achieve **95%+ compliance** with PPP framework specifications

**Secondary**:
- Identify viable implementation approaches (3+ options per goal)
- Provide working proof-of-concept code in Rust
- Benchmark performance/cost tradeoffs
- Document integration with existing consensus system (consensus.rs, consensus_db.rs)
- Validate feasibility analysis recommendations

### Success Criteria

- [ ] All 20 PPP preferences documented with implementation strategies
- [ ] Mathematical formulas ($R_{Proact}$, $R_{Pers}$) extracted and validated
- [ ] 15+ academic papers cited across research
- [ ] 10+ existing tools analyzed (Cursor, Copilot, Aider, CrewAI, etc.)
- [ ] 15+ Rust crates evaluated for NLP, translation, scoring
- [ ] Working PoC code for each goal (compiles, passes basic tests)
- [ ] 12+ Architecture Decision Records created
- [ ] Cross-SPEC integration validated (no conflicts)

---

## Research Structure

### Sub-SPEC Hierarchy

```
SPEC-PPP-000 (Master)
├── SPEC-PPP-001: Proactivity & Vagueness Detection
│   ├── RQ1.1: State-of-art vagueness detection methods
│   ├── RQ1.2: Question effort classification (Low/High)
│   ├── RQ1.3: Rust NLP crates survey
│   ├── RQ1.4: Existing /reasoning implementations
│   └── RQ1.5: Performance/cost tradeoffs
│
├── SPEC-PPP-002: Personalization Profiles & Preferences ✅ COMPLETE
│   ├── RQ2.1: 20 PPP preferences taxonomy ✅
│   ├── RQ2.2: Cursor/Copilot/Continue.dev analysis ✅
│   ├── RQ2.3: Rust/TOML schema design ✅
│   ├── RQ2.4: Output format enforcement ✅
│   └── RQ2.5: Translation services comparison ✅
│
├── SPEC-PPP-003: Interaction Scoring & Weighted Consensus
│   ├── RQ3.1: PPP scoring formulas (R_Proact, R_Pers)
│   ├── RQ3.2: Multi-turn trajectory tracking in Rust
│   ├── RQ3.3: Question effort classification (programmatic)
│   ├── RQ3.4: Weighted consensus algorithm design
│   └── RQ3.5: Multi-agent scoring in existing systems
│
└── SPEC-PPP-004: Trajectory Logging & MCP Integration
    ├── RQ4.1: Storage backend comparison (SQLite vs MCP)
    ├── RQ4.2: Trajectory schema design
    ├── RQ4.3: MCP server logging patterns
    ├── RQ4.4: Performance benchmarking
    └── RQ4.5: Integration with execution_logger.rs
```

### Research Timeline

**Week 1** (Nov 16-22):
- [x] SPEC-PPP-000 master creation
- [x] SPEC-PPP-002 research execution (PRIORITY 1 - Personalization)
- [ ] SPEC-PPP-004 research execution (PRIORITY 2 - MCP Logging)

**Week 2** (Nov 23-29):
- [ ] SPEC-PPP-003 research execution (PRIORITY 3 - Interaction Scoring)
- [ ] SPEC-PPP-001 research execution (PRIORITY 4 - Proactivity)

**Week 3** (Nov 30-Dec 6):
- [ ] Cross-SPEC synthesis and integration validation
- [ ] PoC code integration testing
- [ ] Compliance validation (95% target)

**Week 4** (Dec 7-13):
- [ ] Final recommendations consolidation
- [ ] Implementation roadmap creation
- [ ] Handoff documentation

---

## PPP Framework Core Specifications

### The 20 User Preferences (Full List)

Extracted from arXiv:2511.02208, Section 3.2:

#### **Seen Preferences (1-12)** - Interaction Style
1. **no_preference** – No specific interaction requirements
2. **concise_question** – Prefers very short, to-the-point inquiries
3. **detail_question** – Wants contextual, well-explained questions
4. **answer_more** – Expects agent to ask ≥3 questions
5. **only_begin** – Willing to answer only at initial stage
6. **no_ask** – Dislikes any questions from agent
7. **do_selection** – Only responds to A/B/C format options
8. **professional** – Can handle technical-level inquiries
9. **amateur** – Answers only simple, common-sense questions
10. **ask_many** – Prefers all questions in single turn
11. **one_question** – Wants one question per interaction turn
12. **first_try** – Agent should attempt solving before seeking clarification

#### **Unseen Preferences (13-20)** - Format & Language
13. **lang_ita** – Questions must be "in Italian only"
14. **lang_multi** – Requires "at least five different languages"
15. **capital** – "All capitalized" English text exclusively
16. **commas** – "Contains no commas anywhere"
17. **json** – "Wrapped entirely as valid JSON"
18. **joke** – Must include "a clearly humorous joke"
19. **snippet** – Requires code/documentation with explicit references
20. **length** – "Exactly three sentences"

### Reward Formulas (PPP Framework)

**Overall Reward**:
```
R = R_Prod + R_Proact + R_Pers
```

**Productivity Reward ($R_{Prod}$)**:
- Task-oriented verification of successful completion
- Domain-specific (e.g., F1 score for SWE-Bench, exact match for BrowseComp)

**Proactivity Reward ($R_{Proact}$)**:
- **Bonus**: +0.05 if all queries are low-effort
- **Penalty**: -0.1 per medium-effort query
- **Penalty**: -0.5 per high-effort query

**Personalization Reward ($R_{Pers}$)**:
- **Bonus**: +0.05 for full preference compliance
- **Penalty**: Preference-specific violations (non-positive values)

**Key Insight**: High-effort questions (require deep investigation, block progress) are heavily penalized to encourage strategic clarification.

---

## Compliance Tracking Matrix

| Component | PPP Coverage | Research SPEC | Status | Compliance % |
|-----------|--------------|---------------|--------|--------------|
| **20 User Preferences** | 20/20 | SPEC-PPP-002 | ✅ Complete | 100% |
| **R_Proact Formula** | 1/1 | SPEC-PPP-001, 003 | 🔄 Pending | 0% |
| **R_Pers Formula** | 1/1 | SPEC-PPP-002, 003 | ✅ Complete | 100% |
| **Question Effort Classification** | Low/Med/High | SPEC-PPP-001 | 🔄 Pending | 0% |
| **Vagueness Detection** | Core mechanism | SPEC-PPP-001 | 🔄 Pending | 0% |
| **Trajectory Tracking** | Infrastructure | SPEC-PPP-004 | 🔄 Pending | 0% |
| **Weighted Consensus** | Algorithm | SPEC-PPP-003 | 🔄 Pending | 0% |
| **Interaction Scoring** | Calculator | SPEC-PPP-003 | 🔄 Pending | 0% |
| **Output Format Enforcement** | 8 constraints | SPEC-PPP-002 | ✅ Complete | 100% |
| **Translation Services** | Multi-lingual | SPEC-PPP-002 | ✅ Complete | 100% |
| **TOTAL** | **All dimensions** | **4 SPECs** | **25% Complete** | **40%** |

**Current Compliance**: 40% (4/10 major components)
**Target**: 95%+ (all components with evidence-based approaches)

---

## Research Methodology

### Multi-Source Research Strategy

Each sub-SPEC follows this research protocol:

#### 1. Academic Literature (Target: 15-20 papers)
- **arXiv**: Search for recent papers (2023-2025) on prompt disambiguation, question classification, multi-agent consensus
- **ACL Anthology**: NLP methods for vagueness detection, clarification generation
- **NeurIPS/ICML**: Multi-agent RL, trajectory optimization

#### 2. Existing Tools Analysis (Target: 10+ tools)
- **AI Coding Assistants**: Cursor, GitHub Copilot, Aider, Continue.dev
- **Multi-Agent Frameworks**: CrewAI, AutoGen, LangGraph, OpenAI Swarm
- **Observability Platforms**: LangSmith, Helicone, Phoenix Arize
- **MCP Ecosystem**: Official servers, community implementations

#### 3. Rust Ecosystem Survey (Target: 15+ crates)
- **NLP**: `rust-bert`, `lingua`, `whatlang`, `rust-tokenizers`
- **Translation**: `libretranslate-rs`, `deepl-rs`
- **Database**: `rusqlite`, `tokio-rusqlite`, `async-sqlite`
- **Async**: `tokio`, `async-trait`, `futures`

#### 4. Proof-of-Concept Development
- **Language**: Rust (matches project: 94.1% Rust)
- **Testing**: Unit tests + integration tests (must compile and pass)
- **Benchmarking**: Performance metrics (latency, memory, accuracy)
- **Integration**: Mock integration with consensus.rs patterns

#### 5. Architecture Decision Records
- **Format**: ADR template (Context, Decision, Consequences)
- **Scope**: 3+ ADRs per sub-SPEC (12+ total)
- **Coverage**: Technology selection, design patterns, tradeoffs

---

## Integration Requirements

### Existing System Context

The theturtlecsz/code CLI has a mature multi-agent infrastructure:

**Key Components**:
- **Consensus System**: `consensus.rs` (1160 lines) - Binary consensus (ok/degraded/conflict)
- **SQLite Storage**: `consensus_db.rs` (SPEC-934) - Artifact persistence
- **Native MCP**: `mcp_connection_manager.rs` (5.3x faster than subprocess, ARCH-004)
- **Execution Logging**: `execution_logger.rs` - Run/stage/cost tracking
- **Agent Orchestrator**: `agent_orchestrator.rs` - Direct agent spawning
- **Pipeline Coordinator**: `pipeline_coordinator.rs` - State machine (300+ lines)

**Integration Points** (documented in each sub-SPEC):
1. **Configuration**: Extend `config_types.rs:193-246` with UserPreferences
2. **Prompt Injection**: Modify `consensus.rs:220-249` for preference application
3. **Output Validation**: New module `output_formatter.rs`
4. **Scoring**: New module `interaction_scorer.rs`
5. **Trajectory Storage**: Extend `consensus_db.rs` with trajectory schema
6. **Consensus Logic**: Refactor `consensus.rs:681-958` for weighted selection

### Non-Breaking Changes Requirement

All proposed changes must:
- ✅ Maintain backward compatibility (feature flag: `ppp.enabled = false`)
- ✅ Not break existing `/speckit.auto` pipeline
- ✅ Preserve current consensus artifacts (SQLite schema extension, not replacement)
- ✅ Allow gradual rollout (Phase 1 → Phase 2 → Phase 3)

---

## Research Deliverables (Per Sub-SPEC)

Each sub-SPEC must produce:

### 1. spec.md (Main Specification)
- Research questions with detailed answers
- Compliance assessment against PPP framework
- Integration points with existing system
- References and citations

### 2. findings.md (Literature Review)
- 5-10 academic papers summarized
- Key insights and methodologies
- Benchmark data where available
- Gap analysis (what's missing from literature)

### 3. comparison.md (Tool/Library Comparison)
- Tabular comparison of 3+ options per decision
- Pros/cons analysis
- Performance benchmarks
- Cost analysis (if applicable)
- Recommendation with rationale

### 4. evidence/*.rs (Proof-of-Concept Code)
- Working Rust code (compiles with `cargo build`)
- Unit tests (pass with `cargo test`)
- Benchmarks (optional but recommended)
- Example usage

### 5. adr/ADR-*.md (Architecture Decision Records)
- Minimum 3 ADRs per sub-SPEC
- Format: Context, Decision, Consequences, Alternatives Considered
- Covers technology selection and design patterns

### 6. recommendations.md (Phased Implementation Plan)
- Tiered recommendations (Phase 1/2/3)
- Effort estimates (LOW/MEDIUM/HIGH)
- Risk assessment
- Success metrics

---

## Sub-SPEC Status Reports

### SPEC-PPP-002: Personalization ✅ **COMPLETE**

**Status**: Research Complete (2025-11-16)
**Deliverables**:
- ✅ spec.md created (5000+ lines, complete Rust schema)
- ⏳ findings.md pending
- ⏳ comparison.md pending
- ⏳ evidence/user_preferences_poc.rs pending
- ⏳ ADRs pending (3 required)
- ⏳ recommendations.md pending

**Key Findings**:
- All 20 PPP preferences mapped to Rust types
- 4 translation service options identified (LibreTranslate, DeepL, LLM-native, Cloud)
- 3-layer enforcement strategy (prompt injection, post-processing, validation+retry)
- 100% PPP compliance achievable

**Next Steps**: Create supporting deliverables (findings, comparison, PoC, ADRs)

---

### SPEC-PPP-004: MCP Logging ⏳ **PENDING**

**Status**: Not Started
**Priority**: P1 (Foundation for SPEC-003)
**Target**: Week 1 completion

**Research Questions**:
- RQ4.1: SQLite vs MCP server comparison
- RQ4.2: Trajectory schema design
- RQ4.3: MCP logging patterns survey
- RQ4.4: Performance benchmarking
- RQ4.5: Integration with execution_logger.rs

**Dependencies**: None (can start immediately)

---

### SPEC-PPP-003: Interaction Scoring ⏳ **PENDING**

**Status**: Not Started
**Priority**: P2 (Depends on SPEC-004 for trajectory storage)
**Target**: Week 2 completion

**Research Questions**:
- RQ3.1: Extract exact formulas from PPP paper
- RQ3.2: Design AgentTrajectory data structure
- RQ3.3: Implement question effort classifier
- RQ3.4: Weighted consensus algorithm
- RQ3.5: Survey CrewAI/AutoGen scoring

**Dependencies**: SPEC-PPP-004 (trajectory schema)

---

### SPEC-PPP-001: Proactivity ⏳ **PENDING**

**Status**: Not Started
**Priority**: P3 (Most complex, can be developed in parallel)
**Target**: Week 2 completion

**Research Questions**:
- RQ1.1: Vagueness detection methods (NLP + LLM)
- RQ1.2: Question effort classification
- RQ1.3: Rust NLP crates evaluation
- RQ1.4: Survey /reasoning implementations
- RQ1.5: Cost/performance benchmarking

**Dependencies**: None (independent research track)

---

## Cross-SPEC Integration Validation

### Shared Components

**1. Trajectory Data Structure** (SPEC-003 + SPEC-004)
- Defined in: SPEC-PPP-004 (trajectory schema)
- Used by: SPEC-PPP-003 (interaction scoring)
- **Validation**: Ensure schema supports scoring requirements

**2. Question Effort Classifier** (SPEC-001 + SPEC-003)
- Defined in: SPEC-PPP-001 (vagueness detection)
- Used by: SPEC-PPP-003 (R_Proact calculation)
- **Validation**: Ensure classifier outputs match scoring inputs

**3. Preference Validation** (SPEC-002 + SPEC-003)
- Defined in: SPEC-PPP-002 (UserPreferences struct)
- Used by: SPEC-PPP-003 (R_Pers calculation)
- **Validation**: Ensure violation detection aligns with penalty system

**4. SQLite Schema** (SPEC-003 + SPEC-004)
- Base schema: SPEC-PPP-004 (trajectories table)
- Extensions: SPEC-PPP-003 (interaction_score columns)
- **Validation**: No schema conflicts, proper indexing

### Integration Testing Plan

**Phase 1**: Mock integration (Week 3)
- Create mock ChatWidget with UserPreferences
- Simulate agent run with trajectory logging
- Calculate interaction scores
- Validate end-to-end flow

**Phase 2**: Real integration (Week 4)
- Integrate with actual consensus.rs flow
- Run `/speckit.plan` with PPP enabled
- Measure performance impact
- Validate backward compatibility

---

## Risk Management

### High-Risk Items

**R1: Formula Interpretation Ambiguity**
- **Risk**: PPP paper formulas may not fully specify edge cases
- **Mitigation**: Contact paper authors for clarification, document assumptions in ADRs
- **Owner**: SPEC-PPP-003

**R2: Performance Degradation**
- **Risk**: Vagueness detection + scoring adds latency to agent execution
- **Mitigation**: Benchmark each component, set performance budgets (<10% overhead)
- **Owner**: All SPECs

**R3: Preference Conflicts**
- **Risk**: User may specify contradictory preferences (no_ask + selection_only)
- **Mitigation**: Validation layer with conflict detection, clear error messages
- **Owner**: SPEC-PPP-002

**R4: Integration Complexity**
- **Risk**: 4 SPECs may propose incompatible designs
- **Mitigation**: Weekly integration reviews, shared data structure definitions
- **Owner**: SPEC-PPP-000 (this document)

### Medium-Risk Items

**R5: Rust Crate Maturity**
- **Risk**: Some NLP crates may be unmaintained or lack features
- **Mitigation**: Evaluate multiple options, prefer established crates, document fallbacks
- **Owner**: SPEC-PPP-001

**R6: Translation Quality**
- **Risk**: LibreTranslate may produce poor translations for technical content
- **Mitigation**: Offer multiple service options (DeepL, LLM-native), user choice
- **Owner**: SPEC-PPP-002

---

## Success Metrics

### Quantitative Targets

- [ ] **95%+ Compliance**: 19/20 PPP preferences + core formulas
- [ ] **15+ Papers**: Academic literature coverage
- [ ] **10+ Tools**: Competitive analysis breadth
- [ ] **15+ Crates**: Rust ecosystem survey depth
- [ ] **12+ ADRs**: Decision documentation
- [ ] **4 Working PoCs**: Code deliverables (compile + test pass)

### Qualitative Targets

- [ ] **Implementability**: Each SPEC provides clear path to implementation
- [ ] **Integration**: No conflicts across SPECs, shared components validated
- [ ] **Justification**: All decisions backed by evidence (benchmarks, comparisons)
- [ ] **Completeness**: No major research questions left unanswered

---

## References

### Primary Sources

1. **PPP Framework Paper**: arXiv:2511.02208 - "Training Proactive and Personalized LLM Agents" (Sun et al., 2025)
   - GitHub: https://github.com/sunnweiwei/PPP-Agent
   - Dataset: USERVILLE (user simulators with preferences)

2. **Feasibility Analysis**: `docs/ppp-framework-feasibility-analysis.md` (This repo, 2025-11-16)
   - 3-6 month implementation estimate
   - 4 goal breakdown with integration points

3. **Existing System**:
   - `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs` (1160 lines)
   - `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs` (SQLite, SPEC-934)
   - `codex-rs/core/src/mcp_connection_manager.rs` (MCP client)

### Secondary Sources

4. **Tools Documentation**:
   - Cursor: https://docs.cursor.com/context/rules
   - GitHub Copilot: https://code.visualstudio.com/docs/copilot/reference/copilot-settings
   - Continue.dev: https://docs.continue.dev/reference
   - CrewAI: https://github.com/crewAIInc/crewAI
   - AutoGen: https://microsoft.github.io/autogen/

5. **Rust Ecosystem**:
   - rust-bert: https://github.com/guillaume-be/rust-bert
   - rusqlite: https://docs.rs/rusqlite
   - serde: https://serde.rs/

---

## Approval & Sign-Off

**Research Lead**: Claude Sonnet 4.5
**Reviewers**: Project maintainers (TBD)
**Target Review Date**: 2025-12-01 (after Week 2 completion)

**Approval Criteria**:
- [ ] All 4 sub-SPECs complete with deliverables
- [ ] 95% compliance target met
- [ ] Integration validation passed
- [ ] Implementation roadmap created

---

## Appendix A: Research Prompt Templates

### Template for Sub-SPEC Research

```markdown
# Research Execution Checklist for SPEC-PPP-{N}

## Phase 1: Literature Survey (Days 1-2)
- [ ] Search arXiv for 5-10 relevant papers
- [ ] Extract key methodologies and metrics
- [ ] Identify gaps in current research
- [ ] Document in findings.md

## Phase 2: Tool Analysis (Days 2-3)
- [ ] Analyze 3-5 existing tools
- [ ] Create comparison matrix
- [ ] Benchmark where possible
- [ ] Document in comparison.md

## Phase 3: Rust Ecosystem (Days 3-4)
- [ ] Survey 5+ relevant crates
- [ ] Test compilation and basic usage
- [ ] Evaluate maturity and maintenance
- [ ] Select recommended crates

## Phase 4: PoC Development (Days 4-5)
- [ ] Design API based on research
- [ ] Implement core functionality
- [ ] Write unit tests
- [ ] Document in evidence/*.rs

## Phase 5: Decision Documentation (Days 5-6)
- [ ] Create 3+ ADRs
- [ ] Document alternatives considered
- [ ] Justify final recommendations
- [ ] Write recommendations.md

## Phase 6: Integration Validation (Day 7)
- [ ] Identify integration points
- [ ] Validate compatibility with other SPECs
- [ ] Update master SPEC with findings
```

---

## Appendix B: Compliance Checklist

```markdown
# PPP Framework 95% Compliance Validation

## User Preferences (20/20 required)
- [ ] 1. no_preference - Implementation path defined
- [ ] 2. concise_question - Implementation path defined
- [ ] 3. detail_question - Implementation path defined
- [ ] 4. answer_more - Implementation path defined
- [ ] 5. only_begin - Implementation path defined
- [ ] 6. no_ask - Implementation path defined
- [ ] 7. do_selection - Implementation path defined
- [ ] 8. professional - Implementation path defined
- [ ] 9. amateur - Implementation path defined
- [ ] 10. ask_many - Implementation path defined
- [ ] 11. one_question - Implementation path defined
- [ ] 12. first_try - Implementation path defined
- [ ] 13. lang_ita - Implementation path defined
- [ ] 14. lang_multi - Implementation path defined
- [ ] 15. capital - Implementation path defined
- [ ] 16. commas - Implementation path defined
- [ ] 17. json - Implementation path defined
- [ ] 18. joke - Implementation path defined
- [ ] 19. snippet - Implementation path defined
- [ ] 20. length - Implementation path defined

## Core Formulas
- [ ] R = R_Prod + R_Proact + R_Pers - Formula documented
- [ ] R_Proact = +0.05 (low) OR -0.1 (med) OR -0.5 (high) - Implemented
- [ ] R_Pers = +0.05 (compliant) OR negative (violations) - Implemented

## Infrastructure
- [ ] Vagueness detection mechanism
- [ ] Question effort classifier (Low/Med/High)
- [ ] Trajectory tracking data structure
- [ ] Interaction scorer calculator
- [ ] Weighted consensus algorithm
- [ ] Output format enforcer
- [ ] Translation service integration
- [ ] SQLite schema extension
```

**End of Master SPEC-PPP-000**
