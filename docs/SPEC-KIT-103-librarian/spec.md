# SPEC-KIT-103: Librarian & Repair Jobs

**Status**: ROADMAP / PLANNING (requires refinement before implementation)
**Created**: 2025-12-01
**Dependencies**: SPEC-KIT-102R (Stage 0 Implementation)
**Phase**: 3 (Enhancement)

---

## 1. Executive Summary

The Librarian is an **offline/maintenance component** that performs LLM-powered operations on the memory graph. Unlike Stage 0's hot path (DCC, Tier 2 synthesis), Librarian jobs run in background tasks, nightly schedules, or explicit commands, tolerating higher latency in exchange for deeper reasoning.

### Core Responsibilities

1. **Auto-structuring**: Transform unstructured legacy memories into Template Guardian format
2. **Meta-memory synthesis**: Generate pattern summaries from clusters of related memories
3. **Relationship labeling**: Replace generic "similar" edges with causal types (causes, solves, expands, supersedes, contradicts)

### Key Architectural Decision

Librarian uses a **backend-agnostic `LibrarianLlmBackend` interface** with a reference implementation based on Ollama + qwen2.5:3b. Implementers MAY swap in any LLM backend that meets the latency and capability requirements.

---

## 2. Problem Statement

### 2.1 Current Data Quality Issues

Analysis of the local-memory environment (n=1,161 memories) from SPEC-KIT-102 revealed:

| Finding | Data | Impact |
|---------|------|--------|
| Structure Deficit | 100% unstructured | High noise for Tier 2 |
| Shallow Graph | 77.3% "similar" links | Limited causal reasoning |
| No Meta-Patterns | 0 meta-memories | No cross-memory insights |
| Attribution Gaps | 100% agent_type=unknown | Performance tracking blocked |

### 2.2 What Stage 0 Cannot Do

Stage 0's hot path is optimized for low latency during `/speckit.auto`:
- Cannot spend 5-10s restructuring each retrieved memory
- Cannot run relationship inference between all candidate pairs
- Cannot generate meta-memories during synthesis

These operations require an **offline component** that can process the full memory graph without blocking the pipeline.

---

## 3. LibrarianLlmBackend Interface

### 3.1 Interface Definition

```rust
/// Backend-agnostic interface for Librarian LLM operations.
///
/// Implementations must provide restructure, summarize, and relationship
/// labeling capabilities. The interface is designed to be LLM-agnostic;
/// any backend meeting the capability requirements may be used.
#[async_trait]
pub trait LibrarianLlmBackend: Send + Sync {
    /// Restructure an unstructured memory into Template Guardian format.
    ///
    /// Input: Raw memory content (any format)
    /// Output: Structured memory with type, context, reasoning, outcome
    async fn restructure_memory(&self, raw: &str) -> Result<StructuredMemory>;

    /// Summarize a cluster of related memories into a meta-memory.
    ///
    /// Input: Cluster of 3-20 structured memories
    /// Output: Meta-memory capturing the pattern/decision/insight
    async fn summarize_cluster(&self, memories: &[StructuredMemory]) -> Result<MetaMemory>;

    /// Determine the relationship type between two memories.
    ///
    /// Input: Two structured memories with similarity >= 0.70
    /// Output: Relationship label with confidence and reasoning
    async fn label_relationship(
        &self,
        a: &StructuredMemory,
        b: &StructuredMemory,
    ) -> Result<RelationshipLabel>;

    /// Compress multiple texts for efficient indexing (optional).
    ///
    /// Used for generating searchable summaries of large content.
    async fn compress_for_indexing(&self, texts: &[String]) -> Result<String> {
        // Default: concatenate with truncation
        Ok(texts.join("\n\n").chars().take(2000).collect())
    }

    /// Check if the backend is available and healthy.
    async fn health_check(&self) -> Result<BackendHealth>;
}
```

### 3.2 Output Types

```rust
/// Structured memory in Template Guardian format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredMemory {
    /// Original memory ID
    pub id: String,

    /// Memory type: PATTERN | DECISION | PROBLEM | INSIGHT | BUG | REFACTOR
    pub memory_type: MemoryType,

    /// Situation that led to this memory
    pub context: String,

    /// Why this approach/conclusion was reached
    pub reasoning: String,

    /// Result or expected result
    pub outcome: String,

    /// Auto-extracted or refined tags
    pub tags: Vec<String>,

    /// Original raw content (preserved)
    pub content_raw: String,

    /// Restructuring metadata
    pub restructured_at: DateTime<Utc>,
    pub restructured_by: String, // "librarian:v1"
}

/// Meta-memory synthesized from a cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaMemory {
    /// Generated ID (meta-{hash})
    pub id: String,

    /// Source memory IDs in the cluster
    pub source_ids: Vec<String>,

    /// Pattern type: RECURRING_PATTERN | EVOLUTION | DECISION_TREE | ANTI_PATTERN
    pub pattern_type: PatternType,

    /// Synthesized content
    pub title: String,
    pub summary: String,
    pub key_insights: Vec<String>,

    /// Derived tags (union + inferred)
    pub tags: Vec<String>,

    /// High importance (meta-memories are always valuable)
    pub importance: u8, // Always 9 or 10

    /// Generation metadata
    pub generated_at: DateTime<Utc>,
    pub cluster_size: usize,
}

/// Relationship label with causal semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipLabel {
    /// Relationship type
    pub relationship_type: RelationshipType,

    /// Confidence (0.0 - 1.0)
    pub confidence: f64,

    /// Brief explanation of why this label was chosen
    pub reasoning: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RelationshipType {
    /// A led to or caused B
    Causes,
    /// B is a solution to problem A
    Solves,
    /// A and B present conflicting information
    Contradicts,
    /// B provides additional detail on A
    Expands,
    /// B replaces or updates A
    Supersedes,
    /// A and B are related but no causal connection
    Similar,
}
```

### 3.3 Capability Requirements

The LLM backend must satisfy:

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| Context window | 8K tokens | 16K+ tokens |
| Structured output | Stable JSON fragments | JSON mode |
| Latency per operation | ≤30 seconds | ≤10 seconds |
| Concurrent requests | 1 | 4+ |
| Local deployment | Supported | Required for privacy |

---

## 4. Reference Implementation

> **Policy Compliance**: See `docs/MODEL-POLICY.md` for authoritative model routing.
> Librarian uses local 8–14B synth with Kimi escalation-only for hard sweeps.

### 4.1 Local LLM Backend (Policy-Aligned)

The reference implementation uses:

- **Runtime**: vLLM (default) or llama.cpp (fallback)
- **Model**: Llama 3.1 8B / Qwen 14B instruct (8–14B range per policy)
- **Endpoint**: OpenAI-compatible local API

```rust
pub struct LocalLibrarianBackend {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    timeout: Duration,
}

impl LocalLibrarianBackend {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: "http://localhost:8000/v1".to_string(),  // vLLM default
            model: "Qwen/Qwen2.5-14B-Instruct".to_string(),    // Policy-aligned
            timeout: Duration::from_secs(30),
        }
    }
}
```

### 4.2 Backend Profiles (Policy-Aligned)

Librarian and Stage 0 use tiered profiles with escalation:

| Profile | Use Case | Model | Timeout | Escalation |
|---------|----------|-------|---------|------------|
| `stage0_fast` | IQO extraction, hot path | Local 8B (Llama 3.1 8B) | 10s | None |
| `librarian_batch` | Restructure, summarize | Local 8–14B (Qwen 14B) | 30s | None |
| `librarian_deep` | Complex relationship inference | Local 14B | 60s | Kimi K2 on hard-sweep predicate |

### 4.3 Kimi Escalation (Hard Sweeps Only)

Escalate to Kimi K2 when ANY of:
- `context_estimate > 100k`
- `contradictions_detected == true`
- `tool_heavy == true`
- `full_sweep == true`
- `local_synth_confidence < threshold`

```rust
pub struct KimiEscalationBackend {
    // Used only for hard sweeps per MODEL-POLICY.md
    model: String,  // "kimi-k2-instruct" or "kimi-k2-thinking"
}
```

All profiles implement the same `LibrarianLlmBackend` trait.

---

## 5. Librarian Jobs

### 5.1 Legacy Repair Job

**Purpose**: Restructure unstructured memories into Template Guardian format.

**Trigger**: Manual command or scheduled (nightly)

**Algorithm**:
```
1. Query memories WHERE structure_status != 'structured'
2. For each memory in batches of 50:
   a. Call restructure_memory(raw_content)
   b. Update memory with structured fields
   c. Set structure_status = 'restructured_by_librarian'
   d. Log repair event
3. Report: {processed, succeeded, failed, skipped}
```

**Command**: `/librarian.repair [--dry-run] [--limit=N] [--domain=X]`

### 5.2 Meta-Memory Synthesis Job

**Purpose**: Generate meta-memories from clusters of related memories.

**Trigger**: Manual command or scheduled (weekly)

**Algorithm**:
```
1. Cluster memories by:
   a. Tag overlap (≥3 shared tags)
   b. Domain + time proximity
   c. Relationship graph connectivity
2. For clusters of size 3-20:
   a. Call summarize_cluster(memories)
   b. Create new meta-memory entry
   c. Link meta-memory to sources (relationship: 'synthesizes')
3. Report: {clusters_found, meta_memories_created}
```

**Command**: `/librarian.synthesize [--min-cluster=3] [--max-cluster=20]`

### 5.3 Relationship Enrichment Job

**Purpose**: Replace "similar" edges with causal relationship types.

**Trigger**: Manual command or scheduled (weekly)

**Algorithm**:
```
1. Query relationships WHERE type = 'similar' AND confidence >= 0.70
2. For each relationship:
   a. Load both memories
   b. Call label_relationship(a, b)
   c. If confidence >= 0.75 AND type != 'similar':
      - Update relationship type
      - Record reasoning
   d. Log enrichment event
3. Report: {processed, relabeled, kept_similar}
```

**Command**: `/librarian.enrich [--min-confidence=0.75]`

---

## 6. Schema Additions

### 6.1 Meta-Memories Table

```sql
CREATE TABLE meta_memories (
    id                TEXT PRIMARY KEY,
    pattern_type      TEXT NOT NULL,
    title             TEXT NOT NULL,
    summary           TEXT NOT NULL,
    key_insights      TEXT NOT NULL,  -- JSON array
    tags              TEXT NOT NULL,  -- JSON array
    importance        INTEGER NOT NULL DEFAULT 9,
    source_ids        TEXT NOT NULL,  -- JSON array
    cluster_size      INTEGER NOT NULL,
    generated_at      DATETIME NOT NULL,
    generated_by      TEXT NOT NULL   -- "librarian:v1"
);

CREATE INDEX idx_meta_pattern ON meta_memories(pattern_type);
CREATE INDEX idx_meta_generated ON meta_memories(generated_at);
```

### 6.2 Repair Log Table

```sql
CREATE TABLE librarian_repair_log (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_id         TEXT NOT NULL,
    operation         TEXT NOT NULL,  -- 'restructure', 'synthesize', 'enrich'
    status            TEXT NOT NULL,  -- 'success', 'failed', 'skipped'
    input_hash        TEXT,
    output_hash       TEXT,
    error_message     TEXT,
    duration_ms       INTEGER,
    created_at        DATETIME NOT NULL
);

CREATE INDEX idx_repair_memory ON librarian_repair_log(memory_id);
CREATE INDEX idx_repair_op ON librarian_repair_log(operation);
```

---

## 7. Configuration

```toml
[librarian]
enabled = true

[librarian.backend]
type = "ollama"  # or "openai_compatible", "llama_cpp"
endpoint = "http://localhost:11434"
model = "qwen2.5:3b"
timeout_seconds = 30

[librarian.jobs]
# Batch sizes
repair_batch_size = 50
synthesis_min_cluster = 3
synthesis_max_cluster = 20
enrichment_min_confidence = 0.75

[librarian.schedule]
# Cron expressions (optional)
repair_cron = "0 2 * * *"      # 2 AM daily
synthesis_cron = "0 3 * * 0"   # 3 AM Sunday
enrichment_cron = "0 4 * * 0"  # 4 AM Sunday
```

---

## 8. Error Handling

### 8.1 Backend Unavailable

If no LibrarianLlmBackend is configured or available:

- Librarian commands MUST report "Librarian unavailable: no LLM backend configured"
- Scheduled jobs MUST skip with warning logged
- Stage 0 hot path is NOT affected (independent)

### 8.2 Partial Failures

Jobs are designed for resumability:

- Each memory/relationship is processed independently
- Failures are logged but don't abort the batch
- Re-running a job skips already-processed items (idempotent)

---

## 9. Open Questions

### 9.1 Clustering Algorithm

**Question**: What clustering algorithm should be used for meta-memory synthesis?

**Options**:
- Tag-based overlap (simple, interpretable)
- Embedding-based clustering (requires vector backend)
- Graph-based community detection (uses existing relationships)

**Current lean**: Start with tag-based, add embedding-based later.

### 9.2 Meta-Memory Lifecycle

**Question**: Should meta-memories be updated when source memories change?

**Options**:
- Immutable: Meta-memories are snapshots, regenerate periodically
- Reactive: Invalidate and regenerate when sources change
- Versioned: Keep history of meta-memory versions

**Current lean**: Immutable with periodic regeneration (simpler).

### 9.3 Relationship Directionality

**Question**: How to handle directional relationships (A causes B vs B causes A)?

**Options**:
- Always store in one direction (older → newer)
- Store both directions
- Let LLM determine directionality

**Current lean**: LLM determines, store as directed edge.

### 9.4 Integration with Stage 0

**Question**: Should Stage 0 prefer meta-memories over raw memories?

**Options**:
- Boost meta-memories in scoring
- Include meta-memories as separate context section
- No special treatment (just more memories)

**Current lean**: Boost + separate section ("Synthesized Patterns").

---

## 10. Success Metrics

| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| Structured memories | 0% | >80% | Template validation |
| Causal relationships | 12 | >200 | Non-"similar" edges |
| Meta-memories | 0 | >50 | meta_memories count |
| Repair job success rate | N/A | >95% | Log analysis |

---

## 11. Implementation Roadmap

### Phase 3a: Foundation (2-3 weeks)

- [ ] Define LibrarianLlmBackend trait
- [ ] Implement OllamaLibrarianBackend
- [ ] Create schema migrations
- [ ] Implement `/librarian.repair` command

### Phase 3b: Synthesis (2-3 weeks)

- [ ] Implement clustering algorithm
- [ ] Implement summarize_cluster operation
- [ ] Create `/librarian.synthesize` command
- [ ] Integrate meta-memories into Stage 0

### Phase 3c: Enrichment (1-2 weeks)

- [ ] Implement label_relationship operation
- [ ] Create `/librarian.enrich` command
- [ ] Update relationship graph queries

### Phase 3d: Automation (1 week)

- [ ] Add scheduling support
- [ ] Create health/status dashboard
- [ ] Document operations runbook

---

## 12. Model & Runtime (Spec Overrides)

Policy: docs/MODEL-POLICY.md (version: 1.0.0)

Roles exercised by this spec:
- Stage0 Tier2 (NotebookLM): NO
- Architect/Planner: NO
- Implementer/Rust Ace: NO
- Librarian: YES (this spec defines Librarian jobs)
- Tutor: NO
- Auditor/Judge: NO

Routing mode: local-first with Kimi escalation-only
Librarian default: Local 8–14B synth (Llama 3.1 8B / Qwen 14B)
Kimi escalation: Hard sweeps only (see Section 4.3 for triggers)

Primary tiers:
- stage0_fast: Local 8B (vLLM)
- librarian_batch: Local 8–14B (vLLM)
- librarian_deep: Local 14B + Kimi K2 escalation

Privacy:
- local_only = true (default; Kimi escalation requires explicit override)

High-risk:
- HR = NO (repair jobs are reversible)

Overrides:
- Kimi escalation on hard-sweep predicate (documented in Section 4.3)

---

## 13. Related Specifications

| Spec | Relationship |
|------|--------------|
| SPEC-KIT-102R | Prerequisite (Stage 0 must be stable) |
| SPEC-KIT-104 | Parallel (Metrics & Learning) |

---

*Roadmap Spec v1.0 - 2025-12-01*
*Requires refinement before implementation*
