# SPEC-KIT-104: Stage 0 Metrics & Learning

**Status**: ROADMAP / PLANNING (parameter tuning only; routing deferred to SPEC-KIT-105)
**Created**: 2025-12-01
**Dependencies**: SPEC-KIT-102R (Stage 0 Implementation)
**Phase**: 4 (Adaptation)

---

## 1. Executive Summary

This specification defines a **learned weight tuning system** for Stage 0's scoring and retrieval parameters. The goal is to use the existing evaluation harness and structured logs to automatically adjust weights so that P@K, R@K, and MRR improve across both memory and code lanes.

### Scope

**In scope (SPEC-KIT-104)**:
- Parameter tuning for scoring weights
- Parameter tuning for DCC combination weights
- Evaluation-driven optimization loop
- Config versioning and rollback

**Explicitly out of scope (deferred to SPEC-KIT-105+)**:
- Tier 2 routing decisions (when to call NotebookLM)
- Automatic threshold learning
- Changes to Stage 0 ↔ `/speckit.auto` call graph
- Full ML-based policy learning

### Key Insight

Stage 0 already has:
- A rich evaluation harness (P@K, R@K, MRR per lane)
- Structured logging (Stage0Event)
- Clear, interpretable knobs (scoring weights, DCC weights)

This makes **parameter tuning** the natural first target: use metrics we already compute to adjust parameters we already expose.

---

## 2. Problem Statement

### 2.1 Static Weights

Current Stage 0 uses hardcoded default weights:

```toml
# Scoring weights (sum to 1.0)
usage_weight = 0.30
recency_weight = 0.35
priority_weight = 0.25
decay_weight = 0.10

# DCC combination weights
similarity_weight = 0.4
dynamic_score_weight = 0.4
vector_weight = 0.2
```

These were chosen based on intuition and limited testing. They may not be optimal for:
- Different codebases
- Different usage patterns
- Different memory distributions

### 2.2 No Feedback Loop

Currently, there's no mechanism to:
- Measure whether current weights are good
- Identify when weights should change
- Automatically adjust based on observed performance

The evaluation harness can measure quality, but the results aren't used to improve the system.

---

## 3. Existing Metrics Surface

### 3.1 Evaluation Harness (P86)

The eval harness provides:

| Metric | Description | Source |
|--------|-------------|--------|
| P@K | Precision at K (relevant/retrieved) | `EvalSuiteResult.mean_precision` |
| R@K | Recall at K (relevant/expected) | `EvalSuiteResult.mean_recall` |
| MRR | Mean Reciprocal Rank (1/first_hit_rank) | `EvalSuiteResult.mrr` |
| Pass Rate | Cases meeting threshold | `EvalSuiteResult.pass_rate()` |

Per-lane breakdown available via `--lane={memory,code,both}`.

### 3.2 Structured Events (Stage0Event)

```rust
pub enum Stage0Event {
    Start { spec_id, spec_hash, timestamp },
    Complete {
        spec_id,
        latency_ms,
        memories_used: Vec<String>,
        code_candidates: Vec<String>,
        hybrid_active: bool,
        cache_hit: bool,
        tier2_called: bool,
    },
    Error { spec_id, error, timestamp },
    CacheHit { spec_id, cache_key },
    Tier2Call { spec_id, latency_ms, success },
}
```

### 3.3 Tunable Parameters

**Scoring weights** (affect `calculate_memory_score()`):
- `usage_weight`: How much recent usage boosts score
- `recency_weight`: How much recent access boosts score
- `priority_weight`: How much initial importance matters
- `decay_weight`: How much age penalizes score

**DCC combination weights** (affect final ranking):
- `similarity_weight`: Text similarity signal weight
- `dynamic_score_weight`: Overlay score signal weight
- `vector_weight`: TF-IDF/hybrid signal weight

**Code lane parameters**:
- `code_top_k`: Number of code candidates to include
- `code_lane_enabled`: Whether to include code context

---

## 4. Learning Loop Design

### 4.1 High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     LEARNING LOOP                               │
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │   COLLECT   │───▶│   OPTIMIZE  │───▶│   UPDATE    │        │
│  │   Metrics   │    │   Weights   │    │   Config    │        │
│  └─────────────┘    └─────────────┘    └─────────────┘        │
│         │                  │                  │                │
│         ▼                  ▼                  ▼                │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │ Eval Suite  │    │ Grid/Random │    │  Versioned  │        │
│  │ + Run Logs  │    │   Search    │    │   Config    │        │
│  └─────────────┘    └─────────────┘    └─────────────┘        │
│                                                                 │
│                     ┌─────────────┐                            │
│                     │  VALIDATE   │                            │
│                     │  & Report   │                            │
│                     └─────────────┘                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Collect Phase

**Inputs**:
- Built-in eval cases (5 memory + 3 code)
- External eval cases (JSON files)
- Historical Stage0 run logs (optional)

**Process**:
```rust
async fn collect_metrics(
    backend: &TfIdfBackend,
    cases: &[EvalCase],
    current_config: &Stage0Config,
) -> MetricsSnapshot {
    // Run evaluation
    let memory_results = evaluate_cases(
        backend,
        cases.filter(|c| c.lane == Memory),
        current_config.top_k,
    ).await;

    let code_results = evaluate_cases(
        backend,
        cases.filter(|c| c.lane == Code),
        current_config.code_top_k,
    ).await;

    MetricsSnapshot {
        memory_precision: memory_results.mean_precision,
        memory_recall: memory_results.mean_recall,
        memory_mrr: memory_results.mrr,
        code_precision: code_results.mean_precision,
        code_recall: code_results.mean_recall,
        code_mrr: code_results.mrr,
        config_version: current_config.version.clone(),
        timestamp: Utc::now(),
    }
}
```

### 4.3 Optimize Phase

**Approach**: Grid search or random search over weight space.

**Constraint**: Scoring weights must sum to 1.0 (simplex constraint).

**Search space**:
```rust
struct ScoringWeightSpace {
    usage: Range<f64>,      // 0.1 - 0.5
    recency: Range<f64>,    // 0.1 - 0.5
    priority: Range<f64>,   // 0.1 - 0.4
    decay: Range<f64>,      // 0.0 - 0.2
}

struct DccWeightSpace {
    similarity: Range<f64>,    // 0.2 - 0.6
    dynamic_score: Range<f64>, // 0.2 - 0.6
    vector: Range<f64>,        // 0.0 - 0.4
}
```

**Objective function**:
```rust
fn objective(metrics: &MetricsSnapshot) -> f64 {
    // Weighted combination of all metrics
    let memory_score =
        0.4 * metrics.memory_precision +
        0.3 * metrics.memory_recall +
        0.3 * metrics.memory_mrr;

    let code_score =
        0.4 * metrics.code_precision +
        0.3 * metrics.code_recall +
        0.3 * metrics.code_mrr;

    // Equal weight to both lanes
    0.5 * memory_score + 0.5 * code_score
}
```

**Algorithm**:
```rust
async fn optimize_weights(
    backend: &TfIdfBackend,
    cases: &[EvalCase],
    search_space: &SearchSpace,
    max_iterations: usize,
) -> OptimizationResult {
    let mut best_config = current_config();
    let mut best_score = objective(&collect_metrics(backend, cases, &best_config).await);

    for _ in 0..max_iterations {
        // Sample new weights (grid or random)
        let candidate = search_space.sample();

        // Evaluate candidate
        let metrics = collect_metrics(backend, cases, &candidate).await;
        let score = objective(&metrics);

        if score > best_score {
            best_config = candidate;
            best_score = score;
        }
    }

    OptimizationResult {
        best_config,
        best_score,
        iterations: max_iterations,
        improvement: best_score - initial_score,
    }
}
```

### 4.4 Update Phase

**Config versioning**:
```toml
[stage0.scoring]
version = "tuning_v2"  # Incremented on each tuning run
usage_weight = 0.28    # Tuned value
recency_weight = 0.38  # Tuned value
priority_weight = 0.24 # Tuned value
decay_weight = 0.10    # Tuned value

[stage0.scoring.history]
# Previous versions for rollback
tuning_v1 = { usage = 0.30, recency = 0.35, priority = 0.25, decay = 0.10 }
```

**Rollback support**:
```
/stage0.tune --rollback tuning_v1
```

### 4.5 Validate Phase

After update, run full eval suite to confirm improvement:

```rust
async fn validate_tuning(
    backend: &TfIdfBackend,
    cases: &[EvalCase],
    old_config: &Stage0Config,
    new_config: &Stage0Config,
) -> ValidationResult {
    let old_metrics = collect_metrics(backend, cases, old_config).await;
    let new_metrics = collect_metrics(backend, cases, new_config).await;

    ValidationResult {
        old_score: objective(&old_metrics),
        new_score: objective(&new_metrics),
        improvement: new_score - old_score,
        regressions: find_regressions(&old_metrics, &new_metrics),
        recommendation: if improvement > 0.01 { "accept" } else { "reject" },
    }
}
```

---

## 5. CLI Commands

### /stage0.tune

Run weight optimization and optionally apply results.

```
/stage0.tune [--iterations=100] [--dry-run] [--apply]
```

**Output**:
```
Stage0 Weight Tuning
====================
Iterations: 100
Search space: scoring + dcc weights

Current weights:
  scoring: usage=0.30, recency=0.35, priority=0.25, decay=0.10
  dcc: similarity=0.40, dynamic=0.40, vector=0.20

Running optimization...

Best found:
  scoring: usage=0.28, recency=0.38, priority=0.24, decay=0.10
  dcc: similarity=0.35, dynamic=0.45, vector=0.20

Improvement:
  Memory P@K: 0.72 → 0.78 (+0.06)
  Memory MRR: 0.65 → 0.71 (+0.06)
  Code P@K: 0.60 → 0.62 (+0.02)
  Overall: 0.67 → 0.72 (+0.05)

[--apply to save as tuning_v2]
```

### /stage0.tune-status

Show current tuning state and history.

```
/stage0.tune-status
```

**Output**:
```
Stage0 Tuning Status
====================
Current version: tuning_v2 (applied 2025-12-01)

History:
  tuning_v1: 2025-11-15 (baseline)
  tuning_v2: 2025-12-01 (+5% improvement)

Current metrics:
  Memory: P@K=0.78, R@K=0.72, MRR=0.71
  Code: P@K=0.62, R@K=0.58, MRR=0.55
```

### /stage0.tune-rollback

Rollback to a previous tuning version.

```
/stage0.tune-rollback <version>
```

---

## 6. Metrics Storage

### 6.1 Tuning History Table

```sql
CREATE TABLE tuning_history (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    version           TEXT NOT NULL UNIQUE,

    -- Scoring weights
    usage_weight      REAL NOT NULL,
    recency_weight    REAL NOT NULL,
    priority_weight   REAL NOT NULL,
    decay_weight      REAL NOT NULL,

    -- DCC weights
    similarity_weight     REAL NOT NULL,
    dynamic_score_weight  REAL NOT NULL,
    vector_weight         REAL NOT NULL,

    -- Metrics at time of tuning
    memory_precision  REAL,
    memory_recall     REAL,
    memory_mrr        REAL,
    code_precision    REAL,
    code_recall       REAL,
    code_mrr          REAL,
    overall_score     REAL,

    -- Metadata
    iterations        INTEGER,
    applied_at        DATETIME,
    created_at        DATETIME NOT NULL
);

CREATE INDEX idx_tuning_version ON tuning_history(version);
CREATE INDEX idx_tuning_applied ON tuning_history(applied_at);
```

### 6.2 Run Metrics Table (Optional)

For collecting production run data:

```sql
CREATE TABLE stage0_run_metrics (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id           TEXT NOT NULL,
    spec_hash         TEXT NOT NULL,

    -- Config used
    tuning_version    TEXT NOT NULL,

    -- Results
    memories_count    INTEGER,
    code_count        INTEGER,
    latency_ms        INTEGER,
    cache_hit         BOOLEAN,
    tier2_called      BOOLEAN,

    -- Feedback (manual, optional)
    user_rating       INTEGER,  -- 1-5 if provided

    created_at        DATETIME NOT NULL
);

CREATE INDEX idx_run_spec ON stage0_run_metrics(spec_id);
CREATE INDEX idx_run_version ON stage0_run_metrics(tuning_version);
```

---

## 7. Configuration

```toml
[stage0.tuning]
enabled = true

# Optimization settings
max_iterations = 100
search_method = "random"  # or "grid"

# Objective weights
memory_weight = 0.5
code_weight = 0.5
precision_weight = 0.4
recall_weight = 0.3
mrr_weight = 0.3

# Constraints
min_improvement = 0.01  # Don't apply if < 1% improvement

# Auto-tuning (optional)
auto_tune_enabled = false
auto_tune_interval_days = 7
```

---

## 8. Preparation for SPEC-KIT-105

To support future routing decisions, ensure logs include:

| Field | Purpose |
|-------|---------|
| `spec_length` | Input size signal |
| `iqo_domains` | Domain coverage |
| `candidate_count` | Retrieval breadth |
| `stage0_latency_ms` | Baseline cost |
| `hybrid_used` | Feature flag state |
| `tier2_cache_hit` | Cache effectiveness |
| `tier2_latency_ms` | Tier 2 cost |

These fields are already in `Stage0Event::Complete`. SPEC-KIT-105 will use them to learn routing policies.

---

## 9. Open Questions

### 9.1 Optimization Frequency

**Question**: How often should tuning run?

**Options**:
- Manual only (safest)
- Weekly (with human approval)
- Continuous (risky without guardrails)

**Current lean**: Manual with weekly reminders.

### 9.2 Eval Case Coverage

**Question**: Are built-in cases sufficient, or do we need more?

**Options**:
- Use only built-in cases (8 total)
- Generate synthetic cases from real runs
- Require user-provided cases per project

**Current lean**: Start with built-in, add user-provided over time.

### 9.3 Per-Project Tuning

**Question**: Should weights be global or per-project?

**Options**:
- Global weights (simpler, less data needed)
- Per-project weights (more accurate, more complexity)
- Hybrid (global defaults, per-project overrides)

**Current lean**: Global for now, per-project later.

### 9.4 Regression Prevention

**Question**: How to prevent tuning from hurting some cases?

**Options**:
- Require no individual case regressions
- Allow regressions if overall improves
- Use Pareto optimization (no case gets worse)

**Current lean**: Allow small regressions (<5%) if overall improves >5%.

---

## 10. Success Metrics

| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| Memory P@K | ~0.70 | >0.80 | Eval harness |
| Memory MRR | ~0.65 | >0.75 | Eval harness |
| Code P@K | ~0.55 | >0.65 | Eval harness |
| Tuning improvement | N/A | >5% | Before/after |
| Tuning stability | N/A | <2% variance | Repeated runs |

---

## 11. Implementation Roadmap

### Phase 4a: Foundation (1-2 weeks)

- [ ] Create `tuning_history` table
- [ ] Implement `MetricsSnapshot` collection
- [ ] Implement objective function

### Phase 4b: Optimization (1-2 weeks)

- [ ] Implement grid/random search
- [ ] Add simplex constraint handling
- [ ] Create `/stage0.tune` command

### Phase 4c: Validation (1 week)

- [ ] Implement validation with regression detection
- [ ] Add config versioning
- [ ] Create `/stage0.tune-status` and rollback

### Phase 4d: Polish (1 week)

- [ ] Add CI integration hooks
- [ ] Create tuning documentation
- [ ] Add auto-tune option (disabled by default)

---

## 12. Model & Runtime (Spec Overrides)

Policy: docs/MODEL-POLICY.md (version: 1.0.0)

This spec is **infrastructure-only** (parameter tuning for scoring weights) and does not invoke model routing directly.
Roles exercised: none (no Architect/Implementer/Librarian/Tutor/Judge).
Privacy: local_only = true (deterministic parameter tuning, no LLM calls)
Guardrails still apply: sandboxing, evidence/logging, config versioning.

---

## 13. Related Specifications

| Spec | Relationship |
|------|--------------|
| SPEC-KIT-102R | Prerequisite (Stage 0 must be stable) |
| SPEC-KIT-103 | Parallel (Librarian) |
| SPEC-KIT-105 | Follow-on (Routing decisions) |

---

*Roadmap Spec v1.0 - 2025-12-01*
*Parameter tuning only; routing deferred to SPEC-KIT-105*
