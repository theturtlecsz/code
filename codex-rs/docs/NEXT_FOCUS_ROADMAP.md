# Planner / Spec-Kit — Next Focus Roadmap (Architect Review)

**Date:** 2025-12-19 (Updated: post-automation-review)
**Context:** Post PR7-PR9 (gate/review vocabulary migration + CI hardening complete)

This roadmap is written from an architect / product posture perspective. It assumes:
- Gate/Review vocabulary migration + legacy voting deletion are complete.
- CI hardening is in place (vocabulary drift canary + golden wire-format tests).

---

## CRITICAL UPDATE: Automation Before Gold Run

**Key insight:** Claude Code can implement changes but cannot drive `/speckit.auto` without manual TUI interaction. This blocks CI/CD and repeatable testing.

**Re-prioritization:**
- **P0 = SPEC-KIT-920** (automation/headless command injection) — enables everything else
- **P0.5 = Smoke Spec** (lightweight, not SPEC-KIT-900)
- **P1 = SPEC-KIT-926** (progress visibility)

**Why SPEC-KIT-900 is wrong for gold run:**
- It's a heavyweight integration benchmark (Stage0, NotebookLM, external deps)
- Too many moving parts for a "does it work?" test
- Keep it as quarterly/release-candidate integration test

**Smoke Spec criteria:**
1. In-repo (no external benchmark workspace)
2. No Stage0 dependency for happy path
3. Deterministic acceptance (clear pass/fail)
4. Small surface area
5. Exercises pipeline enough to validate gating + evidence

---

## SPEC-KIT-920 Status (Automation)

**Current state:**
- `--initial-command` EXISTS and works (dispatches slash command after first redraw)
- `--exit-on-complete` EXISTS in CLI but **NOT IMPLEMENTED** (app never reads it)

**Minimum viable scope:**
1. Implement `--exit-on-complete` to actually exit after command completes
2. Exit code reflects success (0) / failure (non-zero)
3. Optional: `--json` for structured progress output

**Files:**
- `tui/src/cli.rs:113` - flag defined
- `tui/src/app.rs` - needs implementation to detect pipeline completion and exit

---

## 1) Current posture (what's strong)

### Engineering posture
- **Conceptual clarity improved:** "Voting consensus" is out; **deterministic gate evaluation + escalation** is in.
- **Compatibility discipline:** wire formats are locked with **golden serialization tests**, and deserialization supports aliases.
- **Regression prevention:** a vocabulary drift script prevents new "consensus" surface area from creeping back in.

### Product posture
- The core product loop is coherent:
  1) create a SPEC
  2) generate staged artifacts (plan/tasks/implement/validate/audit/unlock)
  3) store evidence for auditability

The "evidence-driven workflow automation" value proposition is real and differentiated.

---

## 2) Biggest risks right now

### Risk A — Product narrative drift (docs vs reality)
Some top-level docs still read like a **multi-agent voting / arbiter** system. Post-migration, the system is better described as:
- **single-owner stages**
- optional **sidecars** that emit **signals**
- **gate evaluation** that decides Auto-Apply vs Escalate

If docs retain the old story, you'll keep paying an onboarding tax and you increase the chance of future feature work re-introducing "voting semantics" accidentally.

### Risk B — User trust gap (visibility)
Long-running stages (minutes) without rich progress feedback will keep causing:
- premature cancels
- "is it hung?" confusion
- reduced willingness to run `/speckit.auto`

### Risk C — "Production ready" claim without a canonical gold run
The codebase can have great unit/integration coverage and still be brittle in real runs.

A canonical **end-to-end dogfood run** (and preserved evidence) is the fastest way to find the remaining operational foot-guns.

### Risk D — Setup friction
Stage0 / notebook integration and per-project config behavior are usually the "death by papercuts" blockers for adoption.

---

## 3) Where to focus next (priority order)

### P0 — Prove the happy path with a canonical gold run
**Goal:** One clean, repeatable `/speckit.auto` run that produces a full evidence chain.

**Why now:** it validates that the gate policy + router boundaries aren't only "correct in theory" but survive real execution.

**Definition of Done:**
- A published "gold run" SPEC (e.g., SPEC-KIT-900) with:
  - artifacts for every stage
  - gate/review evidence files
  - cost summary + timing
  - zero manual intervention other than initiation

**Deliverable:** a small "Gold Run Playbook" doc so every contributor can reproduce.

---

### P1 — Ship trust: TUI progress + status visibility (update SPEC-KIT-926)
**Goal:** Make execution observable in real time.

**Key principle:** the UI should expose **stage progress + worker progress + gate verdicts**, not implementation details (tmux, file paths, etc.).

**Recommended UX contract:**
- "Pipeline overview" before starting
- per stage:
  - current stage name + elapsed time
  - current worker (role, provider, streaming state)
  - last meaningful heartbeat ("waiting on tool run", "waiting on model output", "evaluating gate")
- on gate evaluation:
  - verdict (Auto-Apply/Escalate)
  - confidence level
  - block vs advisory signals
  - link to evidence directory

**Definition of Done:**
- a user can tell within **5 seconds** what's happening and whether it's making progress.

---

### P1 — Reduce setup friction (MAINT-12 + MAINT-13)
This is where adoption dies if left too late.

**MAINT-12 (Stage0 HTTP-only + no MCP dependency)**
- Make Stage0 integrations "boringly reliable": explicit endpoints, explicit health checks, explicit error messages.

**MAINT-13 (project config inheritance)**
- Config must apply predictably when running from repo subdirectories.

**Definition of Done:**
- running `/speckit.*` from any subdirectory uses the intended project config
- Stage0 failures are diagnosed in one screen (what is misconfigured, what to do)

---

### P2 — Rewrite product requirements to match the new truth
Do this after P0/P1 so you document reality, not a moving target.

**Target changes:**
- Replace "consensus/voting/arbiter" language with:
  - stage owner
  - signals
  - decision rule
  - escalation
- Recast "tiering" as "how many workers are invoked" rather than "how many agents must agree."

**Definition of Done:**
- A new engineer reading `product-requirements.md` can accurately predict runtime behavior.

---

## 4) What to *not* prioritize right now

- **More vocabulary churn** (beyond targeted doc updates). You already have a drift canary; keep migration opportunistic.
- **Big upstream merges** while you still have P0/P1 product gaps. Do it once the gold run + progress visibility are stable.
- **New orchestration patterns** ("debate", "committee", "synthesis"). The current direction (signals + deterministic gates) is the right complexity level.

---

## 5) Suggested next sprint structure (2-3 weeks)

### Sprint A (Stability & proof)
- Gold run SPEC end-to-end
- Fix issues surfaced
- Document reproducibility

### Sprint B (Trust & UX)
- Update SPEC-KIT-926 spec to remove tmux + voting assumptions
- Implement progress/status in TUI
- Add one E2E test that asserts "progress events emitted"

### Sprint C (Adoption)
- Stage0 friction removal
- Config inheritance
- Update product requirements + planning docs

---

## 6) Why keep any "multi-worker" behavior (without voting)?

The **product-justified** reasons to involve >1 worker are:

1. **Specialization by responsibility**
   Architect/Planner ≠ Implementer ≠ Auditor. Reduces prompt overload and raises determinism.

2. **Risk-driven second opinions**
   Sidecar signals catch contradictions, unsafe commands, missing acceptance criteria—*without* pretending disagreement can be averaged into correctness.

3. **Evidence quality**
   Separate roles produce more structured evidence (and cleaner audit trails) than one model doing everything.

What you should *not* claim as a reason anymore:
- "Truth by agreement" (voting consensus) — correctly removed in PR6.
