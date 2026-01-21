# SPEC-KIT-979 — Migration: local-memory → Memvid + Sunset

**Date:** 2026-01-20
**Status:** READY FOR IMPLEMENTATION
**Owner (role):** Platform Eng
**Version:** 2.0

---

## Summary

Migrate fully off the local-memory daemon after parity gates pass. This spec defines:
1. **Parity gates** — measurable criteria for memvid to become default
2. **Rollout phases** — phased migration from opt-in to default to removal
3. **Rollback/escape hatches** — safe fallback mechanisms
4. **Acceptance tests** — named CI-enforceable tests
5. **CLI/config** — user-facing controls

---

## Implementation Status

> **Audit Date:** 2026-01-21 | **Auditor:** SPEC-KIT-979 drift audit

| Component | Status | Location |
|-----------|--------|----------|
| Parity gate definitions | DEFINED | This spec §Parity Gates |
| Parity gate tests (`test_parity_gate_*`) | NOT STARTED | — |
| ABHarness infrastructure | IMPLEMENTED | `tui/src/memvid_adapter/eval.rs` |
| Memory backend routing | IMPLEMENTED | `tui/src/chatwidget/spec_kit/stage0_integration.rs` |
| lm-import (basic: format, dedupe, dry-run) | IMPLEMENTED | `~/.local-memory/scripts/lm-import.sh` |
| lm-import (--status, --all, --verify, --resume) | NOT STARTED | — |
| CLI flags (--memory-backend, --eval-ab, etc.) | NOT STARTED | — |
| Deprecation warning banners | NOT STARTED | — |
| nightly.yml workflow | IMPLEMENTED | `.github/workflows/nightly-parity.yml` |
| release.yml workflow | NOT STARTED | — |

---

## Decision IDs

**Implemented by this spec:** D14, D39, D40, D52, D94

| ID | Decision | Implementation |
|----|----------|----------------|
| D14 | Logical URIs immutable once returned | `mv2://` scheme enforced |
| D39 | Dual-backend runtime flag for A/B comparison | `ABHarness` in `eval.rs` |
| D40 | Parity gates for migration | This spec §Parity Gates |
| D52 | Migration tool for local-memory corpus | `lm-import` command |
| D94 | Feature flag for local-memory removal | `--disable-local-memory` |

**Referenced (must remain consistent):** D53 (deprecation plan), D35 (export UX)

**Explicitly out of scope:** D60 (multi-tenant hosted service)

---

## Goals

1. Memvid becomes default backend with zero retrieval regressions
2. Safe rollback available at all phases
3. CI gates prevent regression
4. Clear deprecation timeline for local-memory daemon

## Non-Goals

- Hosted multi-tenant memory service (D60)
- Schema changes to LocalMemoryClient trait (frozen)
- Immediate removal of local-memory (phased over 90 days)

---

## Parity Gates

### Gate 1: Retrieval Quality Parity (GATE-RQ)

**Criteria:** Memvid search quality ≥ 95% of local-memory baseline

| Metric | Threshold | Measurement |
|--------|-----------|-------------|
| Mean Precision@10 | ≥ 0.95 × baseline | `ABReport.suite_b.mean_precision` |
| Mean Recall@10 | ≥ 0.95 × baseline | `ABReport.suite_b.mean_recall` |
| MRR | ≥ 0.95 × baseline | `ABReport.suite_b.mrr` |
| Golden queries passed | 100% | All `expected_ids` found |

**Test (to be implemented):** `test_parity_gate_retrieval_quality`
```bash
# Planned command (test does not exist yet)
cargo test -p codex-tui -- parity_gate_retrieval_quality --ignored
```

### Gate 2: Latency Parity (GATE-LP)

**Criteria:** P95 search latency < 250ms (D29)

| Metric | Threshold | Measurement |
|--------|-----------|-------------|
| P95 latency | < 250ms | `ABReport.p95_latency_b()` |
| P50 latency | < 100ms | Informational |
| Max latency | < 1000ms | No outliers |

**Test (to be implemented):** `test_parity_gate_latency`
```bash
# Planned command (test does not exist yet)
cargo test -p codex-tui -- parity_gate_latency --ignored
```

### Gate 3: Stability (GATE-ST)

**Criteria:** 30 consecutive days without fallback activation or data loss

| Metric | Threshold | Measurement |
|--------|-----------|-------------|
| Fallback activations | 0 | Counter in telemetry |
| Data loss incidents | 0 | Manual verification |
| Crash/recovery cycles | All successful | WAL integrity |
| Stability days | ≥ 30 | Calendar tracking |

**Test (to be implemented):** `test_parity_gate_stability` (manual + telemetry review)

### Gate 4: Feature Parity (GATE-FP)

**Criteria:** All local-memory features have memvid equivalents

| Feature | Local-Memory | Memvid | Parity |
|---------|--------------|--------|--------|
| Keyword search | `lm search` | `capsule search` | ✅ |
| Domain filtering | `--domain` | IQO.domains | ✅ |
| Tag filtering | `--tags` | IQO.required_tags | ✅ |
| Importance filter | `--min-importance` | IQO threshold | ✅ |
| Memory zoom | `lm zoom <id>` | `capsule zoom` | ✅ |
| Health check | `lm health` | `capsule doctor` | ✅ |
| Export | `lm export` | `capsule export` | ✅ |
| Import | N/A (new) | `lm-import` | ✅ |

**Test (to be implemented):** `test_parity_gate_features`

---

## Rollout Phases

### Phase 0: Opt-In Memvid (CURRENT)

**Status:** Active
**Duration:** Until GATE-RQ + GATE-LP pass

**Configuration:**
```toml
# stage0.toml (default)
memory_backend = "local-memory"

# Opt-in to memvid
memory_backend = "memvid"
```

**Behavior:**
- local-memory is default
- Memvid available via explicit config
- Fallback: memvid → local-memory on capsule failure
- A/B harness available for evaluation

**CLI:**
```bash
# Check current backend
codex-tui --show-config | grep memory_backend

# Run A/B evaluation
codex-tui --eval-ab --output-dir .speckit/eval/
```

### Phase 1: Default Memvid for New Projects

**Entry Criteria:** GATE-RQ + GATE-LP passed for 7 days
**Duration:** 30 days (GATE-ST observation)

**Configuration:**
```toml
# stage0.toml (new default)
memory_backend = "memvid"

# Override to local-memory (escape hatch)
memory_backend = "local-memory"
```

**Behavior:**
- Memvid is default for new `codex-tui` sessions
- Existing projects keep their config
- Fallback: memvid → local-memory still active
- Deprecation warning shown when using local-memory

**CLI:**
```bash
# Force local-memory (escape hatch)
codex-tui --memory-backend local-memory

# Suppress deprecation warning
codex-tui --no-deprecation-warnings
```

**Deprecation Warning (defined message format — not yet implemented):**
```
⚠️  local-memory backend is deprecated and will be removed in Phase 3.
    Run `lm-import` to migrate your memories to memvid.
    See: docs/SPEC-KIT-979-local-memory-sunset/MIGRATION.md
```

### Phase 2: Deprecate Local-Memory (Warn)

**Entry Criteria:** GATE-ST passed (30 days stability)
**Duration:** 60 days

**Configuration:**
```toml
# stage0.toml
memory_backend = "memvid"  # Only option in config schema

# CLI override still available
# codex-tui --memory-backend local-memory --force-deprecated
```

**Behavior:**
- Config file only accepts `memvid`
- CLI flag `--memory-backend local-memory` requires `--force-deprecated`
- Fallback disabled by default (enable via `--enable-fallback`)
- Strong warning on every session start if local-memory data exists

**CLI:**
```bash
# Use deprecated local-memory (requires explicit flag)
codex-tui --memory-backend local-memory --force-deprecated

# Check migration status
lm-import --status

# Complete migration
lm-import --all --verify
```

**Strong Warning (defined message format — not yet implemented):**
```
🚨 DEPRECATED: local-memory backend will be removed in 60 days.
   Unmigrated memories: 147
   Run: lm-import --all --verify

   To continue with local-memory, add --force-deprecated
```

### Phase 3: Remove Local-Memory (or Legacy Plugin)

**Entry Criteria:** Phase 2 complete + 60 days
**Duration:** Permanent

**Options:**
1. **Full Removal:** Delete LocalMemoryCliAdapter, remove daemon dependency
2. **Legacy Plugin:** Move to optional `codex-tui-legacy-memory` crate

**Configuration:**
```toml
# stage0.toml
memory_backend = "memvid"  # Only valid value

# Legacy plugin (if kept)
[plugins]
legacy_memory = { enabled = true, path = "codex-tui-legacy-memory" }
```

**Behavior:**
- `memory_backend = "local-memory"` is a config error
- CLI flag `--memory-backend local-memory` removed
- Fallback mechanism removed
- Legacy plugin (option 2) provides escape hatch for edge cases

---

## Rollback / Escape Hatches

### Rollback 1: Config Switch (Phase 0-1)

```toml
# Instant rollback via config
memory_backend = "local-memory"
```

### Rollback 2: CLI Override (Phase 1-2)

```bash
# Session-level rollback
codex-tui --memory-backend local-memory

# Phase 2 requires additional flag
codex-tui --memory-backend local-memory --force-deprecated
```

### Rollback 3: Fallback Mechanism (Phase 0-1)

Automatic fallback when capsule fails to open:
```
1. Try memvid (capsule.open())
2. On failure → check local-memory health
3. If healthy → use local-memory with warning
4. If unhealthy → error with instructions
```

**Config to disable fallback:**
```toml
[system_of_record]
fallback_enabled = false  # Fail hard if memvid unavailable
```

### Rollback 4: Environment Override (Emergency)

```bash
# Emergency override (all phases)
CODE_MEMORY_BACKEND=local-memory codex-tui
```

### Rollback 5: Feature Flag (Compile-Time)

```bash
# Build without memvid (emergency rollback)
cargo build -p codex-tui --no-default-features --features local-memory-only
```

---

## Acceptance Tests

### Unit Tests (CI-Required)

| Test Name | Gate | Description |
|-----------|------|-------------|
| `test_memvid_adapter_search` | GATE-FP | Basic search returns results |
| `test_memvid_domain_filtering` | GATE-FP | Domain filter works correctly |
| `test_memvid_tag_filtering` | GATE-FP | Tag filter works correctly |
| `test_memvid_importance_filter` | GATE-FP | Importance threshold works |
| `test_capsule_open_close` | GATE-ST | Capsule lifecycle management |
| `test_capsule_crash_recovery` | GATE-ST | WAL recovery after crash |

### Integration Tests (CI-Required)

| Test Name | Gate | Description |
|-----------|------|-------------|
| `test_ab_harness_runs` | GATE-RQ | A/B harness executes without error |
| `test_golden_queries_coverage` | GATE-RQ | All golden queries have test data |
| `test_backend_switch` | GATE-FP | Config switch changes backend |
| `test_fallback_activation` | GATE-ST | Fallback triggers on capsule failure |
| `test_deprecation_warning` | Phase 1 | Warning shown for local-memory |

### Parity Gate Tests (CI-Blocking after Phase 0)

| Test Name | Gate | Threshold |
|-----------|------|-----------|
| `test_parity_gate_retrieval_quality` | GATE-RQ | P@10 ≥ 0.95 × baseline |
| `test_parity_gate_latency` | GATE-LP | P95 < 250ms |
| `test_parity_gate_stability` | GATE-ST | Manual + telemetry |
| `test_parity_gate_features` | GATE-FP | All features pass |

### Golden Query Tests (Regression Prevention)

| Test Name | Queries | Expected |
|-----------|---------|----------|
| `test_golden_keyword_search` | 3 queries | All expected IDs found |
| `test_golden_domain_filter` | 2 queries | Domain filtering works |
| `test_golden_tag_filter` | 2 queries | Tag filtering works |
| `test_golden_combined` | 2 queries | Combined filters work |
| `test_golden_edge_cases` | 4 queries | Edge cases handled |
| `test_golden_no_match` | 1 query | Empty result for no match |

---

## CI Gates

> **Note:** The workflow schemas below are **proposed designs**. The actual workflow files do not exist yet. See Implementation Status above.

### Pre-Merge (All PRs) — Proposed

```yaml
# .github/workflows/ci.yml (proposed addition)
- name: Memory Backend Tests
  run: |
    cargo test -p codex-tui -- memvid_adapter
    cargo test -p codex-stage0 -- dcc
```

### Nightly (Parity Validation) — Proposed

_The following workflow will be created after parity gate tests are implemented:_

```yaml
# .github/workflows/nightly.yml (does not exist yet)
- name: Parity Gate Validation
  run: |
    cargo test -p codex-tui -- parity_gate --ignored

- name: A/B Harness Report
  run: |
    cargo run -p codex-tui -- --eval-ab --output-dir artifacts/eval/

- name: Upload Eval Report
  uses: actions/upload-artifact@v4
  with:
    name: ab-eval-report
    path: artifacts/eval/
```

### Release Gate (Phase Transitions) — Proposed

_The following workflow will be created for phase transition gating:_

```yaml
# .github/workflows/release.yml (does not exist yet)
phase_1_gate:
  needs: [parity_tests, stability_check]
  if: |
    needs.parity_tests.outputs.gate_rq == 'passed' &&
    needs.parity_tests.outputs.gate_lp == 'passed'
  steps:
    - name: Verify 7-Day Parity
      run: scripts/verify_parity_duration.sh 7

phase_2_gate:
  needs: [phase_1_gate, stability_check]
  if: needs.stability_check.outputs.days >= 30
  steps:
    - name: Verify 30-Day Stability
      run: scripts/verify_stability.sh 30
```

---

## CLI Reference

> **Note:** The CLI flags below are **planned for implementation**. They do not exist in `cli.rs` yet. See Implementation Status above.

### Backend Selection — Planned

```bash
# Check current backend (planned)
codex-tui --show-config | grep memory_backend

# Override backend for session (planned)
codex-tui --memory-backend memvid
codex-tui --memory-backend local-memory

# Phase 2+: Force deprecated backend (planned)
codex-tui --memory-backend local-memory --force-deprecated
```

### Migration Commands

**Currently implemented** (`~/.local-memory/scripts/lm-import.sh`):
```bash
# Import with format auto-detection
lm-import SOURCE

# Specify format explicitly
lm-import --format json|obsidian|markdown|backup SOURCE

# Set domain for imported memories
lm-import --domain my-domain SOURCE

# Set default importance
lm-import --importance 7 SOURCE

# Check for duplicates before import
lm-import --dedupe SOURCE

# Dry run (show what would be imported)
lm-import --dry-run SOURCE
```

**Planned extensions** (not yet implemented):
```bash
# Check migration status
lm-import --status

# Migrate all memories
lm-import --all

# Migrate with verification
lm-import --all --verify

# Resume interrupted migration
lm-import --resume
```

### Evaluation Commands — Planned

```bash
# Run A/B evaluation (planned - ABHarness exists but CLI flag not wired)
codex-tui --eval-ab --output-dir .speckit/eval/

# Run with custom golden queries (planned)
codex-tui --eval-ab --golden-queries golden.json

# Run synthetic test (no real data) (planned)
codex-tui --eval-ab --synthetic
```

### Health & Diagnostics — Planned

```bash
# Capsule health check (planned)
codex-tui --capsule-doctor

# Verify capsule integrity (planned)
codex-tui --capsule-verify

# Show backend status (planned)
codex-tui --backend-status
```

---

## Config Reference

### stage0.toml

```toml
# Memory backend selection
# Values: "memvid" (default in Phase 1+), "local-memory" (deprecated)
memory_backend = "memvid"

# Fallback configuration
[system_of_record]
primary = "memvid"
fallback = "local-memory"
fallback_enabled = true  # false in Phase 2+

# Parity gate thresholds (from model_policy.toml)
[gates.local_memory_sunset]
retrieval_p95_parity = true
search_quality_parity = true
stability_days = 30
zero_fallback_activations = true
```

### Environment Variables

```bash
# Override memory backend (emergency)
CODE_MEMORY_BACKEND=local-memory

# Skip deprecation warnings
CODE_NO_DEPRECATION_WARNINGS=1

# Force parity gate checks
CODE_FORCE_PARITY_GATES=1
```

---

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Memvid API churn | Medium | Pin versions; adapter boundary; contract tests |
| Retrieval regressions | High | A/B harness; golden queries; CI blocking |
| Data loss during migration | High | `lm-import --verify`; backup before migration |
| Fallback loops | Medium | Circuit breaker; fallback counter limit |
| Performance degradation | Medium | P95 gate; nightly benchmarks |

---

## Migration Guide

See: [MIGRATION.md](./MIGRATION.md) (to be created)

Quick start (uses planned commands marked with ⚠️):
```bash
# 1. Check current status (⚠️ --status flag not yet implemented)
lm-import --status

# 2. Backup local-memory (recommended)
lm export --all --output backup-$(date +%Y%m%d).json

# 3. Migrate with verification (⚠️ --all --verify not yet implemented)
lm-import --all --verify

# 4. Switch backend (config change — works today)
echo 'memory_backend = "memvid"' >> ~/.config/code/stage0.toml

# 5. Verify (⚠️ --backend-status not yet implemented)
codex-tui --backend-status
```

---

## Timeline

| Phase | Start | Duration | Entry Criteria |
|-------|-------|----------|----------------|
| Phase 0 | Current | Until gates pass | N/A |
| Phase 1 | GATE-RQ + GATE-LP | 30 days | 7 days parity |
| Phase 2 | Phase 1 + 30 days | 60 days | GATE-ST (30 days) |
| Phase 3 | Phase 2 + 60 days | Permanent | 60 days deprecation |

**Total Migration Timeline:** ~150 days from first parity gate pass

---

## Dependencies

- **SPEC-KIT-971**: Memvid capsule foundation (LOCKED)
- **SPEC-KIT-972**: Hybrid retrieval eval (LOCKED)
- **model_policy.toml**: `[gates.local_memory_sunset]` section

---

## References

- `codex-rs/tui/src/memvid_adapter/eval.rs` — A/B harness implementation
- `codex-rs/stage0/src/config.rs` — MemoryBackend enum
- `codex-rs/model_policy.toml` — Gate thresholds
- `docs/DECISION_REGISTER.md` — D14, D39, D40, D52, D94
