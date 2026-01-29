# SPEC-KIT-979 — Migration: local-memory → Memvid + Sunset

**Date:** 2026-01-20
**Status:** COMPLETED
**Completed:** 2026-01-21
**Owner (role):** Platform Eng
**Version:** 2.0

> **Implementation status:** Canonical completion tracker: `codex-rs/SPEC.md` Completed (Recent).

***

## Summary

Migrate fully off the local-memory daemon after parity gates pass. This spec defines:

1. **Parity gates** — measurable criteria for memvid to become default
2. **Rollout phases** — phased migration from opt-in to default to removal
3. **Rollback/escape hatches** — safe fallback mechanisms
4. **Acceptance tests** — named CI-enforceable tests
5. **CLI/config** — user-facing controls

***

## Implementation Status

> **Audit Date:** 2026-01-21 | **Auditor:** Architect reconciliation audit

| Component                                      | Status                    | Location                                            |
| ---------------------------------------------- | ------------------------- | --------------------------------------------------- |
| Parity gate definitions                        | DEFINED                   | This spec §Parity Gates                             |
| Parity gate tests (`test_parity_gate_*`)       | IMPLEMENTED (`#[ignore]`) | `tui/src/memvid_adapter/eval.rs:1595-2078`          |
| ABHarness infrastructure                       | IMPLEMENTED               | `tui/src/memvid_adapter/eval.rs`                    |
| Memory backend routing                         | IMPLEMENTED               | `tui/src/chatwidget/spec_kit/stage0_integration.rs` |
| lm-import (`code local-memory import`)         | IMPLEMENTED               | `cli/src/local_memory_cmd/import.rs`                |
| lm-import (--status, --all, --verify)          | IMPLEMENTED               | `cli/src/local_memory_cmd/import.rs:28-69`          |
| CLI flags (--memory-backend, --eval-ab, etc.)  | IMPLEMENTED               | `tui/src/cli.rs:149-180`                            |
| Deprecation warning banners                    | IMPLEMENTED               | `tui/src/memvid_adapter/sunset_phase.rs`            |
| Phase enforcement (0-3)                        | IMPLEMENTED               | `tui/src/memvid_adapter/sunset_phase.rs`            |
| Event types (PhaseResolved, FallbackActivated) | IMPLEMENTED               | `tui/src/memvid_adapter/types.rs`                   |
| nightly-parity.yml workflow                    | IMPLEMENTED               | `.github/workflows/nightly-parity.yml`              |
| release.yml workflow                           | NOT STARTED               | —                                                   |

**Note:** Parity gate tests are `#[ignore]` and run in nightly workflow. They use synthetic mode by default; real dual-backend mode requires infrastructure setup (TODO).

***

## Decision IDs

**Implemented by this spec:** D14, D39, D40, D52, D94

| ID  | Decision                                     | Implementation           |
| --- | -------------------------------------------- | ------------------------ |
| D14 | Logical URIs immutable once returned         | `mv2://` scheme enforced |
| D39 | Dual-backend runtime flag for A/B comparison | `ABHarness` in `eval.rs` |
| D40 | Parity gates for migration                   | This spec §Parity Gates  |
| D52 | Migration tool for local-memory corpus       | `lm-import` command      |
| D94 | Feature flag for local-memory removal        | `--disable-local-memory` |

**Referenced (must remain consistent):** D53 (deprecation plan), D35 (export UX)

**Explicitly out of scope:** D60 (multi-tenant hosted service)

***

## Goals

1. Memvid becomes default backend with zero retrieval regressions
2. Safe rollback available at all phases
3. CI gates prevent regression
4. Clear deprecation timeline for local-memory daemon

## Non-Goals

* Hosted multi-tenant memory service (D60)
* Schema changes to LocalMemoryClient trait (frozen)
* Immediate removal of local-memory (phased over 90 days)

***

## Parity Gates

### Gate 1: Retrieval Quality Parity (GATE-RQ)

**Criteria:** Memvid search quality ≥ 95% of local-memory baseline

| Metric                | Threshold         | Measurement                       |
| --------------------- | ----------------- | --------------------------------- |
| Mean Precision\@10    | ≥ 0.95 × baseline | `ABReport.suite_b.mean_precision` |
| Mean Recall\@10       | ≥ 0.95 × baseline | `ABReport.suite_b.mean_recall`    |
| MRR                   | ≥ 0.95 × baseline | `ABReport.suite_b.mrr`            |
| Golden queries passed | 100%              | All `expected_ids` found          |

**Test:** `test_parity_gate_retrieval_quality` (implemented, `#[ignore]`, runs nightly)

```bash
# Runs in synthetic mode by default; validates infrastructure
cargo test -p codex-tui --lib -- parity_gate_retrieval_quality --ignored
```

### Gate 2: Latency Parity (GATE-LP)

**Criteria:** P95 search latency < 250ms (D29)

| Metric      | Threshold | Measurement                |
| ----------- | --------- | -------------------------- |
| P95 latency | < 250ms   | `ABReport.p95_latency_b()` |
| P50 latency | < 100ms   | Informational              |
| Max latency | < 1000ms  | No outliers                |

**Test:** `test_parity_gate_latency` (implemented, `#[ignore]`, runs nightly)

```bash
# CI-aware: relaxed threshold in CI environments
cargo test -p codex-tui --lib -- parity_gate_latency --ignored
```

### Gate 3: Stability (GATE-ST)

**Criteria:** 30 consecutive days without fallback activation or data loss

| Metric                | Threshold      | Measurement          |
| --------------------- | -------------- | -------------------- |
| Fallback activations  | 0              | Counter in telemetry |
| Data loss incidents   | 0              | Manual verification  |
| Crash/recovery cycles | All successful | WAL integrity        |
| Stability days        | ≥ 30           | Calendar tracking    |

**Test:** `test_parity_gate_stability` (implemented, `#[ignore]`, manual + telemetry review)

Supports `STABILITY_REPORT_PATH` env var for external report injection. Validates 30-day duration, zero fallbacks via `FallbackActivated` event counts.

### Gate 4: Feature Parity (GATE-FP)

**Criteria:** All local-memory features have memvid equivalents

| Feature           | Local-Memory       | Memvid             | Parity |
| ----------------- | ------------------ | ------------------ | ------ |
| Keyword search    | `lm search`        | `capsule search`   | ✅      |
| Domain filtering  | `--domain`         | IQO.domains        | ✅      |
| Tag filtering     | `--tags`           | IQO.required\_tags | ✅      |
| Importance filter | `--min-importance` | IQO threshold      | ✅      |
| Memory zoom       | `lm zoom <id>`     | `capsule zoom`     | ✅      |
| Health check      | `lm health`        | `capsule doctor`   | ✅      |
| Export            | `lm export`        | `capsule export`   | ✅      |
| Import            | N/A (new)          | `lm-import`        | ✅      |

**Test:** `test_parity_gate_features` (implemented, `#[ignore]`, runs nightly)

Tests 7 feature scenarios: keywords only, domain filtering, required tags, optional tags, empty keywords with tags, no match, combined filters.

### Parity Test Modes

**Implementation:** `tui/src/memvid_adapter/eval.rs`

| Mode        | Description                            | When Used                                         |
| ----------- | -------------------------------------- | ------------------------------------------------- |
| `synthetic` | Self-comparison using synthetic corpus | Default in CI/nightly (infrastructure validation) |
| `real`      | Actual dual-backend comparison         | Requires infrastructure setup (TODO)              |
| `fallback`  | Track fallback events for GATE-ST      | Via `FallbackActivated` capsule events            |

**Note:** Tests marked `#[ignore]` run in synthetic mode by default. Real dual-backend mode requires both memvid capsule and local-memory daemon to be healthy.

***

## Phase Configuration

**Implementation:** `tui/src/memvid_adapter/sunset_phase.rs:87-116`

### Configuration Sources (Priority Order)

1. **Environment variable:** `CODE_SUNSET_PHASE` (values: 0-3)
2. **Policy file:** `model_policy.toml` → `[gates.local_memory_sunset].current_phase`
3. **Default:** Phase 0

### --force-deprecated Behavior

| Phase | Backend      | --force-deprecated | Result                          |
| ----- | ------------ | ------------------ | ------------------------------- |
| 0     | local-memory | N/A                | Allow (no warning)              |
| 0     | memvid       | N/A                | Allow                           |
| 1     | local-memory | N/A                | Allow with deprecation warning  |
| 1     | memvid       | N/A                | Allow                           |
| 2     | local-memory | `false`            | **Block** (requires flag)       |
| 2     | local-memory | `true`             | Allow with strong warning       |
| 2     | memvid       | N/A                | Allow                           |
| 3     | local-memory | any                | **Block** (permanently removed) |
| 3     | memvid       | N/A                | Allow                           |

**Event Types:**

* `LocalMemorySunsetPhaseResolved`: Emitted at startup with resolution source
* `FallbackActivated`: Emitted when fallback to local-memory occurs (GATE-ST tracking)

***

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

* local-memory is default
* Memvid available via explicit config
* Fallback: memvid → local-memory on capsule failure
* A/B harness available for evaluation

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

* Memvid is default for new `codex-tui` sessions
* Existing projects keep their config
* Fallback: memvid → local-memory still active
* Deprecation warning shown when using local-memory

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

* Config file only accepts `memvid`
* CLI flag `--memory-backend local-memory` requires `--force-deprecated`
* Fallback disabled by default (enable via `--enable-fallback`)
* Strong warning on every session start if local-memory data exists

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

* `memory_backend = "local-memory"` is a config error
* CLI flag `--memory-backend local-memory` removed
* Fallback mechanism removed
* Legacy plugin (option 2) provides escape hatch for edge cases

***

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

***

## Acceptance Tests

### Unit Tests (CI-Required)

| Test Name                       | Gate    | Description                   |
| ------------------------------- | ------- | ----------------------------- |
| `test_memvid_adapter_search`    | GATE-FP | Basic search returns results  |
| `test_memvid_domain_filtering`  | GATE-FP | Domain filter works correctly |
| `test_memvid_tag_filtering`     | GATE-FP | Tag filter works correctly    |
| `test_memvid_importance_filter` | GATE-FP | Importance threshold works    |
| `test_capsule_open_close`       | GATE-ST | Capsule lifecycle management  |
| `test_capsule_crash_recovery`   | GATE-ST | WAL recovery after crash      |

### Integration Tests (CI-Required)

| Test Name                      | Gate    | Description                          |
| ------------------------------ | ------- | ------------------------------------ |
| `test_ab_harness_runs`         | GATE-RQ | A/B harness executes without error   |
| `test_golden_queries_coverage` | GATE-RQ | All golden queries have test data    |
| `test_backend_switch`          | GATE-FP | Config switch changes backend        |
| `test_fallback_activation`     | GATE-ST | Fallback triggers on capsule failure |
| `test_deprecation_warning`     | Phase 1 | Warning shown for local-memory       |

### Parity Gate Tests (CI-Blocking after Phase 0)

| Test Name                            | Gate    | Threshold               |
| ------------------------------------ | ------- | ----------------------- |
| `test_parity_gate_retrieval_quality` | GATE-RQ | P\@10 ≥ 0.95 × baseline |
| `test_parity_gate_latency`           | GATE-LP | P95 < 250ms             |
| `test_parity_gate_stability`         | GATE-ST | Manual + telemetry      |
| `test_parity_gate_features`          | GATE-FP | All features pass       |

### Golden Query Tests (Regression Prevention)

| Test Name                    | Queries   | Expected                  |
| ---------------------------- | --------- | ------------------------- |
| `test_golden_keyword_search` | 3 queries | All expected IDs found    |
| `test_golden_domain_filter`  | 2 queries | Domain filtering works    |
| `test_golden_tag_filter`     | 2 queries | Tag filtering works       |
| `test_golden_combined`       | 2 queries | Combined filters work     |
| `test_golden_edge_cases`     | 4 queries | Edge cases handled        |
| `test_golden_no_match`       | 1 query   | Empty result for no match |

***

## CI Gates

### Pre-Merge (All PRs) — Proposed

```yaml
# .github/workflows/ci.yml (proposed addition)
- name: Memory Backend Tests
  run: |
    cargo test -p codex-tui -- memvid_adapter
    cargo test -p codex-stage0 -- dcc
```

### Nightly (Parity Validation) — IMPLEMENTED

**File:** `.github/workflows/nightly-parity.yml`

Runs daily at 02:00 UTC. Executes `#[ignore]` parity gate tests and creates failure issues on regression.

```yaml
# Actual workflow (implemented)
name: Nightly Parity Gates
on:
  schedule:
    - cron: '0 2 * * *'
  workflow_dispatch:
jobs:
  parity-gates:
    runs-on: ubuntu-latest
    steps:
      - name: Run parity gate tests
        run: |
          cargo test -p codex-tui --lib -- parity_gate --ignored --nocapture
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: parity-gate-report
          path: codex-rs/parity_*.{log,json}
      - name: Create failure issue
        if: failure()
        uses: peter-evans/create-issue-from-file@v5
```

### Release Gate (Phase Transitions) — NOT YET IMPLEMENTED

*The following workflow is planned for phase transition gating:*

```yaml
# .github/workflows/release.yml (TODO)
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

***

## CLI Reference

### Backend Selection — IMPLEMENTED

**Implementation:** `tui/src/cli.rs:149-180`

```bash
# Override backend for session
codex-tui --memory-backend memvid
codex-tui --memory-backend local-memory

# Phase 2+: Force deprecated backend
codex-tui --memory-backend local-memory --force-deprecated
```

### Migration Commands — IMPLEMENTED

**Implementation:** `cli/src/local_memory_cmd/import.rs`

**Command:** `code local-memory import`

```bash
# Show import status (default, safe - no writes)
code local-memory import --status

# Dry run: preview import plan
code local-memory import --dry-run

# Execute import (required flag for writes)
code local-memory import --all

# Post-import verification
code local-memory import --verify

# Filter by domain
code local-memory import --domain my-domain --all

# Filter by minimum importance
code local-memory import --min-importance 7 --all

# Custom capsule/database paths
code local-memory import --capsule .speckit/memvid/workspace.mv2 --all
code local-memory import --database ~/.local-memory/memory.db --all

# Custom namespace (default: spec_id="lm-import", auto-generated run_id)
code local-memory import --spec-id my-import --run-id batch-001 --all
```

**Behavior:**

* Default (no flags): equivalent to `--status`, shows plan without writes
* Deduplication: skips duplicates by (source\_backend, source\_id) with SHA256 content hash
* Conflict detection: same ID, different hash → reported but not auto-resolved

**TODO:** `--resume` (interrupted migration recovery)

### Evaluation Commands — IMPLEMENTED

**Implementation:** `tui/src/cli_diagnostics.rs`

```bash
# Run A/B evaluation (headless, exits after completion)
codex-tui --eval-ab --output-dir .speckit/eval/

# JSON output for CI
codex-tui --eval-ab --json
```

**Behavior:**

* Runs in synthetic mode (Phase 0): self-comparison using synthetic corpus
* Produces JSON + Markdown reports
* Exit codes: 0 (pass), 1 (fail), 2 (config error)

**TODO:** `--golden-queries` (custom query file), `--synthetic` flag (explicit mode selection)

### Health & Diagnostics — PARTIALLY IMPLEMENTED

**Implementation:** `tui/src/cli_diagnostics.rs`

```bash
# Capsule health check (IMPLEMENTED)
codex-tui --capsule-doctor

# JSON output for CI (IMPLEMENTED)
codex-tui --capsule-doctor --json
```

**Checks:** capsule existence, lock status, header integrity, version compatibility

**TODO:** `--capsule-verify` (deep integrity check), `--backend-status` (status display), `--show-config`

***

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

***

## Risks & Mitigations

| Risk                       | Severity | Mitigation                                     |
| -------------------------- | -------- | ---------------------------------------------- |
| Memvid API churn           | Medium   | Pin versions; adapter boundary; contract tests |
| Retrieval regressions      | High     | A/B harness; golden queries; CI blocking       |
| Data loss during migration | High     | `lm-import --verify`; backup before migration  |
| Fallback loops             | Medium   | Circuit breaker; fallback counter limit        |
| Performance degradation    | Medium   | P95 gate; nightly benchmarks                   |

***

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

***

## Timeline

| Phase   | Start             | Duration         | Entry Criteria      |
| ------- | ----------------- | ---------------- | ------------------- |
| Phase 0 | Current           | Until gates pass | N/A                 |
| Phase 1 | GATE-RQ + GATE-LP | 30 days          | 7 days parity       |
| Phase 2 | Phase 1 + 30 days | 60 days          | GATE-ST (30 days)   |
| Phase 3 | Phase 2 + 60 days | Permanent        | 60 days deprecation |

**Total Migration Timeline:** \~150 days from first parity gate pass

***

## Dependencies

* **SPEC-KIT-971**: Memvid capsule foundation (LOCKED)
* **SPEC-KIT-972**: Hybrid retrieval eval (LOCKED)
* **model\_policy.toml**: `[gates.local_memory_sunset]` section

***

## References

* `codex-rs/tui/src/memvid_adapter/eval.rs` — A/B harness implementation
* `codex-rs/stage0/src/config.rs` — MemoryBackend enum
* `codex-rs/model_policy.toml` — Gate thresholds
* `docs/DECISIONS.md` — D14, D39, D40, D52, D94
