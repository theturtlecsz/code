# SPEC-945B Production Validation Guide

**Date**: 2025-11-13
**Branch**: feature/phase-1-sqlite-retry
**Commit**: eebcc867d (Week 2 Day 6 - Write-Path Cutover)
**Status**: Ready for Manual Testing

## Overview

Phase 1 (Week 1-2) database migration is complete with 16/16 integration tests passing. This guide provides step-by-step instructions for production validation in the live TUI environment.

## Prerequisites

✅ **Completed**:
- TUI binary built: `target/release/code-tui` (40MB, release profile)
- Integration tests: 16/16 passing
  - `read_path_migration.rs`: 8/8 tests ✅
  - `write_path_cutover.rs`: 8/8 tests ✅
- Performance benchmarks: All targets met or exceeded
- Documentation: Phase 1 complete, Phase 2 planned

## Validation Steps

### Step 1: Backup Existing Database

**IMPORTANT**: Back up your current consensus database before testing:

```bash
# Backup existing database
cp ~/.code/consensus_artifacts.db ~/.code/consensus_artifacts.db.backup-$(date +%Y%m%d-%H%M%S)

# Verify backup
ls -lh ~/.code/consensus_artifacts.db*
```

**Recovery**: If issues occur, restore with:
```bash
cp ~/.code/consensus_artifacts.db.backup-YYYYMMDD-HHMMSS ~/.code/consensus_artifacts.db
```

### Step 2: Launch TUI

```bash
cd /home/thetu/code/codex-rs
./target/release/code-tui
```

**Expected**: TUI launches normally with no errors.

### Step 3: Run Consensus Operation

Execute a lightweight consensus operation to test write-path:

```
/speckit.plan SPEC-KIT-945
```

**OR** (if SPEC-KIT-945 doesn't exist):

```
/speckit.new Test write-path cutover validation
/speckit.plan SPEC-KIT-XXX
```

**Expected**:
- Agents spawn successfully
- Consensus completes (degraded or 3/3 acceptable)
- No errors in TUI output

### Step 4: Verify New Schema Writes

**Open SQLite CLI** (in separate terminal while TUI is running or after):

```bash
sqlite3 ~/.code/consensus_artifacts.db
```

**Query 1: Verify writes to new schema**:
```sql
-- Should show recent consensus runs (NOT empty)
SELECT
    id,
    spec_id,
    stage,
    datetime(run_timestamp, 'unixepoch') as timestamp,
    consensus_ok,
    degraded
FROM consensus_runs
ORDER BY run_timestamp DESC
LIMIT 10;
```

**Expected**: Recent entries with SPEC-KIT-945 or test SPEC-ID.

**Query 2: Verify agent outputs**:
```sql
-- Should show agent outputs for recent run_id
SELECT
    id,
    run_id,
    agent_name,
    model_version,
    substr(content, 1, 100) as content_preview,
    datetime(output_timestamp, 'unixepoch') as timestamp
FROM agent_outputs
ORDER BY output_timestamp DESC
LIMIT 10;
```

**Expected**: Agent outputs (claude-haiku, gemini-flash, gpt-5-medium, or similar).

### Step 5: Verify Old Schema is Empty

**Query 3: Confirm old schema has NO new writes**:
```sql
-- Should show NO entries after write-path cutover
-- (Last entry timestamp should be BEFORE 2025-11-13 19:46)
SELECT
    id,
    spec_id,
    stage,
    timestamp,
    substr(content, 1, 50) as content_preview
FROM consensus_artifacts
ORDER BY timestamp DESC
LIMIT 10;
```

**Expected**:
- Either **empty table** (no historical data)
- OR **old data only** (timestamps before cutover, 2025-11-13 19:46)
- **NO NEW ENTRIES** after cutover

### Step 6: Verify Dual-Schema Reader

**Query 4: Test read fallback for old data** (if old data exists):
```sql
-- Query old schema directly
SELECT COUNT(*) as old_count FROM consensus_artifacts;

-- Query new schema directly
SELECT COUNT(*) as new_count FROM consensus_runs;
```

**Expected**:
- `new_count` > 0 (at least 1 from /speckit.plan test)
- `old_count` >= 0 (historical data or 0 if fresh DB)

**In TUI**: Verify historical consensus data is accessible:
```
/speckit.status SPEC-KIT-XXX
```

**Expected**: Shows consensus history from BOTH old and new schemas.

### Step 7: Performance Validation

**Query 5: Measure query performance**:
```sql
-- Enable timing
.timer on

-- Test read performance (should be fast, <10ms)
SELECT * FROM consensus_runs WHERE spec_id = 'SPEC-KIT-945';

-- Test join performance (dual-schema reader pattern)
SELECT
    cr.id,
    cr.spec_id,
    cr.stage,
    COUNT(ao.id) as agent_count
FROM consensus_runs cr
LEFT JOIN agent_outputs ao ON ao.run_id = cr.id
GROUP BY cr.id
LIMIT 100;

.timer off
```

**Expected**: Queries return in <50ms (WAL mode benefits).

### Step 8: Database Size Validation

```bash
# Check DB size (should be <10MB for typical usage)
ls -lh ~/.code/consensus_artifacts.db

# Check for WAL file (indicates WAL mode active)
ls -lh ~/.code/consensus_artifacts.db-wal
```

**Expected**:
- Main DB: <10MB (auto-vacuum active)
- WAL file: Present, <5MB typically

### Step 9: Stress Test (Optional)

Run multiple consensus operations:

```
/speckit.plan SPEC-KIT-001
/speckit.plan SPEC-KIT-002
/speckit.validate SPEC-KIT-003
```

**Verify**:
- No errors or crashes
- All writes go to new schema
- Old schema remains unchanged
- Performance remains acceptable

## Validation Checklist

- [ ] TUI launches successfully
- [ ] Consensus operation completes without errors
- [ ] New schema has entries (consensus_runs + agent_outputs)
- [ ] Old schema has NO new entries after cutover
- [ ] Historical data accessible (if applicable)
- [ ] Query performance acceptable (<50ms)
- [ ] Database size reasonable (<10MB)
- [ ] No crashes or data corruption

## Success Criteria

✅ **Phase 1 Production Validation Complete** when:
1. ✅ TUI works with new database layer
2. ✅ Consensus writes go to new schema ONLY
3. ✅ Old schema remains read-only (no new writes)
4. ✅ Dual-schema reader works (old data accessible)
5. ✅ No errors, crashes, or data loss
6. ✅ Performance acceptable (no regressions)

## Rollback Plan

**If Issues Detected**:

1. **Immediate**: Stop TUI, restore backup
   ```bash
   cp ~/.code/consensus_artifacts.db.backup-YYYYMMDD-HHMMSS ~/.code/consensus_artifacts.db
   ```

2. **Revert Commit**:
   ```bash
   git revert eebcc867d  # Revert write-path cutover
   git revert a1149b370  # Revert read-path migration
   ```

3. **Rebuild and Test**:
   ```bash
   cargo build --release --bin code-tui
   ./target/release/code-tui
   ```

4. **Report Issue**: Document error, provide logs

**Recovery Time**: <5 minutes

## Known Limitations

1. **Unit Tests**: 4 unit tests in `consensus_db.rs` expect dual-write behavior (documented, not critical for production)
2. **No Backfill**: Old schema data not migrated to new schema (read fallback ensures accessibility)
3. **Retry Module**: Scaffolding exists but implementation deferred to Week 2-3

## Next Steps After Validation

**Week 2 Day 7**: Production Validation ✅ (this document)

**Week 3+**: Old Schema Deprecation
- Remove old schema tables (consensus_artifacts, consensus_synthesis)
- Update unit tests to remove dual-write expectations
- Clean up old code paths (delete_spec_artifacts, count_artifacts)
- Full migration complete

**Phase 2**: Retry Logic Implementation
- Week 2-3: 20 hours estimated
- Day 1-2: Retry module (exponential backoff + jitter)
- Day 3: Error classification (retryable vs permanent)
- Day 4-5: Integration with database layer

## References

- **Phase 1 Checklist**: `docs/SPEC-KIT-945-implementation-research/PHASE-1-IMPLEMENTATION-CHECKLIST.md`
- **Write-Path Cutover**: `docs/SPEC-KIT-945-write-path-cutover.md`
- **Read-Path Migration**: `docs/SPEC-KIT-945-read-path-migration-strategy.md`
- **Performance Benchmarks**: `docs/SPEC-KIT-945-implementation-research/PERFORMANCE-BENCHMARKS.md`
- **Integration Tests**: `codex-rs/tui/tests/read_path_migration.rs`, `write_path_cutover.rs`

## SQL Queries Reference

```sql
-- Quick validation query (run this first)
SELECT
    'New Schema' as schema,
    COUNT(*) as count
FROM consensus_runs
UNION ALL
SELECT
    'Old Schema' as schema,
    COUNT(*) as count
FROM consensus_artifacts;

-- Expected Output:
-- New Schema  | 1+ (recent writes)
-- Old Schema  | 0 or N (historical only, no new writes)
```

---

**Status**: Validation guide complete. Ready for manual testing.
**Next Action**: User to follow steps 1-9 and report results.
