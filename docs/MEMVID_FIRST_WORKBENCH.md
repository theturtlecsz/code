# Memvid‑First Auditable Agent Workbench
**Architecture Proposal, Roadmap, and ADRs**  
Date: 2026-01-06  
Scope: Codex‑RS + Spec‑Kit pipeline (Rust) — Memory + Evidence substrate upgrade using Memvid capsules (.mv2 / .mv2e)

---

## 0) Executive decisions (what we’re committing to)

### Decision 1 — Capsule strategy (hybrid)
**Default:** *Workspace capsule* (one per repo/workspace) for unified recall + low operational burden.  
**Also supported:** *Per‑run export capsules* for audit handoff and reproducibility.

Why this hybrid works:
- Workspace capsule keeps retrieval simple and fast (no cross‑capsule fan‑out).
- Exporting a run capsule gives you the “share one run” story without forcing per‑run operational overhead during daily work.

**Default paths**
- Workspace capsule: `./.speckit/capsules/workspace.mv2` (or encrypted `workspace.mv2e`)
- Run export: `./docs/specs/<SPEC_ID>/runs/<RUN_ID>.mv2e`

### Decision 2 — Time‑travel boundary model
**Canonical checkpoints = workflow stage boundaries** (Specify, Plan, Tasks, Implement, Validate, Audit, Unlock) plus optional user checkpoints.

Reason: stage boundaries already exist in Spec‑Kit; using them as checkpoints makes “what did the agent know then?” deterministic and auditable.

### Decision 3 — Memvid integration approach
Memvid is **a backend adapter**, not a cross‑cutting dependency.

- Stage0 stays abstracted behind existing traits (`LocalMemoryClient`, `VectorBackend`) and/or a thin new trait that doesn’t leak Memvid types.
- Evidence capture is upgraded by adding a Memvid‑backed `EvidenceRepository` implementation (TUI/CLI), with optional filesystem mirroring during rollout.

### Decision 4 — Feature flags (“good defaults”)
**Default feature set (recommended):**
- `lex` (BM25 full‑text search)
- `vec` (vector similarity search)
- `hybrid` retrieval (lex + vec blend)
- `temporal_track` (time signals / time index)
- **optional** `encryption` (enabled when using `.mv2e`)

**Power user / optional build profiles:**
- `pdf_extract`, `docx_extract`
- `clip` (image embeddings)
- `whisper` (audio transcription)
- `pii_detect` / `masking` (if available) or app‑layer redaction (see Epic 7)

---

## 1) North Star architecture

### 1.1 Component diagram (conceptual)

```
┌──────────────────────────────────────────────────────────────────┐
│                        Codex‑RS TUI / CLI                         │
│                 (speckit.auto, status, timeline)                  │
└───────────────────────────────┬──────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────┐
│                     Spec‑Kit Executor (Rust)                      │
│  stages + gates + tool exec + artifacts                           │
│  (Specify→Plan→Tasks→Implement→Validate→Audit→Unlock)              │
└───────────────┬───────────────────────────────┬──────────────────┘
                │                               │
                │ Stage0 retrieval              │ Evidence events
                ▼                               ▼
┌───────────────────────────┐         ┌────────────────────────────┐
│          Stage0            │         │     Evidence Repository     │
│  IQO → retrieve → rerank   │         │  (FS today → Memvid + FS)   │
└───────────────┬───────────┘         └──────────────┬─────────────┘
                │                                     │
                ▼                                     ▼
        ┌────────────────────────────────────────────────────┐
        │              Storage / Memory Adapter Layer          │
        │   - MemvidMemoryAdapter (new)                        │
        │   - LegacyLocalMemoryAdapter (existing)              │
        │   - Config switch: memory_backend = memvid|legacy    │
        └─────────────────────┬──────────────────────────────┘
                              │
                              ▼
        ┌────────────────────────────────────────────────────┐
        │               Memvid Capsule (.mv2/.mv2e)           │
        │  - append-only frames                               │
        │  - embedded WAL + indexes + time index              │
        │  - hybrid search (BM25 + vector)                    │
        └────────────────────────────────────────────────────┘

Cloud sidecars (unchanged, but now better grounded):
- NotebookLM Tier2: takes *sources retrieved from capsule* → returns cited brief
- Frontier Architect/Judge models: consume cited brief + artifacts
```

### 1.2 Data flow diagram

```
(ingest)          (commit)              (index)             (retrieve)           (export)
  │                 │                     │                    │                  │
  ▼                 ▼                     ▼                    ▼                  ▼
Artifacts ──► put(bytes+meta) ──► checkpoint(stage) ──► lex+vec indexes ──► search(query,as_of) ──► run capsule (.mv2e)
   │                                                                                  │
   └──────────────────────────────────────────────────────────────────────────────────┘
                         Evidence trail includes: what was retrieved, why, and when
```

---

## 2) Capsule strategy & data model

### 2.1 Capsule granularity & lifecycle

**Workspace capsule (default)**
- **Location:** `./.speckit/capsules/workspace.mv2[e]`
- **Contains:** everything needed for retrieval + evidence across all specs/runs
- **Growth:** managed via compaction/rotation policies (see §8)

**Per‑run capsule (export-only)**
- Created via `speckit capsule export --run <RUN_ID> --encrypt`
- Includes only:
  - artifacts + evidence for that run
  - retrieval events and the retrieved source payloads (or hashes, depending on policy)
  - checkpoints timeline for that run

This keeps daily work simple while enabling shareable, reproducible “run capsules.”

### 2.2 Naming conventions

**Workspace**
- Capsule: `./.speckit/capsules/workspace.mv2` (or `workspace.mv2e`)
- Optional rotated capsules: `workspace.0001.mv2e`, `workspace.0002.mv2e`

**Run export**
- `./docs/specs/<SPEC_ID>/runs/<RUN_ID>.mv2e`
- `RUN_ID` format: `YYYYMMDD-HHMMSS-<gitshort>-<random4>`

### 2.3 Metadata tagging schema (minimum viable)

Required tags/fields (string unless noted):
- `spec_id` (e.g., `SPEC-123`)
- `run_id` (see above)
- `stage` (enum: Specify|Plan|Tasks|Implement|Validate|Audit|Unlock|Stage0)
- `role` (Architect|Implementer|Validator|Judge|SidecarCritic|Librarian|…)
- `artifact_kind` (spec_md|plan_md|tasks_md|patch|test_log|audit_verdict|task_brief|divine_truth|…)
- `path` (repo-relative path if applicable)
- `git_commit` (SHA)
- `ts` (RFC3339 timestamp)
- `content_type` (text/markdown, application/json, text/plain, application/diff, …)
- `sha256` (bytes digest)
- `redaction_state` (none|masked_view|sanitized_on_ingest)

Optional (high value):
- `model_provider`, `model_id`, `tokens_in`, `tokens_out`, `cost_usd`
- `tool_name` (cargo test, git diff, etc.), `tool_exit_code`
- `retrieval_query_id` (to tie “what was retrieved” to downstream plan/code)

### 2.4 Canonical URI scheme

A canonical, stable identifier makes citations + replay deterministic.

Example:
- `mv2://workspace/<workspace_id>/spec/<SPEC_ID>/run/<RUN_ID>/stage/<STAGE>/artifact/<KIND>/<PATH>#<FRAME_ID>`

Notes:
- `<FRAME_ID>` is the immutable Memvid frame identifier (or monotonically increasing index).
- `<workspace_id>` can be derived from the git remote + repo root hash.

---

## 3) Integration architecture

### 3.1 Keep Stage0 abstracted

Current Stage0 already isolates memory behind traits:
- `LocalMemoryClient` (CRUD + recall/ingest semantics)
- `VectorBackend` (embed + similarity search)

**Plan:**
- Implement `MemvidVectorBackend` and `MemvidLocalMemoryClient` in a new crate:
  - `codex-rs/memvid-adapter/` (new)
- Stage0 depends only on the trait; memvid adapter is enabled via a cargo feature and config.

**Config**
```toml
[memory]
backend = "memvid"   # or "legacy"
workspace_capsule = ".speckit/capsules/workspace.mv2e"
encryption = true
```

### 3.2 EvidenceRepository upgrade

Today TUI writes evidence to filesystem via an `EvidenceRepository`.

**Plan:**
- Add `MemvidEvidenceRepository` implementation that:
  - stores each evidence artifact as a frame (bytes + metadata)
  - writes an explicit checkpoint frame at each stage boundary
- During rollout, keep the filesystem repository as a mirror:
  - “write‑through”: FS + Memvid
  - “read‑prefer”: Memvid, fallback to FS

### 3.3 Dual-backend rollout (safe)

**Phase A — Shadow write (no behavior change)**
- Keep retrieval from legacy backend.
- Store all artifacts/evidence into Memvid capsule in parallel.
- Emit metrics comparing “what would Memvid have returned?” offline.

**Phase B — Dual read / A-B harness**
- Stage0 runs both retrievals.
- Compare top‑k hits, overlap, and downstream success.

**Phase C — Memvid primary**
- Switch `backend=memvid`.
- Keep fallback if capsule missing/corrupt.

---

## 4) Concurrency model

### 4.1 Single writer by default
- One spec run writes to a workspace capsule at a time.
- Enforce via an OS file lock at `workspace.mv2[e].lock`.

### 4.2 Multi-reader safe
- Reads can proceed while a run is active.
- Writes append frames + commit WAL; readers see last committed state.

### 4.3 Background indexing
- Small artifacts: index synchronously (fast path).
- Large ingestion (PDF/DOCX/audio): queue background jobs:
  - extract → chunk → embed → add frames → checkpoint

---

## 5) Failure model & recovery

### 5.1 Crash recovery expectations
- A crash during ingestion should not corrupt prior committed state.
- On restart:
  - open capsule
  - replay/repair via embedded WAL
  - resume from last durable checkpoint

### 5.2 Corrupted capsule handling
- If open fails integrity checks:
  - mark capsule “unhealthy”
  - automatically fall back to legacy backend
  - offer recovery UX:
    - `speckit capsule doctor --repair`
    - `speckit capsule doctor --salvage --new <path>`

### 5.3 Partial commits
- Checkpoints are the atomic unit of time-travel.
- If a stage fails mid-way, write a `checkpoint_failed` frame:
  - preserves audit trail without pretending success

---

## 6) Security posture

### 6.1 Encryption UX
- Support `.mv2e` encrypted capsules.
- Key / password sources (priority order):
  1. `SPECKIT_CAPSULE_PASSWORD` env var (CI-friendly)
  2. interactive prompt in TUI (no echo)
  3. optional OS keychain integration (later milestone)

### 6.2 Safe export mode (application-layer)
Even with encryption, “safe sharing” needs additional controls.

Export modes:
- `--export raw` (default for internal use)
- `--export safe` (recommended for sharing)
  - redact secrets/PII in rendered views
  - optionally strip raw payloads and include hashes only (policy-driven)
  - include an export manifest describing what was included/excluded

Audit logging:
- exporting/importing capsules emits an evidence event frame:
  - who, when, what mode, destination, checksum

### 6.3 Redaction pipeline (two modes)
- **Mask-on-view (default):** raw data stored, UI/export masking applied
- **Sanitize-on-ingest (high-security):** secrets stripped before writing capsule

---

## 7) Time‑travel UX design (TUI/CLI)

### 7.1 Timeline view
Command:
- `speckit timeline` (workspace)
- `speckit timeline --spec <SPEC_ID> --run <RUN_ID>`

Shows:
- checkpoints (stage boundary, timestamp, git SHA)
- key artifacts emitted at each checkpoint

### 7.2 As-of query
- `speckit recall --as-of <CHECKPOINT_ID> "query"`
- TUI: “search at checkpoint…” prompt

### 7.3 Diff between checkpoints
- `speckit diff --from <CKPT_A> --to <CKPT_B> [--kinds plan_md,patch,test_log]`
- Renders:
  - artifact diffs (text)
  - retrieval diffs (what sources changed)
  - summary of stage outputs

### 7.4 Branching (optional)
If Memvid has native branching, use it.  
If not, implement:
- `speckit capsule branch --from <CKPT> --to <new_capsule>`
- metadata links: `branch_parent_capsule`, `branch_parent_checkpoint`

---

## 8) Retrieval quality & evaluation harness

### 8.1 Golden queries
Maintain a repo-local suite:
- `./.speckit/eval/golden_queries.yaml`
- Each entry:
  - query
  - expected top hits (URIs)
  - filters
  - as_of checkpoint (optional)

### 8.2 A/B compare legacy vs memvid
- Run Stage0 retrieval with both backends on the same corpus
- Metrics:
  - Top‑k overlap
  - Recall@k against golden hits
  - Downstream impact: plan quality (Judge pass rate), retries, time-to-fix

### 8.3 “Explain retrieval”
Return structured “why this result”:
- lexical score
- vector score
- recency boost
- tag match
- final blended score

---

## 9) Performance posture

Targets (initial, adjustable):
- Retrieval latency (top‑k hybrid): **< 50 ms** for typical workspaces
- Stage checkpoint commit: **< 20 ms** for small artifacts
- Capsule open: **< 200 ms** for typical size

Caching strategy:
- cache recent queries + top hits per run
- cache “hot prefixes” (stage0 task brief sources) for NotebookLM synthesis

Capsule growth management:
- periodic compaction (manual + optional scheduled)
- retention policies (e.g., keep last N runs’ raw logs; keep hashes for older)

---

## 10) Roadmap (epics → milestones → acceptance)

### Epic 1 — Foundation: Memvid backend + capsule conventions
**Milestone 1.1 — Capsule conventions + init**
- Deliver:
  - directory layout
  - workspace capsule init command
  - metadata schema + URI conventions doc
- Done when:
  - `speckit capsule init` creates + opens capsule
  - CI test: create → reopen → integrity check

**Milestone 1.2 — Memvid adapter for Stage0 traits**
- Deliver:
  - `MemvidVectorBackend` (embed + search)
  - `MemvidLocalMemoryClient` (ingest + recall + list)
- Done when:
  - Stage0 can ingest 10 markdown artifacts into capsule and retrieve relevant ones

**Milestone 1.3 — Evidence write-through**
- Deliver:
  - `MemvidEvidenceRepository`
  - config switch + fallback
- Done when:
  - one spec run produces artifacts that are queryable from the capsule
  - CI: run → reopen → search → verify URIs resolve

### Epic 2 — Hybrid retrieval + relevance controls
**Milestone 2.1 — Hybrid scoring + filters**
- Deliver:
  - lex + vec blended scorer
  - tag + path filters, top_k control
- Done when:
  - retrieval quality meets baseline and explain output matches expectations

**Milestone 2.2 — Explainability + debug UI**
- Deliver:
  - `--explain` mode for recall
  - TUI panel for “why this hit”
- Done when:
  - at least one workflow uses explain to tune recall without code changes

### Epic 3 — Time‑travel UX
**Milestone 3.1 — Checkpoints**
- Deliver:
  - stage boundary checkpoints written automatically
  - checkpoint listing API
- Done when:
  - “what did we know at Plan?” is reproducible

**Milestone 3.2 — As-of + diff**
- Deliver:
  - as-of recall
  - diff between checkpoints
- Done when:
  - auditor can reproduce “inputs to Architect” at any stage boundary

### Epic 4 — Export/import + encryption UX
**Milestone 4.1 — Export/import CLI**
- Deliver:
  - export run capsule
  - import capsule into workspace (read-only)
- Done when:
  - second machine can reproduce retrieval context offline

**Milestone 4.2 — Encrypted capsule UX**
- Deliver:
  - password prompts/env var support
  - `.mv2e` path support
- Done when:
  - encrypted export/import works end-to-end

### Epic 5 — Multi-modal ingestion (optional)
**Milestone 5.1 — PDF/DOCX ingestion**
- Deliver:
  - pipeline to extract → chunk → index
  - feature-gated build
- Done when:
  - PDFs & docs are searchable and time-travelable

### Epic 6 — Replayable audits (on top of capsule)
**Milestone 6.1 — Record retrieval events**
- Deliver:
  - structured event frames: query, filters, top hits, selected context
- Done when:
  - run replay reproduces “retrieval result set” at checkpoint

**Milestone 6.2 — Deterministic replay report**
- Deliver:
  - replay command that re-runs retrieval and compares output
- Done when:
  - generates a diff + summary report suitable for audit

### Epic 7 — Redaction + policy hardening
**Milestone 7.1 — Mask-on-view**
- Deliver:
  - secret/PII masking in TUI and export manifests
- Done when:
  - “safe export” prevents accidental leak in default views

**Milestone 7.2 — Sanitize-on-ingest (optional)**
- Deliver:
  - configurable sanitizers
- Done when:
  - high-security mode never persists secrets to capsule

---

## 11) ADRs (drafts)

### ADR-001: Capsule granularity & lifecycle
**Decision:** Workspace capsule default + per-run export capsules.  
**Alternatives:** per-spec capsules, per-run capsules only.  
**Why:** unified recall, low ops overhead, still supports shareable runs.

### ADR-002: Default Memvid feature flags
**Decision:** lex+vec+temporal_track on; encryption optional; multimodal gated.  
**Why:** strongest “good default” without build bloat.

### ADR-003: Migration strategy
**Decision:** shadow-write → dual-read A/B → memvid primary with fallback.  
**Why:** zero-downtime rollout; measurable quality parity before switching.

### ADR-004: Time-travel boundary representation
**Decision:** stage checkpoint frames are canonical; timestamps are secondary.  
**Why:** deterministic audit tied to workflow semantics.

---

## 12) Thin end-to-end prototype (E2E smoke test)

Goal: prove capsule lifecycle + time travel before broad integration.

**Prototype CLI (new, minimal)**
- `speckit capsule init`
- `speckit capsule ingest --spec SPEC-123 --stage Specify --path docs/specs/SPEC-123/spec.md`
- `speckit capsule checkpoint --spec SPEC-123 --run <RUN_ID> --stage Specify`
- `speckit capsule search "auth token refresh" --spec SPEC-123 --top-k 5`
- `speckit capsule search "auth token refresh" --as-of <CHECKPOINT_ID>`

**Acceptance**
- Capsule can be reopened after process restart.
- Search results are stable “as of” a checkpoint.
- Exported run capsule reproduces the same top‑k results on another machine.

---

## 13) Open questions (to finalize before coding)
1. Should workspace capsules be encrypted by default (`.mv2e`) or opt-in?
2. Do we want *raw bytes* of tool outputs (test logs) retained forever, or rotated/hashed after N days?
3. What is the minimum set of artifact kinds that must be in capsule to satisfy audits?
4. Any compliance constraints that require sanitize-on-ingest to be default?

---

## 14) References (informational)
- Memvid docs (architecture, WAL, frames, time travel, encryption)
- Current Stage0 traits (`LocalMemoryClient`, `VectorBackend`) in `codex-rs/stage0`
- Current evidence repository in `codex-rs/tui`
