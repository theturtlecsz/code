# HANDOFF: Prompt E Implementation Progress

**Generated:** 2026-01-26
**Audience:** Next session (proceed to Prompt F)
**Scope:** `codex-rs` / Spec‑Kit + product‑knowledge layer (`local-memory` domain `codex-product`)

***

## TL;DR (Current State)

**Prompt E Implementation: 100% Complete**

Core implementation of Product Knowledge Lane is done and builds successfully:

* Config surface added (default OFF)
* Evidence pack types defined per ADR-003
* Retrieval + filtering logic implemented
* Lane assembly with truncation
* TASK\_BRIEF integration
* Capsule snapshotting helper function
* **Unit tests added (9 tests, all passing)**

**Remaining work:**

* Integration testing with actual `codex-product` domain data (optional)
* Proceed to Prompt F: NotebookLM pre-check + post-curation

***

## What Was Implemented (Prompt E)

### 1. Configuration (`codex-rs/stage0/src/config.rs`)

Added `ProductKnowledgeConfig` with:

* `enabled: bool` — default OFF
* `domain: String` — default "codex-product"
* `max_items: usize` — default 10
* `max_chars_per_item: usize` — default 3000
* `max_total_chars: usize` — default 10000
* `min_importance: u8` — default 8

### 2. Evidence Pack Types (`codex-rs/stage0/src/dcc.rs`)

Added per ADR-003:

* `ProductKnowledgeCandidate` — internal candidate struct
* `ProductKnowledgeEvidencePack` — capsule artifact schema
* `ProductKnowledgeQuery` — query metadata
* `ProductKnowledgeFilters` — filter parameters
* `ProductKnowledgeItem` — individual evidence item
* `ProductKnowledgeIntegrity` — SHA-256 integrity hash
* `PRODUCT_KNOWLEDGE_CANONICAL_TYPES` — allowed type values

### 3. Retrieval + Filtering Logic (`codex-rs/stage0/src/dcc.rs`)

Added functions:

* `build_product_knowledge_query()` — deterministic query builder (no LLM)
* `filter_product_knowledge_results()` — client-side filtering:
  * importance >= 8
  * must have canonical `type:*` tag
  * excludes `system:true`
* `assemble_product_knowledge_lane()` — markdown lane with truncation
* `build_product_knowledge_pack()` — construct evidence pack with integrity hash

### 4. DCC Integration (`codex-rs/stage0/src/dcc.rs`)

Updated `compile_context()`:

* Added step 12: product knowledge retrieval from `codex-product` domain
* Queries via `LocalMemoryClient` trait (CLI adapter)
* Filters and assembles lane
* Returns `product_knowledge_lane` and `product_knowledge_pack` in result

Updated `assemble_task_brief()`:

* Added optional `product_knowledge_lane: Option<&str>` parameter
* Inserts lane after Section 3 (Code Context), before Section 4

Updated `CompileContextResult`:

* Added `product_knowledge_lane: Option<String>`
* Added `product_knowledge_pack: Option<ProductKnowledgeEvidencePack>`

### 5. Stage0Result Integration (`codex-rs/stage0/src/lib.rs`)

Added to `Stage0Result`:

* `product_knowledge_pack: Option<ProductKnowledgeEvidencePack>`

Exported new types from crate root.

### 6. Capsule Snapshotting Helper (`codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs`)

Added `write_product_knowledge_to_capsule()`:

* Writes evidence pack to `mv2://<workspace>/<spec>/<run>/artifact/product_knowledge/evidence_pack.json`
* Returns logical URI on success
* Logs and returns error on failure

### 7. Unit Tests (`codex-rs/stage0/src/dcc.rs`)

Added 9 unit tests for product knowledge functionality:

* `test_filter_importance_threshold` - verifies importance >= 8 filtering
* `test_filter_requires_type_tag` - verifies canonical type tag requirement
* `test_filter_excludes_system` - verifies system:true exclusion
* `test_product_knowledge_evidence_pack_schema` - tests pack serialization and integrity hash
* `test_query_builder_deterministic` - verifies deterministic query generation
* `test_query_builder_empty_iqo_fallback` - tests fallback with empty IQO
* `test_query_builder_fully_empty_returns_wildcard` - tests wildcard fallback
* `test_assemble_lane_respects_bounds` - verifies max\_items and max\_total\_chars
* `test_canonical_types_complete` - verifies all 7 ADR-003 types are present

***

## Files Modified

| File                                                         | Changes                                                          |
| ------------------------------------------------------------ | ---------------------------------------------------------------- |
| `codex-rs/stage0/src/config.rs`                              | Added `ProductKnowledgeConfig` struct                            |
| `codex-rs/stage0/src/dcc.rs`                                 | Added types, retrieval, filtering, assembly, pack building       |
| `codex-rs/stage0/src/lib.rs`                                 | Added `product_knowledge_pack` to `Stage0Result`, exported types |
| `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs` | Added `write_product_knowledge_to_capsule()` helper              |
| `codex-rs/stage0/src/dcc.rs`                                 | Added 9 unit tests for product knowledge functionality           |

***

## Build Status

**Builds successfully** with only pre-existing warnings:

```bash
cargo build -p codex-stage0 -p codex-tui
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.85s
```

***

## Remaining Work (Next Session)

### Prompt E Status: COMPLETE

All unit tests added and passing (9 tests total).

### For Prompt F (Next Phase):

* NotebookLM pre-check against `codex-product`
* Post-curation of Tier2 outputs into `codex-product`
* Load `tmp/prompt-pack-product-knowledge/prompt-f-notebooklm-precheck.md`

### Optional Integration Testing:

* Requires `codex-product` domain with test data
* Verify lane appears in TASK\_BRIEF when enabled
* Verify evidence pack is valid JSON
* Test failure modes (local-memory unavailable)

***

## Key Design Decisions Made

1. **Separation of Concerns**: Stage0 crate returns `ProductKnowledgeEvidencePack` in result; caller (TUI/CLI) handles capsule write. This avoids dependency on memvid\_adapter in Stage0 crate.

2. **Feature Flag Default OFF**: Per ADR-003, product knowledge lane is opt-in.

3. **Deterministic Query**: Query builder uses heuristics only (no LLM calls) to ensure replay determinism.

4. **Client-side Filtering**: Filtering happens after retrieval to ensure consistent behavior regardless of local-memory version.

***

## Session Restart Prompt (Next Session)

Copy everything below the `---` into the first message of the next session:

***

Begin Prompt F implementation for NotebookLM pre-check + post-curation in `codex-rs` / Spec-Kit.

**Context:** Prompt E (Product Knowledge Lane) is 100% complete with 9 unit tests passing.

**First, read:**

* `HANDOFF.md` (this file - shows what was done)
* `docs/adr/ADR-003-product-knowledge-layer-local-memory.md` (architecture)
* `tmp/prompt-pack-product-knowledge/prompt-f-notebooklm-precheck.md` (requirements)

**Goal:**

1. **NotebookLM pre-check**: Search `codex-product` before issuing Tier2 questions to avoid redundant calls
2. **Post-curation**: Distill durable NotebookLM outputs into `codex-product` insights

**Files to reference:**

* `codex-rs/stage0/src/config.rs` - ProductKnowledgeConfig
* `codex-rs/stage0/src/dcc.rs` - Core implementation + product knowledge types
* `codex-rs/stage0/src/lib.rs` - Stage0Result
* `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs` - Integration helpers

**Build command:**

```bash
cargo build -p codex-stage0 -p codex-tui
```

**Test command:**

```bash
cargo test -p codex-stage0
```
