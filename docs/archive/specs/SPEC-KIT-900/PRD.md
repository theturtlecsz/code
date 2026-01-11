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
| FR1 | Provide canonical prompts for plan/tasks/validate stages. | Prompts stored below; reviewers confirm wording. |
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

Tech Stack (Baseline):
- Language/Runtime: Rust with Tokio async runtime
- API Framework: Axum (HTTP REST JSON API)
- Storage: SQLite (local persistence, no external DB)
- Scheduler: In-process deterministic scheduler (no external queue)
- Endpoints: /reminders (CRUD), /sync (POST), /healthz (GET)

Produce:
- A three-milestone timeline (Design, Build, Validation) with owners and durations.
- A risk register listing at least three risks (technical / process / operational) and mitigations.
- Success metrics capturing latency, adoption proxy, and telemetry coverage.
- Assumptions and explicit non-goals.

Constraints: two-week delivery window, no external vendor dependencies, must include telemetry + rollback.

Confidentiality: This is a synthetic benchmark workload. Do not include production identifiers, PII, secrets, or team-specific jargon.
```

### Tasks Stage Prompt

```
Decompose the reminder-sync microservice project (Rust + Axum + SQLite stack) into 8–12 implementation tasks.

For each task include:
- Task ID (T1, T2, etc.)
- Title and owner role
- Deliverable description
- Definition of done bullets
- Parallelizable: yes/no
- Dependencies: list task IDs
- Cross-team touchpoints: specify roles (UX, QA, Security, etc.)

Highlight dependencies, surface at least two cross-team touchpoints (e.g., UX review, QA), and flag any tasks requiring security review.

Task Metadata Schema: {id, title, owner, deliverable, dod[], parallelizable, dependencies[], touchpoints[]}
```

### Validate Stage Prompt

```
Create a validation plan for the reminder-sync microservice (Rust + Axum + SQLite) covering:
- Unit, integration, and synthetic load tests (include tooling).
- Monitoring metrics + alert thresholds.
- Rollback procedure and success/failure criteria.
- Estimated runtime cost of the validation suite.

Validation Thresholds:
- Latency: p95 ≤ 200ms @ 50 RPS (local)
- Error Rate: < 1% over 5 minute window
- Resource: RAM < 256MB steady state, CPU < 80% peak
- Rollback Trigger: Sustained error rate ≥ 1% OR p95 > 200ms for 5 minutes

Output sections for Test Matrix, Observability, Rollback, and Launch Readiness Checklist.
```

**Reference Prompts Source**: These prompts are normative and versioned with the SPEC. Copy directly when agents need clarification. Prompt versions tracked in `docs/spec-kit/prompts.json` (embedded at compile-time via `tui/src/spec_prompts.rs`).

---

## 5. Consensus and Evidence Standards

### Consensus Definition

**Agreement Threshold**: ≥90% substantive agreement on conclusions and recommendations.

**Measurement**:
- 3/3 agents participate and produce outputs without conflicting recommendations → "ok"
- 2/3 agents participate OR minor wording variations with same conclusions → "degraded" (acceptable)
- Conflicting recommendations OR <2 agents → "conflict" or "no-consensus" (blocks advancement)

**Consensus Verdict Schema**:
```json
{
  "consensus_ok": boolean,
  "agreement_percent": number,
  "participants": ["gemini", "claude", "code"],
  "summary": "string describing convergence",
  "conflicts": [],
  "degraded": boolean
}
```

### Cost Summary Schema

**File**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/SPEC-KIT-900_cost_summary.json`

**Schema** (version 1):
```json
{
  "schemaVersion": 1,
  "spec_id": "SPEC-KIT-900",
  "currency": "USD",
  "total_cost_usd": 2.71,
  "per_stage": {
    "plan": 0.08,
    "tasks": 0.10,
    "validate": 0.35,
    "implement": 0.11,
    "audit": 0.80,
    "unlock": 0.80
  },
  "breakdown": [
    {
      "stage": "plan",
      "agent": "gemini",
      "input_tokens": 1200,
      "output_tokens": 800,
      "cost_usd": 0.03
    }
  ]
}
```

**Writer Contract**: Stage consensus finalizer updates `per_stage.*`; unlock stage computes `total_cost_usd`.

### Guardrail Script Interface

**Path**: `scripts/spec-kit/guardrail_check.sh` (to be created)

**Exit Codes**:
- 0 = Pass (all checks succeed)
- 1 = Warning (degraded mode, e.g., MCP offline)
- 2 = Fail (hard blocker, e.g., missing tools)

**JSON Output** (stdout):
```json
{
  "mcp_ok": true,
  "tools": {
    "ace": true,
    "ripgrep": true,
    "codegraphcontext": false,
    "hal": true
  },
  "notes": "CodeGraphContext unavailable but not required for this SPEC"
}
```

**Evidence**: Copy written to `evidence/commands/SPEC-KIT-900/tasks_guardrail.json`

---

## 6. QA Checklist

- [ ] Outputs contain only generic terminology ("platform engineer", "reminder service") and no internal project codenames.
- [ ] Plan includes timeline, risks, success metrics, and non-goals.
- [ ] Task list counts between 8 and 12 items with clear parallelisation flags.
- [ ] Validation plan enumerates tests, monitoring, rollback, and cost estimate.
- [ ] Evidence directories populated (`commands/`, `consensus/`, `costs/`).
- [ ] `cost_summary.json` shows three stage entries and total spend (matches schema above).

---

## 7. Rollout & Maintenance

- Store this SPEC under version control (already in `docs/SPEC-KIT-900-generic-smoke/`).
- Update prompts if major routing changes demand different agent scaffolding; log changes in `CHANGELOG.md` (future extension).
- When a run is complete and data captured, archive evidence with `./scripts/evidence_archive.sh --spec SPEC-KIT-900` or remove via `rm` to keep the repository tidy.

---

## 8. Open Questions

1. Should we add a dedicated `/speckit.auto` scenario (Implement + Validate) for longer benchmarks? — TBD.
2. Do we need a CI harness to run this nightly with stubbed agents? — Out of scope for now.

---

Back to [Key Docs](../KEY_DOCS.md)
