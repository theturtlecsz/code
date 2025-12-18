# Gate Policy Specification

**Version**: 1.1.0
**Status**: Active
**Supersedes**: Legacy "consensus" terminology (GR-001)

This document defines the **gate policy** for Spec-Kit pipeline progression. Gates are deterministic decision points that evaluate signals and produce `AutoApply` or `Escalate` resolutions.

> **This is not voting.**
> Gates evaluate signals from a single stage owner plus optional non-authoritative sidecars.
> There is no multi-output comparison, no committee synthesis, no "majority wins."
> See [GR-001](#9-relationship-to-gr-001) for the explicit prohibition.

---

## Terminology

| Term | Definition |
|------|------------|
| **Role** | Named responsibility in the workflow (Architect, Implementer, Validator, Judge) |
| **Worker** | Runtime implementation of a role (model + prompt + tool permissions + timeouts) |
| **Stage** | Step in the pipeline (Specify, Plan, Tasks, Implement, Validate, Audit, Unlock) |
| **Artifact** | Durable output of a stage (spec.md, plan.md, tasks.md, diff, test results) |
| **Gate** | Decision point controlled by policy ("can we proceed?") |
| **Signals** | Inputs to a gate (owner_confidence, risk_flags, test_pass, critic_warnings) |
| **Decision Rule** | The explicit rule for combining signals into a resolution |

**Note**: The term "agent" is avoided due to 2025 semantic overload. Use "role" for responsibilities, "worker" for implementations. The term "quorum" is avoided because it implies voting.

---

## 1. Quality Checkpoints

Each checkpoint runs one or more gates between pipeline stages.

| Checkpoint | Runs After | Runs Before | Gates | Purpose |
|------------|------------|-------------|-------|---------|
| `before-plan` | Specify | Plan | Clarify | Resolve PRD ambiguities early |
| `after-plan` | Plan | Tasks | Checklist | Validate PRD + plan quality |
| `after-tasks` | Tasks | Implement | Analyze | Full consistency check |
| `before-unlock` | Audit | Unlock | Final review | High-risk validation |

**Legacy naming**: The codebase currently uses `BeforeSpecify`, `AfterSpecify`, `AfterTasks` enum variants. These will be renamed in a future PR to match the checkpoint positions above.

---

## 2. Gate Types

### Clarify Gate
**Purpose**: Identify and resolve ambiguities in requirements.
**Stage owner**: Architect role
**Inputs**: PRD content, requirement questions
**Outputs**: Clarified requirements, resolved ambiguities
**Artifact modifications**: Updates to `spec.md` requirement sections

### Checklist Gate
**Purpose**: Score and improve requirement quality.
**Stage owner**: Architect role
**Inputs**: PRD + plan artifacts, quality checklist
**Outputs**: Quality scores, suggested improvements
**Artifact modifications**: Updates to low-scoring requirements

### Analyze Gate
**Purpose**: Check consistency across all artifacts.
**Stage owner**: Validator role
**Inputs**: spec.md, plan.md, tasks.md
**Outputs**: Terminology conflicts, missing items, inconsistencies
**Artifact modifications**: Terminology fixes, missing requirement additions

---

## 3. Signals

### Confidence

Confidence is a **computed gate signal** derived from the stage owner's self-reported confidence and evidence-based adjustments (tests, critic flags, policy violations).

**It is not based on voting across multiple generated answers.**

| Level | Condition | Behavior |
|-------|-----------|----------|
| `High` | `owner_confidence >= 0.80` AND no critical counter-signals | Can auto-apply |
| `Medium` | `owner_confidence >= 0.60` AND only minor counter-signals | Conditional auto-apply |
| `Low` | `owner_confidence < 0.60` OR any critical counter-signal present | Must escalate |

**Counter-signals** (from sidecars/validators):
- `risk_flags`: Security, architecture, or scope concerns
- `contradictions`: Inconsistencies with existing artifacts
- `needs_human`: Explicit flag that human judgment is required
- `test_failures`: Compiler or test failures
- `policy_violations`: GR-* guardrail violations

### Magnitude

Severity of the issue, determined by the stage owner or gate policy.

| Level | Definition | Auto-apply eligible? |
|-------|------------|---------------------|
| `Critical` | Blocks progress, affects core functionality | **Never** — always escalate |
| `Important` | Significant but not blocking | Yes (if High confidence + AutoFix) |
| `Minor` | Nice-to-have, cosmetic | Yes (if confidence sufficient) |

### Resolvability

Whether the fix can be applied automatically.

| Level | Definition | Behavior |
|-------|------------|----------|
| `AutoFix` | Straightforward fix, apply immediately | Can auto-apply |
| `SuggestFix` | Fix available but needs validation | Conditional (High only, or ACE boost) |
| `NeedHuman` | Requires human judgment | **Always escalate** |

---

## 4. Resolution Decision Matrix

The gate evaluates `(Confidence, Magnitude, Resolvability)` to produce a resolution.

### Auto-Apply Conditions

```
AutoApply when:
  (High,   Minor,     AutoFix)     -> YES
  (High,   Minor,     SuggestFix)  -> YES
  (High,   Important, AutoFix)     -> YES
  (Medium, Minor,     AutoFix)     -> YES
  (Medium, Minor,     SuggestFix)  -> YES (only with ACE boost*)
```

*ACE boost: If the ACE playbook contains a helpful pattern (confidence >= 0.7) matching the issue type.

### Escalate Conditions

```
Escalate when:
  Magnitude = Critical           -> Always escalate
  Resolvability = NeedHuman      -> Always escalate
  Confidence = Low               -> Always escalate
  Any critical counter-signal    -> Always escalate
  (Medium, Important, *)         -> Escalate
  (High, Important, SuggestFix)  -> Escalate
```

### Decision Flowchart

```
Issue arrives
    |
    +-- Any critical counter-signal? -----------------> ESCALATE
    |
    +-- Magnitude = Critical? ------------------------> ESCALATE
    |
    +-- Resolvability = NeedHuman? ------------------> ESCALATE
    |
    +-- Confidence = Low? ----------------------------> ESCALATE
    |
    +-- Confidence = High?
    |   +-- Magnitude = Minor?
    |   |   +-- Any Resolvability ------------------> AUTO-APPLY
    |   +-- Magnitude = Important?
    |       +-- Resolvability = AutoFix? -----------> AUTO-APPLY
    |       +-- Otherwise --------------------------> ESCALATE
    |
    +-- Confidence = Medium?
        +-- Magnitude = Minor + AutoFix? -----------> AUTO-APPLY
        +-- ACE boost + SuggestFix? ----------------> AUTO-APPLY
        +-- Otherwise ------------------------------> ESCALATE
```

---

## 5. Escalation Routes

When a gate produces `Escalate`, the issue is routed based on context. Gate Policy determines **what** happens; Model Policy determines **who** executes it.

### Human Approval Required
- All `Critical` magnitude issues
- All `NeedHuman` resolvability issues
- Architectural decisions affecting core functionality
- High-Risk changes (per MODEL-POLICY.md Section 7)

### Judge Role Validation
- Issues that don't meet auto-apply criteria but have a suggested fix
- High-Risk flagged changes
- Final audit before unlock stage

### Role-Specific Escalation Triggers

| Role | Trigger | Escalation Target |
|------|---------|-------------------|
| Architect | `owner_confidence < 0.75` | Judge role |
| Implementer | 2 failed compile/test loops | Implementer fallback lane |
| Librarian | `context > 100k tokens` | Long-context lane |

**Model selection**: See `docs/MODEL-POLICY.md` for the canonical role -> model/provider mapping.

---

## 6. Resolution Struct

```rust
pub enum Resolution {
    AutoApply {
        answer: String,
        confidence: Confidence,
        reason: String,
        validation: Option<JudgeValidationResult>,
    },
    Escalate {
        reason: String,
        counter_signals: Vec<CounterSignal>,
        recommended: Option<String>,
    },
}
```

---

## 7. Evidence & Telemetry

Every gate evaluation produces telemetry stored under:

```
docs/{SPEC-ID}/evidence/
+-- quality-gate/
    +-- {checkpoint}_{timestamp}_verdict.json
    +-- {checkpoint}_{timestamp}_telemetry.json
    +-- {checkpoint}_{timestamp}_resolutions.json
```

### Verdict Schema

```json
{
  "checkpoint": "after-plan",
  "timestamp": "2025-12-18T...",
  "summary": {
    "total_issues": 5,
    "auto_resolved": 3,
    "escalated": 2
  },
  "auto_resolved_details": [...],
  "escalated_details": [...]
}
```

---

## 8. Signal Conflicts

When sidecars raise conflicting signals:

1. **Stage owner is the only writer** — sidecars are critics, not co-authors
2. **Sidecars produce signals, not competing answers** — they flag risks, contradictions, or concerns
3. **If any sidecar raises a block-level signal** -> escalate to human/judge
4. **Auto-apply only when no block-level signals are raised**
5. **Never synthesize or average** — the owner's answer stands or escalates

### Block-level signals (always trigger escalation)
- `risk_flags.critical = true`
- `contradictions.blocking = true`
- `needs_human = true`
- `test_failures > 0`
- `policy_violations > 0`

### Advisory signals (inform but don't block)
- `risk_flags.minor = true`
- `style_suggestions`
- `optimization_hints`

---

## 9. Relationship to GR-001

This gate policy **implements** GR-001 (No consensus by default):

- **Forbidden**: Multi-agent voting or debate
- **Forbidden**: Committee merges or synthesis steps
- **Forbidden**: Requirement that multiple models agree before progressing
- **Allowed**: Single-owner stages with quality gates
- **Allowed**: Deterministic escalation based on signals
- **Allowed**: Optional critic-only sidecar (non-authoritative)

The canonical pipeline remains:
```
Stage 0 -> Single Architect -> Single Implementer -> Single Judge
               (optional critic sidecar if triggered)
```

Quality is enforced by:
- Compiler/tests (GR-006)
- Constitution gates
- Judge audit
- **NOT by voting**

---

## 10. Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SPEC_KIT_CONSENSUS` | `false` | Legacy voting mode (DEPRECATED, do not use) |
| `SPEC_KIT_SIDECAR_CRITIC` | `false` | Enable non-blocking critic sidecar |

### Schema (quality_gates.schema.json)

```json
{
  "min_confidence_for_auto_apply": 0.60,
  "min_test_coverage": null,
  "schema_validation": true,
  "enabled": true
}
```

**Legacy note**: The field `consensus_threshold` is deprecated and will be removed. Use `min_confidence_for_auto_apply` instead.

---

## 11. Wiring Guidance (Router Contract)

**Added in PR1 (2025-12-18)**: The Gate Policy and Router are now explicitly separated with a "thin waist" interface.

### Separation of Concerns

| Component | Decides | Does NOT Decide |
|-----------|---------|-----------------|
| **Gate Policy** | Roles, signals, decision rules, verdicts | Model names, providers, budgets |
| **Router** | Worker implementation (model/provider/budget) | Gate logic, escalation rules |

### Interface Contract

**Gate Policy → Orchestrator**:
```rust
// codex-rs/spec-kit/src/gate_policy.rs
pub fn roles_for_stage(stage: Stage, ctx: &StageContext) -> RoleAssignment;
pub fn checkpoints_for_stage_transition(from: Stage, to: Stage) -> Vec<Checkpoint>;
```

**Router → Orchestrator**:
```rust
// codex-rs/spec-kit/src/router.rs
pub trait Router {
    fn select_worker(&self, role: Role, ctx: &RoutingContext) -> WorkerSpec;
}
```

### Consumption Pattern

```rust
// Pipeline Coordinator (simplified)
let assignment = gate_policy::roles_for_stage(stage, &stage_ctx);
let owner_worker = router.select_worker(assignment.owner, &routing_ctx);

// Execute owner, collect signals
let signals = execute_worker(&owner_worker).await?;

// Run sidecars (if any)
for sidecar_role in assignment.sidecars {
    let sidecar_worker = router.select_worker(sidecar_role, &routing_ctx);
    let counter_signals = execute_worker(&sidecar_worker).await?;
    signals.extend(counter_signals);
}

// Evaluate gate deterministically
let verdict = evaluate_gate(checkpoint, &signals, &decision_rule, &gate_ctx);
```

### Why This Matters

- **No model spaghetti**: Gate policy never mentions "claude", "gemini", etc.
- **Policy is pluggable**: Swap providers by changing Router config, not gate logic
- **Testable**: Gate evaluation is pure function of signals + rules
- **Auditable**: GateVerdict contains all inputs for compliance review

---

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.1.0 | 2025-12-18 | Remove voting semantics; redefine Confidence as computed signal; rename Quorum to Decision Rule; fix checkpoint naming; separate gate policy from model routing |
| 1.0.0 | 2025-12-18 | Initial spec from terminology cleanup (replaces "consensus" language) |

---

Back to [Spec-Kit Documentation](README.md) | [Model Policy](../MODEL-POLICY.md)
