# SPEC-KIT-102: NotebookLM Integration & Tiered Memory Architecture

**Status**: Draft (Research Complete, Pending Implementation)
**Created**: 2025-11-30
**Updated**: 2025-11-30 (P71 Session Decisions)
**Authors**: Research synthesis from PPP Framework analysis
**Dependencies**: local-memory daemon, NotebookLM MCP bridge

### Session Decisions (P71)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Phase 0 (Data Integrity) | Research/Documentation first | No implementation until research complete |
| Seeding Script | Include as fallback | Manual option for initial NotebookLM setup |
| Predictive Prefetching | In scope | Include in implementation plan |
| Local LLM for Guardian | TBD - needs HW analysis | Requires hardware requirement analysis first |

---

## 1. Executive Summary

This specification defines the integration of Google NotebookLM as a "Tier 2" reasoning layer for codex-rs, enabling evolution from a "fast coding tool" (Productivity) into a "context-aware partner" (Proactivity + Personalization) as defined by the PPP Framework.

### Core Architecture Decision

```
┌─────────────────────────────────────────────────────────────────┐
│                    TIERED MEMORY ARCHITECTURE                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  TIER 1: Hot Storage (Local)     TIER 2: Cold Reasoning (NLBM) │
│  ────────────────────────────    ───────────────────────────── │
│  Technology: SQLite + Qdrant     Technology: NotebookLM/Gemini │
│  Role: "The Library Clerk"       Role: "The Staff Engineer"    │
│  Function: WHAT questions        Function: WHY questions       │
│  Latency: Milliseconds           Latency: 5-15 seconds         │
│  Rate Limit: None                Rate Limit: 50 queries/day    │
│  Integration: Every stage        Integration: Stage 0 only     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Key Constraints ("The Reality Gap")

| Metric | speckit.auto Reality | NotebookLM Constraint | Resolution |
|--------|---------------------|----------------------|------------|
| Pipeline Volume | 19-28 LLM calls/run | 50 queries/day | Stage 0 only |
| Memory Store | 1,161 entries | 50 source files | Dynamic compilation |
| Latency | Snappy CLI expected | 5-15s per query | Synthesis caching |
| Cost | $15-40/run | $0.00 | Offload "heavy reading" |

---

## 2. Problem Statement

### 2.1 Current Limitations

The existing codex-rs implementation lacks:

1. **Deep Contextual Reasoning**: Local memory provides retrieval but not synthesis
2. **Proactive Guidance**: System reacts to commands but doesn't anticipate needs
3. **Personalized Learning**: No adaptation to user patterns or historical performance

### 2.2 Data Quality Findings

Analysis of the local-memory environment (n=1,161 memories) revealed:

| Finding | Data | Impact |
|---------|------|--------|
| Structure Deficit | 100% unstructured | High noise for Tier 2 |
| Importance Saturation | 82.7% rated ≥8 | Filtering non-viable |
| Excellent Taxonomy | 4.37 tags/memory | Strong metadata filtering |
| Temporal Blindness | Timestamps unparseable | Trend analysis blocked |
| Attribution Failure | 100% agent_type=unknown | Performance tracking blocked |
| Shallow Graph | 77.3% "similar" links | Limited causal reasoning |

---

## 3. Proposed Solution

### 3.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         codex-rs TUI                             │
│                              │                                   │
│                    /speckit.auto SPEC-XXX                       │
│                              │                                   │
│                              ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    STAGE 0: PLANNING                       │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │           LOCAL-MEMORY (Tier 1 - Evolved)           │  │  │
│  │  │                                                     │  │  │
│  │  │  1. Compile Context (Hybrid Search)                 │  │  │
│  │  │     • Metadata pre-filter (taxonomy)                │  │  │
│  │  │     • Semantic search (Qdrant)                      │  │  │
│  │  │     • Rank by dynamic_score                         │  │  │
│  │  │                                                     │  │  │
│  │  │  2. Check Synthesis Cache                           │  │  │
│  │  │     • Hash(spec + task_brief)                       │  │  │
│  │  │     • Cache hit → instant return                    │  │  │
│  │  │                                                     │  │  │
│  │  └──────────────────────┬──────────────────────────────┘  │  │
│  │                         │ TASK_BRIEF.md                    │  │
│  │                         ▼                                  │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │           NOTEBOOKLM (Tier 2 - MCP Bridge)          │  │  │
│  │  │                                                     │  │  │
│  │  │  • Receives: spec.md + TASK_BRIEF.md                │  │  │
│  │  │  • Returns: "Divine Truth" brief                    │  │  │
│  │  │  • Returns: Suggested causal relationships          │  │  │
│  │  │                                                     │  │  │
│  │  └──────────────────────┬──────────────────────────────┘  │  │
│  │                         │                                  │  │
│  │                         ▼                                  │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │           FEEDBACK LOOP                             │  │  │
│  │  │  • Cache synthesis result                           │  │  │
│  │  │  • Ingest causal relationships to Tier 1            │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                              ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              STAGES 1-5: Execution Pipeline                │  │
│  │              (Plan → Tasks → Implement → Validate → ...)   │  │
│  │              Uses Tier 1 only (no NotebookLM)              │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Component Specifications

#### 3.2.1 Dynamic Context Compiler

**Purpose**: Replace static aggregation with task-specific context compilation

**Endpoint**: `POST /api/v1/compile_context`

**Request**:
```json
{
  "task_spec": "<contents of spec.md>",
  "max_tokens": 8000,
  "top_k": 15,
  "include_domains": ["spec-kit", "infrastructure"],
  "exclude_tags": ["type:deprecated"]
}
```

**Response**:
```json
{
  "task_brief_md": "# Task Brief\n\n## Relevant Context\n...",
  "context_hash": "sha256:abc123...",
  "memories_used": [
    {"id": "mem-123", "score": 0.92, "contribution": "primary"},
    {"id": "mem-456", "score": 0.85, "contribution": "supporting"}
  ],
  "token_count": 6842
}
```

**Pipeline**:
```
Input (spec.md)
     │
     ▼
┌────────────────────────────────────────┐
│ 1. INTENT ANALYSIS                     │
│    Extract: domains, components, tags  │
│    Infer: related SPECs, risk areas    │
└────────────────┬───────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────┐
│ 2. METADATA PRE-FILTER (SQLite)        │
│    WHERE domain IN (inferred_domains)  │
│    AND tags MATCH (inferred_tags)      │
│    Result: 1161 → ~80-150 candidates   │
└────────────────┬───────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────┐
│ 3. SEMANTIC SEARCH (Qdrant)            │
│    Vector similarity on filtered set   │
│    Result: 80-150 → Top-K (15-20)      │
└────────────────┬───────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────┐
│ 4. RANK BY DYNAMIC SCORE               │
│    Combined: similarity + usage + age  │
└────────────────┬───────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────┐
│ 5. COMPILE TASK_BRIEF.md               │
│    Format for NotebookLM ingestion     │
└────────────────────────────────────────┘
```

#### 3.2.2 Tier 2 Synthesis Cache

**Purpose**: Eliminate latency for repeated or prefetched queries

**Schema**:
```sql
CREATE TABLE tier2_synthesis_cache (
    input_hash          TEXT PRIMARY KEY,
    spec_hash           TEXT NOT NULL,
    brief_hash          TEXT NOT NULL,
    synthesis_result    TEXT NOT NULL,
    suggested_links     TEXT,              -- JSON array of relationship suggestions
    created_at          DATETIME NOT NULL,
    expires_at          DATETIME NOT NULL,
    hit_count           INTEGER DEFAULT 0,
    last_hit_at         DATETIME
);

CREATE INDEX idx_cache_expiry ON tier2_synthesis_cache(expires_at);
CREATE INDEX idx_cache_spec ON tier2_synthesis_cache(spec_hash);
```

**Cache Policy**:
- TTL: 24 hours (configurable)
- Invalidation: On related memory updates
- Max entries: 100 (LRU eviction)

#### 3.2.3 Ingestion Guardians

**Purpose**: Ensure data quality at ingestion time

**A. Template Guardian**

Intercepts `POST /api/v1/memories` and enforces structure:

```
Required Template:
─────────────────
[PATTERN|DECISION|PROBLEM|INSIGHT]

CONTEXT: <situation that led to this memory>

REASONING: <why this approach/conclusion>

OUTCOME: <result or expected result>

TAGS: <auto-extracted or provided>
```

**Validation Flow**:
```
Incoming Memory
      │
      ▼
┌──────────────────────────────────────┐
│ TEMPLATE VALIDATOR                   │
│ Check: Has type prefix?              │
│ Check: Has CONTEXT section?          │
│ Check: Has REASONING section?        │
└────────────────┬─────────────────────┘
                 │
        ┌────────┴────────┐
        │                 │
   [VALID]           [INVALID]
        │                 │
        ▼                 ▼
   Store as-is    ┌──────────────────┐
                  │ LOCAL LLM        │
                  │ (qwen2.5:3b)     │
                  │ Auto-restructure │
                  └────────┬─────────┘
                           │
                           ▼
                      Store restructured
```

**B. Metadata Guardian**

Enforces data integrity:

```python
def metadata_guardian(memory: dict) -> dict:
    # Timestamp enforcement
    if not memory.get('created_at') or not is_valid_iso8601(memory['created_at']):
        memory['created_at'] = datetime.utcnow().isoformat() + 'Z'

    # Agent attribution (from tags if column missing)
    if memory.get('agent_type') == 'unknown':
        agent_tag = extract_agent_from_tags(memory.get('tags', ''))
        if agent_tag:
            memory['agent_type'] = agent_tag
        else:
            # Reject or flag for manual review
            raise ValidationError("Agent attribution required")

    return memory
```

#### 3.2.4 Dynamic Relevance Scoring

**Purpose**: Replace saturated importance scores with utility-based relevance

**Schema Migration**:
```sql
-- Preserve original importance
ALTER TABLE memories ADD COLUMN initial_priority INTEGER;
UPDATE memories SET initial_priority = importance;

-- Add tracking columns
ALTER TABLE memories ADD COLUMN usage_count INTEGER DEFAULT 0;
ALTER TABLE memories ADD COLUMN last_accessed_at DATETIME;
ALTER TABLE memories ADD COLUMN dynamic_score FLOAT DEFAULT 0.0;

-- Index for efficient sorting
CREATE INDEX idx_dynamic_score ON memories(dynamic_score DESC);
```

**Scoring Algorithm**:
```python
def calculate_dynamic_score(memory: dict) -> float:
    """
    Calculate utility-based relevance score.

    Factors:
    - Usage frequency (how often retrieved)
    - Recency (when last accessed)
    - Initial priority (user-assigned importance)
    - Age decay (penalize stale memories)
    """
    USAGE_WEIGHT = 0.30
    RECENCY_WEIGHT = 0.35
    PRIORITY_WEIGHT = 0.25
    DECAY_WEIGHT = 0.10

    # Normalize usage (log scale to prevent outlier dominance)
    usage_score = min(1.0, math.log1p(memory['usage_count']) / 5.0)

    # Recency score (exponential decay, half-life = 7 days)
    days_since_access = (now() - memory['last_accessed_at']).days
    recency_score = math.exp(-0.099 * days_since_access)  # ln(2)/7 ≈ 0.099

    # Normalize priority (1-10 → 0-1)
    priority_score = memory['initial_priority'] / 10.0

    # Age decay (exponential, half-life = 30 days)
    days_since_creation = (now() - memory['created_at']).days
    age_penalty = 1.0 - math.exp(-0.023 * days_since_creation)  # ln(2)/30 ≈ 0.023

    return (
        (usage_score * USAGE_WEIGHT) +
        (recency_score * RECENCY_WEIGHT) +
        (priority_score * PRIORITY_WEIGHT) -
        (age_penalty * DECAY_WEIGHT)
    )
```

**Recalculation Schedule**: Background task every 6 hours

#### 3.2.5 Causal Link Enhancement

**Purpose**: Evolve graph from similarity index to causal model

**A. Local Causal Inference**

During relationship discovery, use local LLM to determine relationship type:

```python
def infer_relationship_type(mem_a: str, mem_b: str, similarity: float) -> str:
    """
    Use local LLM to determine relationship type instead of defaulting to 'similar'.
    """
    if similarity < 0.70:
        return None  # Not related enough

    prompt = f"""
    Analyze the relationship between these two memories:

    MEMORY A:
    {mem_a}

    MEMORY B:
    {mem_b}

    What is the relationship? Choose ONE:
    - causes: A led to or caused B
    - solves: B is a solution to problem A
    - contradicts: A and B present conflicting information
    - expands: B provides additional detail on A
    - supersedes: B replaces or updates A
    - similar: A and B are related but no causal connection

    Respond with only the relationship type.
    """

    return ollama_query("qwen2.5:3b", prompt).strip().lower()
```

**B. Tier 2 Feedback Ingestion**

**Endpoint**: `POST /api/v1/relationships/ingest_synthesis`

```json
{
  "source": "tier2_notebooklm",
  "synthesis_id": "syn-abc123",
  "relationships": [
    {
      "from_memory_id": "mem-123",
      "to_memory_id": "mem-456",
      "relationship_type": "causes",
      "confidence": 0.85,
      "reasoning": "Memory A describes the bug, Memory B describes the fix"
    }
  ]
}
```

**Storage**: Mark as high-confidence Tier 2-sourced relationships

---

## 4. Integration with codex-rs

### 4.1 Stage 0 Integration Point

Location: `codex-rs/tui/src/chatwidget/spec_kit/`

**New Module**: `notebooklm_bridge.rs`

```rust
//! NotebookLM integration for Stage 0 planning (SPEC-KIT-102)

use crate::chatwidget::spec_kit::error::{Result, SpecKitError};
use std::path::Path;

/// Configuration for Tier 2 integration
pub struct Tier2Config {
    pub enabled: bool,
    pub cache_ttl_hours: u32,
    pub max_brief_tokens: usize,
    pub notebook_id: Option<String>,
}

/// Result of Stage 0 synthesis
pub struct SynthesisResult {
    pub divine_truth: String,
    pub suggested_relationships: Vec<RelationshipSuggestion>,
    pub cache_hit: bool,
    pub latency_ms: u64,
}

/// Request Stage 0 synthesis from Tier 2
pub async fn request_synthesis(
    spec_content: &str,
    spec_id: &str,
    cwd: &Path,
    config: &Tier2Config,
) -> Result<SynthesisResult> {
    // 1. Compile context via local-memory
    let task_brief = compile_context(spec_content, cwd).await?;

    // 2. Check synthesis cache
    let cache_key = compute_cache_key(spec_content, &task_brief);
    if let Some(cached) = check_cache(&cache_key).await? {
        return Ok(SynthesisResult {
            divine_truth: cached.synthesis_result,
            suggested_relationships: cached.suggested_links,
            cache_hit: true,
            latency_ms: 5,
        });
    }

    // 3. Call NotebookLM via MCP bridge
    let start = std::time::Instant::now();
    let synthesis = call_notebooklm(&task_brief, spec_content, config).await?;
    let latency = start.elapsed().as_millis() as u64;

    // 4. Cache result
    store_in_cache(&cache_key, &synthesis).await?;

    // 5. Ingest suggested relationships back to Tier 1
    if !synthesis.suggested_relationships.is_empty() {
        ingest_relationships(&synthesis.suggested_relationships).await?;
    }

    Ok(SynthesisResult {
        divine_truth: synthesis.divine_truth,
        suggested_relationships: synthesis.suggested_relationships,
        cache_hit: false,
        latency_ms: latency,
    })
}
```

### 4.2 Pipeline Integration

**Modified Stage Flow**:

```
/speckit.auto SPEC-KIT-XXX
         │
         ▼
┌─────────────────────────────────────────┐
│ STAGE 0: PLANNING (NEW)                 │
│ • Compile context (Tier 1)              │
│ • Request synthesis (Tier 2)            │
│ • Inject "Divine Truth" into prompts    │
│ • Concurrency: 1 (sequential)           │
│ • Latency: 5ms (cached) / 5-15s (miss)  │
└─────────────────┬───────────────────────┘
                  │ divine_truth_brief
                  ▼
┌─────────────────────────────────────────┐
│ STAGE 1: SPECIFY                        │
│ • Receives divine_truth as context      │
│ • Standard consensus flow               │
└─────────────────┬───────────────────────┘
                  │
                  ▼
        (Stages 2-5 unchanged)
```

### 4.3 Configuration

**Location**: `~/.config/codex/tier2.toml`

```toml
[tier2]
enabled = true
notebook_id = "your-notebooklm-share-id"

[tier2.cache]
ttl_hours = 24
max_entries = 100

[tier2.context]
max_tokens = 8000
top_k = 15
include_domains = ["spec-kit", "infrastructure"]

[tier2.local_llm]
model = "qwen2.5:3b"
endpoint = "http://localhost:11434"
```

---

## 5. Data Migration Plan

### 5.1 Phase 0: Data Integrity Fixes

**Timeline**: Before any feature work

**Tasks**:
1. Audit `created_at` column format, fix parsing
2. Trace `agent_type` ingestion pipeline, fix attribution
3. Backfill timestamps where possible from file metadata
4. Extract `agent:*` tags to populate `agent_type` column

**Validation**:
```sql
-- Should return 0 after fix
SELECT COUNT(*) FROM memories WHERE created_at IS NULL OR agent_type = 'unknown';
```

### 5.2 Phase 1: Schema Migration

**Timeline**: Week 1

**Migration Script**:
```sql
-- Backup first
CREATE TABLE memories_backup AS SELECT * FROM memories;

-- Add new columns
ALTER TABLE memories ADD COLUMN initial_priority INTEGER;
ALTER TABLE memories ADD COLUMN usage_count INTEGER DEFAULT 0;
ALTER TABLE memories ADD COLUMN last_accessed_at DATETIME;
ALTER TABLE memories ADD COLUMN dynamic_score FLOAT DEFAULT 0.0;

-- Preserve importance
UPDATE memories SET initial_priority = importance;

-- Initialize dynamic score (first pass)
UPDATE memories SET dynamic_score = initial_priority / 10.0;

-- Create cache table
CREATE TABLE tier2_synthesis_cache (
    input_hash TEXT PRIMARY KEY,
    spec_hash TEXT NOT NULL,
    brief_hash TEXT NOT NULL,
    synthesis_result TEXT NOT NULL,
    suggested_links TEXT,
    created_at DATETIME NOT NULL,
    expires_at DATETIME NOT NULL,
    hit_count INTEGER DEFAULT 0,
    last_hit_at DATETIME
);

-- Create indexes
CREATE INDEX idx_dynamic_score ON memories(dynamic_score DESC);
CREATE INDEX idx_cache_expiry ON tier2_synthesis_cache(expires_at);
```

---

## 6. Implementation Phases

### Phase 0: Data Integrity (Week 1)
- [ ] Fix timestamp parsing issue
- [ ] Fix agent_type attribution
- [ ] Backfill missing metadata
- [ ] Validate data quality improvements

### Phase 1: Foundation (Weeks 2-3)
- [ ] Schema migration (dynamic scoring columns)
- [ ] Implement Template Guardian
- [ ] Implement Metadata Guardian
- [ ] Create tier2_synthesis_cache table
- [ ] Implement dynamic score calculation

### Phase 2: Core Integration (Weeks 4-5)
- [ ] Implement Dynamic Context Compiler endpoint
- [ ] Implement Tier 2 Synthesis Cache
- [ ] Create `notebooklm_bridge.rs` module
- [ ] Integrate Stage 0 into pipeline

### Phase 3: Enhancement (Weeks 6-7)
- [ ] Implement local causal inference
- [ ] Implement Tier 2 feedback ingestion
- [ ] Add predictive prefetching (optional)
- [ ] Performance optimization

### Phase 4: Personalization (Future)
- [ ] User DNA profile generation
- [ ] Anti-Mentor risk profiling
- [ ] Closed-loop pipeline feedback
- [ ] Autonomous memory compaction

---

## 7. Success Metrics

| Metric | Baseline | Target | Measurement |
|--------|----------|--------|-------------|
| Context relevance | Unknown | >80% precision | User feedback on Stage 0 output |
| Cache hit rate | N/A | >60% | Cache statistics |
| Stage 0 latency (cached) | N/A | <100ms | Instrumentation |
| Stage 0 latency (miss) | N/A | <20s | Instrumentation |
| Structured memories | 0% | >90% | Template validation |
| Graph causal links | 12 | >200 | Relationship query |
| Dynamic score variance | N/A | σ > 0.2 | Statistical analysis |

---

## 8. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| NotebookLM rate limit exhaustion | Medium | High | Strict Stage 0 only, caching |
| Local LLM quality insufficient | Low | Medium | Template fallback, human review flag |
| Cache invalidation complexity | Medium | Medium | Conservative TTL, manual purge option |
| Latency unacceptable | Low | High | Prefetching, optimistic UI |
| Data migration failure | Low | High | Backup, rollback plan |

---

## 9. Open Questions

1. **Prefetch Triggers**: Should prefetching activate on branch creation, file save, or both?
2. **Cache Invalidation**: TTL-based only, or content-aware invalidation?
3. **Fallback Behavior**: If NotebookLM unavailable, skip Stage 0 or use local-only synthesis?
4. **User DNA Scope**: Per-repository or global across all projects?
5. **Compaction Threshold**: Trigger at +50 memories or time-based (weekly)?

---

## 10. Appendices

### A. NotebookLM MCP Bridge Details

The integration uses `pleaseprompto/notebooklm-mcp` which operates via headless Chromium (patchright), not REST API.

**Implications**:
- Authentication requires stateful browser session (cookies)
- Runtime: ~300MB RAM per instance
- Parallelism impossible (single browser instance)
- Latency: 5-15 seconds per query

### B. Seeding Artifacts (Deprecated)

The original strategy of static aggregated artifacts has been **deprecated** in favor of Dynamic Context Compilation:

| Original Artifact | Status | Replacement |
|-------------------|--------|-------------|
| HISTORY_ROLLUP.md | Deprecated | Dynamic Context Compiler |
| LESSONS_LEARNED.md | Deprecated | Causal graph queries |
| ARCHITECTURE.pdf | Optional | Manual upload if desired |

### C. Related Specifications

- SPEC-KIT-099: Context Bridge (implementation deferred, superseded by this spec)
- SPEC-KIT-101: Branch Enforcement (independent, future work)
- SPEC-KIT-970: PRD Builder Modal (complete)
- SPEC-KIT-971: Clarify Modal + Project Detection (complete)

---

## 11. References

1. **PPP Framework**: "Training Proactive and Personalized LLM Agents" (Research Paper)
2. **NotebookLM MCP**: github.com/pleaseprompto/notebooklm-mcp
3. **Local Memory Environment**: docs/LOCAL-MEMORY-ENVIRONMENT.md
4. **Pipeline Analysis**: docs/SPECKIT-AUTO-PIPELINE-ANALYSIS.md

---

*Draft Version 1.0 - 2025-11-30*
*Ready for offline analysis and refinement*
