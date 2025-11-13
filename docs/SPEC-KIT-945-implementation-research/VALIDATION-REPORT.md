# VALIDATION REPORT: PRD to Implementation Coverage

**SPEC-ID**: SPEC-KIT-945
**Created**: 2025-11-13
**Status**: Complete
**Validation Date**: 2025-11-13

---

## Executive Summary

**Total PRDs Validated**: 7 (SPEC-KIT-933 through 941)
**Total Implementation Specs**: 6 (SPEC-945A through 945F)
**Coverage Status**: ✅ **100% Complete**
**Confidence Level**: **High**

All requirements from PRDs 933-941 are covered by at least one implementation spec. No critical gaps identified. Six implementation specs provide comprehensive technical solutions for all acceptance criteria.

**Key Findings**:
- All 7 PRDs fully mapped to implementation specs
- 100% acceptance criteria coverage validated
- All cross-spec dependencies identified and documented
- Implementation sequence optimized for dependency order

---

## 1. PRD Coverage Matrix

### SPEC-933: Database Integrity & Hygiene

| Requirement | Covered By | Status | Notes |
|-------------|-----------|---------|-------|
| ACID transactions (dual-write elimination) | SPEC-945B §3.2 | ✅ Complete | TransactionManager with BEGIN/COMMIT/ROLLBACK |
| Incremental auto-vacuum (153MB→<5MB) | SPEC-945B §3.3 | ✅ Complete | VacuumScheduler with PRAGMA auto_vacuum=INCREMENTAL |
| Parallel agent spawning (3× speedup) | SPEC-945A §3.1 | ✅ Complete | JoinSet for concurrent agent spawns |
| Daily cleanup automation | SPEC-945B §3.4 | ✅ Complete | Background daemon with tokio::interval |
| Event sourcing NO-GO documentation | SPEC-945B §4.5 | ✅ Complete | Decision record: ACID + YAGNI rationale |

**Acceptance Criteria Mapping**:
- AC1 (Data Integrity): SPEC-945B §3.2 (transactions)
- AC2 (Database Hygiene): SPEC-945B §3.3 (auto-vacuum)
- AC3 (Performance): SPEC-945A §3.1 (parallel spawning)
- AC4 (Cleanup Automation): SPEC-945B §3.4 (VacuumScheduler)
- AC5 (Documentation): SPEC-945B §4.5 (decision records)

---

### SPEC-934: Storage Consolidation

| Requirement | Covered By | Status | Notes |
|-------------|-----------|---------|-------|
| Migrate consensus MCP→SQLite (5× speedup) | SPEC-945B §3.1 | ✅ Complete | consensus_artifacts table with migration strategy |
| Eliminate MCP from orchestration | SPEC-945B §3.1 | ✅ Complete | Direct SQLite writes, no MCP calls |
| Restore SPEC-KIT-072 compliance | SPEC-945F §3.1 | ✅ Complete | Automated policy validation in CI |
| Reduce 4 storage systems→2 | SPEC-945B §1.1 | ✅ Complete | AGENT_MANAGER + SQLite only |
| consensus_synthesis investigation | SPEC-945B §3.1 | ✅ Complete | Dead code removal documented |

**Acceptance Criteria Mapping**:
- AC1 (Policy Compliance): SPEC-945F §3.1 (automated checks)
- AC2 (Performance): SPEC-945B §3.1 (SQLite vs MCP benchmarks)
- AC3 (Architecture Simplification): SPEC-945B §1.1 (4→2 systems)
- AC4 (Data Migration): SPEC-945B §3.1.4 (dual-write mode)

---

### SPEC-936: Tmux Elimination & Async Orchestration

| Requirement | Covered By | Status | Notes |
|-------------|-----------|---------|-------|
| Direct async API calls (65× speedup target) | SPEC-945A §3.2 | ✅ Complete | tokio::process::Command with stdout/stderr streaming |
| Filesystem collection removal | SPEC-945A §1.1 | ✅ Complete | Deprecate filesystem fallback path |
| OAuth2 device code flow | SPEC-945F §3.2 | ✅ Complete | RFC 8628 implementation for Google/Anthropic |
| Alternative diagnostics (logs, TUI viewer) | SPEC-945E §3.2 | ✅ Complete | Structured logging + performance metrics |
| Measurement gap acknowledgment | SPEC-945E §1.1 | ✅ Complete | Pre/post validation strategy |

**Acceptance Criteria Mapping**:
- AC1 (Tmux Elimination): SPEC-945A §3.2 (direct async spawning)
- AC2 (Performance): SPEC-945E §3.1 (benchmark validation)
- AC3 (Filesystem Cleanup): SPEC-945A §1.1 (deprecation path)
- AC4 (Authentication): SPEC-945F §3.2 (OAuth2 device code)
- AC5 (Diagnostics): SPEC-945E §3.2 (tracing infrastructure)

---

### SPEC-938: Enhanced Agent Retry Logic

| Requirement | Covered By | Status | Notes |
|-------------|-----------|---------|-------|
| Error classification (retryable vs permanent) | SPEC-945C §3.1 | ✅ Complete | ErrorCategory enum with provider-specific codes |
| Exponential backoff with jitter | SPEC-945C §3.2 | ✅ Complete | Backoff progression: 1s→2s→4s with ±50% jitter |
| Max retry limits (3 attempts) | SPEC-945C §3.2 | ✅ Complete | RetryPolicy with configurable max_attempts |
| Quality gate integration | SPEC-945C §3.3 | ✅ Complete | Retry within gate before degrading to 2/3 |
| Comprehensive telemetry | SPEC-945E §3.2 | ✅ Complete | Retry attempts + success rate metrics |

**Acceptance Criteria Mapping**:
- AC1 (Error Classification): SPEC-945C §3.1 (all error types)
- AC2 (Exponential Backoff): SPEC-945C §3.2 (jitter + max backoff)
- AC3 (Quality Gate Integration): SPEC-945C §3.3 (retry before degrade)
- AC4 (Telemetry): SPEC-945E §3.2 (structured logging)

---

### SPEC-939: Configuration Management

| Requirement | Covered By | Status | Notes |
|-------------|-----------|---------|-------|
| Hot-reload config when idle | SPEC-945D §3.1 | ✅ Complete | notify crate with filesystem watch |
| Canonical name field (eliminate confusion) | SPEC-945D §3.2 | ✅ Complete | Single canonical_name per agent |
| Startup config validation | SPEC-945D §3.3 | ✅ Complete | Validate required fields, CLI availability, API keys |
| Configurable quality gate agents | SPEC-945D §3.4 | ✅ Complete | quality_gates section in config.json |
| Pluggable validation layers | SPEC-945D §3.4 | ✅ Complete | Per-agent validation config (syntax/schema/semantic) |
| JSON Schema documentation | SPEC-945D §3.5 | ✅ Complete | config.schema.json with IDE integration |
| API key naming guide | SPEC-945F §3.2 | ✅ Complete | Provider name convention (GOOGLE_API_KEY not GEMINI_API_KEY) |

**Acceptance Criteria Mapping**:
- AC1 (Hot-Reload): SPEC-945D §3.1 (filesystem watch + prompt)
- AC2 (Canonical Name): SPEC-945D §3.2 (schema migration)
- AC3 (Startup Validation): SPEC-945D §3.3 (pre-flight checks)
- AC4 (Configurable Agents): SPEC-945D §3.4 (per-checkpoint agents)
- AC5 (Pluggable Validation): SPEC-945D §3.4 (validation layers)
- AC6 (JSON Schema): SPEC-945D §3.5 (autocomplete + validation)
- AC7 (Documentation): SPEC-945F §3.2 (authentication guide)

---

### SPEC-940: Performance Instrumentation

| Requirement | Covered By | Status | Notes |
|-------------|-----------|---------|-------|
| Timing infrastructure (measure_time! macro) | SPEC-945E §3.1 | ✅ Complete | tracing::info with elapsed_ms |
| Benchmark harness (n≥10 iterations) | SPEC-945E §3.1 | ✅ Complete | BenchmarkHarness with statistics |
| Statistical reporting (mean±stddev) | SPEC-945E §3.1 | ✅ Complete | Markdown reports with percentiles |
| Pre/post validation (SPEC-936, 934, 933) | SPEC-945E §3.1 | ✅ Complete | Baseline + validation tests |
| Instrumentation points (P0/P1/P2) | SPEC-945E §3.2 | ✅ Complete | Prioritized coverage (tmux, SQLite, config) |

**Acceptance Criteria Mapping**:
- AC1 (Timing Infrastructure): SPEC-945E §3.1 (measure_time!)
- AC2 (Benchmark Harness): SPEC-945E §3.1 (statistics calculation)
- AC3 (Statistical Reporting): SPEC-945E §3.1 (Markdown output)
- AC4 (Pre/Post Validation): SPEC-945E §3.1 (baseline comparison)

---

### SPEC-941: Automated Policy Compliance

| Requirement | Covered By | Status | Notes |
|-------------|-----------|---------|-------|
| Storage separation validator (SPEC-KIT-072) | SPEC-945F §3.1 | ✅ Complete | Static analysis for MCP consensus violations |
| CI integration (block PRs on violation) | SPEC-945F §3.1 | ✅ Complete | GitHub Actions policy-compliance job |
| Pre-commit hooks (fast feedback) | SPEC-945F §3.1 | ✅ Complete | <5s validation before commit |
| Policy compliance dashboard | SPEC-945F §3.1 | ✅ Complete | Markdown dashboard with pass/fail per rule |
| Tag schema validator | SPEC-945F §3.1 | ✅ Complete | Detect forbidden date tags, task IDs |
| MCP importance threshold check (≥8) | SPEC-945F §3.1 | ✅ Complete | Prevent memory bloat |

**Acceptance Criteria Mapping**:
- AC1 (Storage Separation): SPEC-945F §3.1 (grep-based detection)
- AC2 (CI Integration): SPEC-945F §3.1 (GitHub Actions)
- AC3 (Pre-Commit Hook): SPEC-945F §3.1 (mandatory installation)
- AC4 (Policy Dashboard): SPEC-945F §3.1 (status visualization)
- AC5 (Tag Schema): SPEC-945F §3.1 (forbidden tag detection)

---

## 2. Gap Analysis

### Missing Coverage

**None identified.** All PRD requirements are covered by at least one implementation spec.

### Incomplete Coverage

**None identified.** All requirements are fully specified with implementation details.

### Over-Coverage (Additional Features)

**SPEC-945E Performance Instrumentation** includes:
- Percentile reporting (P50, P95, P99) - Beyond PRD requirements
- Statistical significance testing (Welch's t-test) - Beyond PRD requirements
- **Justification**: Essential for validating SPEC-936 claims (65× speedup estimate). Measurement gap acknowledged in SPEC-936 PRD requires rigorous validation.

**SPEC-945F Policy Compliance** includes:
- Tag schema validation - Beyond SPEC-941 PRD (not explicitly required)
- **Justification**: Prevents tag proliferation identified in MEMORY-POLICY.md analysis. Addresses root cause of memory bloat.

---

## 3. Integration Validation

### Cross-Spec Dependencies

```
SPEC-945A (Async Orchestration)
  ↓ requires
SPEC-945B (SQLite & Transactions)
  ← enables async DB operations with ACID guarantees

SPEC-945A (Async Orchestration)
  ↓ benefits from
SPEC-945C (Retry & Error Handling)
  ← retry in async context (tokio::time::sleep)

SPEC-945B (SQLite & Transactions)
  ↓ coordinates with
SPEC-945C (Retry & Error Handling)
  ← SQLITE_BUSY retry with exponential backoff

SPEC-945D (Configuration & Hot-Reload)
  ↓ triggers
SPEC-945A (Async Orchestration)
  ← hot-reload events via async channels

SPEC-945E (Benchmarking)
  ↓ validates
All SPECs
  ← performance claims (pre/post comparison)

SPEC-945F (Policy Compliance)
  ↓ enforces
SPEC-945B (SQLite)
  ← SPEC-KIT-072 compliance (consensus → SQLite)
```

### Circular Dependencies

**None identified.** Dependency graph is acyclic.

### Implementation Order (Critical Path)

```
Phase 1: Foundation (Parallel)
├─ SPEC-945B: SQLite & Transactions (enables storage consolidation)
│  ├─ consensus_artifacts table
│  ├─ TransactionManager
│  └─ VacuumScheduler
└─ SPEC-945C: Retry & Error Handling (reliability foundation)
   ├─ ErrorCategory enum
   └─ RetryPolicy with backoff

Phase 2: Orchestration (Depends on Phase 1)
└─ SPEC-945A: Async Orchestration (unlocks 3× and 65× speedups)
   ├─ Direct async spawning (requires SPEC-945C retry)
   ├─ JoinSet for parallel agents (requires SPEC-945B transactions)
   └─ Filesystem deprecation

Phase 3: Quality & Configuration (Parallel)
├─ SPEC-945D: Configuration & Hot-Reload (enables flexibility)
│  ├─ Hot-reload (async events require SPEC-945A)
│  ├─ Canonical naming
│  └─ JSON Schema
├─ SPEC-945E: Benchmarking & Instrumentation (validates Phase 2)
│  ├─ Baseline measurements (before SPEC-945A)
│  ├─ measure_time! macro
│  └─ BenchmarkHarness
└─ SPEC-945F: Policy Compliance (enforces Phase 1)
   ├─ Storage validation (SPEC-945B compliance)
   ├─ OAuth2 device code
   └─ CI integration

CRITICAL PATH: SPEC-945B → SPEC-945A → SPEC-945E (validation)
TOTAL DURATION: ~6-8 weeks (with Phase 1+2 sequential, Phase 3 parallel)
```

---

## 4. Acceptance Criteria Validation

### SPEC-933 Acceptance Criteria

**AC1: Data Integrity ✅**
- [x] All HashMap + SQLite updates wrapped in transactions (SPEC-945B §3.2.1)
  - **Testable**: Check transaction boundaries with commit/rollback tests
- [x] Crash recovery tests pass (SPEC-945B §3.2.3)
  - **Testable**: kill -9 during transaction → verify no corruption
- [x] Rollback verification (SPEC-945B §3.2.2)
  - **Testable**: Simulate failed write → verify both HashMap + SQLite revert
- [x] Concurrent agent spawning doesn't cause corruption (SPEC-945A §3.1.2)
  - **Testable**: Spawn 3 agents simultaneously → verify no race conditions

**AC2: Database Hygiene ✅**
- [x] Auto-vacuum enabled (SPEC-945B §3.3.1)
  - **Testable**: Check `PRAGMA auto_vacuum` returns INCREMENTAL
- [x] Database size <10MB after migration (SPEC-945B §3.3.2)
  - **Testable**: Measure DB size post-migration → assert <10MB
- [x] Incremental vacuum runs on idle (SPEC-945B §3.3.3)
  - **Testable**: Monitor `PRAGMA incremental_vacuum` execution
- [x] Full vacuum migration successful (SPEC-945B §3.3.2)
  - **Testable**: Run migration script → verify 153MB→<5MB reduction

**AC3: Performance ✅**
- [x] Parallel spawning implemented (SPEC-945A §3.1.1)
  - **Testable**: Use JoinSet → verify 3 agents spawn concurrently
- [x] Spawn time <70ms (SPEC-945E §3.1)
  - **Testable**: Benchmark harness → mean <70ms over n≥10 runs
- [x] Batch SQLite writes in single transaction (SPEC-945B §3.2.1)
  - **Testable**: Verify BEGIN → multiple INSERTs → COMMIT pattern
- [x] No performance regression on single-agent operations (SPEC-945E §3.1)
  - **Testable**: Baseline vs post-implementation comparison

**AC4: Cleanup Automation ✅**
- [x] Cron/scheduler configured for daily 2am execution (SPEC-945B §3.4)
  - **Testable**: Platform-specific installation scripts verify scheduler setup
- [x] Platform-specific installers (SPEC-945B §3.4)
  - **Testable**: Test on Linux (systemd), macOS (launchd), Windows (Task Scheduler)
- [x] Cleanup deletes records >30 days old (SPEC-945B §3.4)
  - **Testable**: Insert old records → run cleanup → verify deletion
- [x] Logs record cleanup operations (SPEC-945B §3.4)
  - **Testable**: Check log files for cleanup timestamps + record counts

**AC5: Documentation ✅**
- [x] Event sourcing rejection documented (SPEC-945B §4.5)
  - **Testable**: Verify `docs/decisions/933-event-sourcing-rejection.md` exists
- [x] SPEC-931F findings summarized (SPEC-945B §4.5)
  - **Testable**: Check decision record includes 16 ultrathink questions
- [x] Trigger conditions for revisiting documented (SPEC-945B §4.5)
  - **Testable**: Verify "when to revisit" section exists (e.g., 100+ agents/min)

---

### SPEC-934 Acceptance Criteria

**AC1: Policy Compliance ✅**
- [x] All consensus artifacts stored in SQLite (SPEC-945B §3.1)
  - **Testable**: Grep for `mcp.*consensus` in spec_kit/ → 0 results
- [x] MCP only used for human-curated knowledge (SPEC-945F §3.1)
  - **Testable**: CI policy validation passes
- [x] SPEC-KIT-072 compliance validated (SPEC-945F §3.1)
  - **Testable**: Automated check script exits 0
- [x] Documentation updated (SPEC-945B §4.1)
  - **Testable**: MEMORY-POLICY.md references SPEC-934 fix

**AC2: Performance ✅**
- [x] Consensus storage <50ms (SPEC-945E §3.1)
  - **Testable**: Benchmark SQLite writes → mean <50ms (down from 150ms MCP)
- [x] Consensus retrieval <10ms (SPEC-945E §3.1)
  - **Testable**: Benchmark SQLite reads → mean <10ms (down from 200ms MCP)
- [x] No performance regression on quality gates (SPEC-945E §3.1)
  - **Testable**: End-to-end quality gate benchmark comparison

**AC3: Architecture Simplification ✅**
- [x] MCP removed from agent orchestration code (SPEC-945B §3.1)
  - **Testable**: No MCP imports in spec_kit/ modules
- [x] 4 storage systems → 2 (SPEC-945B §1.1)
  - **Testable**: Architecture diagram shows only AGENT_MANAGER + SQLite
- [x] consensus_synthesis table investigated (SPEC-945B §3.1.5)
  - **Testable**: Decision documented (DROP or keep with rationale)

**AC4: Data Migration ✅**
- [x] Existing MCP consensus artifacts migrated (SPEC-945B §3.1.4)
  - **Testable**: Optional migration script runs successfully (if executed)
- [x] No data loss during migration (SPEC-945B §3.1.4)
  - **Testable**: Record count before/after migration matches
- [x] All quality gate tests pass with SQLite storage (SPEC-945B §3.1)
  - **Testable**: Full test suite passes post-migration

---

### SPEC-936 Acceptance Criteria

**AC1: Tmux Elimination ✅**
- [x] All agent spawning uses direct async calls (SPEC-945A §3.2)
  - **Testable**: Grep for `tmux` commands in orchestration → 0 results
- [x] Tmux session creation code removed (SPEC-945A §3.2)
  - **Testable**: No `create_tmux_session()` function in codebase
- [x] Pane management code removed (SPEC-945A §3.2)
  - **Testable**: No `tmux split-window` or `tmux send-keys` commands
- [x] Stability polling removed (SPEC-945A §3.2)
  - **Testable**: No `poll_tmux_stability()` function

**AC2: Performance ✅**
- [x] Agent spawn time <200ms (SPEC-945E §3.1)
  - **Testable**: Benchmark harness → mean <200ms (target 0.1s ±100ms variance)
- [x] End-to-end quality gate 20-30% faster (SPEC-945E §3.1)
  - **Testable**: Before/after comparison (6.5s overhead eliminated)
- [x] No performance regression on single-agent operations (SPEC-945E §3.1)
  - **Testable**: Baseline vs post-implementation comparison

**AC3: Filesystem Cleanup ✅**
- [x] Legacy filesystem collection removed (SPEC-945A §1.1)
  - **Testable**: No `fetch_agent_payloads_from_filesystem()` in codebase
- [x] `~/.code/agents/` directory deleted (SPEC-945A §1.1)
  - **Testable**: Directory does not exist post-cleanup
- [x] All quality gate tests pass without filesystem fallback (SPEC-945A §1.1)
  - **Testable**: Full test suite passes without filesystem reads

**AC4: Authentication ✅**
- [x] OAuth2 device code flow investigated (SPEC-945F §3.2)
  - **Testable**: Provider support matrix documented (Google, Anthropic, OpenAI)
- [x] Implementation for ≥1 provider (SPEC-945F §3.2)
  - **Testable**: Google device code flow works (RFC 8628 compliant)
- [x] Fallback strategy documented (SPEC-945F §3.2)
  - **Testable**: Manual pre-auth guide exists in docs/authentication.md
- [x] Clear error messages when authentication required (SPEC-945F §3.2)
  - **Testable**: Missing API key → error message includes `export GOOGLE_API_KEY=...`

**AC5: Diagnostics ✅**
- [x] Structured logging with agent context (SPEC-945E §3.2)
  - **Testable**: Logs include agent_id, elapsed_ms fields
- [x] TUI log viewer widget implemented (SPEC-945E §3.2)
  - **Testable**: `L` keybinding opens AgentLogViewer widget
- [x] Evidence files capture prompts + responses (SPEC-945E §3.2)
  - **Testable**: Evidence directory contains `*_prompt.md`, `*_response.json`
- [x] `--debug-agent` manual mode works (SPEC-945E §3.2)
  - **Testable**: `codex-tui --debug-agent gemini` shows live log stream

---

### SPEC-938 Acceptance Criteria

**AC1: Error Classification ✅**
- [x] All agent errors classified (SPEC-945C §3.1)
  - **Testable**: ErrorCategory covers timeout, rate limit, 5xx, 4xx, network, API key
- [x] Classification logic covers all error types (SPEC-945C §3.1)
  - **Testable**: Unit tests verify each error maps to correct category
- [x] Unit tests verify classification correctness (SPEC-945C §3.1)
  - **Testable**: Test suite includes 15+ error classification tests

**AC2: Exponential Backoff ✅**
- [x] Backoff follows exponential progression (SPEC-945C §3.2)
  - **Testable**: Verify 1s → 2s → 4s → ... pattern in logs
- [x] Jitter added to prevent thundering herd (SPEC-945C §3.2)
  - **Testable**: Verify ±50% variance in backoff delays
- [x] Max backoff limit enforced (SPEC-945C §3.2)
  - **Testable**: Verify backoff never exceeds 16s (config.max_backoff_ms)
- [x] Max retry attempts enforced (SPEC-945C §3.2)
  - **Testable**: Verify failure after 3 attempts (config.max_attempts)

**AC3: Quality Gate Integration ✅**
- [x] Quality gates retry failed agents before degrading to 2/3 (SPEC-945C §3.3)
  - **Testable**: Simulate transient failure → verify retry → 3/3 consensus
- [x] Full 3/3 consensus achieved via retry (SPEC-945C §3.3)
  - **Testable**: Quality gate test with 1 transient failure → 3/3 result
- [x] Telemetry logs retry attempts with context (SPEC-945E §3.2)
  - **Testable**: Logs include attempt number, backoff_ms, error_category

**AC4: Telemetry ✅**
- [x] Retry attempts logged with backoff delays (SPEC-945E §3.2)
  - **Testable**: Structured logs include `attempt`, `backoff_ms` fields
- [x] Success/failure after retries logged (SPEC-945E §3.2)
  - **Testable**: Logs include `total_attempts`, `final_status` fields
- [x] Error categories logged (SPEC-945E §3.2)
  - **Testable**: Logs include `error_category` field (retryable/permanent)

---

### SPEC-939 Acceptance Criteria

**AC1: Hot-Reload ✅**
- [x] TUI detects config file changes (SPEC-945D §3.1)
  - **Testable**: Edit config.json → verify filesystem watch triggers event
- [x] Prompts user to reload when idle (SPEC-945D §3.1)
  - **Testable**: Config change → TUI shows "Config changed. Reload? [Y/n]"
- [x] Defers reload if quality gate active (SPEC-945D §3.1)
  - **Testable**: Config change during quality gate → reload deferred
- [x] Session state preserved after reload (SPEC-945D §3.1)
  - **Testable**: Reload config → verify active SPEC routing preserved

**AC2: Canonical Name ✅**
- [x] All agents have `canonical_name` field (SPEC-945D §3.2)
  - **Testable**: JSON Schema validation enforces canonical_name presence
- [x] Code references `canonical_name` everywhere (SPEC-945D §3.2)
  - **Testable**: Grep for `agent.name` → 0 results (all use canonical_name)
- [x] Auto-migration for old configs (SPEC-945D §3.2)
  - **Testable**: Load old config.json → verify canonical_name populated from name

**AC3: Startup Validation ✅**
- [x] Config validated on TUI startup (SPEC-945D §3.3)
  - **Testable**: Invalid config → TUI fails to start with clear error
- [x] Required fields checked (SPEC-945D §3.3)
  - **Testable**: Missing canonical_name → error "Agent missing canonical_name"
- [x] Command availability verified (SPEC-945D §3.3)
  - **Testable**: Non-existent CLI → error "Command 'gemni' not found"
- [x] API keys checked if requires_auth (SPEC-945D §3.3)
  - **Testable**: Missing GOOGLE_API_KEY → error "export GOOGLE_API_KEY=..."
- [x] Clear error messages with hints (SPEC-945D §3.3)
  - **Testable**: All errors include file path, field name, fix hint

**AC4: Configurable Agents ✅**
- [x] Quality gate agents loaded from config (SPEC-945D §3.4)
  - **Testable**: Edit quality_gates.plan.agents → verify agents used
- [x] Users can customize agent selection per checkpoint (SPEC-945D §3.4)
  - **Testable**: Set tasks to single agent → verify only 1 spawned
- [x] Validation: Quality gate agents must exist in agents array (SPEC-945D §3.4)
  - **Testable**: Reference non-existent agent → startup error

**AC5: Pluggable Validation ✅**
- [x] Validation layers configurable per agent (SPEC-945D §3.4)
  - **Testable**: Set agent.validation.layers → verify only those run
- [x] Timeout configurable per agent (SPEC-945D §3.4)
  - **Testable**: Set agent.validation.timeout_ms → verify enforced
- [x] Strict mode toggle (SPEC-945D §3.4)
  - **Testable**: strict_mode=false → minor validation failures ignored

**AC6: JSON Schema ✅**
- [x] config.schema.json created (SPEC-945D §3.5)
  - **Testable**: Schema file exists, validates against JSON Schema draft-07
- [x] IDE autocomplete works (SPEC-945D §3.5)
  - **Testable**: VS Code shows autocomplete for config.json fields
- [x] Schema validation on file save (SPEC-945D §3.5)
  - **Testable**: Invalid config → VS Code shows validation errors

**AC7: Documentation ✅**
- [x] API key naming guide (SPEC-945F §3.2)
  - **Testable**: docs/authentication.md exists, covers all providers
- [x] Clear provider → env var mapping (SPEC-945F §3.2)
  - **Testable**: Table shows GOOGLE_API_KEY (not GEMINI_API_KEY)
- [x] Common mistakes documented (SPEC-945F §3.2)
  - **Testable**: "Common Mistakes" section lists GEMINI_API_KEY as wrong

---

### SPEC-940 Acceptance Criteria

**AC1: Timing Infrastructure ✅**
- [x] measure_time! macro implemented (SPEC-945E §3.1)
  - **Testable**: Macro compiles, outputs elapsed_ms to logs
- [x] All P0 instrumentation points covered (SPEC-945E §3.2)
  - **Testable**: Logs show timing for tmux, spawning, transactions
- [x] All P1 instrumentation points covered (SPEC-945E §3.2)
  - **Testable**: Logs show timing for MCP, SQLite, config
- [x] Logs capture operation name + elapsed time (SPEC-945E §3.2)
  - **Testable**: Structured logs include `operation`, `elapsed_ms` fields

**AC2: Benchmark Harness ✅**
- [x] BenchmarkHarness runs n≥10 iterations (SPEC-945E §3.1)
  - **Testable**: Harness config specifies iterations=10, verify all run
- [x] Warmup iterations discard first 2 runs (SPEC-945E §3.1)
  - **Testable**: Warmup results not included in statistics
- [x] Statistics calculated (SPEC-945E §3.1)
  - **Testable**: Output includes mean, stddev, min/max, P50/P95/P99
- [x] Failed iterations excluded from stats (SPEC-945E §3.1)
  - **Testable**: Simulate 1 failed run → statistics computed from n=9

**AC3: Statistical Reporting ✅**
- [x] Performance reports generated (SPEC-945E §3.1)
  - **Testable**: Markdown table created with all benchmarks
- [x] Reports saved to evidence directory (SPEC-945E §3.1)
  - **Testable**: File exists at `docs/SPEC-KIT-936/evidence/performance-baseline.md`
- [x] All benchmarks include comprehensive stats (SPEC-945E §3.1)
  - **Testable**: Verify mean±stddev, min, P50, P95, P99, max, n columns

**AC4: Pre/Post Validation ✅**
- [x] SPEC-936 baseline measured (before tmux elimination) (SPEC-945E §3.1)
  - **Testable**: Baseline file exists with tmux spawn times
- [x] SPEC-936 validation measured (after tmux elimination) (SPEC-945E §3.1)
  - **Testable**: Validation file exists with direct spawn times
- [x] SPEC-934 baseline measured (MCP storage) (SPEC-945E §3.1)
  - **Testable**: Baseline file exists with MCP timing
- [x] SPEC-934 validation measured (SQLite storage) (SPEC-945E §3.1)
  - **Testable**: Validation file exists with SQLite timing
- [x] Statistical significance tested (SPEC-945E §3.1)
  - **Testable**: Welch's t-test p<0.05 for all improvements

---

### SPEC-941 Acceptance Criteria

**AC1: Storage Separation Validation ✅**
- [x] Script detects MCP consensus storage (SPEC-945F §3.1)
  - **Testable**: Add `mcp.*consensus` to spec_kit/ → script catches violation
- [x] Script verifies SQLite consensus storage (SPEC-945F §3.1)
  - **Testable**: Verify ≥3 consensus_artifacts call sites detected
- [x] Script checks MCP importance threshold (SPEC-945F §3.1)
  - **Testable**: Add importance=7 → script catches violation
- [x] Clear error messages with context (SPEC-945F §3.1)
  - **Testable**: Error includes file path, line number, fix hint

**AC2: CI Integration ✅**
- [x] GitHub Actions job runs on every PR (SPEC-945F §3.1)
  - **Testable**: Open PR → verify policy-compliance job executes
- [x] Policy violations block PR merge (SPEC-945F §3.1)
  - **Testable**: Introduce violation → CI fails, blocks merge
- [x] CI job completes in <30s (SPEC-945F §3.1)
  - **Testable**: Monitor CI execution time → assert <30s

**AC3: Pre-Commit Hook ✅**
- [x] Hook runs on spec_kit module changes (SPEC-945F §3.1)
  - **Testable**: Edit spec_kit/ file → commit → hook executes
- [x] Hook provides <5s feedback (SPEC-945F §3.1)
  - **Testable**: Monitor hook execution time → assert <5s
- [x] Hook allows bypass for emergencies (SPEC-945F §3.1)
  - **Testable**: `git commit --no-verify` bypasses hook
- [x] Installation mandatory via setup-hooks.sh (SPEC-945F §3.1)
  - **Testable**: README.md step 1 requires running setup-hooks.sh

**AC4: Policy Dashboard ✅**
- [x] Dashboard shows all policy rules (SPEC-945F §3.1)
  - **Testable**: Dashboard includes storage, tags, importance sections
- [x] Status per rule (SPEC-945F §3.1)
  - **Testable**: Each rule shows ✅ PASS or ❌ FAIL
- [x] Generated as Markdown (SPEC-945F §3.1)
  - **Testable**: Output is valid Markdown with tables

**AC5: Tag Schema Validation ✅**
- [x] Detects forbidden date tags (SPEC-945F §3.1)
  - **Testable**: Add "2025-10-20" tag → script catches violation
- [x] Detects forbidden task ID tags (SPEC-945F §3.1)
  - **Testable**: Add "t84" tag → script catches violation
- [x] Encourages namespaced tags (SPEC-945F §3.1)
  - **Testable**: Script reports count of spec:, type:, component: tags

---

## 5. Implementation Sequence Recommendations

### Critical Path Analysis

**CRITICAL PATH**: SPEC-945B → SPEC-945A → SPEC-945E (validation)

**Rationale**:
1. **SPEC-945B (SQLite)** must be first:
   - Provides consensus storage (SPEC-934 compliance)
   - Enables ACID transactions (SPEC-933 safety)
   - Required by SPEC-945A (async DB operations)

2. **SPEC-945A (Async)** depends on SPEC-945B:
   - Parallel spawning requires transaction safety
   - Direct spawning eliminates tmux (SPEC-936 speedup)
   - Must measure baseline BEFORE implementation (SPEC-945E)

3. **SPEC-945E (Benchmarking)** validates SPEC-945A:
   - Baseline measurements before tmux elimination
   - Post-implementation validation (prove 65× claim)
   - Statistical rigor for all performance claims

### Phased Rollout Strategy

**Phase 1: Foundation (Weeks 1-2)**
```
PARALLEL:
├─ SPEC-945B: SQLite & Transactions (2-3 weeks)
│  ├─ Week 1: Transaction infrastructure + auto-vacuum
│  └─ Week 2: Cleanup automation + MCP migration
└─ SPEC-945C: Retry & Error Handling (1 day)
   ├─ Error classification (3-4h)
   └─ Exponential backoff (2-3h)

RISK: SPEC-945B is blocking, SPEC-945C can proceed independently
```

**Phase 2: Orchestration (Weeks 3-5)**
```
SEQUENTIAL (depends on Phase 1):
└─ SPEC-945E: Baseline Measurements (3 days)
   ├─ Day 1: Timing infrastructure (P0/P1 instrumentation)
   ├─ Day 2: Benchmark harness + statistical reporting
   └─ Day 3: BASELINE measurements (tmux, MCP, sequential spawn)

THEN:
└─ SPEC-945A: Async Orchestration (2-3 weeks)
   ├─ Week 1: Direct async spawning (20-30h)
   ├─ Week 2: Filesystem cleanup (5-7h) + OAuth2 investigation (8-12h)
   └─ Week 3: Alternative diagnostics (7-10h)

THEN:
└─ SPEC-945E: Validation Measurements (1 day)
   └─ Compare baseline vs post-implementation (prove speedup)

RISK: Must measure baseline BEFORE SPEC-945A implementation
```

**Phase 3: Quality & Compliance (Weeks 6-8, Parallel)**
```
PARALLEL:
├─ SPEC-945D: Configuration & Hot-Reload (1-1.5 weeks)
│  ├─ Week 1: Hot-reload + canonical name (5-6h)
│  │         Configurable agents (8-12h)
│  │         Pluggable validation (2-3h)
│  └─ Week 2: Startup validation (3-4h)
│             Error messages (1-2h)
│             JSON Schema (2-3h)
│             Documentation (1-2h)
└─ SPEC-945F: Policy Compliance (1-2 days)
   ├─ Day 1: Storage validation + CI (5-6h)
   └─ Day 2: Tag schema + dashboard + hooks (3-4h)

RISK: None (both independent of critical path)
```

### Recommended Milestones

**Milestone 1: Data Integrity & Reliability (End of Phase 1)**
- ✅ ACID transactions operational
- ✅ Database bloat eliminated (153MB→<5MB)
- ✅ Retry logic with exponential backoff
- ✅ Daily cleanup automation

**Milestone 2: Performance Optimization (End of Phase 2)**
- ✅ 65× speedup validated (tmux elimination)
- ✅ 3× speedup validated (parallel spawning)
- ✅ 5× speedup validated (SQLite vs MCP)
- ✅ Statistical evidence for all claims

**Milestone 3: Quality & Flexibility (End of Phase 3)**
- ✅ Hot-reload configuration
- ✅ Configurable quality gates
- ✅ Policy compliance automated (CI + pre-commit)
- ✅ OAuth2 device code flows

---

## 6. Risk Mitigation Strategies

### High-Priority Risks

**Risk 1: Baseline Measurement Gap** (SPEC-936, SPEC-940)
- **Mitigation**: Implement SPEC-945E baseline measurements BEFORE SPEC-945A
- **Validation**: Evidence files exist for tmux spawn, MCP storage, sequential spawn
- **Fallback**: If baseline missed, document as limitation in SPEC-936 validation

**Risk 2: SPEC-945B Blocking Critical Path**
- **Mitigation**: Prioritize SPEC-945B, allocate 2-3 weeks upfront
- **Validation**: Transaction tests pass before proceeding to SPEC-945A
- **Fallback**: SPEC-945C can proceed in parallel (no dependency)

**Risk 3: Policy Drift During Implementation** (SPEC-941)
- **Mitigation**: Implement SPEC-945F CI checks in Phase 1 (before Phase 2 changes)
- **Validation**: CI blocks any policy-violating PRs
- **Fallback**: Manual code review for policy compliance

### Medium-Priority Risks

**Risk 4: Hot-Reload Edge Cases** (SPEC-939)
- **Mitigation**: Comprehensive concurrency tests (idle→active race conditions)
- **Validation**: Lock quality gate state during reload check
- **Fallback**: Defer reload until quality gate completes

**Risk 5: OAuth2 Provider Support** (SPEC-936)
- **Mitigation**: Research phase before implementation (SPEC-945F §3.2)
- **Validation**: Google device code flow minimum requirement
- **Fallback**: Manual pre-auth fallback for unsupported providers

---

## 7. Success Metrics Summary

### Performance Targets (Validated by SPEC-945E)

| Metric | Baseline (Estimated) | Target | Validation Method |
|--------|---------------------|--------|-------------------|
| Agent spawn time | 6.5s (tmux) | <200ms (65× faster) | BenchmarkHarness n≥10 |
| Parallel spawn (3 agents) | 150ms (sequential) | 50ms (3× faster) | JoinSet timing |
| Consensus storage | 150ms (MCP) | <50ms (SQLite, 3× faster) | SQLite benchmark |
| Consensus retrieval | 200ms (MCP) | <10ms (SQLite, 20× faster) | SQL query benchmark |
| Database size | 153MB (bloat) | <5MB (96% reduction) | File size measurement |

### Quality Metrics

| Metric | Target | Validation Method |
|--------|--------|-------------------|
| Transaction atomicity | 100% (crash recovery) | kill -9 tests |
| Policy compliance | 100% (no violations) | CI automated checks |
| Test coverage | ≥80% (all components) | cargo tarpaulin |
| Configuration errors | <5s detection (startup) | Validation script |

### User Experience Metrics

| Metric | Target | Validation Method |
|--------|--------|-------------------|
| Config restarts | 90% reduction | Hot-reload adoption |
| Error fix time | 80% faster | Clear error messages |
| Authentication setup | <5min (device code) | User testing |

---

## Conclusion

All 7 PRDs (SPEC-933 through 941) are **100% covered** by the 6 implementation specs (SPEC-945A through 945F). No gaps identified. All acceptance criteria are testable and mapped to specific implementation sections.

**Key Takeaways**:
1. **Critical Path**: SPEC-945B → SPEC-945A → SPEC-945E (validation)
2. **Total Duration**: 6-8 weeks (sequential critical path + parallel quality work)
3. **Highest Risk**: Baseline measurements must occur before SPEC-945A implementation
4. **Confidence**: High - comprehensive coverage with detailed implementation plans

**Recommended Next Steps**:
1. Approve SPEC-945B implementation (Week 1-2 start date)
2. Schedule SPEC-945E baseline measurements (before SPEC-945A)
3. Set up SPEC-945F CI checks early (prevent policy drift)
4. Allocate resources: 2 developers × 6-8 weeks for critical path

---

**Validation Sign-Off**:
- [x] All PRDs reviewed against implementation specs
- [x] All acceptance criteria mapped and validated as testable
- [x] Implementation sequence optimized for dependencies
- [x] Risk mitigation strategies documented

**Report Generated**: 2025-11-13
**Next Review**: After Phase 1 completion (SPEC-945B + SPEC-945C)
