# Data Integrity Analysis - Local Memory

**Date**: 2025-11-30 (P72)
**Database**: `~/.local-memory/unified-memories.db`
**Total Records**: 1,161 memories across 63 sessions

---

## Executive Summary

| Issue | Severity | Records Affected | Remediation |
|-------|----------|------------------|-------------|
| Timestamp format (Go-style) | Medium | 100% | Normalize during Phase 0 |
| agent_type = 'unknown' | Critical | 100% (1,161) | Extract from tags during Phase 0 |
| Missing agent attribution | High | 69.6% (808) | No recovery possible |
| Importance inflation | High | 82.7% (960) | Add dynamic relevance scoring |
| access_scope = 'session' only | Medium | 100% | Schema design issue |

---

## 1. Timestamp Analysis

### Format
```
2025-09-25 16:34:53.227315703 -0400 EDT m=+6984.437758214
```

**Components:**
- Date/time: Standard ISO-like format ✓
- Timezone: `-0400 EDT` ✓
- Monotonic clock: `m=+6984.437758214` ✗ (non-standard, should not persist)

**Root Cause**: Go's `time.Time` serialized directly without formatting. The monotonic clock marker is internal to Go runtime and should be stripped before storage.

**Impact**: Date-based queries work (prefix matching), but:
- Comparison operations may fail
- External tools expect ISO 8601
- Storage waste (~30 bytes/record)

**Null timestamps**: 0 (all records have timestamps)

### Remediation
```sql
-- Phase 0: Normalize timestamps
UPDATE memories SET
  created_at = substr(created_at, 1, 26),
  updated_at = substr(updated_at, 1, 26);
```

---

## 2. Agent Attribution Analysis

### agent_type Column
```sql
SELECT agent_type, COUNT(*) FROM memories GROUP BY agent_type;
-- Result: unknown|1161 (100% unknown)
```

**Root Cause**: The `agent_type` column was added to schema but ingestion pipeline never populates it. Agent information IS captured in tags but not extracted.

### agent:* Tags
```sql
SELECT COUNT(*) FROM memories WHERE tags LIKE '%agent:%';
-- Result: 353 (30.4%)
```

**Agent types found in tags:**
| Agent | Count (estimated) |
|-------|-------------------|
| agent:claude | ~150 |
| agent:gemini | ~100 |
| agent:gpt_pro | ~50 |
| agent:code | ~30 |
| agent:gpt_codex | ~20 |

**Missing attribution**: 808 memories (69.6%) have NO agent information anywhere.

### Remediation
```sql
-- Phase 0: Extract agent from tags for existing records
UPDATE memories
SET agent_type = json_extract(
  (SELECT value FROM json_each(tags) WHERE value LIKE 'agent:%' LIMIT 1),
  '$'
)
WHERE tags LIKE '%agent:%' AND agent_type = 'unknown';

-- Strip 'agent:' prefix
UPDATE memories
SET agent_type = replace(agent_type, 'agent:', '')
WHERE agent_type LIKE 'agent:%';
```

**Note**: 808 records cannot be backfilled - agent information was never captured.

---

## 3. Importance Distribution (Inflation Problem)

```sql
SELECT importance, COUNT(*) FROM memories GROUP BY importance ORDER BY importance DESC;
```

| Importance | Count | Percentage |
|------------|-------|------------|
| 10 | 134 | 11.5% |
| 9 | 304 | 26.2% |
| 8 | 522 | 45.0% |
| 7 | 66 | 5.7% |
| 6 | 119 | 10.2% |
| 5 | 16 | 1.4% |

**Problem**: 82.7% of memories rated 8-10 (high importance). This defeats the purpose of importance-based filtering.

**Root Cause**:
1. No calibration guidance during storage
2. Pipeline memories auto-assigned high importance
3. Human tendency to rate own work as important

**Impact**:
- Importance-based queries return too many results
- Cannot distinguish truly critical information
- Search relevance degraded

**Remediation**: Dynamic relevance scoring (Phase 2) that combines:
- Static importance (user-assigned)
- Recency decay
- Query-context similarity
- Access frequency

---

## 4. Domain Distribution

```sql
SELECT domain, COUNT(*) FROM memories GROUP BY domain ORDER BY COUNT(*) DESC LIMIT 10;
```

| Domain | Count | Percentage |
|--------|-------|------------|
| spec-kit | 672 | 57.9% |
| infrastructure | 291 | 25.1% |
| rust | 67 | 5.8% |
| debugging | 40 | 3.4% |
| NULL | 36 | 3.1% |
| spec-tracker | 15 | 1.3% |
| upstream-sync | 12 | 1.0% |
| programming | 9 | 0.8% |
| docs-ops | 5 | 0.4% |
| impl-notes | 5 | 0.4% |

**Assessment**: Good domain coverage with 36 NULL records (3.1%) needing classification.

---

## 5. Access Scope

```sql
SELECT access_scope, COUNT(*) FROM memories GROUP BY access_scope;
-- Result: session|1161 (100% session-scoped)
```

**Problem**: All memories are session-scoped. The `global` and `private` scopes are unused.

**Impact**: Cross-session knowledge sharing not functioning as designed.

**Root Cause**: Default value is 'session' and ingestion never overrides.

**Remediation**: Review access_scope policy and implement auto-classification based on:
- Content type (architecture decisions → global)
- Importance (≥9 → global candidate)
- Domain (infrastructure → global)

---

## 6. Temporal Distribution

```sql
SELECT substr(created_at, 1, 10) as date, COUNT(*)
FROM memories GROUP BY date ORDER BY date DESC LIMIT 15;
```

| Date | Count |
|------|-------|
| 2025-11-30 | 17 |
| 2025-11-29 | 87 |
| 2025-11-28 | 34 |
| 2025-11-27 | 25 |
| 2025-11-26 | 27 |
| 2025-11-25 | 22 |
| ... | ... |

**Average**: ~15-20 memories/day
**Peak**: 87 memories on 2025-11-29 (likely intensive session)
**Date range**: 2025-09-25 to 2025-11-30 (~66 days)

---

## 7. Session Distribution

```sql
SELECT COUNT(DISTINCT session_id) FROM memories;
-- Result: 63
```

**Memories per session**: 1161 / 63 = ~18.4 average

---

## Phase 0 Migration Script (Draft)

```sql
-- Run in transaction
BEGIN;

-- 1. Normalize timestamps
UPDATE memories SET
  created_at = substr(created_at, 1, 26),
  updated_at = substr(updated_at, 1, 26);

-- 2. Extract agent_type from tags
UPDATE memories
SET agent_type = (
  SELECT replace(json_each.value, 'agent:', '')
  FROM json_each(memories.tags)
  WHERE json_each.value LIKE 'agent:%'
  LIMIT 1
)
WHERE tags LIKE '%agent:%' AND agent_type = 'unknown';

-- 3. Verify
SELECT 'Timestamps normalized' as check,
       COUNT(*) as affected
FROM memories WHERE created_at NOT LIKE '%m=+%';

SELECT 'Agent types extracted' as check,
       agent_type, COUNT(*) as count
FROM memories GROUP BY agent_type;

-- 4. Commit if satisfied
COMMIT;
```

---

## Open Questions

1. **Backfill strategy**: Can we infer agent from session patterns or content analysis?
2. **Importance recalibration**: Should we normalize existing importance scores?
3. **Access scope policy**: What criteria should promote memories to global scope?
4. **Timestamp precision**: Is microsecond precision needed, or should we truncate to seconds?

---

## Next Steps

1. [ ] Review Phase 0 migration script with user
2. [ ] Test migration on backup database
3. [ ] Implement ingestion fixes to prevent future issues
4. [ ] Design dynamic relevance scoring algorithm
