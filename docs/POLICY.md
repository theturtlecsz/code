# Policy Reference

**Version**: 1.0.0
**Status**: Active
**Last Updated**: 2026-01-21
**Consolidated From**: MODEL-POLICY.md, GATE\_POLICY.md, evidence-policy.md, testing-policy.md

***

## Overview

This document consolidates all policy documentation for the spec-kit pipeline. It defines:

* **Model Policy**: Role definitions, routing, escalation rules
* **Gate Policy**: Quality checkpoints, signals, decision rules
* **Evidence Policy**: Retention, archival, cleanup procedures
* **Testing Policy**: Coverage targets, test strategy

**Precedence**: This document supersedes individual policy files. For model-specific reasoning guidance, see [MODEL-GUIDANCE.md](MODEL-GUIDANCE.md).

***

## 1. Model Policy

> **Source**: Previously `docs/MODEL-POLICY.md` (v2 Track)

### 1.1 Policy Lifecycle

1. **Author** policy in repo (this doc + structured config)
2. **Validate** in CI (schema + golden scenarios)
3. **Deploy** with version bump
4. **Enforce** at router + gates
5. **Snapshot** into capsule (`PolicySnapshot.json`) per run/checkpoint
6. **Monitor** and audit via replay reports

### 1.2 Default Role Routing

| Role                   | Default Model                    | Notes                                               |
| ---------------------- | -------------------------------- | --------------------------------------------------- |
| Architect              | `gpt-5.2-xhigh`                  | Cloud frontier reasoning/design                     |
| Judge                  | `gpt-5.2-xhigh`                  | Cloud frontier for unlock gates                     |
| Implementer            | `gpt-5.2-xhigh` / `gpt-5.2-high` | Cloud coder; can be escalated to                    |
| **Implementer.Reflex** | `gpt-oss-20b` (local via SGLang) | Routing mode for sub-second compiler/test iteration |
| SidecarCritic          | `gpt-5.2-mini`                   | Always-on cheap critique                            |
| NotebookLM Tier2       | NLM service                      | Non-blocking synthesis                              |

**Implementation Note**: Treat "Reflex" as `role=Implementer` + `mode=reflex`, not as a new Stage0 role.

### 1.3 Local Reflex Alternatives

* **Qwen3-Coder-30B-A3B-Instruct (AWQ/GPTQ)** — Fallback when GPT-OSS-20B underperforms
* **Qwen2.5-Coder-32B (AWQ INT4)** — Dense fallback if MoE models regress

**Rule**: Default reflex stays `gpt-oss-20b`. Alternatives are opt-in via config.

### 1.4 Escalation Rules

| Condition                                          | Action                                                     |
| -------------------------------------------------- | ---------------------------------------------------------- |
| Reflex exhausts `reflex_max_attempts` (default: 2) | Escalate to cloud Implementer                              |
| High-risk specs                                    | Skip reflex, route directly to cloud Implementer/Architect |
| Unlock decisions                                   | Judge role (always cloud)                                  |

### 1.5 Evidence Requirements

Every model/tool call logs:

* Role, stage, provider, model, attempt
* **Selection reason**
* `PolicySnapshot.json` stored per run

***

## 2. Gate Policy

> **Source**: Previously `docs/spec-kit/GATE_POLICY.md` (v1.1.0)
>
> **Critical**: This is not voting. Gates evaluate signals from a single stage owner plus optional non-authoritative sidecars. See [GR-001](#24-gr-001-enforcement) for the explicit prohibition.

### 2.1 Terminology

| Term         | Definition                                                                      |
| ------------ | ------------------------------------------------------------------------------- |
| **Role**     | Named responsibility (Architect, Implementer, Validator, Judge)                 |
| **Worker**   | Runtime implementation of a role (model + prompt + tools + timeouts)            |
| **Stage**    | Pipeline step (Specify, Plan, Tasks, Implement, Validate, Audit, Unlock)        |
| **Artifact** | Durable output (spec.md, plan.md, tasks.md, diff, test results)                 |
| **Gate**     | Decision point controlled by policy                                             |
| **Signals**  | Inputs to a gate (owner\_confidence, risk\_flags, test\_pass, critic\_warnings) |

### 2.2 Quality Checkpoints

| Checkpoint      | Runs After | Runs Before | Gates        | Purpose                     |
| --------------- | ---------- | ----------- | ------------ | --------------------------- |
| `before-plan`   | Specify    | Plan        | Clarify      | Resolve PRD ambiguities     |
| `after-plan`    | Plan       | Tasks       | Checklist    | Validate PRD + plan quality |
| `after-tasks`   | Tasks      | Implement   | Analyze      | Full consistency check      |
| `before-unlock` | Audit      | Unlock      | Final review | High-risk validation        |

### 2.3 Signals

#### Confidence (Computed Gate Signal)

| Level    | Condition                                                  | Behavior               |
| -------- | ---------------------------------------------------------- | ---------------------- |
| `High`   | `owner_confidence >= 0.80` AND no critical counter-signals | Can auto-apply         |
| `Medium` | `owner_confidence >= 0.65` AND only minor counter-signals  | Conditional auto-apply |
| `Low`    | `owner_confidence < 0.65` OR any critical counter-signal   | Must escalate          |

**Counter-signals**: `risk_flags`, `contradictions`, `needs_human`, `test_failures`, `policy_violations`

#### Magnitude

| Level       | Definition                                  | Auto-apply eligible?               |
| ----------- | ------------------------------------------- | ---------------------------------- |
| `Critical`  | Blocks progress, affects core functionality | **Never**                          |
| `Important` | Significant but not blocking                | Yes (if High confidence + AutoFix) |
| `Minor`     | Nice-to-have, cosmetic                      | Yes (if confidence sufficient)     |

#### Resolvability

| Level        | Definition                         | Behavior                              |
| ------------ | ---------------------------------- | ------------------------------------- |
| `AutoFix`    | Straightforward fix                | Can auto-apply                        |
| `SuggestFix` | Fix available but needs validation | Conditional (High only, or ACE boost) |
| `NeedHuman`  | Requires human judgment            | **Always escalate**                   |

### 2.4 Decision Matrix

**Auto-Apply Conditions**:

```
(High,   Minor,     AutoFix)     -> YES
(High,   Minor,     SuggestFix)  -> YES
(High,   Important, AutoFix)     -> YES
(Medium, Minor,     AutoFix)     -> YES
(Medium, Minor,     SuggestFix)  -> YES (with ACE boost)
```

**Escalate Conditions**:

```
Magnitude = Critical           -> Always escalate
Resolvability = NeedHuman      -> Always escalate
Confidence = Low               -> Always escalate
Any critical counter-signal    -> Always escalate
(Medium, Important, *)         -> Escalate
(High, Important, SuggestFix)  -> Escalate
```

### 2.5 Escalation Routes

| Role        | Trigger                     | Target                    |
| ----------- | --------------------------- | ------------------------- |
| Architect   | `owner_confidence < 0.75`   | Judge role                |
| Implementer | 2 failed compile/test loops | Implementer fallback lane |
| Librarian   | `context > 100k tokens`     | Long-context lane         |

### 2.6 GR-001 Enforcement

This gate policy **implements** GR-001 (No consensus by default):

* **Forbidden**: Multi-agent voting or debate
* **Forbidden**: Committee merges or synthesis steps
* **Forbidden**: Requirement that multiple models agree before progressing
* **Allowed**: Single-owner stages with quality gates
* **Allowed**: Deterministic escalation based on signals
* **Allowed**: Optional critic-only sidecar (non-authoritative)

**Canonical Pipeline**:

```
Stage 0 -> Single Architect -> Single Implementer -> Single Judge
               (optional critic sidecar if triggered)
```

### 2.7 Configuration

| Variable                  | Default | Description                |
| ------------------------- | ------- | -------------------------- |
| `SPEC_KIT_CONSENSUS`      | `false` | Legacy voting (DEPRECATED) |
| `SPEC_KIT_SIDECAR_CRITIC` | `false` | Enable non-blocking critic |

**Schema** (`quality_gates.schema.json`):

```json
{
  "min_confidence_for_auto_apply": 0.65,
  "min_test_coverage": null,
  "schema_validation": true,
  "enabled": true
}
```

### 2.8 Stage→Agent Routing

> **Added**: SPEC-KIT-981 (2026-01-31)

**Principle**: Spec-Kit stages are single-owner; no consensus.

**Default Routing** (GPT-first):

| Stage                       | Default Agent | Notes                   |
| --------------------------- | ------------- | ----------------------- |
| Specify, Plan, Tasks        | `gpt_pro`     | Architect/Planner roles |
| Implement                   | `gpt_codex`   | Code generation         |
| Validate, Audit, Unlock     | `gpt_pro`     | Judge/Validator roles   |
| Clarify, Analyze, Checklist | `gpt_pro`     | Quality gate commands   |

**Override via Config** (`config.toml`):

```toml
[speckit.stage_agents]
plan = "claude"       # Override Plan stage to use Claude
implement = "gemini"  # Override Implement stage to use Gemini
```

**Note**: TUI UI for editing stage→agent defaults is tracked separately (SPEC-KIT-983).

***

## 3. Evidence Policy

> **Source**: Previously `docs/spec-kit/evidence-policy.md` (v1.0)

### 3.1 Overview

**Location**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

**Structure**:

```
evidence/
├── commands/<SPEC-ID>/     # Guardrail telemetry JSON
├── consensus/<SPEC-ID>/    # Multi-agent consensus artifacts
└── .locks/<SPEC-ID>.lock   # File locks (ARCH-007)
```

### 3.2 Size Limits

| Scope                | Soft Limit | Trigger Action              |
| -------------------- | ---------- | --------------------------- |
| **Per-SPEC**         | 25 MB      | Review for cleanup          |
| **Total Repository** | 500 MB     | Archive old SPECs           |
| **Per-File**         | 5 MB       | Investigate agent verbosity |

**Monitoring**:

```bash
scripts/spec_ops_004/evidence_stats.sh [--spec SPEC-ID]
```

### 3.3 Retention Policy

| Status                                                | Policy              | Timeline                                          |
| ----------------------------------------------------- | ------------------- | ------------------------------------------------- |
| **Active** (Backlog, In Progress, In Review, Blocked) | Keep all            | Indefinitely                                      |
| **Completed** (Done, unlocked + merged)               | Keep 30 days        | Day 30: compress, Day 90: offload, Day 180: purge |
| **Abandoned** (>90 days inactive)                     | Archive immediately | No retention period                               |

### 3.4 Archival Strategy

**Compression** (in-place):

```bash
cd evidence/consensus/SPEC-KIT-XXX/
tar czf ../SPEC-KIT-XXX-consensus.tar.gz *.json && rm *.json
```

**Expected compression**: 70-85% (JSON compresses well)

**Offload** (external storage after 90 days):

```bash
tar czf SPEC-KIT-XXX-$(date +%Y%m%d).tar.gz evidence/{commands,consensus}/SPEC-KIT-XXX/
# Move to external storage, then rm -rf
```

### 3.5 Automated Cleanup (SPEC-933 C4)

**Module**: `codex-rs/tui/src/chatwidget/spec_kit/evidence_cleanup.rs`

**Configuration**:

```rust
archive_after_days: 30,
purge_after_days: 180,
warning_threshold_mb: 45,
hard_limit_mb: 50,
```

**Safety Features**: In-progress detection, archive before purge, dry-run mode

***

## 4. Testing Policy

> **Source**: Previously `docs/spec-kit/testing-policy.md` (v1.0)

### 4.1 Current State

**Achievement**: \~42-48% coverage (604 tests) - **Phase 2+3+4 COMPLETE**
**Target**: 40% by Q1 2026 - **EXCEEDED** (4 months early)

### 4.2 Coverage by Module

| Module         | Coverage | Target | Status        |
| -------------- | -------- | ------ | ------------- |
| `handler.rs`   | \~47%    | 30%    | Exceeded      |
| `consensus.rs` | \~30%    | 50%    | Acceptable    |
| `quality.rs`   | \~21%    | 60%    | State-focused |
| `state.rs`     | \~40%    | 40%    | Met           |
| `schemas.rs`   | \~35%    | 25%    | Exceeded      |
| `error.rs`     | \~27%    | 20%    | Exceeded      |

### 4.3 Priority Modules

| Priority | Modules                              | Focus                                          |
| -------- | ------------------------------------ | ---------------------------------------------- |
| **P0**   | handler.rs, consensus.rs, quality.rs | Multi-agent coordination, consensus validation |
| **P1**   | evidence.rs, guardrail.rs, state.rs  | Infrastructure, safety                         |
| **P2**   | schemas.rs, error.rs                 | Supporting                                     |

### 4.4 Test Infrastructure

**Isolation Tools**:

* `MockSpecKitContext` - Fakes ChatWidget interactions
* `MockEvidence` - In-memory evidence repository
* `MockMcpManager` - MCP call mocking

**Coverage Measurement**:

```bash
cargo tarpaulin --workspace --out Stdout
```

### 4.5 Validation Tiers

| Change Size                | Validation                                       |
| -------------------------- | ------------------------------------------------ |
| <50 lines                  | Trust model self-check                           |
| 50-200 lines               | `fmt` + `clippy`                                 |
| >200 lines or cross-module | Full harness (`fmt`, `clippy`, `build`, `tests`) |

***

## 5. Related Documentation

| Document                                            | Purpose                                                 |
| --------------------------------------------------- | ------------------------------------------------------- |
| [MODEL-GUIDANCE.md](MODEL-GUIDANCE.md)              | Model-specific reasoning and extended thinking triggers |
| [DECISIONS.md](DECISIONS.md)                        | Locked decisions (D1-D134)                              |
| [GOLDEN\_PATH.md](GOLDEN_PATH.md)                   | End-to-end workflow walkthrough                         |
| [memory/constitution.md](../memory/constitution.md) | Project charter and guardrails                          |

***

## 6. Change History

| Version | Date       | Changes                                                                                          |
| ------- | ---------- | ------------------------------------------------------------------------------------------------ |
| 1.0.0   | 2026-01-21 | Initial consolidated policy (merged MODEL-POLICY, GATE\_POLICY, evidence-policy, testing-policy) |

**Source Document Versions**:

* MODEL-POLICY.md: v2 Track (2026-01-10)
* GATE\_POLICY.md: v1.1.0 (2025-12-18)
* evidence-policy.md: v1.0 (2025-10-18)
* testing-policy.md: v1.0 (2025-10-18)

***

Back to [INDEX.md](INDEX.md) | [SPEC.md](../SPEC.md)
