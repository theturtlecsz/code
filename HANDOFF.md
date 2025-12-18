# Pair Programming Review — Gate Policy Alignment (2025-12-18)

This note is intended as an implementation target you can hand to a coder.
Scope: vocabulary + architectural alignment between **Gate Policy** docs and the current **consensus.rs** implementation.

---

## Progress Tracker

| PR | Status | Description |
|----|--------|-------------|
| **PR1** | ✅ Complete | Define canonical types (Role, Stage, Signal, Verdict, Router trait) in `codex-rs/spec-kit/` |
| **PR1.1** | ✅ Complete | Fix P0 env boundary leak, align thresholds to 0.65, add PolicyToggles |
| **PR2** | 🔲 Pending | Config alias (`consensus_threshold` → `min_confidence_for_auto_apply`) |
| **PR3** | 🔲 Pending | Env var alias (`SPEC_KIT_CRITIC` → `SPEC_KIT_SIDECAR_CRITIC`) |
| **PR4** | 🔲 Pending | Module rename (`consensus.rs` → `gate_policy.rs`) + callsite migrations |
| **PR6** | 🔲 Pending | Delete or feature-gate legacy voting path |

### PR1 + PR1.1 Commits
- `a29f9668e` - fix(spec-kit): make gate policy deterministic, align thresholds (PR1.1)
- `89b7a83e6` - feat(spec-kit): add gate policy and router contracts (PR1)

### Files Created
- `codex-rs/spec-kit/src/gate_policy.rs` — Domain vocabulary (Stage, Role, Signal, Verdict, etc.)
- `codex-rs/spec-kit/src/router.rs` — Router trait and WorkerSpec
- Updated `codex-rs/spec-kit/src/lib.rs` — Re-exports
- Updated `docs/spec-kit/GATE_POLICY.md` — Added §11 Wiring Guidance

---

## 1) What's already true (design intent)

- **Single-owner pipeline is the default**: one stage owner produces the artifact, gates evaluate signals, and escalation is deterministic.
- **No voting / no committee synthesis**: sidecars can contribute *signals* (block/advisory), but do not produce "competing answers" that get compared/merged.
- **Policy is separated from routing**:
  - **Gate Policy** = what must happen (signals, decision rules, escalation triggers).
  - **Model Policy** = who executes it (role → worker → model/provider mapping).

---

## 2) What landed (confirmed in main)

- `docs/spec-kit/GATE_POLICY.md` added with explicit "This is not voting" callout.
- Confidence is defined as computed from `owner_confidence` plus evidence/counter-signals, with explicit thresholds.
- "Quorum" renamed to "Decision Rule" to avoid voting semantics.
- Checkpoint naming table now matches actual pipeline boundaries (with a legacy naming note).

---

## 3) The remaining mismatch (doc vs code)

Today, the implementation still uses **"consensus" vocabulary** in the runtime path (e.g., `run_spec_consensus`, `consensus_ok`, `/spec-consensus`, `SPEC_KIT_CRITIC`).

That's okay as a transition, but the mismatch causes confusion and slows onboarding.

**Goal:** rename + reshape the code so "gate evaluation" is the primary concept and "legacy consensus voting" becomes a quarantined, deprecated optional mode.

---

## 4) Target architecture for the coder (minimal, crisp)

### 4.1. Core types

- `Signal`: structured observation produced by stage owner, tests, or sidecars.
  - fields: `source`, `severity {advisory|block}`, `kind`, `summary`, `evidence_refs[]`
- `CounterSignal`: same shape as Signal, but used specifically to reduce confidence / trigger escalation.

- `GateVerdict`:
  - `resolution {AutoApply|Escalate}`
  - `confidence_level {High|Medium|Low}`
  - `effective_confidence: f32` (optional but recommended)
  - `signals[]` and `counter_signals[]`
  - `reason`

### 4.2. Decision rule (deterministic)

Inputs:
- `owner_confidence` (0..1)
- `counter_signals` (block/advisory)
- evidence (tests pass/fail, policy violations, high-risk flags)

Compute:
- **effective_confidence**
  - simplest rule: start at `owner_confidence`, subtract penalties for counter-signals/tests, clamp 0..1
- **confidence_level**
  - High: >= 0.80 and no block-level counter-signal
  - Medium: >= 0.60 and no block-level counter-signal
  - Low: otherwise

Resolution:
- `Escalate` if any block-level counter-signal OR tests fail OR High-Risk flag OR confidence_level=Low.
- `AutoApply` only when confidence_level in {High, Medium} and no block-level counter-signals.

### 4.3. Routing separation (important)

Gate evaluation should NOT hardcode "Gemini vs Claude" (or any model names).
Instead:
- map stage -> **role** (Architect/Implementer/Validator/Judge)
- ask the router/policy engine to pick a worker for that role (per `docs/MODEL-POLICY.md`)

This prevents "Model Policy duplication" and keeps gate policy stable.

---

## 5) Migration plan (don't break users)

### 5.1 Env vars
- New: `SPEC_KIT_SIDECAR_CRITIC`
- Deprecated: `SPEC_KIT_CRITIC` (still honored, warn once)

### 5.2 Config keys
- New: `min_confidence_for_auto_apply`
- Deprecated: `consensus_threshold` (still parsed, warn)

### 5.3 JSON evidence schema
- Keep reading old fields.
- Emit new fields going forward.
- Add a schema version so dashboards can handle both.

---

## 6) Acceptance criteria (what "done" looks like)

1. UI and logs say **"Gate evaluation"** not "consensus" in the default path.
2. There is a single place where:
   - signals are gathered
   - effective confidence is computed
   - decision rule produces a verdict
3. Model selection is handled by the model-policy/router (no stage->model hardcoding in gate-policy module).
4. Deprecated env vars/config keys still work with a warning.
5. Unit tests cover:
   - default path (no env vars) = gate evaluation only
   - sidecar enabled = signals captured, decisions unchanged unless block-level
   - legacy voting mode (if kept) is explicitly isolated + warns

---

## 7) What NOT to build

- No "debate loop," no multi-answer comparison, no "best of N outputs."
- No implicit compromises or synthesis when sources disagree.
- No dynamic agent swarm/orchestration — this is a pipeline with optional sidecars.

---

## 8) Remaining PRs (ordered by dependency)

### PR2: Config Alias (consensus_threshold → min_confidence_for_auto_apply)
- Add serde alias to config struct
- Implement warn-once on deprecated key usage
- Files: `codex-rs/spec-kit/src/config.rs`, any config loading code

### PR3: Env Var Alias (SPEC_KIT_CRITIC → SPEC_KIT_SIDECAR_CRITIC)
- Add env var alias with warn-once deprecation
- Consuming code should read from `PolicyToggles` (already wired in PR1.1)
- Files: Config loading, possibly `codex-tui/` initialization

### PR4: Module Rename + Callsite Migrations
- Rename `consensus.rs` → `gate_policy.rs` (re-export old module path temporarily)
- Rename `run_spec_consensus()` → `evaluate_gate()` (keep old fn as wrapper)
- Replace `preferred_agent_for_stage()` with `preferred_role_for_stage()` (router handles worker/model)
- Update UI labels from "consensus" to "gate evaluation"

### PR6: Delete or Feature-Gate Legacy Voting
- Compile-time feature `#[cfg(feature = "legacy_voting")]` or full deletion
- Quarantine voting code so it cannot drift back into default behavior
- Decision: recommend deletion over feature-gating (simpler, less maintenance)
