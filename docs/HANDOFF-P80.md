# HANDOFF-P80: Shadow Notebook Seeder V1 Complete → VectorBackend V2

## Session Summary (P80)

**Completed**: Shadow Notebook Seeder V1 implementation for SPEC-KIT-102

### Files Created/Modified

#### New Files
- `tui/src/chatwidget/spec_kit/stage0_seeding.rs` (~1100 lines)
  - `SeedKind` enum (5 artifact types)
  - `SeedArtifact`, `SeedingConfig`, `SeedingResult` types
  - `run_shadow_seeding()` main pipeline
  - 5 artifact seeders: Architecture Bible, Stack Justification, Bug Retros, Debt Landscape, Project Diary
  - 12 unit tests

#### Modified Files
- `tui/src/chatwidget/spec_kit/mod.rs` - Added `stage0_seeding` module
- `tui/src/chatwidget/spec_kit/commands/special.rs` - Added `SpecKitSeedCommand`
- `tui/src/chatwidget/spec_kit/command_registry.rs` - Registered command (31 total now)

### New Slash Command

```
/speckit.seed [--max=50] [--output=/path/to/dir]
```

Aliases: `/notebooklm-seed`

Generates 5 NotebookLM-ready Markdown files:
- `NL_ARCHITECTURE_BIBLE.md` - Design decisions and patterns from local-memory
- `NL_STACK_JUSTIFICATION.md` - Dependency rationale from Cargo.toml + memories
- `NL_BUG_RETROS_01.md` - Bug patterns from local-memory
- `NL_DEBT_LANDSCAPE.md` - TODO/FIXME/HACK comments scanned from codebase
- `NL_PROJECT_DIARY_01.md` - Chronological session/milestone entries

Default output: `evidence/notebooklm/`

### Test Results
- 12 new stage0_seeding tests: all passing
- 16 command_registry tests: all passing
- 496 total TUI tests: all passing

---

## Next Session Scope: V2 – VectorBackend + Evaluation Harness

### Design Decisions (from user input)

1. **Vector Backend Choice**: In-memory TF-IDF stub first
   - Simplest to implement in pure Rust
   - No external dependencies
   - Good enough to shake out API, scoring, and evaluation tooling
   - Can add Tantivy as "real" backend later

2. **Index Scope**: Memories only for V2
   - Code-unit indexing deferred to V2.5 "Shadow Code Brain"
   - Memories are cleanest substrate with existing overlay scoring
   - Avoids complexity of parsing/chunking code

3. **Evaluation Cases**: Both approaches
   - Hardcoded in tests for CI stability
   - JSON file in `evidence/vector_eval_cases.json` for manual/integration eval

---

## Implementation Plan for Next Session

### 1. VectorBackend Abstraction (`stage0/src/vector.rs`)

```rust
#[derive(Debug, Clone)]
pub struct VectorDocument {
    pub id: String,           // e.g. local-memory id
    pub kind: String,         // "memory" | "code" | "spec" | ...
    pub text: String,         // raw text to embed
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct VectorFilters {
    pub kinds: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ScoredVector {
    pub id: String,
    pub score: f64,
    pub kind: String,
    pub metadata: serde_json::Value,
}

#[async_trait]
pub trait VectorBackend: Send + Sync {
    async fn index_documents(&self, docs: Vec<VectorDocument>) -> Result<(), Stage0Error>;
    async fn search(
        &self,
        query_text: &str,
        filters: &VectorFilters,
        top_k: usize,
    ) -> Result<Vec<ScoredVector>, Stage0Error>;
}
```

### 2. In-Memory TF-IDF Backend (initial implementation)

Implementation pattern:
- `HashMap<String, IndexedDoc>` storage
- On `index_documents()`:
  - Normalize text (lowercase, strip punctuation)
  - Tokenize on whitespace
  - Compute per-doc term frequencies
  - Build global document frequency (df) map
- On `search()`:
  - Tokenize query same way
  - Filter by kind/tags from metadata
  - Compute TF-IDF score: `sum(tf_doc(token) * log(N / (df+1)))`
  - Return top_k sorted by score

### 3. Evaluation Harness (`stage0/src/eval.rs`)

```rust
#[derive(Debug, Clone)]
pub struct EvalCase {
    pub name: String,
    pub spec_snippet: String,
    pub expected_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EvalResult {
    pub case_name: String,
    pub precision_at_k: f64,
    pub recall_at_k: f64,
    pub hits: Vec<String>,
}

/// Built-in eval cases for unit tests
pub fn built_in_eval_cases() -> Vec<EvalCase>;

/// Run evaluation
pub async fn evaluate_backend<B: VectorBackend>(
    backend: &B,
    cases: &[EvalCase],
    filters: &VectorFilters,
    top_k: usize,
) -> Vec<EvalResult>;
```

### 4. TUI Integration

Add two new commands:

**`/stage0.index`** - Index memories into vector backend
- Uses `LocalMemoryMcpAdapter` to fetch memories
- Constructs `VectorDocument` entries
- Calls `VectorBackend::index_documents()`

**`/stage0.eval-backend`** - Run evaluation
- Loads eval cases from `evidence/vector_eval_cases.json` (or uses built-ins)
- Runs `evaluate_backend()`
- Prints results table:
```
Case                         P@10   R@10   Hits
bugs-resize                  1.00   1.00   mem-bug-resize-1, mem-bug-resize-foo
spec-architecture-stage0     0.50   0.50   mem-decide-stage0-1
```

### 5. Test Coverage

Stage0 crate tests:
- Mock VectorBackend implementation
- Index synthetic documents
- Run `evaluate_backend()` with `built_in_eval_cases()`
- Assert precision/recall calculations are correct

---

## Files to Create

```
codex-rs/stage0/src/vector.rs        # VectorBackend trait + types
codex-rs/stage0/src/eval.rs          # Evaluation harness
codex-rs/stage0/src/tfidf.rs         # In-memory TF-IDF backend (or inline in vector.rs)
evidence/vector_eval_cases.json      # JSON eval cases for manual runs
```

## Files to Modify

```
codex-rs/stage0/src/lib.rs           # Export vector and eval modules
codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs  # Add /stage0.index, /stage0.eval-backend
codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs  # Register new commands
```

---

## Out of Scope for V2

- NO changes to `compile_context` (hybrid retrieval is V3)
- NO changes to seeding to use vector DB
- NO external vector DBs (Qdrant, pgvector, etc.)
- NO code-unit indexing (V2.5)

V2 is purely about:
1. Getting the VectorBackend abstraction right
2. Having a working in-memory implementation
3. Building the evaluation harness
4. Proving end-to-end indexing works

---

## Commit Reference

This session: `feat(stage0): SPEC-KIT-102 Shadow Notebook Seeder V1`

Files committed:
- stage0_seeding.rs
- command_registry.rs updates
- special.rs updates
- mod.rs updates
