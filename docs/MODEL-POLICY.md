# Model Routing Policy

**Version**: 2.0.0
**Created**: 2025-12-14
**Updated**: 2026-01-04 (S35 - Single-GPU optimization)
**Status**: Active

This document is the **single source of truth** for model routing in codex-rs. All specs MUST reference this document and comply with its constraints.

---

## 0. Routing Philosophy (v2.0)

### Core Principle: "Cloud where quality wins, Local where speed wins"

**NOT** "local-first with cloud escalation" — that philosophy assumed privacy constraints that don't apply.

Since **privacy is not a constraint**, the routing decision should be based purely on:
1. **Quality-per-minute** — cloud models generally win for deep reasoning
2. **Quality-per-dollar** — DeepSeek/Sonnet are cheap enough for most work
3. **Latency sensitivity** — local wins for tight loops (compile/test/fix cycles)
4. **Volume economics** — local wins for high-volume automation (capex paid)

### Single-GPU Constraint (RTX 5090 32GB)

With one 5090, assume:
- **One vLLM model loaded at a time** (practically)
- **Model swapping** for different roles, not concurrent loading
- Local is the **fast reflex lane**, not the default brain

### What Local Wins At

| Advantage | Why It Matters |
|-----------|----------------|
| **Always-on reflex** | No quotas, no network, no latency spikes |
| **High-volume economics** | Capex paid, marginal cost is electricity |
| **Deterministic pinned behavior** | Stable snapshots for CI/eval harnesses |
| **Tight tool loops** | Latency dominates compile/test/fix cycles |

### What Local Cannot Beat

| Limitation | Implication |
|------------|-------------|
| **Frontier reasoning quality** | Cloud wins for deep planning, multi-file refactors |
| **Concurrent model loading** | Can't run 14B + 32B simultaneously on 32GB |

---

## 1. Model Routing Table

| Role | Default | Provider | Local/Cloud | When to Use Local | Fallback | Constraints |
|------|---------|----------|-------------|-------------------|----------|-------------|
| **Architect / Planner** | Sonnet / GPT-5.1 | Anthropic/OpenAI | **Cloud** | Fast draft only, offline mode | DeepSeek R2 | Must emit `plan.json` + confidence. Cloud-first for quality. |
| **Implementer / Rust Ace** | Local MoE coder | Local | **Local (attempt 1)** | Small-to-medium diffs, reflex patches | DeepSeek → Sonnet/GPT-5.1 | Cloud-first for: cross-crate, unsafe, public API. Escalate after 2 fails. |
| **Librarian (long-context)** | Local 8–14B synth | Local | **Local** | Routine cleanup, structure | Kimi K2 (escalation-only) | Kimi only for hard sweeps (>100k, contradictions, tool-heavy). |
| **Tutor (maieutic coach)** | Local MoE coder | Local | **Local** | Always — perfect fit | Haiku | Fast, interactive, cheap. Keep sessions short. |
| **Auditor / Final Judge** | GPT-5.1 High / Claude Opus | OpenAI/Anthropic | **Cloud-only** | Never local | Sonnet / DeepSeek R2 | Always cloud. Rare + budget-gated. Must cite evidence. |
| **Stage0 Tier2 Synth** | NotebookLM | Google | **Cloud** | — | Local summarizer | Citation-grounded. Use session IDs. |
| **Stage0 Retrieval** | Deterministic | — | **Local** | Always | — | No LLM except optional "explain scoring" lane. |
| **ACE Reflector / Curator** | Local 8B instruct | Local | **Local** | Routine curation | Kimi (escalation-only) | Escalate only for "interesting failures". |
| **Embeddings** | bge-m3 | Local | **Local** | Always | OpenAI embeddings | Must be stable & reproducible. GPU-accelerated. |
| **CV / Camera** | YOLO/TensorRT | Local | **Local** | Always | — | Separate from LLM scheduling. |

### Cloud-First Lanes (Quality Wins)

```
Architect/Planner ──────► Sonnet / GPT-5.1 (default)
                              │
                              ▼ (fallback)
                         DeepSeek R2

Auditor/Judge ──────────► GPT-5.1 High / Opus (always cloud)

Serious Implementer ────► Cloud for:
                          • Cross-crate refactors
                          • async/concurrency/lifetimes
                          • unsafe, FFI, sandbox changes
                          • Public API changes
```

### Local-First Lanes (Speed/Volume Wins)

```
Tutor ──────────────────► Local MoE (always)

Reflex Implementer ─────► Local MoE for:
                          • Small patches (<100 lines)
                          • Formatting/boilerplate
                          • First-pass attempts
                          • Tool-loop retries

Librarian ──────────────► Local 8–14B (routine)
                              │
                              ▼ (escalation)
                         Kimi K2 (hard sweeps only)

Embeddings + Retrieval ─► Local (always)
```

---

## 2. Consensus Policy (GR-001)

### Forbidden Patterns

The following multi-agent patterns are **explicitly forbidden** in the default path:

- 3-agent debate / voting / swarm synthesis
- Committee merges or "consensus synthesis" steps
- Any requirement that multiple models agree before progressing
- Dedicated "synthesizer agent" that merges multiple agent outputs

### Allowed Patterns (Non-Authoritative Only)

These patterns are allowed **only as non-authoritative sidecars**:

1. **Critic-only sidecar**: One model writes, another critiques without rewriting. Critic outputs: risks, contradictions, missing requirements, guardrail conflicts.

2. **Escalation-only second opinion**: Triggered only on failure/high-risk conditions (confidence < threshold, repeated failures, user request).

3. **Self-critique within same model**: Preferred over multi-agent. Architect does draft → maieutic self-interrogation → final.

### Canonical Pipeline

```
Stage 0 → Single Architect → Single Implementer → Single Judge
              (optional critic-only sidecar if triggered)
```

Quality enforced by: compiler/tests, constitution gates, Judge audit — **not by voting**.

### Implementation (GR001-001, P109)

**Status**: ✅ Implemented

**Feature Flags**:
- `SPEC_KIT_CONSENSUS=false` (default) — Consensus disabled, single-owner pipeline
- `SPEC_KIT_CONSENSUS=true` — Legacy mode with deprecation warning (NOT RECOMMENDED)
- `SPEC_KIT_CRITIC=true` — Non-blocking critic-only sidecar

**Behavior**:
- `expected_agents_for_stage()` returns single preferred agent by default
- `run_spec_consensus()` returns `consensus_ok=true` when disabled (skips validation)
- Legacy mode emits tracing::warn on first invocation

**Code**: `tui/src/chatwidget/spec_kit/consensus.rs`

---

## 3. Escalation Triggers

### Architect: N/A (Cloud-First)

Architect/Planner is **cloud-first by default**. No escalation needed — start with Sonnet/GPT.

Local planner only used for:
- Fast draft plans (then cloud reviews)
- Offline mode
- High-volume batch planning where latency matters more than quality

### Implementer Escalation (Local → Cloud)

**Start local** for small-to-medium diffs, **escalate to cloud** when:
- 2 failed compile/test loops
- Low-confidence self-check
- Change is High-Risk (see §7):
  - Cross-crate refactor / async / concurrency / lifetimes
  - `unsafe`, FFI, sandbox changes
  - Auth/security/crypto
  - Public API changes
- User explicitly requests cloud

**Direct to cloud** (skip local) when:
- Multi-file architectural refactor
- Scope estimate > 500 lines
- Requirements are ambiguous

### Librarian Escalation (to Kimi)
- `context_estimate > 100k`
- `contradictions_detected == true`
- `tool_heavy == true`
- `full_sweep == true`
- `local_synth_confidence < threshold`

---

## 4. Kimi Migration Scope

### Models
- **Kimi-K2-Instruct-0905**: Default escalation for longform synthesis
- **kimi-k2-thinking**: Deep reasoning + tool loops
- **kimi-k2-thinking-turbo**: Speed-optimized (higher cost)
- **moonshot-v1-128k**: Legacy fallback

### Roles Using Kimi (Escalation-Only)
- Librarian (long-context writer)
- ACE Reflector / Curator

### Where Called
- Internal router → OpenAI-compatible HTTP API
- All calls enforce: escalation predicates, budgets, logging, retries

---

## 5. DeepSeek Migration Scope

### Models
- **deepseek-chat**: Non-thinking mode (V3.2), 128K context
- **deepseek-reasoner**: Thinking mode (V3.2), 128K context

### Roles Using DeepSeek (Escalation-Only)
- Implementer / Rust Ace (when stuck)

### Access
- Cloud-only (official API: `https://api.deepseek.com`)
- OpenAI-compatible SDK

---

## 6. Local Services Baseline

### MUST Run Locally
- codex-rs TUI/CLI orchestrator
- Model Router / Policy Engine
- Local LLM runtime (vLLM primary, llama.cpp fallback)
- Stage0 Engine + indexes (TF-IDF/MMR + Shadow Code Brain)
- local-memory daemon + overlay SQLite DB
- Tool executor (compile/test loops)
- Evidence/artifact store
- Telemetry/logs/cost tracking
- Embeddings pipeline (bge-m3)
- Git harvester

### Hardware Configuration (Single RTX 5090)

```
GPU: NVIDIA GeForce RTX 5090 (32GB VRAM)
├── Driver: 580.105.08
├── CUDA: 13.0
└── Constraint: ONE model loaded at a time

Runtime: vLLM (primary), llama.cpp (fallback)
Ollama: Dev bring-up only (not production)
```

### Recommended Local Model (Single-GPU Optimized)

**Primary**: Qwen3-Coder-30B-A3B-Instruct (MoE)
- Total params: 30.5B
- Activated params: 3.3B (efficient inference)
- Context: 262,144 tokens
- VRAM: ~12-16GB with AWQ 4-bit quantization
- Use case: Tutor, Reflex Implementer, Librarian

**Why MoE?** High capability with low activated params = fast inference + headroom for KV cache.

**Alternative** (if MoE unavailable): Ministral 3 14B
- Optimized for local deployment
- 256k context
- ~28GB VRAM at FP16

### Model Swapping Strategy

Since only one model fits at production quality:

```
Workflow          │ Model Loaded
──────────────────┼─────────────────────────────
Tutor session     │ Qwen3-Coder-30B-A3B
Reflex implement  │ Qwen3-Coder-30B-A3B (same)
Librarian sweep   │ Qwen3-Coder-30B-A3B (same)
──────────────────┼─────────────────────────────
Architect/Plan    │ Cloud (Sonnet/GPT) — no swap
Serious implement │ Cloud (DeepSeek/Sonnet) — no swap
Judge/Audit       │ Cloud (GPT-5.1 High) — no swap
```

**Result**: One local model serves all local lanes. No swapping overhead in practice.

---

## 7. Guardrails (GR-001 through GR-013)

### Governance & Routing
- **GR-001**: No consensus by default
- **GR-002**: High-Risk requires cloud Judge
- **GR-003**: Local models cannot approve merges
- **GR-004**: Escalation rules are deterministic
- **GR-005**: No silent degradation (must emit audit line)

### Evidence & Correctness
- **GR-006**: Compiler/tests are truth
- **GR-007**: Evidence artifacts required
- **GR-008**: NotebookLM must be citation-grounded; no hallucinated citations

### Privacy, Safety, Budget
- **GR-009**: `local_only` forbids ALL cloud calls
- **GR-010**: Tool execution sandboxing
- **GR-011**: Budget caps enforced by router

### Structured Output
- **GR-012**: Structured JSON outputs mandatory for automation
- **GR-013**: Escalations must record minimal provenance

### High-Risk Definition
A change is **High-Risk** if it includes:
- Cross-crate refactor (async/concurrency, lifetimes, shared state)
- `unsafe`, FFI, sandbox/execpolicy changes
- Auth/security/crypto, secrets, credentials, PII
- Data loss/migrations, persistence formats, schema changes
- Build/tooling/runtime changes (model routing, providers, CLI/MCP)
- Public API changes (CLI flags, commands, config schema)

---

## 8. Spec Compliance

Every spec MUST include:

```md
## Model & Runtime (Spec Overrides)

Policy: docs/MODEL-POLICY.md (version: 2.0.0)

Roles exercised by this spec:
- Stage0 Tier2 (NotebookLM): YES/NO
- Architect/Planner: YES/NO
- Implementer/Rust Ace: YES/NO
- Librarian: YES/NO
- Tutor: YES/NO
- Auditor/Judge: YES/NO

Routing mode: cloud-where-quality-wins, local-where-speed-wins
Architect: Cloud-first (Sonnet/GPT)
Implementer: Local for small diffs, cloud for serious/HR work
Librarian: Local + Kimi escalation (hard sweeps)
Judge: Cloud-only (always)

Primary tiers:
- cloud_quality: Sonnet / GPT-5.1 (Architect, Serious Implementer)
- local_reflex: Qwen3-Coder-30B-A3B MoE (Tutor, Reflex Implementer, Librarian)
- cloud_escalation: DeepSeek R2 / Kimi K2 (when local fails)
- premium_judge: GPT-5.1 High / Claude Opus (HR required)

Privacy:
- local_only = false  # if true → forbids all cloud calls

High-risk:
- HR = YES/NO (if YES → cloud Architect + cloud Judge required)

Overrides (must be rare):
- <none | list>
```

Infrastructure-only specs may use:

```md
## Model & Runtime (Spec Overrides)

Policy: docs/MODEL-POLICY.md (version: 2.0.0)

This spec is **infrastructure-only** and does not invoke model routing directly.
Roles exercised: none (no Architect/Implementer/Librarian/Tutor/Judge).
Privacy: local_only = <true|false>
Guardrails still apply: sandboxing, evidence/logging, no hallucinated citations.
```

---

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2026-01-04 | **Philosophy shift**: "cloud-where-quality-wins, local-where-speed-wins". Single-GPU optimization (RTX 5090). Architect/Planner now cloud-first. Simplified to single local MoE model (Qwen3-Coder-30B-A3B). Added §0 Routing Philosophy. |
| 1.0.0 | 2025-12-14 | Initial policy from Q0.1–Q0.9 audit |

---

Back to [Key Docs](KEY_DOCS.md)
