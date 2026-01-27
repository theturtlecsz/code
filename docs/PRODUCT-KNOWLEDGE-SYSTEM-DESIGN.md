# Product Knowledge System Design (Canonical)

This is the **canonical architecture document** for the Product Knowledge integration (`codex-product`) in Spec‑Kit / `codex-rs`, including **systems-of-record (SoR) boundaries**, **replay determinism**, and **failure semantics**.

## Executive Summary

Spec‑Kit uses a three-surface “truth model”:

* **Memvid capsule (`mv2://…`)** is the SoR for spec‑kit artifacts/events/policy snapshots.
* **OverlayDb (Stage0)** is the SoR for vision/constitution and operational caches.
* **local-memory domain `codex-product`** is a curated product knowledge layer (decisions/patterns/discoveries/milestones) used to improve planning and reduce redundant Tier2 calls; it is **not** a SoR.

Product knowledge consultation must remain **optional**, **non-invasive**, and **deterministic**: if it influences a run, the run must be replayable offline without requiring local-memory access by snapshotting the used inputs into capsule (and/or emitting retrieval events).

```mermaid
flowchart LR
  %% System / Truth Surfaces Diagram (component graph)
  SpecKit[Spec‑Kit Pipeline\n(/speckit.*)] --> Stage0[Stage0Engine\n(DCC + Tier2)]

  subgraph SoR[Systems of Record]
    Capsule[(Memvid Capsule\nmv2://…)]
    Overlay[(OverlayDb\nVision/Constitution + caches)]
  end

  subgraph Curated[Curated Knowledge Layer (NOT SoR)]
    LM[(local-memory\nDomain: codex-product)]
  end

  subgraph Tier2[Tier2 Synthesis]
    NLM[[NotebookLM\nNotebook: codex-product]]
  end

  FS[(Filesystem projections\n docs/, memory/ )]

  Stage0 --> Overlay
  Stage0 --> Capsule
  Stage0 -.->|best-effort (search/remember)| LM
  Stage0 -.->|HTTP| NLM
  NLM -.->|curated outputs| LM

  Capsule --> FS
  Overlay --> FS
```

## Non‑Negotiables (SoR boundaries + determinism)

### Hard Constraints (Non‑Negotiables) — verbatim

* Capsule (`mv2://…`) is the system of record (SoR) for spec-kit artifacts/events/policy snapshots.
* Vision/constitution SoR is OverlayDb (not capsule).
* Filesystem (`docs/`, `memory/`) artifacts are projections; projection failures must not block SoR writes.
* local-memory must remain a separate product knowledge + curation + semantic layer (not a competing SoR).
* local-memory usage policy: CLI + REST only; NO MCP. Do not configure local-memory as an MCP server.
* Upstream compatibility: integration must be optional/non-invasive; never block core flows if local-memory is unavailable.

### Required Evidence Anchors

Note: anchors intentionally avoid `path:line` to reduce anchor rot. Prefer `path (symbol)` and use `rg`/tree-sitter to locate call-sites.

* Capsule SoR: `codex-rs/docs/MODEL-POLICY.md (System of Record → Memvid-First Architecture)`
* Vision SoR: `codex-rs/tui/src/chatwidget/spec_kit/vision_core.rs (persist_vision_to_overlay)`
* Projection failures don’t block: `codex-rs/cli/src/speckit_cmd.rs`
* Stage0 memory backend routing / best-effort local-memory health gating: `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec)`
* Retrieval events schema: `codex-rs/tui/src/memvid_adapter/types.rs (RetrievalRequestPayload, RetrievalResponsePayload)`
* Capsule artifact write API: `codex-rs/tui/src/memvid_adapter/capsule.rs (CapsuleHandle::put)`
* ADR schema for `ProductKnowledgeEvidencePack`: `docs/adr/ADR-003-product-knowledge-layer-local-memory.md (Capsule Snapshot Schema: ProductKnowledgeEvidencePack)`

## Current State (As‑Is)

### What exists in code today

* **Stage0 execution entrypoint (TUI)**: `run_stage0_for_spec` (`codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec)`)
  * Loads `Stage0Config`, routes by `memory_backend`, and gates local-memory health for the LocalMemory backend (`codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec)`).
* **Tier1 context compilation (DCC)**: `Stage0Engine::compile_context` (`codex-rs/stage0/src/lib.rs (Stage0Engine::compile_context)`) → `dcc::compile_context` (`codex-rs/stage0/src/dcc.rs (compile_context)`).
* **Product Knowledge lane (Tier1)** (when enabled):
  * Deterministic query builder: `build_product_knowledge_query` (`codex-rs/stage0/src/dcc.rs (build_product_knowledge_query)`)
  * Domain-scoped search via `LocalMemoryClient`: `ctx.local_mem.search_memories(...)` in DCC step 12 (`codex-rs/stage0/src/dcc.rs (compile_context: search_memories)`)
  * Deterministic filtering and lane assembly: `assemble_product_knowledge_lane` (`codex-rs/stage0/src/dcc.rs (assemble_product_knowledge_lane)`)
  * Evidence pack built per ADR: `build_product_knowledge_pack` (`codex-rs/stage0/src/dcc.rs (build_product_knowledge_pack)`) and ADR schema (`docs/adr/ADR-003-product-knowledge-layer-local-memory.md (Capsule Snapshot Schema: ProductKnowledgeEvidencePack)`)
* **Pre-check before Tier2 (Prompt F)**:
  * Best-effort search of `codex-product` and threshold gating to skip Tier2 (`codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec: codex_stage0::precheck_product_knowledge)`)
  * Shared precheck logic: `precheck_product_knowledge` (`codex-rs/stage0/src/dcc.rs (precheck_product_knowledge)`)
* **Post-curation after Tier2 (Prompt F)**:
  * Runs in a background thread, never blocks the pipeline (`codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec: post-curation thread spawn)`)
  * Curation adapter: `ProductKnowledgeCurationAdapter::curate_tier2_output` (`codex-rs/tui/src/stage0_adapters.rs (ProductKnowledgeCurationAdapter::curate_tier2_output)`)
  * Stores via local-memory CLI wrapper: `local_memory_cli::remember_blocking` (`codex-rs/tui/src/local_memory_cli.rs (remember_blocking)`)
* **Capsule snapshot is enforced by pipeline coordinator**:
  * Enforcement: `persist_or_strip_product_knowledge` (`codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (persist_or_strip_product_knowledge)`) called from `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs (persist_or_strip_product_knowledge call site)`
  * Helper: `write_product_knowledge_to_capsule` (`codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (write_product_knowledge_to_capsule)`)
  * Uses `CapsuleHandle::put` (`codex-rs/tui/src/memvid_adapter/capsule.rs (CapsuleHandle::put)`) with path `artifact/product_knowledge/evidence_pack.json`

### Current vs target summary

| Area                           | As‑Is                                                                                                                 | To‑Be         |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------- | ------------- |
| Product knowledge lane (Tier1) | Assembled inside DCC when enabled; conditioned on determinism snapshot rules via `persist_or_strip_product_knowledge` | Same behavior |
| Evidence pack snapshot         | Enforced via `persist_or_strip_product_knowledge` (pipeline\_coordinator.rs:565); omits lane if snapshot fails        | Same behavior |
| Retrieval events               | Correlated events emitted for lane + precheck via Phase 2 implementation                                              | Same behavior |
| Pre‑check (Tier2 avoidance)    | Implemented when enabled + LocalMemory backend; best-effort, non-blocking                                             | Same behavior |
| Post‑curation                  | Implemented as background best-effort `remember` calls; curated (importance ≥ 8, canonical types)                     | Same behavior |

## Target State (To‑Be)

### Goals

1. **Product knowledge lives in local-memory (`codex-product`) and is not a SoR**.
2. **Stage0/spec-kit consults `codex-product` without breaking replay determinism**.
3. **Failures degrade without blocking**, and degraded mode is auditable.

### Minimal-churn target behavior (uses existing primitives)

* Keep the lane generation and evidence pack schema as defined in ADR‑003.
* Wire capsule snapshotting via existing `CapsuleHandle::put` and existing artifact path contract.
* Emit retrieval request/response events using existing payloads (recommended).

```mermaid
sequenceDiagram
  %% Sequence Diagram: Stage0 run with Product Knowledge lane
  participant SK as Spec‑Kit Pipeline
  participant S0 as Stage0 Integration
  participant ENG as Stage0Engine (DCC)
  participant LM as local-memory (codex-product)
  participant CAP as Capsule (mv2://…)
  participant NLM as NotebookLM

  SK->>S0: run_stage0_for_spec(spec_id, spec.md)
  S0->>S0: load Stage0Config (flags + memory_backend)

  alt product_knowledge enabled AND local-memory healthy
    ENG->>LM: search(codex-product, deterministic query)
    ENG-->>S0: Stage0Result(product_knowledge_pack, task_brief_md)
    S0->>CAP: put(artifact/product_knowledge/evidence_pack.json)
    opt emit retrieval events (recommended)
      S0->>CAP: emit RetrievalRequest/Response
    end
  else local-memory unavailable/unhealthy
    Note over S0,LM: Best-effort gating; never blocks core flows
    ENG-->>S0: Stage0Result(without lane/pack)
  end

  alt Tier2 enabled AND no pre-check hit
    S0->>NLM: healthcheck + query
    NLM-->>S0: Tier2 synthesis
    opt post-curation enabled
      S0-->>LM: remember(durable insights) [async, non-blocking]
    end
  else pre-check hit
    Note over S0,LM: Skip Tier2; snapshot reused evidence
  end
```

## Determinism & Snapshotting (events + evidence pack)

### Snapshot rule

If `codex-product` knowledge influences Stage0 outputs (lane injection, Tier2 skipped due to cached insights), **snapshot what was used** such that offline replay does not require live local-memory access.

### Evidence pack artifact

ADR‑003 defines the capsule snapshot schema:

* `ProductKnowledgeEvidencePack` schema: `docs/adr/ADR-003-product-knowledge-layer-local-memory.md (Capsule Snapshot Schema: ProductKnowledgeEvidencePack)`
* Recommended capsule path: `mv2://<workspace>/<spec_id>/<run_id>/artifact/product_knowledge/evidence_pack.json`
* Capsule write primitive: `codex-rs/tui/src/memvid_adapter/capsule.rs (CapsuleHandle::put)`

```mermaid
flowchart TD
  %% Data / Artifact Diagram
  subgraph CAP[Capsule artifact]
    PK[ProductKnowledgeEvidencePack\nschema_version=product_knowledge_evidence_pack@1.0]
    PK --> Q[queries[]\n(query, mode, limit, filters, executed_at)]
    PK --> I[items[]\n(lm_id, type, importance, tags,\ncontent, content_sha256,\nwhy_included, snippets[])]
    PK --> H[integrity\n(pack_sha256)]
  end

  subgraph EVT[Optional capsule events]
    RR[RetrievalRequestPayload\n(request_id, query, config, source)]
    RS[RetrievalResponsePayload\n(request_id, hit_uris, fused_scores, error)]
    RR --> RS
  end

  PK -.->|correlates via request_id| RR
```

### Retrieval events (recommended complement)

* Payload definitions: `codex-rs/tui/src/memvid_adapter/types.rs (RetrievalRequestPayload, RetrievalResponsePayload)`
* Emit helpers: `codex-rs/tui/src/memvid_adapter/capsule.rs (CapsuleHandle::emit_retrieval_request/emit_retrieval_response)`

Recommended practice:

* Emit `RetrievalRequest` before search and `RetrievalResponse` after search with a shared `request_id`.
* Store the evidence pack artifact for deterministic offline replay of the exact text inputs used.

## Failure Semantics (best-effort, non-blocking)

### local-memory unavailable / unhealthy

* Skip product knowledge lane and pre-check; proceed with capsule + OverlayDb behavior only.
* Record a degraded-mode diagnostic (event preferred) so the run is auditable.
* Must not block core flows (hard constraint).

### Capsule artifact write fails (evidence pack)

* Treat artifact writes as best-effort (do not block pipeline).
* Determinism rule: if snapshotting fails, **omit the lane** (or ensure an equivalent deterministic capsule artifact exists) and record a degraded-mode diagnostic.

### Filesystem projection failures

* Filesystem is projection only; projection failures must not block SoR writes.
* Evidence anchor: `codex-rs/cli/src/speckit_cmd.rs`

## Config & Enforcement (deterministic, upstream compatible)

### Configuration surface

* `Stage0Config.context_compiler.product_knowledge.*` (default OFF):
  * `enabled`, `domain`, `max_items`, `max_chars_per_item`, `max_total_chars`, `min_importance`
  * `precheck_enabled`, `precheck_threshold`, `curation_enabled`
  * See: `codex-rs/stage0/src/config.rs (Stage0Config)`
* Backend routing and health gating:
  * `Stage0Config.memory_backend` routed in `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec)`

### Deterministic enforcement rules

* Query generation is heuristics-only (no LLM): `codex-rs/stage0/src/dcc.rs (build_product_knowledge_query)`
* Filtering is deterministic and client-side: `codex-rs/stage0/src/dcc.rs (assemble_product_knowledge_lane)`
* local-memory integration remains CLI + REST only (no MCP): `codex-rs/tui/src/stage0_adapters.rs` (module docs)

## Code Map (Tree-sitter) — key modules/entrypoints + responsibilities

Tree-sitter-informed code analysis was used to confirm that diagrams align with code ownership and call-sites.

### Entrypoints (Stage0/spec-kit)

* Pipeline wiring:
  * `super::stage0_integration::run_stage0_for_spec(...)`: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` (search: `run_stage0_for_spec(`)
  * `super::stage0_integration::spawn_stage0_async(...)`: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` (search: `spawn_stage0_async(`)
* Stage0 integration entrypoints:
  * `spawn_stage0_async`: `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (spawn_stage0_async)`
  * `run_stage0_for_spec`: `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec)`
* Stage0 engine entrypoints:
  * `Stage0Engine::run_stage0`: `codex-rs/stage0/src/lib.rs (Stage0Engine::run_stage0)`
  * `Stage0Engine::compile_context`: `codex-rs/stage0/src/lib.rs (Stage0Engine::compile_context)`
  * `dcc::compile_context`: `codex-rs/stage0/src/dcc.rs (compile_context)`

### Where retrieval events are emitted

* Payload schema: `codex-rs/tui/src/memvid_adapter/types.rs (RetrievalRequestPayload, RetrievalResponsePayload)`
* Capsule emit helpers:
  * `CapsuleHandle::emit_retrieval_request`: `codex-rs/tui/src/memvid_adapter/capsule.rs (CapsuleHandle::emit_retrieval_request)`
  * `CapsuleHandle::emit_retrieval_response`: `codex-rs/tui/src/memvid_adapter/capsule.rs (CapsuleHandle::emit_retrieval_response)`
* Higher-level wrapper:
  * `AuditEventEmitter::emit_retrieval_request`: `codex-rs/tui/src/chatwidget/spec_kit/event_emitter.rs (AuditEventEmitter::emit_retrieval_request)`
  * `AuditEventEmitter::emit_retrieval_response`: `codex-rs/tui/src/chatwidget/spec_kit/event_emitter.rs (AuditEventEmitter::emit_retrieval_response)`

### Where capsule artifacts are written

* Primary API: `CapsuleHandle::put`: `codex-rs/tui/src/memvid_adapter/capsule.rs (CapsuleHandle::put)`
* Product knowledge evidence pack helper: `write_product_knowledge_to_capsule`: `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (write_product_knowledge_to_capsule)`

### Where local-memory CLI/REST is invoked (no MCP)

* Health gating + CLI wrappers: `codex-rs/tui/src/local_memory_cli.rs` (module docs)
* Stage0 adapter: `LocalMemoryCliAdapter::search_memories`: `codex-rs/tui/src/stage0_adapters.rs (LocalMemoryCliAdapter::search_memories)`
* Pre-check search call: `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec: codex_stage0::precheck_product_knowledge)`
* Post-curation write call: `codex-rs/tui/src/stage0_adapters.rs (ProductKnowledgeCurationAdapter::curate_tier2_output: remember_blocking call)`

### Reproducible AST checks (optional)

The following commands reproduce key facts used in the code map above (definitions + call-sites) using AST-based tooling:

```bash
# Parse sanity (should succeed on .rs files)
tree-sitter parse --quiet codex-rs/stage0/src/dcc.rs

# Tags-based discovery (symbols + rough locations)
tree-sitter tags codex-rs/stage0/src/dcc.rs | rg 'ProductKnowledgeEvidencePack|build_product_knowledge_query|assemble_product_knowledge_lane|build_product_knowledge_pack'

# AST-based exact matches (structural)
sg run -l Rust -p 'fn build_product_knowledge_query($$$) { $$$ }' codex-rs/stage0/src/dcc.rs
sg run -l Rust -p 'write_product_knowledge_to_capsule($$$)' codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs
```

If `tree-sitter parse` reports “No language found”, install `tree-sitter-rust` and ensure it’s in a configured parser directory (see `tree-sitter init-config`).

## Rollout Plan (phases, defaults, success metrics)

### Phase 0: canonical doc + safe defaults

* Keep `product_knowledge.enabled = false` as default.
* Treat this document as the canonical reference for product knowledge integration.

### Phase 1: enforce deterministic snapshotting

* Wire `write_product_knowledge_to_capsule` into the pipeline after Stage0 completes when capsule access is available.
* Enforce: if snapshot write fails, omit lane + record degraded diagnostic.
* Success metrics:
  * Runs with product knowledge enabled contain `artifact/product_knowledge/evidence_pack.json` in capsule.
  * Offline replay does not require local-memory access.

### Phase 2: add correlated retrieval events (recommended)

* Emit `RetrievalRequest/Response` around `codex-product` searches and pre-check reuse paths.
* Success metrics:
  * Retrievals have correlated request/response events and are auditable.

### Phase 3: dogfood and measure Tier2 reduction

* Track:
  * Tier2 calls avoided (pre-check hit rate)
  * Quality of curated insights (importance ≥ 8, canonical types)
  * Snapshot failure rate (should not block; should degrade deterministically)

## Open Questions / Risks (if any)

* ~~When `memory_backend = memvid`, should product knowledge lane still consult local-memory...~~ **Resolved**: Product knowledge consults local-memory (`codex-product`) best-effort regardless of `memory_backend`. When memvid is SoR, product knowledge still sources from local-memory and snapshots evidence to capsule.
* ~~Determinism enforcement point: the lane is currently assembled inside DCC; enforcing "snapshot succeeded ⇒ lane included" may require moving the injection boundary.~~ **Resolved**: Tier2 prompt + Tier2 cache key use PK-stripped brief; PK lane is for downstream prompts only.
* Privacy: evidence packs store content; eligibility must remain "curated insights only" (importance ≥ 8, canonical types, no system:true).
