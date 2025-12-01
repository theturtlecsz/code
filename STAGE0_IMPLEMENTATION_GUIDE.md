# SPEC-KIT-102 Stage 0 Overlay Engine (V1) – Implementation Guide

This guide describes how to implement **SPEC-KIT-102 V1** as a **Stage 0 Overlay Engine**
around your existing **local-memory** daemon.

## 0. Reality: local-memory is a closed daemon

- The current `local-memory` daemon is **closed source**.
- It exposes a **REST API** and **MCP tools** (e.g., search, store_memory, relationships, analysis).
- You **cannot**:
  - add new REST endpoints,
  - modify its SQLite schema,
  - or change its internal logic.

Therefore, SPEC-KIT-102 V1 will be implemented as a **separate Rust layer** that:

- Lives in your own repo (e.g., as a new crate/module alongside `codex-rs`).
- Talks to local-memory via its existing REST/MCP APIs.
- Maintains its own small overlay SQLite database.
- Implements:
  - Ingestion guardians (for *your* memories).
  - Dynamic relevance scoring (`dynamic_score`).
  - Dynamic Context Compiler (DCC: IQO + hybrid search + diversity).
  - Tier 2 (NotebookLM) synthesis + cache.
  - Causal link ingestion back into local-memory via its relationships API.
  - Stage 0 observability.

The overlay engine is effectively **Tier 1.5**: a “Reasoning Manager” that treats local-memory
as a powerful backend, without trying to replace or modify it.

---

## 1. High-Level Architecture

### 1.1 Components

- **Local-Memory Daemon (Tier 0.9)** – black box
  - REST: `/memories`, `/search`, `/relationships`, etc.
  - MCP: `mcp__local-memory__search`, `store_memory`, `update_memory`, `relationships`, `analysis`, ...
  - Owns: raw memories, tags/domains, base relationships, its own analytics.

- **Stage0 Overlay Engine (Tier 1)** – new Rust module/crate (you own)
  - Maintains an overlay SQLite DB:
    - dynamic scores,
    - usage metadata,
    - structure status,
    - Tier 2 cache & dependencies,
    - optional raw content.
  - Implements:
    - Metadata Guardian & Template Guardian (for all writes from codex-rs).
    - Dynamic Scoring.
    - DCC (IQO + hybrid search + diversity reranking).
    - Tier 2 orchestration (NotebookLM) & caching.
    - Causal link ingestion via local-memory’s public APIs.
    - Structured logging for Stage 0 runs.

- **NotebookLM MCP (Tier 2)** – Staff Engineer
  - Expensive, slow, rate-limited.
  - Called only by Stage0 Overlay Engine as needed.

- **codex-rs** – client
  - Continues to own `/speckit.auto` pipeline.
  - Stage 0 step now calls Stage0 Overlay Engine instead of trying to do planning by hand.

### 1.2 Data Flow

```text
/speckit.auto (codex-rs)
     │
     ▼
[Stage0 Overlay Engine]
     │
     ├──> Guardians on new memories → local-memory.store_memory (MCP/REST)
     │
     ├──> DCC:
     │       - IQO via local LLM
     │       - local-memory.search / analysis
     │       - overlay scores
     │       - diversity reranking
     │       - summarization → TASK_BRIEF.md
     │
     ├──> Tier 2:
     │       - check overlay Tier2 cache
     │       - call NotebookLM MCP (on miss)
     │       - store synthesis + dependencies
     │       - ingest suggested links via local-memory.relationships
     │
     ▼
Divine Truth + TASK_BRIEF → fed into Stages 1–6 of /speckit.auto
```

---

## 2. Stage0 Overlay Phases (V1 Roadmap)

We keep the same spirit as the original SPEC-KIT-102 phases, but **all implementation occurs
in the overlay engine**, not inside local-memory.

### Phase A – Overlay DB + Data Integrity (Local)

- Create a small SQLite DB for the overlay (e.g., `~/.config/codex/local-memory-overlay.db`).
- Apply the schema in `STAGE0_SCHEMA.sql`.
- Optionally run a one-time sync job to seed overlay data from local-memory (e.g., import existing memory IDs
  and assign default scores).

**Done when:**

- Overlay DB exists and can be opened from Rust.
- Tables for scores, structure status, and Tier2 cache are created.
- There is a CLI or dev-only function to verify the overlay schema.

### Phase B – Ingestion Guardians (Your Writes Only)

- Implement **Metadata Guardian** in Rust:
  - Called before any `local_memory.store_memory` or `update_memory` you perform.
  - Normalizes timestamps to UTC RFC3339 with `Z`.
  - Normalizes or infers an `agent_type` tag/value (local convention).
  - Chooses an initial priority (1–10) for your overlay.

- Implement **Template Guardian** in Rust:
  - Before writing a new memory:
    - Run the Template Guardian LLM prompt to produce structured content.
    - Store structured content into local-memory.
    - Store `content_raw` + `structure_status` in overlay DB.
  - Optionally add an async worker to retro-fit older memories (pull from local-memory, restructure, write back).

**Done when:**

- All memories created by codex-rs from now on go through guardians.
- Overlay DB tracks structure status and raw content for new memories.

### Phase C – Dynamic Scoring + Usage Tracking

- Implement the dynamic scoring formula from `STAGE0_SCORING_AND_DCC.md` in Rust.
- Store `dynamic_score`, `usage_count`, and `last_accessed_at` in overlay DB.
- Update scores:
  - Periodically (e.g., every few hours), and
  - On-demand for memories used in DCC results.

**Done when:**

- Overlay DB has non-null `dynamic_score` for all overlay-tracked memories.
- There is a helper that, given a list of local-memory IDs, returns their scores.

### Phase D – Dynamic Context Compiler (DCC)

- Implement `compile_context(spec, env)` in Rust:
  1. Build IQO via local LLM (or simple heuristic fallback).
  2. Call local-memory.search / analysis using IQO-derived filters.
  3. Join results with overlay scores.
  4. Combine semantic similarity + dynamic_score.
  5. Apply diversity reranking (MMR-like).
  6. Summarize selected memories and assemble `TASK_BRIEF.md`.

- Provide an explainability mode that returns per-memory scores and components.

**Done when:**

- Stage0 engine can produce a sensible `TASK_BRIEF.md` for a known spec.
- `compile_context` is callable from codex-rs and handles small/large specs gracefully.

### Phase E – Tier 2 (NotebookLM) Integration + Cache

- Implement `run_stage0(spec_id, spec_content, env) -> Stage0Result`:

  1. Call `compile_context`.
  2. Compute `(spec_hash, brief_hash, input_hash)`.
  3. Check overlay Tier2 cache.
  4. On miss:
     - Call NotebookLM MCP with Tier 2 prompt.
     - Store synthesis_result + suggested_links + dependencies.
  5. Return Divine Truth, memories_used, and diagnostics.

- Implement cache invalidation for:
  - Any memory updates **you** perform via local-memory.
  - TTL-based expiry (e.g., 24h) for safety.

**Done when:**

- `run_stage0` works end-to-end for at least one `/speckit.auto` scenario.
- Cache hits and misses behave as expected.

### Phase F – Causal Link Ingestion & Observability

- Parse Tier 2’s suggested causal links and push them to local-memory via existing relationships APIs.
- Emit structured `stage0_run` events to logs with a correlation ID (`request_id`).

**Done when:**

- New relationships appear in local-memory for Tier 2 suggested links.
- Structured logs for Stage 0 runs are visible (for future V3 analysis).

---

## 3. Where to Put This in Your Repo

The exact paths depend on your codebase, but a reasonable structure is:

```text
codex-rs/
  crates/
    stage0_engine/
      src/
        lib.rs
        config.rs
        overlay_db.rs
        guardians.rs
        scoring.rs
        dcc.rs
        tier2.rs
        observability.rs
```

And then in the `/speckit.auto` pipeline in codex-rs, you:

- inject a `Stage0Engine` instance,
- call `stage0_engine.run_stage0(spec_id, spec_content, env)` at Stage 0,
- attach the returned Divine Truth + TASK_BRIEF to downstream prompts.

---

## 4. Notes for Claude Code

When using this package in Claude Code:

- Treat **local-memory** as a remote dependency accessed via MCP or REST client types.
- Do **not** attempt to change local-memory’s schema or endpoints.
- Focus on building the Stage0 overlay engine:
  - overlay DB schema,
  - Rust modules,
  - API for codex-rs to call,
  - integration with NotebookLM MCP.
- Use the other files in this zip (`STAGE0_SCHEMA.sql`, `STAGE0_SCORING_AND_DCC.md`, etc.)
  as detailed behavior specs for each module.
