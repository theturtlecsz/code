# ARB Session Handoff - Section H (ACE + Maieutics)

**Generated:** 2026-01-19
**From:** Session 8 (Phase 1 Complete)
**To:** Session 9+ (Phase 2: Section H)

---

## Quick Summary

**Phase 1 (A1-G3): COMPLETE**
All 15 architectural questions decided. Decisions recorded in `ARCHITECT_REVIEW_BOARD_OUTPUT.md`.

**Phase 2 (Section H): PENDING**
8 new questions defining ACE + Maieutics as core product identity.

---

## All Decisions Made (Phase 1)

### Product & Evidence Store

| ID | Decision | Summary |
|----|----------|---------|
| A1 | Option 3 (modified) | Spec-to-ship automation, human-controlled; evidence store is trust backbone |
| A2 | Option 5 | Tiered parity: core=full parity; UX=TUI-first |
| B1 | Option 2 | Events + projections; events authoritative |
| B2 | Option 5 | Hybrid retention: 90d TTL + milestone protection (1yr) |
| C1 | Option 5 | Checkpoint-gated; over-capture blocked; under-capture warned |
| C2 | Option 4 | Policy-derived default; template uses `prompts_only` |

### Pipeline & Enforcement

| ID | Decision | Summary |
|----|----------|---------|
| D1 | Option 1 | Monolith with internal trait seams |
| D2 | Option 2 | Blocking-with-override; GateDecision events |
| E1 | Option 2 | Hard-fail-core-only |
| E2 | Confirm+adjust | Tier 1 = policy sovereignty (over-capture, non-logical URIs, SOR violations) |

### Maintenance & Verification

| ID | Decision | Summary |
|----|----------|---------|
| F1 | Option 4 | Tiered triggers (event + scheduled + on-demand) |
| F2 | Option 1 | Health Check first |
| G1 | Option 4 | Hybrid (contracts + workflow tests) |
| G2 | Confirm | Prioritize E.3/E.4 gaps |
| G3 | Three-layer | Property + Golden + Snapshot |

---

## New Decisions to Lock (D113-D126)

```
D113: Tiered parity model
D114: Events are authoritative SOR; projections rebuildable
D115: Lazy snapshots deferred until measured need
D116: Hybrid retention (TTL + milestone protection)
D117: Milestone markers: SpecCompleted, ReleaseTagged, MilestoneMarked, Stage6
D118: 90d TTL; 1yr milestone; configurable
D119: Over-capture = Tier 1 absolute hard-fail
D120: Under-capture = checkpoint-blocked until resolved/acknowledged
D121: CaptureGapAcknowledged events
D122: Monolith + internal seams; no plugins/actors
D123: Blocking-with-override + GateDecision events
D124: Policy-derived capture default; template = prompts_only
D125: Policy sovereignty = Tier 1 absolute
D126: Tiered maintenance; Health Check first
```

---

## Critical Product Identity Framing

**NON-NEGOTIABLE:**
1. **Spec-Kit is the product** — "spec-to-ship automation with human control & explainability"
2. **Memvid is a backend** — use "evidence store (backend currently Memvid)"
3. **ACE + Maieutics are core identity** — explicitly REPLACE the prior "consensus model"

---

## Section H Questions (Pending)

| ID | Question | Purpose |
|----|----------|---------|
| H0 | Consensus model replacement | Define what replaces old consensus artifacts |
| H1 | ACE explanation scope | When must ACE frames be generated |
| H2 | Control model | How user controls automation |
| H3 | Maieutic spec elicitation enforcement | How mandatory is Socratic interview |
| H4 | Explainability artifacts vs capture mode | Storage when capture=none |
| H5 | Gating & testing for ACE/maieutics | Do missing outputs block progress |
| H6 | Multi-surface parity for explainability | TUI/CLI/headless requirements |
| H7 | ACE Frame Schema contract | JSON schema formality |

---

## Research Workflow (Per Question)

1. **Web research**: ≥6 sources (2 authoritative, 1 security, 1 practitioner, 1 implementation, 1 counterpoint)
2. **Repo evidence**: Check archived specs in `docs/archive/specs/`
3. **Options**: ≥3 with pros/cons/best-when/risk-flags
4. **ACE/Maieutic impact notes**: How option affects explainability
5. **Ask for choice**: Option # + rationale

---

## Key Files to Read

```
ARCHITECT_REVIEW_BOARD_OUTPUT.md  # Current state + decisions
ARCHITECT_QUESTIONS.md            # Question definitions
codex-rs/SPEC.md                  # Invariants
codex-rs/docs/DECISION_REGISTER.md
codex-rs/docs/MODEL-POLICY.md
codex-rs/model_policy.toml
docs/archive/specs/               # Historical specs
```

---

## RESUME PROMPT (Copy This to Start New Session)

```markdown
# ARB Pass 2 Resume - Session 9 (Section H: ACE + Maieutics)

You are an Architecture Review Board facilitator + researcher for Codex-RS / Spec-Kit.

## Context
- Session 9 of N
- Phase 1 (A1-G3): COMPLETE — all decisions in ARCHITECT_REVIEW_BOARD_OUTPUT.md
- Phase 2 (Section H: ACE + Maieutics): STARTING

## Critical Identity (Non-Negotiable)
1. Spec-Kit is the product: "spec-to-ship automation with human control & explainability"
2. Memvid is a backend: use "evidence store (backend currently Memvid)"
3. ACE + Maieutics REPLACE the prior "consensus model" — this is core identity

## Locked Decisions (Do Not Re-Litigate)
- A1: Spec-to-ship automation, human-controlled
- C1: Checkpoint-gated enforcement; over-capture blocked
- D123: GateDecision events for overrides
- D125: Policy sovereignty = Tier 1 hard-fail

## Session 9 Targets
Begin Section H (3 questions):
- **H0**: Consensus model replacement — What replaces old consensus artifacts?
- **H1**: ACE explanation scope — When must ACE frames be generated?
- **H2**: Control model — How does user control automation?

## Research Workflow (Per Question)
1. Web research (≥6 sources: authoritative, security, practitioner, implementation, counterpoint)
2. Repo evidence from archive specs
3. Options (≥3) with pros/cons/best-when/risk-flags/ACE-impact
4. Decision rubric
5. Ask me to choose: Option # + rationale

## Neutrality
- Do NOT decide for me
- Provide situational recommendations ("if priority X, option Y tends to fit")
- Always ask me to choose

## Key Files
- `ARCHITECT_REVIEW_BOARD_OUTPUT.md` — current state
- `ARCHITECT_QUESTIONS.md` — question definitions
- `codex-rs/SPEC.md` — invariants
- `docs/archive/specs/` — historical specs for consensus/ACE patterns

## Output
After H0, H1, H2:
1. Update ARCHITECT_REVIEW_BOARD_OUTPUT.md with decisions
2. Output STATE + RESUME PROMPT for next session

START: Read ARCHITECT_REVIEW_BOARD_OUTPUT.md, then begin H0 research brief.
```

---

## Files Modified This Session

| File | Action |
|------|--------|
| `ARCHITECT_REVIEW_BOARD_OUTPUT.md` | Created/updated with all Phase 1 decisions |
| `ARCHITECT_QUESTIONS.md` | Created with all questions + quick format |
| `ARB_HANDOFF.md` | This file — session handoff context |

---

*End of Handoff*
