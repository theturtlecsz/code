# P84 Session Prompt – HARDENING STAGE 0 (Cache TTL, Hybrid Signaling, Seeder Hooks)

**Ultrathink** Continue SPEC-KIT-102 Stage 0 Integration - Phase P84: Hardening

## Prior Session (P83) Completed

- V2.5b Hybrid Retrieval Integration fully wired
- `VectorIndexConfig` added to stage0 config (`max_memories_to_index`)
- `get_memory_ids_for_indexing()` added to OverlayDb
- `VectorState` created in TUI (`Arc<RwLock<TfIdfBackend>>`)
- `/stage0.index` populates shared VectorState
- `run_stage0_blocking` uses shared backend when available
- `/stage0.eval-backend` compares baseline vs hybrid with JSON output
- Integration tests in `tui/tests/stage0_local_memory_integration.rs`
- Sample eval cases in `evidence/vector_eval_cases.json`
- **109 stage0 tests pass** (1 pre-existing flaky: `cache_ttl_respected`)

Reference: docs/HANDOFF-P84-PROMPT.md

## P84 Scope: Hardening & Signals

This session focuses on THREE hardening areas:
1. Fix the flaky Tier2 cache TTL test and lock in cache semantics
2. Expose hybrid usage & Stage0 signals to ExecutionLogger (event structure only)
3. Tighten seeder + Tier2 relationship with index headers and prompt updates

---

### 1. TIER2 CACHE TTL – FIX THE FLAKY TEST AND SEMANTICS [PRIORITY]

**Files:** `stage0/src/overlay_db.rs`, `stage0/src/lib.rs`

The `cache_ttl_respected` test is flaky due to wall-clock timing issues.

**Tasks:**

1.1 **Revisit TTL semantics** in `OverlayDb`:
   - Create `get_tier2_cache_with_ttl(input_hash, ttl_hours, now: DateTime<Utc>)` method
   - Single source of truth: `created_at + TTL` checked at read time
   - Do NOT use separate `expires_at` field – compute freshness at query time
   - Return `Ok(None)` if entry exists but expired

1.2 **Stabilize `cache_ttl_respected` test:**
   - Use FIXED timestamps instead of `Utc::now()` inside test
   - Create `base_time = DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")`
   - Insert cache entry with `created_at = base_time`
   - Test cases:
     - `now = base_time + Duration::hours(ttl - 1)` → expect `Some(entry)`
     - `now = base_time + Duration::hours(ttl + 1)` → expect `None`
   - NO wall-clock dependency, NO `Instant::now()` for TTL logic

1.3 **Unify `now` in `run_stage0`:**
   - Capture single `let now = Utc::now();` at call boundary
   - Pass same `now` to: cache lookup, cache insert, usage updates
   - Use separate `Instant::now()` ONLY for `latency_ms` measurement

---

### 2. STAGE0 SIGNALING – EVENT STRUCTURE ONLY (NO METRICS CRATE)

**Files:** `tui/src/chatwidget/spec_kit/execution_logger.rs`, `stage0_integration.rs`

**Design Decision:** Shape events for metrics compatibility but DO NOT add metrics crate integration. Leave clean seam for future SPEC.

**Tasks:**

2.1 **Confirm `hybrid_retrieval_used` flows through:**
   - Already added to `Stage0ExecutionResult` in P83
   - Verify it's set from `run_stage0_blocking` return tuple
   - Trace through: run_stage0_blocking → Stage0ExecutionResult → Stage0Complete event

2.2 **Extend `ExecutionEvent::Stage0Complete` payload:**
   ```rust
   Stage0Complete {
       run_id: String,
       spec_id: String,
       duration_ms: u64,
       tier2_used: bool,           // Tier2 enabled AND (cache_hit OR call succeeded)
       cache_hit: bool,            // From Stage0Result.cache_hit
       hybrid_used: bool,          // From Stage0ExecutionResult.hybrid_retrieval_used
       memories_used: usize,       // res.memories_used.len()
       task_brief_written: bool,   // true if TASK_BRIEF.md write succeeded
       skip_reason: Option<String>, // "disabled", "no MCP", "no memories", etc.
       timestamp: String,
   }
   ```

2.3 **Add structured tracing (no metrics calls):**
   ```rust
   tracing::info!(
       target: "stage0",
       event_type = "Stage0Complete",
       spec_id,
       result = "success",
       duration_ms,
       tier2_used,
       cache_hit,
       hybrid_used,
       memories_used,
       task_brief_written,
       skip_reason = skip_reason.as_deref().unwrap_or(""),
       "Stage 0 completed"
   );
   ```

2.4 **Document metrics shape** (for future SPEC):
   - Add comment block noting intended counters:
     - `stage0_runs_total{result=success|error|skipped}`
     - `stage0_tier2_calls_total{outcome=success|fallback|error}`
     - `stage0_hybrid_used_total{value=true|false}`
   - Do NOT add metrics crate dependency

---

### 3. SEEDER + TIER2 RELATIONSHIP – INDEX HEADERS ONLY (NO DB TRACKING)

**Files:** `tui/src/chatwidget/spec_kit/stage0_seeding.rs`, Tier2 prompt templates

**Design Decision:** Add index headers with timestamps directly in NL_* files. Do NOT add DB-level `seeding_at` tracking – defer to separate SPEC if needed.

**Tasks:**

3.1 **Add index section to NL_* artifact generation:**
   ```markdown
   # NL_ARCHITECTURE_BIBLE

   > Seeded by Stage0 on 2025-12-01T13:45:22Z
   > Source: codex-rs commit abc123

   ## Index
   - Core subsystems (TUI, Stage0, local-memory integration)
   - Design decisions:
     - tokio vs async-std
     - ratatui vs cursive
     - staging vs production profiles
   ```
   - Include timestamp from `Utc::now()` at generation time
   - Include git commit hash if available (`git rev-parse --short HEAD`)
   - Generate index based on content sections detected

3.2 **Update Tier2 prompt to reference NL_* explicitly:**
   - In `STAGE0_TIER2_PROMPT.md` or Tier2Client prompt builder, add:
   > "Your seeded knowledge includes files prefixed NL_ARCHITECTURE_BIBLE,
   > NL_BUG_RETROS, NL_DEBT_LANDSCAPE, NL_PROJECT_DIARY that summarize
   > architecture decisions, bug patterns, technical debt, and project history.
   > Cross-reference these sources when answering."

3.3 **Skip DB-level seeding_at for now:**
   - Do NOT add `seeding_meta` table or overlay schema changes
   - Timestamps in artifact headers provide sufficient traceability
   - DB tracking can be separate SPEC when routing logic needs it

---

## OUT OF SCOPE FOR P84

- Full metrics crate integration (defer to telemetry SPEC)
- DB-level seeding version tracking (timestamps in files suffice)
- Fixing pre-existing dead code warnings (show_prd_builder, etc.) – separate PR
- New features for hybrid retrieval or VectorState

---

## Success Criteria

- [ ] `cargo test -p codex-stage0` passes (ALL tests including `cache_ttl_respected`)
- [ ] `get_tier2_cache_with_ttl()` added with injected `now` parameter
- [ ] `cache_ttl_respected` test uses fixed timestamps, no wall-clock
- [ ] `ExecutionEvent::Stage0Complete` includes tier2_used, cache_hit, hybrid_used fields
- [ ] Structured tracing logs all Stage0 signals at INFO level
- [ ] NL_* artifacts have index headers with generation timestamp
- [ ] Tier2 prompt explicitly mentions NL_* artifact names

---

## Design Decisions (Confirmed)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Cache TTL | `created_at + TTL` at read time | No expires_at field, simpler schema |
| Timestamps | Single `now` per run_stage0 | Consistency for cache logic |
| Metrics | Event structure only | Defer crate choice to app-level SPEC |
| Seeding tracking | Timestamps in NL_* headers | No DB changes, sufficient traceability |
| Warning cleanup | Defer to separate PR | Keep P84 focused on Stage0 hardening |

---

## Files to Modify

```
stage0/src/overlay_db.rs          # get_tier2_cache_with_ttl()
stage0/src/lib.rs                 # run_stage0 timestamp unification
stage0/src/lib.rs (tests)         # cache_ttl_respected fix
tui/src/chatwidget/spec_kit/execution_logger.rs  # Stage0Complete fields
tui/src/chatwidget/spec_kit/stage0_integration.rs  # event emission
tui/src/chatwidget/spec_kit/stage0_seeding.rs     # index headers
docs/STAGE0_TIER2_PROMPT.md       # NL_* artifact references
```

---

## Estimated Test Coverage After P84

- 110 stage0 tests passing (fix +1)
- TUI stage0 integration tests passing
- local-memory integration tests passing (env-gated)
