# P98 Session Continuation Prompt

Copy this to start the next session:

---

**Ultrathink** P98 Session: SPEC-KIT-103 Librarian v1 Integration

Read docs/HANDOFF-P97.md for implementation context.

**Primary Goal**: Wire Librarian v1 to live local-memory corpus with safety guardrails

## Session Lineage
P89 → P90 → P91 → P92 → P93 → P94 → P95 → P96 → P97 → **P98** (MCP Integration)

## Constraints (Hard Rules)
1. **No algorithm changes** — `classifier.rs`, `templater.rs`, `causal.rs` are frozen
2. **Dry-run default** — Without explicit `--apply`, no writes occur
3. **Domain/limit enforcement** — All operations must respect filters
4. **Idempotent** — Skip memories already matching suggested type/template

## Task Breakdown

### Task 1: MCP Query Integration
- Introduce `LocalMemoryClient` wrapper for Stage 0
- Replace sample data with `list_memories` + `get_memory` calls
- Respect `--domains=`, `--limit=` filters
- Graceful error handling (skip failed memories, don't abort sweep)

### Task 2: Apply Mode for Memory Changes
- Add `--apply` flag to enable writes
- Call `update_memory` for accepted changes only when `--apply` present
- Add `--max-changes=N` safety cap
- Report applied changes in JSON output

### Task 3: Causal Edge Creation
- Add `RelationshipsClient` wrapper for MCP relationships API
- Gate behind `--apply-causal` (separate from `--apply`)
- Create CAUSES/BLOCKS/RELATES_TO edges via `create_relationship`
- Avoid duplicate edges; add-only behavior

### Task 4: JSON Report Schema
```json
{
  "run_id": "sweep-YYYYMMDD-NNN",
  "timestamp": "ISO8601",
  "config": { "domains": [], "limit": N, "apply": false },
  "summary": { "scanned": N, "retyped": N, "edges_created": N },
  "changes": [{ "id": "", "old_type": "", "new_type": "", "applied": false }],
  "edges": [{ "from": "", "to": "", "kind": "", "applied": false }]
}
```

## Non-Goals (Defer to SPEC-KIT-104+)
- Classifier threshold tuning
- Template format changes
- Causal algorithm improvements
- Eval metrics integration
- Full corpus migration

## Test Plan
```bash
# Read-only validation (should work immediately)
/stage0.librarian sweep --dry-run --domains=spec-kit --limit=10 --json-report

# Apply mode (after Task 2)
/stage0.librarian sweep --apply --domains=spec-kit --limit=5 --max-changes=3

# Causal edges (after Task 3)
/stage0.librarian sweep --apply --apply-causal --domains=spec-kit --limit=5
```

Start by reading `codex-rs/stage0/src/librarian/mod.rs` and the sweep implementation in `tui/src/chatwidget/spec_kit/commands/librarian.rs`. Commit after each task.

**ultrathink**
