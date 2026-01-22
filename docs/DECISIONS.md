# Decisions Register

**Version**: 1.0.0 (2026-01-22)
**Status**: 134 locked decisions (D1-D134)
**Scope**: Codex-RS / Spec-Kit

***

## Table of Contents

* [1. Core Capsule Decisions (D1-D20)](#1-core-capsule-decisions-d1-d20)
* [2. Retrieval & Storage (D21-D40)](#2-retrieval--storage-d21-d40)
* [3. Model Policy (D41-D60)](#3-model-policy-d41-d60)
* [4. Branching & Graph (D61-D80)](#4-branching--graph-d61-d80)
* [5. Memvid Integration (D81-D98)](#5-memvid-integration-d81-d98)
* [6. Reproducibility & Replay (D99-D112)](#6-reproducibility--replay-d99-d112)
* [7. ARB Pass 2 (D113-D134)](#7-arb-pass-2-d113-d134)
  * [Product & Parity (A1-A2)](#product--parity-a1-a2)
  * [Evidence Store (B1-B2)](#evidence-store-b1-b2)
  * [Capture Mode (C1-C2)](#capture-mode-c1-c2)
  * [Pipeline & Gates (D1-D2, E1-E2)](#pipeline--gates-d1-d2-e1-e2)
  * [Maintenance (F1-F2)](#maintenance-f1-f2)
  * [ACE + Maieutics (H0-H7)](#ace--maieutics-h0-h7)
* [Change History](#change-history)

## 1. Core Capsule Decisions (D1-D20)

|  ID |  Choice | Decision                         | Notes                                                                                            |
| --: | :-----: | -------------------------------- | ------------------------------------------------------------------------------------------------ |
|  D1 |    A    | Capsule granularity default      | Workspace capsule default + per-run export capsules.                                             |
|  D2 |    C    | Capsule placement                | Split roots: workspace in `./.speckit/...`, run exports under `./docs/specs/<SPEC_ID>/runs/...`. |
|  D3 |    B    | Time-travel boundary identity    | Stage checkpoints canonical + timestamps as convenience.                                         |
|  D4 |    B    | Commit cadence                   | Commit at stage boundaries + manual "commit now".                                                |
|  D5 |    C    | Retrieval strategy               | Hybrid fusion (lex + vec) with explainability required.                                          |
|  D6 |    A    | Indexing cadence                 | Background indexing + stage-boundary "index barrier" for determinism.                            |
|  D7 |    A    | Concurrency model                | Single-writer capsule (global lock + queued writes).                                             |
|  D8 |    B    | Encryption posture               | Optional: encrypt exports by default; workspace encryption is a config flag.                     |
|  D9 |  B + C  | Redaction strategy               | Safe-export pipeline + sanitize-on-ingest available (opt-in); UI masking always on.              |
| D10 |    B    | Structured extraction            | Start with structured Memory Cards for key domains; grow to full logic mesh.                     |
| D11 |   ALL   | Multi-modal ingestion scope      | Support text+PDF+DOCX+images+audio, but ship behind feature gates/profiles.                      |
| D12 |    C    | Policy versioning & audit        | Policy authored in repo/config; runtime PolicySnapshot embedded in capsule per run.              |
| D13 | B (now) | Local inference server backend   | SGLang now (radix attention + schema decode); fallback to vLLM allowed if unstable.              |
| D14 |    C    | Local-memory migration posture   | Phased migration: dual backend until parity; then remove legacy.                                 |
| D15 |    B    | Prompt/output storage in capsule | Store structured summaries + hashes by default; full content optional.                           |
| D16 |    B    | Run-capsule export automation    | Export for high-risk specs or on request (not every run).                                        |
| D17 |    B    | Policy approval/signing          | Soft enforcement now (warn if unsigned); harden later.                                           |
| D18 |    B    | Checkpoint granularity           | Stage checkpoints + manual user commits.                                                         |
| D19 |    A    | Multi-capsule querying           | Support querying workspace + imported/exported capsules via aggregator.                          |
| D20 |    A    | Capsule growth management        | Retention/compaction/rotation with size caps.                                                    |

***

## 2. Retrieval & Storage (D21-D40)

|  ID | Choice | Decision                          | Notes                                                                                          |
| --: | :----: | --------------------------------- | ---------------------------------------------------------------------------------------------- |
| D21 |    A   | Retrieval caching                 | Cache prefill + query results for latency; must be replay-safe.                                |
| D22 |    C   | Evidence capture                  | Evidence events and artifacts stored in capsule + keep existing directory tree during rollout. |
| D23 |    B   | "Safe export"                     | Mandatory safe-export mode with audit log of exports.                                          |
| D24 |    C   | Embeddings                        | BGE-M3 (or equivalent) + Memvid vector index; keep model replaceable.                          |
| D25 |    C   | Implementer escalation            | Local reflex first; escalate to cloud coder after 2 failed attempts.                           |
| D26 |    B   | Architect low-confidence behavior | Re-plan with stronger model; escalate to human if still low confidence.                        |
| D27 |    B   | Judge posture                     | Judge always cloud (no local judge).                                                           |
| D28 |    B   | SidecarCritic posture             | Always-on by default.                                                                          |
| D29 |    A   | NotebookLM Tier2                  | Always-on, but fed by local retrieval/evidence first; timeout + graceful fallback.             |
| D30 |    B   | Budget enforcement                | Warn at 80%, hard stop at 100% with explicit override flag.                                    |
| D31 |    A   | Graph/Logic Mesh                  | Full graph/logic mesh enabled (cards + graph + temporal), not just blobs.                      |
| D32 |    A   | Branching                         | Branching supported (capsule branch/clone semantics).                                          |
| D33 |    B   | Replayable audits                 | Retrieval replay by default; optional LLM re-run + diff when requested/high-risk.              |
| D34 |    C   | PII posture                       | UI masking + safe export always; sanitize-on-ingest opt-in high-security mode.                 |
| D35 |    B   | Retrieval evaluation harness      | Golden queries + A/B + stress tests; gate regressions in CI.                                   |
| D36 |    C   | Policy testing                    | Policy tests (unit+integration+simulation) required for policy changes.                        |
| D37 |    A   | Feature-flag strategy             | "Good default" features on; power features behind cargo features/build profiles.               |
| D38 |    A   | Operational footprint             | Prefer single-binary, no-daemon design; daemons only as optional legacy.                       |
| D39 |    A   | Legacy deprecation                | Remove `local-memory` backend after Memvid parity + reliability gates pass.                    |
| D40 |    A   | Migration tooling                 | Provide one-time import/migrate tool; keep legacy data until verified.                         |

***

## 3. Model Policy (D41-D60)

|  ID | Choice | Decision                              | Notes                                                                                                           |
| --: | :----: | ------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| D41 |    A   | Default cloud model: Architect        | OpenAI "best reasoning" (e.g., GPT-5.2 High/XHigh) as default; configurable in TUI.                             |
| D42 |    A   | Default cloud model: Judge            | OpenAI "best reasoning" (e.g., GPT-5.2 XHigh) as default; configurable in TUI.                                  |
| D43 |    A   | Standard local reflex model           | GPT-OSS-20B (MXFP4) as the always-on reflex model (served by SGLang).                                           |
| D44 |    B   | Model configuration UX                | TUI provides a Model Policy panel to edit role->model + thresholds; writes config + stores snapshot in capsule. |
| D45 |    A   | Git posture for workspace capsule     | Do NOT commit workspace capsule; keep in .gitignore.                                                            |
| D46 |    A   | Encryption default for exports        | Do NOT commit per-run export capsules by default; allow opt-in for audit repos.                                 |
| D47 |    B   | Code capture granularity              | Compression ON by default (capsule).                                                                            |
| D48 |    B   | Large/volatile artifact exclusion     | Compaction/rotation on demand + size thresholds.                                                                |
| D49 |    B   | Embedding compute placement (default) | Embeddings computed on CPU by default; GPU optional.                                                            |
| D50 |    B   | Graph/Logic Mesh update cadence       | GPU embeddings opportunistic; do not contend with local reflex model.                                           |
| D51 |    B   | Replay audit as an Unlock gate        | Pin embedding model per capsule; provide re-embed/migration tooling.                                            |
| D52 |    B   | Legacy deprecation posture            | Dual-write + parity harness; remove legacy after gates pass.                                                    |
| D53 |    C   | Canonical source of evidence          | Dual-canonical evidence during migration; converge to capsule-canonical later.                                  |
| D54 |    B   | Encrypted capsule key UX              | Prompt for password on first use; optional OS keychain caching.                                                 |
| D55 |    A   | Structured output enforcement         | Generation-time JSON/schema constraints where possible (SGLang/server-side).                                    |
| D56 |    B   | Policy enforcement mode on failure    | Fail-closed for high-risk; warn+fallback for low-risk if policy load fails.                                     |
| D57 |    B   | Policy override hierarchy             | Overrides: workspace + per-spec layers (no per-run overrides by default).                                       |
| D58 |    A   | LLM selection for graph extraction    | Graph extraction & replay summarization: local reflex first; escalate to cloud as needed.                       |
| D59 |    C   | Observability sink                    | Logs: local logs + optional OpenTelemetry (OTLP) export.                                                        |
| D60 |    B   | Capsule remote sync strategy          | Remote sync supported via optional sync adapter trait; off by default.                                          |

***

## 4. Branching & Graph (D61-D80)

|  ID | Choice | Decision                            | Notes                                                                                                           |
| --: | :----: | ----------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| D61 |    A   | Branching semantics                 | Branching uses native Memvid branch pointers inside one capsule (validate support).                             |
| D62 |    B   | Graph ontology scope                | Graph ontology v1 is Spec-Kit native: requirements, tasks, risks, deps, tests, etc.                             |
| D63 |    B   | Graph extraction method             | Graph extraction is LLM-first on most ingests (higher coverage, higher variability).                            |
| D64 |    A   | When graph updates run              | Graph updates computed at stage-boundary commits (checkpoint aligned).                                          |
| D65 |    C   | Replay audit depth                  | Replay audits are full end-to-end including model calls (with deterministic+exact modes).                       |
| D66 |    B   | Reproducibility target              | Primary reproducibility target: deterministic retrieval context at a checkpoint.                                |
| D67 |    B   | Default workspace capsule size cap  | Capsule size cap: 5GB (warn + recommend compaction/rotation).                                                   |
| D68 |    A   | NotebookLM output ingestion         | NotebookLM outputs ingested into capsule (tagged as derived + citation required).                               |
| D69 |    C   | Embedding upgrades inside a capsule | Multi-track embeddings: keep old track for replay; add new track for future; preserve comparability.            |
| D70 |    C   | Capsule format migration strategy   | Explicit `memvid migrate` command with backup + verification; no silent auto-migration.                         |
| D71 |    B   | Git posture for capsules (refined)  | Workspace capsule remains gitignored; share via explicit exports (optionally stored in audit repos).            |
| D72 |    B   | Replay default mode (refined)       | Exact replay by default: record enough to replay without re-calling models; re-run models only when requested.  |
| D73 |    A   | Branch lifecycle default            | Always create a branch per run; merge on success; keep main branch clean.                                       |
| D74 |    B   | Branch merge policy                 | Merge curated artifacts + graph state deltas by default; keep full-fidelity merge as an escape hatch.           |
| D75 |    B   | Graph conflict handling             | Multi-valued facts + confidence + provenance; surface conflicts in UI; no silent overwrite.                     |
| D76 |    B   | Automatic Replay Audit triggers     | Auto-run replay audits for high-risk specs (policy decides); manual override available.                         |
| D77 |    C   | Graph extraction model choice       | Cloud default + local fallback + "no-extract" mode (policy-controlled).                                         |
| D78 |    C   | Single-GPU scheduling priority      | Stage-based scheduling: prioritize reflex during Implement; allow embeddings/extraction during Plan/Audit/idle. |
| D79 |    B   | Deletion / purge expectations       | Tombstone + encrypted shredding of sensitive frames; true purge via compaction later if required.               |
| D80 |    C   | Multi-capsule query support         | Build a meta-index capsule for cross-capsule search; supports mounted read-only capsules + discovery.           |

***

## 5. Memvid Integration (D81-D98)

|  ID | Choice | Decision                                   | Notes                                                                                                                     |
| --: | :----: | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| D81 |    B   | Memvid dependency strategy                 | Pin Memvid tags + commit SHA + wrap behind adapter crate; keep a "hotfix fork" option.                                    |
| D82 |    B   | Policy change enforcement strictness       | Start soft-enforce now (warn + log + require evidence), hard-enforce later after harness is stable.                       |
| D83 |    B   | Evaluation gating schedule                 | CI: smoke + golden queries; Nightly: full suite; allow manual override w/ evidence capsule + signed approval.             |
| D84 |    A   | Embedding compute location                 | Local embeddings by default (offline-first, no external embedding API dependency).                                        |
| D85 |    B   | Default embedding model family             | Use a strong open-weights embedding model (configurable) with a code-friendly default; keep cloud-embedding optional.     |
| D86 |    B   | Embedding serving method                   | Local embedding service (OpenAI-compatible or native SDK) behind a trait; avoid hard-binding to one server.               |
| D87 |    B   | Chunking strategy for code/docs            | Structure-aware chunking (Rust syntax-aware for code + section-aware for docs) + stable chunk IDs for dedup/replay.       |
| D88 |    C   | Vector index compression posture           | Enable PQ/other compression automatically above a size threshold to control capsule growth and query latency.             |
| D89 |    B   | Recency/usage bias in ranking              | Simple recency boost + provenance weighting + "why this result" explain; no learned ranker yet.                           |
| D90 |    B   | Retrieval quality evaluation harness depth | Golden queries + A/B harness (local-memory vs memvid) + stress tests; deeper eval later.                                  |
| D91 |    C   | Policy/eval gating strictness              | Hard-gate on regressions for golden queries + policy tests; override requires evidence + approval.                        |
| D92 |    B   | NotebookLM Tier2 failure behavior          | Always-on, but never blocking: degrade to Tier1/local synthesis + log + evidence.                                         |
| D93 |    B   | Local reflex failure behavior              | Auto-fallback to cloud coder after N errors/timeouts; record event in evidence + keep pipeline moving.                    |
| D94 |    B   | Canonical source-of-truth during migration | Dual-canonical during rollout; after parity gate, capsule becomes canonical and filesystem becomes a derived/export view. |
| D95 |    C   | Replay capture scope                       | Always capture retrieval + tool calls + prompts/outputs + gate decisions in capsule (redaction/retention apply).          |
| D96 |    B   | Branching UX exposure                      | Expose branch create/switch + compare; implement as capsule branch pointers/copies; avoid complex merges in v1.           |
| D97 |    B   | Logic mesh extraction cadence              | Extract Memory Cards/graph at stage commits (and key ingests) so time-travel checkpoints have deterministic graph state.  |
| D98 |    B   | Graph + replay surfaces                    | Add /speckit.timeline, /speckit.diff, /speckit.replay, /speckit.graph as first-class workbench features.                  |

***

## 6. Reproducibility & Replay (D99-D112)

|   ID | Choice | Decision                               | Notes                                                                                                                                                                      |
| ---: | :----: | -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  D99 |    A   | Run capsule reproducibility package    | Each exported run capsule captures enough code-state to reproduce offline (git commit + diff + key file snapshots).                                                        |
| D100 |    B   | Code snapshot granularity              | Keep exports sane-size: store changed file contents + hashes + diff; don't embed full repo snapshot by default.                                                            |
| D101 |    B   | Tool execution capture                 | Replay can re-run or at least inspect exact build/test outputs; summaries stay fast for UI.                                                                                |
| D102 |    B   | Tier2 source capture                   | NotebookLM/Grounded synth is replayable/auditable without ballooning capsule size.                                                                                         |
| D103 |    A   | Imported capsules read-only            | Attach audit/customer capsules without mutating workspace memory; avoids merge corruption.                                                                                 |
| D104 |    A   | Auto-register mounted capsules         | Mounting/import is immediately discoverable via workspace search and timeline UX.                                                                                          |
| D105 |    B   | Multi-modal ingestion implementation   | Keep core build lean; enable PDF/DOCX/images/audio via feature flags/plugins.                                                                                              |
| D106 |    B   | Embedding/extraction compute isolation | Avoid GPU contention; run embeddings/extraction in separate worker/process to keep reflex latency stable.                                                                  |
| D107 |    A   | Replay audit outputs                   | Human-readable report + machine-verifiable artifact, stored in capsule and exportable.                                                                                     |
| D108 |    A   | Replay supports A/B diffs              | Run deterministic replay comparisons to validate policy/model changes and generate diffs.                                                                                  |
| D109 |    B   | Policy rollout strategy                | New policies ship behind flags/canaries; reduces blast radius, supports rollback.                                                                                          |
| D110 |    A   | Local reflex backup model              | Primary always-on reflex remains **GPT-OSS-20B** (see D43). Standardize Qwen3-A3B as the fallback/bakeoff candidate when GPT-OSS underperforms on repo-specific Rust/code. |
| D111 |    B   | Local inference server default         | Implement SGLang first for radix/prefix caching + schema decoding; keep vLLM as fallback for stability.                                                                    |
| D112 |    B   | Reflex promotion gate                  | Local reflex routing is enabled only after Rust-Reflex-Bench passes TTFT/TPS + JSON validity + cargo check metrics.                                                        |

***

## 7. ARB Pass 2 (D113-D134)

> **Source**: ARCHITECT\_REVIEW\_BOARD\_OUTPUT.md - Sessions 8-10
> **Locked**: 2026-01-19

### Product & Parity (A1-A2)

|   ID | Source | Decision                                                | Notes                                                                                                                                |
| ---: | :----: | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| D113 |   A2   | Tiered parity: Tier 1 = full parity; Tier 2 = TUI-first | Core automation features (artifacts, gating, exit codes) = full parity TUI/CLI/headless; visualization = TUI-first with CLI fallback |

### Evidence Store (B1-B2)

|   ID | Source | Decision                                           | Notes                                                             |
| ---: | :----: | -------------------------------------------------- | ----------------------------------------------------------------- |
| D114 |   B1   | Events + immutable artifacts are authoritative SOR | Projections are rebuildable; events are truth                     |
| D115 |   B1   | Lazy snapshots deferred                            | Until measured latency exceeds thresholds                         |
| D116 |   B2   | Hybrid retention: TTL + milestone protection       | Routine events TTL; ship points protected                         |
| D117 |   B2   | Milestone markers defined                          | SpecCompleted, ReleaseTagged, MilestoneMarked, Stage 6 completion |
| D118 |   B2   | Default TTL: 90 days; milestone: 1 year            | Configurable via policy                                           |

### Capture Mode (C1-C2)

|   ID | Source | Decision                                    | Notes                                                                  |
| ---: | :----: | ------------------------------------------- | ---------------------------------------------------------------------- |
| D119 |   C1   | Over-capture is always hard-blocked         | Tier 1 absolute - storing more than policy permits is never acceptable |
| D120 |   C1   | Under-capture warned, blocked at checkpoint | Must be resolved or acknowledged with auditable record                 |
| D121 |   C1   | Capture gap acknowledgments auditable       | Creates `CaptureGapAcknowledged` event                                 |
| D124 |   C2   | Capture mode defaults are policy-derived    | Template uses `prompts_only`; `full_io` is explicit opt-in             |

### Pipeline & Gates (D1-D2, E1-E2)

|   ID | Source | Decision                             | Notes                                                            |
| ---: | :----: | ------------------------------------ | ---------------------------------------------------------------- |
| D122 |   D1   | Monolith with internal seams         | No dynamic plugins; no actor model; 8 fixed stages               |
| D123 |   D2   | Blocking-with-override gates         | Overrides emit GateDecision event; discouraged at ship           |
| D125 |   E2   | Policy sovereignty = Tier 1 absolute | Over-capture, non-logical URIs, SOR violations, merge invariants |

### Maintenance (F1-F2)

|   ID | Source | Decision                               | Notes                                                       |
| ---: | :----: | -------------------------------------- | ----------------------------------------------------------- |
| D126 |  F1/F2 | Tiered maintenance; Health Check first | Event + scheduled + on-demand triggers; no permanent daemon |

### ACE + Maieutics (H0-H7)

|   ID | Source | Decision                                      | Notes                                                                                                                       |
| ---: | :----: | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| D127 |   H0   | ACE Frames + Maieutic Specs replace consensus | Multi-model voting deprecated; two canonical explainability artifacts                                                       |
| D128 |   H1   | ACE explanation scope is tiered               | Tier 1 (mandatory): failures/overrides/policy; Tier 2: boundaries; Tier 3: selective; Tier 4: on-demand; Tier 5: event-only |
| D129 |   H2   | Tiered autonomy with delegation               | Tier 0-1 (auto), Tier 2 (checkpoint), Tier 3 (explicit), Tier 4 (human-only)                                                |
| D130 |   H3   | Maieutic step always mandatory                | Fast path allowed; no exceptions; must complete before automation                                                           |
| D131 |   H4   | Explainability follows capture mode           | `capture=none` persists no artifacts (in-memory guidance still runs)                                                        |
| D132 |   H5   | Ship hard-fail without artifacts              | `capture=none` is non-shippable (Tier 1 absolute)                                                                           |
| D133 |   H6   | A2-aligned multi-surface parity               | Tier 1 via shared executor; headless requires `--maieutic`/`--maieutic-answers`; headless never prompts                     |
| D134 |   H7   | ACE Frame schema generated + versioned        | `#[derive(JsonSchema)]` via schemars; `schema_version` embedded; immutable once released                                    |

***

## Change History

| Version | Date       | Changes                                                              |
| ------- | ---------- | -------------------------------------------------------------------- |
| 1.0.0   | 2026-01-22 | Initial canonical version; migrated from DECISION\_REGISTER.md v0.13 |
| 0.13    | 2026-01-20 | Added D113-D134 (ARB Pass 2)                                         |
| 0.12    | 2026-01-19 | Added D99-D112 (Reproducibility)                                     |
| 0.11    | 2026-01-18 | Added D81-D98 (Memvid Integration)                                   |

***

**Navigation**: [INDEX.md](INDEX.md) | [POLICY.md](POLICY.md) | [ARCHITECTURE.md](ARCHITECTURE.md)
