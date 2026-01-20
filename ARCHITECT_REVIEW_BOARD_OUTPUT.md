# ARCHITECT_REVIEW_BOARD_OUTPUT.md

**Generated:** 2026-01-19
**Session:** 10 of N (Phase 2 COMPLETE: H0-H7 all decided)
**Facilitator:** Architecture Review Board (Pass 2)

---

## 1. STATE

### Progress Tracker

| Question | Status | Session | Choice | Summary |
|----------|--------|---------|--------|---------|
| A1: Primary product promise | ✅ DECIDED | 8 | Option 3 (modified) | Spec-to-ship automation, human-controlled |
| A2: Must-have interaction surfaces | ✅ DECIDED | 8 | Option 5 | Tiered parity (core=all; UX=TUI-first) |
| B1: Capsule SOR content | ✅ DECIDED | 8 | Option 2 | Raw + Derived (events + projections) |
| B2: Retention posture | ✅ DECIDED | 8 | Option 5 | Hybrid (TTL + milestone protection) |
| C1: Capture-mode enforcement | ✅ DECIDED | 8 | Option 5 | Checkpoint-gated; over-capture blocked |
| C2: Default capture mode | ✅ DECIDED | 8 | Option 4 | Policy-derived; template uses prompts_only |
| D1: Pipeline modularity | ✅ DECIDED | 8 | Option 1 | Monolith with internal seams |
| D2: Quality gates posture | ✅ DECIDED | 8 | Option 2 | Blocking-with-override |
| E1: Enforcement strictness | ✅ DECIDED | 8 | Option 2 | Hard-fail-core-only |
| E2: Hard-fail policy set | ✅ DECIDED | 8 | Confirm + adjust | Tier 1 includes policy sovereignty |
| F1: Maintenance framework timing | ✅ DECIDED | 8 | Option 4 | Tiered (event + scheduled + on-demand) |
| F2: First maintenance job family | ✅ DECIDED | 8 | Option 1 | Health Check first |
| G1: Cross-cutting verification | ✅ DECIDED | 8 | Option 4 | Hybrid (contracts + workflow tests) |
| G2: Capability matrix definition | ✅ DECIDED | 8 | Confirm | Prioritize E.3/E.4 gaps |
| G3: Regression prevention | ✅ DECIDED | 8 | Three-layer | Property + Golden + Snapshot |
| **H0: Consensus replacement** | ✅ DECIDED | 9 | Option 3 | ACE Frames + Maieutic Specs |
| **H1: ACE explanation scope** | ✅ DECIDED | 9 | Option 4 | Tiered scope (mandatory→selective→on-demand) |
| **H2: Control model** | ✅ DECIDED | 9 | Option 4 | Tiered autonomy with delegation |
| **H3: Maieutic enforcement** | ✅ DECIDED | 10 | Option 1 | Always mandatory pre-execution |
| **H4: Explainability vs capture** | ✅ DECIDED | 10 | Option 2 | capture=none → no persisted artifacts |
| **H5: ACE/maieutic gating** | ✅ DECIDED | 10 | Option 3 | Ship hard-fail if artifacts missing |
| **H6: Multi-surface parity** | ✅ DECIDED | 10 | Option 2 | A2-aligned tiered parity |
| **H7: ACE Frame schema** | ✅ DECIDED | 10 | Option 3 | Generated schema from Rust structs |

### Phase Status
- **Phase 1 (A1-G3):** COMPLETE
- **Phase 2 (Section H: ACE + Maieutics):** COMPLETE (H0-H7 all decided)

---

## 2. Decisions Table

### Product Identity Decisions (A1-A2)

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| A1 | Primary product promise | 3 (modified) | Spec-to-ship automation workbench for solo devs; human-controlled automation with explainability; evidence store (backend currently Memvid) is trust backbone, not the product |
| A2 | Interaction surfaces | 5 (Tiered Parity) | Core automation features = full parity across TUI/CLI/headless; visualization/UX features = TUI-first with CLI fallback |

### Evidence Store Decisions (B1-B2)

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| B1 | Capsule SOR content | 2 (Raw + Derived) | Events + immutable artifacts are authoritative; projections persisted for query speed but verifiable/rebuildable |
| B2 | Retention posture | 5 (Hybrid) | TTL for routine events (90 days); milestone protection for ship points (1 year minimum); configurable via policy |

### Capture Mode Decisions (C1-C2)

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| C1 | Capture enforcement | 5 (Checkpoint-gated) | Over-capture always blocked immediately; under-capture warned during work, blocked at checkpoint until resolved/acknowledged |
| C2 | Default capture mode | 4 (Policy-Derived) | Default configured in model_policy.toml per workspace; template policy uses `prompts_only` for solo devs to get "why/intent trail" without full response storage |

### Pipeline Decisions (D1-D2)

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| D1 | Pipeline modularity | 1 (Monolith + seams) | Keep single binary with 8 fixed stages; modularity via internal traits/modules, not plugins or actors; preserves reliability and single-writer invariants |
| D2 | Quality gates posture | 2 (Blocking + override) | Block when it matters; explicit override path for non-core gates; overrides recorded as GateDecision events; surfaced at checkpoint/ship milestones |

### Enforcement Decisions (E1-E2)

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| E1 | Enforcement strictness | 2 (Hard-fail-core-only) | Tool-truth and critical policy/safety hard-fail; everything else advisory or blocking-with-override; keeps solo-dev flow moving |
| E2 | Hard-fail policy set | Confirm + adjust | Accept proposed tiers with adjustment: policy sovereignty violations added to Tier 1 absolute hard-fail |

#### E2 Tier Assignments (Final)

| Tier | Category | Gates | Policy |
|------|----------|-------|--------|
| **1** | Absolute Hard-Fail | Compile, TypeCheck, SchemaValidation, SafetyRisk, Critical Security (CVSS 9.0+), **Policy Sovereignty** (over-capture, non-logical URIs, writing outside SOR), **Run/Merge Invariants** | No bypass ever |
| **2** | Hard-Fail + Override | UnitTests, IntegrationTests, High Security (CVSS 7.0-8.9), PolicyViolation (non-sovereignty) | Override must be explicit + logged; discouraged at ship milestones |
| **3** | Configurable | Lint warnings, HighRiskChange, Contradiction, Medium Security (CVSS 4.0-6.9) | Soft by default |
| **4** | Advisory Only | Format, Ambiguity, MissingAcceptanceCriteria, PerformanceRisk, Low Security | Never blocks |

### Maintenance Decisions (F1-F2)

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| F1 | Maintenance timing | 4 (Tiered) | Event-triggered for immediate checks; scheduled for cleanup/repair; on-demand for compaction; avoids permanent daemon |
| F2 | First job family | 1 (Health Check) | Read-only foundation; integrity verification, projection consistency, quota warnings; prevents irreversible mistakes |

#### F1 Tiered Schedule

| Tier | Trigger | Jobs |
|------|---------|------|
| Event | Pipeline complete, evidence write | Health checks, evidence limit warnings |
| Daily | Cron 2 AM | Evidence cleanup >30 days, stats |
| Weekly | Cron Sunday 3 AM | Librarian repair, meta-memory synthesis |
| On-demand | User request | Compaction, graph repair, emergency |

### Verification Decisions (G1-G3)

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| G1 | Cross-cutting verification | 4 (Hybrid) | Keep W01-W15 workflow tests; add explicit contracts for Stage/Gate/Evidence interfaces; formalize traceability matrix |
| G2 | Capability matrix | Confirm | Accept categories (P/Q/E/S/X); prioritize closure of E.3 (archival) and E.4 (integrity verification) gaps |
| G3 | Regression prevention | Three-layer | Property-based invariants (expand proptest); Golden fixtures (semantic drift detection); Snapshot tests (mask volatile fields) |

### ACE + Maieutics Decisions (H0-H2) — Session 9

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| H0 | Consensus model replacement | 3 (ACE + Maieutics) | Replace consensus/*.json with two canonical explainability artifacts: (1) Maieutic Spec (pre-execution clarification) and (2) ACE Frames (action/control explanations + post-execution learning). Matches "explainability as core identity" by providing prevention + accountability/learning without multi-model voting complexity. |
| H1 | ACE explanation scope | 4 (Tiered Scope) | Mandatory ACE frames for failures/overrides/policy violations and for stage/checkpoint/ship boundaries; selective deep reflection for interesting outcomes; on-demand explanations available; routine low-impact steps remain event-only to avoid noise/cost. |
| H2 | Control model | 4 (Tiered Autonomy) | Default solo-dev preset: auto for read/tests/small edits (Tier 0-1), checkpoint approval for bounded changes like new files/refactors/config (Tier 2), explicit approval for destructive ops/merge (Tier 3), human-only for ship/external/secrets (Tier 4). Control points always emit ACE frames and audit events. |

#### H1 Tier Definitions (ACE Explanation Scope)

| Tier | Trigger | ACE Frame Required? |
|------|---------|---------------------|
| **1: Always** | Failures, blocked gates, overrides, policy sovereignty events | Yes (mandatory) |
| **2: Boundaries** | Stage transitions, checkpoints, unlock/merge, ship milestones | Yes |
| **3: Selective** | Compile/test failures, lint issues, large changes (>5 files/>200 lines) | Yes (current `should_reflect()`) |
| **4: On-Demand** | User explicitly requests "Explain this" | Yes |
| **5: Never** | Routine read/small edits/green micro-steps | No (event-only) |

#### H2 Tier Definitions (Control/Autonomy)

| Tier | User Role | Example Actions | Approval Required? |
|------|-----------|-----------------|-------------------|
| **0: Full Delegation** | Observer | Read file, search code, run tests | No |
| **1: Routine Delegation** | Approver (async) | Formatting, small refactors, doc updates | No (auditable at checkpoint) |
| **2: Bounded Delegation** | Approver | New files/modules, multi-file refactors, config changes | Checkpoint approval |
| **3: Explicit Approval** | Collaborator | Delete/rename trees, branch switch/revert, merge/unlock | Real-time approval |
| **4: Human-Only** | Operator | Ship/release/publish, external API calls, secrets | User performs manually |

### ACE + Maieutics Enforcement Decisions (H3-H5) — Session 10

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| H3 | Maieutic elicitation enforcement | 1 (Always Mandatory) | Maieutics is core identity; every spec/run must complete pre-execution elicitation before automation proceeds. Fast path (minimal questions) allowed but step cannot be skipped. |
| H4 | Explainability artifacts vs capture mode | 2 (Capture-following) | Explainability artifacts (Maieutic Spec + ACE Frames) persisted only when capture mode allows; `capture=none` produces no persisted explainability record. |
| H5 | Missing ACE/Maieutics gating | 3 (Ship hard-fail) | Shipping is not allowed without persisted Maieutic Spec + ACE milestone frame(s); no override. Combined with H4: `capture=none` is non-shippable. |

#### Key Reconciliation Statement

> **Capture mode `none` is "private scratch mode": it allows running work without persisting explainability artifacts, but it is not eligible for shipping milestones because ship requires persisted Maieutic Spec + ACE milestone frames.**

#### Two Explicit Modes (from H3/H4/H5)

| Mode | Capture Setting | Maieutic/ACE Behavior | Ship Allowed? |
|------|-----------------|----------------------|---------------|
| **Shippable** (default) | `prompts_only` or `full_io` | Maieutic step runs, artifacts persisted, ACE frames persisted per H1 tiering | Yes |
| **Private Scratch** (opt-in) | `none` | Maieutic step runs **in-memory** for guidance, nothing persisted | No (hard block) |

### Multi-Surface and Schema Decisions (H6-H7) — Session 10

| ID | Question | Option | Rationale |
|----|----------|--------|-----------|
| H6 | Multi-surface parity | 2 (A2-Aligned Tiered) | Core (artifact creation, persistence, gating, exit codes) = full parity TUI/CLI/headless via shared executor. UX (interactive interview, ACE visualization) = TUI-first with CLI text fallback; headless is non-interactive (pre-supplied answers + structured output). |
| H7 | ACE Frame schema | 3 (Generated Schema) | JSON Schema generated from Rust structs via schemars. Single source of truth, automatic versioning, formal contract without manual maintenance. Schema version embedded in every ACE Frame. |

#### H6 Surface Parity Matrix

| Surface | Tier 1 (Core) | Tier 2 (UX) | Maieutic Input | Notes |
|---------|--------------|-------------|----------------|-------|
| **TUI** | Full | Full | Interactive interview widgets | Rich ACE frame cards, timeline |
| **CLI** | Full | Degraded-graceful | Text interview OR `--maieutic <path>` | JSON output via `--json` flag |
| **Headless** | Full | None | `--maieutic <artifact>` or `--maieutic-answers <json>` required | No prompts ever; exit codes for NEEDS_INPUT, NEEDS_APPROVAL, BLOCKED_SHIP |

#### H7 ACE Frame Schema Requirements

| Requirement | Implementation |
|-------------|----------------|
| Schema generation | `#[derive(JsonSchema)]` via schemars on ACE Frame structs |
| Schema artifacts | Committed to `schemas/ace_frame/ace_frame.schema.v1.json` |
| Version embedding | Every ACE Frame includes `schema_version: "ace_frame@1.0"` |
| Immutability | Breaking change = new version (v2); never mutate v1 in place |

---

## 3. Architectural Consequences

### From A1 (Product Identity)
- **Reframe documentation**: "Memvid" → "evidence store (backend currently Memvid)"
- **ACE + Maieutics become core identity** (H0-H7 decided)
- **Success metrics**: Spec-to-ship completion rate, automation coverage, explainability coverage, replay confidence, failure quality

### From A2 (Tiered Parity)
- **Tier 1 (Full Parity)**: Pipeline stages, evidence capture, events, exit codes, checkpoint approval
- **Tier 2 (TUI-first)**: Timeline visualization, branch trees, rich ACE widgets, diff views
- **Test matrix**: Tier 1 features tested across all surfaces; Tier 2 tested in TUI with CLI fallback validation

### From B1-B2 (Evidence Store)
- **Events are authoritative**: Projections must be verifiable and rebuildable
- **Milestone markers**: SpecCompleted, ReleaseTagged, MilestoneMarked, Stage 6 completion
- **Retention policy**: 90-day TTL routine; 1-year milestone protection; configurable via policy

### From C1-C2 (Capture)
- **Over-capture: HARD BLOCK** (Tier 1) — storing more than policy permits is never acceptable
- **Under-capture: WARN → CHECKPOINT BLOCK** — must be resolved or acknowledged with auditable record
- **Default template**: `prompts_only` in model_policy.toml; `full_io` requires explicit opt-in

### From D1-D2 (Pipeline)
- **No plugins/actors**: Monolithic binary with 8 fixed stages; traits for internal modularity
- **GateDecision events**: All overrides emit events; surfaced at milestones

### From E1-E2 (Enforcement)
- **Policy sovereignty = Tier 1**: Non-logical URIs, over-capture, SOR violations are absolute hard-fail
- **Test failures = Tier 2**: Override allowed but recorded and discouraged at ship

### From F1-F2 (Maintenance)
- **Health Check first**: Foundation before cleanup/compaction
- **No permanent daemon**: Tiered triggers only

### From G1-G3 (Verification)
- **Contract tests needed**: Stage transition, quality gate, evidence interface contracts
- **Gap closure priority**: E.3 archival tests, E.4 integrity tests
- **Three-layer defense**: Property (40+ invariants) + Golden (fixtures) + Snapshot (insta with masking)

### From H0 (ACE + Maieutics Replacement)
- **Deprecate**: `consensus/*.json` as canonical reasoning output
- **Introduce two canonical artifacts**:
  1. **Maieutic Spec**: Pre-flight interview output (assumptions, clarifications, resolved questions, spec completeness)
  2. **ACE Frame**: Action/control explanations (what, why, evidence, risks, controls) + post-run learning summary
- **Trust model shift**: "trust = clarity before action + explanations at critical points + user control where it matters" replaces "trust = multiple models agreed"

### From H1 (Tiered Explanation Scope)
- **Tier 1 (Mandatory)**: Failures, overrides, policy sovereignty events → always generate ACE frame
- **Tier 2 (Boundaries)**: Stage transitions, checkpoints, ship milestones → always generate ACE frame
- **Tier 3 (Selective)**: Current `should_reflect()` triggers (failures, lint, large changes) → generate on interesting outcomes
- **Tier 4 (On-Demand)**: User-triggered "Explain this" → generate when requested
- **Tier 5 (Never)**: Routine read/small edits → event-only, no ACE frame
- **Acceptance tests**: No ship without milestone ACE frame; no override without ACE frame; stage transitions emit summary frame

### From H2 (Tiered Autonomy)
- **Tier 0-1 (Auto)**: Read/tests/small edits run without approval
- **Tier 2 (Checkpoint)**: Bounded changes require checkpoint approval (current GR-001 behavior)
- **Tier 3 (Approve)**: Destructive ops require real-time explicit approval (current `confirm:` behavior)
- **Tier 4 (Manual)**: Ship/external/secrets are human-only
- **Delegation contract**: Maieutic Spec defines what's allowed without asking
- **Escalation explicit**: Crossing tier boundaries pauses for approval
- **Parity requirement**: TUI/CLI/headless trigger same tier boundaries (presentation differs, contract does not)

### From H3 (Always Mandatory Maieutics)
- **Pipeline blocks at pre-execution**: Cannot enter automated stages until maieutic step complete
- **Fast path allowed**: Minimal question set (goal, constraints, acceptance, risks, delegation) ≈ 30-90 seconds
- **Deep path on demand**: Expands when ambiguity/complexity detected
- **No exceptions**: Even simple specs go through fast maieutic path

### From H4 (Capture-Following Explainability)
- **capture=none is private scratch**: No Maieutic Spec or ACE Frames persisted
- **In-memory guidance still runs**: System uses maieutic/ACE reasoning during run, just doesn't persist
- **Privacy story maximally simple**: `none` = truly no record

### From H5 (Ship Hard-Fail)
- **Ship requires persisted artifacts**: Maieutic Spec + ACE milestone frames must exist
- **capture=none cannot ship**: Explicit block with actionable message ("switch capture mode to ship")
- **No override**: This is Tier 1 absolute (like policy sovereignty)

### From H6 (A2-Aligned Multi-Surface Parity)
- **Shared executor**: TUI/CLI/headless use same SpeckitExecutor (SPEC-KIT-921)
- **Tier 1 parity enforced**: Artifact creation, persistence, gating, exit codes identical
- **Headless non-interactive**: Must use `--maieutic <path>` or `--maieutic-answers <json>`
- **Headless exit codes**: NEEDS_INPUT (missing maieutic), NEEDS_APPROVAL (Tier 2/3 action), BLOCKED_SHIP (H5)
- **No prompts in headless**: Hard assertion, never prompt codepath

### From H7 (Generated ACE Frame Schema)
- **Schemars derivation**: `#[derive(JsonSchema)]` on ACE Frame structs
- **Schema committed**: `schemas/ace_frame/ace_frame.schema.v1.json`
- **Version embedded**: Every ACE Frame has `schema_version` field
- **Immutable versions**: Breaking change = new version, never mutate released schema

---

## 4. Decisions to Lock in DECISION_REGISTER

### Previously Proposed (D113-D121)

| ID | Decision | Source |
|----|----------|--------|
| D113 | Tiered parity: Tier 1 (automation-critical) = full parity; Tier 2 (visualization) = TUI-first | A2 |
| D114 | Events + immutable artifacts are authoritative SOR; projections are rebuildable | B1 |
| D115 | Lazy snapshots deferred until measured latency exceeds thresholds | B1 |
| D116 | Hybrid retention: TTL for routine events, milestone protection for ship points | B2 |
| D117 | Milestone markers: SpecCompleted, ReleaseTagged, MilestoneMarked, Stage 6 completion | B2 |
| D118 | Default TTL: 90 days; milestone protection: 1 year minimum; configurable via policy | B2 |
| D119 | Over-capture is always hard-blocked immediately (Tier 1 absolute) | C1 |
| D120 | Under-capture warned during work, blocked at checkpoint until resolved/acknowledged | C1 |
| D121 | Capture gap acknowledgments create auditable CaptureGapAcknowledged events | C1 |

### New Decisions from Session 8 (D122-D126)

| ID | Decision | Source |
|----|----------|--------|
| D122 | Pipeline architecture remains monolithic; extensibility via internal trait seams only (no dynamic plugins; actor model rejected) | D1 |
| D123 | Quality gates are blocking-with-override; overrides must emit GateDecision event and are disallowed/discouraged at protected milestones unless explicitly acknowledged | D2 |
| D124 | Capture mode defaults are policy-derived; default policy template uses `prompts_only` for solo dev; `full_io` is explicit opt-in | C2 |
| D125 | Enforcement tiers: "policy sovereignty" violations (over-capture, non-logical URIs, SOR violations, merge invariants) are Tier 1 absolute hard-fail | E2 |
| D126 | Maintenance framework uses tiered triggers; Health Check is the first job family to implement | F1, F2 |

### New Decisions from Session 9 (D127-D129)

| ID | Decision | Source |
|----|----------|--------|
| D127 | Consensus model replaced by ACE Frames (action/control explanations + learning) + Maieutic Specs (pre-execution clarification); multi-model voting deprecated | H0 |
| D128 | ACE explanation scope is tiered: Tier 1 (mandatory for failures/overrides/policy), Tier 2 (stage/checkpoint/ship boundaries), Tier 3 (selective per `should_reflect`), Tier 4 (on-demand), Tier 5 (event-only for routine) | H1 |
| D129 | Control model uses tiered autonomy: Tier 0-1 (auto), Tier 2 (checkpoint approval), Tier 3 (explicit approval), Tier 4 (human-only); delegation defined by Maieutic Spec; tier crossing requires explicit escalation | H2 |

### New Decisions from Session 10 (D130-D132)

| ID | Decision | Source |
|----|----------|--------|
| D130 | Maieutic elicitation step is mandatory for every run/spec before automation begins (fast path allowed; no exceptions) | H3 |
| D131 | Persistence of Maieutic Spec + ACE frames follows capture mode; `capture=none` persists no explainability artifacts (in-memory guidance still runs) | H4 |
| D132 | Ship milestones hard-fail if required explainability artifacts are missing; `capture=none` is non-shippable (no override, Tier 1 absolute) | H5 |

### New Decisions from Session 10 continued (D133-D134)

| ID | Decision | Source |
|----|----------|--------|
| D133 | A2-aligned parity for ACE/Maieutics: Tier 1 (artifacts, gating, exit codes) = full parity TUI/CLI/headless via shared executor; Tier 2 UX = TUI-first with degraded CLI and non-interactive headless (pre-supplied answers required) | H6 |
| D134 | ACE Frame schema is a published, versioned JSON Schema generated from Rust structs via schemars; schema version embedded in every ACE Frame; breaking changes = new schema version | H7 |

### Section H Complete

All H0-H7 decisions locked. Phase 2 complete.

---

## 5. Reconciliation Issues

### RECONCILIATION-001: Product Identity Framing

**Issue:** Prior documentation sometimes frames "Memvid" as the product rather than a backend implementation.

**Resolution:** All references should use:
> "Evidence store trust backbone (backend currently Memvid)"

**Affected Documents:**
- SPEC.md invariants
- ARCHITECT_REVIEW_RESEARCH.md
- Any user-facing documentation

**Status:** RESOLVED (2026-01-20)

---

## 6. Capability Matrix (G2 Confirmed)

| Category | ID | Capability | Invariant | Test(s) | Status |
|----------|-----|------------|-----------|---------|--------|
| **Pipeline** | P.1 | Stage transitions | #5, #6 | W01-W15 | ✅ |
| **Pipeline** | P.2 | Stage dependencies | SPEC-948 | config_validator | ✅ |
| **Pipeline** | P.3 | Skip conditions | SPEC-948 | skip_condition_tests | ✅ |
| **Quality** | Q.1 | Gate execution | GR-001 | Q01-Q10 | ✅ |
| **Quality** | Q.2 | Issue resolution | - | resolution_tests | ✅ |
| **Quality** | Q.3 | Escalation paths | - | escalation_tests | ✅ |
| **Evidence** | E.1 | Artifact lifecycle | #2 | S01-S03 | ✅ |
| **Evidence** | E.2 | 50MB enforcement | SPEC-909 | limit_tests | ⚠️ Partial |
| **Evidence** | E.3 | Archival (>30 days) | SPEC-909 | evidence_archival_tests | ✅ |
| **Evidence** | E.4 | Integrity (SHA256) | - | evidence_integrity_tests | ✅ |
| **State** | S.1 | Persistence | #2 | S04-S06 | ✅ |
| **State** | S.2 | Recovery | - | E02-E05 | ✅ |
| **State** | S.3 | Concurrency | - | C01-C10 | ✅ |
| **Cross-cutting** | X.1 | Telemetry | - | schema_tests | ✅ |
| **Cross-cutting** | X.2 | Error handling | - | E06-E10 | ✅ |
| **Cross-cutting** | X.3 | Security (GR-*) | GR-001-013 | guardrail_tests | ✅ |
| **Cross-cutting** | X.4 | Configuration | - | config_tests | ✅ |

**Priority Actions:**
1. (none - E.3/E.4 gaps closed, tests implemented in evidence_archival_tests.rs and evidence_integrity_tests.rs)

---

## 7. ACE + Maieutics Enforcement Plan (COMPLETE — H0-H7 DECIDED)

### Decided (Session 9)

| Component | Status | Description | Source |
|-----------|--------|-------------|--------|
| **Artifact Types** | ✅ DECIDED | Two canonical artifacts: Maieutic Spec (pre) + ACE Frames (during/post) | H0 |
| **Explanation Triggers** | ✅ DECIDED | Tiered: mandatory for failures/overrides/boundaries; selective for interesting outcomes; on-demand available | H1 |
| **Control Model** | ✅ DECIDED | Tiered autonomy: auto → checkpoint → explicit → manual; Maieutic Spec is delegation contract | H2 |

### Decided (Session 10: H3-H7)

| Component | Status | Decision | Source |
|-----------|--------|----------|--------|
| Maieutic Elicitation | ✅ DECIDED | Always mandatory pre-execution (fast path allowed) | H3 |
| Explainability vs Capture | ✅ DECIDED | Artifacts follow capture mode; capture=none persists nothing | H4 |
| Gating Rules | ✅ DECIDED | Ship hard-fail if artifacts missing; no override | H5 |
| Multi-Surface Parity | ✅ DECIDED | A2-aligned tiered: Tier 1 full parity, Tier 2 TUI-first | H6 |
| ACE Frame Schema | ✅ DECIDED | Generated JSON Schema from Rust structs via schemars | H7 |

### Enforcement Rules (From H0-H5)

| Rule | Enforcement | Test |
|------|-------------|------|
| No ship without milestone ACE frame | Hard block at Ship stage | `test_ship_requires_ace_frame` |
| No override without ACE frame | Hard block on GateDecision | `test_override_requires_ace_frame` |
| Stage transitions emit summary ACE | Emit at state machine transition | `test_stage_transition_ace_frame` |
| Tier 3 actions require explicit approval | Real-time pause before execution | `test_tier3_requires_approval` |
| Tier 4 actions are human-only | No automation path exists | `test_tier4_manual_only` |
| Tier crossing escalates | Pause and request approval | `test_tier_crossing_escalates` |
| **Maieutic step required before execution** | Pipeline cannot proceed until complete | `test_maieutic_required_before_execute` |
| **capture=none persists no artifacts** | No Maieutic Spec or ACE frames written | `test_capture_none_no_persisted_artifacts` |
| **capture=none cannot ship** | Hard block at ship with actionable message | `test_capture_none_blocks_ship` |
| **Ship requires persisted Maieutic + ACE** | Hard block if artifacts absent | `test_ship_requires_maieutic_and_ace` |
| **Headless requires maieutic input** | Fail with NEEDS_INPUT exit code if missing | `test_headless_requires_maieutic_input` |
| **Headless never prompts** | Hard assertion: no interactive codepath | `test_headless_never_prompts` |
| **Headless NEEDS_APPROVAL exit code** | Halt with exit code when Tier 2/3 approval needed | `test_headless_needs_approval_exit_code` |
| **Shared executor same artifacts** | TUI/CLI/headless produce same Tier 1 payloads | `test_shared_executor_same_core_artifacts` |
| **ACE Frame schema generation stable** | Generated schema matches checked-in file | `test_ace_frame_schema_generation_stable` |
| **ACE Frame examples validate** | Canonical frames pass schema validation | `test_ace_frame_examples_validate` |
| **Schema version field required** | Frames without schema_version fail validation | `test_schema_version_field_required` |

---

## 8. Resurrection Map (Initialized)

| Capability | Archived Spec IDs | Current Status | Proposed Equivalent | Reinstate/Harvest/Keep | New SPEC/Task | Acceptance Tests |
|------------|-------------------|----------------|---------------------|------------------------|---------------|------------------|
| Consensus artifacts | SPEC-KIT-068, SPEC-KIT-904 | Deprecated | ACE Frames + Maieutic Specs | **REPLACED** (H0) | D127 | ACE schema validation, milestone ACE tests |
| Quality gates | SPEC-KIT-941 | Partial | Blocking-with-override + GateDecision events | **HARVEST** | D123 | Gate flow tests |
| Evidence cleanup | SPEC-KIT-909 | Scripts exist | Tiered maintenance (Health → Cleanup → Librarian) | **HARVEST** | D126 | E.3, E.4 tests |
| Librarian | SPEC-KIT-103 | Modules exist | Weekly scheduled maintenance | **HARVEST** | F1/F2 | Librarian tests |
| OAuth multi-provider | SPEC-KIT-947 | Implemented | Keep as-is | **KEEP** | - | Existing tests |
| Pipeline modularity | SPEC-KIT-948 | Implemented | Monolith + internal seams | **KEEP** (D122) | - | Config tests |

---

## 9. RESUME PROMPT

```markdown
# ARB Pass 2 — COMPLETE

All questions decided. Phase 1 (A1-G3) and Phase 2 (H0-H7) complete.

## Final Decision Summary

### Phase 1 (Sessions 8): Architecture Foundation
- A1-A2: Product identity, tiered surface parity
- B1-B2: Evidence store SOR, hybrid retention
- C1-C2: Capture enforcement, policy-derived defaults
- D1-D2: Monolith pipeline, blocking-with-override gates
- E1-E2: Hard-fail-core-only, policy sovereignty = Tier 1
- F1-F2: Tiered maintenance, Health Check first
- G1-G3: Hybrid verification, capability matrix, three-layer regression

### Phase 2 (Sessions 9-10): ACE + Maieutics as Core Identity
- H0: ACE Frames + Maieutic Specs replace consensus (D127)
- H1: Tiered explanation scope — 5 tiers (D128)
- H2: Tiered autonomy with delegation — 5 tiers (D129)
- H3: Maieutic always mandatory pre-execution (D130)
- H4: Explainability follows capture mode (D131)
- H5: Ship hard-fail without artifacts (D132)
- H6: A2-aligned multi-surface parity (D133)
- H7: Generated ACE Frame schema (D134)

## Key Reconciliation Statements
1. capture=none is "private scratch mode": can run, cannot ship
2. Headless requires pre-supplied maieutic answers; never prompts
3. ACE Frame schema versioned and immutable once released

## Locked Decisions: D113-D134 (22 new decisions)

## Implementation Status
All post-ARB tasks complete:
- D113-D134 locked in DECISION_REGISTER.md (5c96e0fe4)
- Enforcement test suite added (PR #10)
- ACE Frame JSON Schema v1 generated (PR #9)
- Headless maieutic executor updated (b9415695f)
```

---

*End of ARB Pass 2*
*Phase 1 + Phase 2 COMPLETE. 22 decisions locked (D113-D134).*
