# Next Session: Implementation Research & Detailed PRDs

**Session**: SPEC-932 Implementation Phase
**Date**: 2025-11-13 (Session 2 complete)
**Status**: Ready for Session 3 - Implementation Research
**Purpose**: Transform 7 architectural PRDs into detailed implementation specs with online research

---

## Session 2 Accomplishments

### ✅ Complete
- Generated 5 PRDs (936/938/939/940/941) - ~22,000 words
- Validated 11 open questions systematically
- Updated SPEC.md with 167-239h implementation backlog
- Total: 7 comprehensive PRDs (933-941) ready for implementation

---

## Ultrathink Prompt for Next Session

```markdown
TASK: Create SPEC-KIT-945 "Implementation Research & Detailed Specs" - Research best practices online and generate detailed implementation PRDs for SPEC-933 through SPEC-941.

CONTEXT:
You are beginning the implementation phase of SPEC-932 planning work. We have 7 comprehensive architectural PRDs (933-941) totaling 167-239h effort addressing critical findings from SPEC-931 deep dive. These PRDs define WHAT to build and WHY. Now we need detailed implementation specs defining HOW to build with modern best practices, battle-tested patterns, and production-proven approaches.

BACKGROUND - SPEC-932 PRDs Summary:
1. SPEC-933: Database Integrity & Hygiene (65-96h, P0-CRITICAL)
   - ACID transactions (dual-write elimination)
   - Incremental auto-vacuum (153MB→<5MB)
   - Parallel agent spawning (3× speedup)
   - Daily cleanup automation

2. SPEC-934: Storage Consolidation (10-13h, P1-HIGH)
   - Migrate consensus MCP→SQLite (5× faster)
   - Eliminate MCP from orchestration
   - Restore SPEC-KIT-072 compliance
   - Reduce 4 storage systems→2

3. SPEC-936: Tmux Elimination & Async Orchestration (45-65h, P1-HIGH)
   - Direct async API calls (65× speedup target)
   - Remove filesystem collection
   - OAuth2 device code flows
   - Alternative diagnostics (logs, TUI viewer)

4. SPEC-938: Enhanced Agent Retry Logic (4-6h, P2-MEDIUM)
   - Error classification (retryable vs permanent)
   - Exponential backoff with jitter
   - Quality gate integration (3/3 consensus)
   - Comprehensive telemetry

5. SPEC-939: Configuration Management (22-32h, P2-MEDIUM)
   - Hot-reload config when idle
   - Canonical name field
   - Configurable quality gate agents
   - JSON Schema documentation

6. SPEC-940: Performance Instrumentation (12-16h, P2-MEDIUM)
   - Timing infrastructure (measure_time! macro)
   - Benchmark harness (n≥10, statistics)
   - Pre/post validation (p<0.05)
   - Statistical reporting

7. SPEC-941: Automated Policy Compliance (8-10h, P2-MEDIUM)
   - CI checks (SPEC-KIT-072 violations)
   - Static analysis (storage separation)
   - Mandatory pre-commit hooks
   - Policy compliance dashboard

RESEARCH OBJECTIVES:
For each PRD, research online to find:
1. **Best Practices**: Industry-proven patterns for the technology (Rust async, SQLite, tokio, benchmarking, etc.)
2. **Battle-Tested Libraries**: Production-ready crates/tools (rusqlite, tokio, notify, proptest, etc.)
3. **Performance Patterns**: Optimization techniques, benchmarking methodologies, profiling approaches
4. **Error Handling**: Robust retry strategies, circuit breakers, graceful degradation
5. **Testing Strategies**: Integration testing, property-based testing, benchmark validation
6. **Migration Patterns**: Zero-downtime upgrades, backward compatibility, rollback strategies

RESEARCH AREAS (by Technology):
- **Rust Async/Tokio**: spawn, join_all, tokio::process::Command, async error handling
- **SQLite**: ACID transactions, auto-vacuum modes, connection pooling, WAL mode, pragmas
- **Configuration**: Hot-reload patterns, filesystem watching (notify crate), validation
- **Benchmarking**: criterion.rs, statistical analysis, CI integration, regression detection
- **OAuth2**: Device code flow (RFC 8628), non-interactive auth, token refresh
- **Policy Enforcement**: Pre-commit hooks, CI validation, static analysis tools
- **Retry Logic**: Exponential backoff, jitter, circuit breaker patterns, tokio-retry

WEB SEARCHES TO CONDUCT (examples):
1. "Rust tokio async best practices 2024"
2. "SQLite ACID transactions Rust rusqlite patterns"
3. "Rust exponential backoff retry implementation"
4. "Criterion.rs benchmark best practices"
5. "OAuth2 device code flow Rust implementation"
6. "Rust notify crate hot reload configuration"
7. "Pre-commit hooks enforcement best practices"
8. "SQLite auto-vacuum incremental performance"
9. "Rust async process spawning tokio Command"
10. "Statistical benchmarking regression detection CI"

IMPLEMENTATION SPECS TO CREATE:
Create detailed implementation PRDs (child specs) as needed. Group by technology area or keep per-PRD as appropriate:

**Option A: Technology-Grouped Specs** (Recommended if overlaps exist):
- SPEC-945A: Async Orchestration Implementation (covers 933, 936)
  - tokio patterns, process spawning, parallel execution
- SPEC-945B: SQLite & Transactions Implementation (covers 933, 934)
  - ACID transactions, auto-vacuum, migrations, connection pooling
- SPEC-945C: Retry & Error Handling Implementation (covers 938)
  - Error classification, backoff algorithms, circuit breakers
- SPEC-945D: Configuration & Hot-Reload Implementation (covers 939)
  - notify crate, validation, JSON Schema integration
- SPEC-945E: Benchmarking & Instrumentation Implementation (covers 940)
  - criterion.rs, statistical analysis, CI integration
- SPEC-945F: Policy Compliance Implementation (covers 941)
  - Pre-commit hooks, CI checks, static analysis

**Option B: Per-PRD Specs** (Use if technologies are distinct):
- SPEC-945-933: Database Integrity Implementation Details
- SPEC-945-934: Storage Consolidation Implementation Details
- SPEC-945-936: Tmux Elimination Implementation Details
- SPEC-945-938: Enhanced Retry Implementation Details
- SPEC-945-939: Configuration Management Implementation Details
- SPEC-945-940: Performance Instrumentation Implementation Details
- SPEC-945-941: Policy Compliance Implementation Details

**Choose Option A or B** based on technology overlap discovered during research.

EACH IMPLEMENTATION SPEC SHOULD INCLUDE:
1. **Technology Research Summary** (2-3 pages)
   - Best practices discovered via web search
   - Recommended crates/libraries with justification
   - Performance characteristics, trade-offs
   - Links to authoritative sources (official docs, proven blog posts, RFCs)

2. **Detailed Implementation Plan** (3-5 pages)
   - Code structure (modules, files, traits)
   - Data flow diagrams (before/after)
   - Error handling strategies (specific error types, retry logic)
   - Testing approach (unit, integration, property-based, benchmarks)

3. **Code Examples** (1-2 pages)
   - Key function signatures
   - Critical algorithms (retry backoff, transaction coordination)
   - Integration patterns (how components connect)

4. **Migration Strategy** (1-2 pages)
   - Step-by-step migration path
   - Backward compatibility approach
   - Rollback procedure
   - Risk mitigation

5. **Performance Validation** (1 page)
   - Benchmarks to run (pre/post)
   - Success criteria (specific metrics)
   - Regression detection strategy

6. **Dependencies & Sequencing** (1 page)
   - Crate dependencies (with version constraints)
   - Implementation order (what to build first)
   - Integration points (how specs coordinate)

OUTPUT STRUCTURE:
1. Create `docs/SPEC-KIT-945-implementation-research/`
2. Create master `spec.md` with research summary and child spec index
3. Create child specs (945A-F or 945-933 through 945-941)
4. Each child spec: ~8-12 pages comprehensive implementation guide
5. Cross-reference PRDs (933-941) for requirements
6. Include web search findings with source URLs

VALIDATION CRITERIA:
- All 7 PRDs (933-941) have implementation guidance
- All research backed by authoritative sources (Rust docs, RFC, proven libraries)
- Code examples compile and follow Rust best practices
- Migration strategies account for production safety
- Performance validation includes statistical rigor
- Dependencies specify version constraints and compatibility

METHODOLOGY:
1. **Research Phase** (2-3 hours):
   - Conduct 10-15 web searches per technology area
   - Synthesize findings from official docs, proven blog posts, RFCs
   - Identify recommended crates (criterion, tokio-retry, notify, etc.)

2. **Grouping Phase** (30 min):
   - Identify technology overlaps (e.g., tokio patterns used in 933+936)
   - Decide Option A (technology-grouped) vs Option B (per-PRD)
   - Create master spec.md with child spec structure

3. **Implementation Spec Generation** (4-6 hours):
   - Create 6-7 detailed child specs (one per technology area or PRD)
   - Each spec: 8-12 pages with research, code examples, migration, validation
   - Include source URLs for all research findings

4. **Cross-Reference & Validate** (1 hour):
   - Verify all PRD requirements covered
   - Check for gaps or missing implementation details
   - Validate code examples compile
   - Ensure migration strategies are production-safe

ESTIMATED EFFORT: 8-10 hours total
- Research: 2-3 hours
- Grouping: 30 min
- Spec generation: 4-6 hours
- Validation: 1 hour

DELIVERABLES:
- Master spec: `docs/SPEC-KIT-945-implementation-research/spec.md`
- 6-7 child specs: `docs/SPEC-KIT-945-implementation-research/SPEC-945[A-F or -933 through -941].md`
- Total: ~50-80 pages comprehensive implementation guidance
- All research backed by web search with source URLs

SUCCESS CRITERIA:
- Developer can start coding any PRD with clear, detailed guidance
- All technology choices justified with research evidence
- Migration paths safe for production deployment
- Performance validation strategies defined with metrics
- No missing implementation details (ready to execute)

NEXT STEPS AFTER COMPLETION:
1. Review implementation specs for completeness
2. Validate technology choices align with project constraints
3. Begin Phase 1 implementation (SPEC-933, SPEC-934)
4. Use implementation specs as reference during coding
```

---

## Key Files for Reference

### SPEC-932 Planning Outputs:
- `docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md` (6,026 words)
- `docs/SPEC-KIT-934-storage-consolidation/PRD.md` (4,600 words)
- `docs/SPEC-KIT-936-tmux-elimination/PRD.md` (7,500 words)
- `docs/SPEC-KIT-938-enhanced-agent-retry/PRD.md` (3,200 words)
- `docs/SPEC-KIT-939-configuration-management/PRD.md` (4,800 words)
- `docs/SPEC-KIT-940-performance-instrumentation/PRD.md` (3,500 words)
- `docs/SPEC-KIT-941-automated-policy-compliance/PRD.md` (3,100 words)

### SPEC-931 Research (for context):
- `docs/SPEC-KIT-931-architectural-deep-dive/` (10 child specs A-J)
- Question consolidation analysis (222→135 questions)

### Project Architecture:
- `CLAUDE.md` - Development guidelines
- `MEMORY-POLICY.md` - Storage separation policy (SPEC-KIT-072)
- `codex-rs/` - Rust workspace root

---

## Expected Session 3 Output

**Master Spec**: SPEC-945 Implementation Research
- Research summary (all web searches conducted)
- Technology recommendations with justification
- Child spec index (6-7 implementation specs)

**Child Specs** (6-7 detailed implementation guides):
- Each 8-12 pages
- Research-backed best practices
- Code examples, migration strategies
- Performance validation plans

**Total Documentation**: ~50-80 pages implementation guidance

---

## Usage Instructions

**To start Session 3**:
1. Copy the ultrathink prompt from this document
2. Paste into new session
3. Let research and spec generation run (8-10 hours estimated)
4. Review output for completeness
5. Validate technology choices
6. Ready to begin implementation!

**Pattern Established**:
- SPEC-931: Deep architectural research (questions, analysis)
- SPEC-932: Planning & PRD generation (what/why)
- SPEC-945: Implementation research (how, with best practices)
- Then: Actual implementation (SPEC-933→941 execution)
