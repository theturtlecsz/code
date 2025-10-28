# Plan: Generic Multi-Agent Smoke Scenario

**SPEC-ID**: SPEC-KIT-900-generic-smoke
**Plan Version**: v0.1
**Created**: 2025-10-28

---

## Inputs

**Spec**: docs/SPEC-KIT-900-generic-smoke/spec.md (hash: efc0380066521dc53cfc340b098cb3f21c143298099a6dd4181eb2ca4f9156dc)
**Constitution**: memory/constitution.md (v1.1)
**PRD**: docs/SPEC-KIT-900-generic-smoke/PRD.md
**Prompt Version**: 20251002-plan-a

---

## Work Breakdown

### Step 1: Calibrate Reference Prompts

**Description**: Review and copy the canonical plan/tasks/validate prompts, confirm wording against PRD §4, and capture baseline expectations (timeline, risks, metrics, QA checklist).

**Dependencies**: None

**Success Signal**: Prompts validated and pasted into `/speckit.plan` dry run notes without edits; QA checklist ready.

**Owner**: Platform Product Owner

**Estimated Effort**: 0.5 day

### Step 2: Generate Plan Stage Baseline

**Description**: Execute `/speckit.plan SPEC-KIT-900` using reference routing, ensure three-milestone plan, risk register, metrics, assumptions, and non-goals are produced, and store consensus artifacts to evidence + local-memory.

**Dependencies**: Step 1

**Success Signal**: Plan consensus verdict shows `consensus_ok: true`, `conflicts: []`; cost summary includes `per_stage.plan`.

**Owner**: Staff Platform Engineer

**Estimated Effort**: 1 day

### Step 3: Decompose Tasks and Instrument Telemetry

**Description**: Run `/speckit.tasks SPEC-KIT-900`, verify 8–12 tasks with owners, parallelisation flags, and cross-team touchpoints; ensure telemetry emits task-stage artifacts and archives to evidence paths.

**Dependencies**: Step 2

**Success Signal**: Tasks consensus artifact stored; telemetry JSON lists all tasks with dependency graph and at least two cross-team checks.

**Owner**: Backend Engineer partnering with Telemetry Analyst

**Estimated Effort**: 3 days

### Step 4: Validate Scenario Observability

**Description**: Execute `/speckit.validate SPEC-KIT-900`, ensuring validation plan covers unit/integration/load tests, monitoring metrics with alert thresholds, rollback, and cost estimate; review telemetry for schema v1 compliance.

**Dependencies**: Step 3

**Success Signal**: Validate consensus artifact stored with zero conflicts; cost summary gains `per_stage.validate`; monitoring + rollback sections satisfy QA checklist.

**Owner**: QA Lead with SRE Partner

**Estimated Effort**: 3 days

### Step 5: Benchmark, Archive, and Cleanup

**Description**: Run cheap vs premium routing comparisons (as needed), analyse consensus latency + cost deltas, archive baseline evidence, and prune transient artifacts per evidence policy.

**Dependencies**: Step 4

**Success Signal**: Baseline archive created under `docs/SPEC-KIT-900-generic-smoke/baseline/`; evidence footprint <15 MB; SPEC.md updated with links and outcomes.

**Owner**: SRE Partner

**Estimated Effort**: 2.5 days

---

## Technical Design

### Data Model Changes

**Entity**: ReminderSyncWorkload
**Changes**:
- Add field: `scenario_id` (STRING) to tag smoke-run sessions deterministically.
- Modify: `cost_summary` aggregation to include per-routing-profile deltas.
- Remove: _n/a_ (baseline scenario uses ephemeral storage only).

**Migration**: Initialise synthetic reminder payloads via fixture script; no production data touched.

### API Contracts

**Endpoint**: `/api/reminder-sync/v1/reminders`
**Method**: POST
**Request**:
```json
{
  "device_id": "string",
  "reminder_payload": {
    "title": "string",
    "trigger_at": "ISO8601",
    "priority": "normal|high"
  },
  "scenario_id": "SPEC-KIT-900"
}
```
**Response**:
```json
{
  "status": "accepted",
  "sync_id": "uuid",
  "estimated_latency_ms": 120
}
```

**Error Cases**:
- 400: Invalid payload (missing `device_id` or malformed timestamp).
- 409: Duplicate sync detected for same `scenario_id` and `device_id`.
- 500: Telemetry pipeline unavailable; emit alert and abort run.

### Component Architecture

**New Components**:
- `ReminderSyncStub`: Deterministic microservice responding to plan/tasks/validate prompts.
- `TelemetryIngestor`: Collects consensus metrics, latency, and cost per stage.

**Modified Components**:
- `SpecKitEvidenceWriter`: Ensure SPEC-KIT-900 runs deposit artifacts in scenario-specific folders.

**Interactions**:
```
ReminderSyncStub → TelemetryIngestor → EvidenceStore
                                     ↘ CostSummariser
```

---

## Acceptance Mapping

| Requirement (from Spec/PRD) | Validation Step | Test/Check Artifact |
|-----------------------------|-----------------|---------------------|
| FR1: Canonical prompts documented | Confirm PRD §4 prompts unchanged during Step 1 | QA checklist signed (docs/SPEC-KIT-900-generic-smoke/PRD.md §5)
| FR2: 4–6k token output per stage | Inspect telemetry `output_tokens` after Steps 2–4 | docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/SPEC-KIT-900_cost_summary.json
| FR3: Consensus + cost artifacts generated | Verify evidence folders populated post Steps 2–4 | docs/SPEC-OPS-004-integrated-coder-hooks/evidence/{consensus,commands}/SPEC-KIT-900/
| FR4: ≥90% agent agreement | Read synthesis JSON for each stage; ensure `conflicts: []` | docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-900/*_synthesis.json
| FR5: Outputs stay generic | Run QA spot-check + keyword scan during Step 5 | QA report stored alongside baseline archive

---

## Risks & Unknowns

### Risk 1: Consensus Drift Across Runs

**Impact**: High

**Probability**: Medium

**Mitigation**: Capture three baseline runs, monitor variance (<10% section changes), lock router parameters, and document anomalies in local-memory (`importance:8`).

**Owner**: Staff Platform Engineer

### Risk 2: Agent Degradation or Timeouts

**Impact**: Medium

**Probability**: Medium

**Mitigation**: Enable built-in retry loop (3 attempts, exponential backoff), accept 2/3 consensus only with documented warning, and rerun stage if consensus missing.

**Owner**: Automation Duty Engineer

### Risk 3: Evidence Footprint Exceeds Policy

**Impact**: Low

**Probability**: Medium

**Mitigation**: Track evidence size with `/spec-evidence-stats`, archive baseline promptly, prune transient artifacts via `./scripts/evidence_archive.sh --spec SPEC-KIT-900`.

**Owner**: SRE Partner

---

## Multi-Agent Consensus

### Agreements

**All agents aligned on:**
- Two-week delivery window with three milestones (Design, Build, Validation).
- Emphasis on telemetry coverage, cost tracking, and consensus artifacts as primary success metrics.
- Need to keep prompts and outputs generic to remain reusable across routing experiments.

### Conflicts Resolved

**Issue**: Optimal allocation for Build vs Validation milestone duration.

**Positions**:
- Gemini: Proposed 3-5-2 day split prioritising rapid validation.
- Claude: Suggested longer Validation (4 days) to accommodate QA review.
- GPT-Pro: Balanced 3-6-3 distribution to absorb telemetry checks and routing comparisons.

**Resolution**: Adopt 3-day design, 6-day build, 3-day validation timeline (Steps 2–4) to stay within two-week window while leaving buffer for telemetry verification.

**Arbiter Decision**: GPT-Pro synthesis accepted as the midpoint that meets PRD constraints and risk mitigations.

---

## Exit Criteria

- [ ] All work breakdown steps completed
- [ ] Acceptance mapping validated (FR1–FR5 checks logged)
- [ ] Technical design artefacts reviewed with stakeholders
- [ ] Risks mitigated or formally accepted in local-memory entries
- [ ] Evidence artifacts created and archived per policy
- [ ] Consensus synthesis recorded in local-memory with importance ≥8
- [ ] Ready for /speckit.tasks stage

---

## Evidence References

**Plan Consensus**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-900/spec-plan_synthesis.json`

**Telemetry**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-900/spec-plan_[TIMESTAMP].json`

**Agent Outputs**: `.code/agents/[UUID]/result.txt` (gemini, claude, gpt_pro)

