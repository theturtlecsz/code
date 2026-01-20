# Architecture Review Board - Decision Questions

**Generated:** 2026-01-19
**Source:** ARCHITECT_REVIEW_RESEARCH.md (Pass 1 Complete)
**Purpose:** Consolidated decision questions for architect review

---

## Decisions Already Made (Sessions 1-8)

| Question | Decision | Summary |
|----------|----------|---------|
| **A1** | Option 3 (modified) | Spec-to-ship automation, human-controlled; Memvid as trust backbone |
| **A2** | Option 5 | Tiered parity: core automation = full parity; visualization = TUI-first |
| **B1** | Option 2 | Raw + Derived (events + projections); events authoritative |
| **B2** | Option 5 | Hybrid retention: TTL + milestone protection |
| **C1** | Option 5 | Checkpoint-gated enforcement; over-capture blocked, under-capture warned |

---

## Remaining Questions

### C2: Default Capture Mode

**What default capture mode for new specs/runs?**

| Option | Description |
|--------|-------------|
| **1** | `none` — No LLM I/O captured; events only; maximum privacy |
| **2** | `prompts_only` — Prompts captured, responses not; current default |
| **3** | `full_io` — Complete request/response pairs; maximum fidelity |
| **4** | Policy-Derived — Default set in model_policy.toml per workspace |
| **5** | `prompts_only` + `full_io` for Debug — Elevate when debugging |
| **6** | `prompts_only` + `full_io` for Ship Milestones — Elevate at ship points |

**Key considerations:**
- A1 said "trustworthy narrative"; needs enough data to explain "why"
- Storage budget: 25-50MB per SPEC
- Current default: `prompts_only`
- GDPR: Privacy by default suggests minimal capture

---

### D1: Pipeline Modularity

**What architectural pattern for the spec-kit pipeline?**

| Option | Description |
|--------|-------------|
| **1** | Monolith — Single codebase, all stages in one binary (current state) |
| **2** | Plugin-per-stage — Dynamic loading, stages as separate libraries |
| **3** | Trait-based Injection — Compile-time polymorphism via Rust traits |
| **4** | Full Actor Model — Message-passing concurrency (SEDA pattern) |

**Key considerations:**
- Already has enum-based design (`StageType` with 8 stages)
- SPEC-KIT-931 rejected Actor Model as "too complex"
- Fixed stage count (8) reduces extensibility need
- Single-writer model eliminates coordination concerns

---

### D2: Quality Gates Posture

**What enforcement posture for quality gates?**

| Option | Description |
|--------|-------------|
| **1** | Warn-only — Non-blocking warnings, developer discretion |
| **2** | Blocking-with-override — Block by default, explicit bypass available |
| **3** | Hard-fail — No exceptions, gate failure stops pipeline |
| **4** | Progressive Escalation — Start permissive, tighten over time |

**Key considerations:**
- Already implements hybrid: `SignalSeverity::Advisory` vs `SignalSeverity::Block`
- Confidence-based auto-apply: >= 0.65 threshold for advisory signals
- Tool-truth (compile/test/lint) always escalates
- GR-001 enforces single-owner pipeline

---

### E1: Enforcement Strictness

**What enforcement posture across the pipeline?**

| Option | Description |
|--------|-------------|
| **1** | Soft-fail-everywhere — All gates warn, developer discretion |
| **2** | Hard-fail-core-only — Compile/tests/critical-security hard-fail, others soft |
| **3** | Hard-fail-all — Every gate blocks, no bypass mechanism |
| **4** | Graduated-by-stage — Early stages permissive, late stages strict |

**Key considerations:**
- Already implements Hard-fail-core-only with graduated characteristics
- Tool-truth (compile, test, lint): Always block
- Block-severity signals: Always escalate
- Advisory signals: Auto-apply if confidence >= 0.65
- Stage progression: Each stage has harder gates than previous

---

### E2: Hard-fail Policy Set

**Which specific gates should have no bypass?**

**Proposed Tiers:**

| Tier | Gates | Policy |
|------|-------|--------|
| **Tier 1: Absolute Hard-Fail** | Compile, TypeCheck, SchemaValidation, SafetyRisk, Critical Security (CVSS 9.0+) | No bypass ever |
| **Tier 2: Hard-Fail + Override** | UnitTests, IntegrationTests, PolicyViolation (GR-*), High Security (CVSS 7.0-8.9) | Documented override |
| **Tier 3: Configurable** | Lint (warnings), HighRiskChange, Contradiction, Medium Security (CVSS 4.0-6.9) | Soft by default |
| **Tier 4: Advisory Only** | Format, Ambiguity, MissingAcceptanceCriteria, PerformanceRisk, Low Security | Never blocks |

**Decision needed:** Confirm or adjust tier assignments.

---

### F1: Maintenance Framework Timing

**When should maintenance jobs run?**

| Option | Description |
|--------|-------------|
| **1** | Continuous — Always running, polling for work |
| **2** | Scheduled — Fixed cron schedules (daily, weekly) |
| **3** | Event-triggered — Fire on specific events (pipeline complete, threshold crossed) |
| **4** | Tiered — Hybrid: event + scheduled + on-demand triggers |

**Recommended Tiered Pattern:**

| Tier | Trigger | Jobs |
|------|---------|------|
| Event | Pipeline complete, evidence write | Health checks, evidence limit warnings |
| Daily | Cron 2 AM | Evidence cleanup >30 days, stats |
| Weekly | Cron Sunday 3 AM | Librarian repair, meta-memory synthesis |
| On-demand | User request | Compaction, graph repair, emergency |

---

### F2: First Maintenance Job Family

**Which job family to implement first?**

| Option | Description | Status |
|--------|-------------|--------|
| **1** | Health Check — System observability, resource monitoring, integrity verification | NOT IMPLEMENTED |
| **2** | Evidence Cleanup — Enforce 50MB limit, archive >30 days, purge >180 days | PARTIAL (scripts exist) |
| **3** | Librarian — Meta-memory synthesis, graph repair, causal relationship labeling | PARTIAL (modules exist) |
| **4** | Compaction — Retention hardening, orphan pruning, history truncation | BACKLOG (SYNC-026) |

**Recommended Order:** Health Check → Evidence Cleanup → Librarian → Compaction

**Dependencies:**
```
Health Check (P1, read-only)
    ↓
Evidence Cleanup (P0, write: archive, purge)
    ↓
Librarian (P2, write: classify, template, relate)
    ↓
Compaction (P3, write: consolidate, prune)
```

---

### G1: Cross-cutting Capability Verification

**How to validate all components work together?**

| Option | Description |
|--------|-------------|
| **1** | E2E tests only — Full pipeline runs testing complete workflows |
| **2** | Contract testing — Define and verify inter-component contracts |
| **3** | Component tests with mocks — Test groups of services as isolated units |
| **4** | Hybrid approach — Combine contracts, component tests, and E2E |

**Current State:**
- W01-W15 workflow tests cover handler → consensus → evidence → guardrail → state
- MockMcpManager enables fixture-based determinism
- 604 tests total (unit: 135, integration: 256+, workflow: 60, property: 35)

**Recommendation:** Hybrid Contract + Workflow Testing
- Keep W01-W15 workflow tests
- Add explicit contracts for Stage/Gate/Evidence interfaces
- Formalize traceability matrix mapping SPEC.md invariants to tests

---

### G2: Capability Matrix Definition

**What's the canonical list of capabilities that must be tested?**

| Category | Capabilities | Test Coverage |
|----------|-------------|---------------|
| **Pipeline (P)** | Stage transitions, dependencies, skip conditions | W01-W15 ✅ |
| **Quality (Q)** | Gates, checkpoints, escalation | Q01-Q10 ✅ |
| **Evidence (E)** | Lifecycle, limits, archival | Partial ⚠️ |
| **State (S)** | Persistence, recovery, concurrency | E01-E15, C01-C10 ✅ |
| **Cross-cutting (X)** | Telemetry, errors, security | Various ✅ |

**Gaps Identified:**
- E.3: Archival (>30 days) — No tests
- E.4: Integrity verification (SHA256) — No tests

**Decision needed:** Confirm capability categories and prioritize gap closure.

---

### G3: Regression Prevention

**How to prevent capability regressions during refactoring?**

| Option | Description |
|--------|-------------|
| **1** | Characterization tests — Capture current behavior before refactoring |
| **2** | Property-based testing — Verify invariants across random inputs |
| **3** | Snapshot/golden file testing — Compare outputs to known-good references |
| **4** | Contract versioning — Version interfaces to detect breaking changes |
| **5** | Mutation testing — Verify tests catch injected bugs |

**Current State:**
- proptest: 2,560+ generated test cases for property-based testing
- insta: VT100 terminal output snapshots

**Recommendation:** Three-Layer Defense
1. **Property-Based Invariant Layer**: Expand proptest to 40+ properties covering all SPEC.md invariants
2. **Golden Master Layer**: Maintain fixture-based approach; add semantic drift detection
3. **Snapshot Layer**: Refine insta usage; mask volatile fields

---

## Quick Decision Format

For each question, provide:
```
[Question ID]: Option [#] — [1-2 sentence rationale]
```

Example:
```
C2: Option 2 — prompts_only balances privacy with "trustworthy narrative" and fits storage budget.
D1: Option 1 — Monolith is sufficient; 8 fixed stages don't need plugin extensibility.
D2: Option 2 — Blocking-with-override matches current implementation and provides emergency escape.
E1: Option 2 — Hard-fail-core-only is already implemented; formalizes current behavior.
E2: Confirm tiers — Accept proposed tier assignments with minor adjustments.
F1: Option 4 — Tiered combines responsiveness with efficiency.
F2: Option 1 — Health Check first; foundation for all other maintenance.
G1: Option 4 — Hybrid already in place; formalize contracts.
G2: Confirm matrix — Accept categories; prioritize E.3/E.4 gaps.
G3: Three-layer — Property + Golden + Snapshot defense.
```

---

## New Decisions to Lock (Proposed)

Based on decisions made in Sessions 1-8:

| ID | Decision | Spec |
|----|----------|------|
| D113 | Tiered parity: Tier 1 (automation-critical) = full parity; Tier 2 (visualization) = TUI-first | A2 |
| D114 | Events + immutable artifacts are authoritative SOR; projections are rebuildable | B1 |
| D115 | Lazy snapshots deferred until measured latency exceeds thresholds | B1 |
| D116 | Hybrid retention: TTL for routine events, milestone protection for ship points | B2 |
| D117 | Milestone markers: SpecCompleted, ReleaseTagged, MilestoneMarked, Stage 6 completion | B2 |
| D118 | Default TTL: 90 days; milestone protection: 1 year minimum; configurable via policy | B2 |
| D119 | Over-capture (more than policy permits) is always hard-blocked immediately | C1 |
| D120 | Under-capture is warned during work, blocked at checkpoint until resolved/acknowledged | C1 |
| D121 | Capture gap acknowledgments create auditable CaptureGapAcknowledged events | C1 |

---

*End of Questions Document*
