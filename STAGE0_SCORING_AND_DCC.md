# STAGE0_SCORING_AND_DCC.md

This doc defines:

- Dynamic relevance scoring (`dynamic_score`) stored in the overlay DB.
- The Intent Query Object (IQO).
- The Dynamic Context Compiler (DCC) pipeline, with diversity reranking and explainability.

## 1. Dynamic Relevance Scoring

The scoring math is identical to the prior V1 spec, but implemented in your overlay instead
of inside local-memory.

Each overlay row has:

- `memory_id`        – local-memory's ID for the memory.
- `initial_priority` – 1–10 (copied from or chosen in place of `importance`).
- `usage_count`      – how many times this memory has been used in Stage 0 context.
- `last_accessed_at` – last time Stage 0 used this memory.
- `dynamic_score`    – the computed utility score.

### 1.1 Configuration

```yaml
scoring:
  recalculation_interval: "6h0m0s"
  weights:
    usage: 0.30
    recency: 0.30
    priority: 0.25
    decay: 0.15
  novelty_boost_threshold: 5
  novelty_boost_factor_max: 0.5
```

### 1.2 Formula (same as before, in overlay)

Let:

- `U = usage_count`
- `P = initial_priority` (1–10)
- `T_access = last_accessed_at` or (if NULL) `created_at` derived from local-memory
- `T_create = created_at` (via local-memory)
- `now = current time (UTC)`

```text
usage_score   = min(1.0, log(1 + U) / log(6))
recency_days  = max(0, days(now - T_access))
recency_score = exp(-ln(2) * recency_days / 7)
priority_score= clamp(P, 1, 10) / 10.0
age_days      = max(0, days(now - T_create))
age_penalty   = 1.0 - exp(-ln(2) * age_days / 30)
```

Novelty boost:

```text
if U < novelty_boost_threshold:
    novelty_factor = 1.0 + novelty_boost_factor_max * (1 - U / novelty_boost_threshold)
else:
    novelty_factor = 1.0
```

Base score:

```text
base_score =
    w_usage   * usage_score +
    w_recency * recency_score +
    w_priority* priority_score -
    w_decay   * age_penalty
```

Final:

```text
dynamic_score = clamp(base_score * novelty_factor, 0.0, 1.5)
```

### 1.3 Usage Tracking

- Whenever a memory appears in the final DCC `TASK_BRIEF.md`:
  - increment `usage_count`,
  - set `last_accessed_at = now`,
  - recompute `dynamic_score`.

- Optionally also update `usage_count` when the user manually recalls a memory via UI.

---

## 2. Intent Query Object (IQO)

The IQO shapes how you query local-memory.

```json
{
  "domains": ["spec-kit", "infrastructure"],
  "required_tags": ["spec:SPEC-KIT-102"],
  "optional_tags": ["stage:plan", "type:pattern"],
  "keywords": ["NotebookLM", "Tiered Memory"],
  "max_candidates": 150
}
```

Generation:

- Use a small local LLM (or heuristic) to transform the spec and environment into an IQO.
- See `STAGE0_CONFIG_AND_PROMPTS.md` for the IQO prompt.
- Log IQO outputs for later tuning.

---

## 3. DCC Pipeline in the Overlay Engine

The DCC implementation in Rust roughly follows:

```rust
pub async fn compile_context(&self, spec: &str, env: &EnvCtx, explain: bool) -> CompileContextResult
```

Steps:

1. **IQO Generation**
   - Call local LLM with spec + env to get IQO JSON.
   - If LLM fails, fall back to a simple IQO:
     - domain = "spec-kit"
     - keywords = top N words from spec title/body.

2. **Local-Memory Query**
   - Use IQO to construct search requests to local-memory:
     - Example: filter by domains/tags via query parameters or the analysis tool.
   - Retrieve candidate memories (ID + content + tags + created_at, if available).

3. **Join with Overlay & Score Combination**
   - For each candidate memory:
     - look up overlay row (create on the fly if missing),
     - retrieve or compute `dynamic_score`,
     - get semantic similarity (either from local-memory or your own embeddings).
   - Combine:

     ```text
     final_score =
         semantic_similarity_weight * similarity_score +
         dynamic_score_weight       * dynamic_score
     ```

     where weights come from config.

4. **Diversity Reranking (MMR)**

   - Apply an MMR-like process to select Top-K:

     ```text
     Selected = []
     Candidates = sort_by(final_score desc)

     while len(Selected) < top_k and Candidates not empty:
         for c in Candidates:
             diversity_penalty = max_{s in Selected} sim(c, s)
             mmr_score = λ * c.final_score - (1-λ) * diversity_penalty
         pick highest mmr_score → move to Selected
     ```

   - sim(c, s) can reuse Qdrant similarity, or a local cosine similarity between embeddings.

5. **Summarization & TASK_BRIEF.md**

   - Summarize each selected memory using a local summarization prompt.
   - Combine snippets into a Markdown brief describing:
     - patterns/decisions,
     - pitfalls,
     - key architectural notes.
   - Enforce `max_tokens` by dropping least-important entries or compressing further.

6. **Explainability (Optional)**

   - If `explain=true`, return a structure like:

     ```json
     {
       "memories": [
         {
           "id": "mem-123",
           "similarity": 0.91,
           "dynamic_score": 0.82,
           "combined_score": 0.88,
           "components": {
             "usage": 0.7,
             "recency": 0.9,
             "priority": 1.0,
             "age_penalty": 0.2,
             "novelty_boost": 0.3
           }
         }
       ]
     }
     ```

This DCC runs entirely in your overlay engine, talking to local-memory only for raw memories
and using your overlay DB for scores and metadata.
