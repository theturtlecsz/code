# STAGE0_OBSERVABILITY.md

Observability for the Stage0 overlay engine serves two purposes:

1. Help you debug and tune Stage 0 behavior.
2. Provide a dataset for future V3 work (learned routing, learned utility).

---

## 1. Correlation IDs

- For each `run_stage0` invocation, generate a `request_id` (UUID).
- Include `request_id` in all logs emitted during that run:
  - DCC steps,
  - cache hits/misses,
  - NotebookLM calls,
  - causal link ingestion.

---

## 2. Stage 0 Run Event

Emit a structured event (to logs) for each Stage 0 run, e.g. as JSON:

```json
{
  "timestamp": "2025-11-30T15:42:01Z",
  "event_type": "stage0_run",
  "request_id": "3c1e31c4-9f13-4f2b-9e9a-0a034f3b9c5b",
  "spec_id": "SPEC-KIT-102",
  "tier2_used": true,
  "cache_hit": false,
  "tier2_latency_ms": 12450,
  "dcc": {
    "candidate_count": 87,
    "top_k": 15,
    "token_count": 3982
  },
  "result": {
    "status": "success",
    "error": null
  }
}
```

Use your logging framework of choice (e.g., `tracing` with JSON formatting).

---

## 3. Guardian Events

- **Metadata Guardian Warning:** when auto-filling or normalizing fields.
- **Template Guardian Error:** when LLM restructuring fails or times out.

Example:

```json
{
  "timestamp": "2025-11-30T15:39:01Z",
  "event_type": "metadata_guardian_warning",
  "memory_id": "mem-123",
  "reason": "auto-filled created_at",
  "fields": {
    "created_at_before": null,
    "created_at_after": "2025-11-30T15:39:01Z"
  }
}
```

```json
{
  "timestamp": "2025-11-30T15:40:55Z",
  "event_type": "template_guardian_error",
  "memory_id": "mem-123",
  "error": "LLM request timeout"
}
```

---

## 4. Cache Invalidation Events

When you invalidate Tier 2 cache entries due to memory updates in local-memory,
emit events such as:

```json
{
  "timestamp": "2025-11-30T15:45:22Z",
  "event_type": "tier2_cache_invalidation",
  "memory_id": "mem-789",
  "cache_hash": "sha256:abc123...",
  "reason": "memory_update"
}
```

This helps you understand how often Stage 0 is re-synthesizing expensive NotebookLM calls.

---

## 5. Explainability Snapshots

When Stage 0 runs with explainability enabled, you can emit a summary of the top-scoring memories:

```json
{
  "timestamp": "2025-11-30T15:43:10Z",
  "event_type": "dcc_explain_snapshot",
  "request_id": "3c1e31c4-9f13-4f2b-9e9a-0a034f3b9c5b",
  "spec_id": "SPEC-KIT-102",
  "top_memories": [
    {
      "id": "mem-001",
      "combined_score": 0.88,
      "components": {
        "similarity": 0.91,
        "dynamic_score": 0.82
      }
    },
    {
      "id": "mem-002",
      "combined_score": 0.84,
      "components": {
        "similarity": 0.87,
        "dynamic_score": 0.80
      }
    }
  ]
}
```

---

## 6. Storage

V1 does not mandate a dedicated metrics backend. You can:

- Emit JSON logs and collect them with your existing stack.
- Optionally write logs into the overlay DB for offline analysis.

Consistency and structure are more important than the actual sink.
