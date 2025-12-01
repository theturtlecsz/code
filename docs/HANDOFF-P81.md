# HANDOFF-P81: SPEC-KIT-102 Stage 0 Integration - Phase V2 Complete

**Date**: 2025-12-01
**Session**: P81
**Commit**: (pending)

## Summary

Implemented VectorBackend abstraction and in-memory TF-IDF evaluation harness for SPEC-KIT-102 Stage 0 integration.

## Completed Work

### 1. VectorBackend Abstraction (`stage0/src/vector.rs`)
- `VectorDocument` struct with id, kind, text, metadata
- `DocumentKind` enum (Memory, Code, Spec, Adr, Other)
- `DocumentMetadata` with tags, domain, source_path, overlay_score
- `VectorFilters` for query-time filtering (kinds, tags, domain, min_score)
- `ScoredVector` for search results with relevance scores
- `VectorBackend` trait with async methods:
  - `index_documents()` - Batch indexing
  - `search()` - Query with filters and top_k
  - `document_count()`, `clear()`, `get_document()`, `delete_document()`

### 2. In-Memory TF-IDF Backend (`stage0/src/tfidf.rs`)
- BM25-style scoring with configurable k1 and b parameters
- IDF smoothing: log((N + 1) / (df + 1)) + 1
- Simple tokenizer with stop words filtering
- Thread-safe via RwLock for concurrent access
- Full VectorBackend trait implementation

### 3. Evaluation Harness (`stage0/src/eval.rs`)
- `EvalCase` - Test case with query and expected_ids
- `EvalResult` - Per-case precision@k, recall@k, reciprocal_rank
- `EvalSuiteResult` - Aggregate metrics with pass rate
- `evaluate_backend()` - Run full evaluation suite
- `built_in_eval_cases()` - 5 hardcoded test cases
- `built_in_test_documents()` - 12 synthetic documents
- JSON file loading/saving support

### 4. TUI Commands
- `/stage0.index` - Index local-memory into TF-IDF backend
  - Args: `--max=N` for max memories
- `/stage0.eval-backend` - Run evaluation harness
  - Alias: `/stage0.eval`
  - Args: `--top-k=N`, `--json`, `--json=path`

### 5. Test Coverage
- 34 new tests passing:
  - 10 VectorBackend/types tests
  - 17 TF-IDF backend tests
  - 7 Evaluation harness tests

### Files Created/Modified
```
codex-rs/stage0/src/vector.rs       # NEW: VectorBackend trait + types
codex-rs/stage0/src/tfidf.rs        # NEW: TF-IDF implementation
codex-rs/stage0/src/eval.rs         # NEW: Evaluation harness
codex-rs/stage0/src/lib.rs          # MODIFIED: Export new modules
codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs  # MODIFIED: New commands
codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs  # MODIFIED: Register commands
evidence/vector_eval_cases.json     # NEW: JSON eval cases
```

## Test Results
- 108 stage0 tests total (107 passing, 1 flaky pre-existing timing test)
- 16 command_registry tests passing
- 34 new vector/tfidf/eval tests passing

---

## Next Session (P82): Hybrid Retrieval Integration

### Design Decisions (User Confirmed)

1. **Hybrid Retrieval**: Implement in V2.5 with feature flags
   - Keep local-memory search as primary channel
   - TF-IDF VectorBackend as additional signal
   - Config-guarded with fallback to current behavior

2. **Persistence**: Keep in-memory only
   - Index rebuilds from local-memory each session
   - No SQLite persistence for TF-IDF (defer to V3 if needed)
   - VectorBackend is derivative data, not source of truth

3. **Testing**: Add real local-memory integration tests
   - Gate behind `LOCAL_MEMORY_TEST_URL` env var
   - Seed fixture data via MCP tools
   - Test full Stage0Engine pipeline

### Implementation Tasks for P82

#### 1. Extend DccContext with VectorBackend

```rust
pub struct DccContext<'a, Lm, Ll, V> {
    pub cfg: &'a Stage0Config,
    pub db: &'a OverlayDb,
    pub local_mem: &'a Lm,
    pub llm: &'a Ll,
    pub vector: Option<&'a V>, // NEW
}
```

#### 2. Add Hybrid Search in compile_context

After IQO building:
1. Call `local_mem.search_memories(...)` (existing)
2. If `vector.is_some()` and `cfg.context_compiler.hybrid_enabled`:
   - Call `vector.search(spec_snippet, filters, top_k_vec)`
3. Merge candidates by memory_id:
   - Update existing candidates with `vector_score`
   - Optionally add vector-only candidates

#### 3. Extend MemoryCandidate

```rust
pub struct MemoryCandidate {
    // ... existing fields ...
    pub vector_score: f64, // NEW: default 0.0
}
```

#### 4. Update Combined Score Formula

```rust
combined_score =
    sim_weight * similarity_score +
    dyn_weight * dynamic_score +
    vec_weight * vector_score;
```

#### 5. Add Config Fields

```toml
[stage0_overlay.context_compiler]
hybrid_enabled = true
vector_weight = 0.2
vector_top_k = 50
```

#### 6. Add Integration Tests

Create `tui/tests/stage0_local_memory_integration.rs`:
- Check `LOCAL_MEMORY_TEST_URL` env var
- Seed fixture memories via MCP
- Test LocalMemoryMcpAdapter
- Test Stage0Engine::compile_context end-to-end

### Files to Modify (P82)

```
codex-rs/stage0/src/config.rs       # Add hybrid config fields
codex-rs/stage0/src/dcc.rs          # DccContext + hybrid search
codex-rs/stage0/src/scoring.rs      # Add vector_score to formula
codex-rs/stage0/src/lib.rs          # Update compile_context signature
codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs  # Pass vector backend
codex-rs/tui/tests/stage0_local_memory_integration.rs  # NEW: Integration tests
```

### Success Criteria for P82

- [ ] Hybrid retrieval working behind `hybrid_enabled` flag
- [ ] Evaluation harness shows P@k/R@k comparison (hybrid vs non-hybrid)
- [ ] Integration tests pass against real local-memory daemon
- [ ] No regression in existing Stage0 tests
- [ ] Config documented in stage0.toml

---

## Out of Scope for P82

- External vector DBs (Qdrant, pgvector) - V3+
- Code-unit indexing - V2.5+ (after memories work)
- Persistent TF-IDF index - defer unless performance demands

## Reference Links

- Prior session: docs/HANDOFF-P80.md
- Main spec: docs/SPEC-KIT-102-notebooklm-integration/spec.md
- Implementation plan: docs/SPEC-KIT-102-notebooklm-integration/IMPLEMENTATION-PLAN.md
