# Session 35 Prompt - Source Registry Implementation

**Last updated:** 2025-12-27
**Status:** S34 Complete - All bugs fixed and verified
**Primary SPEC:** SPEC-SOURCE-MGMT

---

## Session 34 Accomplishments

| Item | Status | Location |
|------|--------|----------|
| CLI delete-source bug | ✅ FIXED & VERIFIED | service-client.ts:627-631 |
| Upsert title return | ✅ FIXED & VERIFIED | sources.ts:389-418 |
| Off-by-one delete | ✅ FIXED & VERIFIED | sources.ts:361 |
| Source registry schema | Designed | `docs/SPEC-SOURCE-MGMT/spec.md` |
| Source cleanup | Done | 13 → 5 sources |

---

## Bugs Fixed in notebooklm-client (All Verified)

### 1. CLI delete-source "Invalid JSON response" ✅ FIXED

**Fix applied (service-client.ts:627-631):**
```typescript
if (body) {
  const bodyStr = JSON.stringify(body);
  req.setHeader("Content-Length", Buffer.byteLength(bodyStr).toString());
  req.write(bodyStr);
}
```

**Verified:** `notebooklm delete-source -i 2 --json` returns valid JSON

### 2. Upsert returns NLM-generated title ✅ FIXED

**Fix applied (sources.ts:389-418):**
- Added `listSources()` call after adding source
- Returns `nlmTitle` field with actual NotebookLM-assigned title

**Verified:** Upsert response includes `nlmTitle` field

**Minor refinement:** Title matching uses substring but NLM transforms significantly. Future: use word-based fuzzy matching.

### 3. Off-by-one in upsert delete ✅ FIXED

**Fix applied (sources.ts:361):**
```typescript
// Removed incorrect -1 offset
await sourceManager.deleteSource(notebookUrl, existingSource.index, {...})
```

**Verified:** Delete works correctly with 1-based index

---

## Source Registry Implementation (S35)

### Phase 1: Prerequisites ✅ COMPLETE

All bugs fixed and verified. Ready for registry implementation.

### Phase 2: Add better-sqlite3 to notebooklm-client

```bash
cd ~/notebooklm-client
npm install better-sqlite3
npm install -D @types/better-sqlite3
```

### Phase 3: Implement SourceRegistry class

**Location:** `~/notebooklm-client/src/sources/source-registry.ts`

**Schema:** See `~/code/docs/SPEC-SOURCE-MGMT/spec.md`

### Phase 4: Integrate with handleUpsertSource

1. Lookup existing source by (notebook_id, spec_id, source_type)
2. If found, delete by stored notebooklm_title
3. After add, update registry with new title

---

## Current Source State

```
Sources in notebook (5 total):
  1. Divine Truth Tier 2 SPEC Analysis Framework [ready]
  2. Golden Path Dogfooding Validation: Stage0 Verification Spec [ready]
  3. Protocol for Active Testing Specifications [ready]
  4. The Codex TUI Dogfooding Protocol [ready]
  5. Golden Path Dogfooding Validation: SPEC-DOGFOOD-001 [ready]
```

**Static sources (don't delete):** 1, 3, 4
**Dynamic sources (managed by registry):** 2, 5

---

## Key Files

| Location | Purpose |
|----------|---------|
| `~/code/docs/SPEC-SOURCE-MGMT/spec.md` | Registry architecture spec |
| `~/notebooklm-client/src/client/service-client.ts` | HTTP client (Content-Length fix) |
| `~/notebooklm-client/src/service/handlers/sources.ts` | Upsert handler (title return) |
| `~/notebooklm-client/src/sources/source-deleter.ts` | Delete implementation |

---

## Commits (S34)

| Commit | Description |
|--------|-------------|
| (pending) | docs: SPEC-SOURCE-MGMT and S34 milestone |

---

## Test Results

All fixes verified:
```bash
# Delete-source CLI works
notebooklm delete-source -i 2 --json
# Returns: {"success": true, "data": {"deletedTitle": "..."}}

# Upsert returns NLM title
curl -X POST ".../api/sources/upsert" -d '{"name": "...", "content": "..."}'
# Returns: {"data": {"nlmTitle": "...", "action": "created"}}
```
