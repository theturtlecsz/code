# SPEC-KIT-931: Architectural Deep Dive - Master Index

**Status**: IN PROGRESS - Systematic Analysis
**Priority**: P0 (Critical - blocks SPEC-930 implementation)
**Created**: 2025-11-12
**Updated**: 2025-11-12 (restructured as master with child specs)
**Parent**: SPEC-KIT-930 (master research spec)
**Type**: Master Coordination Spec (10 focused child specs)
**Effort**: 10-15 hours total (1-2 hour sessions each)

---

## Purpose & Scope

**MASTER INDEX for systematic architectural analysis** - coordinates 10 focused child specs.

**Why Restructure**:
- Original scope: 5 phases × 4-6 hours = 20-30 hours (unmanageable in one spec)
- New approach: 10 child specs × 1-2 hours each = thorough, focused, single-session
- Benefit: Methodical coverage, clear deliverables, manageable scope

**Objectives**:
1. **Complete current system understanding** (Group A-B: 5 child specs)
2. **Validate SPEC-930 patterns** (Group C: 3 child specs)
3. **Make product decisions** (Group D: 2 child specs)
4. **Provide migration roadmap** (synthesized from all children)

**Out of Scope**: Implementation (covered by future task SPECs after decisions made)

---

## Child Spec Structure

### Group A: Current System Understanding (What EXISTS)

**SPEC-931A: Component Architecture** ✅ COMPLETE
- **Scope**: Core components (agent_tool, tmux, consensus_db, quality gates)
- **Delivered**:
  - phase1-inventory.md (component map, 40 questions)
  - phase1-dataflows.md (timing diagrams, failure scenarios)
  - phase1-database.md (schema analysis, bloat discovery)
  - phase1-summary.md (10 findings, 69 total questions, decisions)
- **Key Findings**:
  - No ACID compliance (dual-write problem)
  - Database 99.97% bloat (153MB → 53KB data)
  - 2 dead tables (0 rows, 0 callers)
  - Tmux is 93% of overhead (6.5s of 7s)
  - 4× storage redundancy
- **Status**: ✅ Complete (2025-11-12)
- **Next**: SPEC-931B

---

**SPEC-931B: Configuration & Integration Points** ✅ COMPLETE
- **Scope**: config.toml, prompts.json, environment variables, MCP operations
- **Delivered**:
  - SPEC-931B-config-integration.md (spec document)
  - SPEC-931B-analysis.md (complete analysis, 8 sections, 350+ lines)
- **Key Findings**:
  - Quality gate artifacts stored to MCP (violates SPEC-KIT-072)
  - MCP validation search unnecessary (AGENT_MANAGER has data)
  - Agent naming has 3-4 variants per agent (complexity)
  - No config validation at startup (typos discovered late)
  - Config hot-reload would improve DX
- **Decisions Made**:
  - **D1**: Move MCP artifacts to SQLite (5× faster, aligns architecture)
  - **D2**: Read validation from AGENT_MANAGER (eliminate MCP search)
  - **D3**: Implement hot-reload when idle (safe iteration)
  - **D4**: Add canonical_name field (simplify matching)
- **Status**: ✅ Complete (2025-11-12)
- **Next**: SPEC-931C

---

**SPEC-931C: Error Handling & Recovery** ✅ COMPLETE
- **Scope**: All error paths, SPEC-928 regression prevention, recovery mechanisms
- **Delivered**:
  - SPEC-931C-analysis.md (complete analysis, 10 sections, 350+ lines)
  - Complete error taxonomy (35 distinct error types)
  - Error categorization matrix (60% retryable, 26% permanent, 14% systemic)
  - SPEC-928 regression checklist (10 bugs with test criteria)
  - Recovery assessment (current mechanisms + gaps)
- **Key Findings**:
  - 95 distinct error paths across 4 layers (spawn, execute, validate, store)
  - No crash recovery mechanism (critical gap)
  - No transaction support (dual-write HashMap + SQLite without ACID)
  - Retry logic ad-hoc (only in broker, not orchestrator)
  - 4 silent failures identified (logged but not propagated)
- **Recommendations**:
  - **P0**: Implement crash recovery (4 hours) - data loss on crash
  - **P0**: Add transaction support (8 hours) - data corruption risk
  - **P0**: Regression test suite (3 hours) - prevent SPEC-928 bugs from returning
  - **P1**: Add orchestrator retry logic (2 hours)
- **Status**: ✅ Complete (2025-11-13)
- **Next**: SPEC-931D

---

### Group B: Constraints (What CAN'T Change)

**SPEC-931D: External Contracts** ✅ COMPLETE
- **Scope**: User-facing APIs, system-facing protocols, breaking change impact
- **Delivered**:
  - SPEC-931D-analysis.md (complete analysis, 9 sections, 600+ lines)
  - Contract inventory (72+ API surfaces across 4 categories)
  - Breaking change impact matrix (14 contracts assessed)
  - Consumer analysis (internal + external consumers)
  - 47 open questions accumulated for stakeholder discussion
- **Key Findings**:
  - 23 slash commands (41 total names with 18 legacy aliases)
  - Protocol enums (Op, EventMsg) with 40+ variants each
  - 3 evidence formats (consensus, commands, quality gates)
  - Command telemetry has schemaVersion ✅ (consensus does not ❌)
  - Most contracts lack explicit versioning (high breakage risk)
- **Recommendations**:
  - **P0**: Add schema versioning to all JSON outputs
  - **P0**: Implement database migration framework
  - **P1**: Add protocol version handshake
  - **P1**: Document contract stability guarantees
- **Status**: ✅ Complete (2025-11-13)
- **Next**: SPEC-931E

---

**SPEC-931E: Technical Limits** ✅ COMPLETE
- **Scope**: Hard constraints from libraries, platforms, external services
- **Delivered**:
  - SPEC-931E-analysis.md (comprehensive analysis, 11 sections, 800+ lines)
  - 5 constraint domains quantified (Ratatui, SQLite, Provider APIs, Tokio, Platform)
  - All SPEC-930 patterns validated (6 patterns, compatibility matrix)
  - No hard blockers identified (all constraints solvable with mitigations)
- **Key Findings**:
  - **Ratatui**: Synchronous rendering (33ms/30 FPS), no tokio::select!, async via channels
  - **SQLite**: Single-writer (not blocking), WAL mode recommended, 10K-50K writes/sec
  - **Provider APIs**: OpenAI 90K-10M TPM, Gemini 5-∞ RPM, rate limiter MANDATORY
  - **Tokio**: spawn_blocking ~500 threads, actor model compatible
  - **Platform**: Cross-platform (Linux/macOS/Windows), no OS-specific blockers
- **Recommendations**:
  - **P0**: Implement queue + rate limiter (MANDATORY to avoid 429 errors)
  - **P1**: Adapt TUI for actor events (use channels, current pattern)
  - **P2**: Enable SQLite WAL mode, token estimation heuristic
- **Status**: ✅ Complete (2025-11-13)
- **Next**: SPEC-931F

---

### Group C: Pattern Validation (SPEC-930 Fit)

**SPEC-931F: Event Sourcing Feasibility** 📋 PLANNED
- **Scope**: Event log design, migration path, performance validation
- **Questions**:
  - Can we migrate agent_executions → event_log + projections?
  - What's the replay performance at scale (1,000 events)?
  - How to run old + new systems in parallel?
  - What's the cutover strategy + rollback plan?
- **Deliverables**:
  - Event sourcing schema design
  - Migration step-by-step plan
  - Performance benchmarks (prototype)
  - Decision: GO/NO-GO with timeline
- **Effort**: 2-3 hours (includes prototyping)
- **Status**: Not started

---

**SPEC-931G: Tmux Removal Investigation** 📋 PLANNED
- **Scope**: Provider CLI non-interactive testing, direct API architecture
- **Questions**:
  - Can Claude CLI run without OAuth2 device code UI?
  - How to handle large prompts without wrapper scripts?
  - What observability alternatives exist (no tmux attach)?
  - What's the migration risk (parallel run strategy)?
- **Deliverables**:
  - Provider CLI test results (Claude, Gemini, Code)
  - Direct API call architecture
  - Observability strategy (logs, TUI, metrics)
  - Decision: GO/NO-GO with migration plan
- **Effort**: 2-3 hours (includes testing)
- **Status**: Not started

---

**SPEC-931H: Actor Model Feasibility** ✅ COMPLETE
- **Scope**: Ratatui async integration, supervisor pattern, TUI contract, actor isolation
- **Delivered**:
  - SPEC-931H-actor-model-analysis.md (comprehensive analysis, 400+ lines)
  - Ratatui async compatibility validated (tokio::select! officially supported)
  - Actor system design for quality gates (supervisor pattern, message protocol)
  - Message types specified (SupervisorCommand, AgentMessage, TUIUpdate)
  - TUI integration contract (tokio::select! event loop, channel architecture)
  - Migration complexity quantified (~1,100 LOC, 3-5 days)
  - 15 questions added to MASTER-QUESTIONS.md (Q172-Q186)
  - 4 evidence gaps identified (benchmarks, tests, prototypes)
- **Key Findings**:
  - ✅ Ratatui compatible (NO async blocker)
  - ❌ Doesn't solve dual-write (same 3 storage systems: Supervisor + AGENT_MANAGER + SQLite)
  - ❌ Can't eliminate AGENT_MANAGER (TUI needs sync read cache for 60 FPS)
  - ✅ Better isolation (agents crash independently, supervisor coordinates)
  - ❌ Adds complexity (600 LOC actors vs 300 LOC orchestrator)
  - ⏳ Migration effort significant (3-5 days, medium risk)
- **DECISION**: ❌ **NO-GO for Phase 1** - Defer to Phase 2 as refactoring opportunity
- **Rationale**:
  - Doesn't solve core problems (dual-write, storage complexity)
  - Better alternatives exist (ACID transactions, testing improvements)
  - Actors are code organization pattern, not problem solution
  - Fix problems first (SPEC-931F ACID approach), refactor later
- **Status**: ✅ Complete (2025-11-13)

---

### Group D: Product Decisions (Should It EXIST)

**SPEC-931I: Storage Consolidation** 📋 PLANNED
- **Scope**: Reduce 4 systems → 2, MCP migration, single source of truth
- **Questions**:
  - Which storage systems serve product needs?
  - Should consensus artifacts use SQLite or MCP?
  - Can we eliminate filesystem (result.txt files)?
  - What's the single source of truth architecture?
- **Deliverables**:
  - Storage necessity assessment (keep/remove per system)
  - MCP → SQLite migration plan
  - Single source of truth architecture
  - Decision: Target architecture (2 systems, which ones)
- **Effort**: 1-2 hours
- **Status**: Not started

---

**SPEC-931J: Dead Code Elimination** 📋 PLANNED
- **Scope**: Remove consensus_artifacts, consensus_synthesis, unused methods
- **Questions**:
  - Why do dead tables exist (git history review)?
  - Are they planned features or abandoned code?
  - What's the safe removal process?
  - Any hidden dependencies?
- **Deliverables**:
  - Git history analysis (table creation intent)
  - Dead code inventory (~225 LOC)
  - Removal plan with validation
  - Decision: Remove vs implement proper usage
- **Effort**: 1 hour
- **Status**: Not started

---

## Original Phase Structure (Archived)

### Phase 1: Current System Inventory (What EXISTS) - NOW SPEC-931A

**Goal**: Comprehensive map of current architecture - components, data flows, dependencies, constraints.

#### 1.1 Component Inventory
**Task**: Document every major component involved in agent orchestration

- **Agent Execution**: agent_tool.rs, tmux.rs, execute_agent(), execute_model()
- **State Management**: AGENT_MANAGER (in-memory HashMap), consensus_db.rs (SQLite)
- **Quality Gates**: native_quality_gate_orchestrator.rs, quality_gate_broker.rs
- **Provider Integration**: model_provider_info.rs, OAuth2 flows, API key management
- **Result Processing**: json_extractor.rs, consensus synthesis
- **Telemetry**: Evidence repository, consensus_artifacts table

**Deliverable**: Component diagram with responsibilities, dependencies, LOC

#### 1.2 Data Flow Analysis
**Task**: Trace agent execution end-to-end

- Spawn request → Queue? → Execution → Output capture → Validation → State update → Consensus
- Identify all state mutations, file I/O, database writes, MCP calls
- Map where tmux fits in current flow
- Document current error handling paths

**Deliverable**: Sequence diagram for successful execution + failure scenarios

#### 1.3 Database Schema Review
**Task**: Analyze current database usage

- `~/.code/consensus_artifacts.db` schema
  - agent_executions table
  - consensus_artifacts table
  - consensus_synthesis table
- Usage patterns: When written, when read, who owns lifecycle
- Redundancy: Is data duplicated between in-memory and SQLite?
- **Critical question**: Does this schema serve product needs or is it implementation artifact?

**Deliverable**: Schema diagram + usage analysis + product value assessment

#### 1.4 MCP Integration Points
**Task**: Map all MCP operations related to agent orchestration

- Consensus storage (local-memory MCP)
- Provider model metadata
- Configuration reads
- Evidence writing

**Deliverable**: MCP operation inventory + necessity assessment (keep/remove/redesign)

#### 1.5 Configuration Surface
**Task**: Document all configuration that affects agent orchestration

- config.toml: agent definitions, model configs, orchestrator instructions
- Environment variables: SPEC_KIT_*, timeouts, feature flags
- Runtime state: What can change without restart?

**Deliverable**: Configuration map + flexibility analysis

---

### Phase 2: Constraint Identification (What CAN'T Change)

**Goal**: Identify hard constraints - technical debt, external dependencies, user-facing contracts.

#### 2.1 External Contract Analysis
**Task**: What promises have we made that can't break?

- **User-facing**:
  - /speckit.* commands must work (can internals change?)
  - Consensus artifacts format (consumed by what?)
  - Evidence repository structure (scripts depend on it?)

- **System-facing**:
  - MCP protocol compliance
  - SQLite schema compatibility (migrations?)
  - OAuth2 provider integrations

**Deliverable**: Contract inventory + breaking change impact analysis

#### 2.2 Technical Constraints
**Task**: What are the hard limits?

- **Ratatui TUI**: Sync rendering, can't block on async (current impedance issue)
- **SQLite**: Single-writer limitation
- **Provider APIs**: Rate limits, authentication flows
- **Rust ecosystem**: tokio version, async runtime

**Deliverable**: Constraint list + how SPEC-930 patterns address them

#### 2.3 Current Bugs & Workarounds
**Task**: What bugs exist that new architecture MUST fix?

- 10 bugs fixed in SPEC-928 (must not regress)
- Claude async hang (SPEC-929, should be fixed)
- Known issues in GitHub/docs

**Deliverable**: Bug inventory + how new architecture prevents regression

---

### Phase 3: Pattern Validation (Does SPEC-930 FIT?)

**Goal**: Validate each SPEC-930 pattern against current system constraints.

#### 3.1 Event Sourcing Feasibility
**Pattern**: Immutable event log, state derived from replay, snapshots

**Analysis Questions**:
- Can we migrate current agent_executions to event_log + projections?
- Performance: Replay cost vs current direct state reads?
- Does event sourcing solve any current bugs? (e.g., state corruption on crash)
- What's the migration path? (run both systems in parallel?)
- **Product value**: Does time-travel debugging justify complexity?

**Deliverable**: Go/No-Go decision + migration complexity estimate

#### 3.2 Actor Model Feasibility
**Pattern**: Supervisor + agent actors, message passing via mpsc, isolated state

**Analysis Questions**:
- How do actors integrate with Ratatui TUI? (sync/async bridge still needed?)
- Can current AGENT_MANAGER be replaced with supervisor actor?
- What happens to in-flight agents during crash/restart?
- **Tmux removal**: Can we execute providers directly in actor? (OAuth2 async calls)
- **Product value**: Does actor isolation solve current race conditions?

**Deliverable**: Go/No-Go decision + actor system design for our use case

#### 3.3 Rate Limiting Feasibility
**Pattern**: Multi-provider token buckets, TPM/RPM/QPM tracking

**Analysis Questions**:
- Do we currently track tokens? (or just request count?)
- Can we get token counts before execution? (or only after?)
- Where should rate limiter live? (per-agent, global, per-supervisor?)
- **Product value**: Will we hit limits at medium scale (500 agents/day)?

**Deliverable**: Go/No-Go decision + rate limiter architecture

#### 3.4 Caching-Based Testing Feasibility
**Pattern**: Record/replay API responses for deterministic tests

**Analysis Questions**:
- Can we cache OAuth2-authenticated responses?
- How to invalidate cache on prompt/model/version changes?
- Does this solve current testing pain? (what tests fail now?)
- **Product value**: Worth the complexity vs current mock approach?

**Deliverable**: Go/No-Go decision + test strategy

#### 3.5 TUI Dashboard Feasibility
**Pattern**: Ratatui async widgets with tokio::select! event loop

**Analysis Questions**:
- Can we add dashboard without breaking current TUI?
- Where does it fit? (modal, separate view, inline?)
- Does current ChatWidget architecture support this?
- **Product value**: Does real-time visibility solve current pain? (what visibility gaps exist?)

**Deliverable**: Go/No-Go decision + TUI integration design

---

### Phase 4: Product Design Review (Should It EXIST?)

**Goal**: Question whether current components serve product needs or are just plumbing.

#### 4.1 Consensus DB Necessity
**Current**: SQLite database with agent_executions, consensus_artifacts, consensus_synthesis

**Questions**:
- What product features depend on this data?
- Is SQLite the right storage? (vs event log, vs MCP, vs in-memory only)
- Do we need historical consensus artifacts? (or just final result?)
- Can we simplify schema?

**Analysis**: Inventory all reads/writes, map to product features, identify what's truly needed

**Deliverable**: Keep/Redesign/Remove decision + simplified schema if keeping

#### 4.2 MCP Integration Necessity
**Current**: local-memory MCP for consensus storage, model metadata, config

**Questions**:
- Which MCP operations are essential vs convenience?
- Should consensus live in local-memory or separate DB?
- Is MCP the right abstraction for agent orchestration?

**Analysis**: Map MCP operations to product value, identify over-engineering

**Deliverable**: Keep/Simplify/Remove decision per MCP integration point

#### 4.3 Evidence Repository Necessity
**Current**: File-based evidence under docs/SPEC-OPS-004-*/evidence/

**Questions**:
- What product features consume evidence?
- Is file-based storage right? (vs SQLite, vs logs only)
- Do we need 25MB per SPEC? (what's essential?)

**Analysis**: Usage patterns, retention needs, cleanup automation (already exists via MAINT-4)

**Deliverable**: Keep/Redesign decision + retention policy

#### 4.4 Tmux Removal Impact
**Current**: Tmux panes for large prompts, observable execution, output capture

**Questions**:
- Why was tmux added? (what problem did it solve?)
- Can direct async API calls replace it? (OAuth2 flow, streaming responses)
- What visibility do we lose? (can TUI dashboard replace?)
- **Migration risk**: How to test in parallel?

**Analysis**: Tmux benefits vs costs, direct API call viability, observability alternatives

**Deliverable**: Remove/Keep decision + migration strategy

#### 4.5 Quality Gate Architecture
**Current**: native_quality_gate_orchestrator.rs, multi-agent spawning, consensus resolution

**Questions**:
- Is quality gate architecture right? (or over-engineered?)
- Should it be part of orchestration or separate concern?
- Can supervisor pattern simplify it?

**Analysis**: Current complexity, failure modes, how SPEC-930 patterns would simplify

**Deliverable**: Keep/Redesign decision + architecture recommendation

---

### Phase 5: Critical Path Analysis (Detailed Migration)

**Goal**: Detailed analysis of high-risk, high-impact migration paths.

#### 5.1 State Management Migration
**From**: Dual-write (in-memory HashMap + SQLite agent_executions)
**To**: Event sourcing (event_log + snapshots + projections) OR simplified transactions

**Critical Path**:
1. Schema design (event_log, snapshots, projections)
2. Event types (AgentQueued, AgentStarted, etc.)
3. Replay engine implementation
4. Snapshot frequency/strategy
5. Migration script (existing executions → events)
6. Parallel run validation (old + new systems)
7. Cutover strategy

**Risk Analysis**:
- **High**: Data loss during migration
- **Medium**: Performance regression (replay overhead)
- **Low**: Event schema evolution (add versioning)

**Deliverable**: Step-by-step migration plan + rollback strategy

#### 5.2 Execution Engine Migration
**From**: tmux-based execution (wrapper scripts, pane capture, polling)
**To**: Direct async API calls (OAuth2 or API keys)

**Critical Path**:
1. Provider client implementations (async OpenAI, Anthropic, Google)
2. OAuth2 flow integration (token refresh, device code)
3. Streaming response handling
4. Error mapping (API errors → AgentError)
5. Observability alternative (how to "watch" execution?)
6. Testing strategy (can't use tmux-based tests)

**Risk Analysis**:
- **High**: OAuth2 complexity, token management
- **High**: Loss of observable execution (current tmux attach)
- **Medium**: API client bugs (streaming, parsing)

**Deliverable**: Provider client design + OAuth2 integration plan + observability strategy

#### 5.3 Actor System Integration
**From**: No actors (spawn tokio tasks directly)
**To**: Supervisor + agent actors with message passing

**Critical Path**:
1. Actor message types (SupervisorCommand, AgentMessage)
2. Supervisor actor implementation
3. Agent actor lifecycle
4. Integration with TUI (how to send commands, receive events)
5. Graceful shutdown design
6. Restart policy (when to restart crashed actors)

**Risk Analysis**:
- **High**: TUI integration complexity (sync/async bridge)
- **Medium**: Actor supervision logic
- **Low**: Message passing (tokio mpsc is solid)

**Deliverable**: Actor system design doc + TUI integration contract

#### 5.4 Rate Limiter Integration
**From**: No rate limiting (hit limits, fail, retry)
**To**: Multi-provider token buckets with queueing

**Critical Path**:
1. Token tracking (how to count before execution?)
2. Rate limiter design (global vs per-supervisor)
3. Queue integration (where does queue sit?)
4. Backpressure handling (what to do when full?)
5. Provider fallback logic

**Risk Analysis**:
- **Medium**: Token counting accuracy
- **Medium**: Queue fairness (priority vs FIFO)
- **Low**: Token bucket algorithm (well-known)

**Deliverable**: Rate limiter design + queue integration plan

#### 5.5 Testing Infrastructure Migration
**From**: Manual testing, some mocks, tmux-based E2E
**To**: Caching-based integration, mock actors, fixture management

**Critical Path**:
1. Response cache design (storage, invalidation)
2. Cache key generation (prompt hash? version?)
3. Fixture extraction (from real API responses)
4. Mock actor implementations
5. Test harness setup

**Risk Analysis**:
- **Low**: Well-understood pattern
- **Medium**: Cache invalidation logic

**Deliverable**: Testing strategy + infrastructure plan

---

## Analysis Deliverables

### Strategic Architecture Document
**Content**:
- Component diagram (current + proposed)
- Data flow diagrams (current + proposed)
- Decision matrix (Go/No-Go for each SPEC-930 pattern)
- Product value assessment (keep/remove/redesign per component)
- Migration sequencing (what order to tackle)

### Critical Path Migration Plans
**Content**:
- State management: detailed steps + risks
- Execution engine: provider clients + OAuth2 + observability
- Actor system: supervisor + agents + TUI integration
- Rate limiter: design + queue + backpressure
- Testing: caching + mocks + fixtures

### Risk Assessment
**Content**:
- High-risk paths + mitigation strategies
- Breaking changes + migration strategies
- Performance regression risks
- Data loss scenarios + prevention

### Task Breakdown (Input for Implementation SPECs)
**Content**:
- Phase 1 tasks (foundation)
- Phase 2 tasks (core features)
- Phase 3 tasks (migration)
- Phase 4 tasks (validation)
- Effort estimates per task

---

## Success Criteria

**Analysis Complete When**:
1. ✅ Every SPEC-930 pattern has Go/No-Go decision with clear rationale
2. ✅ Every current component has Keep/Remove/Redesign decision with product value justification
3. ✅ Critical migration paths have detailed step-by-step plans
4. ✅ Risk assessment complete with mitigation strategies
5. ✅ Task breakdown ready for implementation SPEC creation

**Quality Gates**:
- All "CAN'T change" constraints documented
- All "SHOULD change" opportunities identified
- No unanswered questions about feasibility
- Clear sequencing of work (what depends on what)

---

## Analysis Approach

### Session Structure (4-6 hours)

**Hour 1-2: Current System Inventory**
- Component mapping
- Data flow tracing
- Database/MCP/config review

**Hour 2-3: Constraint + Pattern Validation**
- Hard constraints identification
- SPEC-930 pattern feasibility (each pattern)

**Hour 3-4: Product Design Review**
- Question every component
- Keep/remove/redesign decisions

**Hour 4-5: Critical Path Deep Dive**
- State management migration
- Execution engine replacement
- Actor system integration

**Hour 5-6: Documentation + Decisions**
- Write up findings
- Create decision matrix
- Generate task breakdown

### Tools & Methods

**For component analysis**:
- Explore agent (codebase traversal)
- Grep for key patterns
- Read critical files (agent_tool.rs, consensus_db.rs, tmux.rs)

**For data flow**:
- Trace execution paths
- Identify state mutations
- Map database operations

**For decisions**:
- Product value scoring (0-10)
- Complexity scoring (low/medium/high)
- Risk scoring (low/medium/high)
- Decision matrix: Value vs Complexity + Risk

---

## Questions to Answer

### SPEC-930 Pattern Validation
1. **Event Sourcing**: Worth the complexity for our use case?
2. **Actor Model**: Solves our problems or adds overhead?
3. **Rate Limiting**: Needed at our scale? (500 agents/day)
4. **Caching Tests**: Better than current approach?
5. **TUI Dashboard**: Fits in current UI?

### Component Redesign
6. **Consensus DB**: Right schema or over-engineered?
7. **MCP Integration**: Essential or convenience?
8. **Evidence Repo**: Right storage or overkill?
9. **Tmux**: Can we remove it safely?
10. **Quality Gates**: Right architecture?

### Migration Strategy
11. **Sequencing**: What order to implement?
12. **Parallel Run**: Can old + new coexist?
13. **Rollback**: How to undo if fails?
14. **Testing**: How to validate migration?
15. **Timeline**: Realistic estimate?

---

## Next Steps (After Analysis)

1. **Create implementation SPECs** based on task breakdown
2. **Prototype critical paths** (state management, actor system)
3. **Validate assumptions** (performance, complexity)
4. **Begin Phase 1 implementation**

---

## References

**Parent**: SPEC-KIT-930 (master research spec)
**Context**: SPEC-KIT-928 (10 bugs fixed), SPEC-KIT-929 (Claude hang)
**Files**: agent_tool.rs, tmux.rs, consensus_db.rs, native_quality_gate_orchestrator.rs

**Research**: SPEC-930 industry patterns (LangGraph, Temporal, Tokio, rate limiting, testing, Ratatui)
