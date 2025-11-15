# SPEC-933 Component 3: Parallel Agent Spawning - Session Start

**Previous Session**: 2025-11-14 - SPEC-933 PRD Creation Complete
**Current Branch**: main (all feature branches merged and pushed)
**Next Work**: SPEC-933 Component 3 - Parallel Agent Spawning
**Memory ID**: 4f566d03-94f9-4cc2-8c43-62f810d01d55

---

## Quick Context

**SPEC-933 Status**: 75% Complete (2 of 4 components done via SPEC-945B)
- ✅ Component 1: ACID transactions (transactions.rs)
- ✅ Component 2: Auto-vacuum INCREMENTAL (153MB→84KB, 99.95% reduction)
- ❌ Component 3: Parallel agent spawning (10-15h) ← **THIS SESSION**
- ❌ Component 4: Daily cleanup cron (8-12h) ← NEXT SESSION

**Priority**: P0-CRITICAL (Data Integrity + Performance)
**Blocks**: SPEC-934 (Storage Consolidation) - now unblocked
**Enables**: SPEC-936 (Tmux Elimination) - benefits from parallel spawning

---

## Session Verification (Run First)

**Step 1: Verify PRD exists** (5 seconds)
```bash
cat docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md | head -20
```
Expected: PRD header with "SPEC-KIT-933: Database Integrity & Hygiene"

**Step 2: Load memory context** (30 seconds)
```
Use mcp__local-memory__get_memory_by_id:
- id: 4f566d03-94f9-4cc2-8c43-62f810d01d55
```

**Step 3: Check current branch** (5 seconds)
```bash
git branch --show-current && git status --short
```
Expected: On `main`, clean tree (or only untracked session files)

---

## Work Plan: Component 3 - Parallel Agent Spawning

### Goal
Implement concurrent agent initialization to reduce spawn time from 150ms → 50ms (3× speedup).

### Success Criteria
- ✅ Spawn time ≤50ms (p95) over 100 spawn cycles
- ✅ 3× speedup demonstrated with statistical validation
- ✅ Zero data corruption under concurrent spawning
- ✅ Spawn metrics instrumented and exportable
- ✅ All tests passing (604 baseline + new tests)

### Implementation Steps (10-15 hours)

#### Phase 1: Analysis & Design (2h)

**Tasks**:
1. Locate current agent spawning code
   ```bash
   cd /home/thetu/code/codex-rs
   rg "spawn.*agent|initialize.*agent" spec-kit/src/ -t rust -C 3
   ```
   Likely files: `consensus.rs`, `agent_orchestrator.rs`, or similar

2. Identify sequential bottlenecks
   - Look for synchronous loops: `for agent in agents { spawn_sync(agent); }`
   - Measure current baseline spawn time (if not already known)

3. Design `spawn_agents_parallel()` function
   - Signature: `async fn spawn_agents_parallel(agents: Vec<AgentConfig>) -> Result<Vec<AgentHandle>, SpecKitError>`
   - Use `tokio::task::JoinSet` for structured concurrency
   - Plan error propagation strategy

**Deliverables**:
- Analysis document: `docs/SPEC-KIT-933-database-integrity-hygiene/analysis.md`
- List of files to modify
- Function signature and architecture sketch

#### Phase 2: Implementation (4-6h)

**File**: Likely `spec-kit/src/consensus/mod.rs` or new `parallel_spawn.rs`

**Core Function** (reference from PRD):
```rust
use tokio::task::JoinSet;
use std::time::Instant;

pub async fn spawn_agents_parallel(
    agents: Vec<AgentConfig>
) -> Result<Vec<AgentHandle>, SpecKitError> {
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

**Metrics Integration**:
- Add `record_spawn_metric(model: String, duration: Duration)` helper
- Export to telemetry JSON: `evidence/commands/<SPEC-ID>/spawn_metrics.json`
- Include: per-agent duration, total parallel time, sequential baseline comparison

**Tasks**:
1. Create `spawn_agents_parallel()` function
2. Integrate with existing orchestration (minimal disruption)
3. Add spawn time metrics collection
4. Update calling code to use parallel spawning

**Deliverables**:
- Implementation code (~200-300 LOC)
- Metrics instrumentation
- Integration with existing consensus flow

#### Phase 3: Testing (3-4h)

**Unit Tests** (new file: `spec-kit/src/consensus/parallel_spawn_tests.rs`):
```rust
#[tokio::test]
async fn test_parallel_spawn_faster_than_sequential() {
    // Spawn 3 agents in parallel, measure time
    // Compare against sequential baseline
    // Assert: parallel_time < sequential_time * 0.5 (>50% improvement)
}

#[tokio::test]
async fn test_parallel_spawn_error_propagation() {
    // Include 1 failing agent in 3-agent spawn
    // Assert: All spawns cancelled, error propagated correctly
}

#[tokio::test]
async fn test_spawn_metrics_recorded() {
    // Spawn agents, verify metrics JSON created
    // Assert: Metrics include per-agent duration, total time
}
```

**Integration Tests** (add to `tui/tests/`):
```rust
#[tokio::test]
async fn test_parallel_spawn_no_corruption() {
    // Run 100 concurrent spawn cycles
    // Execute consensus after each cycle
    // Verify database integrity after all cycles
}

#[tokio::test]
async fn test_parallel_spawn_stress() {
    // Spawn 20+ agents concurrently
    // Verify: No deadlocks, all complete successfully
}
```

**Performance Benchmarks**:
- Baseline: Measure current sequential spawn time (if not 150ms, document actual)
- Parallel: Measure parallel spawn time (3, 5, 10 agents)
- Statistical validation: n≥100 runs, calculate mean, stddev, p95
- Target: p95 ≤50ms

**Tasks**:
1. Write 3+ unit tests
2. Write 2+ integration tests
3. Run performance benchmarks (document results)
4. Verify all 604 existing tests still pass

**Deliverables**:
- Test suite: 5+ new tests
- Benchmark results: `docs/SPEC-KIT-933-database-integrity-hygiene/benchmarks.md`
- Evidence of 3× improvement

#### Phase 4: Validation (1-2h)

**Full Test Suite**:
```bash
cd /home/thetu/code/codex-rs
cargo test --workspace
```
Expected: 100% pass rate (604 baseline + new tests)

**Performance Profiling**:
```bash
cargo test --release -- --nocapture benchmark_parallel_spawn
```
Verify: p95 spawn time ≤50ms

**Database Integrity Check**:
```bash
sqlite3 ~/.code/consensus_artifacts.db "PRAGMA integrity_check;"
```
Expected: ok

**Code Quality**:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
```
Expected: 0 errors, 0 warnings (or document pre-existing)

**Deliverables**:
- Test results summary
- Performance validation report
- DB integrity confirmation
- Clean build output

#### Phase 5: Documentation & Wrap-Up (1h)

**Update SPEC.md**:
- Mark SPEC-933 Component 3 as DONE
- Update status from "75% complete" → "87.5% complete"

**Store in local-memory**:
```
Use mcp__local-memory__store_memory:
- content: "SPEC-933 Component 3 Complete: Parallel agent spawning implemented. Achieved 150ms→XXms spawn time (X× speedup, p95). Implementation: spawn_agents_parallel() in [file]. Tests: X unit + X integration (100% pass). Benchmarks: [results]. Zero corruption over 100 concurrent cycles. Pattern: tokio::JoinSet for structured concurrency. Files: [list]. Next: Component 4 cleanup cron (8-12h)."
- domain: "spec-kit"
- tags: ["type:milestone", "spec:SPEC-933", "component-3", "performance"]
- importance: 9
```

**Create next session handoff**:
- Summary of Component 3 completion
- Handoff prompt for Component 4 (cleanup cron)
- Status update for implementation backlog

**Deliverables**:
- SPEC.md updated
- Local-memory stored
- Next session prompt created

---

## Expected Outcomes

**At Session End**:
- ✅ `spawn_agents_parallel()` function operational
- ✅ Spawn time ≤50ms (p95) demonstrated
- ✅ 3× speedup validated with benchmarks
- ✅ Zero corruption under concurrency
- ✅ 5+ new tests passing (100% pass rate maintained)
- ✅ Spawn metrics exported to telemetry
- ✅ SPEC.md status updated
- ✅ Local-memory stored
- ✅ Next session handoff ready

**Next Session** (Component 4):
- Daily cleanup cron implementation
- 8-12 hours estimated
- Lower priority, operational hygiene
- Completes SPEC-933 (100%)

---

## Quick Reference

**PRD Location**: `docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md`
**Workspace Root**: `/home/thetu/code/codex-rs`
**Database**: `~/.code/consensus_artifacts.db` (84KB, WAL mode)
**Test Command**: `cargo test --workspace`
**Evidence Root**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

**Related SPECs**:
- SPEC-945B (Complete): Delivered Components 1-2
- SPEC-934 (Blocked → Unblocked): Requires Component 1 (done)
- SPEC-936 (Related): Benefits from Component 3
- SPEC-940 (Related): Performance validation framework

**Memory References**:
- Previous session: 4f566d03-94f9-4cc2-8c43-62f810d01d55 (PRD creation)
- SPEC-945B completion: 1afe940e-8653-4e46-81aa-00ba4b6ad242
- SPEC-932 planning: 15de14ca-0d9c-465d-8b6d-ea725d7e6770

---

## Copy-Paste to Begin

```
SPEC-933 Component 3: Parallel Agent Spawning

Previous Work:
- SPEC-933 PRD created (memory: 4f566d03-94f9-4cc2-8c43-62f810d01d55)
- Components 1-2 complete via SPEC-945B (ACID transactions, auto-vacuum)
- Current branch: main (clean, all pushed)

Target This Session:
- Component 3: Parallel agent spawning (150ms→50ms, 3× speedup)
- Effort: 10-15 hours
- Priority: P0-CRITICAL
- Blocks: None (SPEC-934 already unblocked by Component 1)

Verify Before Starting:
1. PRD exists: cat docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md | head -20
2. Load memory: mcp__local-memory__get_memory_by_id with id "4f566d03-94f9-4cc2-8c43-62f810d01d55"
3. Check branch: git branch --show-current (expect: main)

Work Plan (5 phases, 10-15h):
1. Analysis & Design (2h): Locate spawning code, identify bottlenecks, design function
2. Implementation (4-6h): Create spawn_agents_parallel() using tokio::JoinSet, add metrics
3. Testing (3-4h): Unit tests (3+), integration tests (2+), benchmarks (validate 3× speedup)
4. Validation (1-2h): Full test suite, performance profiling, DB integrity check
5. Documentation (1h): Update SPEC.md, store memory, create next handoff

Success Criteria:
- Spawn time ≤50ms (p95) over 100 cycles
- 3× speedup demonstrated (statistical validation)
- Zero corruption under concurrency
- All tests passing (604 + new)
- Metrics instrumented

Please:
1. Verify PRD and load memory context
2. Locate current agent spawning code (rg "spawn.*agent" in spec-kit/src/)
3. Analyze sequential bottlenecks and baseline spawn time
4. Design spawn_agents_parallel() function architecture
5. Implement parallel spawning with tokio::JoinSet
6. Add spawn time metrics (per-agent, total, baseline comparison)
7. Write tests (3 unit, 2 integration, benchmarks)
8. Validate: full test suite, performance profiling, DB integrity
9. Update SPEC.md status (75%→87.5% complete)
10. Store completion in local-memory (importance: 9)
11. Provide next session handoff for Component 4

Estimated: 10-15 hours
Priority: P0-CRITICAL
Next: Component 4 (cleanup cron, 8-12h) to complete SPEC-933
```

---

**END OF SESSION HANDOFF PROMPT**
