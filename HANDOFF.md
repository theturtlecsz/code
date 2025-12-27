# Session 35 Prompt - Source Registry Implementation

**Last updated:** 2025-12-27
**Status:** S34 Complete - Registry designed, bugs documented
**Primary SPEC:** SPEC-SOURCE-MGMT

---

## Session 34 Accomplishments

| Item | Status | Location |
|------|--------|----------|
| CLI delete-source bug identified | Root cause found | Missing Content-Length header |
| Source registry schema designed | SQLite schema ready | `docs/SPEC-SOURCE-MGMT/spec.md` |
| Manual source cleanup | Done | 13 → 5 sources (via curl workaround) |
| SPEC.md updated | S34 milestone added | SPEC.md:498-513 |

---

## Bugs Found in notebooklm-client

### 1. CLI delete-source "Invalid JSON response"

**Root cause:** `ServiceClient.request()` in `service-client.ts:574-629` doesn't set `Content-Length` header when sending body. Node.js uses chunked transfer encoding, server returns 400 with empty body.

**Fix needed in `~/notebooklm-client/src/client/service-client.ts`:**

```typescript
// Lines 574-584 - add Content-Length
private async request<T>(
  method: string,
  path: string,
  body?: unknown,
  timeout?: number
): Promise<ServiceResponse<T>> {
  return new Promise((resolve, reject) => {
    const bodyStr = body ? JSON.stringify(body) : undefined;

    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      Accept: "application/json",
    };

    if (bodyStr) {
      headers["Content-Length"] = Buffer.byteLength(bodyStr).toString();
    }

    const options: http.RequestOptions = {
      hostname: this.config.host,
      port: this.config.port,
      path,
      method,
      headers,
      timeout: timeout ?? this.config.timeout,
    };
    // ... rest unchanged, but use bodyStr instead of JSON.stringify(body)
```

### 2. Upsert doesn't return NLM-generated title

**Location:** `~/notebooklm-client/src/service/handlers/sources.ts:388-396`

**Issue:** Response only includes original name, not the title NotebookLM generated.

**Fix needed:** After `addSource()`, call `listSources()` to discover the new title:

```typescript
// After line 386, before res.json:
const updatedList = await sourceManager.listSources(notebookUrl, { showBrowser: false });
const newSource = updatedList.sources.find(s =>
  s.title.toLowerCase().includes(nameWithoutExt)
);

res.json({
  success: true,
  data: {
    name: nameNormalized,
    action,
    sourceType: "text",
    processingTimeMs: addResult.processingTimeMs,
    notebooklmTitle: newSource?.title,  // NEW: actual title
  },
});
```

### 3. Potential off-by-one in upsert delete

**Location:** `~/notebooklm-client/src/service/handlers/sources.ts:360`

**Code:** `deleteSource(notebookUrl, existingSource.index - 1, ...)`

**Question:** Source indices are 1-based from listSources. Does deleteSource expect 0-based or 1-based?

**Check:** Review `source-deleter.ts:87-88` which validates `sourceIndex < 1` suggesting 1-based.

---

## Source Registry Implementation (S35)

### Phase 1: Fix notebooklm-client bugs (prerequisite)

1. Fix Content-Length header in ServiceClient.request()
2. Add notebooklmTitle to upsert response
3. Verify delete index handling

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

## Workarounds

Until bugs are fixed, use curl directly for delete operations:

```bash
# Delete source by index (1-based)
curl -s -X DELETE "http://127.0.0.1:3456/api/sources/3" \
  -H "Content-Type: application/json" \
  -H "Content-Length: 2" \
  -d '{}'

# List sources
notebooklm list-sources
```
