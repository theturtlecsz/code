# P97 Session Handoff — SPEC-KIT-103 Librarian v1

**Session:** P97
**Date:** 2025-12-02
**Focus:** Memory corpus quality engine
**Lineage:** P89 → P90 → P91 → P92 → P93 → P94 → P95 → P96 → **P97**

---

## Completed Work

### Part 1: SPEC-KIT-103 Specification
- Created `docs/SPEC-KIT-103/spec.md` with full design
- Defined MemoryType enum (Pattern, Decision, Problem, Insight, Exception, Reference, Unknown)
- Documented template structure (CONTEXT/REASONING/OUTCOME/TAGS)
- Specified CausalRelation types (Causes, Blocks, Enables, RelatesTo)

### Task 1: Memory Classifier + Templater (`codex-rs/stage0/src/librarian/`)
- `classifier.rs`: Heuristic-based classification with 51 tests
  - Signal patterns for each memory type
  - Confidence scoring with thresholds
  - `classify_memory()` returns `ClassificationResult`
- `templater.rs`: Content restructuring
  - `apply_template()` converts to canonical format
  - Preserves already-structured content
  - Type-specific extraction strategies

### Task 2: CLI Command (`/stage0.librarian`)
- `codex-rs/tui/src/chatwidget/spec_kit/commands/librarian.rs`
- Subcommands: `sweep`, `status`, `help`
- Flags: `--dry-run`, `--domains=`, `--limit=`, `--min-importance=`, `--json-report`
- Registered as command #38 with alias "librarian"

### Task 3: Causal Inference Stub
- `causal.rs`: Basic relationship detection
  - Pattern matching for CAUSES/BLOCKS/ENABLES language
  - `infer_relationships()` for edge creation
  - Content overlap scoring

### Part 3: Telemetry
- LibrarianSweepRun event logged with full metrics
- tracing::info! target "stage0.librarian"

---

## Test Results

```
cargo test -p codex-stage0 -- librarian
→ 51 passed; 0 failed

cargo test -p codex-tui --lib command_registry
→ 16 passed; 0 failed

~/code/build-fast.sh
→ ✅ Build successful
```

---

## Files Changed

```
docs/SPEC-KIT-103/spec.md                           (new)
docs/HANDOFF-P97.md                                 (new)
codex-rs/stage0/src/librarian/mod.rs                (new)
codex-rs/stage0/src/librarian/classifier.rs         (new)
codex-rs/stage0/src/librarian/templater.rs          (new)
codex-rs/stage0/src/librarian/causal.rs             (new)
codex-rs/stage0/src/lib.rs                          (modified - added librarian export)
codex-rs/tui/src/chatwidget/spec_kit/commands/mod.rs (modified)
codex-rs/tui/src/chatwidget/spec_kit/commands/librarian.rs (new)
codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs (modified)
```

---

## Design Decisions Confirmed

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Classification | Heuristic-first, LLM opt-in | Fast, deterministic, no API costs |
| Template enforcement | Permissive with warnings | Don't block on malformed legacy |
| Relationship storage | local-memory MCP | Use existing API |
| Batch size | 100 default | Balance memory vs round trips |
| Confidence threshold | 0.7 for auto-apply | Conservative default |

---

## Next Steps (P98+)

### P98 Goal: Wire Librarian to Live Corpus (Safely)

Turn Librarian from a **simulator** into a **real maintenance tool**:
- Read real memories via MCP
- Propose and optionally **apply** retyping/templating changes
- Create causal edges
- Without redesigning algorithms

### P98 Hard Constraints

| Constraint | Rationale |
|------------|-----------|
| No algorithm changes | `classifier.rs`, `templater.rs`, `causal.rs` frozen for P98 |
| Dry-run default | Without `--apply`, no `update_memory` or `create_relationship` calls |
| Domain/limit enforced | Blast-radius controls prevent accidental corpus-wide changes |
| Idempotent | Skip memories already matching suggested type/template |

### P98 Tasks

1. **MCP Query Integration** — Replace samples with `list_memories` + `get_memory`
2. **Apply Mode** — `--apply` flag enables `update_memory` calls
3. **Causal Edges** — `--apply-causal` creates relationships via MCP
4. **JSON Report Schema** — Standardize output for CI and future metrics

### Future (SPEC-KIT-104+)
- Learned weight tuning using classification results
- Auto-reconciliation suggestions for conflicts
- Full corpus migration with metrics baseline
- LLM-enhanced classification for ambiguous cases

---

## P98 Continuation Prompt

See `docs/PROMPT-P98-LIBRARIAN.md` for the full prompt with:
- 6 detailed tasks with code snippets
- SQLite schema for audit trail
- Test plan and success criteria
- Commit strategy

**Design Decisions Confirmed**:
| Decision | Choice |
|----------|--------|
| Rollback | Audit trail only (SQLite tables, no `--rollback`) |
| MCP Client | Hybrid trait (stage0 trait, TUI impl, mock tests) |
| Progress | Both modes (quiet default, `--verbose` streaming) |
| Optional | None (keep minimal) |

Quick start:

```
**Ultrathink** P98 Session: SPEC-KIT-103 Librarian v1 Integration

Read docs/PROMPT-P98-LIBRARIAN.md for full task breakdown.

**Goal**: Wire Librarian v1 to live local-memory corpus

Tasks (6):
1. LocalMemoryClient trait in stage0
2. MCP implementation in TUI
3. Audit trail SQLite tables
4. --apply mode for memory writes
5. --verbose progress + output modes
6. RelationshipsClient for causal edges

Commit after each task.

**ultrathink**
```

---

## Metrics

- **Duration:** ~45 minutes
- **Files created:** 6
- **Files modified:** 3
- **Tests added:** 51 (librarian) + 0 (registry updates)
- **Lines of code:** ~1200
