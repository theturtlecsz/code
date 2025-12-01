# P85 Session Prompt: Shadow Code Brain V1 (Code-Unit Indexing + Hybrid DCC Integration)

## Prior Session (P84) Completed
- V2.5b Hardening complete (commit pending)
- Fixed flaky `cache_ttl_respected` test with fixed timestamps
- Added `upsert_tier2_cache_at()` for deterministic TTL testing
- Extended `Stage0Complete` event with `hybrid_used` field
- Added structured tracing with `target: "stage0"` for future metrics
- Added index headers to NL_* artifacts (timestamp + git commit)
- Updated Tier2 prompt to explicitly reference NL_* artifact names
- All tests pass: 113 stage0 + 499 TUI

## P85 Scope: Shadow Code Brain V1

Give Stage 0 a view of the *actual code* so it can surface relevant functions/modules alongside memories in TASK_BRIEF and Divine Truth.

### Design Decisions (User-Selected)
- **Parser**: tree-sitter-rust (accurate AST parsing over regex heuristics)
- **Relevance text**: Simple heuristics only (no LLM calls), designed for future LLM bolt-on
- **Index scope**: codex-rs core only (`stage0/src/`, `tui/src/`, `core/src/`) - no tests/examples
- **Eval harness**: Defer to P86 - focus P85 on core indexing + DCC integration

---

## Implementation Steps

### 1. CODE-UNIT EXTRACTION MODULE
**File**: `tui/src/chatwidget/spec_kit/code_index.rs`

Add tree-sitter-rust dependency to `tui/Cargo.toml`:
```toml
[dependencies]
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
```

Define the code unit model:
```rust
pub struct CodeUnit {
    pub id: String,            // e.g. "code:tui/src/chatwidget/spec_kit/pipeline_coordinator.rs::handle_spec_auto"
    pub repo: String,          // "codex-rs"
    pub path: String,          // "tui/src/chatwidget/spec_kit/pipeline_coordinator.rs"
    pub symbol: Option<String>,// e.g. "handle_spec_auto"
    pub kind: CodeUnitKind,    // Function, Struct, Impl, Module
    pub text: String,          // snippet (definition + ~20 lines context)
    pub line_start: usize,     // for linking
}

pub enum CodeUnitKind {
    Function,
    Struct,
    Impl,
    Trait,
    Module,
}
```

Implement `CodeUnitExtractor`:
- Walk `stage0/src/`, `tui/src/`, `core/src/` for `*.rs` files
- Use tree-sitter to parse each file
- Extract: `fn`, `pub fn`, `struct`, `impl`, `trait` definitions
- Capture symbol name + surrounding context (up to 20 lines, max 500 chars)
- Generate stable IDs: `code:{relative_path}::{symbol_name}`

### 2. TF-IDF BACKEND INDEXING FOR CODE UNITS
**File**: Extend `tui/src/chatwidget/spec_kit/stage0_commands.rs` (or new `code_indexing.rs`)

Extend `/stage0.index` command:
1. After indexing memories, add code indexing step
2. Walk codebase via `CodeUnitExtractor`
3. Build `VectorDocument` for each code unit:
```rust
VectorDocument {
    id: code_unit.id.clone(),
    kind: "code".to_string(),
    text: code_unit.text.clone(),
    metadata: json!({
        "repo": code_unit.repo,
        "path": code_unit.path,
        "symbol": code_unit.symbol,
        "unit_kind": code_unit.kind.to_string(),
        "line_start": code_unit.line_start,
    }),
}
```
4. Call `VectorBackend::index_documents` with code documents
5. Add config: `vector_index.max_code_units_to_index` (default: 0 = no limit)

### 3. DCC HYBRID INTEGRATION FOR CODE LANE
**File**: `stage0/src/dcc.rs`

Add config flags to `ContextCompilerConfig`:
```rust
pub struct ContextCompilerConfig {
    // ... existing fields ...
    /// Enable code lane in TASK_BRIEF (requires indexed code units)
    pub code_lane_enabled: bool,
    /// Number of code units to include in TASK_BRIEF
    pub code_top_k: usize,  // default: 10
}
```

In `compile_context`:
1. After memory retrieval, if `code_lane_enabled` and vector backend available:
2. Query TF-IDF with `filters.kinds = ["code"]`, `top_k = code_top_k`
3. Build `CodeCandidate` for each result:
```rust
pub struct CodeCandidate {
    pub id: String,
    pub path: String,
    pub symbol: Option<String>,
    pub unit_kind: String,
    pub snippet: String,
    pub score: f32,
    pub why_relevant: String,  // heuristic-generated
}
```

Generate `why_relevant` via simple heuristics:
- "Matches spec keywords: `['Stage 0', 'speckit.auto']`"
- "File path suggests pipeline component"
- No LLM calls - deterministic only

### 4. TASK_BRIEF CODE CONTEXT SECTION
**File**: `stage0/src/dcc.rs` (in `assemble_task_brief`)

Add "Code Context" section after memories:
```markdown
## 3. Code Context

### 3.1 Key Code Units

#### Code Unit 1
- **Location:** `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` (symbol: `handle_spec_auto`)
- **Why relevant:** Matches spec keywords `["Stage 0","speckit.auto"]`; part of /speckit.auto pipeline.
- **Lines:** 145-180
> ```rust
> pub async fn handle_spec_auto(...) { ... }
> ```

### 3.2 Other Code References

- `tui/src/chatwidget/spec_kit/state.rs::SpecAutoState` – state structure for /speckit.auto
- `stage0/src/lib.rs::Stage0Engine` – core Stage 0 engine
```

### 5. SEEDER TOUCH-UP (LIGHT)
**File**: `tui/src/chatwidget/spec_kit/stage0_seeding.rs`

When generating NL_ARCHITECTURE_BIBLE and NL_BUG_RETROS:
- If a memory mentions a file path matching an indexed code unit path
- Add bullet: "Relevant code: `path::symbol`"
- Simple heuristic matching, no full Librarian logic

### 6. TESTS
**Files**: `tui/tests/code_index_tests.rs` (new)

Required tests:
1. **Extraction test**: Tiny synthetic codebase in `tests/fixtures/`, verify `CodeUnitExtractor` finds expected functions/structs
2. **TF-IDF code indexing**: In-memory backend, index code docs, verify `kind="code"` search works with filters
3. **DCC integration**: Mock LocalMemoryClient + LlmClient, pass VectorBackend with code docs, assert TASK_BRIEF includes "Code Context" section

---

## OUT OF SCOPE for P85
- Incremental indexing (keep `/stage0.index` as full rebuild)
- Metrics crate integration
- Learned routing or scoring
- Eval harness extension for code hits (defer to P86)
- LLM-generated "why relevant" explanations
- Indexing tests/ or examples/

---

## Success Criteria
1. `cargo test -p codex-stage0 -p codex-tui` passes all tests
2. `/stage0.index` indexes both memories AND code units to TF-IDF
3. `/stage0.run` with code lane enabled produces TASK_BRIEF with "Code Context" section
4. Code units have stable IDs and can be reverse-mapped to file:line
5. tree-sitter-rust parses real codex-rs code without panics

---

## File Changes Summary

| File | Action |
|------|--------|
| `tui/Cargo.toml` | Add tree-sitter, tree-sitter-rust deps |
| `tui/src/chatwidget/spec_kit/code_index.rs` | NEW: CodeUnit, CodeUnitExtractor |
| `tui/src/chatwidget/spec_kit/mod.rs` | Export code_index module |
| `tui/src/chatwidget/spec_kit/stage0_commands.rs` | Extend /stage0.index for code |
| `stage0/src/dcc.rs` | Add code lane config, CodeCandidate, code section in TASK_BRIEF |
| `stage0/src/config.rs` | Add code_lane_enabled, code_top_k to config |
| `tui/src/chatwidget/spec_kit/stage0_seeding.rs` | Light code references in NL_* |
| `tui/tests/code_index_tests.rs` | NEW: extraction, indexing, DCC tests |
| `tui/tests/fixtures/sample_code/` | NEW: tiny synthetic codebase for tests |

---

## Config Shape (stage0.toml)

```toml
[vector_index]
max_memories_to_index = 500
max_code_units_to_index = 0  # 0 = no limit

[context_compiler]
code_lane_enabled = true
code_top_k = 10
```

---

## Reference Commits
- P83: fb8caa4a8 feat(stage0): SPEC-KIT-102 V2.5 Hybrid Retrieval Integration
- P84: (pending commit) feat(stage0): SPEC-KIT-102 P84 Hardening
