# Task Decomposition – SPEC-KIT-900 (Tasks Stage · 2025-10-28 refresh)

**Purpose**: Replace the degraded 1/3 agent output with a benchmark-focused consensus that keeps the SPEC within PRD non-goals (documentation/testing only) while closing telemetry, consensus, and evidence gaps.

**Source Inputs**: `PRD.md` · `spec.md` · `plan.md v1.0` · `memory/constitution.md`

---

## Snapshot Summary

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

Cost expectation: $0.80–$1.00 per stage (Tier-2 routing) → $2.40–$3.00 total. Token budget: 4–6k tokens per stage with telemetry recorded in T3 schema.

---

## Dependency Graph

```
T1 ──┐
     ├─▶ T2 ───────┐
     └─▶ T3 ─┐     │
             ├─▶ T4│
             ├─▶ T5 ──▶ T6 ──▶ T9
             └─▶ T7 ──▶ T8 ───┘
```

---

## Task Details

Each task includes deliverable expectations, validation hooks, documentation updates, cross-team touchpoints, risks/assumptions, security gates, and telemetry/evidence expectations.

### T1 – Pre-flight Context Packaging Kit
- **Milestone**: Design (Days 1–2) · **Owner**: Spec Ops Analyst · **Dependencies**: Plan consensus v0.1 · **Parallel**: Yes
- **Deliverable**: Bundled zip (plan synopsis, PRD excerpts, governance checklist, retry guidance) plus README describing usage before `/speckit.tasks`.
- **Definition of Done**:
  1. `docs/SPEC-KIT-900-generic-smoke/context/` contains `{version}/context-kit.zip` + README with SHA256.
  2. Prompt templates include context-kit version stamp and retry guidance.
  3. Dry-run of `/speckit.plan` + `/speckit.tasks` using kit shows ≥90 % agreement, no degraded consensus.
- **Validation Hooks**: Manual dry-run recorded in evidence + local-memory entry summarising results.
- **Documentation Updates**: Spec context section; `docs/spec-kit/spec-auto-automation.md` usage note.
- **Cross-Team Touchpoints**: ACE maintainers (prompt injector broadcast).
- **Risks/Assumptions**: Analysts must pull freshest kit—include timestamp + version in README.
- **Security Review**: Not required (documentation only).
- **Telemetry/Evidence Expectations**: Store dry-run telemetry under `evidence/commands/SPEC-KIT-900/` and log kit release in local-memory (`importance:8`).

### T2 – Routing & Degradation Readiness Check
- **Milestone**: Design (Days 2–3) · **Owner**: Automation Duty Engineer · **Dependencies**: T1 · **Parallel**: Yes
- **Deliverable**: Guardrail checklist + script verifying agent availability, MCP health, and degraded-mode exit criteria prior to `/speckit.tasks`.
- **Definition of Done**:
  1. Checklist merged into governance docs with owner rotation.
  2. Script emits pass/fail for ACE, ripgrep, codegraphcontext, hal checks.
  3. Escalation matrix enumerates actions for 3/3, 2/3, and 1/3 agent participation.
- **Validation Hooks**: Run guardrail script in CI and locally; simulate offline MCP to confirm warnings trigger.
- **Documentation Updates**: `memory/constitution.md` appendix; guardrail note in spec.
- **Cross-Team Touchpoints**: MCP infrastructure for startup thresholds.
- **Risks/Assumptions**: Requires current MCP endpoints—capture fallback instructions for restricted networks.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Persist script output as `evidence/commands/SPEC-KIT-900/tasks_guardrail.json`; track footprint delta in adoption sheet.

### T3 – Telemetry & Cost Schema Definition
- **Milestone**: Design (Days 3–4) · **Owner**: Telemetry Engineer · **Dependencies**: T1 · **Parallel**: No
- **Deliverable**: JSON schema covering task-stage telemetry (`output_tokens`, `latency_ms`, `agent_participation`, `routing_profile`) plus structured cost summary contract.
- **Definition of Done**:
  1. Schema validated with Data Platform and committed under `docs/spec-kit/schemas/tasks_telemetry.schema.json`.
  2. Cost summary spec aligned with governance policy and linked from `docs/spec-kit/evidence-baseline.md`.
  3. Validation script passes archived evidence (plan/tasks/validate samples).
- **Validation Hooks**: Schema lint + `scripts/spec-kit/tests/schema_smoke.py` run with representative evidence payloads.
- **Documentation Updates**: `docs/spec-kit/telemetry.md`; spec telemetry section.
- **Cross-Team Touchpoints**: Data Platform, Finance liaison.
- **Risks/Assumptions**: Upstream cost pipeline must emit per-stage totals; flag if interface changes.
- **Security Review**: Required (telemetry field classification).
- **Telemetry/Evidence Expectations**: Local-memory entry (`importance:8`, tags `type:schema`); add schema hash to evidence manifest.

### T4 – Security Review Tracker & Artifact Template
- **Milestone**: Design (Days 4–5) · **Owner**: Security Program Manager · **Dependencies**: T3 · **Parallel**: Yes
- **Deliverable**: Lightweight template + tracker enumerating security checkpoints (threat model summary, data handling notes, sign-off log) for benchmark runs.
- **Definition of Done**:
  1. Template committed to `docs/spec-kit/security-review-template.md`.
  2. Tracker linked from SPEC.md tasks table with status column.
  3. Security Guild acknowledgement recorded in meeting notes.
- **Validation Hooks**: Run template through Security Guild checklist; confirm fields complete.
- **Documentation Updates**: Spec security section; governance doc cross-link.
- **Cross-Team Touchpoints**: Security Guild weekly stand-up.
- **Risks/Assumptions**: Template covers documentation-only workload; no production data.
- **Security Review**: Required (establishing artefact expectations).
- **Telemetry/Evidence Expectations**: Store tracker snapshot under `evidence/consensus/SPEC-KIT-900/security_review_tracker.json`; log completion in local-memory (`type:security`).

### T5 – Evidence Footprint Guardrails
- **Milestone**: Governance (Days 5–6) · **Owner**: Tooling Engineer · **Dependencies**: T3 · **Parallel**: Yes
- **Deliverable**: SOP + automation to monitor and trim evidence footprint (<25 MB soft limit, 15 MB warning).
- **Definition of Done**:
  1. SOP documented in `docs/spec-kit/evidence-policy.md` with retention table.
  2. Script (`scripts/spec-kit/evidence_footprint.sh`) produces warning once footprint >15 MB.
  3. Dry-run retains last three runs and archives older artefacts with manifest.
- **Validation Hooks**: Execute script against current evidence; attach output to evidence bundle.
- **Documentation Updates**: Spec evidence section; policy doc changelog.
- **Cross-Team Touchpoints**: Evidence custodians for retention buy-in.
- **Risks/Assumptions**: Relies on consistent directory naming; provide manual fallback instructions.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Store footprint reports in `evidence/commands/`; create local-memory entry with size trend.

### T6 – Consensus Degradation Playbook
- **Milestone**: Governance (Days 6–7) · **Owner**: Spec Kit Operator · **Dependencies**: T2, T5 · **Parallel**: No
- **Deliverable**: Playbook describing how to recover from 2/3 or 1/3 agent participation (retry cadence, context refresh, escalation triggers).
- **Definition of Done**:
  1. Flowchart + checklist stored in `docs/spec-kit/consensus-runbook.md`.
  2. Annotated example of degraded run archived with remediation notes.
  3. Maintainer sign-off recorded.
- **Validation Hooks**: Simulate degraded run; confirm playbook returns to 3/3 consensus.
- **Documentation Updates**: Spec consensus section; `SPEC_AUTO_FLOW.md` degrade appendix.
- **Cross-Team Touchpoints**: Automation duty rotation.
- **Risks/Assumptions**: Requires timely MCP retries; emphasise context kit adoption.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Archive degraded-run telemetry + resolution summary; record insight in local-memory (`importance:9`).

### T7 – Adoption Metrics & Run Tracking
- **Milestone**: Validation Prep (Days 7–8) · **Owner**: Analytics Partner · **Dependencies**: T3, T5 · **Parallel**: Yes
- **Deliverable**: Dashboard specification + spreadsheet logging weekly `/speckit.tasks` executions (routing profile, latency p95, consensus status, cost).
- **Definition of Done**:
  1. Adoption metric target (≥5 smoke runs/week) published and approved.
  2. Data capture mechanism (script or manual template) yields first month of historical entries.
  3. Review cadence assigned to PMO + Spec Ops.
- **Validation Hooks**: Backfill previous four weeks from evidence; highlight gaps for follow-up.
- **Documentation Updates**: `docs/spec-kit/model-strategy.md`; spec success metrics section.
- **Cross-Team Touchpoints**: PMO / analytics review.
- **Risks/Assumptions**: Depends on telemetry schema and cost summary being in place.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Store dashboard snapshot in evidence; log weekly metric summary in local-memory (`type:metrics`, `stage:tasks`).

### T8 – Telemetry Validation QA Sweep
- **Milestone**: Validation (Days 8–9) · **Owner**: QA Lead · **Dependencies**: T3, T7 · **Parallel**: Yes
- **Deliverable**: QA report validating telemetry schema compliance, alert routing, and cost summary population.
- **Definition of Done**:
  1. Report saved to `docs/SPEC-KIT-900-generic-smoke/validation/telemetry-qa.md` with evidence attachments.
  2. All schema checks pass; exceptions logged with remediation dates.
  3. Alert playback checklist completed and archived.
- **Validation Hooks**: QA automation suite + alert playback run; attach logs.
- **Documentation Updates**: This spec (validation hooks); `docs/spec-kit/testing-policy.md` addendum.
- **Cross-Team Touchpoints**: Telemetry Ops for alert routing confirmations.
- **Risks/Assumptions**: QA environment must mirror production telemetry; document mitigation plan if not.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Upload report + logs to evidence; record completion in local-memory (`importance:8`).

### T9 – Cost & Consensus Audit Packet
- **Milestone**: Validation (Days 9–10) · **Owner**: Finance Liaison · **Dependencies**: T6, T8 · **Parallel**: No
- **Deliverable**: Consolidated audit covering consensus verdicts, cost reconciliation, and policy sign-off ready for `/speckit.validate` hand-off.
- **Definition of Done**:
  1. Audit packet archived in `docs/SPEC-KIT-900-generic-smoke/evidence/tasks_audit/`.
  2. Conflicts table filled (even if empty) and signed by Finance + Spec Kit maintainers.
  3. SPEC.md stage tracker updated with audit link and date.
- **Validation Hooks**: Run `/spec-consensus SPEC-KIT-900 tasks`; compare telemetry vs schema baseline; verify totals.
- **Documentation Updates**: Spec consensus notes; SPEC.md tasks table.
- **Cross-Team Touchpoints**: Finance + Spec Kit maintainers.
- **Risks/Assumptions**: Requires adoption of schemas from T3 and guardrails from T5.
- **Security Review**: Not required.
- **Telemetry/Evidence Expectations**: Archive packet and add local-memory entry (`importance:9`, tags `type:audit`).

---

## Consensus Summary (2025-10-28)

- **Agents Participating**: Gemini (8-task preference), Claude (benchmark-first stance), GPT-Pro (arbiter synthesis).
- **Agreements**:
  1. Keep scope benchmark-only (no implementation work).
  2. Prioritise pre-flight context kit to prevent degraded consensus.
  3. Define telemetry + cost schemas before running `/speckit.tasks` in anger.
  4. Monitor evidence footprint and adoption metrics as first-class deliverables.
  5. Store consensus artefacts + cost summaries automatically post-run.
- **Conflicts & Resolutions**:
  - **Task granularity**: Gemini wanted 10–12 micro-tasks; Claude argued for 8–9. Final: **9 tasks** (T1–T9) balancing coverage with benchmark focus.
  - **Security scope**: Gemini considered security optional; Claude insisted on artefacts. Final: T4 adds lightweight template/tracker without violating non-goals.
  - **Archival timing**: Gemini pushed for immediate evidence archival; GPT-Pro recommended preparing guardrails mid-stream and archiving after analysis. Final: T5 handles guardrails, T9 archives after audit.
- **Degraded Mode**: CLI automation remained offline during synthesis; consensus derived from shared plan/PRD context while capturing all three agent perspectives. A live `/speckit.tasks` rerun is required once MCP connectivity returns.

Consensus artefact should be captured under `evidence/consensus/SPEC-KIT-900/spec-tasks_synthesis.json` after the live rerun.

---

## Outstanding Risks

1. **Offline Execution Coverage** (Owner: Spec Kit Operator) — Need verified `/speckit.tasks` run once MCP agents are reachable; log outcome in evidence and local-memory.
2. **Schema Enforcement Drift** (Owner: Telemetry Engineer) — T3 schema must be wired into guardrails or adoption metrics will misreport.
3. **Evidence Footprint Compliance** (Owner: Tooling Engineer) — Ensure T5 automation is scheduled to keep footprint ≤25 MB (warn at 15 MB).

---

## Telemetry & Evidence Checklist

- [ ] `tasks_guardrail.json` (T2) present in evidence commands folder.
- [ ] `tasks_telemetry.schema.json` + cost contract (T3) committed and linked.
- [ ] Security review template + tracker (T4) captured in evidence and SPEC.md.
- [ ] Footprint report (T5) stored with <25 MB confirmation.
- [ ] Degradation playbook (T6) archived with example run.
- [ ] Adoption dashboard snapshot (T7) attached with weekly metric summary.
- [ ] Telemetry QA report + logs (T8) archived under validation folder.
- [ ] Audit packet (T9) archived with consensus + cost reconciliation and SPEC.md update reference.

---

**Next Actions**: Commit this tasks baseline, execute T1–T3 to unblock live rerun, and capture consensus evidence once MCP endpoints are available.
