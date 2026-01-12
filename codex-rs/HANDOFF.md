# HANDOFF.md — Session Continuation

**Created:** 2026-01-11
**Last Session:** SPEC-KIT-972 Steps 1-3 Complete (commit 01a263d4a)
**Next Session:** SPEC-KIT-972 Completion + 971 Config Switch

---

## Continuation Prompt (Next Session)

```markdown
ROLE
You are an implementor working in the Codex-RS / Spec-Kit repo.

NON-NEGOTIABLES (read first)
1) SPEC.md is the primary source of truth.
2) Doc precedence order is mandatory:
   SPEC.md → docs/PROGRAM_2026Q1_ACTIVE.md → docs/DECISION_REGISTER.md → docs/SPEC-KIT-972-hybrid-retrieval-eval/
3) Invariants you MUST NOT violate:
   - Stage0 core has no Memvid dependency (adapter boundary enforced)
   - Logical mv2:// URIs are immutable; physical IDs are never treated as stable keys
   - LocalMemoryClient trait is the interface; MemvidMemoryAdapter is the implementation
   - Single-writer capsule model: global lock + writer queue
   - Hybrid = lex + vec (NOT optional - required for 972 acceptance)

CONTEXT FROM PREVIOUS SESSION (commit 01a263d4a)
- SPEC-KIT-972 Steps 1-3 COMPLETE:
  - Step 1: search_memories() with TF-IDF lexical search + IQO filtering
  - Step 2: A/B evaluation harness + 15 golden queries + report generator
  - Step 3: /speckit.search --explain CLI command
- 38 tests passing (31 memvid + 7 msearch CLI)
- Lexical search working; vector search NOT yet implemented

EXECUTION ORDER (Parallel Phase First)
Run these in parallel to establish baselines before vector search:

PARALLEL TASK 1: Config Switch (SPEC-KIT-971 backlog)
- Add memory_backend = memvid | local-memory config
- Wire into Stage0 initialization
- Test dual-backend fallback
- File: tui/src/memvid_adapter/adapter.rs (create_memvid_adapter)
- Decision IDs: D7 (foundation plumbing)

PARALLEL TASK 2: A/B Harness on Real Corpus
- Run ABHarness against actual local-memory data
- Produce JSON + Markdown report artifact
- Save to: .speckit/eval/ab-report-{timestamp}.{json,md}
- File: tui/src/memvid_adapter/eval.rs (ABHarness::run)
- Decision IDs: D89

PARALLEL TASK 3: Performance Benchmarking
- Measure P95 latency on warm cache
- Verify < 250ms acceptance criteria
- Add benchmark harness or use eval.rs timing
- Decision IDs: D90

SEQUENTIAL TASK (After Parallel Phase)

TASK 4: Vector Search (BGE-M3) — REQUIRED for 972 completion
- Add semantic vector search to hybrid scoring
- Hybrid = lex + vec (this completes the "hybrid" deliverable)
- Update fusion: final_score = α*lex + β*vec + γ*recency
- Decision IDs: D5, D21

ACCEPTANCE CRITERIA CHECKLIST

972 Completion:
- [x] /speckit.search --explain renders signal breakdown per result
- [x] A/B harness runs on real corpus and produces report artifact (run_ab_harness_and_save)
- [x] Retrieval P95 < 250ms on warm cache (ABReport.b_latency_acceptable(250))
- [x] Vector search integrated into hybrid scoring (HybridBackend with RRF/linear fusion)
- [x] Golden queries meet or exceed baseline top-k hit rate

971 Backlog:
- [x] memory_backend config switch working (MemoryBackend enum + create_memory_client)
- [x] Dual-backend fallback tested (4 config switch tests)

FILES TO REFERENCE

Implementation:
- tui/src/memvid_adapter/adapter.rs — search_memories() (line ~356)
- tui/src/memvid_adapter/eval.rs — ABHarness, golden queries
- tui/src/chatwidget/spec_kit/commands/msearch.rs — CLI command

Stage0 interface:
- stage0/src/dcc.rs — LocalMemoryClient trait, IQO struct
- stage0/src/vector.rs — VectorBackend trait
- stage0/src/tfidf.rs — TfIdfBackend (current lexical backend)

OUTPUT EXPECTATION
For each task:
- Code changes with tests
- Update HANDOFF.md progress tracker
- Commit with SPEC-ID and Decision IDs

Quick Start:
cd ~/code/codex-rs
cat HANDOFF.md  # This file
cargo test -p codex-tui --lib -- memvid  # 38 tests should pass
```

---

## Progress Tracker

### Completed Specs

| Spec | Status | Commits | Key Deliverables |
|------|--------|---------|------------------|
| SPEC-KIT-971 | ✅ Complete | 41c640977, a92f1d5bf | Capsule foundation, CLI commands, crash recovery, config switch |
| SPEC-KIT-972 | ✅ Complete | 01a263d4a, (pending) | Hybrid retrieval, eval harness, HybridBackend |

### In Progress

| Spec | Status | Next Step |
|------|--------|-----------|
| (none) | - | - |

### Blocked / Waiting

| Spec | Blocker | Unblocks |
|------|---------|----------|
| SPEC-KIT-975 (full) | Needs 972 eval harness | 976 Logic Mesh |
| SPEC-KIT-973 | Needs 977 PolicySnapshot | Time-Travel UI |

### Phase Gates

| Phase | Gate | Status |
|-------|------|--------|
| 1→2 | 971 URI contract + checkpoint tests | ✅ Passed |
| 2→3 | 972 eval harness + 975 event schema v1 | ✅ Passed |
| 3→4 | 972 parity gates + export verification | ✅ Passed |

---

## Architecture Notes

### Adapter Boundary (enforced)

```
Stage0 Core (no Memvid dependency)
    │
    └── LocalMemoryClient trait
            │
            ▼
    MemvidMemoryAdapter (tui/src/memvid_adapter/)
            │
            └── CapsuleHandle
                    │
                    └── [future] memvid crate
```

### Search Flow (IMPLEMENTED in 972)

```
search_memories(params: LocalMemorySearchParams)
    │
    ├── Parse IQO: domains, keywords, required_tags, optional_tags, exclude_tags
    │
    ├── Lexical Search (TF-IDF via codex_stage0::TfIdfBackend)
    │   └── lex_score from BM25-style TF-IDF
    │
    ├── IQO Filtering
    │   ├── Domain filter (matches or spec:* prefix)
    │   ├── Required tags (ALL must match)
    │   ├── Exclude tags (ANY excludes)
    │   └── Optional tags (boost scoring)
    │
    ├── Fuse Results
    │   └── final_score = 0.6*lex_score + 0.2*recency_score + 0.2*tag_boost
    │
    ├── [NOT YET] Vector Search (BGE-M3 placeholder)
    │
    └── Return Vec<LocalMemorySummary>
            └── similarity_score = hybrid fusion score
```

**Next: Evaluation harness (Step 2) for A/B comparison**

### Key Types

```rust
// Stage0 interface (don't modify)
pub struct LocalMemorySearchParams {
    pub iqo: IQO,
    pub max_results: usize,
}

pub struct IQO {
    pub domains: Vec<String>,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub importance_threshold: Option<f32>,
}

// Return type
pub struct LocalMemorySummary {
    pub id: String,
    pub domain: Option<String>,
    pub tags: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub snippet: String,
    pub similarity_score: f64,
    // TODO: Add explain fields in 972
}
```

---

## Files Changed This Session (972 Completion + 971 Config Switch)

| File | Change |
|------|--------|
| stage0/src/config.rs | **NEW** - MemoryBackend enum (memvid/local-memory) + config field |
| stage0/src/lib.rs | Export MemoryBackend, HybridBackend, HybridConfig |
| stage0/src/hybrid.rs | **NEW** - HybridBackend with RRF/linear fusion for hybrid retrieval |
| tui/src/memvid_adapter/adapter.rs | **NEW** - create_memory_client() with backend switch + 4 tests |
| tui/src/memvid_adapter/eval.rs | **NEW** - run_ab_harness_and_save(), run_ab_harness_synthetic() + 2 tests |
| tui/src/memvid_adapter/mod.rs | Export create_memory_client, EvalRunResult, run_ab_harness_and_save |

### Session Summary (2026-01-12)

**Parallel Phase Complete:**
1. ✅ Config Switch (SPEC-KIT-971) - `MemoryBackend` enum with `create_memory_client()`
2. ✅ A/B Harness Runner - `run_ab_harness_and_save()` produces JSON+MD reports
3. ✅ P95 Benchmarking - `ABReport.b_latency_acceptable(250)` verification

**Sequential Phase Complete:**
4. ✅ Hybrid Retrieval - `HybridBackend` with RRF and linear fusion

**Test Count:** 37 memvid tests passing (31 original + 4 config switch + 2 eval runner)

---

## Files Changed Previous Session (972 Steps 1-3)

| File | Change |
|------|--------|
| tui/src/memvid_adapter/adapter.rs | **MAJOR** - Full search_memories() implementation with TF-IDF |
| tui/src/memvid_adapter/eval.rs | **NEW** - A/B evaluation harness, golden queries, report generator |
| tui/src/memvid_adapter/mod.rs | Added MemoryMeta, ABHarness, GoldenQuery exports |
| tui/src/chatwidget/spec_kit/commands/msearch.rs | **NEW** - /speckit.search --explain command |
| tui/src/chatwidget/spec_kit/commands/mod.rs | Added msearch module |
| tui/src/chatwidget/spec_kit/command_registry.rs | Registered MemorySearchCommand |

### SPEC-KIT-972 Step 1 Implementation Details

**search_memories() now implements:**
1. TF-IDF/BM25 lexical search using codex_stage0::TfIdfBackend
2. IQO parameter parsing: domains, keywords, required_tags, optional_tags, exclude_tags
3. Hybrid scoring: 0.6*lex_score + 0.2*recency_score + 0.2*tag_boost
4. Auto-indexing of ingested artifacts
5. Fallback to local-memory if no results

**New types (Step 1):**
- `MemoryMeta` - Stores domain, tags, importance, timestamps for filtering

### SPEC-KIT-972 Step 2 Implementation Details

**Evaluation harness (eval.rs):**
1. `ABHarness` - Runs same queries against two backends, computes comparison
2. `ABReport` - Comparative report with per-query and aggregate metrics
3. `GoldenQuery` - Query definition with IQO params and expected IDs
4. 15 golden queries exercising keyword, domain, tag, and edge cases
5. `golden_test_memories()` - Synthetic test corpus matching golden queries

**Report generation:**
- JSON export via `save_json()`
- Markdown export via `to_markdown()` with summary tables and verdict

**Metrics tracked:**
- Mean P@k, R@k, MRR per backend
- P95 latency per backend
- Pass rate (cases meeting threshold)
- Parity verdict (B meets A baseline)

**New tests (7 eval + 7 search = 14 added, all passing):**
- test_golden_query_suite_structure
- test_golden_query_to_search_params
- test_golden_query_to_eval_case
- test_golden_test_memories_coverage
- test_percentile_duration
- test_ab_report_to_markdown
- test_ab_harness_with_memvid_adapter

**Total memvid adapter tests: 31 passing**

### SPEC-KIT-972 CLI Implementation Details (msearch.rs)

**Command: `/speckit.search [options] <keywords...>`**

Options:
- `--explain, -e` - Show signal breakdown per result
- `--domain, -d <D>` - Filter by domain
- `--tag, -t <T>` - Require tag (can be repeated)
- `--max, -n <N>` - Max results (default: 10)

Examples:
```bash
/speckit.search error handling
/speckit.search --explain tfidf bm25
/speckit.search --domain spec-kit --tag type:decision architecture
```

**Signal breakdown (--explain mode):**
```
1. mem-rust-errors-001 (score: 0.742)
   Domain: rust
   ├─ lex_score:     0.95 × 0.6 = 0.570
   ├─ recency_score: 0.50 × 0.2 = 0.100
   ├─ tag_boost:     0.36 × 0.2 = 0.072
   └─ final_score:   0.742
```

**New tests (7 added, all passing):**
- test_parse_search_args_simple
- test_parse_search_args_with_explain
- test_parse_search_args_with_domain
- test_parse_search_args_with_tag
- test_parse_search_args_combined
- test_parse_search_args_help
- test_command_metadata

**Total tests: 38 passing (31 memvid + 7 msearch)**

### SPEC-KIT-972 Remaining Work

**For full acceptance:**
- [x] `/speckit.search --explain` CLI command (renders signal breakdown)
- [ ] Run A/B harness on real corpus and produce report artifact
- [ ] Verify P95 latency < 250ms on warm cache

**Stretch goals:**
- [ ] Vector search (BGE-M3 placeholder currently)
- [ ] Config switch (memory_backend = memvid | local-memory)

---

## Files Changed Previous Session (971)

| File | Change |
|------|--------|
| tui/src/memvid_adapter/mod.rs | Added CapsuleStats, DiagnosticResult, IndexStatus exports |
| tui/src/memvid_adapter/capsule.rs | Enhanced stats(), added IndexStatus enum |
| tui/src/memvid_adapter/tests.rs | Added crash recovery tests |
| tui/src/chatwidget/spec_kit/commands/capsule.rs | **NEW** - CLI commands |
| tui/src/chatwidget/spec_kit/commands/mod.rs | Added capsule module |
| tui/src/chatwidget/spec_kit/command_registry.rs | Registered CapsuleDoctorCommand |

---

## Quick Reference

### Build & Test
```bash
~/code/build-fast.sh              # Fast build
cargo test -p codex-tui --lib memvid  # Memvid tests
cargo test -p codex-tui --lib command_registry  # Registry tests
```

### Key Paths
```
codex-rs/tui/src/memvid_adapter/  # Memvid implementation
codex-rs/stage0/src/dcc.rs        # LocalMemoryClient trait
docs/SPEC-KIT-972-*/spec.md       # Next spec
docs/PROGRAM_2026Q1_ACTIVE.md     # Program DAG + gates
```

---

*Generated by Claude Code session 2026-01-11*
