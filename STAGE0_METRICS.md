# STAGE0_METRICS.md

## Purpose

Define metrics (counters, histograms, gauges) to monitor and tune:

* Stage 0 reliability and performance,
* DCC retrieval quality,
* Tier2 usage and cache effectiveness,
* long-term trends in memory usage and seeding.

Implementation may start as structured logs approximating these metrics; wiring into Prometheus / OpenTelemetry can be done when ready.

---

## General Guidelines

* Prefer **low-cardinality labels** in metrics (e.g., `tier2_used`, `cache_hit`, `error_category`).
* Put detailed/high-cardinality data (memory IDs, file paths) in logs, not metrics.
* Keep metric names stable once introduced; additive changes over renames.

---

## Core Metrics

### 1. Stage 0 Runs

* **Counter**: `stage0_runs_total`

  * Labels:

    * `result` ∈ {`success`, `degraded_config`, `degraded_db`, `degraded_local_memory`, `degraded_internal`}

* **Histogram**: `stage0_run_latency_ms`

  * Labels:

    * optionally `tier2_used` (`true|false`),
    * optionally `cache_hit` (`true|false`).

Purpose: baseline health and performance.

---

### 2. Tier 2 Usage & Cache

* **Counter**: `stage0_tier2_calls_total`

  * Labels:

    * `outcome` ∈ {`success`, `timeout`, `error`}

* **Counter**: `stage0_tier2_cache_hits_total`

* **Counter**: `stage0_tier2_cache_misses_total`

Optional:

* **Gauge**: `stage0_tier2_cache_entries`
* **Gauge**: `stage0_tier2_cache_stale_entries` (if tracked in overlay).

Purpose: understand Tier2 load, cache effectiveness, and error behavior.

---

### 3. DCC Candidate & Selection Stats

* **Histogram**: `stage0_dcc_candidate_count`

  * number of candidate memories considered after pre-filter.

* **Histogram**: `stage0_dcc_selected_count`

  * number of memories actually used in `TASK_BRIEF.md`.

Optional:

* **Histogram**: `stage0_dcc_combined_score_selected`

  * distribution of combined_score for selected memories.

Purpose: tune IQO, pre-filtering, scoring weights, and top_k.

---

### 4. Error Rates

* **Counter**: `stage0_errors_total`

  * Labels:

    * `category` ∈ {`CONFIG_ERROR`, `OVERLAY_DB_ERROR`, `LOCAL_MEMORY_ERROR`, `DCC_ERROR`, `TIER2_ERROR`, `PROMPT_ERROR`, `INTERNAL_ERROR`}

Purpose: spot regressions and recurring failure patterns.

---

### 5. Memory Scoring & Usage (from overlay)

* **Histogram**: `stage0_memory_dynamic_score`

  * sample distribution of dynamic_score across overlay.

* **Histogram**: `stage0_memory_usage_count`

  * distribution of usage_count (e.g. "0 uses", "1–5", "6–20", "21+").

Optional label:

* `domain` (low cardinality, e.g. `spec-kit`, `tui`, `infrastructure`) → `stage0_memory_usage_count_by_domain`.

Purpose: check that dynamic scoring meaningfully separates memories, and see which areas get reused.

---

### 6. Seeding & Notebook Artifacts (after V2.9)

* **Counter**: `stage0_seeding_runs_total`
* **Histogram**: `stage0_seeding_artifacts_generated`
* **Histogram**: `stage0_seeding_memories_per_artifact`

Purpose: monitor how often NotebookLM seed docs are regenerated and how dense they are.

---

## Suggested Dashboards

1. **Stage 0 Health Dashboard**

   * `stage0_runs_total` by `result`.
   * `stage0_run_latency_ms` (p50/p95).
   * `stage0_errors_total` by `category`.

2. **Tier 2 Performance Dashboard**

   * `stage0_tier2_calls_total` by `outcome`.
   * `stage0_tier2_cache_hits_total` vs `_misses_total`.
   * Tier2 latency distribution (if broken out).

3. **DCC & Context Quality Dashboard**

   * `stage0_dcc_candidate_count` & `stage0_dcc_selected_count`.
   * `stage0_memory_dynamic_score` distribution.

4. **Seeding / NotebookLM Dashboard** (when implemented)

   * `stage0_seeding_runs_total` over time.
   * `stage0_seeding_artifacts_generated` distribution.
   * approximate coverage: `stage0_seeding_memories_per_artifact`.

---

## Implementation Notes

* Initially, you can approximate metrics by:

  * emitting JSON events that contain metric fields,
  * building small offline scripts to aggregate them.
* When you introduce a metrics backend:

  * map these conceptual metrics into your chosen library,
  * reuse the same names and labels to avoid confusion.
* Any additional fine-grained metrics (e.g. per-notebook, per-domain) should be added cautiously to avoid cardinality explosions.
