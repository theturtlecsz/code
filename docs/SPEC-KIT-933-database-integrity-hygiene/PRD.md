# SPEC-KIT-933: Database Integrity & Hygiene

**Status**: BACKLOG (75% Complete via SPEC-945B)
**Priority**: P0 - CRITICAL (Data Corruption Risk)
**Created**: 2025-11-13 (from SPEC-932 Planning Session)
**Updated**: 2025-11-14 (Reconstructed from Research)  
**Estimated Effort**: 18-27 hours (remaining), 65-96 hours (original)

---

## Executive Summary

**Problem**: Consensus artifacts database exhibited critical integrity issues: dual-write corruption risks, 153MB bloat, slow agent spawning (150ms), unbounded evidence growth.

**Solution**: Four-component database integrity overhaul: ACID compliance, storage optimization, performance improvements, operational hygiene.

**Current Status**: **75% COMPLETE** via SPEC-945B (2025-11-14)
- ✅ Component 1: ACID transactions (DONE)
- ✅ Component 2: Auto-vacuum INCREMENTAL (DONE - 153MB→84KB, 99.95% reduction)  
- ❌ Component 3: Parallel agent spawning (REMAINING - 10-15h)
- ❌ Component 4: Daily cleanup cron (REMAINING - 8-12h)

**Dependencies**:
- Blocks: SPEC-934 (Storage Consolidation)
- Enables: SPEC-936 (Tmux Elimination)  

---

## Background

### Problem Statement (from SPEC-931A)

1. **Data Corruption Risk** (P0-CRITICAL): Dual-write pattern creates corruption window
2. **Database Bloat** (P1-HIGH): 153MB (99.5% empty space), auto_vacuum=NONE
3. **Performance Bottleneck** (P1-HIGH): Sequential spawning 3×150ms = 450ms overhead
4. **Operational Burden** (P2-MEDIUM): Manual cleanup, unbounded growth

### Research Findings

- SPEC-932: Consolidated 222→135 questions, generated 7 PRDs
- SPEC-945: 70+ pages research, 60+ authoritative sources
- SPEC-945B: Delivered Components 1-2 during migration (2025-11-14)

---

## Scope

### Remaining Work

**Component 3: Parallel Agent Spawning** (10-15h)
- Concurrent initialization via `tokio::spawn`
- Target: 150ms → 50ms (3× speedup)
- Spawn metrics instrumentation
- Concurrency safety verification

**Component 4: Daily Cleanup Cron** (8-12h)
- Background task for old consensus artifacts
- 90-day retention (archive >30d, purge >90d)
- Integration with evidence_stats.sh
- 50MB limit enforcement (SPEC-KIT-909)

### Already Complete (SPEC-945B)

**Component 1: ACID Transactions** ✅
- Implementation: transactions.rs
- Features: BEGIN/COMMIT/ROLLBACK, deadlock retry
- Impact: Eliminates dual-write corruption

**Component 2: Auto-Vacuum INCREMENTAL** ✅
- Implementation: vacuum.rs + connection.rs
- Result: 153MB→84KB (99.95% reduction)
- Features: Daily scheduler, space reclamation

### Out of Scope

- ❌ Event sourcing (YAGNI, saved 150-180h)
- ❌ Actor model refactoring (not problem-solving)
- ❌ Schema optimizations (2.3MB not worth 4-6h)
- ❌ MCP→SQLite migration (deferred to SPEC-934)

---

## Requirements

### FR1: Parallel Agent Spawning

**FR1.1**: Concurrent initialization via `tokio::spawn`
**FR1.2**: Spawn time ≤50ms (p95)
**FR1.3**: Spawn metrics (per-agent duration, total time, baseline comparison)
**FR1.4**: Concurrency safety (100 parallel cycles, 0 corruption)

### FR2: Daily Cleanup Cron

**FR2.1**: Daily scheduler (`tokio::time::interval` or cron)
**FR2.2**: Retention policy (archive >30d, purge >90d, exempt In Progress SPECs)
**FR2.3**: 50MB limit monitoring (warn + block automation)
**FR2.4**: Cleanup telemetry (files archived/purged, space reclaimed)

### Non-Functional Requirements

**NFR1**: Performance
- 3× spawn speedup (150ms→50ms), cleanup <5min

**NFR2**: Reliability  
- Zero corruption under concurrency, cleanup safety (never delete active work)

**NFR3**: Observability
- Spawn metrics (per-agent, total, success/failure rates)
- Cleanup metrics (daily summary, space reclaimed, errors)

**NFR4**: Maintainability
- Project patterns (Result<T, SpecKitError>), 40%+ test coverage

---

## Implementation Plan

### Phase 1: Component 3 - Parallel Spawning (10-15h)

**Step 1: Analysis & Design** (2h)
- Locate spawning code (consensus.rs / agent_orchestrator.rs)
- Identify sequential bottlenecks
- Design spawn_agents_parallel() function

**Step 2: Implementation** (4-6h)
```rust
use tokio::task::JoinSet;

pub async fn spawn_agents_parallel(agents: Vec<AgentConfig>) 
    -> Result<Vec<AgentHandle>, SpecKitError> 
{
    let mut join_set = JoinSet::new();
    
    for agent in agents {
        join_set.spawn(async move {
            let start = Instant::now();
            let handle = initialize_agent(agent).await?;
            let duration = start.elapsed();
            record_spawn_metric(agent.model, duration);
            Ok::<AgentHandle, SpecKitError>(handle)
        });
    }
    
    let mut handles = Vec::new();
    while let Some(result) = join_set.join_next().await {
        handles.push(result??);
    }
    
    Ok(handles)
}
```

**Step 3: Testing** (3-4h)
- Unit: test_parallel_spawn_faster, test_error_propagation, test_metrics_recorded
- Integration: test_no_corruption (100 cycles), test_stress (20+ agents)
- Benchmarks: Sequential vs parallel, document 3× improvement

**Step 4: Validation** (1-2h)
- Full test suite (604 tests), performance profiling, DB integrity check

**Deliverables**:
- spawn_agents_parallel() function
- Spawn metrics instrumentation
- 3+ tests (unit + integration)
- Performance benchmark results

### Phase 2: Component 4 - Cleanup Cron (8-12h)

**Step 1: Design** (2h)
- Retention policy: archive >30d, purge >180d
- Background tokio task or system cron
- Integration with evidence_stats.sh

**Step 2: Implementation** (4-6h)
```rust
pub async fn run_daily_cleanup() -> Result<CleanupSummary, SpecKitError> {
    let mut summary = CleanupSummary::default();
    
    let artifacts = find_old_artifacts(30_days)?;
    let purge_candidates = find_old_artifacts(180_days)?;
    
    for artifact in artifacts {
        if !is_in_progress(artifact.spec_id)? {
            archive_artifact(&artifact, &mut summary)?;
        }
    }
    
    for artifact in purge_candidates {
        if !is_in_progress(artifact.spec_id)? {
            purge_artifact(&artifact, &mut summary)?;
        }
    }
    
    check_evidence_limits(&mut summary)?;
    write_cleanup_summary(&summary)?;
    
    Ok(summary)
}
```

**Step 3: Testing** (2-3h)
- Unit: test_identify_old, test_in_progress_exemption, test_archive_creation
- Integration: test_cleanup_execution, test_retention_enforcement
- Dry-run: Validate on production data

**Step 4: Documentation** (1-2h)
- Cleanup schedule in evidence-policy.md
- Monitoring via /spec-evidence-stats
- Integration with TUI startup

**Deliverables**:
- evidence_cleanup.rs module
- TUI startup integration
- 4+ tests
- Policy documentation

### Phase 3: Integration & Validation (2-3h)

- Full test suite execution (604 + new tests)
- Performance validation (SPEC-940 framework)
- End-to-end testing (/speckit.auto with parallel spawning)
- Documentation updates (SPEC.md, CLAUDE.md)

---

## Acceptance Criteria

### Component 3: Parallel Spawning

| # | Criterion | Validation | Evidence |
|---|-----------|------------|----------|
| AC1 | Spawn time ≤50ms (p95) | Benchmarks (n≥100) | spawn_metrics.json |
| AC2 | 3× speedup vs sequential | Statistical (p<0.05) | parallel_vs_sequential.md |
| AC3 | Zero corruption | Integrity tests (100 cycles) | test_parallel_spawn_no_corruption |
| AC4 | Metrics instrumented | Telemetry validation | evidence/commands/<SPEC-ID>/ |
| AC5 | All tests passing | Test suite | cargo test --workspace |

### Component 4: Cleanup Cron

| # | Criterion | Validation | Evidence |
|---|-----------|------------|----------|
| AC6 | Daily execution | Scheduler logs (7d) | evidence/cleanup/daily-summaries/ |
| AC7 | Retention policy enforced | Dry-run validation | Cleanup summary |
| AC8 | 50MB limit enforced | Evidence stats | /spec-evidence-stats output |
| AC9 | In Progress exempt | Test validation | test_in_progress_exemption |
| AC10 | Cleanup <5min | Performance measurement | Cleanup telemetry |

---

## Risks & Mitigations

**Risk 1: Parallel Spawning Corruption** (HIGH)
- Mitigation: ACID transactions foundation ✅, comprehensive testing, Tokio patterns
- Contingency: Revert to sequential + investigate

**Risk 2: Cleanup Deletes Critical Data** (MEDIUM)
- Mitigation: In Progress exemption, archive before purge, dry-run testing
- Contingency: Archive recovery mechanism

**Risk 3: Performance Target Not Met** (LOW)
- Mitigation: Baseline measurements, profiling, SPEC-940 validation
- Contingency: Adjust target, optimize initialization

**Risk 4: Cleanup Interferes** (LOW)
- Mitigation: Background task, CPU limits, low-usage schedule
- Contingency: Make optional, manual cleanup only

---

## Dependencies

**Upstream (Blocks SPEC-933)**:
- ✅ SPEC-945B: COMPLETE - delivers Components 1-2

**Downstream (Blocked by SPEC-933)**:
- SPEC-934: Requires Component 1 (✅ complete, UNBLOCKED)
- SPEC-936: Benefits from Component 3 (not blocking)

**Lateral**:
- SPEC-940: Validates Component 3 performance
- SPEC-KIT-909: Defines Component 4 policies (✅ complete)

---

## Success Metrics

### Quantitative

1. Database size: 153MB→84KB ✅ ACHIEVED (Component 2)
2. Spawn time: 150ms→≤50ms ⏳ TARGET (Component 3)
3. Cleanup execution: ≤5min ⏳ TARGET (Component 4)
4. Evidence compliance: 100% <50MB ⏳ TARGET (Component 4)
5. Zero corruption: 0 violations ⏳ TARGET (Component 3)

### Qualitative

1. Data integrity: ACID compliance ✅ ACHIEVED (Component 1)
2. Operational hygiene: Automated cleanup ⏳ TARGET (Component 4)
3. Developer experience: Faster startup ⏳ TARGET (Component 3)
4. Maintainability: Well-tested, documented ⏳ IN PROGRESS

---

## Timeline

### Original Estimate (SPEC-932, 2025-11-13)
- Total: 65-96h (2-3 weeks)
- Component 1: 25-35h, Component 2: 20-28h
- Component 3: 10-15h, Component 4: 10-18h

### Revised Estimate (Post-SPEC-945B, 2025-11-14)
- Total: 18-27h (2-3 days)
- Components 1-2: ✅ 0h (COMPLETE)
- Component 3: ⏳ 10-15h (REMAINING)
- Component 4: ⏳ 8-12h (REMAINING)

### Recommended Sequence
1. Component 3 First (parallel spawning): Higher impact, enables SPEC-936
2. Component 4 Second (cleanup cron): Operational hygiene, lower risk

---

## References

### Research Documents
- SPEC-931A: Component architecture analysis
- SPEC-932: Implementation planning (222→135 questions)
- SPEC-945: Implementation research (70+ pages)
- SPEC-945B: SQLite optimization (Components 1-2)

### Related SPECs
- SPEC-945B (Complete): Delivers Components 1-2
- SPEC-934 (Blocked): Storage consolidation
- SPEC-936 (Related): Tmux elimination
- SPEC-940 (Related): Performance instrumentation
- SPEC-909 (Complete): Evidence lifecycle

### Memory References
- 1afe940e-8653-4e46-81aa-00ba4b6ad242: SPEC-945B success
- 15de14ca-0d9c-465d-8b6d-ea725d7e6770: SPEC-932 planning
- 688b1d40-3674-4d9d-a97b-a42d0c639256: SPEC-945 research

---

**END OF PRD**
