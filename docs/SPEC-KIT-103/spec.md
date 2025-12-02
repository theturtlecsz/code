# SPEC-KIT-103: Librarian v1 — Memory Corpus Quality Engine

**Status:** Implementation
**Priority:** High
**Session:** P97
**Depends On:** SPEC-KIT-102 (Stage 0 Overlay), SPEC-KIT-105 (Constitution)

---

## 1. Problem Statement

The local-memory corpus has accumulated quality issues that limit the effectiveness of already-built infrastructure:

| Issue | Impact |
|-------|--------|
| ~1.1k memories, mostly unstructured | Stage 0 DCC retrieval returns noisy context |
| Importance saturation (too many priority 8-10) | Dynamic scoring loses discriminative power |
| Almost all relationships tagged as `similar` | Causal inference provides no value |
| Missing agent/type/timestamp metadata | Guardian classification falls back to Unknown |
| No template enforcement on legacy content | TASK_BRIEF structure degrades |

Stage 0, Divine Truth, alignment checks, and exception handling all work—but they work on noisy data.

## 2. Goals

1. **Auto-structure legacy memories** into CONTEXT/REASONING/OUTCOME/TAGS templates
2. **Assign MemoryType classification** (PATTERN, DECISION, PROBLEM, INSIGHT, etc.)
3. **Infer basic causal relationships** beyond `similar` (CAUSES, BLOCKS, ENABLES)
4. **Provide dry-run mode** for safe iteration without modifying corpus
5. **Generate actionable reports** (JSON for CI, human-readable summary)

## 3. Non-Goals

- **Learned weight tuning** (SPEC-KIT-104) — Librarian classifies, doesn't learn weights
- **Auto-reconciliation suggestions** (SPEC-KIT-106) — No AI-generated conflict fixes
- **Importance re-scoring** — Separate pass after classification
- **Full corpus migration** — MVP processes incrementally, not all at once
- **LLM-mandatory classification** — Heuristics first, LLM opt-in

## 4. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    /stage0.librarian sweep                      │
├─────────────────────────────────────────────────────────────────┤
│  Flags: --dry-run  --domains=  --limit=N  --json-report         │
│         --min-importance=N                                       │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │   Memory Classifier          │
          │   (Heuristic + LLM opt-in)   │
          │   → MemoryType enum          │
          └──────────────┬───────────────┘
                         │
          ┌──────────────┴───────────────┐
          ▼                              ▼
┌─────────────────────┐      ┌─────────────────────┐
│   Templater         │      │   Causal Inference  │
│   → CONTEXT/        │      │   → CAUSES/BLOCKS/  │
│      REASONING/     │      │      ENABLES edges  │
│      OUTCOME/TAGS   │      │                     │
└─────────┬───────────┘      └──────────┬──────────┘
          │                             │
          └──────────────┬──────────────┘
                         ▼
              ┌─────────────────────┐
              │   local-memory MCP  │
              │   (write back)      │
              └─────────────────────┘
```

## 5. Data Model

### 5.1 MemoryType Enum

```rust
pub enum MemoryType {
    Pattern,    // Recurring solutions, established approaches
    Decision,   // Architectural choices with rationale
    Problem,    // Issues encountered + resolutions
    Insight,    // Observations from execution (learnings)
    Exception,  // Constitution exceptions (links to P95)
    Reference,  // External docs/links
    Unknown,    // Unclassifiable (flag for review)
}
```

### 5.2 Template Structure

All memories get restructured to this canonical format:

```markdown
## CONTEXT
<when/where this applies>

## REASONING
<why this exists, rationale>

## OUTCOME
<result, recommendation, or resolution>

## TAGS
- type:<MemoryType>
- component:<if applicable>
- spec:<SPEC-ID if linked>
```

### 5.3 CausalRelation Enum

```rust
pub enum CausalRelation {
    Causes,     // X directly caused Y
    Blocks,     // X prevents Y
    Enables,    // X makes Y possible
    RelatesTo,  // Weaker semantic connection (existing `similar`)
}
```

## 6. Classification Heuristics

The classifier uses keyword/pattern matching before LLM fallback:

| MemoryType | Primary Signals |
|------------|-----------------|
| Pattern | "pattern:", "recurring", "always do", "standard approach" |
| Decision | "decision:", "chose", "decided", "because we", "trade-off" |
| Problem | "problem:", "issue:", "bug:", "error:", "failed", "broke" |
| Insight | "learned:", "realized:", "observed:", "TIL:", "note:" |
| Exception | "exception:", "exemption:", "override:", ConstitutionType::Exception |
| Reference | starts with URL, "see:", "ref:", "docs:" |
| Unknown | No strong signals (flagged for manual review) |

Confidence threshold: Heuristic needs 2+ signals or strong single match.

## 7. API Surface

### 7.1 Rust Module: `codex_stage0::librarian`

```rust
// Classification
pub fn classify_memory(content: &str) -> (MemoryType, f32); // type + confidence

// Templating
pub fn apply_template(content: &str, mem_type: MemoryType) -> String;

// Causal inference
pub fn infer_relationships(content: &str, other_ids: &[String]) -> Vec<CausalEdge>;

// Sweep orchestration
pub async fn sweep_memories(
    client: &impl LocalMemoryClient,
    config: SweepConfig,
) -> SweepResult;
```

### 7.2 CLI Command: `/stage0.librarian`

```
/stage0.librarian sweep [flags]

Flags:
  --dry-run           Preview changes without writing
  --domains=<list>    Filter by domain (comma-separated)
  --limit=<N>         Process max N memories
  --json-report       Output diff as JSON for CI
  --min-importance=<N> Only process memories >= importance
```

### 7.3 Telemetry Events

| Event | Fields | Purpose |
|-------|--------|---------|
| LibrarianSweepRun | timestamp, memories_processed, dry_run, domains, duration_ms | Track sweep operations |
| MemoryRetyped | memory_id, old_type, new_type, confidence | Track classification changes |
| CausalEdgeInferred | source_id, target_id, relation, confidence | Track relationship discovery |

## 8. Implementation Plan

### Task 1: Memory Classifier + Templater

**Files:** `codex-rs/stage0/src/librarian/{mod,classifier,templater}.rs`

1. Define `MemoryType` enum with Display/FromStr
2. Implement `classify_memory()` with heuristic patterns
3. Implement `apply_template()` to restructure content
4. Add unit tests for classification edge cases

### Task 2: Librarian CLI Skeleton

**Files:** `codex-rs/tui/src/chatwidget/spec_kit/commands/librarian.rs`

1. Create `Stage0LibrarianCommand` implementing `SpecKitCommand`
2. Parse flags: `--dry-run`, `--domains`, `--limit`, `--json-report`, `--min-importance`
3. Wire into command registry
4. Implement dry-run output format

### Task 3: Causal Inference Stub

**Files:** `codex-rs/stage0/src/librarian/causal.rs`

1. Define `CausalRelation` enum
2. Implement `infer_relationships()` with keyword parsing
3. Store edges via local-memory MCP relationships API
4. Add tests for causal language detection

## 9. Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Classification method | Heuristic-first, LLM opt-in | Fast, deterministic, no API costs |
| Template enforcement | Permissive with warnings | Don't block on malformed legacy content |
| Relationship storage | local-memory MCP | Use existing API, no new storage |
| Batch size | Chunked (100 per batch) | Balance memory usage vs. round trips |
| Confidence threshold | 0.7 for auto-apply | Conservative default, flag borderline |

## 10. Success Criteria

1. `/stage0.librarian sweep --dry-run` produces valid JSON diff
2. At least one memory successfully reclassified in tests
3. Causal inference stub creates at least one edge
4. All existing Stage0 tests still pass (170+)
5. Build passes (`~/code/build-fast.sh`)

## 11. Testing

```bash
# Stage0 librarian tests
cd codex-rs && cargo test -p codex-stage0 -- librarian

# TUI command registry
cargo test -p codex-tui --lib command_registry

# Build verification
~/code/build-fast.sh
```

## 12. Future Work (Out of Scope)

- **SPEC-KIT-104:** Learned weight tuning using Librarian classifications
- **SPEC-KIT-106:** Auto-reconciliation suggestions for conflicts
- **Full corpus migration:** Bulk processing after metrics baseline
- **LLM-enhanced classification:** Use Claude for ambiguous cases

---

## Appendix A: Sample Sweep Output (JSON)

```json
{
  "sweep_id": "sweep-20241202-001",
  "dry_run": true,
  "config": {
    "domains": ["spec-kit"],
    "limit": 100,
    "min_importance": 7
  },
  "summary": {
    "memories_scanned": 100,
    "memories_retyped": 23,
    "memories_templated": 45,
    "causal_edges_created": 8,
    "unknown_flagged": 5
  },
  "changes": [
    {
      "memory_id": "abc123",
      "action": "retype",
      "old_type": null,
      "new_type": "Pattern",
      "confidence": 0.85
    },
    {
      "memory_id": "def456",
      "action": "causal_edge",
      "source_id": "def456",
      "target_id": "ghi789",
      "relation": "Causes",
      "confidence": 0.72
    }
  ]
}
```

## Appendix B: Session Lineage

P89 (Data Model) → P90 (TASK_BRIEF + Tier-2) → P91 (Conflict Detection) → P92 (Block + Cache) → P93 (Vision Q&A) → P94 (Drift Detection) → P95 (Constitution-Aware Refinement) → P96 (Context Freeze) → **P97** (Librarian v1)
