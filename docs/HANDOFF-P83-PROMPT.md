# Next Session Prompt (P83)

**Continue SPEC-KIT-102 Stage 0 Integration - Phase V2.5b: TF-IDF Backend Wiring, Real Integration Tests, and Hybrid Evaluation**

Prior session (P82) completed:
- V2.5 Hybrid Retrieval Integration
- `compile_context` and `run_stage0` accept `Option<&V: VectorBackend>`
- `vector_score` field in MemoryCandidate/ExplainScore
- NoopVectorBackend for when hybrid is disabled
- 110 tests passing

Commit: fb8caa4a8

---

## P83 OBJECTIVES

This session implements three production-path components:

1. **Wire TfIdfBackend into Stage0 pipeline** with `/stage0.index` command
2. **Real local-memory integration tests** (env-gated)
3. **Hybrid vs non-hybrid P@k evaluation** via `/stage0.eval-backend`

---

## 1. TF-IDF BACKEND WIRING

### 1.1 Add Vector Index Config (`stage0/src/config.rs`)

Add new config section:

```rust
/// Vector index configuration (V2.5b)
#[derive(Debug, Deserialize, Clone)]
pub struct VectorIndexConfig {
    /// Maximum memories to index (0 = no limit, index all)
    /// When set, indexes top N by dynamic_score DESC
    #[serde(default = "default_vector_index_limit")]
    pub max_memories_to_index: usize,
}

fn default_vector_index_limit() -> usize {
    0 // No limit by default - index all
}
```

Add to Stage0Config:
```rust
pub struct Stage0Config {
    // ... existing fields ...
    #[serde(default)]
    pub vector_index: VectorIndexConfig,
}
```

### 1.2 Extend OverlayDb for Index Queries (`stage0/src/overlay_db.rs`)

Add method to fetch memories ordered by score:

```rust
impl OverlayDb {
    /// Get memory IDs ordered by dynamic_score DESC, limited by count
    /// Falls back to initial_priority if dynamic_score is NULL
    pub fn get_memory_ids_for_indexing(&self, limit: Option<usize>) -> Result<Vec<String>> {
        // SELECT memory_id FROM overlay_memories
        // ORDER BY COALESCE(dynamic_score, initial_priority * 0.1) DESC
        // LIMIT ?
    }
}
```

### 1.3 TUI State: Shared TfIdfBackend (`tui/src/chatwidget/spec_kit/`)

Create `tui/src/chatwidget/spec_kit/vector_state.rs`:

```rust
use codex_stage0::tfidf::TfIdfBackend;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared vector backend state for Stage0 hybrid retrieval
pub struct VectorState {
    pub backend: Arc<RwLock<TfIdfBackend>>,
    pub indexed_count: usize,
    pub last_indexed: Option<chrono::DateTime<chrono::Utc>>,
}

impl VectorState {
    pub fn new() -> Self {
        Self {
            backend: Arc::new(RwLock::new(TfIdfBackend::new())),
            indexed_count: 0,
            last_indexed: None,
        }
    }
}
```

### 1.4 `/stage0.index` Command

Add TUI command that:

1. Checks for local-memory MCP availability
2. Gets memory IDs from overlay (ordered by score, respecting limit)
3. For each memory:
   - Fetch content via LocalMemoryMcpAdapter
   - Create VectorDocument with id, kind=Memory, text, metadata
4. Clear and repopulate TfIdfBackend
5. Update VectorState with count and timestamp
6. Print summary

```
/stage0.index

Stage 0 Vector Index
━━━━━━━━━━━━━━━━━━━━
Config: max_memories_to_index = 0 (no limit)
Fetching memories from local-memory...
  Found 847 memories in overlay
  Fetching content for 847 memories...
Indexing into TF-IDF backend...
  Indexed 847 documents (12,431 unique tokens)
  Duration: 342ms

Index ready for hybrid retrieval.
```

### 1.5 Wire TfIdfBackend into run_stage0

Update `run_stage0_blocking` to use VectorState:

```rust
fn run_stage0_blocking(
    // ... existing params ...
    vector_state: Option<&VectorState>,
) -> Result<codex_stage0::Stage0Result, String> {
    rt.block_on(async {
        let engine = Stage0Engine::new()?;

        // Get vector backend if available and indexed
        let vector_guard;
        let vector_opt: Option<&TfIdfBackend> = if let Some(vs) = vector_state {
            if vs.indexed_count > 0 {
                vector_guard = vs.backend.read().await;
                Some(&*vector_guard)
            } else {
                None
            }
        } else {
            None
        };

        // Call run_stage0 with vector backend
        engine.run_stage0(&local_memory, &llm, vector_opt, &tier2, ...)
    })
}
```

---

## 2. REAL LOCAL-MEMORY INTEGRATION TESTS

### 2.1 Test Harness (`tui/tests/stage0_local_memory_integration.rs`)

Create env-gated integration tests:

```rust
//! Real local-memory integration tests for Stage0
//!
//! These tests require a running local-memory daemon.
//! Set STAGE0_LM_TEST_URL to enable (e.g., "http://localhost:3000")
//!
//! Run with: STAGE0_LM_TEST_URL=... cargo test --test stage0_local_memory_integration

use std::env;

fn get_test_url() -> Option<String> {
    env::var("STAGE0_LM_TEST_URL").ok()
}

#[tokio::test]
async fn test_compile_context_retrieves_seeded_memory() {
    let Some(lm_url) = get_test_url() else {
        eprintln!("STAGE0_LM_TEST_URL not set; skipping integration test");
        return;
    };

    // 1. Create adapters
    // 2. Seed a test memory via MCP store_memory
    // 3. Call compile_context with matching spec snippet
    // 4. Assert seeded memory appears in memories_used
    // 5. Cleanup: delete test memory
}

#[tokio::test]
async fn test_run_stage0_full_pipeline() {
    let Some(lm_url) = get_test_url() else {
        eprintln!("STAGE0_LM_TEST_URL not set; skipping integration test");
        return;
    };

    // 1. Seed fixture memories
    // 2. Call run_stage0 with NoopTier2Client
    // 3. Assert Stage0Result has:
    //    - Non-empty task_brief_md
    //    - Fallback divine_truth (tier2_used = false)
    //    - Seeded memories in memories_used
    //    - Usage recorded in overlay
    // 4. Cleanup
}
```

### 2.2 Test Memory Seeding Helper

```rust
/// Seed a test memory and return its ID
async fn seed_test_memory(
    local_mem: &LocalMemoryMcpAdapter,
    content: &str,
    tags: Vec<String>,
) -> Result<String, String> {
    // Call local-memory MCP store_memory
    // Return generated memory ID
}

/// Delete a test memory by ID
async fn cleanup_test_memory(
    local_mem: &LocalMemoryMcpAdapter,
    id: &str,
) -> Result<(), String> {
    // Call local-memory MCP delete_memory
}
```

---

## 3. HYBRID VS NON-HYBRID EVALUATION

### 3.1 Eval Case JSON Schema

Create `docs/SPEC-KIT-102-notebooklm-integration/evidence/vector_eval_cases.json`:

```json
[
  {
    "name": "stage0_architecture",
    "spec_snippet": "Design Stage 0 overlay engine with NotebookLM integration",
    "expected_ids": ["mem-stage0-arch", "mem-stage0-constraints"]
  },
  {
    "name": "tfidf_implementation",
    "spec_snippet": "Implement TF-IDF vector backend for hybrid retrieval",
    "expected_ids": ["mem-vector-backend", "mem-tfidf-design"]
  }
]
```

### 3.2 `/stage0.eval-backend` Command

Add TUI command with options:

```
/stage0.eval-backend --cases ./evidence/vector_eval_cases.json --top-k 10
/stage0.eval-backend --cases ./evidence/vector_eval_cases.json --top-k 10 --json
```

Behavior:

1. Load eval cases from JSON
2. Ensure TfIdfBackend is indexed (warn if not)
3. For each case, run DCC twice:
   - **Baseline**: hybrid_enabled=false (or vector=None)
   - **Hybrid**: hybrid_enabled=true with TfIdfBackend
4. Compute P@k and Recall@k for each
5. Output results:

**Text format (default):**
```
Stage 0 Hybrid Retrieval Evaluation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Cases: 5  |  Top-K: 10  |  Index: 847 docs

Case                    Mode      P@10  R@10  Hits
─────────────────────────────────────────────────
stage0_architecture     baseline   0.40  0.50  mem-stage0-arch
stage0_architecture     hybrid     0.60  0.75  mem-stage0-arch, mem-overlay-design
tfidf_implementation    baseline   0.30  0.33  mem-vector-backend
tfidf_implementation    hybrid     0.70  0.67  mem-vector-backend, mem-tfidf-design

Summary:
  Baseline avg P@10: 0.35
  Hybrid avg P@10:   0.65  (+85.7%)
```

**JSON format (--json):**
```json
{
  "config": { "top_k": 10, "index_size": 847 },
  "results": [
    {
      "case_name": "stage0_architecture",
      "mode": "baseline",
      "precision_at_k": 0.4,
      "recall_at_k": 0.5,
      "expected_ids": ["mem-stage0-arch", "mem-stage0-constraints"],
      "hits": ["mem-stage0-arch"]
    },
    ...
  ],
  "summary": {
    "baseline_avg_precision": 0.35,
    "hybrid_avg_precision": 0.65,
    "improvement_pct": 85.7
  }
}
```

### 3.3 Eval Types (`stage0/src/eval.rs`)

Extend existing eval module:

```rust
/// Eval case loaded from JSON
#[derive(Debug, Clone, Deserialize)]
pub struct EvalCase {
    pub name: String,
    pub spec_snippet: String,
    pub expected_ids: Vec<String>,
}

/// Result of evaluating one case in one mode
#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    pub case_name: String,
    pub mode: String, // "baseline" | "hybrid"
    pub precision_at_k: f64,
    pub recall_at_k: f64,
    pub expected_ids: Vec<String>,
    pub hits: Vec<String>,
}

/// Full evaluation output
#[derive(Debug, Serialize)]
pub struct EvalOutput {
    pub config: EvalConfig,
    pub results: Vec<EvalResult>,
    pub summary: EvalSummary,
}
```

---

## FILES TO CREATE/MODIFY

**New Files:**
- `tui/src/chatwidget/spec_kit/vector_state.rs` - Shared TfIdfBackend state
- `tui/tests/stage0_local_memory_integration.rs` - Integration tests
- `docs/.../evidence/vector_eval_cases.json` - Sample eval cases

**Modified Files:**
- `stage0/src/config.rs` - Add VectorIndexConfig
- `stage0/src/overlay_db.rs` - Add get_memory_ids_for_indexing
- `stage0/src/eval.rs` - Extend eval types for JSON loading
- `tui/src/chatwidget/spec_kit/mod.rs` - Add vector_state module
- `tui/src/chatwidget/spec_kit/stage0_integration.rs` - Wire VectorState

---

## SUCCESS CRITERIA

1. **`/stage0.index` works**:
   - Indexes memories from local-memory
   - Respects `max_memories_to_index` config (0 = all)
   - Orders by dynamic_score DESC
   - Prints summary with doc count and token count

2. **Integration tests pass** (when env var set):
   - `test_compile_context_retrieves_seeded_memory`
   - `test_run_stage0_full_pipeline`
   - Tests skip cleanly when STAGE0_LM_TEST_URL not set

3. **`/stage0.eval-backend` works**:
   - Loads cases from JSON
   - Runs baseline vs hybrid comparison
   - Outputs text table (default) or JSON (--json)
   - Shows P@k, R@k, and hits for each case

4. **No regressions**: All 110 existing tests still pass

---

## OUT OF SCOPE (P83)

- External vector DBs (Qdrant, pgvector)
- Incremental indexing (deferred - full rebuild is fine for ~1k memories)
- Persistent TF-IDF index (in-memory only)
- Code-unit indexing (memory-only for now)

---

## REFERENCE

- Prior session: docs/HANDOFF-P82.md
- Commit: fb8caa4a8
- Config decisions from P82: hybrid_enabled=true, vector_weight=0.20, vector_top_k=50
