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

## Task Decomposition (Tasks Stage · 2025-10-28 refresh)

> Full task briefs, dependency graph, and consensus transcript live in `docs/SPEC-KIT-900-generic-smoke/tasks.md`. The table below provides a quick reference for stage orchestration.

| Task | Title | Milestone | Owner | Dependencies | Parallel? |
|------|-------|-----------|-------|--------------|-----------|
| T1 | Pre-flight Context Packaging Kit | Design | Spec Ops Analyst | Plan consensus v0.1 | ✅ |
| T2 | Routing & Degradation Readiness Check | Design | Automation Duty Engineer | T1 | ✅ |
| T3 | Telemetry & Cost Schema Definition | Design | Telemetry Engineer | T1 | ❌ |
| T4 | Security Review Tracker & Artifact Template | Design | Security Program Manager | T3 | ✅ |
| T5 | Evidence Footprint Guardrails | Governance | Tooling Engineer | T3 | ✅ |
| T6 | Consensus Degradation Playbook | Governance | Spec Kit Operator | T2, T5 | ❌ |
| T7 | Adoption Metrics & Run Tracking | Validation Prep | Analytics Partner | T3, T5 | ✅ |
| T8 | Telemetry Validation QA Sweep | Validation | QA Lead | T3, T7 | ✅ |
| T9 | Cost & Consensus Audit Packet | Validation | Finance Liaison | T6, T8 | ❌ |

### T1 – Pre-flight Context Packaging Kit
- **Milestone**: Design (Days 1–2) · **Owner**: Spec Ops Analyst · **Dependencies**: Plan consensus v0.1 · **Parallel**: Yes
- **Deliverable**: Zip + README bundling plan synopsis, PRD excerpts, governance checklist, and retry guidance for `/speckit.tasks` runs.
- **Definition of Done**: (1) Context kit published under `docs/SPEC-KIT-900-generic-smoke/context/`; (2) Retry guidance embedded in prompts with version stamp; (3) Dry-run shows no degraded consensus when kit supplied.
- **Validation Hooks**: `/speckit.plan` + `/speckit.tasks` dry-run using kit; record degradation metrics.
- **Documentation Updates**: Update this spec (context section) and `docs/spec-kit/spec-auto-automation.md` with kit usage note.
- **Cross-Team Touchpoints**: ACE bulletin update for prompt injectors.
- **Risks/Assumptions**: Analysts must download the latest kit—timestamp release notes to minimise drift.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Store kit release note in local-memory (`importance:8`, tags `spec:SPEC-KIT-900`, `stage:tasks`); archive dry-run telemetry under `evidence/commands/SPEC-KIT-900/`.

### T2 – Routing & Degradation Readiness Check
- **Milestone**: Design (Days 2–3) · **Owner**: Automation Duty Engineer · **Dependencies**: T1 · **Parallel**: Yes
- **Deliverable**: Checklist + scripted sanity run verifying agent availability, MCP health, and degraded-mode exit criteria before `/speckit.tasks` executions.
- **Definition of Done**: (1) Checklist merged into governance docs; (2) Script reports pass/fail for ACE, ripgrep, codegraphcontext, hal; (3) Escalation matrix defined for degraded consensus.
- **Validation Hooks**: Run guardrail script; simulate offline MCP to ensure warnings fire; capture output in evidence.
- **Documentation Updates**: `memory/constitution.md` governance appendix; guardrail reference in this spec.
- **Cross-Team Touchpoints**: MCP infrastructure team for startup thresholds.
- **Risks/Assumptions**: Requires up-to-date MCP endpoints; document fallback path for restricted networks.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Log script telemetry to `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-900/tasks_guardrail.json` and add footprint impact to monitoring sheet.

### T3 – Telemetry & Cost Schema Definition
- **Milestone**: Design (Days 3–4) · **Owner**: Telemetry Engineer · **Dependencies**: T1 · **Parallel**: No
- **Deliverable**: JSON schema for task-stage telemetry (`output_tokens`, `latency_ms`, `agent_participation`) plus cost summary contract aligned with governance policy.
- **Definition of Done**: (1) Schema reviewed with Data Platform; (2) Validation script passes sample logs; (3) Cost summary spec cross-referenced in `docs/spec-kit/evidence-baseline.md`.
- **Validation Hooks**: Schema lint + `scripts/spec-kit/tests/schema_smoke.py` dry-run against archived evidence.
- **Documentation Updates**: `docs/spec-kit/telemetry.md`; link schema in this spec.
- **Cross-Team Touchpoints**: Data Platform and Finance liaison.
- **Risks/Assumptions**: Assumes cost pipeline produces per-stage totals; flag if upstream API shifts.
- **Security Review**: Required (telemetry data classification).
- **Telemetry/Evidence Expectations**: Publish schema under `docs/spec-kit/schemas/tasks_telemetry.schema.json`; log approval in local-memory (`importance:8`).

### T4 – Security Review Tracker & Artifact Template
- **Milestone**: Design (Days 4–5) · **Owner**: Security Program Manager · **Dependencies**: T3 · **Parallel**: Yes
- **Deliverable**: Template + tracker enumerating required security checkpoints (threat model summary, data handling notes, sign-off log) for benchmark runs.
- **Definition of Done**: (1) Template committed to `docs/spec-kit/security-review-template.md`; (2) Tracker integrated into SPEC.md tasks table; (3) Security Guild acknowledgement recorded.
- **Validation Hooks**: Run template through security checklist review; verify required fields present.
- **Documentation Updates**: This spec (security section); governance doc cross-link.
- **Cross-Team Touchpoints**: Security Guild weekly stand-up.
- **Risks/Assumptions**: Template focuses on documentation-only workload—no production data.
- **Security Review**: Required (establishing review artefact).
- **Telemetry/Evidence Expectations**: Record review outcomes in `evidence/consensus/SPEC-KIT-900/security_review_tracker.json` and local-memory (`type:security`).

### T5 – Evidence Footprint Guardrails
- **Milestone**: Governance (Days 5–6) · **Owner**: Tooling Engineer · **Dependencies**: T3 · **Parallel**: Yes
- **Deliverable**: Cleanup SOP + automated footprint report enforcing the 25 MB evidence ceiling (warn at 15 MB).
- **Definition of Done**: (1) SOP documented in `docs/spec-kit/evidence-policy.md`; (2) Script outputs warning once footprint >15 MB; (3) Dry-run retains last three runs while archiving older data.
- **Validation Hooks**: Execute footprint script against existing evidence; attach report.
- **Documentation Updates**: Update this spec (evidence section) and policy doc; add status line to SPEC.md tracker.
- **Cross-Team Touchpoints**: Evidence custodians for archival retention.
- **Risks/Assumptions**: Requires consistent evidence directory naming; document manual fallback.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Store SOP history in local-memory (`importance:8`) and attach footprint metrics to `evidence/commands/` bundle.

### T6 – Consensus Degradation Playbook
- **Milestone**: Governance (Days 6–7) · **Owner**: Spec Kit Operator · **Dependencies**: T2, T5 · **Parallel**: No
- **Deliverable**: Playbook detailing recovery actions for 2/3 or 1/3 agent participation, including retry cadence, context refresh, and escalation triggers.
- **Definition of Done**: (1) Flowchart + step list stored in `docs/spec-kit/consensus-runbook.md`; (2) Example degraded run annotated; (3) Maintainer sign-off captured.
- **Validation Hooks**: Simulate degraded run via sandbox logs; confirm playbook restores full consensus.
- **Documentation Updates**: This spec (consensus notes) and `SPEC_AUTO_FLOW.md` degrade section.
- **Cross-Team Touchpoints**: Automation duty rotation for sign-off.
- **Risks/Assumptions**: Depends on timely MCP retries; emphasise context kit adoption.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Capture degraded-run telemetry and resolution summary in evidence plus local-memory (`importance:9`).

### T7 – Adoption Metrics & Run Tracking
- **Milestone**: Validation Prep (Days 7–8) · **Owner**: Analytics Partner · **Dependencies**: T3, T5 · **Parallel**: Yes
- **Deliverable**: Dashboard spec + spreadsheet logging weekly `/speckit.tasks` executions with routing profile, latency p95, and consensus outcome.
- **Definition of Done**: (1) Adoption metric published (≥5 runs/week target); (2) Data capture automated or templated; (3) Review cadence assigned.
- **Validation Hooks**: Backfill last four weeks of runs from evidence; verify thresholds highlight gaps.
- **Documentation Updates**: `docs/spec-kit/model-strategy.md` adoption section; add dashboard link here.
- **Cross-Team Touchpoints**: PMO for adoption cadence.
- **Risks/Assumptions**: Relies on accurate telemetry schema; highlight if cost data missing.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Store dashboard snapshot in evidence and log metric summary in local-memory with tags `type:metrics`, `stage:tasks`.

### T8 – Telemetry Validation QA Sweep
- **Milestone**: Validation (Days 8–9) · **Owner**: QA Lead · **Dependencies**: T3, T7 · **Parallel**: Yes
- **Deliverable**: QA report validating telemetry schema compliance, alert routing, and cost summary population for the tasks stage.
- **Definition of Done**: (1) Report stored under `docs/SPEC-KIT-900-generic-smoke/validation/telemetry-qa.md`; (2) All schema checks pass; (3) Alert playback results logged.
- **Validation Hooks**: Execute QA automation + alert playback checklist; attach logs.
- **Documentation Updates**: Update this spec (validation hooks) and `docs/spec-kit/testing-policy.md`.
- **Cross-Team Touchpoints**: Telemetry Ops for alert routing confirmation.
- **Risks/Assumptions**: QA environment must mirror production telemetry; document mitigation plan for missing data.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Add QA report to evidence repository; log completion in local-memory (`importance:8`).

### T9 – Cost & Consensus Audit Packet
- **Milestone**: Validation (Days 9–10) · **Owner**: Finance Liaison · **Dependencies**: T6, T8 · **Parallel**: No
- **Deliverable**: Consolidated audit including consensus verdict summary, cost reconciliation, and policy sign-off checklist ready for `/speckit.validate` hand-off.
- **Definition of Done**: (1) Audit packet archived under `docs/SPEC-KIT-900-generic-smoke/evidence/tasks_audit/`; (2) Conflicts table completed (even if empty); (3) SPEC.md status updated with audit link.
- **Validation Hooks**: Run `/spec-consensus SPEC-KIT-900 tasks` to confirm no conflicts; compare cost telemetry vs schema baseline.
- **Documentation Updates**: Update this spec (consensus notes) and `SPEC.md` stage tracker.
- **Cross-Team Touchpoints**: Finance + Spec-Kit maintainers for approval signatures.
- **Risks/Assumptions**: Depends on telemetry schema adoption; highlight any unresolved deltas.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Archive audit results and attach summary to local-memory with `importance:9`, `type:audit`.

### Consensus & Agent Notes
- **Agent Participation**: Gemini, Claude, and GPT-Pro delivered task proposals; CLI automation remained offline, so synthesis used shared plan/PRD context while capturing 3/3 perspectives.
- **Agreements**: All agents aligned on benchmark-only scope, need for pre-flight context packaging, telemetry/cost schema ownership, and consensus reliability safeguards before `/speckit.validate`.
- **Conflicts/Divergence**:
  - *Task count & scope*: Gemini preferred 10–12 granular items; Claude advocated 8–9; GPT-Pro emphasised keeping work documentation-focused. Resolved at nine tasks that satisfy orchestration coverage without reintroducing implementation work.
  - *Security gate breadth*: Claude argued governance requires explicit artefacts; Gemini deemed security unnecessary for synthetic data. Added T4 to balance minimal template work with auditability.
  - *Evidence archival timing*: Gemini pushed for immediate archival; Claude suggested deferring; GPT-Pro proposed prepping guardrails mid-stream and archiving post-analysis. Adopted GPT-Pro’s middle ground (T5 + T9).
- **Follow-ups**: Schedule a live `/speckit.tasks` rerun once MCP endpoints are reachable to confirm automation succeeds with the new context kit; store this consensus in local-memory (`importance:9`).

### Outstanding Risks After Tasks Stage
1. **Offline Execution Coverage**: Without a verified live run, `/speckit.tasks` must be re-executed once MCP connectivity is restored (owner: Spec Kit Operator).
2. **Schema Enforcement Drift**: Telemetry schema (T3) must be wired into guardrails or adoption metrics (T7) will degrade (owner: Telemetry Engineer).
3. **Evidence Footprint Compliance**: Guardrail script (T5) needs continuous monitoring to keep archives below the 25 MB policy ceiling (owner: Tooling Engineer).

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
