# HANDOFF.md — Session Continuation

**Created:** 2026-01-11
**Last Session:** 2026-01-12 (SPEC-KIT-972 Complete + V6 Doc Contract)
**Next Session:** SPEC-KIT-971/977/978 Parallel Implementation

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

COMPLETED IN PREVIOUS SESSION (2026-01-12)
- SPEC-KIT-972: COMPLETE
  - HybridBackend with RRF/linear fusion (stage0/src/hybrid.rs)
  - A/B evaluation harness with run_ab_harness_and_save()
  - P95 < 250ms verification via ABReport.b_latency_acceptable()
  - 37 memvid tests + 260 stage0 tests passing
- SPEC-KIT-971 Config Switch: COMPLETE
  - MemoryBackend enum (memvid | local-memory)
  - create_memory_client() factory function
  - 4 config switch tests
- V6 Doc Contract: COMPLETE
  - SPEC.md with invariants and replay truth table
  - docs/PROGRAM_2026Q1_ACTIVE.md with DAG
  - docs/DECISION_REGISTER.md with D1-D112
  - doc_lint.py integrated into pre-commit
- Golden Path Validation: 10/10 PASSED
  - scripts/golden_path_test.py validates full workflow
  - Reports in .speckit/eval/golden-path-*.{json,md}

COMMITS FROM LAST SESSION
- abb5358fa: feat(memvid): SPEC-KIT-972 completion + 971 config switch
- 593a9f000: docs: V6 doc contract + doc_lint.py enforcement
- 18b0326bd: test: golden-path E2E validation (10/10 passed)

===================================================================
NEXT SESSION TASKS (Execute in Parallel)
===================================================================

TASK 1: SPEC-971 Completion — CLI + resolve_uri API
Priority: HIGH
Decision: Full CLI implementation (user confirmed)

Deliverables:
1. CLI Commands:
   - `speckit capsule checkpoints` — List all checkpoints with IDs and labels
   - `speckit capsule commit --label <LABEL>` — Create manual checkpoint
   - TUI aliases: `/speckit.checkpoints`, `/speckit.commit <LABEL>`

2. API Implementation:
   - resolve_uri(uri: &str, branch: Option<&str>, as_of: Option<DateTime>) -> Result<ResolvedUri>
   - list_checkpoints(branch: Option<&str>) -> Result<Vec<CheckpointInfo>>
   - create_checkpoint(label: &str) -> Result<CheckpointId>

3. Requirements:
   - Checkpoints queryable by ID AND by label (non-negotiable)
   - Labels must be unique within a branch
   - as_of parameter enables point-in-time resolution

Files to modify:
- tui/src/memvid_adapter/capsule.rs — Add checkpoint listing and resolve_uri
- tui/src/chatwidget/spec_kit/commands/capsule.rs — Add CLI commands
- stage0/src/dcc.rs — Add resolve_uri to LocalMemoryClient trait

Decision IDs: D6, D11, D14, D18

---

TASK 2: SPEC-977 PolicySnapshot — Capture at Boundaries
Priority: HIGH
Decision: JSON format compiled from human-readable source

Deliverables:
1. PolicySnapshot struct:
   ```rust
   pub struct PolicySnapshot {
       pub schema_version: String,  // "1.0"
       pub policy_id: String,       // UUID
       pub hash: String,            // SHA256 of canonical JSON
       pub created_at: DateTime<Utc>,
       pub model_config: ModelConfig,
       pub weights: ScoringWeights,
       pub prompts: HashMap<String, String>,
       pub source_files: Vec<String>,  // model_policy.toml, MODEL-POLICY.md
   }
   ```

2. Storage locations:
   - Filesystem cache: `.speckit/policies/snapshot-<POLICY_ID>.json`
   - Capsule storage: `mv2://.../policy/<POLICY_ID>`

3. Capture points:
   - Run start (capture active policy)
   - Stage boundary (if policy changed)
   - Events tagged with policy_id for traceability

4. API:
   - capture_policy_snapshot() -> Result<PolicySnapshot>
   - get_policy_for_run(run_id: &str) -> Result<PolicySnapshot>
   - list_policy_snapshots() -> Result<Vec<PolicySnapshotInfo>>

Files to create:
- stage0/src/policy.rs — PolicySnapshot struct and capture logic
- tui/src/memvid_adapter/policy_capture.rs — Integration with capsule

Decision IDs: D100, D101, D102

---

TASK 3: SPEC-978 Reflex Stack — SGLang Primary
Priority: MEDIUM
Decision: SGLang primary, vLLM fallback (OpenAI-compatible)

Deliverables:
1. ReflexBackend trait:
   ```rust
   #[async_trait]
   pub trait ReflexBackend: Send + Sync {
       async fn generate(&self, prompt: &str, schema: Option<&JsonSchema>) -> Result<String>;
       async fn generate_structured<T: DeserializeOwned>(&self, prompt: &str) -> Result<T>;
       fn name(&self) -> &str;
   }
   ```

2. Implementations:
   - SGLangBackend — Primary (constrained decoding, prefix caching)
   - VLLMBackend — Fallback (OpenAI-compatible endpoint)

3. Configuration:
   ```toml
   [reflex]
   primary = "sglang"
   fallback = "vllm"
   sglang_endpoint = "http://localhost:30000"
   vllm_endpoint = "http://localhost:8000"
   timeout_ms = 30000
   ```

4. Integration:
   - Wire into A/B harness for bakeoff testing
   - Capture reflex calls in capsule for replay

Files to create:
- stage0/src/reflex.rs — ReflexBackend trait + implementations
- stage0/src/config.rs — Add [reflex] section

Decision IDs: D110, D111, D112

---

TASK 4: Progress UI Enhancement
Priority: LOW
Decision: Add progress bars and streaming output

Deliverables:
- Add indicatif or similar for progress bars in golden_path_test.py
- Stream test output as it runs rather than batch at end
- Show elapsed time per step during execution

Files to modify:
- scripts/golden_path_test.py — Add progress reporting

===================================================================
EXECUTION ORDER
===================================================================

1. Start TASK 1 and TASK 2 in parallel (both HIGH priority)
2. Begin TASK 3 after initial progress on 1 and 2
3. TASK 4 is optional polish, do last if time permits

VERIFICATION COMMANDS
After completing each task, run:
```bash
cargo test -p codex-tui --lib -- memvid
cargo test -p codex-stage0 --lib
python3 scripts/doc_lint.py
python3 scripts/golden_path_test.py
```

ACCEPTANCE CRITERIA
- [ ] `speckit capsule checkpoints` lists checkpoints with ID and label
- [ ] `speckit capsule commit --label X` creates checkpoint
- [ ] resolve_uri works with branch and as_of parameters
- [ ] PolicySnapshot captured at run start
- [ ] PolicySnapshot stored in capsule and filesystem
- [ ] SGLang backend connects and generates
- [ ] vLLM fallback activates when SGLang unavailable
- [ ] All existing tests still pass (37 memvid + 260 stage0)
- [ ] Golden path validation still 10/10

OUTPUT EXPECTATION
- Code changes with tests for each task
- Update HANDOFF.md progress tracker after each commit
- Commit with SPEC-ID and Decision IDs
- Push after each logical unit of work
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
| SPEC-KIT-971 (CLI) | 🔄 5% | Implement checkpoint listing CLI |
| SPEC-KIT-977 | 🔄 0% | Create PolicySnapshot struct |
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
    MemvidMemoryAdapter (tui/src/memvid_adapter/)
            │
            └── CapsuleHandle
                    │
                    └── [future] memvid crate
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

### PolicySnapshot Flow (TO IMPLEMENT)

```
Run Start
    │
    ├── capture_policy_snapshot()
    │   ├── Read model_policy.toml + MODEL-POLICY.md
    │   ├── Compile to canonical JSON
    │   ├── Compute SHA256 hash
    │   └── Generate policy_id
    │
    ├── Store to filesystem: .speckit/policies/snapshot-{id}.json
    │
    ├── Store to capsule: mv2://.../policy/{id}
    │
    └── Tag all events with policy_id
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

## Files Changed This Session (2026-01-12)

| File | Change |
|------|--------|
| stage0/src/config.rs | MemoryBackend enum + memory_backend config field |
| stage0/src/lib.rs | Export MemoryBackend, HybridBackend, HybridConfig |
| stage0/src/hybrid.rs | **NEW** - HybridBackend with RRF/linear fusion |
| tui/src/memvid_adapter/adapter.rs | create_memory_client() factory |
| tui/src/memvid_adapter/eval.rs | run_ab_harness_and_save(), run_ab_harness_synthetic() |
| tui/src/memvid_adapter/mod.rs | Export create_memory_client, EvalRunResult |
| SPEC.md | **NEW** - Root docs contract with invariants |
| docs/PROGRAM_2026Q1_ACTIVE.md | **NEW** - Active program DAG |
| docs/DECISION_REGISTER.md | **NEW** - Locked decisions D1-D112 |
| scripts/doc_lint.py | **NEW** - Doc contract validator |
| scripts/golden_path_test.py | **NEW** - E2E validation (10/10 passed) |
| .githooks/pre-commit | Added doc lint integration |

---

## Test Summary

| Package | Tests | Status |
|---------|-------|--------|
| codex-tui (memvid) | 37 | ✅ All passing |
| codex-stage0 | 260 | ✅ All passing |
| Golden Path | 10/10 | ✅ All passing |
| Doc Lint | 0 errors | ✅ Passing |

---

## Quick Reference

### Build & Test
```bash
~/code/build-fast.sh              # Fast build
cargo test -p codex-tui --lib memvid  # Memvid tests
cargo test -p codex-stage0 --lib      # Stage0 tests
python3 scripts/doc_lint.py           # Doc contract lint
python3 scripts/golden_path_test.py   # Golden path E2E
```

### Key Paths
```
codex-rs/tui/src/memvid_adapter/  # Memvid implementation
codex-rs/stage0/src/              # Stage0 core (no Memvid dep)
codex-rs/stage0/src/hybrid.rs     # HybridBackend
codex-rs/SPEC.md                  # Root docs contract
docs/PROGRAM_2026Q1_ACTIVE.md     # Program DAG + gates
docs/DECISION_REGISTER.md         # Locked decisions
scripts/doc_lint.py               # Doc contract validator
scripts/golden_path_test.py       # E2E validation
```

---

*Generated by Claude Code session 2026-01-12*
