# Plan: Radical Model Cost Optimization

**SPEC-ID**: SPEC-KIT-070
**Plan Version**: 20251026-plan-a
**Created**: 2025-10-26

---

## Inputs

**Spec**: docs/SPEC-KIT-070-model-cost-optimization/spec.md (hash: b9c1c205ea73944d202e7ca6b6fcf6b5afe2609be1ea4bef6428a8af8d88834f)
**Constitution**: memory/constitution.md (v1.1, amended 2025-10-26)
**PRD**: docs/SPEC-KIT-070-model-cost-optimization/PRD.md
**Prompt Version**: spec-plan/20251002-plan-a

---

## Work Breakdown

### Step 1: Phase 1 Validation & Telemetry Bring-Up

**Description**: Resume Phase 1 as soon as OpenAI rate limits expire. Validate Gemini Flash, Claude Haiku, and GPT-4o with existing prompts, capture cost deltas, and wire the in-memory `CostTracker` into `handler.rs` so every agent call records spend and emits `_cost_summary.json` in the evidence tree.

**Dependencies**: Provider rate limit reset (ETA 2025-10-26 23:30 UTC), existing cost-tracker unit tests, access to API billing dashboards.

**Success Signal**: (a) Cheap-model runs complete for `/speckit.plan`, `/speckit.tasks`, `/speckit.validate` with ≥90% consensus agreement in A/B review, (b) measured spend falls within the $5.5–6.6 Phase 1 target per `/speckit.auto`, (c) `CostTracker::record_agent_call` hooks fire for all stages and persist `cost_summary.json` artifacts, (d) no regression in 180 existing tests.

**Owner**: Code

**Estimated Effort**: 6–8 hours

### Step 2: Phase 2 – Cost Tracker Integration Hardening

**Description**: Promote the instrumentation from Step 1 into guarded production paths: enforce per-SPEC budgets, surface Warning/Critical alerts inside the TUI, and add snapshot telemetry for `spec-plan`/`spec-tasks` commands. Extend evidence schema to capture pricing metadata and link to provider invoices.

**Dependencies**: Step 1 completed, `docs/spec-kit/testing-policy.md` guidance for telemetry, evidence locking in `evidence.rs`.

**Success Signal**: (a) Budget alerts surface in-line for `/speckit.auto`, (b) telemetry JSON includes `per_stage` and `per_model` breakdowns, (c) regression tests for budget thresholds (new 4–6 tests) pass, (d) documentation updated in `PHASE1_COMPLETE.md` and `PHASE2_COMPLEXITY_ROUTING.md` to describe new evidence fields.

**Owner**: Code

**Estimated Effort**: 3 hours

### Step 3: Phase 3 – Complexity-Based Routing & Implement Refactor

**Description**: Classify all 13 `/speckit.*` commands into Tier S/M/C/X, introduce a routing table (cheap-primary + validator + aggregator selection), and refactor `/speckit.implement` to drop from 4 premium models to “premium + validator” with fallback escalation. Update prompts where cheaper models need different scaffolding.

**Dependencies**: Steps 1–2 complete, `docs/spec-kit/prompts.json` loaded via `include_str!`, consensus retry harness (`handler.rs::run_consensus_with_retry`).

**Success Signal**: (a) Routing matrix stored in config and exercised via automated integration test that walks all commands, (b) `/speckit.implement` cost shrinks from ~$8 → ≤$1.5 while maintaining validation success ≥95%, (c) total `/speckit.auto` run lands in the $2–3 band, (d) prompts compile for all selected models with no MCP regressions.

**Owner**: Code

**Estimated Effort**: 15–20 hours

### Step 4: Phase 4 – Dynamic Optimization & Production Rollout

**Description**: Layer on “try-cheap-first” logic with automatic escalation when validators flag quality issues, add dashboards for cost/quality drift, and finalize production readiness (playbooks, runbooks, and rollback toggles). Validate across 3 real SPECs before merge.

**Dependencies**: Steps 1–3 complete, evidence footprint within 25 MB soft limit, guardrails from `guardrail.rs`.

**Success Signal**: (a) Escalation path proven via injected failures, (b) Grafana-style cost dashboard (or TUI panel) surfaced with per-stage KPIs, (c) final `/speckit.auto` telemetry shows $1–3 cost with ≥90% consensus, (d) release checklist signed in SPEC.md + SPEC-KIT-070 docs.

**Owner**: Code

**Estimated Effort**: 8 hours

---

## Technical Design

### Data Model Changes

**Entity**: `SpecCostTracker`
- Track `spec_id`, `budget`, `spent`, `per_stage`, `per_model`, `call_count`, `alerts`.
- Extend with `pricing_version` and `provider_exchange_rate` to capture billing drift.

**Entity**: `TaskComplexity`
- Enum `{Simple, Medium, Complex, Critical}` stored within `handler.rs` routing table; future-proof with `Custom(ModelProfile)` variant for future experimentation.

**Entity**: `ModelProfile`
- Struct describing `model_id`, `provider`, `reasoning_mode`, price bands, and validator relationships.

**Migration**: None required (pure runtime structs with evidence serialization); ensure new JSON schema version field defaults to `1` when persisting summaries.

### API Contracts

**Function**: `CostTracker::record_agent_call(spec_id, stage, agent, model, input_tokens, output_tokens) -> (f64, Option<BudgetAlert>)`
- Called immediately after each agent returns; returns per-call USD cost and optional alert to bubble to UI.

**Function**: `CostTracker::write_summary(spec_id, evidence_dir: &Path) -> Result<PathBuf>`
- Persists `cost_summary.json` alongside existing telemetry; invoked at end of `/speckit.auto` and `/speckit.plan` stand-alone runs.

**Function**: `TaskComplexity::for_command(command: &SpecCommand) -> TaskComplexity`
- Pure function mapping command metadata and historical error rate into a tier; used by routing table.

**API Contract**: Routing Table Config
- TOML/JSON entry (or Rust constant) describing `tier -> [primary_models, validator, aggregator, escalation_policy]` with checksum to detect drift.

### Component Architecture

**New/Modified Components**:
- `handler.rs`: owns `CostTracker`, invokes classifiers, dispatches agents, handles escalation when validators fail, and writes telemetry.
- `cost_tracker.rs`: extended with pricing versioning, budget enforcement, and summary serialization.
- `spec_id_generator.rs`: already replaces consensus; expose metrics hook so Phase 1 telemetry includes SPEC-ID latency.
- `docs/spec-kit/prompts.json`: prompt variants for Flash/Haiku stored with version keys; plan requires loader to select prompt by model tier.

**Interactions**:
```
/speckit.auto → handler.rs (determine stage) → TaskComplexity classifier
  → Routing table selects models → spawn agents → CostTracker logs spend
  → Validator verdict OK? yes → continue
                       no  → escalate tier & log evidence
  → After stage complete → CostTracker write_summary + telemetry append
```

---

## Acceptance Mapping

| Requirement (from Spec) | Validation Step | Test/Check Artifact |
|-------------------------|-----------------|---------------------|
| FR1: Replace expensive models with cheaper equivalents | Run `/speckit.plan SPEC-KIT-070` under new routing; inspect TUI log for Flash/Haiku usage and confirm cost summary shows ≤$0.80 spend for stage. | `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-070/spec-plan_*.json` + cost summary |
| FR2: Native SPEC-ID generation | Execute `/speckit.new "Test"`; ensure SPEC-ID returned instantly with no agent spawn and cross-check evidence logs. | TUI output + `docs/SPEC-KIT-071-*/` directory timestamps |
| FR3: Per-SPEC cost tracking with budgets | Run `/speckit.auto SPEC-KIT-070`; verify `cost_summary.json` includes `per_stage`, `per_model`, and alert status. | `docs/.../evidence/costs/SPEC-KIT-070_cost_summary.json` |
| FR4: Complexity-based routing | Unit test routing table (new `routing_matrix_tests`), then run `/speckit.tasks`; confirm only Tier-M models launched. | `cargo test routing_matrix` + TUI log snippet |
| NFR1: Maintain ≥90% consensus quality | Blind A/B between premium vs cheap outputs for plan/tasks; require reviewer sign-off that agreement ≥90%. | `docs/SPEC-KIT-070/PHASE1_VALIDATION.md` with scorecard |
| NFR2: 100% automated tests pass | `cargo test --all` after each phase. | CI log, `test result: ok. 180 passed` |
| NFR3: Cost estimates within ±15% | Compare `CostTracker` totals with provider billing export for same window. | `docs/.../cost-validation.csv` |
| NFR4: No production regressions | Monitor `/speckit.status` + telemetry for 1 week; zero new P0 incidents. | Incident log + SPEC.md status note |

---

## Risks & Unknowns

### Risk 1: Quality degradation when demoting models
**Impact**: High | **Probability**: Medium
**Mitigation**: Gate each phase on ≥90% agreement A/B tests; keep premium fallback path live; add validator strictness knobs before rollout.
**Owner**: Code

### Risk 2: Cost tracker drift vs real billing
**Impact**: Medium | **Probability**: Medium
**Mitigation**: Store `pricing_version` and compare against monthly provider CSVs; add integration test that replays historical usage to detect ≥10% drift.
**Owner**: Code

### Risk 3: Provider rate limits or model availability
**Impact**: Medium | **Probability**: High (recent outage)
**Mitigation**: Stage runs to avoid burst limits, keep secondary credentials, and allow routing table to swap to alternate cheap models (Flash 1.5 ↔ 2.0) without code changes.
**Owner**: Code

---

## Multi-Agent Consensus

### Agreements
- Gemini + Claude both emphasized phased rollout (validate → instrument → route → optimize) and keeping `/speckit.implement` refactor as the largest savings lever.
- Both agents agree quality gates (≥90% consensus, ±15% cost accuracy) must guard every phase before promotion.
- Evidence-first workflow (cost summaries + telemetry) is non-negotiable so later stages stay data-driven.

### Conflicts Resolved
- **Issue**: Level of detail in work breakdown (3 macro steps vs 15 granular tasks).
  - Gemini favored three macro phases; Claude argued for 15 explicit tasks across four phases.
  - **Resolution**: This plan adopts four phases with explicit deliverables per phase while keeping macro framing for clarity.
- **Issue**: Aggregator (gpt_pro) unavailable—model not supported in current account, so no synthesized consensus.
  - **Resolution**: Document degraded consensus state, rely on Gemini + Claude overlap for agreements, and log gpt_pro failure in evidence for follow-up before `/speckit.tasks`.

---

## Exit Criteria

- [ ] Phase 1 validation artifacts show cheap-model quality ≥90% agreement and measured cost ≤$6 per `/speckit.auto`.
- [ ] `CostTracker` integrated with alerts, telemetry, and documentation; new tests green.
- [ ] Routing table + `/speckit.implement` refactor in place with automated coverage proving $2–3 per `/speckit.auto`.
- [ ] Dynamic escalation logic exercised with injected failures and documented rollback.
- [ ] Evidence directory contains `cost_summary.json`, A/B scorecards, and updated PHASE docs.
- [ ] SPEC.md updated only for references (not overwritten) once exit criteria met and plan approved.

---

## Evidence References

**Plan Consensus**: docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-070/spec-plan_synthesis.json
**Telemetry**: docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-070/spec-plan_20251026T2330Z.json
**Agent Outputs**: .code/agents/931c6db2-1eb8-4cbc-9ef1-db2e77e32020/result.txt (gemini), .code/agents/9a1892f6-86cf-4c79-b993-a71878022ac7/result.txt (claude), .code/agents/15be345a-702d-4d6e-98e5-e6d343d0b4be/result.txt (gpt_pro – failed, document degraded consensus)

---

## Exit Checklist for /speckit.tasks Readiness

- [ ] Multi-agent aggregator (gpt_pro) repaired or alternate model approved
- [ ] Plan reviewed with maintainer and consensus recorded in local-memory (importance ≥8)
- [ ] SPEC auto-pipeline unblocked once plan sign-off recorded

