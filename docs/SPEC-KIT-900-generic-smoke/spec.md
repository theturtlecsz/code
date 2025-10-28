**SPEC-ID**: SPEC-KIT-900-generic-smoke
**Feature**: Generic Multi-Agent Smoke Scenario
**Status**: Ready for Testing
**Created**: 2025-10-28
**Branch**: feature/spec-kit-069-complete
**Owner**: Code

**Context**: This SPEC provides a neutral, design-agnostic workload that exercises `/speckit.plan`, `/speckit.tasks`, and `/speckit.validate` without touching production-sensitive content. It exists purely to benchmark orchestration behaviour (latency, model mix, cost) after router changes such as SPEC-KIT-070. Analysts can run the same scenario under different model configurations and compare evidence artifacts, cost summaries, and consensus quality without mutating real roadmap items.

---

## Test Objectives

1. **Repeatability**: The prompts remain stable across runs so cost/quality deltas reflect routing changes, not domain shifts.
2. **Coverage**: The scenario forces all three stages (plan, tasks, validate) to execute with typical agent output volume (~4-6k tokens per stage).
3. **Neutrality**: Content is intentionally generic ("launch sample productivity microservice")—no team-specific jargon or confidential details.
4. **Evidence Quality**: Each stage must emit consensus verdicts, local-memory entries, and `cost_summary.json` for downstream analysis.

---

## Workload Summary

- **High-level goal**: "Design, decompose, and validate a small productivity microservice that syncs reminders across devices."
- **Primary actor**: Internal platform engineer responsible for developer tooling.
- **Constraints**: Lightweight scope (two-week implementation), no external dependencies, focus on API + UI parity, include telemetry and rollback strategies.
- **Non-Goals**: No integration with billing, auth, or existing customer data.

---

## Stage Guidance

### `/speckit.plan SPEC-KIT-900`

Prompt should solicit:
- Three milestone timeline (design, implementation, validation).
- Risk register with at least three items (technical, process, operational).
- Success metrics (latency, user adoption proxy, telemetry coverage).

Acceptance checks:
- Plan includes timeline table, risk/mitigation list, and measurable success metrics.
- Consensus summary references all three participating agents.

### `/speckit.tasks SPEC-KIT-900`

Prompt should decompose work into 8–12 tasks grouped by milestone. Requirements:
- Each task has owner role, deliverable, and definition-of-done bullet.
- Identify at least two cross-team touchpoints (e.g., UX review, QA sign-off).
- Flag tasks suitable for parallel execution.

Acceptance checks:
- Task list saved to evidence with `stage:tasks` tag.
- Parallelisation guidance present ("run in parallel" or equivalent wording).

### `/speckit.validate SPEC-KIT-900`

Prompt should request validation strategy covering:
- Unit, integration, and synthetic load tests.
- Rollback and monitoring checklist.
- Estimated cost of running the validation suite.

Acceptance checks:
- Validation plan references monitoring KPIs and rollback trigger.
- Lifecycle telemetry written under `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-900/`.
- Cost summary updated when consensus completes.

---

## Task Decomposition (Tasks Stage · 2025-10-28)

### T1 – Prompt Calibration Kickoff
- **Milestone**: Design (Days 1–3) · **Owner**: Prompt Architect · **Dependencies**: Plan baseline v0.1 · **Parallel**: Yes
- **Deliverable**: Refined `/speckit.tasks` prompt pack referencing latency (<150 ms p95), adoption (≥5 runs/week), telemetry (100% schema compliance).
- **Definition of Done**: (1) Prompt text mirrors PRD §4 wording with metric guardrails; (2) Token budget rationale (4–6k output) documented; (3) Version ID circulated to agent roster.
- **Validation Hooks**: Prompt lint script + dry-run token counter.
- **Documentation Updates**: `docs/spec-kit/prompts.json`; introduce tasks preface in this spec.
- **Cross-Team Touchpoints**: Content Design async sign-off.
- **Risks/Assumptions**: Assumes upstream prompt compiler unchanged; drift mitigated via versioning.
- **Security Review**: Not required.

### T2 – Reminder Sync API Contract Definition
- **Milestone**: Design (Days 1–3) · **Owner**: Backend Lead · **Dependencies**: T1 · **Parallel**: No (waits on prompt baselines)
- **Deliverable**: OpenAPI contract for deterministic reminder-sync stub with latency budgets.
- **Definition of Done**: (1) Contract encodes 150 ms p95 target + retry semantics; (2) Mock payloads verified vendor-neutral; (3) Platform architect sign-off captured.
- **Validation Hooks**: Schema validation + mock integration test.
- **Documentation Updates**: `docs/spec-kit/api/reminder-sync.md`; update milestone narrative in this spec.
- **Cross-Team Touchpoints**: Platform Architecture sync.
- **Risks/Assumptions**: Telemetry fields must be specified early to avoid rework.
- **Security Review**: Required (design-time data handling).

### T3 – Consensus Risk Mitigation Plan
- **Milestone**: Design (Days 1–3) · **Owner**: Spec Ops Analyst · **Dependencies**: T1, T2 · **Parallel**: Yes
- **Deliverable**: Mitigation matrix covering consensus drift, agent degradation, evidence growth.
- **Definition of Done**: (1) Risk register with triggers/owners/response plans; (2) Retry guardrails mapped to handler logic; (3) Spec Ops lead approval recorded.
- **Validation Hooks**: `/speckit.checklist` dry-run against risk items.
- **Documentation Updates**: `docs/spec-kit/risk-register.md`; expand risk section in this spec.
- **Cross-Team Touchpoints**: MCP infrastructure for retry and throughput limits.
- **Risks/Assumptions**: Assumes MCP throughput stable; highlights evidence footprint growth as open concern.
- **Security Review**: Not required.

### T4 – Reminder Sync Service Skeleton
- **Milestone**: Build & Telemetry (Days 4–9) · **Owner**: Backend Engineer · **Dependencies**: T2 · **Parallel**: Yes
- **Deliverable**: Rust microservice stub implementing deterministic reminder schedule and logging hooks.
- **Definition of Done**: (1) Service compiles with telemetry emitters stubbed; (2) Latency instrumentation active with <150 ms guard; (3) Unit tests covering CRUD path.
- **Validation Hooks**: `cargo test reminder_sync` + latency smoke profile.
- **Documentation Updates**: `services/reminder-sync/README.md`; task table excerpt in this spec.
- **Cross-Team Touchpoints**: None.
- **Risks/Assumptions**: CI environment must support new crate; latency metrics risk tracked for validation stage.
- **Security Review**: Required (code-layer data handling).

### T5 – Telemetry Ingestor & Schema Validation
- **Milestone**: Build & Telemetry (Days 4–9) · **Owner**: Telemetry Engineer · **Dependencies**: T4 · **Parallel**: Yes
- **Deliverable**: Telemetry ingestion pipeline enforcing schema v1 with alert thresholds for latency/adoption.
- **Definition of Done**: (1) 100% of events pass schema validator; (2) Alerts configured for latency breach & adoption drop; (3) Integration tests cover success/failure flows.
- **Validation Hooks**: Telemetry schema validator; alert simulation harness.
- **Documentation Updates**: `docs/spec-kit/telemetry.md`; observability appendix in this spec.
- **Cross-Team Touchpoints**: Data Platform team for pipeline alignment.
- **Risks/Assumptions**: Shared telemetry bus capacity; risk of alert fatigue flagged.
- **Security Review**: Required (telemetry data handling).

### T6 – Evidence Writer Enhancements
- **Milestone**: Build & Telemetry (Days 4–9) · **Owner**: Tooling Engineer · **Dependencies**: T4, T5 · **Parallel**: No
- **Deliverable**: Evidence writer updates guaranteeing consensus artifacts + cost summaries persist for tasks stage.
- **Definition of Done**: (1) Artifacts stored under `docs/SPEC-OPS-004.../evidence/tasks/`; (2) Cost summary contains `per_stage.tasks`; (3) Integration test proving persistence + locking.
- **Validation Hooks**: Evidence integration test; evidence footprint script.
- **Documentation Updates**: `docs/spec-kit/evidence-baseline.md`; evidence section here.
- **Cross-Team Touchpoints**: Evidence custodians for archival policy.
- **Risks/Assumptions**: File-locking must withstand concurrent runs; growth monitored via cleanup scripts.
- **Security Review**: Not required.

### T7 – Security Assessment & Threat Modeling
- **Milestone**: Build & Telemetry (Days 4–9) · **Owner**: Security Engineer · **Dependencies**: T2, T4, T5 · **Parallel**: No
- **Deliverable**: STRIDE assessment + remediation ticket capturing telemetry/microservice risks.
- **Definition of Done**: (1) Threat model stored with mitigations; (2) Security ticket closed with sign-off; (3) Follow-up tasks logged in SPEC tracker if needed.
- **Validation Hooks**: Security review checklist; static analysis run.
- **Documentation Updates**: `docs/security/reminder-sync-threat-model.md`; risk log in this spec.
- **Cross-Team Touchpoints**: Security Guild weekly review.
- **Risks/Assumptions**: Assumes no PII exposure; telemetry classification risk highlighted.
- **Security Review**: Required (primary security gate).

### T8 – Performance & Load Benchmarking
- **Milestone**: Validation & Benchmarking (Days 10–14) · **Owner**: Performance Engineer · **Dependencies**: T4, T5 · **Parallel**: Yes
- **Deliverable**: Benchmark suite + report demonstrating <150 ms p95 under reference load.
- **Definition of Done**: (1) Five benchmark runs recorded with stable metrics; (2) Report + charts stored in evidence; (3) Regression thresholds codified in CI guardrail.
- **Validation Hooks**: Load-test harness; monitoring dashboards.
- **Documentation Updates**: `docs/spec-kit/perf-plan.md`; success metrics section here.
- **Cross-Team Touchpoints**: SRE for monitoring hooks.
- **Risks/Assumptions**: Requires staging capacity; noisy-neighbour impact documented.
- **Security Review**: Not required.

### T9 – Telemetry Coverage QA & Alert Verification
- **Milestone**: Validation & Benchmarking (Days 10–14) · **Owner**: QA Lead · **Dependencies**: T5 · **Parallel**: Yes
- **Deliverable**: QA report confirming 100% schema compliance + alert behaviour.
- **Definition of Done**: (1) QA cases cover success/failure flows; (2) Alert fire drills documented; (3) Coverage dashboard screenshots archived.
- **Validation Hooks**: QA automation suite; alert playback checklist.
- **Documentation Updates**: `docs/spec-kit/testing-policy.md` addendum; validation checklist in this spec.
- **Cross-Team Touchpoints**: Telemetry Ops for routing confirmation.
- **Risks/Assumptions**: QA env must mirror production schemas; flaky alerts tracked.
- **Security Review**: Not required.

### T10 – Consensus & Cost Artifact Audit
- **Milestone**: Validation & Benchmarking (Days 10–14) · **Owner**: Spec Kit Operator · **Dependencies**: T6, T8, T9 · **Parallel**: No
- **Deliverable**: Final audit packet confirming ≥90% agent agreement + evidence completeness + cost totals.
- **Definition of Done**: (1) Consensus synthesis present, conflicts = []; (2) Cost summary reconciled with telemetry; (3) SPEC.md tracker updated with links + status.
- **Validation Hooks**: `/spec-consensus` check; evidence footprint analyser.
- **Documentation Updates**: Update this spec (tasks, audit, costs); refresh SPEC.md entry.
- **Cross-Team Touchpoints**: Finance liaison for cost verification.
- **Risks/Assumptions**: Relies on MCP uptime; degraded mode process documented.
- **Security Review**: Not required.

### Consensus & Agent Notes
- **Agent Participation**: `gpt_pro` produced full task matrix. `gemini` and `claude` attempts failed to access untracked documents and returned unusable summaries. Stage recorded as **degraded (1/3 actionable outputs)**.
- **Agreements**: All agents that returned content aligned on three milestones, 10-task scope, and telemetry-first success metrics.
- **Conflicts/Divergence**: No conflicting recommendations surfaced; primary gap was missing actionable content from gemini/claude due to workspace context mismatch. Mitigation: include inline context for future stages or ensure docs committed before agent fan-out.
- **Action Items**: Store gpt_pro consensus details in local-memory (`importance:9`) once evidence pipeline is available; repeat `/speckit.tasks` in cheap-tier routing after docs are committed to verify non-degraded run.

### Outstanding Risks After Tasks Stage
1. **Telemetry Pipeline Saturation**: Tasks T5/T9 depend on shared ingest capacity; require validation during load tests (assignee: Telemetry Engineer).
2. **Evidence Footprint Growth**: Tasks T6/T10 must coordinate cleanup to remain <15 MB per policy (assignee: Tooling Engineer).
3. **Security Review Follow-through**: T2/T4/T5/T7 flagged security reviews—track ticket closure before `/speckit.validate`.

---

## Success Criteria (for the SPEC itself)

- [ ] All three stages complete without manual editing of prompts.
- [ ] `local-memory search "spec:SPEC-KIT-900 stage:plan"` returns ≥1 artifact per agent.
- [ ] Cost summary JSON exists and contains `per_stage.plan`, `per_stage.tasks`, `per_stage.validate` entries.
- [ ] Consensus verdicts show ≥90% agreement (no conflicts) for plan/tasks/validate when using the reference cheap routing.
- [ ] Manual review rates outputs "adequate" or better for clarity and structure.

---

## Usage Notes

- **Environment**: Run from `/home/thetu/code/codex-rs` with the current routing configuration under test (e.g., SPEC-KIT-070 cheap-tier routing).
- **Command Sequence** (typical):
  1. `/speckit.plan SPEC-KIT-900`
  2. `/speckit.tasks SPEC-KIT-900`
  3. `/speckit.validate SPEC-KIT-900`
- **Evidence Paths**:
  - Cost summary: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/SPEC-KIT-900_cost_summary.json`
  - Stage telemetry/commands: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-900/`
  - Consensus synthesis: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-900/`
- **Teardown**: Evidence can be archived with `./scripts/evidence_archive.sh --spec SPEC-KIT-900` once analysis is complete.

---

## Rollback / Cleanup

- If prompts drift or agents begin inventing implementation details, reset by restoring the reference prompt templates in `docs/SPEC-KIT-900-generic-smoke/PRD.md` (below).
- If routing experiments leave lingering cost summaries, remove them with `rm docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/SPEC-KIT-900_cost_summary.json` before the next run to avoid confusion.
