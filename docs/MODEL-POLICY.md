# Model Routing Policy

**Version**: 1.0.0
**Created**: 2025-12-14
**Status**: Active

This document is the **single source of truth** for model routing in codex-rs. All specs MUST reference this document and comply with its constraints.

---

## 1. Model Routing Table

| Role | Default Model | Provider | Local/Cloud | Runtime | Fallback | Constraints |
|------|---------------|----------|-------------|---------|----------|-------------|
| **Architect / Planner** | Local 14B reasoning (Ministral 3 14B / Qwen 14B thinking) | Local | Local | vLLM | GPT-5.1 High → DeepSeek R2 | Must emit `plan.json` + confidence. Escalate if `confidence < 0.75`. |
| **Implementer / Rust Ace** | Local 32B coder (Qwen2.5/3 Coder 32B) | Local | Local | vLLM | DeepSeek coder/R2 → Sonnet/GPT-5.1 High | Follow plan strictly. Escalate after 2 failed compile/test loops. |
| **Librarian (long-context)** | Local 8–14B synth (Llama 3.1 8B / Qwen 14B) | Local | Local | vLLM | Kimi K2 (escalation-only) → Sonnet | Kimi only for hard sweeps (>100k context, contradictions, tool-heavy). |
| **Tutor (maieutic coach)** | Local 7B instruct (Qwen2.5-Coder-7B) | Local | Local | vLLM/llama.cpp | Haiku → GPT-5.1 low | Ask questions first. Keep sessions short. |
| **Auditor / Final Judge** | GPT-5.1 High (or Claude Opus) | OpenAI/Anthropic | Cloud | — | Sonnet / DeepSeek R2 | Rare + budget-gated. Must cite evidence. Run after diff + tests. |
| **Stage0 Tier2 Synth** | NotebookLM (citation-grounded) | Google | Cloud | — | Local 8–14B summarizer | Must be citation-grounded. Use session IDs. |
| **Stage0 Retrieval** | Deterministic | — | Local | TF-IDF + MMR | — | No LLM except optional "explain scoring" lane. |
| **ACE Reflector / Curator** | Local 8B instruct (Llama 3.1 8B / Qwen 14B) | Local | Local | vLLM/llama.cpp | Kimi (escalation-only) | Escalate only for "interesting failures". |
| **Embeddings** | bge-m3 | Local | Local | sentence-transformers | OpenAI embeddings | Must be stable & reproducible. |
| **CV / Camera** | YOLO/TensorRT | Local | Local | TensorRT/PyTorch | — | Separate from LLM scheduling. |

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

### Architect Escalation
- `confidence < 0.75`
- Scope is large or requirements ambiguous
- User explicitly requests escalation

### Implementer Escalation (to DeepSeek)
- 2 failed compile/test loops
- Low-confidence self-check
- Complex multi-crate refactor / high coupling surface
- Tool-heavy refactor beyond local model stability

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

### Hardware Assumptions
- GPU: RTX 5090 32GB VRAM
- Runtime: vLLM (default), llama.cpp (fallback)
- Ollama: Dev bring-up only (not production default)

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

Policy: docs/MODEL-POLICY.md (version: 1.0.0)

Roles exercised by this spec:
- Stage0 Tier2 (NotebookLM): YES/NO
- Architect/Planner: YES/NO
- Implementer/Rust Ace: YES/NO
- Librarian: YES/NO
- Tutor: YES/NO
- Auditor/Judge: YES/NO

Routing mode: local-first with escalation
Architect escalation: confidence < 0.75
Implementer escalation: 2 failed compile/test loops
Kimi: escalation-only (Librarian hard sweeps)
DeepSeek: escalation-only (Implementer stuck)

Primary tiers:
- fast_local: <local 14B planner> + <local 32B coder> (vLLM on RTX 5090)
- slow_cloud_escalation: DeepSeek (reasoner/chat) [escalation-only]
- premium_judge: GPT-5.1 High / Claude Opus (HR required)

Privacy:
- local_only = false  # if true → forbids all cloud calls

High-risk:
- HR = YES/NO (if YES → cloud Judge required)

Overrides (must be rare):
- <none | list>
```

Infrastructure-only specs may use:

```md
## Model & Runtime (Spec Overrides)

Policy: docs/MODEL-POLICY.md (version: 1.0.0)

This spec is **infrastructure-only** and does not invoke model routing directly.
Roles exercised: none (no Architect/Implementer/Librarian/Tutor/Judge).
Privacy: local_only = <true|false>
Guardrails still apply: sandboxing, evidence/logging, no hallucinated citations.
```

---

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-12-14 | Initial policy from Q0.1–Q0.9 audit |

---

Back to [Key Docs](KEY_DOCS.md)
