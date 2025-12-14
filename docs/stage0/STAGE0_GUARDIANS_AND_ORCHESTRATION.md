# STAGE0_GUARDIANS_AND_ORCHESTRATION.md

This doc covers:

- Metadata & Template Guardians implemented in the overlay engine.
- Stage 0 orchestration (`run_stage0`).
- Tier 2 cache & invalidation using overlay DB.
- Causal link ingestion via local-memory.

---

## 1. Guardians on Your Writes

Since local-memory is closed, we only enforce guardians for **writes initiated by your code**
(e.g., codex-rs). Other tools may write less-structured content, but that is acceptable for V1.

### 1.1 Metadata Guardian

Invoked before any call to `store_memory` / `update_memory` MCP tools.

Responsibilities:

- Ensure timestamps are valid UTC RFC3339 strings with `Z` suffix.
- Attach an `agent_type` tag or field according to your conventions (`human`, `llm_claude`, etc.).
- Choose an initial priority (1–10) for overlay scoring.

Implementation:

- In Rust, write a function that takes your high-level `NewMemory` struct and returns a normalized version:
  - `created_at` populated with `Utc::now()` if missing,
  - `tags` enriched with something like `agent:llm_claude`,
  - `initial_priority` set based on context (e.g., 7 by default, or 9 for important spec artifacts).

- This function does **not** need to know local-memory’s schema beyond what the MCP tool requires.

### 1.2 Template Guardian

Responsibilities:

- Transform arbitrary text into the standard memory template:

  ```text
  [PATTERN|DECISION|PROBLEM|INSIGHT]: <One-line Summary>

  CONTEXT: <situation>

  REASONING: <why this approach>

  OUTCOME: <result or expected impact>
  ```

- Guarantee *no hallucinations*:
  - Only use information present in the input.
  - If a section is missing, leave it empty or mark as `TODO`.

Implementation:

- Before calling `local-memory.store_memory`, you:

  1. Save the raw text to your overlay DB (`content_raw`).
  2. Call a local LLM with the Template Guardian prompt to produce structured content.
  3. Send the structured content to local-memory as the `content` field.
  4. Record `structure_status='structured'` in overlay.

- For older memories, you may optionally run a background job to:
  - fetch them via MCP search,
  - restructure them,
  - update them via local-memory `update_memory`,
  - record structure_status in overlay.

---

## 2. Stage 0 Orchestration: `run_stage0`

The main entrypoint for codex-rs is a function like:

```rust
pub struct Stage0Result {
    pub divine_truth: String,
    pub task_brief_md: String,
    pub memories_used: Vec<String>,   // memory IDs
    pub cache_hit: bool,
    pub latency_ms: u64,
    pub explain_scores: Option<ExplainScores>,
}

impl Stage0Engine {
    pub async fn run_stage0(
        &self,
        spec_id: &str,
        spec_content: &str,
        env: &EnvCtx,
        explain: bool,
    ) -> anyhow::Result<Stage0Result> {
        // 1. DCC
        // 2. Cache lookup
        // 3. Tier 2 call on miss
        // 4. Cache store + dependency tracking
        // 5. Causal link ingestion
        // 6. Logging
    }
}
```

Steps in `run_stage0`:

1. **Compile Context**
   - Call `self.compile_context(spec_content, env, explain)` from `STAGE0_SCORING_AND_DCC.md`.
   - Receive `task_brief_md`, `memories_used`, `explain_scores`.

2. **Hashing & Cache Lookup**
   - Compute `spec_hash` and `brief_hash` (e.g., SHA-256 of spec and brief).
   - Compute `input_hash = hash(spec_hash + brief_hash)`.
   - Query `tier2_synthesis_cache` in overlay DB:
     - If hit:
       - increment `hit_count`,
       - update `last_hit_at`,
       - return cached `synthesis_result` as `divine_truth` (cache_hit = true).

3. **Tier 2 Call (NotebookLM MCP)**
   - On cache miss:
     - Construct the Tier 2 prompt using `spec_content` + `task_brief_md`.
     - Call NotebookLM MCP (e.g., via its MCP tool or a REST bridge).
     - Parse the result:
       - `divine_truth` (Markdown),
       - `suggested_links` (JSON array of causal relationships).

4. **Cache Store**
   - Insert into `tier2_synthesis_cache`:
     - `input_hash`, `spec_hash`, `brief_hash`, `synthesis_result`, `suggested_links`, timestamps.
   - Insert a row into `cache_memory_dependencies` for each `memory_id` in `memories_used`.

5. **Causal Link Ingestion**
   - For each item in `suggested_links`:
     - Validate: both `from_id` and `to_id` exist in local-memory (via MCP lookup, optional).
     - Call local-memory `relationships` MCP tool or REST endpoint to create edges
       with `relationship_type` (`causes`, `solves`, etc.) and embed Tier 2 reasoning in the context.

6. **Observability**
   - Record a `stage0_run` event containing:
     - `request_id` (UUID),
     - `spec_id`,
     - DCC stats,
     - Tier 2 usage & latency,
     - whether cache was hit,
     - and any error states.

---

## 3. Cache Invalidation in the Overlay

Since we cannot see all edits inside local-memory, we focus on **edits you perform**.

- Whenever codex-rs calls `update_memory` or similar to local-memory:
  - Also notify the Stage0 overlay engine (e.g., via a function call).
  - That function will:

    ```rust
    async fn invalidate_cache_for_memory(&self, memory_id: &str) -> Result<()> {
        let cache_hashes = self.overlay.find_cache_hashes_by_memory_id(memory_id).await?;
        for h in cache_hashes {
            self.overlay.delete_cache_entry(&h).await?;
            // log a cache invalidation event
        }
        Ok(())
    }
    ```

- Additionally, enforce a TTL-based policy in the overlay (e.g., entries older than 24h
  are treated as expired, even if not explicitly invalidated).
