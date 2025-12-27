# SPEC-SOURCE-MGMT: NotebookLM Source Lifecycle Management

**Status:** Draft
**Created:** 2025-12-27
**Author:** S34 Session

---

## Problem Statement

The current source-based Tier2 architecture creates orphan sources over time because:

1. **Title Mutation:** NotebookLM generates semantic titles from content (unpredictable)
2. **Fuzzy Matching Failure:** The current `handleUpsertSource` uses fuzzy title matching, but changed titles escape detection
3. **No Tracking:** No persistent record of which sources belong to which spec
4. **No Cleanup:** Stale sources accumulate indefinitely
5. **Hard Limit:** NotebookLM has a 50-source limit per notebook

### Evidence

Session 33 observed 13 sources in notebook, 6 redundant after multiple `/speckit.auto` runs:
- Same spec uploaded multiple times with slight content variations
- Each run created new source instead of updating existing
- Sources 2,3,6,7,12,13 identified as redundant

---

## Architecture Decision

**Chosen:** Option A - Local SQLite Registry

**Rationale:**
- Persistent tracking survives restarts
- Can track source history and last-used timestamps
- Enables cleanup policies based on age
- SQLite is lightweight, file-based, no server needed
- Can be integrated at service layer (notebooklm-client) or adapter layer (codex-rs)

**Rejected alternatives:**
- **Option B (Naming Convention):** NotebookLM rewrites titles regardless of prefixes
- **Option C (Source Pinning):** Requires manual title tracking, same fundamental problem

---

## Schema Design

### SQLite Database: `~/.config/code/source-registry.db`

```sql
-- Notebooks table (cache of library.json notebooks)
CREATE TABLE notebooks (
    id TEXT PRIMARY KEY,              -- Notebook ID from library
    url TEXT NOT NULL UNIQUE,         -- NotebookLM URL
    name TEXT NOT NULL,               -- Human-readable name
    source_count INTEGER DEFAULT 0,   -- Cached count
    created_at TEXT NOT NULL,         -- ISO8601 timestamp
    updated_at TEXT NOT NULL          -- ISO8601 timestamp
);

-- Sources table
CREATE TABLE sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    notebook_id TEXT NOT NULL,          -- FK to notebooks.id
    spec_id TEXT NOT NULL,              -- e.g., "SPEC-DOGFOOD-001"
    source_type TEXT NOT NULL,          -- "spec" | "task_brief" | "static" | "evidence"
    original_name TEXT NOT NULL,        -- Name used in upsert (e.g., "CURRENT_SPEC")
    notebooklm_title TEXT,              -- Title assigned by NotebookLM (nullable, discovered)
    content_hash TEXT,                  -- SHA256 of content (for change detection)
    word_count INTEGER,                 -- Approximate word count
    created_at TEXT NOT NULL,           -- ISO8601 timestamp
    last_used_at TEXT NOT NULL,         -- ISO8601 timestamp (updated on each upsert)
    status TEXT DEFAULT 'active',       -- "active" | "orphaned" | "deleted"
    FOREIGN KEY (notebook_id) REFERENCES notebooks(id) ON DELETE CASCADE,
    UNIQUE(notebook_id, spec_id, source_type)  -- One source per spec+type per notebook
);

-- Upsert history for debugging
CREATE TABLE upsert_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL,
    action TEXT NOT NULL,               -- "created" | "updated" | "deleted" | "orphaned"
    previous_title TEXT,                -- Title before operation
    new_title TEXT,                     -- Title after operation
    timestamp TEXT NOT NULL,            -- ISO8601 timestamp
    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX idx_sources_notebook ON sources(notebook_id);
CREATE INDEX idx_sources_spec ON sources(spec_id);
CREATE INDEX idx_sources_status ON sources(status);
CREATE INDEX idx_sources_last_used ON sources(last_used_at);
```

---

## Integration Points

### 1. notebooklm-client (Recommended)

**Location:** `~/notebooklm-client/src/sources/source-registry.ts`

**Flow:**
```
handleUpsertSource()
  ├── 1. Registry lookup by (notebook_id, spec_id, source_type)
  ├── 2. If found: delete existing source by stored notebooklm_title
  ├── 3. Add new source
  ├── 4. Capture new title from response
  └── 5. Update registry with new title + timestamp
```

**Advantages:**
- Single integration point for all clients (CLI, TUI, MCP)
- Access to browser-extracted title after processing
- Can track all source operations

### 2. codex-rs/tui (Alternative)

**Location:** `~/code/codex-rs/tui/src/source_registry.rs`

**Flow:**
```
upsert_source_blocking()
  ├── 1. Registry lookup
  ├── 2. Call delete-source API if found
  ├── 3. Call upsert-source API
  └── 4. Update registry
```

**Disadvantages:**
- Requires Rust SQLite library (rusqlite)
- Only tracks TUI operations, not CLI usage

---

## API Extensions

### New notebooklm-client Endpoints

```yaml
# Get registry stats
GET /api/sources/registry
Response:
  success: true
  data:
    total_tracked: 15
    active: 10
    orphaned: 3
    by_notebook:
      - notebook_id: "code-project-docs"
        source_count: 10
        specs: ["SPEC-DOGFOOD-001", "SPEC-SOURCE-MGMT"]

# Get sources for a spec
GET /api/sources/registry/:spec_id
Response:
  success: true
  data:
    spec_id: "SPEC-DOGFOOD-001"
    sources:
      - source_type: "spec"
        notebooklm_title: "Golden Path Dogfooding..."
        last_used_at: "2025-12-27T12:00:00Z"
      - source_type: "task_brief"
        notebooklm_title: "SPEC-DOGFOOD-001: Golden..."
        last_used_at: "2025-12-27T12:00:00Z"

# Cleanup orphaned sources
POST /api/sources/cleanup
Body:
  max_age_days: 7           # Delete sources not used in N days
  dry_run: true             # Preview only
Response:
  success: true
  data:
    mode: "preview"
    sources_to_delete: 3
    specs_affected: ["SPEC-OLD-001"]

# Manual cleanup by spec
DELETE /api/sources/registry/:spec_id
Response:
  success: true
  data:
    deleted_count: 2
    deleted_titles: ["Golden Path...", "SPEC-DOGFOOD..."]
```

---

## CLI Commands

```bash
# View registry stats
notebooklm registry stats

# List sources for a spec
notebooklm registry list SPEC-DOGFOOD-001

# Preview cleanup
notebooklm registry cleanup --dry-run --max-age 7

# Execute cleanup
notebooklm registry cleanup --max-age 7 --confirm

# Delete all sources for a spec
notebooklm registry delete SPEC-OLD-001 --confirm
```

---

## Implementation Plan

### Phase 1: Schema & Registry Module (S34)

1. Create SQLite schema in `~/.config/code/source-registry.db`
2. Implement `SourceRegistry` class in notebooklm-client
3. Add registry lookup/update to `handleUpsertSource`

### Phase 2: Delete-Before-Upsert (S34)

1. Before upserting, query registry for existing source
2. If found, call delete-source API
3. Track success/failure in upsert_log

### Phase 3: API & CLI (S35)

1. Add `/api/sources/registry/*` endpoints
2. Add `notebooklm registry` CLI commands
3. Add cleanup cron/hook suggestion

---

## Acceptance Criteria

| ID | Criterion | Verification |
|----|-----------|--------------|
| A1 | No orphan sources after 10 runs | Run `/speckit.auto` 10x, `list-sources` shows constant count |
| A2 | Cleanup command exists | `notebooklm registry cleanup --dry-run` shows stats |
| A3 | Source count tracked | `notebooklm registry stats` shows per-spec counts |
| A4 | Manual cleanup works | `notebooklm registry delete SPEC-X` removes sources |
| A5 | Registry persists | Restart service, registry data intact |

---

## Open Questions

1. **Where to store registry?**
   - `~/.config/code/source-registry.db` (spec-kit scope)
   - `~/.local/share/notebooklm-mcp/source-registry.db` (notebooklm scope)
   - **Decision:** Use notebooklm-mcp scope since registry is NLM-specific

2. **Title discovery timing?**
   - After upsert, NotebookLM may still be "processing"
   - Need to wait for ready state before extracting title
   - **Finding:** Current `handleUpsertSource` doesn't return NLM-generated title
   - **Requires:** Add `listSources()` call after `addSource()` to discover title

3. **Handle static sources?**
   - Some sources (templates, docs) should never be deleted
   - Mark as `source_type: "static"` in registry

4. **Delete index bug?**
   - Line 360 in sources.ts: `deleteSource(notebookUrl, existingSource.index - 1, ...)`
   - Source indices are 1-based, but this subtracts 1
   - **Needs verification:** May cause off-by-one errors

---

## Known Issues Found (S34)

### Bug 1: CLI delete-source "Invalid JSON response" - FIXED

**Root cause:** `ServiceClient.request()` didn't set `Content-Length` header.

**Fix applied (service-client.ts:627-631):**
```typescript
if (body) {
  const bodyStr = JSON.stringify(body);
  req.setHeader("Content-Length", Buffer.byteLength(bodyStr).toString());
  req.write(bodyStr);
}
```

**Status:** ✅ Verified working - delete-source CLI returns valid JSON

### Bug 2: Upsert doesn't return NLM-generated title - FIXED

**Fix applied (sources.ts:389-418):**
- Added `listSources()` call after adding source
- Returns `nlmTitle` field with actual NotebookLM-assigned title
- Falls back to last source if name matching fails

**Status:** ✅ Verified working - upsert returns `nlmTitle` field

**Minor refinement needed:** Title matching uses substring, but NLM transforms names significantly:
- Input: "S34_TEST_SOURCE"
- NLM output: "S34 Registry Validation Protocol Test Source"
- Matching should use word-based fuzzy matching (like upsert delete logic)

### Bug 3: Off-by-one in upsert delete - FIXED

**Fix applied (sources.ts:361):**
```typescript
// Before (wrong): existingSource.index - 1
// After (correct): existingSource.index
await sourceManager.deleteSource(notebookUrl, existingSource.index, {...})
```

**Status:** ✅ Verified working - delete uses correct 1-based index

---

## Related Documents

- [HANDOFF.md](../HANDOFF.md) - Session 34 context
- [SPEC-DOGFOOD-001](../SPEC-DOGFOOD-001/) - Source-based Tier2 validation
- [stage0_adapters.rs](../../codex-rs/tui/src/stage0_adapters.rs) - Tier2HttpAdapter
