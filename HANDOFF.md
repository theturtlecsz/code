# Pair Programming Review — Gate Policy Alignment (2025-12-18)

This note is intended as an implementation target you can hand to a coder.
Scope: vocabulary + architectural alignment between **Gate Policy** docs and the current **consensus.rs** implementation.

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

## 8) Recommended next PRs

1. **Rename + compatibility layer**
   - `SPEC_KIT_CRITIC` -> `SPEC_KIT_SIDECAR_CRITIC` (+warn-once)
   - `consensus_threshold` -> `min_confidence_for_auto_apply` (+warn-once)

2. **Refactor naming**
   - `consensus.rs` -> `gate_policy.rs` (re-export old module path temporarily)
   - `run_spec_consensus()` -> `evaluate_gate()` (keep old fn as wrapper)

3. **Remove model routing duplication**
   - replace `preferred_agent_for_stage()` with `preferred_role_for_stage()` (router handles worker/model)

4. **Quarantine or delete legacy voting path**
   - compile-time feature or deeply isolated module so it cannot drift back into default behavior
