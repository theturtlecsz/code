# Gate Policy Alignment — Session Handoff

**Last updated:** 2025-12-18
**Status:** PR3 complete, PR4 ready to implement

---

## Progress Tracker

| PR | Status | Description |
|----|--------|-------------|
| **PR1** | ✅ Complete | Define canonical types (Role, Stage, Signal, Verdict, Router trait) in `codex-rs/spec-kit/` |
| **PR1.1** | ✅ Complete | Fix P0 env boundary leak, align thresholds to 0.65, add PolicyToggles |
| **PR2** | ✅ Complete | Config alias (`consensus_threshold` → `min_confidence_for_auto_apply`) |
| **PR3** | ✅ Complete | Env var alias (`SPEC_KIT_CRITIC` → `SPEC_KIT_SIDECAR_CRITIC`) |
| **PR4** | 🔲 Pending | Module rename (`consensus.rs` → `gate_policy.rs`) + callsite migrations |
| **PR6** | 🔲 Pending | Delete legacy voting path |

### Commits (chronological)
```
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

## PR4 Planning Notes

After PR3, PR4 does module rename + callsite migrations:

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
| `consensus.rs` | `gate_policy.rs` (re-export old path temporarily) |
| `run_spec_consensus()` | `evaluate_gate()` (wrapper for old fn) |
| `preferred_agent_for_stage()` | `preferred_role_for_stage()` |
| UI "consensus" labels | "gate evaluation" |

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
| `docs/spec-kit/GATE_POLICY.md` | Canonical vocabulary spec |
| `docs/MODEL-POLICY.md` | Role → worker mapping policy |

---

## Next Session Start Command

```
Load HANDOFF.md. Implement PR4 (module rename consensus.rs → gate_policy.rs + callsite migrations). Run tests after each change.
```

---

## Design Intent Summary

**Single-owner pipeline**: One stage owner produces the artifact, gates evaluate signals, escalation is deterministic.

**No voting**: Sidecars contribute signals (block/advisory), not competing answers.

**Policy separation**:
- Gate Policy = what must happen (signals, decision rules, escalation)
- Model Policy = who executes it (role → worker → model/provider)

**Migration strategy**: Compatibility aliases with warn-once, then rename, then delete legacy.
