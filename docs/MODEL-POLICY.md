# Model Policy (v2 Track)
**Last Updated:** 2026-01-10

> This document is being upgraded from the earlier v1 policy to **Model Policy v2**.
> The canonical decision locks live in `docs/DECISION_REGISTER.md` and are snapshotted into Memvid capsules per run.

## Policy lifecycle
- Author policy in repo (this doc + structured config).
- Validate in CI (schema + golden scenarios).
- Deploy with version bump.
- Enforce at router + gates.
- Snapshot into capsule (`PolicySnapshot.json`) per run/checkpoint.
- Monitor and audit via replay reports.

## Default role routing (configurable)
| Role | Default | Notes |
| --- | --- | --- |
| Architect | `gpt-5.2-xhigh` | Cloud frontier reasoning/design |
| Judge | `gpt-5.2-xhigh` | Cloud frontier for unlock gates |
| Implementer | `gpt-5.2-xhigh` / `gpt-5.2-high` | Cloud coder; can be escalated to |
| **Implementer.Reflex** (local loop) | `gpt-oss-20b` (local via SGLang) | Not a new pipeline stage: a **routing mode** used inside the Implement stage for sub-second compiler/test iteration |
| SidecarCritic | `gpt-5.2-mini` | Always-on cheap critique |
| NotebookLM Tier2 | NLM service | Non-blocking synthesis |

**Important implementation note:** in code, treat “Reflex” as **`role=Implementer` + `mode=reflex`** (or a similar flag), not as a new Stage0 role. This keeps the workflow UX stable while still allowing distinct model routing, telemetry, and bakeoff gates.

## Local Reflex Alternatives (Backup Candidates)
- **Qwen3-Coder-30B-A3B-Instruct (AWQ/GPTQ)** — standardized fallback/bakeoff candidate when GPT-OSS-20B underperforms on repo-specific Rust/crate APIs.
- (Optional) **Qwen2.5-Coder-32B (AWQ INT4)** — dense fallback if MoE models regress; higher VRAM + lower context headroom.

**Rule:** Default reflex stays `gpt-oss-20b`. Alternatives are opt-in via config and must pass the bakeoff gate before promotion.
## Escalation rules (minimum set)
- Reflex gets `reflex_max_attempts` (default: 2). After that, escalate to cloud Implementer.
- High-risk specs can skip reflex and route directly to cloud Implementer/Architect.
- Judge remains cloud for unlock decisions.

## Evidence requirements
- Every model/tool call logs: role, stage, provider, model, attempt, and **selection reason**.
- Every run stores `PolicySnapshot.json` into the capsule and references it from timeline events.

## Related specs
- SPEC-KIT-977 — Model Policy v2 (Lifecycle + Enforcement)
- SPEC-KIT-975 — Replayable Audits v1
