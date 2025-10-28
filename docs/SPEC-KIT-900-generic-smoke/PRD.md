# Product Requirements Document – SPEC-KIT-900 Generic Smoke Scenario

**Purpose**: Supply a reusable, anonymised workload that runs the full Spec-Kit pipeline for benchmarking (latency, model routing, cost, consensus quality) after infrastructure changes. The scenario must remain simple, deterministic, and free of team-specific context so it can be executed repeatedly without review cycles.

---

## 1. Problem Statement

- SPEC-KIT-070 and future routing experiments require a neutral baseline SPEC to compare premium vs. cheap-model runs.
- Current validation often reuses the SPEC being modified, creating circular dependencies and biased prompts.
- We need a stable script that any engineer can run to gather cost/quality evidence without touching roadmap artefacts.

### Goals

1. Provide canonical prompts for `/speckit.plan`, `/speckit.tasks`, `/speckit.validate` that yield medium-complexity outputs.
2. Ensure each stage generates consensus artefacts, local-memory entries, and cost summaries by default.
3. Keep content generic enough that reviewers can sign off quickly, yet structured enough to exercise agent reasoning.

### Non-Goals

- No implementation instructions beyond documentation/testing.
- No deployment or integration with production services.
- No dependency on organisation-specific terminology.

---

## 2. Functional Requirements

| ID | Requirement | Validation |
|----|-------------|------------|
| FR1 | Provide canonical prompts for plan/tasks/validate stages. | Prompts stored below and mirrored in spec.md; reviewers confirm wording. |
| FR2 | Drive multi-agent output of 4–6k tokens per stage. | Measure token usage in `~/.code/logs/codex-tui.log` (or cost summary) ≥ specified range. |
| FR3 | Generate consensus artefacts and cost summary automatically. | After run, `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-900/` and `costs/SPEC-KIT-900_cost_summary.json` exist. |
| FR4 | Achieve ≥90% agent agreement in standard routing. | Consensus verdicts show `consensus_ok: true` and conflicts array empty when using reference configuration. |
| FR5 | Outputs remain free of confidential data or team-specific jargon. | Manual spot check (QA checklist below). |

---

## 3. Non-Functional Requirements

- **Repeatability**: Prompts must produce comparable structure even if wording evolves (target variance <10% sections across runs).
- **Simplicity**: Analysts should spend <5 minutes setting up and executing the spec.
- **Observability**: Every run yields cost, consensus, and local-memory artefacts without manual intervention.
- **Portability**: Scenario should work regardless of routing (premium-only or cheap-tier) and on both developer laptops and CI smoke environments.

---

## 4. Reference Prompts

### Plan Stage Prompt

```
You are drafting a lightweight product plan for a cross-device reminder sync microservice.
Produce:
- A three-milestone timeline (Design, Build, Validation) with owners and durations.
- A risk register listing at least three risks (technical / process / operational) and mitigations.
- Success metrics capturing latency, adoption proxy, and telemetry coverage.
- Assumptions and explicit non-goals.

Constraints: two-week delivery window, no external vendor dependencies, must include telemetry + rollback.
```

### Tasks Stage Prompt

```
Decompose the reminder-sync microservice project into 8–12 implementation tasks.
For each task include:
- Title and owner role.
- Deliverable description.
- Definition of done bullets.
- Whether it can run in parallel.

Highlight dependencies, surface at least two cross-team touchpoints (e.g., UX review, QA), and flag any tasks requiring security review.
```

### Validate Stage Prompt

```
Create a validation plan for the reminder-sync microservice covering:
- Unit, integration, and synthetic load tests (include tooling).
- Monitoring metrics + alert thresholds.
- Rollback procedure and success/failure criteria.
- Estimated runtime cost of the validation suite.

Output sections for Test Matrix, Observability, Rollback, and Launch Readiness Checklist.
```

Prompts may be copy-pasted directly when running the TUI if agents require clarification.

---

## 5. QA Checklist

- [ ] Outputs contain only generic terminology ("platform engineer", "reminder service") and no internal project codenames.
- [ ] Plan includes timeline, risks, success metrics, and non-goals.
- [ ] Task list counts between 8 and 12 items with clear parallelisation flags.
- [ ] Validation plan enumerates tests, monitoring, rollback, and cost estimate.
- [ ] Evidence directories populated (`commands/`, `consensus/`, `costs/`).
- [ ] `cost_summary.json` shows three stage entries and total spend.

---

## 6. Rollout & Maintenance

- Store this SPEC under version control (already in `docs/SPEC-KIT-900-generic-smoke/`).
- Update prompts if major routing changes demand different agent scaffolding; log changes in `CHANGELOG.md` (future extension).
- When a run is complete and data captured, archive evidence with `./scripts/evidence_archive.sh --spec SPEC-KIT-900` or remove via `rm` to keep the repository tidy.

---

## 7. Open Questions

1. Should we add a dedicated `/speckit.auto` scenario (Implement + Validate) for longer benchmarks? — TBD.
2. Do we need a CI harness to run this nightly with stubbed agents? — Out of scope for now.

