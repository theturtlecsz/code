# Gate Policy Alignment — Session Handoff

**Last updated:** 2025-12-19
**Status:** PR7-PR9 complete

---

## Progress Tracker

| PR | Status | Description |
|----|--------|-------------|
| **PR1** | ✅ Complete | Define canonical types (Role, Stage, Signal, Verdict, Router trait) in `codex-rs/spec-kit/` |
| **PR1.1** | ✅ Complete | Fix P0 env boundary leak, align thresholds to 0.65, add PolicyToggles |
| **PR2** | ✅ Complete | Config alias (`consensus_threshold` → `min_confidence_for_auto_apply`) |
| **PR3** | ✅ Complete | Env var alias (`SPEC_KIT_CRITIC` → `SPEC_KIT_SIDECAR_CRITIC`) |
| **PR4** | ✅ Complete | Module rename (`consensus.rs` → `gate_evaluation.rs`) + vocabulary migration |
| **PR6** | ✅ Complete | Delete legacy voting path (behavioral change) |
| **PR7** | ✅ Complete | Internal vocabulary alignment (deprecation notes + serde aliases + type aliases) |
| **PR8** | ✅ Complete | Clippy cleanup across spec-kit + stage0 + tui |
| **PR9** | ✅ Complete | Command UX: `/spec-review` canonical, `/spec-consensus` deprecated alias with warn-once |

### Commits (chronological)
```
b3c073b5d docs: update HANDOFF.md with PR4 completion status
d9fa6313d feat(spec-kit): gate policy vocabulary migration (PR4)
a2efa21f5 docs: finalize PR4 scope decisions and implementation plan
184ca7ca4 docs: extend DEPRECATIONS.md with gate policy renames (PR3.5)
7f7a3e8f6 docs: incorporate PR3 architectural review into PR4 plan
a5fd5b681 feat(spec-kit): env var alias SPEC_KIT_CRITIC -> SPEC_KIT_SIDECAR_CRITIC (PR3)
2a4fbb8b8 docs: comprehensive HANDOFF.md for gate policy alignment next session
959ccbeb7 docs: update HANDOFF.md with PR2 completion
f930c4643 feat(spec-kit): config alias consensus_threshold -> min_confidence_for_auto_apply (PR2)
e0097d802 docs: update HANDOFF.md with gate policy alignment target
a29f9668e fix(spec-kit): make gate policy deterministic, align thresholds (PR1.1)
89b7a83e6 feat(spec-kit): add gate policy and router contracts (PR1)
```

---

## PR2 Architectural Review — Action Items

### ✅ P0: Schema Validation Order (VERIFIED)

**Risk:** Schema validation may reject legacy config files if it validates *input* (pre-deserialize) against a schema that doesn't allow `consensus_threshold`.

**Verification result:** SAFE ✓

The schema validation is safe due to a two-layer deserialization pattern:
1. **Raw layer** (`QualityGateConfigRaw`): Accepts both `consensus_threshold` AND `min_confidence_for_auto_apply`
2. **Canonical layer** (`QualityGateConfig`): Only has `min_confidence_for_auto_apply`

**Flow:**
1. TOML/ENV → `QualityGateConfigRaw` (accepts legacy key)
2. `From` impl → `QualityGateConfig` (canonical only)
3. `serde_json::to_value(config)` → JSON with canonical field
4. Schema validates canonical JSON ✓

**Evidence:** All 20 loader tests pass, including `test_deprecated_env_var_overrides_file_canonical_key`.

### 🟡 Warning Text Improvement (Minor)

Cross-layer case: file sets canonical, env sets deprecated → warning says "both keys present" but env override changes value after warning.

**Recommendation:** Rephrase to "Both keys present; precedence will be applied; remove deprecated key."

### 🟡 Defaults Layer Masking (Document)

Lines 379-385 strip canonical key from defaults layer to prevent false "both present" detection. This is correct but affects config debugging/dumping.

**Mitigation:** Any "effective config" display should use deserialized canonical struct, not raw merged map.

---

## PR3 Design Specification (COMPLETE)

### Goal
Rename `SPEC_KIT_CRITIC` → `SPEC_KIT_SIDECAR_CRITIC` with backward compatibility and warn-once.

### Implementation Summary
- **File modified**: `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs`
- **Function updated**: `is_critic_enabled()` now checks both env vars with precedence logic
- **Warn-once helper**: `warn_once_deprecated_spec_kit_critic()` added
- **Tests added**: 4 tests covering canonical, deprecated, default, and precedence scenarios
- **Clippy clean**: No new warnings introduced

### Architectural Feedback (from review)

**A) Test isolation risk**: Tests mutate process-global env vars and require `--test-threads=1`.
- **Fix in PR4**: Extract pure decision function, unit test that, keep one integration test for env wiring.

**B) Config boundary leakage**: `consensus.rs` owns env parsing, but it should be centralized.
- **Fix in PR4**: Move policy toggles IO to `spec-kit/src/config/policy_toggles.rs`.

**C) Deprecation signaling**: Need user-facing docs listing renamed knobs.
- **Done**: Extended `docs/DEPRECATIONS.md` with:
  - Renamed env vars (PR3)
  - Renamed config keys (PR2)
  - Legacy storage naming (intentionally preserved)
  - Removal timeline

### Design Decisions (confirmed)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Precedence** | Env overrides file | Consistent with PR2 and standard layering |
| **Both set** | Canonical wins + warn | Same pattern as PR2 |
| **Warn-once location** | Config loader module | Single boundary for env reads |
| **PolicyToggles wiring** | Already done in PR1.1 | Gate policy reads from context, not env |

### Precedence Policy

```
Layer order: defaults < file < env

If SPEC_KIT_SIDECAR_CRITIC is set → use it (canonical)
Else if SPEC_KIT_CRITIC is set → use it + warn once (deprecated)
Else → use file config or default (false)
```

### Warn-Once Helper Location

Recommend centralizing in `codex-rs/spec-kit/src/config/loader.rs` alongside existing `warn_once_deprecated_consensus_threshold()`.

Pattern:
```rust
fn warn_once_deprecated_spec_kit_critic() {
    use std::sync::Once;
    static WARN_ONCE: Once = Once::new();
    WARN_ONCE.call_once(|| {
        eprintln!("WARNING: Env var 'SPEC_KIT_CRITIC' is deprecated. Use 'SPEC_KIT_SIDECAR_CRITIC' instead.");
    });
}
```

### Test Matrix for PR3

| Scenario | Expected Result |
|----------|-----------------|
| Neither env var set | sidecar disabled (default) |
| Legacy `SPEC_KIT_CRITIC=true` | enabled + warn once |
| Canonical `SPEC_KIT_SIDECAR_CRITIC=true` | enabled, no warning |
| Both set, same value | canonical wins + warn once |
| Both set, conflicting | canonical wins + warn once |
| Hot reload | no repeated warnings (process-wide Once) |

### Product Gotcha: local_only Constraint

Verify sidecar critic respects `local_only` flag:
- If `local_only=true` and sidecar requires cloud → sidecar should be disabled
- Check `roles_for_stage()` in `gate_policy.rs` already handles this via `ctx.local_only`

### Files to Modify

1. `codex-rs/spec-kit/src/config/loader.rs` — env var alias + warn-once
2. `codex-rs/spec-kit/src/config/registry.rs` — add canonical env var to known list
3. Possibly `codex-rs/tui/` initialization — if env is read there (should be centralized)

### Verification Before Merge

1. `grep -r "SPEC_KIT_CRITIC" codex-rs/` — should only appear in config loader
2. Schema/validation doesn't reject legacy env-driven config

---

## PR4 Implementation Notes (COMPLETE)

PR4 implemented gate policy vocabulary migration.

### Acceptance Checklist (All Verified ✅)

- [x] **No DB schema migrations** — tables stay named as-is
- [x] **No evidence dir moves** — read compatibility intact for historical artifacts
- [x] **No model routing logic moved** — router remains thin-waist
- [x] **All user-facing "consensus" wording removed** — "[Stage Review]", "REVIEW OK/CONFLICT/DEGRADED"
- [x] **One canonical "policy toggles" boundary** — `spec-kit/src/config/policy_toggles.rs`
- [x] **Test module names unchanged** — tests remain in `gate_evaluation.rs`

### Scope Decisions (Finalized)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| `is_consensus_enabled()` refactor | **Yes, together with critic** | Both toggles belong in PolicyToggles boundary; makes PR6 deletion trivial |
| Module rename style | **Full rename + re-export shim** | Blast-radius control; shim marked `#[deprecated]`, removed in PR6 |
| Log/tracing messages | **Yes, update all** | Logs are user-facing; checklist requires "consensus" wording removed |
| Test file renames | **No, keep stable** | Developer-facing, not operator-facing; reduces PR4 churn |

### Boundary Decisions

| Layer | Responsibility |
|-------|----------------|
| `spec-kit/src/gate_policy.rs` | Domain vocabulary (Stage, Role, Signal, etc.) — **data only, no IO** |
| `spec-kit/src/config/policy_toggles.rs` | Pure decision logic + env/config resolution (NEW) |
| TUI orchestration | Calls `PolicyToggles::from_env_and_config()` once at startup |

**Key insight**: Separate "policy decision" from "env IO" to enable pure-function unit tests.

```rust
// Pure decision logic (unit testable, no env IO)
fn resolve_sidecar_critic(
    canonical: Option<&str>,
    deprecated: Option<&str>,
) -> (bool, Option<DeprecationWarning>)

// Thin IO wrapper (one integration test)
pub fn load_policy_toggles() -> PolicyToggles { ... }
```

### Interface Contract (must exist before PR4)
```rust
// Gate Policy → Orchestrator
fn roles_for_stage(stage: Stage, ctx: &StageContext) -> RoleAssignment;
fn checkpoints_for_stage_transition(from: Stage, to: Stage) -> Vec<Checkpoint>;

// Router → Orchestrator
trait Router {
    fn select_worker(&self, role: Role, ctx: &RoutingContext) -> WorkerSpec;
}

// Gate → Orchestrator
fn evaluate_gate(signals: &[Signal], rule: &DecisionRule, ctx: &GateContext) -> GateVerdict;
```

### Rename Mapping
| Old | New |
|-----|-----|
| `tui/.../consensus.rs` | `tui/.../gate_evaluation.rs` (re-export old path temporarily) |
| `run_spec_consensus()` | `evaluate_gate()` (wrapper for old fn) |
| `preferred_agent_for_stage()` | `preferred_role_for_stage()` |
| `is_critic_enabled()` | Move to `policy_toggles.rs` in spec-kit |
| UI "consensus" labels | "gate evaluation" / "stage review" |

### Vocabulary (lock in for PR4+)
| Term | Meaning |
|------|---------|
| **Role** | Responsibility (Architect, Implementer, Judge, SidecarCritic) |
| **Worker** | Implementation/model/tooling pack |
| **Sidecar** | Signal-producing reviewer (non-authoritative) |
| **Gate** | Decision point between stages |

---

## PR6 Planning Notes

Decision: **Delete** legacy voting path (not feature-gate).

Rationale:
- Simpler maintenance
- No feature flag complexity
- Legacy voting was never the intended design

Keep **read compatibility** for historical evidence (old artifact directories/JSON fields).

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `codex-rs/spec-kit/src/gate_policy.rs` | Domain vocabulary (Stage, Role, Signal, Verdict, etc.) |
| `codex-rs/spec-kit/src/router.rs` | Router trait and WorkerSpec |
| `codex-rs/spec-kit/src/config/loader.rs` | Config loading + alias handling |
| `codex-rs/spec-kit/src/config/registry.rs` | Known env vars + config paths |
| `codex-rs/spec-kit/src/config/policy_toggles.rs` | **NEW (PR4)** — PolicyToggles boundary, pure decision functions |
| `codex-rs/tui/src/chatwidget/spec_kit/gate_evaluation.rs` | **RENAMED (PR4)** — Gate evaluation logic (was consensus.rs) |
| `docs/spec-kit/GATE_POLICY.md` | Canonical vocabulary spec |
| `docs/MODEL-POLICY.md` | Role → worker mapping policy |

---

## Next Session Start Command

```
Load HANDOFF.md. Execute PR7-PR9 (Internal Vocabulary Alignment).

## Context
- PR1-PR6 complete (canonical types, config/env aliases, vocabulary migration, voting path deleted)
- SPEC_KIT_CONSENSUS now warns-once for truthy values only (no spam)
- consensus.rs shim deleted, expected_agents_for_stage() simplified
- Ready for internal vocabulary cleanup

## PR7: Internal Vocabulary Alignment (Wire/Domain Separation)

### Architectural Pattern (per architect review)
Separate "wire types" (serde-focused, backward compatible) from "domain types" (clean code).

**Wire module** (serialization layer):
- Keeps legacy JSON field names for read compatibility
- Uses `#[serde(alias = "...")]` for both old and new formats
- Located in existing files, no moves

**Domain module** (what code uses):
- `GateVerdict`, `GateRun`, `StageReviewStatus`
- Clean vocabulary, no legacy naming
- Conversion functions between wire ↔ domain

### PR7 Scope (Minimal - Recommended)
1. Introduce domain type aliases in gate_policy.rs:
   - `type GateVerdict = ConsensusVerdict;` (with deprecation note)
   - `type GateArtifact = ConsensusArtifactData;`
2. Add serde aliases for new field names (don't remove old ones)
3. Migrate internal callsites to domain types gradually
4. Keep wire types stable for persistence/readers

### PR7 Anti-Patterns (Avoid)
- Don't rename everything everywhere in one PR
- Don't break JSON serialization of historical evidence
- Don't touch consensus_db or consensus_coordinator modules (wire compatibility)

### Transitional API Note
`expected_agents_for_stage()` is legacy scaffolding. It should not be used as
routing source of truth. Plan removal once all routing goes stage→role→router.

## PR8: Clippy Cleanup (Pure Mechanical)

### Scope
- Fix all clippy warnings across spec-kit + tui
- Focus on deprecated `QualityGates_ConsensusThreshold` warnings (5 occurrences)
- No behavior changes, no refactoring
- Easy to review, safe to git bisect across

### Command
```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## PR9: Command UX (/spec-consensus → /spec-review)

### Rollout Pattern
1. `/spec-review` becomes canonical command
2. `/spec-consensus` remains as deprecated alias (one release window)
3. Warn-once when alias is used
4. Update help, docs, command palette

### Scope (Command Alias Only - Recommended)
- Add `/spec-review` as primary
- Keep `/spec-consensus` as deprecated alias with warn-once
- Don't rename handler functions (keep handle_spec_consensus)

## Acceptance Criteria (All PRs)

| Criterion | PR7 | PR8 | PR9 |
|-----------|-----|-----|-----|
| No wire format changes | ✓ | N/A | N/A |
| Historical evidence readable | ✓ | N/A | N/A |
| No behavior changes | N/A | ✓ | N/A |
| Warn-once for deprecated | N/A | N/A | ✓ |
| Tests pass | ✓ | ✓ | ✓ |

## Commits (chronological, this session)
- 6d718a008: chore(spec-kit): delete legacy voting path (PR6)
- 397139091: fix(spec-kit): only warn for truthy SPEC_KIT_CONSENSUS values (PR6 polish)
```

---

## Optional: Strict Mode for CI (Future)

Per architect suggestion, consider adding strict mode for CI environments:

```rust
// If SPEC_KIT_CONSENSUS=true in strict mode (CI), return non-zero exit
// This prevents "stale config" silently masking intent
pub fn resolve_legacy_voting_strict(val: Option<&str>, strict: bool) -> Result<(), ConfigError> {
    if strict && val.map(parse_bool).unwrap_or(false) {
        return Err(ConfigError::RemovedFeature("SPEC_KIT_CONSENSUS"));
    }
    Ok(())
}
```

This is optional operator ergonomics, not required for PR7-PR9.

---

## Design Intent Summary

**Single-owner pipeline**: One stage owner produces the artifact, gates evaluate signals, escalation is deterministic.

**No voting**: Sidecars contribute signals (block/advisory), not competing answers.

**Policy separation**:
- Gate Policy = what must happen (signals, decision rules, escalation)
- Model Policy = who executes it (role → worker → model/provider)

**Migration strategy**: Compatibility aliases with warn-once, then rename, then delete legacy.
