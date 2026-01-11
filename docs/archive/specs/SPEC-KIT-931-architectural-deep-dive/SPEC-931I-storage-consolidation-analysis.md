# SPEC-931I: Storage Consolidation Feasibility Analysis

**Date**: 2025-11-13
**Analyst**: Claude (Ultrathink Mode)
**Parent**: SPEC-931 Architectural Deep Dive (9/10 complete)
**Prior Work**: SPEC-931A-H (8 analyses complete, 2 NO-GO decisions)

**Status**: ✅ **ANALYSIS COMPLETE** - Ready for implementation decision

---

## Executive Summary

**DECISION**: ✅ **GO - Storage consolidation from 4→2 systems is FEASIBLE and RECOMMENDED**

**Current State** (4 storage systems):
1. **AGENT_MANAGER HashMap** (in-memory coordination)
2. **SQLite consensus_db** (persistent artifacts - PARTIALLY used)
3. **Filesystem result.txt** (legacy orchestrator fallback)
4. **MCP local-memory** (knowledge + **POLICY VIOLATION**: consensus artifacts)

**Target State** (2 systems):
1. **AGENT_MANAGER HashMap** (keep - TUI requirement)
2. **SQLite consensus_db** (expand - eliminate MCP for consensus)

**Key Findings**:
- **SPEC-KIT-072 Policy Violation CONFIRMED**: Quality gates store consensus to MCP (5× slower, violates policy)
- **Partial Implementation**: Single-agent stages use SQLite ✅, quality gates use MCP ❌
- **AGENT_MANAGER Non-Eliminable**: Required for TUI rendering performance (SPEC-931F/H validated)
- **Filesystem Eliminable**: Native orchestrator doesn't need it (legacy fallback only)
- **Migration Effort**: 5-8 hours (low complexity, high value)

**Cross-References**:
- SPEC-931A Q70: Which storage systems are necessary? (**NOW ANSWERED**)
- SPEC-931B D1: Move MCP artifacts to SQLite (**APPROVED but NOT IMPLEMENTED**)
- SPEC-931F: Event sourcing doesn't eliminate AGENT_MANAGER (TUI needs sync cache)
- SPEC-931H: Actor model doesn't eliminate AGENT_MANAGER (same issue)

---

## Current Architecture Analysis

### Storage System Inventory

**1. AGENT_MANAGER HashMap** (`codex-rs/core/src/agent_tool.rs:62-64`)
```rust
lazy_static! {
    pub static ref AGENT_MANAGER: Arc<RwLock<AgentManager>> = ...
}
```
- **Purpose**: In-memory agent coordination, TUI real-time updates
- **Data**: agents HashMap<String, Agent>, handles HashMap<String, JoinHandle>
- **Accessed By**: TUI rendering (60 FPS), native_quality_gate_orchestrator, quality_gate_broker
- **Verdict**: ✅ **KEEP** (TUI performance requirement, SPEC-931F/H confirmed non-eliminable)

**2. SQLite consensus_db** (`~/.code/consensus_artifacts.db`)
- **Tables**:
  - `consensus_artifacts`: Agent outputs (content_json, response_text, run_id)
  - `consensus_synthesis`: Final consensus results (output_markdown, status, conflicts)
  - `agent_executions`: Execution tracking (spawned_at, completed_at, extraction_error)
- **Used By**:
  - ✅ agent_orchestrator.rs:1513 (single-agent stages: plan, tasks, implement)
  - ❌ quality_gate_handler.rs (quality gates use MCP instead)
  - ✅ native_quality_gate_orchestrator.rs (record_agent_spawn, record_agent_completion)
- **Verdict**: ✅ **KEEP + EXPAND** (fix quality gate policy violation)

**3. Filesystem result.txt** (`.code/agents/{agent_id}/result.txt`)
- **Written By**: CLI agents (gemini, claude, code - external to our code)
- **Read By**: quality_gate_handler.rs:1586 (legacy orchestrator fallback)
- **Usage Pattern**:
  - Native path (PRIMARY): AGENT_MANAGER → broker reads memory
  - Legacy path (FALLBACK): result.txt → broker scans filesystem
- **Code**: quality_gate_handler.rs:122-137 has dual-path branching
- **Verdict**: ⏳ **DEPRECATE** (remove legacy fallback, native path sufficient)

**4. MCP local-memory** (via mcp-server-local-memory)
- **Policy** (SPEC-KIT-072): Consensus → SQLite, Knowledge → MCP
- **Reality** (VIOLATION): quality_gate_handler.rs:1627-1638 stores consensus to MCP
- **Evidence**: quality_gate_handler.rs:1747-1790 `store_artifact_async()` calls MCP `store_memory`
- **Impact**: 5× slower (150ms MCP vs 30ms SQLite, per SPEC-931B D1)
- **Verdict**: ⏳ **DEMOTE** (keep for knowledge, eliminate consensus artifacts)

---

## Critical Discoveries

### 1. SPEC-KIT-072 Policy Violation (ACTIVE)

**Finding**: Quality gate consensus artifacts stored to MCP, violating policy separation.

**Evidence**:
```rust
// quality_gate_handler.rs:1747-1790
async fn store_artifact_async(
    mcp_manager: Arc<Mutex<Option<Arc<McpConnectionManager>>>>,
    ...
) -> Result<(), String> {
    ...
    manager.call_tool("local-memory", "store_memory", Some(args), ...).await
}
```

**Impact**:
- **Performance**: 5× slower (150ms MCP vs 30ms SQLite)
- **Policy**: Violates SPEC-KIT-072 separation (consensus should be SQLite, not MCP)
- **Consistency**: Single-agent stages use SQLite ✅, quality gates use MCP ❌

**Root Cause**: SPEC-931B D1 approved SQLite migration but quality_gate_handler.rs was never updated.

**Proof**: agent_orchestrator.rs:1501 has comment `// SPEC-KIT-072: Store to SQLite`, but quality_gate_handler.rs has no such comment or implementation.

---

### 2. Partial Implementation (Inconsistent Policy Application)

**Finding**: SQLite usage varies by orchestrator - single-agent stages compliant, quality gates non-compliant.

**Code Paths**:

**Single-Agent Stages** (agent_orchestrator.rs:1501-1538):
```rust
// SPEC-KIT-072: Store to SQLite for persistent consensus artifacts
if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
    for (agent_name, response_text) in &agent_results {
        if let Err(e) = db.store_artifact(...) { // ✅ SQLite
            tracing::warn!("Failed to store {} artifact: {}", agent_name, e);
        }
    }
}
```

**Quality Gates** (quality_gate_handler.rs:1627-1638):
```rust
let handle = tokio::spawn(async move {
    store_artifact_async(  // ❌ MCP
        mcp_clone, ...
    ).await
});
```

**Hypothesis**: Implementation order was:
1. consensus_db.rs created (schema + methods)
2. agent_orchestrator.rs migrated to SQLite
3. quality_gate_handler.rs **LEFT BEHIND** (oversight or incomplete rollout)

---

### 3. Dual Orchestrator Architecture

**Finding**: Native orchestrator is PRIMARY, filesystem is legacy FALLBACK only.

**Architecture**:
```rust
// quality_gate_handler.rs:122-137
// Use memory-based collection for native orchestrator, filesystem for legacy
if let Some(agent_ids) = native_agent_ids {
    widget.quality_gate_broker.fetch_agent_payloads_from_memory(  // Native path
        spec_id, checkpoint, expected_agents, agent_ids
    );
} else {
    widget.quality_gate_broker.fetch_agent_payloads(  // Legacy path (filesystem)
        spec_id, checkpoint, expected_agents, gate_names
    );
}
```

**Flow** (Native Path - PRIMARY):
1. quality_gate_handler.rs:1145 spawns via `native_quality_gate_orchestrator::spawn_quality_gate_agents_native()`
2. Agents populate AGENT_MANAGER HashMap
3. Event `QualityGateNativeAgentsComplete` sent with agent_ids (line 1188)
4. app.rs:2818 sets agent_ids in phase
5. Handler checks agent_ids present → memory collection (reads AGENT_MANAGER)

**Flow** (Legacy Path - FALLBACK):
1. LLM orchestrator spawns CLI agents
2. Agents write result.txt files (external)
3. Handler scans `.code/agents/*/result.txt` (filesystem)

**Current Usage**: Native path always used, legacy path is backward compatibility only.

---

### 4. AGENT_MANAGER Non-Eliminable

**Finding**: AGENT_MANAGER HashMap is REQUIRED for product functionality, cannot be eliminated.

**Cross-Validation**:
- **SPEC-931F** (Event Sourcing): Concluded event sourcing doesn't eliminate AGENT_MANAGER (TUI needs sync cache)
- **SPEC-931H** (Actor Model): Concluded actors don't eliminate AGENT_MANAGER (same issue - TUI needs sync reads)

**Reason**: TUI rendering requires synchronous reads at 60 FPS. Async message passing (event log replay, actor messages) is too slow.

**Pattern**:
- **Source of Truth**: Varies (SQLite, event log, actor state)
- **TUI Cache**: AGENT_MANAGER serves as denormalized read cache
- **Implication**: Dual-write problem persists unless ACID transactions wrap both updates

---

### 5. Data Redundancy Analysis

**Redundancy Mapping** (based on code analysis):

| Storage System | Data Stored | Read By | Write By | Redundancy |
|----------------|-------------|---------|----------|-----------|
| **AGENT_MANAGER.result** | In-memory string | TUI, quality_gate_broker (native) | execute_agent() | PRIMARY READ SOURCE |
| **SQLite consensus_artifacts** | content_json, response_text | (unused by quality gates) | agent_orchestrator.rs ONLY | PARTIAL (single-agent only) |
| **SQLite agent_executions.response_text** | Execution tracking | Debugging, extraction failures | native orchestrator | NON-REDUNDANT (tracking) |
| **Filesystem result.txt** | CLI agent output | quality_gate_broker (legacy) | CLI agents (external) | LEGACY FALLBACK |
| **MCP local-memory** | Consensus artifacts (VIOLATION) | quality_gate_broker (validation) | quality_gate_handler.rs | POLICY VIOLATION |

**Key Findings**:
- **SQLite consensus_artifacts table**: Schema exists but only written by single-agent stages, NOT quality gates
- **No True Redundancy**: Each system serves different code path (native vs legacy vs single-agent)
- **Potential Elimination**: Filesystem (deprecate legacy), MCP consensus (migrate to SQLite)

---

## Target Architecture (4→2 Systems)

### Recommended Target

**Systems to KEEP**:
1. **AGENT_MANAGER HashMap** (in-memory coordination)
   - **Why**: TUI rendering performance (60 FPS synchronous reads)
   - **Validated By**: SPEC-931F, SPEC-931H (non-eliminable)
   - **Role**: Real-time coordination, denormalized TUI cache

2. **SQLite consensus_db** (persistent artifacts)
   - **Why**: ACID transactions, fast queries, proper lifecycle
   - **Expand Usage**: Migrate quality gates from MCP → SQLite
   - **Role**: Single source of truth for persistent artifacts

**Systems to REMOVE**:
3. **Filesystem result.txt** (legacy orchestrator)
   - **Why**: Native orchestrator doesn't need it (reads AGENT_MANAGER directly)
   - **Removal Strategy**: Deprecate legacy fallback code path
   - **Risk**: Low (native path is already primary)

4. **MCP local-memory** (consensus artifacts only)
   - **Why**: Policy violation (consensus should be SQLite)
   - **Keep For**: Knowledge persistence (SPEC-KIT-072 compliant usage)
   - **Removal Strategy**: Migrate quality gates to SQLite, keep MCP for knowledge only

### Architecture Diagram

**BEFORE** (Current - 4 Systems):
```
┌─────────────────────────────────────────────────────────┐
│                    TUI (60 FPS Rendering)                │
└──────────────────────┬──────────────────────────────────┘
                       │ sync reads
                       ↓
              ┌────────────────┐
              │ AGENT_MANAGER  │ (System 1: In-Memory)
              │    HashMap     │
              └────────┬───────┘
                       │
      ┌────────────────┼────────────────┐
      │                │                │
      ↓                ↓                ↓
┌──────────┐   ┌─────────────┐   ┌──────────┐
│ SQLite   │   │ Filesystem  │   │   MCP    │
│consensus │   │ result.txt  │   │local-mem │
│   _db    │   │  (legacy)   │   │(VIOLATION│
└──────────┘   └─────────────┘   └──────────┘
(System 2)      (System 3)        (System 4)

USAGE:
- Single-agent: SQLite ✅
- Quality gates: MCP ❌ (should be SQLite)
- Legacy: Filesystem (fallback)
```

**AFTER** (Target - 2 Systems):
```
┌─────────────────────────────────────────────────────────┐
│                    TUI (60 FPS Rendering)                │
└──────────────────────┬──────────────────────────────────┘
                       │ sync reads
                       ↓
              ┌────────────────┐
              │ AGENT_MANAGER  │ (System 1: In-Memory Cache)
              │    HashMap     │
              └────────┬───────┘
                       │ async writes
                       │ (dual-write with ACID)
                       ↓
              ┌────────────────┐
              │ SQLite         │ (System 2: Source of Truth)
              │ consensus_db   │
              │ - artifacts    │
              │ - synthesis    │
              │ - executions   │
              └────────────────┘

ELIMINATED:
- Filesystem: Native orchestrator only (no legacy fallback)
- MCP consensus: Kept for knowledge, removed for consensus
```

---

## Migration Strategy

### Phase 1: Fix SPEC-KIT-072 Violation (2-3 hours)

**Goal**: Migrate quality gates from MCP → SQLite

**Changes Required**:

**1. Replace MCP store with SQLite** (quality_gate_handler.rs):
```rust
// BEFORE (lines 1617-1638):
let handle = tokio::spawn(async move {
    store_artifact_async(mcp_clone, ...).await  // ❌ MCP
});

// AFTER:
let db = super::consensus_db::ConsensusDb::init_default()?;
db.store_artifact(  // ✅ SQLite
    &spec_id, checkpoint.to_stage(), &agent_name,
    &json_str, Some(response_text), Some(&run_id)
)?;
```

**2. Remove store_artifact_async()** (lines 1744-1790):
- Delete entire async function (no longer needed)
- Remove MCP dependency from quality_gate_handler.rs

**3. Update broker to query SQLite** (if needed):
- Verify quality_gate_broker.rs doesn't search MCP for quality gate artifacts
- If it does, update to query consensus_db instead

**Complexity**: LOW (schema exists, method signatures compatible)
**Risk**: MEDIUM (breaking quality gates during migration)
**Mitigation**: Parallel write period (write to both MCP + SQLite for 1-2 weeks, verify SQLite, cutover)

---

### Phase 2: Remove Legacy Filesystem Fallback (1-2 hours)

**Goal**: Deprecate legacy orchestrator code path

**Changes Required**:

**1. Remove legacy path** (quality_gate_handler.rs:130-137):
```rust
// BEFORE:
if let Some(agent_ids) = native_agent_ids {
    widget.quality_gate_broker.fetch_agent_payloads_from_memory(...);
} else {
    widget.quality_gate_broker.fetch_agent_payloads(...);  // ❌ Remove this
}

// AFTER:
let agent_ids = native_agent_ids.expect("Native orchestrator required");
widget.quality_gate_broker.fetch_agent_payloads_from_memory(...);
```

**2. Remove filesystem scanning** (quality_gate_broker.rs:405-573):
- Delete `fetch_agent_payloads_from_filesystem()` function
- Delete `fetch_agent_payloads()` wrapper (calls filesystem scan)

**3. Clean up old orchestrator code** (quality_gate_handler.rs:1541-1667):
- Delete `store_quality_gate_artifacts()` function (reads result.txt files)
- Delete `get_completed_agents()` function (scans .code/agents/ directory)

**Complexity**: LOW (well-isolated code)
**Risk**: LOW (native path is already primary)
**Mitigation**: Feature flag to enable legacy path if needed (can be removed after 1 release)

---

### Phase 3: Data Migration (Optional, 1 hour)

**Goal**: Migrate historical consensus artifacts from MCP → SQLite

**Options**:

**Option A**: Accept data loss (RECOMMENDED)
- Historical artifacts in MCP are transient workflow data (not curated knowledge)
- No product feature depends on historical consensus artifacts
- Effort: 0 hours

**Option B**: Migrate historical data
- Extract from local-memory (CLI/REST): `local-memory search "*" --domain spec-kit --tags "quality-gate" --json`
- Transform: Parse JSON, map to consensus_artifacts schema
- Load: Bulk insert to SQLite
- Effort: 1-2 hours (scripting + validation)

**Recommendation**: Option A (accept data loss). Consensus artifacts are transient, no product value in migration.

---

## Migration Complexity Summary

| Phase | Description | Effort | Risk | Priority |
|-------|-------------|--------|------|----------|
| **Phase 1** | Fix SPEC-KIT-072 (MCP → SQLite) | 2-3 hours | MEDIUM | P0 (policy violation) |
| **Phase 2** | Remove legacy filesystem fallback | 1-2 hours | LOW | P1 (cleanup) |
| **Phase 3** | Data migration (optional) | 1 hour | LOW | P2 (optional) |
| **Testing** | Integration + regression tests | 2-3 hours | - | - |
| **TOTAL** | Implementation + testing | **5-8 hours** | **MEDIUM** | **HIGH VALUE** |

**Parallel Write Strategy** (Risk Mitigation):
1. **Week 1**: Deploy Phase 1 with dual write (MCP + SQLite)
2. **Week 2**: Monitor SQLite writes, verify data integrity
3. **Week 3**: Cut over (remove MCP writes), monitor for regressions
4. **Week 4**: Deploy Phase 2 (filesystem cleanup)

**Rollback Plan**:
- Keep MCP storage code in commented-out form for 1 release
- Feature flag to re-enable MCP storage if SQLite issues found
- Rollback window: 2 weeks (before Phase 2 cleanup)

---

## Risks & Mitigation

### Risk 1: Breaking Quality Gates During Migration

**Probability**: MEDIUM
**Impact**: HIGH (quality gates are critical path for /speckit.auto)
**Mitigation**:
- Parallel write period (MCP + SQLite) for 1-2 weeks
- Integration tests for quality gate artifact storage/retrieval
- Gradual rollout: Canary deployment (10% users for 1 week)

---

### Risk 2: Performance Regression (SQLite vs MCP)

**Probability**: LOW
**Impact**: LOW (SQLite is 5× FASTER than MCP, not slower)
**Mitigation**:
- Benchmark before/after: Measure quality gate latency (spawn → consensus)
- Monitor: Add telemetry for artifact storage duration
- Expected: ~120ms improvement per quality gate (150ms MCP → 30ms SQLite)

---

### Risk 3: Data Loss During Migration

**Probability**: LOW (only if Option B chosen)
**Impact**: LOW (consensus artifacts are transient)
**Mitigation**:
- Recommend Option A (accept data loss) - no migration needed
- If Option B: Test migration script on dev environment first
- Backup: Export MCP data before cutover (JSON dump for recovery)

---

### Risk 4: Incomplete Migration (Missed Code Paths)

**Probability**: MEDIUM
**Impact**: MEDIUM (MCP still being called from unexpected locations)
**Mitigation**:
- Grep audit: Search for `store_memory` calls in all files
- Static analysis: Add lint rule to detect MCP consensus artifact writes
- Testing: Verify no MCP calls during quality gate execution

---

## Decision Framework

### Should We Consolidate Storage?

**YES - Consolidation is RECOMMENDED**

**Reasons**:
1. ✅ **Policy Compliance**: Fixes SPEC-KIT-072 violation (consensus → SQLite)
2. ✅ **Performance**: 5× faster (150ms MCP → 30ms SQLite)
3. ✅ **Simplicity**: Reduces from 4 systems to 2 (less cognitive overhead)
4. ✅ **Low Risk**: Native orchestrator already primary, legacy is fallback only
5. ✅ **Feasible Effort**: 5-8 hours (low complexity, high value)
6. ✅ **Validated Approach**: SPEC-931B D1 already approved this migration

**Contraindications**: None identified

---

## Open Questions

**Unanswered Questions** (5/13 = 38%):
1. **Q193**: Detailed migration plan with rollback strategy (**ADDRESSED in this report**)
2. **Q194**: Root cause analysis - Why wasn't quality_gate_handler.rs migrated? (Hypothesis: oversight)
3. **Q196**: Exact location of MCP storage in broker result handling flow (Still needed for Phase 1)
5. **Runtime Measurements**: Actual redundancy quantification, storage footprint analysis (Optional)

**Next Steps to Resolve**:
- Q196: Trace broker result handling → find where MCP store is triggered
- Q194: Git blame analysis on quality_gate_handler.rs (historical investigation)
- Runtime metrics: Add telemetry to quantify actual storage usage (optional)

---

## Recommendations

### PRIMARY RECOMMENDATION: ✅ **GO - Implement Storage Consolidation**

**Implement**:
1. **Phase 1** (P0): Fix SPEC-KIT-072 violation (MCP → SQLite) - 2-3 hours
2. **Phase 2** (P1): Remove legacy filesystem fallback - 1-2 hours
3. **Phase 3** (P2): Skip data migration (accept loss) - 0 hours

**Total Effort**: 3-5 hours implementation + 2-3 hours testing = **5-8 hours**

**Expected Benefits**:
- ✅ Policy compliance (SPEC-KIT-072)
- ✅ 5× faster artifact storage (150ms → 30ms)
- ✅ Reduced cognitive complexity (4 → 2 systems)
- ✅ Eliminates 2 storage systems (filesystem, MCP consensus)

**Timeline**: 2-3 weeks (including parallel write period)

---

### SECONDARY RECOMMENDATION: Add ACID Transactions (Future Work)

**Context**: SPEC-931A Q1 identified dual-write problem (AGENT_MANAGER + SQLite).

**Solution** (deferred to future SPEC):
- Wrap HashMap + SQLite updates in ACID transaction
- Use Write-Ahead Log (WAL) for crash recovery
- Implement 2-phase commit pattern

**Rationale**: Storage consolidation (this SPEC) is prerequisite for ACID transactions (future SPEC).

---

## Conclusion

Storage consolidation from 4→2 systems is **FEASIBLE, LOW-RISK, and HIGH-VALUE**.

**Target Architecture**:
1. **AGENT_MANAGER HashMap** (keep - TUI requirement)
2. **SQLite consensus_db** (expand - eliminate MCP violation)

**Elimination**:
3. **Filesystem result.txt** (deprecate legacy fallback)
4. **MCP local-memory consensus** (demote to knowledge only)

**Migration Effort**: 5-8 hours (implementation + testing)
**Risk**: MEDIUM (mitigated by parallel write period)
**Value**: HIGH (policy compliance + performance + simplicity)

**Decision**: ✅ **PROCEED with implementation**

---

## Appendix: Related Findings

### Cross-Validation with SPEC-931A/B/F/H

**SPEC-931A Q70**: Which storage systems are necessary?
- **Answered**: AGENT_MANAGER + SQLite are necessary, filesystem + MCP consensus are eliminable

**SPEC-931B D1**: Move MCP artifacts to SQLite
- **Status**: APPROVED but NOT IMPLEMENTED (quality gates still use MCP)
- **Action**: Implement Phase 1 to complete D1 migration

**SPEC-931F**: Event Sourcing Feasibility
- **Finding**: Event sourcing doesn't eliminate AGENT_MANAGER (TUI needs sync cache)
- **Validation**: Confirms AGENT_MANAGER is non-eliminable

**SPEC-931H**: Actor Model Feasibility
- **Finding**: Actor model doesn't eliminate AGENT_MANAGER (same TUI requirement)
- **Validation**: Confirms AGENT_MANAGER is non-eliminable

**Consistency**: All findings align - AGENT_MANAGER + SQLite is the correct target architecture.

---

## Change Log

- **2025-11-13 06:00**: SPEC-931I analysis complete (13 questions, 8 answered)
- **2025-11-13 06:30**: Migration strategy designed (3 phases, 5-8 hours)
- **2025-11-13 07:00**: Report finalized - DECISION: GO (storage consolidation recommended)

---

**SPEC-931I Status**: ✅ **COMPLETE** - Ready for implementation approval
