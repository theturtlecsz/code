# P87 Session Prompt – Stage 0 Pipeline Integration & Validation

## Prior Session Summary (P72-P86)

### Session Lineage

| Session | Focus | Key Deliverables |
|---------|-------|------------------|
| P72 | Research | SPEC-KIT-102 analysis, local-memory constraints discovered (closed-source Go) |
| P73-P74 | Foundation | Stage0Engine, overlay database pattern, run_stage0() |
| P75-P78 | DCC & Tier2 | Dynamic Context Compiler, Tier2 cache, NotebookLM integration |
| P79-P81 | Integration | /speckit.auto wiring, Divine Truth injection, Stage0Result |
| P82-P83 | Hybrid | TF-IDF backend, VectorBackend trait, hybrid retrieval signal |
| P84 | Hardening | Test coverage, edge cases, error handling |
| P85 | Code Brain | Shadow Code Brain V1, tree-sitter extraction, code lane in DCC |
| P86 | Eval Harness | P@K/R@K/MRR metrics, lane-aware evaluation, /stage0.eval-* commands |

### Key Commits

```
079318695 - feat(stage0): SPEC-KIT-102 P86 Eval Harness for Code Lane & Metrics
4659863e0 - feat(stage0): SPEC-KIT-102 P85 Shadow Code Brain V1 Implementation
165439ed6 - feat(stage0): SPEC-KIT-102 P84 Hardening
78f3ca8b8 - feat(stage0): SPEC-KIT-102 V2.5b TF-IDF Backend Wiring
fb8caa4a8 - feat(stage0): SPEC-KIT-102 V2.5 Hybrid Retrieval Integration
```

### Architectural Decisions

1. **Pure Rust** (not Python orchestrator) - overlay pattern avoids touching closed-source local-memory
2. **Separate overlay DB** - `stage0_overlay.db` never modifies local-memory's SQLite
3. **TF-IDF not Qdrant** - zero external dependencies, sufficient for hybrid boost
4. **Library integration** - direct function calls, no REST API overhead

---

## P87 Goal

**Primary Focus**: Test the full `/speckit.auto` pipeline flow with Stage 0 integration.

Validate that:
1. Stage 0 is invoked at pipeline start
2. `compile_context()` produces a valid TASK_BRIEF with memory + code context
3. Tier 2 cache is checked before NotebookLM calls
4. Divine Truth flows correctly to Stage 1 (SPECIFY)
5. Stage0Result is properly integrated into pipeline state

---

## Current State

### What's Working

- **Stage0Engine**: `run_stage0()` orchestrates DCC + Tier2 + causal ingestion
- **DCC**: `compile_context()` produces TASK_BRIEF with memory lane + code lane
- **Tier2 Cache**: TTL-based caching with hit/miss tracking
- **Hybrid Retrieval**: TF-IDF + local-memory + overlay scoring
- **Code Lane**: tree-sitter extraction → TF-IDF indexing → code candidates
- **Eval Harness**: P@K, R@K, MRR metrics with lane filtering

### Test Status

```bash
# Stage0 crate tests (127 passing)
cargo test -p codex-stage0

# TUI tests (507 passing)
cargo test -p codex-tui

# Eval-specific tests (26 passing)
cargo test -p codex-stage0 eval::
```

### Commands Available

| Command | Purpose |
|---------|---------|
| `/stage0.index` | Index memories + code into TF-IDF backend |
| `/stage0.eval-backend` | Run eval harness (--lane, --strict, --json) |
| `/stage0.eval-code` | Shortcut for code lane evaluation |
| `/speckit.seed` | Generate NL_* artifacts for NotebookLM seeding |

---

## Integration Points to Validate

### 1. Stage 0 Invocation

**Location**: `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`

```rust
// handle_spec_auto should call run_stage0() early in pipeline
let stage0_result = run_stage0(&spec_content, &spec_id, &cwd, &config).await?;
```

**Verify**:
- [ ] Stage 0 is called before Stage 1 (SPECIFY)
- [ ] Stage0Config is loaded from config file or defaults
- [ ] Errors are handled gracefully (fallback if Stage 0 fails)

### 2. TASK_BRIEF Generation

**Location**: `stage0/src/dcc.rs`

```rust
let result = compile_context(&spec_content, &idf_scores, &memories, &config)?;
// result.task_brief_md contains the compiled context
```

**Verify**:
- [ ] IQO extraction works on real spec content
- [ ] Memory candidates are retrieved and ranked
- [ ] Code candidates are included (if code_lane_enabled)
- [ ] Token budget is respected

### 3. Tier 2 Cache

**Location**: `stage0/src/overlay_db.rs`

```rust
// Cache check before NotebookLM
if let Some(cached) = check_tier2_cache(&cache_key)? {
    return Ok(cached);
}
```

**Verify**:
- [ ] Cache key is computed correctly (spec + brief hash)
- [ ] TTL is respected (24h default)
- [ ] Cache hits return quickly
- [ ] Cache misses trigger NotebookLM call

### 4. Divine Truth Injection

**Location**: `tui/src/chatwidget/spec_kit/stage0_integration.rs`

**Verify**:
- [ ] Divine Truth is extracted from Stage0Result
- [ ] It's injected into Stage 1 prompts
- [ ] Causal links are ingested back to local-memory

### 5. Pipeline State

**Verify**:
- [ ] SpecAutoState includes Stage 0 result
- [ ] Evidence is recorded for Stage 0
- [ ] Errors are logged as Stage0Events

---

## Quick Wins List

Small improvements that could be tackled in P87 if time permits:

| Priority | Item | Effort | Impact |
|----------|------|--------|--------|
| 1 | Add `/stage0.status` command to show current config/state | 30 min | Debugging |
| 2 | Add more built-in eval cases (5 memory + 5 code minimum) | 1 hr | Eval coverage |
| 3 | Improve IQO extraction for edge cases (empty specs, very long specs) | 1 hr | Robustness |
| 4 | Add `--verbose` flag to `/stage0.eval-backend` for detailed output | 30 min | Debugging |
| 5 | Cache pre-warming on `/stage0.index` completion | 1 hr | Performance |

---

## Configuration Examples

### Minimal Stage0 Config

```toml
# ~/.config/codex/stage0.toml
[stage0]
enabled = true

[stage0.tier2]
enabled = true
cache_ttl_hours = 24
```

### Full Config with Code Lane

```toml
[stage0]
enabled = true
explain_scores = true  # Verbose scoring breakdown

[stage0.tier2]
enabled = true
cache_ttl_hours = 24
notebook_id = "your-notebooklm-share-id"

[stage0.context]
max_tokens = 8000
top_k = 15
include_domains = ["spec-kit", "infrastructure"]

[stage0.scoring]
usage_weight = 0.30
recency_weight = 0.35
priority_weight = 0.25
decay_weight = 0.10

[stage0.code_lane]
enabled = true
code_top_k = 10

[stage0.hybrid]
enabled = true
vector_weight = 0.3
```

### Tier2-Disabled Config (for testing)

```toml
[stage0]
enabled = true

[stage0.tier2]
enabled = false  # Skip NotebookLM, use DCC output only
```

---

## Test Commands

```bash
# Full stage0 test suite
cargo test -p codex-stage0
# Expected: 127 tests passing

# Eval tests only
cargo test -p codex-stage0 eval::
# Expected: 26 tests passing

# TUI tests (includes Stage 0 integration)
cargo test -p codex-tui
# Expected: 507 tests passing

# Run eval harness manually
/stage0.index              # Index memories + code
/stage0.eval-backend       # Run full eval (both lanes)
/stage0.eval-code          # Code lane only

# Pipeline integration test
/speckit.auto SPEC-KIT-900  # Neutral workload spec for validation
```

---

## Reference Documentation

| Document | Purpose |
|----------|---------|
| `docs/SPEC-KIT-102R-implementation-report/spec.md` | Authoritative implementation report |
| `docs/SPEC-KIT-103-librarian/spec.md` | Phase 3 roadmap (Librarian) |
| `docs/SPEC-KIT-104-metrics-learning/spec.md` | Phase 4 roadmap (Metrics) |
| `SPEC.md` (Stage 0 section) | Tracking table and phase status |

---

## Success Criteria for P87

1. **Pipeline Integration Verified**: `/speckit.auto` successfully invokes Stage 0 and uses its output
2. **TASK_BRIEF Quality**: DCC produces meaningful context with both memory and code candidates
3. **Cache Behavior Confirmed**: Tier 2 cache hits/misses work as expected
4. **Divine Truth Flows**: Stage 1 receives and uses Divine Truth in prompts
5. **Tests Pass**: All 127 stage0 + 507 TUI tests remain green
6. **No Regressions**: Existing `/speckit.auto` functionality unchanged

---

## Notes for Next Session

- Run `/speckit.auto SPEC-KIT-900` as the integration test target
- Check Stage0Events are being logged correctly
- Verify NotebookLM MCP bridge is authenticated (`mcp__notebooklm__get_health`)
- If Tier 2 is slow, consider testing with `tier2.enabled = false` first
- The eval harness can help diagnose retrieval quality issues
