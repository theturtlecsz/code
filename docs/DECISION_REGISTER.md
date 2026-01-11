# Decision Register v0.12 (Memvid + Model Policy)

- **Date:** 2026-01-09
- **Scope:** Codex-RS / Spec-Kit — Memvid-first workbench + model policy + evaluation + governance.
- **Status:** Locked decisions D1–D112 (with mapping pending where noted).

---

## Locked Decisions

| ID | Choice | Decision | Notes |
|---:|:------:|---|---|
| D1 | A | Capsule granularity default | Workspace capsule default + per-run export capsules. |
| D2 | C | Capsule placement | Split roots: workspace in `./.speckit/…`, run exports under `./docs/specs/<SPEC_ID>/runs/…`. |
| D3 | B | Time-travel boundary identity | Stage checkpoints canonical + timestamps as convenience. |
| D4 | B | Commit cadence | Commit at stage boundaries + manual “commit now”. |
| D5 | C | Retrieval strategy | Hybrid fusion (lex + vec) with explainability required. |
| D6 | A | Indexing cadence | Background indexing + stage-boundary “index barrier” for determinism. |
| D7 | A | Concurrency model | Single-writer capsule (global lock + queued writes). |
| D8 | B | Encryption posture | Optional: encrypt exports by default; workspace encryption is a config flag. |
| D9 | B + C | Redaction strategy | Safe-export pipeline + sanitize-on-ingest available (opt-in); UI masking always on. |
| D10 | B | Structured extraction | Start with structured Memory Cards for key domains; grow to full logic mesh. |
| D11 | ALL | Multi-modal ingestion scope | Support text+PDF+DOCX+images+audio, but ship behind feature gates/profiles. |
| D12 | C | Policy versioning & audit | Policy authored in repo/config; runtime PolicySnapshot embedded in capsule per run. |
| D13 | B (now) | Local inference server backend | SGLang now (radix attention + schema decode); fallback to vLLM allowed if unstable. |
| D14 | C | Local-memory migration posture | Phased migration: dual backend until parity; then remove legacy. |
| D15 | B | Prompt/output storage in capsule | Store structured summaries + hashes by default; full content optional. |
| D16 | B | Run-capsule export automation | Export for high-risk specs or on request (not every run). |
| D17 | B | Policy approval/signing | Soft enforcement now (warn if unsigned); harden later. |
| D18 | B | Checkpoint granularity | Stage checkpoints + manual user commits. |
| D19 | A | Multi-capsule querying | Support querying workspace + imported/exported capsules via aggregator. |
| D20 | A | Capsule growth management | Retention/compaction/rotation with size caps. |
| D21 | A | Retrieval caching | Cache prefill + query results for latency; must be replay-safe. |
| D22 | C | Evidence capture | Evidence events and artifacts stored in capsule + keep existing directory tree during rollout. |
| D23 | B | “Safe export” | Mandatory safe-export mode with audit log of exports. |
| D24 | C | Embeddings | BGE‑M3 (or equivalent) + Memvid vector index; keep model replaceable. |
| D25 | C | Implementer escalation | Local reflex first; escalate to cloud coder after 2 failed attempts. |
| D26 | B | Architect low-confidence behavior | Re-plan with stronger model; escalate to human if still low confidence. |
| D27 | B | Judge posture | Judge always cloud (no local judge). |
| D28 | B | SidecarCritic posture | Always-on by default. |
| D29 | A | NotebookLM Tier2 | Always-on, but fed by local retrieval/evidence first; timeout + graceful fallback. |
| D30 | B | Budget enforcement | Warn at 80%, hard stop at 100% with explicit override flag. |
| D31 | A | Graph/Logic Mesh | Full graph/logic mesh enabled (cards + graph + temporal), not just blobs. |
| D32 | A | Branching | Branching supported (capsule branch/clone semantics). |
| D33 | B | Replayable audits | Retrieval replay by default; optional LLM re-run + diff when requested/high-risk. |
| D34 | C | PII posture | UI masking + safe export always; sanitize-on-ingest opt-in high-security mode. |
| D35 | B | Retrieval evaluation harness | Golden queries + A/B + stress tests; gate regressions in CI. |
| D36 | C | Policy testing | Policy tests (unit+integration+simulation) required for policy changes. |
| D37 | A | Feature-flag strategy | “Good default” features on; power features behind cargo features/build profiles. |
| D38 | A | Operational footprint | Prefer single-binary, no-daemon design; daemons only as optional legacy. |
| D39 | A | Legacy deprecation | Remove `local-memory` backend after Memvid parity + reliability gates pass. |
| D40 | A | Migration tooling | Provide one-time import/migrate tool; keep legacy data until verified. |
| D41 | A | Default cloud model: Architect | OpenAI “best reasoning” (e.g., GPT‑5.2 High/XHigh) as default; configurable in TUI. |
| D42 | A | Default cloud model: Judge | OpenAI “best reasoning” (e.g., GPT‑5.2 XHigh) as default; configurable in TUI. |
| D43 | A | Standard local reflex model | GPT‑OSS‑20B (MXFP4) as the always-on reflex model (served by SGLang). |
| D44 | B | Model configuration UX | TUI provides a Model Policy panel to edit role→model + thresholds; writes config + stores snapshot in capsule. |
| D45 | A | Git posture for workspace capsule (`workspace.mv2`) | Do NOT commit workspace capsule; keep in .gitignore. |
| D46 | A | Encryption default for exports (`run.mv2e`) | Do NOT commit per-run export capsules by default; allow opt-in for audit repos. |
| D47 | B | Code capture granularity for time-travel & replay | Compression ON by default (capsule). |
| D48 | B | Large/volatile artifact exclusion (growth control) | Compaction/rotation on demand + size thresholds. |
| D49 | B | Embedding compute placement (default) | Embeddings computed on CPU by default; GPU optional. |
| D50 | B | Graph/Logic Mesh update cadence | GPU embeddings opportunistic; do not contend with local reflex model. |
| D51 | B | Replay audit as an Unlock gate | Pin embedding model per capsule; provide re-embed/migration tooling. |
| D52 | B | Legacy deprecation posture (evidence tree + local-memory daemon) | Dual-write + parity harness; remove legacy after gates pass. |
| D53 | C | Canonical source of evidence | Dual-canonical evidence during migration; converge to capsule-canonical later. |
| D54 | B | Encrypted capsule key UX | Prompt for password on first use; optional OS keychain caching. |
| D55 | A | Structured output enforcement for tool calls (SpecKit JSON) | Generation-time JSON/schema constraints where possible (SGLang/server-side). |
| D56 | B | Policy enforcement mode on policy load/parse failure | Fail-closed for high-risk; warn+fallback for low-risk if policy load fails. |
| D57 | B | Policy override hierarchy | Overrides: workspace + per-spec layers (no per-run overrides by default). |
| D58 | A | LLM selection for graph extraction & replay summarization | Graph extraction & replay summarization: local reflex first; escalate to cloud as needed. |
| D59 | C | Observability sink (local + optional enterprise) | Logs: local logs + optional OpenTelemetry (OTLP) export. |
| D60 | B | Capsule remote sync strategy (beyond export/import) | Remote sync supported via optional sync adapter trait; off by default. |
| D61 | A | Branching semantics (how a branch is represented) | Branching uses native Memvid branch pointers inside one capsule (validate support). |
| D62 | B | Graph ontology scope (what goes into the logic mesh v1) | Graph ontology v1 is Spec-Kit native: requirements, tasks, risks, deps, tests, etc. |
| D63 | B | Graph extraction method (how we populate the mesh) | Graph extraction is LLM-first on most ingests (higher coverage, higher variability). |
| D64 | A | When graph updates run | Graph updates computed at stage-boundary commits (checkpoint aligned). |
| D65 | C | Replay audit depth (what gets replayed) | Replay audits are full end-to-end including model calls (with deterministic+exact modes). |
| D66 | B | Reproducibility target | Primary reproducibility target: deterministic retrieval context at a checkpoint. |
| D67 | B | Default workspace capsule size cap (before compaction/rotation) | Capsule size cap: 5GB (warn + recommend compaction/rotation). |
| D68 | A | Should NotebookLM “Divine Truth” be ingested back into the capsule? | NotebookLM outputs ingested into capsule (tagged as derived + citation required). |
| D69 | C | Embedding upgrades inside a capsule | Multi-track embeddings: keep old track for replay; add new track for future; preserve comparability. |
| D70 | C | Capsule format migration strategy | Explicit `memvid migrate` command with backup + verification; no silent auto-migration. |
| D71 | B | Git posture for capsules (refined) | Workspace capsule remains gitignored; share via explicit exports (optionally stored in audit repos). |
| D72 | B | Replay default mode (refined) | Exact replay by default: record enough to replay without re-calling models; re-run models only when requested. |
| D73 | A | Branch lifecycle default | Always create a branch per run; merge on success; keep main branch clean. |
| D74 | B | Branch merge policy | Merge curated artifacts + graph state deltas by default; keep full-fidelity merge as an escape hatch. |
| D75 | B | Graph conflict handling | Multi-valued facts + confidence + provenance; surface conflicts in UI; no silent overwrite. |
| D76 | B | Automatic Replay Audit triggers | Auto-run replay audits for high-risk specs (policy decides); manual override available. |
| D77 | C | Graph extraction model choice | Cloud default + local fallback + “no-extract” mode (policy-controlled). |
| D78 | C | Single-GPU scheduling priority | Stage-based scheduling: prioritize reflex during Implement; allow embeddings/extraction during Plan/Audit/idle. |
| D79 | B | Deletion / purge expectations | Tombstone + encrypted shredding of sensitive frames; true purge via compaction later if required. |
| D80 | C | Multi-capsule query support (advanced) | Build a meta-index capsule for cross-capsule search; supports mounted read-only capsules + discovery. |

## Addendum: D81–D94 (Answers received, mapping pending)

> **Note:** You replied `81B 82B 83B` and later `84A 85B 86B 87B 88C 89B 90B 91C 92B 93B 94B`, but the exact question text for D81–D94 wasn’t present in the last captured prompt list.
> I’m tentatively mapping D81–D94 to the most likely “next decisions” (dependency control, policy enforcement strictness, eval gating, embeddings/retrieval ops, and failure behaviors).
> If any mismatch, reply `D## mismatch` with the intended question text (one line is enough) and I’ll re-slot immediately.

| ID | Choice | Decision | Notes |
| --- | --- | --- | --- |

| D81 | B | Memvid dependency strategy (pinning/forking) | **Assumption mapping.** Pin Memvid tags + commit SHA + wrap behind adapter crate; keep a “hotfix fork” option. |
| D82 | B | Policy change enforcement strictness (signed approval) | **Assumption mapping.** Start soft-enforce now (warn + log + require evidence), hard-enforce later after harness is stable. |
| D83 | B | Evaluation gating schedule (CI vs nightly) | **Assumption mapping.** CI: smoke + golden queries; Nightly: full suite; allow manual override w/ evidence capsule + signed approval. |

| D84 | A | Embedding compute location for Memvid vector index | **Assumption mapping.** Local embeddings by default (offline-first, no external embedding API dependency). |
| D85 | B | Default embedding model family | **Assumption mapping.** Use a strong open-weights embedding model (configurable) with a code-friendly default; keep cloud-embedding optional. |
| D86 | B | Embedding serving method | **Assumption mapping.** Local embedding service (OpenAI-compatible or native SDK) behind a trait; avoid hard-binding to one server. |
| D87 | B | Chunking strategy for code/docs | **Assumption mapping.** Structure-aware chunking (Rust syntax-aware for code + section-aware for docs) + stable chunk IDs for dedup/replay. |
| D88 | C | Vector index compression posture | **Assumption mapping.** Enable PQ/other compression automatically above a size threshold to control capsule growth and query latency. |
| D89 | B | Recency/usage bias in ranking | **Assumption mapping.** Simple recency boost + provenance weighting + “why this result” explain; no learned ranker yet. |
| D90 | B | Retrieval quality evaluation harness depth | **Assumption mapping.** Golden queries + A/B harness (local-memory vs memvid) + stress tests; deeper eval later. |
| D91 | C | Policy/eval gating strictness | **Assumption mapping.** Hard-gate on regressions for golden queries + policy tests; override requires evidence + approval. |
| D92 | B | NotebookLM Tier2 failure behavior | **Assumption mapping.** Always-on, but never blocking: degrade to Tier1/local synthesis + log + evidence. |
| D93 | B | Local reflex failure behavior | **Assumption mapping.** Auto-fallback to cloud coder after N errors/timeouts; record event in evidence + keep pipeline moving. |
| D94 | B | Canonical source-of-truth during migration | **Assumption mapping.** Dual-canonical during rollout; after parity gate, capsule becomes canonical and filesystem becomes a derived/export view. |
| D95 | C | Replay capture scope | **Tentative mapping.** A) minimal replay logs; B) high-risk only; C) always-on full event stream | Always capture retrieval + tool calls + prompts/outputs + gate decisions in capsule (redaction/retention apply). |
| D96 | B | Branching UX exposure | **Tentative mapping.** A) internal only; B) lightweight “what-if” branches in TUI; C) full DVCS-like branches/merges | Expose branch create/switch + compare; implement as capsule branch pointers/copies; avoid complex merges in v1. |
| D97 | B | Logic mesh extraction cadence | **Tentative mapping.** A) on-demand; B) stage-boundary; C) continuous streaming | Extract Memory Cards/graph at stage commits (and key ingests) so time-travel checkpoints have deterministic graph state. |
| D98 | B | Graph + replay surfaces | **Tentative mapping.** A) internal only; B) TUI/CLI commands; C) public API | Add /speckit.timeline, /speckit.diff, /speckit.replay, /speckit.graph as first-class workbench features. |

---

## Addendum: D99–D109 (Answers received — mapping proposed)

> Note: If any of these labels don’t match what you intended for that decision number, reply “D### mismatch: <correct question>”.

| Decision | Your choice | Proposed label | What this locks |
|---|---:|---|---|
| D99 | A | Run capsule reproducibility package includes code state | Each exported run capsule captures enough code-state to reproduce offline (git commit + diff + key file snapshots). |
| D100 | B | Code snapshot granularity = changed-files + diff (not full repo) | Keep exports sane-size: store changed file contents + hashes + diff; don’t embed full repo snapshot by default. |
| D101 | B | Tool execution capture = raw logs (compressed) + structured summary | Replay can re-run or at least inspect exact build/test outputs; summaries stay fast for UI. |
| D102 | B | Tier2 source capture = query + answer + citations + excerpted sources | NotebookLM/Grounded synth is replayable/auditable without ballooning capsule size. |
| D103 | A | Imported capsules are mounted read-only by default | We can attach audit/customer capsules without mutating workspace memory; avoids merge corruption. |
| D104 | A | Auto-register mounted capsules into meta-index | Mounting/import is immediately discoverable via workspace search and timeline UX. |
| D105 | B | Multi-modal ingestion implemented as pluggable ingestors | Keep core build lean; enable PDF/DOCX/images/audio via feature flags/plugins. |
| D106 | B | Embedding/extraction compute isolated from reflex inference | Avoid GPU contention; run embeddings/extraction in separate worker/process to keep reflex latency stable. |
| D107 | A | Replay audit outputs include both Markdown + JSON | Human-readable report + machine-verifiable artifact, stored in capsule and exportable. |
| D108 | A | Replay supports A/B diffs across models and policies | Run deterministic replay comparisons to validate policy/model changes and generate diffs. |
| D109 | B | Policy rollout strategy = staged/feature-flag rollouts | New policies ship behind flags/canaries; reduces blast radius, supports rollback. |
| D110 | A | Local reflex **backup** model = Qwen3-Coder-30B-A3B (AWQ/GPTQ) | Clarification: Primary always-on reflex remains **GPT-OSS-20B** (see D43). Standardize Qwen3-A3B as the fallback/bakeoff candidate when GPT-OSS underperforms on repo-specific Rust/code. |
| D111 | B | Local inference server default = Option B (SGLang now; vLLM fallback) | We implement SGLang first for radix/prefix caching + schema decoding; keep vLLM as fallback for stability. (Assumption: mapping) |
| D112 | B | Reflex promotion gate = Option B (Bakeoff required) | Local reflex routing is enabled only after Rust-Reflex-Bench passes TTFT/TPS + JSON validity + cargo check metrics. (Assumption: mapping) |
