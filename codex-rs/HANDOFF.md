# HANDOFF.md — Session Continuation

**Created:** 2026-01-11
**Last Session:** 2026-01-13 (SPEC-KIT-971 Pipeline Integration In Progress)
**Next Session:** Complete 971-A5 Acceptance Tests + Run Verification

---

## Continuation Prompt (Next Session)

```markdown
ROLE
You are an implementor working in the Codex-RS / Spec-Kit repo.

NON-NEGOTIABLES (read first)
1) SPEC.md is the primary source of truth.
2) Doc precedence order is mandatory:
   SPEC.md → docs/PROGRAM_2026Q1_ACTIVE.md → docs/DECISION_REGISTER.md
3) Invariants you MUST NOT violate:
   - Stage0 core has no Memvid dependency (adapter boundary enforced)
   - Logical mv2:// URIs are immutable; physical IDs are never treated as stable keys
   - LocalMemoryClient trait is the interface; MemvidMemoryAdapter is the implementation
   - Single-writer capsule model: global lock + writer queue
   - Hybrid = lex + vec (required, not optional)
   - Merge modes are `curated` or `full` only (never squash/ff/rebase)

===================================================================
CURRENT STATE — Session interrupted 2026-01-13
===================================================================

TASK IN PROGRESS: SPEC-KIT-971 Pipeline Integration (971-A5)
Goal: `/speckit.auto` MUST NOT hard-require local-memory when `Stage0Config.memory_backend=memvid`

IMPLEMENTATION COMPLETED (not yet tested):
1. UnifiedMemoryClient enum — Type-safe backend abstraction (adapter.rs)
2. create_unified_memory_client() — Factory with fallback logic (adapter.rs)
3. Backend routing — stage0_integration.rs uses config-based routing
4. Stage0Progress::CreatingMemoryClient variant — Progress reporting (mod.rs)
5. 3 acceptance tests added — test_971_a5_* in stage0_integration.rs

KEY IMPLEMENTATION PATTERN:
```rust
pub enum UnifiedMemoryClient {
    Memvid(MemvidMemoryAdapter),
    LocalMemory(crate::stage0_adapters::LocalMemoryCliAdapter),
}

#[async_trait]
impl LocalMemoryClient for UnifiedMemoryClient {
    async fn search_memories(&self, params: LocalMemorySearchParams) -> Stage0Result<Vec<LocalMemorySummary>> {
        match self {
            UnifiedMemoryClient::Memvid(adapter) => adapter.search_memories(params).await,
            UnifiedMemoryClient::LocalMemory(adapter) => adapter.search_memories(params).await,
        }
    }
}
```

PROBLEM SOLVED:
- `dyn LocalMemoryClient` doesn't implement `Sized` (required by Stage0Engine::run_stage0)
- Solution: Enum dispatch pattern provides type safety + Sized guarantee

===================================================================
NEXT STEPS (In Order)
===================================================================

1. READ THESE FILES FIRST:
   - tui/src/memvid_adapter/adapter.rs (lines 1-100 for UnifiedMemoryClient)
   - tui/src/chatwidget/spec_kit/stage0_integration.rs (lines 300-400 for backend routing)
   - tui/src/chatwidget/spec_kit/stage0_integration.rs (tests at bottom)

2. RUN TESTS:
   ```bash
   cargo test -p codex-tui --lib -- stage0_integration::tests::test_971
   ```

   Expected: 3 tests pass:
   - test_971_a5_memvid_backend_without_local_memory
   - test_971_a5_memvid_fallback_to_local_memory
   - test_971_a5_memvid_no_fallback_fails

3. IF TESTS FAIL:
   - Check that UnifiedMemoryClient implements all LocalMemoryClient methods
   - Verify create_unified_memory_client handles all 4 scenarios:
     a) memvid OK → use memvid
     b) memvid fails, local-memory healthy → fallback to local-memory
     c) memvid fails, local-memory unhealthy → error
     d) config says local-memory → use local-memory directly

4. RUN FULL TEST SUITE:
   ```bash
   cargo test -p codex-tui --lib -- memvid
   cargo test -p codex-stage0 --lib
   python3 scripts/doc_lint.py
   python3 scripts/golden_path_test.py
   ```

5. COMMIT AND PUSH:
   ```bash
   git add -A && git commit -m "feat(stage0): 971-A5 pipeline honors memory_backend config

   - UnifiedMemoryClient enum for type-safe backend dispatch
   - create_unified_memory_client() factory with fallback logic
   - Backend routing in run_speckit_auto_pipeline()
   - 3 acceptance tests for 971-A5 scenarios

   Decision IDs: D1, D7
   Closes: 971-A5

   🤖 Generated with [Claude Code](https://claude.com/claude-code)

   Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
   git push
   ```

6. THEN PROCEED TO:
   - TASK 2: SPEC-977 PolicySnapshot (if not started)
   - TASK 3: SPEC-978 Reflex Stack

===================================================================
ACCEPTANCE TESTS FOR 971-A5 (Reference)
===================================================================

The tests verify:

1. test_971_a5_memvid_backend_without_local_memory
   - Config: memory_backend = Memvid
   - Mock: local-memory daemon NOT running
   - Mock: capsule path exists
   - Expected: Pipeline proceeds using memvid (no local-memory check failure)

2. test_971_a5_memvid_fallback_to_local_memory
   - Config: memory_backend = Memvid
   - Mock: capsule creation fails
   - Mock: local-memory daemon IS running
   - Expected: Fallback to local-memory, pipeline continues

3. test_971_a5_memvid_no_fallback_fails
   - Config: memory_backend = Memvid
   - Mock: capsule creation fails
   - Mock: local-memory daemon NOT running
   - Expected: Pipeline fails with clear error

===================================================================
WARNINGS TO ADDRESS (if any remain after testing)
===================================================================

Compiler warnings noted in previous session:
- Unused imports (clean up after testing)
- Dead code warnings (may be fixed once tests exercise the code)

Run `cargo clippy -p codex-tui` after tests pass to clean up.

===================================================================
FILES MODIFIED THIS SESSION (2026-01-13)
===================================================================

| File | Change |
|------|--------|
| tui/src/memvid_adapter/adapter.rs | Added UnifiedMemoryClient enum + create_unified_memory_client() |
| tui/src/memvid_adapter/mod.rs | Export UnifiedMemoryClient, create_unified_memory_client |
| tui/src/chatwidget/spec_kit/stage0_integration.rs | Backend routing + Stage0Progress::CreatingMemoryClient + 3 tests |
| tui/src/chatwidget/mod.rs | Match arm for CreatingMemoryClient progress variant |

===================================================================
OUTPUT EXPECTATION
===================================================================

- Run tests first, fix any failures
- Commit with SPEC-ID and Decision IDs
- Push after verification passes
- Update progress tracker below
```

---

## Progress Tracker

### Completed Specs

| Spec | Status | Commits | Key Deliverables |
|------|--------|---------|------------------|
| SPEC-KIT-971 (core) | ✅ Complete | 41c640977, a92f1d5bf, abb5358fa | Capsule foundation, crash recovery, config switch |
| SPEC-KIT-972 | ✅ Complete | 01a263d4a, abb5358fa | Hybrid retrieval, eval harness, HybridBackend |

### In Progress

| Spec | Status | Next Step |
|------|--------|-----------|
| SPEC-KIT-971 (A5) | 🔄 90% | Run 971-A5 acceptance tests, commit |
| SPEC-KIT-971 (CLI) | 🔄 5% | Implement checkpoint listing CLI (after A5) |
| SPEC-KIT-977 | 🔄 40% | PolicySnapshot struct created in stage0/src/policy.rs, needs integration |
| SPEC-KIT-978 | 🔄 0% | Create ReflexBackend trait |

### Blocked / Waiting

| Spec | Blocker | Unblocks |
|------|---------|----------|
| SPEC-KIT-975 (full) | Needs 977 PolicySnapshot | 976 Logic Mesh |
| SPEC-KIT-973 | Needs 971 checkpoint CLI | Time-Travel UI |

### Phase Gates

| Phase | Gate | Status |
|-------|------|--------|
| 1→2 | 971 URI contract + checkpoint tests | ✅ Passed |
| 2→3 | 972 eval harness + 975 event schema v1 | ✅ Passed |
| 3→4 | 972 parity gates + export verification | ✅ Passed |
| 4→5 | 977 PolicySnapshot + 978 reflex stack | ⏳ Pending |

---

## Architecture Notes

### Adapter Boundary (enforced)

```
Stage0 Core (no Memvid dependency)
    │
    └── LocalMemoryClient trait
            │
            ▼
    UnifiedMemoryClient (enum dispatch)
            │
            ├── Memvid(MemvidMemoryAdapter)
            │       └── CapsuleHandle
            │
            └── LocalMemory(LocalMemoryCliAdapter)
                    └── `lm` CLI commands
```

### Backend Routing Flow (IMPLEMENTED 2026-01-13)

```
run_speckit_auto_pipeline()
    │
    ├── Read Stage0Config.memory_backend
    │
    ├── If Memvid:
    │   ├── Try create capsule
    │   │   ├── Success → UnifiedMemoryClient::Memvid
    │   │   └── Failure:
    │   │       ├── Check local-memory health
    │   │       │   ├── Healthy → UnifiedMemoryClient::LocalMemory (fallback)
    │   │       │   └── Unhealthy → Error (no backend available)
    │   │
    │   └── Continue pipeline with memory client
    │
    └── If LocalMemory:
        └── UnifiedMemoryClient::LocalMemory (direct)
```

### Search Flow (IMPLEMENTED)

```
search_memories(params: LocalMemorySearchParams)
    │
    ├── Parse IQO: domains, keywords, required_tags, optional_tags, exclude_tags
    │
    ├── Lexical Search (TF-IDF via TfIdfBackend)
    │   └── lex_score from BM25-style scoring
    │
    ├── [IMPLEMENTED] HybridBackend (stage0/src/hybrid.rs)
    │   ├── RRF fusion: 1/(k + rank_lex) + 1/(k + rank_vec)
    │   └── Linear fusion: lex_weight * lex + vec_weight * vec
    │
    ├── IQO Filtering
    │   ├── Domain filter (matches or spec:* prefix)
    │   ├── Required tags (ALL must match)
    │   └── Exclude tags (ANY excludes)
    │
    └── Return Vec<LocalMemorySummary>
```

### PolicySnapshot Flow (PARTIALLY IMPLEMENTED)

```
Run Start
    │
    ├── capture_policy_snapshot() [IMPLEMENTED in stage0/src/policy.rs]
    │   ├── Read model_policy.toml + MODEL-POLICY.md
    │   ├── Compile to canonical JSON
    │   ├── Compute SHA256 hash
    │   └── Generate policy_id
    │
    ├── Store to filesystem: .speckit/policies/snapshot-{id}.json [IMPLEMENTED]
    │
    ├── Store to capsule: mv2://.../policy/{id} [TODO: integration]
    │
    └── Tag all events with policy_id [TODO]
```

### Reflex Stack Flow (TO IMPLEMENT)

```
Reflex Call
    │
    ├── Try SGLang (primary)
    │   ├── Constrained decoding if schema provided
    │   └── Prefix caching for repeated prompts
    │
    ├── If SGLang fails → Try vLLM (fallback)
    │   └── OpenAI-compatible endpoint
    │
    ├── Capture call in capsule for replay
    │
    └── Return structured output
```

---

## Files Changed This Session (2026-01-13)

| File | Change |
|------|--------|
| tui/src/memvid_adapter/adapter.rs | UnifiedMemoryClient enum + create_unified_memory_client() factory |
| tui/src/memvid_adapter/mod.rs | Export new types |
| tui/src/chatwidget/spec_kit/stage0_integration.rs | Backend routing + CreatingMemoryClient progress + 3 tests |
| tui/src/chatwidget/mod.rs | Match arm for new progress variant |

---

## Test Summary

| Package | Tests | Status |
|---------|-------|--------|
| codex-tui (memvid) | 37+ | ⏳ Need to run |
| codex-stage0 | 260+ | ⏳ Need to run |
| Golden Path | 10/10 | ⏳ Need to run |
| Doc Lint | 0 errors | ⏳ Need to run |
| 971-A5 Acceptance | 3 | ⏳ Need to run |

---

## Quick Reference

### Build & Test
```bash
~/code/build-fast.sh              # Fast build
cargo test -p codex-tui --lib -- stage0_integration::tests::test_971  # 971-A5 tests
cargo test -p codex-tui --lib memvid  # Memvid tests
cargo test -p codex-stage0 --lib      # Stage0 tests
python3 scripts/doc_lint.py           # Doc contract lint
python3 scripts/golden_path_test.py   # Golden path E2E
```

### Key Paths
```
codex-rs/tui/src/memvid_adapter/  # Memvid implementation
codex-rs/tui/src/memvid_adapter/adapter.rs  # UnifiedMemoryClient (NEW)
codex-rs/stage0/src/              # Stage0 core (no Memvid dep)
codex-rs/stage0/src/hybrid.rs     # HybridBackend
codex-rs/stage0/src/policy.rs     # PolicySnapshot
codex-rs/SPEC.md                  # Root docs contract
docs/PROGRAM_2026Q1_ACTIVE.md     # Program DAG + gates
docs/DECISION_REGISTER.md         # Locked decisions
scripts/doc_lint.py               # Doc contract validator
scripts/golden_path_test.py       # E2E validation
```

---

*Generated by Claude Code session 2026-01-13*
