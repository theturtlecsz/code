# P98 Session Continuation Prompt

Copy this to start the next session:

---

**Ultrathink** P98 Session: SPEC-KIT-103 Librarian v1 Integration

Read `docs/SPEC-KIT-103/spec.md` for implementation context.

**Primary Goal**: Wire Librarian v1 to live local-memory corpus with safety guardrails

## Session Lineage
P89 → P90 → P91 → P92 → P93 → P94 → P95 → P96 → P97 → **P98** (MCP Integration)

---

## Design Decisions (Confirmed)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Rollback | **Audit trail only** | SQLite tables capture changes; no `--rollback` command in P98 |
| MCP Client | **Hybrid trait** | `LocalMemoryClient` trait in stage0, MCP impl in TUI, mock for tests |
| Progress | **Both modes** | Default quiet + final summary; `--verbose` streams to stderr |
| Optional features | **None** | Keep P98 minimal; defer backup export, domain auto-detect, preview UI |

---

## Hard Constraints

1. **No algorithm changes** — `classifier.rs`, `templater.rs`, `causal.rs` are FROZEN
2. **Dry-run default** — Without explicit `--apply`, no writes occur
3. **Domain/limit enforcement** — All operations must respect filters
4. **Idempotent** — Skip memories already matching suggested type/template
5. **JSON to stdout, progress to stderr** — Clean separation for tooling

---

## Task Breakdown

### Task 1: LocalMemoryClient Trait (Hybrid Architecture)

**Location**: `codex-rs/stage0/src/librarian/client.rs` (new)

```rust
pub trait LocalMemoryClient: Send + Sync {
    fn list_memories(&self, params: ListParams) -> Result<Vec<MemoryMeta>, Error>;
    fn get_memory(&self, id: &str) -> Result<Memory, Error>;
    fn update_memory(&self, id: &str, change: MemoryChange) -> Result<(), Error>;
}

pub struct ListParams {
    pub domains: Vec<String>,
    pub limit: usize,
    pub min_importance: Option<i32>,
}
```

**Tasks**:
- Define trait + types in stage0
- Create `MockLocalMemoryClient` for tests
- Update Librarian sweep to accept `&dyn LocalMemoryClient`
- Remove sample data, wire to trait

### Task 2: MCP Implementation in TUI

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/mcp_clients/local_memory.rs` (new)

```rust
pub struct LocalMemoryMcpClient { /* MCP handle */ }

impl LocalMemoryClient for LocalMemoryMcpClient {
    fn list_memories(&self, params: ListParams) -> Result<Vec<MemoryMeta>, Error> {
        // Call mcp__local-memory__search with domain filter
    }
    // ...
}
```

**Tasks**:
- Implement `LocalMemoryMcpClient` wrapping MCP tools
- Wire into `/stage0.librarian sweep` command
- Handle pagination for large result sets
- Graceful error handling (skip failed memories, don't abort)

### Task 3: Audit Trail (SQLite Tables)

**Location**: Extend existing Stage0 overlay DB

```sql
-- Sweep metadata
CREATE TABLE librarian_sweeps (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id          TEXT NOT NULL UNIQUE,
    started_at      TEXT NOT NULL,
    finished_at     TEXT,
    args_json       TEXT NOT NULL,
    stats_json      TEXT,
    status          TEXT DEFAULT 'running'  -- running/completed/failed
);

-- Per-memory changes
CREATE TABLE librarian_changes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sweep_id        INTEGER NOT NULL REFERENCES librarian_sweeps(id),
    memory_id       TEXT NOT NULL,
    change_type     TEXT NOT NULL,  -- retype/template/both
    old_type        TEXT,
    new_type        TEXT,
    old_content     TEXT,
    new_content     TEXT,
    confidence      REAL,
    applied         INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL
);

-- Causal edges created
CREATE TABLE librarian_edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sweep_id        INTEGER NOT NULL REFERENCES librarian_sweeps(id),
    from_id         TEXT NOT NULL,
    to_id           TEXT NOT NULL,
    relation_type   TEXT NOT NULL,
    reason          TEXT,
    applied         INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL
);
```

**Tasks**:
- Add migration for new tables
- Log sweep start/end to `librarian_sweeps`
- Log each change to `librarian_changes` (applied=0 for dry-run, 1 for apply)
- Log edges to `librarian_edges`
- Display sweep-id in CLI output

### Task 4: Apply Mode Implementation

**Flags**:
- `--apply` — Enable `update_memory` calls for type/template changes
- `--apply-causal` — Enable `create_relationship` calls for edges
- `--max-changes=N` — Safety cap on writes (default: unlimited)
- `--max-edges=N` — Safety cap on edge creation (default: unlimited)

**Behavior**:
```
Without --apply:     Log to SQLite (applied=0), no MCP writes
With --apply:        Log to SQLite (applied=1), call update_memory
With --apply-causal: Also create relationship edges
```

**Tasks**:
- Add flags to CLI parser
- Gate `update_memory` behind `--apply`
- Gate `create_relationship` behind `--apply-causal`
- Enforce `--max-changes` / `--max-edges` limits
- Skip already-correct memories (idempotency)

### Task 5: Progress & Output Modes

**Default (quiet)**:
```
Librarian sweep completed
  sweep-id: LRB-20251202-001
  domains: spec-kit
  scanned: 500 memories
  changed: 42 memories (dry-run)
  causal edges: 17 proposed
```

**With `--verbose`** (to stderr):
```
[Librarian] Scanning memories in domain: spec-kit
[Librarian] Processed 50/500 (10%)...
[Librarian] Processed 100/500 (20%)...
...
```

**With `--json-report`** (to stdout):
```json
{
  "run_id": "LRB-20251202-001",
  "timestamp": "2025-12-02T03:00:00Z",
  "config": { "domains": ["spec-kit"], "limit": 500, "apply": false },
  "summary": { "scanned": 500, "retyped": 42, "templated": 38, "edges": 17 },
  "changes": [...],
  "edges": [...]
}
```

**Tasks**:
- Add `--verbose` flag for streaming progress
- Progress to stderr, JSON to stdout
- Generate run_id format: `LRB-YYYYMMDD-NNN`
- Final summary always printed (to stderr if --json-report)

### Task 6: RelationshipsClient for Causal Edges

**Location**: `codex-rs/stage0/src/librarian/relationships.rs` (new)

```rust
pub trait RelationshipsClient: Send + Sync {
    fn create_relationship(&self, edge: CausalEdge) -> Result<(), Error>;
    fn relationship_exists(&self, from: &str, to: &str, kind: &str) -> Result<bool, Error>;
}
```

**Tasks**:
- Define trait in stage0
- Implement MCP wrapper in TUI (`mcp__local-memory__relationships`)
- Check for existing edges before creating (avoid duplicates)
- Wire into sweep when `--apply-causal` is set

---

## Test Plan

```bash
# Task 1-2: Read-only validation with real MCP
/stage0.librarian sweep --dry-run --domains=spec-kit --limit=10 --json-report

# Task 3: Verify SQLite audit trail
sqlite3 ~/.code/stage0_overlay.db "SELECT * FROM librarian_sweeps;"

# Task 4: Apply mode (after audit trail works)
/stage0.librarian sweep --apply --domains=spec-kit --limit=5 --max-changes=3

# Task 5: Verbose mode
/stage0.librarian sweep --dry-run --verbose --domains=spec-kit --limit=50

# Task 6: Causal edges
/stage0.librarian sweep --apply --apply-causal --domains=spec-kit --limit=10

# Full pipeline test
/stage0.librarian sweep --apply --apply-causal --verbose --domains=spec-kit --limit=100 --json-report 2>/tmp/progress.log
```

---

## Non-Goals (Defer)

- Classifier/templater/causal algorithm changes → SPEC-KIT-104
- `--rollback` command → Future session
- Memory backup export → Future session
- Domain auto-detection → Future session
- Interactive diff preview → Future session
- Full corpus migration → After P98 proves safe

---

## File Changes Expected

```
codex-rs/stage0/src/librarian/
├── mod.rs              (update: re-exports)
├── client.rs           (NEW: LocalMemoryClient trait)
├── relationships.rs    (NEW: RelationshipsClient trait)
└── audit.rs            (NEW: SQLite audit trail)

codex-rs/tui/src/chatwidget/spec_kit/
├── mcp_clients/
│   ├── mod.rs          (NEW)
│   └── local_memory.rs (NEW: LocalMemoryMcpClient)
└── commands/
    └── librarian.rs    (update: wire new clients, add flags)

codex-rs/stage0/migrations/
└── NNNN_librarian_audit.sql (NEW: audit tables)
```

---

## Success Criteria

1. `/stage0.librarian sweep --dry-run` works against real corpus
2. Audit trail populated in SQLite for all runs
3. `--apply` mode successfully updates memories
4. `--apply-causal` mode creates relationship edges
5. `--verbose` shows streaming progress
6. `--json-report` produces valid JSON to stdout
7. All existing Stage0 tests pass (170+)
8. New Librarian integration tests with mocks

---

## Commit Strategy

Commit after each task:
1. `feat(stage0): add LocalMemoryClient trait for Librarian`
2. `feat(tui): implement LocalMemoryMcpClient for MCP integration`
3. `feat(stage0): add Librarian audit trail SQLite tables`
4. `feat(tui): add --apply mode to /stage0.librarian`
5. `feat(tui): add --verbose progress and output modes`
6. `feat(stage0): add RelationshipsClient for causal edges`

---

Start by reading:
1. `codex-rs/stage0/src/librarian/mod.rs` — Current sweep implementation
2. `codex-rs/tui/src/chatwidget/spec_kit/commands/librarian.rs` — CLI handler
3. `codex-rs/stage0/src/overlay_db.rs` — Existing SQLite patterns

Commit after each task. Run tests frequently.

**ultrathink**
