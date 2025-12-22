# Gate Policy Alignment — Session Handoff

**Last updated:** 2025-12-22
**Status:** SPEC-KIT-921 COMPLETE — CLI/TUI Parity Achieved

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
| **SPEC-KIT-921** | ✅ Complete | CLI Adapter + Shared SpeckitExecutor Core (1771 LOC) |

### SPEC-KIT-921 Implementation Summary

**All phases complete:**

| Phase | Status | Description |
|-------|--------|-------------|
| Phase A | ✅ | Types + skeleton (`SpeckitCommand`, `Outcome`, executor shell) |
| Phase B | ✅ | Status + Review commands with shared executor |
| Phase C | ✅ | ValidateStage (Plan, Tasks, Implement, Validate, Audit, Unlock) |
| Phase D | ✅ | Run (batch validation) + Migrate (spec.md → PRD.md) |
| Phase E | ✅ | CLI wiring in `cli/src/speckit_cmd.rs` + `main.rs` integration |

**CLI Commands Available:**
```bash
code speckit status --spec SPEC-ID [--stale-hours N] [--json]
code speckit review --spec SPEC-ID --stage STAGE [--strict-*] [--explain] [--json]
code speckit specify --spec SPEC-ID [--execute] [--json]
code speckit plan --spec SPEC-ID [--dry-run] [--strict-prereqs] [--json]
code speckit tasks --spec SPEC-ID [--dry-run] [--strict-prereqs] [--json]
code speckit implement --spec SPEC-ID [--dry-run] [--strict-prereqs] [--json]
code speckit validate --spec SPEC-ID [--dry-run] [--strict-prereqs] [--json]
code speckit audit --spec SPEC-ID [--dry-run] [--strict-prereqs] [--json]
code speckit unlock --spec SPEC-ID [--dry-run] [--strict-prereqs] [--json]
code speckit run --spec SPEC-ID --from STAGE --to STAGE [--json]
code speckit migrate --spec SPEC-ID [--dry-run] [--json]
```

**Key Files:**
- `codex-rs/spec-kit/src/executor/mod.rs` — SpeckitExecutor implementation
- `codex-rs/spec-kit/src/executor/command.rs` — SpeckitCommand shared model
- `codex-rs/cli/src/speckit_cmd.rs` — CLI adapter (1771 LOC)
- `codex-rs/cli/src/main.rs:126,368-370` — CLI wiring

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

**Copy this prompt to continue:**
```
load HANDOFF.md **ultrathink**

## Session Context (2025-12-22)

Previous session (P64) accomplished:
- ✅ Fixed rustfmt CI failure (commit aa8b6439d)
- ✅ Committed architectural changes (MCP→CLI/REST migration for local-memory)
- ✅ Updated SPEC.md tracker with SPEC-KIT-921 completion
- ✅ Updated HANDOFF.md with continuation prompt

## Primary Focus: CI Stabilization

1. **Verify CI workflows pass**:
   - Check Quality Gates (rustfmt fix just pushed)
   - Check Spec-Kit CI (was passing, verify still green)
   - Monitor: `gh run list --limit 5`

2. **P3: Pipeline Coordinator Extraction** (include in scope):
   - Goal: Extract `/speckit.auto` orchestration from TUI to SpeckitExecutor
   - Enables: Headless full pipeline runs via CLI
   - Files to modify: `spec-kit/src/executor/mod.rs`, `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
   - Approach: Move stage sequencing logic, keep TUI as thin adapter

3. **MAINT-12 continuation** (if time permits):
   - Stage0: NotebookLM HTTP-only + local-memory CLI/REST (no MCP)
   - Goal: Stage0 must not require MCP at all

## Quick Verify
```bash
gh run list --limit 5                              # Check CI status
./codex-rs/target/release/code speckit status --spec SPEC-KIT-900 --json  # CLI works
cargo test -p codex-cli -- speckit                # Run speckit tests
```

## Tracking Summary

| Item | Status | Next Action |
|------|--------|-------------|
| SPEC-KIT-921 | ✅ Complete | Done |
| CI (Quality Gates) | ⏳ Pending | Verify rustfmt fix |
| CI (Spec-Kit) | ✅ Passing | Monitor |
| P3 Coordinator | 🔶 Scoped | Extract from TUI |
| MAINT-12 Stage0 | 🔶 In Progress | Remove MCP deps |
```

**Key context:**
- PR1-PR9 COMPLETE (vocabulary migration, clippy cleanup, command UX)
- SPEC-KIT-921 COMPLETE (CLI adapter + SpeckitExecutor + all 11 subcommands)
- CI hardening COMPLETE (vocabulary drift canary + golden wire-format tests)
- SPEC-KIT-920 superseded (PTY automation was wrong approach)
- **New:** `docs/spec-kit/CLI-REFERENCE.md` documents all CLI commands

---

## Post-921 Priorities (Updated 2025-12-22)

### ✅ P0: Verify CLI Build & Run — DONE
```bash
cd codex-rs && cargo build -p codex-cli --release
./target/release/code speckit status --spec SPEC-KIT-921 --json
./target/release/code speckit review --spec SPEC-KIT-921 --stage plan --explain
```

### ✅ P1: CI Integration — DONE
- Added `code speckit run --from plan --to audit` to CI workflow
- Added `code speckit status --json` structure validation
- See `.github/workflows/spec-kit-ci.yml` lines 240-283

### ✅ P2: Documentation — DONE
- Created `docs/spec-kit/CLI-REFERENCE.md` (300+ lines)
- Archived SPEC-KIT-920 with superseded notice
- HANDOFF.md updated with completion status

### 🔶 P3: Pipeline Coordinator Extraction (Optional)
- `/speckit.auto` still runs in TUI context
- Extract to SpeckitExecutor if headless orchestration needed
- **Lower priority** — individual stages work via CLI

### 🔶 P4: Commit & Push (Pending)
- Session created uncommitted documentation
- Commit message: `docs(spec-kit): add CLI reference and archive SPEC-KIT-920`

---

## Superseded: SPEC-KIT-921 Planning (COMPLETE)

The following sections are preserved for historical reference but the work is done.

### Original Problem Statement
TUI slash commands depended on `&mut ChatWidget`, making CLI impossible without re-implementing logic. PTY automation (SPEC-KIT-920 approach) failed in CI/Proxmox.

### Solution Implemented
Extracted `SpeckitExecutor` in `spec-kit/src/executor/` that:
- Accepts typed `SpeckitCommand` (shared model)
- Returns `Outcome` variants (Status, Review, Stage, Specify, Run, Migrate, Error)
- No ChatWidget dependency — pure business logic

### Files Created
| File | Role |
|------|------|
| `spec-kit/src/executor/mod.rs` | SpeckitExecutor + Outcome types |
| `spec-kit/src/executor/command.rs` | SpeckitCommand enum |
| `spec-kit/src/executor/status.rs` | Status dashboard rendering |
| `spec-kit/src/executor/review.rs` | Review dashboard rendering |
| `cli/src/speckit_cmd.rs` | CLI adapter (1771 LOC) |

## PR7-PR9 Completion Summary

| PR | Status | Description |
|----|--------|-------------|
| **PR7** | ✅ Complete | Internal vocabulary alignment (type aliases, serde aliases, deprecation notes) |
| **PR8** | ✅ Complete | Clippy cleanup across spec-kit + stage0 + tui |
| **PR9** | ✅ Complete | Command UX: `/spec-review` canonical, `/spec-consensus` deprecated alias |

## Commits (this session)
```
6fc0dbbd1 docs: add SPEC-KIT-900 gold run spec and playbook (P0)
9a5e5a743 docs: add NEXT_FOCUS_ROADMAP.md (post-PR7 architect review)
379030a13 feat(spec-kit): add vocabulary audit script and golden evidence tests (PR7 hardening)
08d1a8610 chore(spec-kit): suppress dead_code warnings on transitional type aliases (PR7 polish)
5e1e0a7dc feat(spec-kit): vocabulary alignment and command UX (PR7-PR9)
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
