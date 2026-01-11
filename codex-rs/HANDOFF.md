# HANDOFF.md — Session Continuation

**Created:** 2026-01-11
**Last Session:** SPEC-KIT-971 Memvid Capsule Foundation (Phase 1 + 2 complete)
**Next Session:** SPEC-KIT-972 Hybrid Retrieval + Evaluation Harness

---

## Continuation Prompt

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
   - Replay is offline-first: exact for retrieval + events; model I/O depends on capture mode

CONTEXT FROM PREVIOUS SESSION
- SPEC-KIT-971 is COMPLETE (commits 41c640977, a92f1d5bf)
- Phase 2 gate passed: URI contract + checkpoint listing tests passing
- MemvidMemoryAdapter implements LocalMemoryClient with search_memories() stub
- CapsuleHandle has put(), commit_stage(), resolve_uri() implemented
- CLI commands working: /speckit.capsule doctor|stats|checkpoints|commit
- 35 tests passing (17 memvid + 16 registry + 2 capsule)

PRIMARY TASK: SPEC-KIT-972 Hybrid Retrieval + Evaluation Harness

Step 0 — Read these docs (in order)
- docs/SPEC-KIT-972-hybrid-retrieval-eval/spec.md (deliverables + acceptance criteria)
- tui/src/memvid_adapter/adapter.rs (search_memories stub at line 182-221)
- codex-stage0/src/dcc.rs (LocalMemorySearchParams, IQO struct)

Step 1 — First PR Target
Implement hybrid search in MemvidMemoryAdapter::search_memories():
1. Parse IQO parameters (domains, keywords, tags, importance threshold)
2. Implement lexical search (BM25 or simple TF-IDF baseline)
3. Implement vector search stub (placeholder for BGE-M3 integration)
4. Fuse results with weighted scoring
5. Return as LocalMemorySummary with explain fields

Step 2 — Second PR Target
Add evaluation harness:
1. Create golden query suite (10-20 representative queries)
2. Build A/B harness comparing local-memory vs memvid
3. Output report artifact (JSON + markdown summary)

Acceptance Criteria for 972:
- [ ] /speckit.search --explain renders signal breakdown per result
- [ ] Golden queries meet or exceed baseline top-k hit rate
- [ ] A/B harness runs and produces report artifact
- [ ] Retrieval P95 < 250ms on warm cache

SECONDARY TASKS (971 backlog - lower priority)

1. Dedup tracks (BLAKE3 + SimHash)
   - Enable when memvid crate is integrated
   - Add contract tests for exact + near-dup detection
   - File: tui/src/memvid_adapter/capsule.rs (config.enable_dedup)

2. Config switch (memory_backend)
   - Add memory_backend = memvid | local-memory config
   - Wire into Stage0 initialization
   - Test dual-backend fallback

FILES TO REFERENCE

Key implementation files:
- tui/src/memvid_adapter/adapter.rs:182 — search_memories() stub (TODO marker)
- tui/src/memvid_adapter/types.rs — LogicalUri, UriIndex
- tui/src/memvid_adapter/capsule.rs — CapsuleHandle lifecycle

Stage0 interface files:
- codex-stage0/src/dcc.rs — LocalMemoryClient trait, LocalMemorySearchParams
- codex-stage0/src/dcc.rs — IQO struct (domains, keywords, tags, importance)

Test files:
- tui/src/memvid_adapter/tests.rs — 17 lifecycle tests
- tui/src/memvid_adapter/adapter.rs:258 — adapter_tests module

DECISION IDS FOR 972

Implemented by 972: D5, D21, D24, D35, D89, D90
Referenced: D66, D80
Out of scope: D31

ACCEPTANCE CRITERIA CHECKLIST

For each PR:
- [ ] PR references SPEC-ID and exact deliverable(s)
- [ ] PR lists Decision IDs implemented/referenced/out-of-scope
- [ ] Tests added/updated and pass locally
- [ ] doc_lint passes (or evidence provided)
- [ ] Fallback to local-memory preserved until SPEC-979 gates pass

OUTPUT EXPECTATION
For each PR:
- Code changes
- Tests
- Any necessary doc updates (only in active program docs/specs)
- Short PR summary explaining how the change satisfies SPEC acceptance criteria

When finished with 972 baseline, update this HANDOFF.md for the next session.
```

---

## Progress Tracker

### Completed Specs

| Spec | Status | Commits | Key Deliverables |
|------|--------|---------|------------------|
| SPEC-KIT-971 | ✅ Complete | 41c640977, a92f1d5bf | Capsule foundation, CLI commands, crash recovery |

### In Progress

| Spec | Status | Next Step |
|------|--------|-----------|
| SPEC-KIT-972 | 🔄 Starting | Implement search_memories() |

### Blocked / Waiting

| Spec | Blocker | Unblocks |
|------|---------|----------|
| SPEC-KIT-975 (full) | Needs 972 eval harness | 976 Logic Mesh |
| SPEC-KIT-973 | Needs 977 PolicySnapshot | Time-Travel UI |

### Phase Gates

| Phase | Gate | Status |
|-------|------|--------|
| 1→2 | 971 URI contract + checkpoint tests | ✅ Passed |
| 2→3 | 972 eval harness + 975 event schema v1 | ⏳ Pending |
| 3→4 | 972 parity gates + export verification | ⏳ Pending |

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

### Search Flow (to implement in 972)

```
search_memories(params: LocalMemorySearchParams)
    │
    ├── Parse IQO: domains, keywords, tags, importance
    │
    ├── Lexical Search (BM25/TF-IDF)
    │   └── Score: lex_score
    │
    ├── Vector Search (BGE-M3 stub)
    │   └── Score: vec_score
    │
    ├── Fuse Results
    │   └── final_score = α*lex + β*vec + γ*recency
    │
    └── Return Vec<LocalMemorySummary>
            └── with explain: { lex_score, vec_score, recency, tags_matched }
```

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

## Files Changed This Session

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
