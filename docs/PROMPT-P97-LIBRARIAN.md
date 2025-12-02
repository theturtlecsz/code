# P97 Session Continuation Prompt

**Ultrathink** P97 Session: SPEC-KIT-103 Librarian v1 (Design + Partial Implementation)

Read docs/HANDOFF-P96.md for P95 completion context.

**Primary Goal**: Improve memory corpus quality to make existing Stage 0 / constitution infrastructure more effective

## Session Lineage
P89 (Data Model) → P90 (TASK_BRIEF + Tier-2) → P91 (Conflict Detection) → P92 (Block + Cache) → P93 (Vision Q&A) → P94 (Drift Detection) → P95 (Constitution-Aware Refinement) → P96 (Context Freeze) → **P97** (Librarian v1)

## Problem Statement

The local-memory corpus has quality issues that limit the effectiveness of already-built infrastructure:
- ~1.1k memories, mostly unstructured
- Importance saturation (too many high-priority items)
- Almost all relationships tagged as `similar`
- Missing agent/type/timestamp metadata
- No template enforcement on legacy content

Stage 0, Divine Truth, alignment checks, and exception handling all work—but they work on noisy data.

## P97 Scope: Design + Partial Implementation

### Part 1: Librarian MVP Specification

Create `docs/SPEC-KIT-103/spec.md` with:

**Goals:**
- Auto-structure legacy memories into existing templates
- Assign MemoryType classification (PATTERN, DECISION, PROBLEM, INSIGHT, etc.)
- Basic causal relationship inference (CAUSES, BLOCKS, RELATES_TO)
- Dry-run mode for safe iteration

**Non-Goals:**
- Learned weight tuning (SPEC-KIT-104)
- Auto-reconciliation suggestions (SPEC-KIT-106)
- Importance re-scoring (separate pass)

**Inputs/Outputs:**
- Input: Current local-memory rows + overlay metadata
- Output: Updated memories, new relationship edges, diff/report format

### Part 2: Core Implementation (2-3 Tasks)

#### Task 1: Memory Classifier + Templater
**Files**: `codex-rs/stage0/src/librarian/` (new module)

```rust
pub enum MemoryType {
    Pattern,    // Recurring solutions
    Decision,   // Architectural choices with rationale
    Problem,    // Issues encountered + resolutions
    Insight,    // Observations from execution
    Exception,  // Constitution exceptions (links to P95)
    Reference,  // External docs/links
    Unknown,    // Unclassifiable (flag for review)
}
```

Implement:
- `classify_memory(content: &str) -> MemoryType` - Heuristic + optional LLM classification
- `apply_template(memory: &Memory, mem_type: MemoryType) -> TemplatedMemory` - Reformat to CONTEXT/REASONING/OUTCOME/TAGS structure
- Write changes back via local-memory MCP with dry-run support

#### Task 2: Librarian CLI Skeleton
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/librarian.rs` (new)

Command: `/stage0.librarian sweep [flags]`

Flags:
- `--dry-run`: Preview changes without writing
- `--domains=`: Filter by domain (comma-separated)
- `--limit=N`: Process max N memories
- `--json-report`: Output diff as JSON for CI
- `--min-importance=N`: Only process memories >= importance

Wire into existing command registry.

#### Task 3: Minimal Causal Inference Stub
**Files**: `codex-rs/stage0/src/librarian/causal.rs`

Simple relationship extraction:
- Parse memory content for causal language ("caused", "blocked", "enabled", "led to")
- Extract entity pairs and relationship type
- Store edges via local-memory relationships MCP
- No complex ranking—just prove the pipe works

Relationship types:
```rust
pub enum CausalRelation {
    Causes,     // X directly caused Y
    Blocks,     // X prevents Y
    Enables,    // X makes Y possible
    RelatesTo,  // Weaker semantic connection
}
```

### Part 3: Telemetry Hooks

Add events for future SPEC-KIT-104 integration:
- `LibrarianSweepRun` - timestamp, memories_processed, dry_run, domains
- `MemoryRetyped` - memory_id, old_type, new_type, confidence
- `CausalEdgeInferred` - source_id, target_id, relation, confidence

## Design Decisions to Confirm

| Decision | Options | Recommendation |
|----------|---------|----------------|
| Classification method | Heuristic-only vs LLM-assisted | Start heuristic, LLM opt-in |
| Template enforcement | Strict vs permissive | Permissive with warnings |
| Relationship storage | local-memory MCP vs overlay DB | local-memory (existing API) |
| Batch size | Per-memory vs chunked | Chunked (100 per batch) |

## Files to Create/Modify

```
docs/SPEC-KIT-103/
├── spec.md                    # MVP specification
└── evidence/                  # Test outputs

codex-rs/stage0/src/
├── lib.rs                     # Export librarian module
└── librarian/
    ├── mod.rs                 # Module root
    ├── classifier.rs          # MemoryType classification
    ├── templater.rs           # Template application
    └── causal.rs              # Relationship inference

codex-rs/tui/src/chatwidget/spec_kit/commands/
├── mod.rs                     # Register librarian command
└── librarian.rs               # /stage0.librarian implementation
```

## Tests

```bash
# Stage0 librarian tests
cd codex-rs && cargo test -p codex-stage0 -- librarian

# TUI command registry
cargo test -p codex-tui --lib command_registry

# Build verification
~/code/build-fast.sh
```

## Out of Scope (Future Sessions)

- Importance re-scoring algorithm (SPEC-KIT-104)
- LLM-based relationship explanation (SPEC-KIT-104)
- Auto-fix suggestions for conflicts (SPEC-KIT-106)
- Full corpus migration (requires baseline metrics first)

## Success Criteria

1. `docs/SPEC-KIT-103/spec.md` exists with clear goals/non-goals
2. `/stage0.librarian sweep --dry-run` produces valid JSON diff
3. At least one memory successfully reclassified in tests
4. Causal inference stub creates at least one edge
5. All existing Stage0 tests still pass (170+)

---

## Quick Start

```bash
# Read context
cat docs/HANDOFF-P96.md

# Check current memory state (optional)
# Use local-memory MCP to query corpus stats

# Begin implementation
# Task 1 → Task 2 → Task 3, commit after each
```

**ultrathink**
