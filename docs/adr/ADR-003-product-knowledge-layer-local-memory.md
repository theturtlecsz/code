# ADR-003: Product Knowledge Layer (local-memory `codex-product`) with Memvid + OverlayDb SoR

**Status:** Accepted\
**Date:** 2026-01-25\
**Deciders:** Product + Architecture (locked decision)\
**Context:** Spec-Kit “Architect-in-a-box” continuity + replay determinism

***

## Decision

Adopt a **three-surface truth model**:

1. **Memvid capsule (`mv2://…`)** is the **system of record (SoR)** for **spec-kit artifacts, events, and policy snapshots**.
2. **OverlayDb (Stage0)** is the **SoR for vision/constitution** (operational state + caches).
3. **local-memory domain `codex-product`** is a **separate, curated, cross-project product knowledge graph** (decisions/patterns/discoveries/milestones) with semantic retrieval and explicit relationships.

**Integration principle:** local-memory is **never required** for core spec-kit flows. If local-memory is unavailable, spec-kit proceeds using capsule + OverlayDb only, while recording a degraded-mode trace for audit/replay.

***

## SoR Boundaries (Non-Negotiable)

### Capsule is SoR for spec-kit artifacts/events/policy snapshots

* Canonical statement: `codex-rs/docs/MODEL-POLICY.md (System of Record → Memvid-First Architecture)`
* Project tracker statement: `codex-rs/SPEC.md (Invariants → System of Record)`

### OverlayDb is SoR for vision (not capsule)

* Vision persistence core: `codex-rs/tui/src/chatwidget/spec_kit/vision_core.rs (persist_vision_to_overlay)`
* Projection command design notes: `codex-rs/tui/src/chatwidget/spec_kit/commands/projections.rs` (module docs)

### Filesystem is projection; projection failures must not block SoR writes

* Headless intake flow: `codex-rs/cli/src/speckit_cmd.rs`
* Ops contract: `docs/OPERATIONS.md (Capsule as System of Record)`

### local-memory integration discipline (no MCP)

* Stage0 adapters (CLI+REST for local-memory; HTTP-only for NotebookLM): `codex-rs/tui/src/stage0_adapters.rs` (module docs)
* Policy: `/home/thetu/infra/localmemory-policy/skills/local-memory/SKILL.md (Non-negotiables)`

### Upstream compatibility

* local-memory must never become "required" for memvid runs; mirror best-effort health gating patterns:
  `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec)`

***

## What local-memory MUST / MUST NOT Do

### MUST be used for

* A curated **product knowledge store** that is:
  * cross-project,
  * semantically retrievable,
  * relationship-rich (insight graph),
  * usable by automated systems (stable types/tags/quality rules),
  * and references capsule/OverlayDb via evidence pointers (not foreign-key edges).

### MUST NEVER be used for

* Competing SoR for spec-kit artifacts/events/policy snapshots (capsule remains SoR).
* Replacing OverlayDb as the vision/constitution SoR.

***

## Architecture (High-Level)

Treat each system as a different truth surface:

* **Capsule (`mv2://`)**: immutable provenance record + replay/time-travel.
* **OverlayDb**: operational state + vision constitution store + Tier2 caches.
* **local-memory (`codex-product`)**: curated, cross-project semantic product knowledge graph.

### Data flow (conceptual)

1. **Before** product question / planning: search `codex-product` for relevant prior insights.
2. If needed, call Tier2 (NotebookLM) for long-context synthesis (optional/explicit).
3. Curate durable outputs back into `codex-product`.
4. If product knowledge influences any run output, **snapshot what was used into capsule** for replay determinism.

***

## Replay Determinism: Snapshotting “What Was Used”

### Invariant

If Stage0/spec-kit uses local-memory to influence outputs, the run must be replayable offline with “retrieval + events is exact”.

* Determinism statement: `codex-rs/SPEC.md (Invariants → Replay Determinism)`
* Retrieval event payloads exist: `codex-rs/tui/src/memvid_adapter/types.rs (RetrievalRequestPayload, RetrievalResponsePayload)`

### Mechanisms (use either/both)

1. **Capsule artifact (recommended):** store a `ProductKnowledgeEvidencePack` used during the run.
2. **Capsule retrieval events:** emit retrieval request/response events (already modeled).

The artifact is preferred for deterministic replay without requiring local-memory access and for convenient offline inspection.

***

## Capsule Snapshot Schema: `ProductKnowledgeEvidencePack`

### Purpose

Persist a deterministic, offline-replayable record of the local-memory product knowledge inputs used by a run or stage.

### Artifact location (recommended)

* `mv2://<workspace>/<spec_id>/<run_id>/artifact/product_knowledge/evidence_pack.json`

### Schema (design)

* `schema_version`: `product_knowledge_evidence_pack@1.0`
* `created_at`: RFC3339 timestamp
* `created_via`: `stage0_context_compiler|tier2_precheck|manual`
* `domain`: always `codex-product`
* `queries[]`:
  * `query`: string (the text query used)
  * `mode`: `search|recall`
  * `limit`: integer
  * `filters`: `{ types?: [], min_importance?: number, tags?: [] }`
  * `executed_at`: RFC3339 timestamp
* `items[]` (the evidence actually used):
  * `lm_id`: string (local-memory ID)
  * `type`: canonical type
  * `importance`: integer
  * `tags[]`: string
  * `content`: full text (WHAT/WHY/EVIDENCE/OUTCOME)
  * `content_sha256`: sha256 of `content`
  * `why_included`: short rationale (1–2 sentences)
  * `snippets[]` (optional): selected excerpts used by the system
* `integrity`:
  * `pack_sha256`: sha256 of canonical JSON serialization
  * `notes` (optional): any integrity caveats

### Minimal enforcement rule

If local-memory product knowledge is used, **either**:

* at least one `RetrievalRequest/Response` event pair is emitted, **or**
* a `ProductKnowledgeEvidencePack` artifact is written.

Preferred: do both (events for timeline, artifact for deterministic payload).

***

## local-memory Domain Contract: `codex-product`

### Allowed content (“insights only”)

Each entry must be an insight with:

* **WHAT**: the decision/pattern/discovery/etc.
* **WHY**: rationale.
* **EVIDENCE**: references to `mv2://…`, doc paths, PRs/commits, or observed behavior.
* **OUTCOME**: what changed / why it matters / how to apply.

Ops guidance: `docs/OPERATIONS.md (Memory Workflow quick reference)`

### Canonical `type` values

Use one of:

* `decision`
* `pattern`
* `bug-fix`
* `milestone`
* `discovery`
* `limitation`
* `architecture`

Policy: `/home/thetu/infra/localmemory-policy/skills/local-memory/SKILL.md (Enforced canonical type values)`

### Tagging rules

Required tags (minimum):

* `component:<name>` (e.g., `component:spec-kit`, `component:capsule`, `component:stage0`, `component:intake`)

Optional tags:

* `area:<lane>` (e.g., `area:product-knowledge`, `area:replay`, `area:policy`)
* `status:<draft|accepted|superseded>`

### Relationship rules

* Relationships are **insight → insight only** (no edges to capsule entities).
* Evidence pointers inside content may reference `mv2://...` URIs and/or doc paths.

### Promotion / quality criteria

Store to `codex-product` only if:

* importance ≥ 8 (durable, re-usable),
* type is canonical,
* includes WHAT/WHY/EVIDENCE/OUTCOME,
* and is linkable (component tag + at least one relationship to an existing insight when obvious).

***

## NotebookLM: Single Product Notebook + local-memory Caching

### Requirement

Maintain a **single product NotebookLM notebook** for Tier2 product knowledge synthesis.

**Canonical notebook identifier:** `codex-product` (any stable unique ID is acceptable).

### Pre-check (avoid redundant NotebookLM calls)

Before issuing a product question to NotebookLM:

1. Search `codex-product` for near-duplicates (same question, same decision).
2. If a strong match exists, reuse it and snapshot the evidence into capsule.

### Post-write (distill durable outputs)

After a NotebookLM call:

1. Distill the output into one or more durable `codex-product` insights (if it meets quality threshold).
2. Link to related insights (relationships).
3. Snapshot the used evidence pack into capsule if it influences run outputs.

### Cache interplay (OverlayDb vs local-memory)

* OverlayDb Tier2 cache remains the operational cache for quick reuse:
  `codex-rs/stage0/src/overlay_db.rs (Tier2CacheEntry)`
* local-memory is the curated durable knowledge store (not a raw blob dump).

Optionally, store “Q/A cache” memories only if explicitly tagged as system/internal and curated to avoid noise.

***

## Workflow Integration Points (Spec-Kit)

### Where product knowledge is consulted

* Stage0 Tier1 context compilation optionally includes a **Product Knowledge lane** sourced from `codex-product`.
* Before Tier2 product questions, consult `codex-product` (pre-check).
* After Tier2 calls, curate/store durable insights back into `codex-product`.

### SoR safety rule

Product knowledge can influence outputs only if the run snapshots its inputs into capsule (events and/or evidence pack).

***

## Failure Semantics (Required)

### local-memory unavailable/unhealthy

* Proceed with capsule + OverlayDb-only behavior (never block core flows).
* Record degraded-mode trace in capsule (event and/or artifact) so the run’s context is auditable.

### NotebookLM unavailable/unhealthy

* Proceed Tier1-only.
* Prefer: OverlayDb cache + `codex-product` prior insights (if available).
* Record Tier2 degraded-mode trace (for audit).

***

## Enforcement Strategy (Behavioral + Config)

### Principle

Do not rely on “skills” alone. Skills are probabilistic; configs/hooks are deterministic.

Reference: `~/local-memory-configs-export/README.md`

### Required environment alignment

Ensure Code TUI + Codex CLI environments:

1. Run minimal `lm` checks early (domain + recall/search) for product context.
2. Explicitly instruct product-knowledge retrieval from `codex-product`.
3. Do not configure local-memory as MCP (CLI + REST only).

### Known gap to fix (config export)

`~/local-memory-configs-export/project-examples/code-project/AGENTS.md` is missing the local-memory section.

Plan: update config export templates so new projects consistently include the local-memory discipline and the `codex-product` domain.

***

## What Not To Build Right Now

Defer additional Memvid work unless local-memory cannot logically provide the capability.

Examples of deferables:

* memvid adjacency/type listing TODOs: `codex-rs/cli/src/speckit_cmd.rs`
* memvid importance-threshold filtering: `codex-rs/tui/src/memvid_adapter/adapter.rs`

***

## Rollout Plan (Phased, Optional/Non-Invasive)

### Phase 0 — ADR + config readiness (now)

* Ship this ADR and a short operator-facing guide describing the three truth surfaces.
* Add/confirm feature flag defaults:
  * `product_knowledge.mode = best_effort` (attempt, never require)
  * `product_knowledge.domain = codex-product`

**Exit criteria**

* Operators understand SoR boundaries and failure semantics.

### Phase 1 — Product Knowledge lane (Tier1), best-effort

* Stage0 context compilation optionally pulls from `codex-product`.
* When used, write `ProductKnowledgeEvidencePack` (and/or retrieval events) into capsule.

**Success metrics**

* `% runs with product knowledge lane` (attempted vs succeeded)
* `evidence_pack coverage` for runs that used product knowledge (target: 100%)

### Phase 2 — NotebookLM pre-check + post-curation

* Pre-check `codex-product` before Tier2 product questions.
* Distill durable NotebookLM outputs into `codex-product` insights with relationships.

**Success metrics**

* Reduced NotebookLM calls due to pre-check hits
* Increase in reuse across specs/projects (how often `codex-product` contributes to Stage0 packs)

### Phase 3 — Quality + graph enrichment

* Enforce/automate quality gates for what gets stored (type/importance/evidence/outcome).
* Add relationship hygiene (auto-link suggestions; human confirmation).

**Success metrics**

* Curation quality: % of stored entries meeting the contract
* Graph density: average relationships per insight in `codex-product`

***

## Open Questions

1. What is the minimal degraded-mode trace format we standardize on in capsule (event vs artifact vs both)?
2. Should the evidence pack store full text always, or allow “snippet-only” in some environments?

***

## References

* `codex-rs/docs/MODEL-POLICY.md (System of Record → Memvid-First Architecture)` (capsule SoR)
* `codex-rs/SPEC.md (Invariants → System of Record)` (capsule SoR)
* `codex-rs/SPEC.md (Invariants → Replay Determinism)` (offline replay determinism)
* `codex-rs/tui/src/chatwidget/spec_kit/vision_core.rs (persist_vision_to_overlay)` (vision persisted to OverlayDb)
* `codex-rs/tui/src/chatwidget/spec_kit/commands/projections.rs` (projection rebuild design)
* `codex-rs/cli/src/speckit_cmd.rs` (projection failures must not block SoR writes)
* `docs/OPERATIONS.md (Memory Workflow quick reference)` (insight content discipline)
* `docs/OPERATIONS.md (Capsule as System of Record)` (capsule as SoR + projections)
* `codex-rs/tui/src/stage0_adapters.rs` (CLI+REST local-memory; HTTP NotebookLM)
* `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs (run_stage0_for_spec)` (best-effort health gating)
* `codex-rs/tui/src/memvid_adapter/types.rs (RetrievalRequestPayload, RetrievalResponsePayload)` (retrieval event payloads)
* `codex-rs/stage0/src/overlay_db.rs (Tier2CacheEntry)` (Tier2 cache)
* `/home/thetu/infra/localmemory-policy/skills/local-memory/SKILL.md (Non-negotiables)` (no MCP; CLI+REST)

***

## Changelog

| Date       | Change                                              |
| ---------- | --------------------------------------------------- |
| 2026-01-25 | Initial ADR for Product Knowledge layer integration |
