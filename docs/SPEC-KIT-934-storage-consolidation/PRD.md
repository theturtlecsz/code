# PRD: Storage Consolidation

**SPEC-ID**: SPEC-KIT-934
**Created**: 2025-11-13
**Status**: Draft - **HIGH PRIORITY**
**Priority**: **P1** (Policy Violation + Performance)
**Owner**: Code
**Estimated Effort**: 10-13 hours (2-3 days)
**Dependencies**: SPEC-933 (transactions enable safe migration)
**Blocks**: SPEC-936 (filesystem elimination depends on this)

---

## 🔥 Executive Summary

**Current State**: 4 separate storage systems for workflow data - AGENT_MANAGER (HashMap), SQLite (consensus_artifacts.db), Filesystem (~/.code/agents/), and MCP local-memory (policy violation!). Consensus artifacts go to MCP (5× slower, violates SPEC-KIT-072 separation of concerns). Unused tables confuse architecture.

**Proposed State**: 2 storage systems only - AGENT_MANAGER (TUI in-memory coordination) + SQLite (persistent workflow state). MCP used exclusively for human-curated knowledge per policy. Consensus artifacts migrated to SQLite (5× faster: 30ms vs 150ms). Dead code removed (consensus_synthesis table).

**Impact**:
- ✅ Restores SPEC-KIT-072 policy compliance (workflow vs knowledge separation)
- ✅ 5× faster consensus storage (30ms vs 150ms)
- ✅ Reduces 4 systems → 2 (simpler architecture)
- ✅ Eliminates MCP from agent orchestration entirely

**Source**: SPEC-931A architectural analysis + SPEC-931B configuration analysis identified policy violation and performance overhead.

---

## 1. Problem Statement

### Issue #1: SPEC-KIT-072 Policy Violation (CRITICAL)

**Policy** (MEMORY-POLICY.md:351-375):
- **Workflow data** (transient, orchestration state) → SQLite
- **Knowledge** (human-curated, reusable insights) → MCP local-memory

**Current Reality** (quality_gate_handler.rs:1775):
```rust
// Consensus artifacts going to MCP - VIOLATES POLICY!
mcp_client.store_memory(
    content: consensus_artifact_json,  // Workflow data, NOT knowledge
    domain: "spec-kit",
    tags: ["consensus", "stage:validate"],
    importance: 7
);
```

**Evidence of Violation** (SPEC-931B-analysis.md:464-467):
- Policy says: Consensus → SQLite, Knowledge → MCP
- Reality shows: Consensus → MCP (wrong system!)

**Impact**:
- MCP polluted with workflow data (not human-curated knowledge)
- 5× slower (150ms MCP calls vs 30ms SQLite writes)
- Policy drift (documented architecture doesn't match reality)
- Harder to find actual knowledge (buried in workflow noise)

---

### Issue #2: Four Storage Systems For One Workflow (HIGH)

**Current Architecture** (SPEC-931A phase1-dataflows.md:687-692):

1. **AGENT_MANAGER** (HashMap, in-memory):
   - Purpose: TUI coordination, 60 FPS rendering
   - Required: Yes (can't eliminate, TUI needs in-memory)

2. **SQLite** (consensus_artifacts.db):
   - Purpose: Persistent routing, agent tracking
   - Required: Yes (persistent state, query support)

3. **Filesystem** (~/.code/agents/):
   - Purpose: Tmux collection, legacy fallback
   - Required: No (after SPEC-936 tmux elimination)

4. **MCP local-memory**:
   - Purpose: Should be knowledge only (per policy)
   - Current Use: Consensus artifacts (VIOLATION!)
   - Required: No (for agent orchestration)

**Complexity Cost**:
- 4 separate I/O patterns to maintain
- Multiple failure modes (filesystem full, MCP timeout, SQLite lock)
- Confusing data flow (which system is source of truth?)
- Debugging requires checking 4 locations

**Proposed**: Reduce to 2 systems (AGENT_MANAGER + SQLite).

---

### Issue #3: Unused Database Tables (MEDIUM)

**consensus_artifacts Table** (SPEC-931A phase1-database.md:144-148):
- **Status**: 0 rows, table exists but unused
- **Hypothesis**: Created for MCP→SQLite migration (never completed)
- **Reality**: Consensus currently goes to MCP (wrong!)
- **Decision**: USE this table (migrate MCP → SQLite)

**consensus_synthesis Table** (SPEC-931A phase1-database.md:196-200):
- **Status**: 0 rows, method defined but never called
- **Expected Usage**: Store final consensus after merging agents
- **Reality**: Quality gates apply auto-resolution directly, skip synthesis
- **Investigation Needed**: Is this dead code or just not implemented?

**Impact of Unused Tables**:
- Confusing schema (why do these tables exist?)
- Developers waste time investigating unused code
- Unclear architecture intent

---

### Issue #4: MCP Performance Overhead (MEDIUM)

**MCP Storage** (quality_gate_handler.rs:1775):
- Store consensus artifact: **~150ms** (MCP call + network + local-memory write)
- Search validation results: **~200ms** (MCP search query)

**SQLite Alternative**:
- Store consensus artifact: **~30ms** (local disk write + index update)
- Read validation results: **~5ms** (direct HashMap lookup or SQL query)

**Performance Gap**:
- Storage: **5× slower** (150ms vs 30ms)
- Retrieval: **40× slower** (200ms vs 5ms)

**Frequency**: 10 quality gates/day × 3 agents = 30 MCP calls/day = 4.5s/day overhead.
Not critical, but unnecessary. **Eliminate for principle** (simplicity + policy compliance).

---

## 2. Proposed Solution

### Component 1: Migrate Consensus Artifacts MCP → SQLite (6-8h)

**Implementation**:
```rust
// OLD (quality_gate_handler.rs:1775)
mcp_client.store_memory(
    content: artifact_json,
    domain: "spec-kit",
    tags: ["consensus", stage],
    importance: 7,
)?;

// NEW (use existing consensus_artifacts table)
db.execute(
    "INSERT INTO consensus_artifacts (
        spec_id, stage, agent_name, content_json, created_at
    ) VALUES (?, ?, ?, ?, ?)",
    params![spec_id, stage, agent_name, artifact_json, now()],
)?;
```

**consensus_artifacts Schema** (already exists!):
```sql
CREATE TABLE consensus_artifacts (
    id INTEGER PRIMARY KEY,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,         -- "plan", "tasks", "validate", etc.
    agent_name TEXT NOT NULL,    -- "gemini", "claude", "code"
    content_json TEXT NOT NULL,  -- Consensus artifact JSON
    created_at TEXT NOT NULL,

    INDEX idx_spec_stage (spec_id, stage),
    INDEX idx_created_at (created_at)
);
```

**Files to Modify**:
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (storage calls)
- `codex-tui/src/chatwidget/spec_kit/consensus.rs` (artifact queries)
- `codex-core/src/consensus_db.rs` (add helper methods)

**Migration** (one-time):
```rust
// Optional: Migrate existing MCP consensus artifacts to SQLite
async fn migrate_mcp_to_sqlite() -> Result<usize> {
    let mcp_artifacts = mcp_client.search(
        query: "consensus",
        tags: ["consensus"],
        limit: 1000,
    ).await?;

    let mut migrated = 0;
    for artifact in mcp_artifacts {
        db.execute(
            "INSERT INTO consensus_artifacts (...) VALUES (...)",
            // Extract from MCP artifact
        )?;
        migrated += 1;
    }

    Ok(migrated)
}
```

---

### Component 2: Investigate consensus_synthesis Table (2h)

**Investigation Steps**:
1. **Code Search**: Grep for `consensus_synthesis` across entire codebase
2. **Git History**: Check when table was added, original intent
3. **Quality Gate Flow**: Verify auto-resolution skips synthesis step
4. **Decision**:
   - If dead code → DROP TABLE
   - If planned feature → Document in TODO, keep table
   - If unclear → Keep table, add comment explaining uncertainty

**Files to Check**:
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (auto-resolution logic)
- `codex-core/src/consensus_db.rs` (synthesis storage method)
- Git history: `git log --all --grep="consensus_synthesis"`

**Proposed Outcome**: Likely dead code (never called in 2 months of operation).
**Action**: DROP TABLE unless investigation reveals planned usage.

**Drop Script**:
```sql
-- If consensus_synthesis is dead code
DROP TABLE IF EXISTS consensus_synthesis;
```

---

### Component 3: Eliminate MCP from Agent Orchestration (2-3h)

**After Component 1** (consensus → SQLite):
- MCP no longer used for workflow data
- MCP only stores human-curated knowledge (per policy)

**Cleanup Actions**:
1. **Remove MCP Imports** from orchestration modules:
   - `quality_gate_handler.rs`: Remove `use mcp_client::*`
   - `consensus.rs`: Remove MCP search calls
   - `handler.rs`: Remove MCP artifact storage

2. **Update Documentation**:
   - `MEMORY-POLICY.md`: Confirm compliance (workflow → SQLite, knowledge → MCP)
   - `ARCHITECTURE.md`: Update data flow diagrams (remove MCP from orchestration)

3. **Test Coverage**:
   - Verify quality gates work WITHOUT MCP
   - Confirm consensus retrieval from SQLite
   - Check no MCP calls in orchestration tests

**Validation**:
```bash
# Ensure no MCP calls in spec_kit modules (except knowledge storage)
grep -r "mcp_client" codex-tui/src/chatwidget/spec_kit/ | \
    grep -v "// Optional: Store important decision to MCP for knowledge"

# Should return 0 results (no MCP in orchestration)
```

---

### Component 4: Update SPEC-KIT-072 Compliance Validation (+1h)

**Create Automated Check** (`scripts/validate_storage_policy.sh`):
```bash
#!/bin/bash
# Validate SPEC-KIT-072 storage separation policy

echo "Checking for policy violations..."

# Check 1: No consensus artifacts in MCP calls
VIOLATIONS=$(grep -r "mcp.*consensus" codex-tui/src/chatwidget/spec_kit/ | grep -v "^//" | wc -l)

if [ $VIOLATIONS -gt 0 ]; then
    echo "❌ FAILED: Found $VIOLATIONS MCP consensus storage calls (violates SPEC-KIT-072)"
    grep -rn "mcp.*consensus" codex-tui/src/chatwidget/spec_kit/ | grep -v "^//"
    exit 1
fi

# Check 2: Consensus artifacts go to SQLite
SQLITE_CALLS=$(grep -r "consensus_artifacts" codex-tui/src/chatwidget/spec_kit/ | wc -l)

if [ $SQLITE_CALLS -lt 3 ]; then
    echo "❌ FAILED: Not enough consensus_artifacts SQLite calls (expected ≥3 for insert/query/update)"
    exit 1
fi

echo "✅ PASSED: Storage policy compliance validated"
echo "   - No MCP consensus storage (workflow → SQLite)"
echo "   - $SQLITE_CALLS consensus_artifacts SQLite calls found"
```

**Add to CI Pipeline**:
```yaml
# .github/workflows/ci.yml
- name: Validate Storage Policy
  run: bash scripts/validate_storage_policy.sh
```

---

## 3. Acceptance Criteria

### AC1: Policy Compliance ✅
- [ ] All consensus artifacts stored in SQLite (not MCP)
- [ ] MCP only used for human-curated knowledge
- [ ] SPEC-KIT-072 compliance validated (automated check passes)
- [ ] Documentation updated (MEMORY-POLICY.md, ARCHITECTURE.md)

### AC2: Performance ✅
- [ ] Consensus storage: <50ms (down from 150ms, 3× faster)
- [ ] Consensus retrieval: <10ms (down from 200ms, 20× faster)
- [ ] No performance regression on quality gates

### AC3: Architecture Simplification ✅
- [ ] MCP removed from agent orchestration code
- [ ] 4 storage systems → 2 (AGENT_MANAGER + SQLite)
- [ ] consensus_synthesis table investigated (DROP if dead code)

### AC4: Data Migration ✅
- [ ] Existing MCP consensus artifacts migrated to SQLite (optional, if valuable)
- [ ] No data loss during migration
- [ ] All quality gate tests pass with SQLite storage

---

## 4. Technical Implementation

### Phase 1: Consensus Migration (6-8h)

**Day 1**:
- Update `consensus_db.rs` with insert/query methods for consensus_artifacts
- Modify `quality_gate_handler.rs` storage calls (MCP → SQLite)
- Add migration script for existing MCP artifacts (optional)

**Day 2**:
- Update `consensus.rs` retrieval calls (MCP search → SQLite query)
- Integration tests (quality gate flow with SQLite)
- Performance benchmarks (verify <50ms storage, <10ms retrieval)

**Files**:
- `codex-core/src/consensus_db.rs` (+150 LOC)
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (-50 LOC MCP, +30 LOC SQLite)
- `codex-tui/src/chatwidget/spec_kit/consensus.rs` (-30 LOC MCP search, +20 LOC SQL)
- `scripts/migrate_mcp_consensus.sh` (optional migration)

---

### Phase 2: consensus_synthesis Investigation (2h)

**Day 3**:
- Code search (`grep -r "consensus_synthesis"`)
- Git history analysis (`git log --all --grep="synthesis"`)
- Quality gate flow review (verify auto-resolution skips synthesis)
- **Decision**: DROP TABLE or document planned usage
- Execute drop script if dead code

**Files**:
- `codex-core/src/consensus_db.rs` (DROP TABLE migration)
- `docs/decisions/934-consensus-synthesis-removal.md` (if removed)

---

### Phase 3: MCP Elimination + Validation (3h)

**Day 4**:
- Remove MCP imports from orchestration modules
- Update documentation (MEMORY-POLICY.md, ARCHITECTURE.md)
- Create `scripts/validate_storage_policy.sh` automated check
- Add CI integration

**Day 5**:
- Final testing (all quality gate tests pass)
- Performance validation (<50ms consensus storage)
- Policy compliance check (`validate_storage_policy.sh` passes)

**Files**:
- `codex-tui/src/chatwidget/spec_kit/*.rs` (remove MCP imports)
- `MEMORY-POLICY.md` (compliance notes)
- `scripts/validate_storage_policy.sh` (new automated check)
- `.github/workflows/ci.yml` (add policy validation step)

---

## 5. Success Metrics

### Performance Metrics
- **Consensus Storage**: 150ms → <50ms (3× faster)
- **Consensus Retrieval**: 200ms → <10ms (20× faster)
- **End-to-End Quality Gate**: No performance regression

### Architecture Metrics
- **Storage Systems**: 4 → 2 (50% reduction)
- **MCP Calls (Orchestration)**: 30/day → 0 (100% elimination)
- **Policy Compliance**: 100% (automated validation passes)

### Code Metrics
- **LOC Reduction**: ~100 LOC removed (MCP calls, unused imports)
- **Complexity Reduction**: 4 I/O patterns → 2 (simpler)

---

## 6. Risk Analysis

### Risk 1: Migration Data Loss (HIGH)
**Scenario**: MCP→SQLite migration fails, consensus artifacts lost.
**Mitigation**:
- Migration is **optional** (existing MCP data can stay)
- New artifacts go to SQLite (future-proof)
- Backup MCP data before migration (`mcp export`)
**Likelihood**: Low (migration is non-critical)

---

### Risk 2: consensus_synthesis Actually Needed (MEDIUM)
**Scenario**: Investigation concludes synthesis table is planned feature, not dead code.
**Mitigation**:
- Keep table if investigation is inconclusive
- Add TODO comment explaining future intent
- Document in ARCHITECTURE.md
**Likelihood**: Low (table unused for 2+ months)

---

### Risk 3: Performance Regression (LOW)
**Scenario**: SQLite writes slower than expected (>50ms).
**Mitigation**:
- SPEC-933 transactions already optimize SQLite writes
- Benchmark before/after migration
- Rollback to MCP if regression confirmed (unlikely)
**Likelihood**: Very Low (SQLite is proven fast for this workload)

---

## 7. Open Questions

### Q1: Should we keep historical MCP consensus artifacts?
**Context**: Migration script can copy MCP artifacts to SQLite, but is this valuable?
**Decision**: **OPTIONAL** - Migrate only if user wants historical analysis. New artifacts more important.

---

### Q2: What's the long-term MCP usage strategy?
**Context**: After SPEC-934, MCP only for human knowledge. Should we document this clearly?
**Decision**: YES - Update MEMORY-POLICY.md with explicit "MCP for knowledge ONLY, never workflow" rule.

---

## 8. Implementation Strategy

### Day 1: Consensus Migration Core (6h)
- Update consensus_db.rs with SQLite methods
- Modify quality_gate_handler.rs storage (MCP → SQLite)
- Basic integration test

### Day 2: Retrieval + Performance (2h)
- Update consensus.rs retrieval (MCP search → SQL query)
- Performance benchmarks (<50ms storage, <10ms retrieval)

### Day 3: consensus_synthesis Investigation (2h)
- Code search, git history, quality gate flow review
- Decision: DROP TABLE or keep with TODO

### Day 4: MCP Cleanup + Validation (2h)
- Remove MCP imports from orchestration
- Create automated policy validation script
- Update documentation

### Day 5: Final Testing + CI (1h)
- All quality gate tests pass
- CI policy validation step added
- Documentation review

**Total**: 13h (within 10-13h estimate)

---

## 9. Deliverables

1. **Code Changes**:
   - `codex-core/src/consensus_db.rs` - SQLite consensus storage
   - `codex-tui/src/chatwidget/spec_kit/*.rs` - Remove MCP, use SQLite
   - `scripts/migrate_mcp_consensus.sh` - Optional migration

2. **Documentation**:
   - `MEMORY-POLICY.md` - Updated compliance notes
   - `docs/decisions/934-consensus-synthesis-removal.md` - If table dropped
   - `ARCHITECTURE.md` - Updated data flow diagrams

3. **Validation**:
   - `scripts/validate_storage_policy.sh` - Automated policy check
   - `.github/workflows/ci.yml` - CI integration

4. **Tests**:
   - Integration tests (quality gate with SQLite)
   - Performance benchmarks (consensus storage/retrieval)

---

## 10. Validation Plan

### Integration Tests (5 tests)
- Quality gate flow with SQLite consensus storage
- Consensus retrieval from SQLite (not MCP)
- Multi-agent consensus coordination

### Performance Tests (3 benchmarks)
- Consensus storage time (<50ms)
- Consensus retrieval time (<10ms)
- End-to-end quality gate (no regression)

### Policy Tests (1 automated check)
- `validate_storage_policy.sh` passes in CI
- No MCP calls in orchestration code
- consensus_artifacts table has ≥3 call sites

**Total**: 9 tests

---

## 11. Conclusion

SPEC-934 restores SPEC-KIT-072 policy compliance, eliminates MCP from agent orchestration, and consolidates 4 storage systems to 2. **Estimated effort: 10-13 hours over 5 days.**

**Key Benefits**:
- ✅ Policy compliance restored (workflow → SQLite, knowledge → MCP)
- ✅ 3-20× faster consensus operations (30ms vs 150ms)
- ✅ 50% architecture simplification (4 → 2 systems)
- ✅ Automated compliance validation (CI check)

**Next Steps**:
1. Review and approve SPEC-934
2. Schedule implementation (5 day sprint)
3. Optional: Decide on MCP artifact migration
4. Coordinate with SPEC-933 (transactions enable safe migration)

---

Back to [Key Docs](../KEY_DOCS.md)
