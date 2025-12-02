# HANDOFF-P95: Constitution-Aware Refinement (SPEC-KIT-105 Phase 8)

## Session Lineage
P89 (Data Model) → P90 (TASK_BRIEF + Tier-2) → P91 (Conflict Detection + Gate + Command) → P92 (Block + Cache + Plan) → P93 (Vision Q&A) → P94 (Drift Detection) → **P95** (Constitution-Aware Refinement)

## P94 Completion Summary

### Implemented
1. **Guardrails in `/speckit.vision`** - 6th question for hard constraints (security, privacy, compliance)
2. **Constitution version tracking** - Specs record `Constitution-Version` in frontmatter at creation
3. **`/speckit.check-alignment`** - Version-based drift detection with TUI table + `--json` for CI
4. **Event logging** - `VisionDefined` and `AlignmentCheckRun` telemetry events

### Commits
- `71c21af8e` - feat(tui): add guardrails question to /speckit.vision (P94 Task 1)
- `0000644f6` - feat(tui): track constitution_version at spec creation (P94 Task 2)
- `a3a6870b4` - feat(tui): implement /speckit.check-alignment command (P94 Task 3)
- `461e88de9` - feat(tui): add event logging for constitution lifecycle (P94 Task 4)

---

## P95 Design Decisions (Confirmed)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Auto-reconciliation | **Defer to P96** | Focus P95 on detection + display + manual resolution |
| `--deep` CI behavior | **Explicit only** | Must pass `--deep` flag manually; never auto-runs |
| Conflict gate in `/speckit.specify` | **Soft block** | Require `--force` to proceed; log override |
| Exception memory type | **Yes, include** | Add `ConstitutionType::Exception` for sanctioned violations |

---

## P95 Scope: Constitution-Aware Refinement

### Task 1: Add `ConstitutionType::Exception`
**Files**: `codex-rs/stage0/src/overlay_db.rs`, `codex-rs/stage0/src/lib.rs`

Add new constitution type for sanctioned guardrail violations:

```rust
pub enum ConstitutionType {
    Guardrail,   // priority 10
    Principle,   // priority 9
    Goal,        // priority 8
    NonGoal,     // priority 8
    Exception,   // NEW - priority 7 (lower than guardrails)
}
```

Exception memory structure (Template Guardian enforced):
```text
[EXCEPTION]: <Brief title>

CONTEXT: <Which guardrail/principle this exempts>
RATIONALE: <Why this exception is acceptable>
SCOPE:
- spec_id: <SPEC-ID>
- code_units: [optional list of affected code]
- expires_after_version: <optional>

APPROVED_BY: <owner> on <date>
```

### Task 2: Implement `/speckit.specify` Constitution Awareness
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`, `codex-rs/tui/src/chatwidget/spec_kit/handler.rs`

**User Flow:**

1. **Load spec context**:
   - Read spec's `Constitution-Version` from frontmatter
   - Get current constitution version from overlay DB
   - Run Stage 0 (or use cached `Stage0Result`) to get:
     - Relevant constitution memories (principles, guardrails)
     - `constitution_aligned_ids`
     - `constitution_conflicts` text

2. **Display conflicts** (if any):
   ```
   Constitution Conflicts Detected

   Spec: SPEC-KIT-123 (created v1, current v3)

   CONFLICT: Spec proposes direct file writes
   GUARDRAIL G2: All file operations must be sandboxed

   Resolution options:
     [A] Modify spec to comply with guardrail
     [B] Record exception (sanctioned violation)
     [C] Proceed with conflicts (--force)
   ```

3. **Handle resolution**:
   - **Option A**: Continue to normal specify Q&A flow
   - **Option B**: Open exception form → create `ConstitutionType::Exception` memory
   - **Option C**: Emit `ConstitutionOverride` event and proceed

4. **Soft block behavior**:
   - TUI: Must explicitly select option to proceed
   - CLI: Require `--force` flag when conflicts present
   - Log all overrides with `ConstitutionOverride` event

### Task 3: Implement `--deep` Mode for `/speckit.check-alignment`
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`

Extend existing command with content-level drift detection:

```
/speckit.check-alignment --deep [--spec SPEC-ID]
```

**Behavior:**
- Without `--deep`: Version-only check (existing P94 behavior)
- With `--deep`: Run Stage 0 / Tier-2 alignment for each spec
- With `--spec`: Only check specified spec (reduces Tier-2 usage)

**Output (extended):**
```
SPEC ID          | Created Ver | Current Ver | Version | Content  | Exceptions
-----------------+-------------+-------------+---------+----------+-----------
SPEC-KIT-105     | 3           | 3           | fresh   | aligned  | 0
SPEC-KIT-102     | 1           | 3           | stale   | conflict | 1
SPEC-KIT-099     | 1           | 3           | stale   | aligned  | 0

Summary: 1 fresh, 2 stale | 2 aligned, 1 conflict | 1 exception
```

**JSON mode (`--deep --json`):**
```json
[
  {
    "spec_id": "SPEC-KIT-102",
    "constitution_version_at_creation": 1,
    "current_constitution_version": 3,
    "staleness": "stale",
    "alignment": "conflict",
    "conflict_details": ["Violates G2: sandboxed file operations"],
    "exception_ids": ["exception-uuid-123"]
  }
]
```

### Task 4: Add Event Logging for Reconciliation
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`, `handler.rs`

**ConstitutionOverride event** (when user proceeds with conflicts):
```rust
tracing::info!(
    event_type = "ConstitutionOverride",
    spec_id,
    constitution_version,
    conflict_count,
    guardrail_ids = ?conflicting_guardrails,
    "User proceeded with unresolved conflicts"
);
```

**ConstitutionExceptionCreated event** (when exception recorded):
```rust
tracing::info!(
    event_type = "ConstitutionExceptionCreated",
    exception_id,
    spec_id,
    guardrail_id,
    "Exception recorded for guardrail violation"
);
```

**DeepAlignmentCheckRun event**:
```rust
tracing::info!(
    event_type = "DeepAlignmentCheckRun",
    total_specs,
    aligned_count,
    conflict_count,
    exception_count,
    tier2_calls,
    "Deep alignment check completed"
);
```

### Task 5: Add `/speckit.constitution add-exception` Subcommand
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`

Allow administrators to create exceptions outside `/speckit.specify`:

```
/speckit.constitution add-exception --guardrail G2 --spec SPEC-KIT-123 --reason "Debug mode only"
```

Opens form to capture:
- Target guardrail/principle
- Affected spec(s)
- Rationale
- Optional: expiration version, code units

---

## Tests

```bash
cd codex-rs && cargo test -p codex-stage0 -- --test-threads=1
cargo test -p codex-tui --lib command_registry
~/code/build-fast.sh
```

### New Tests to Add
- `test_constitution_type_exception_priority` - Exception has lower priority than Guardrail
- `test_specify_detects_conflicts` - Conflicts from Stage0Result shown
- `test_specify_soft_block_requires_force` - CLI exits without --force
- `test_deep_alignment_runs_stage0` - --deep triggers Stage 0
- `test_exception_suppresses_conflict` - Exceptions shown but don't fail

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `codex-rs/stage0/src/overlay_db.rs` | ConstitutionType::Exception |
| `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs` | Commands |
| `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` | Specify flow |
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Gate logic |

---

## Out of Scope (P96+)

- **Auto-reconciliation suggestions** - AI-generated resolution options
- **Exception approval workflow** - Multi-party sign-off
- **Exception expiration enforcement** - Auto-invalidate expired exceptions
- **CI auto-deep mode** - Configurable automatic --deep in CI

---

## Data Flow Diagram

```
/speckit.specify SPEC-ID
        │
        ▼
┌──────────────────────────────┐
│ 1. Load spec frontmatter     │
│    - Constitution-Version    │
│    - Current version from DB │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ 2. Run Stage 0 (cached ok)   │
│    - constitution_aligned_ids│
│    - constitution_conflicts  │
│    - relevant memories       │
└──────────────┬───────────────┘
               │
        Has conflicts?
        /           \
       No           Yes
       │             │
       ▼             ▼
┌───────────┐  ┌──────────────────┐
│ Continue  │  │ Show conflict UI │
│ to Q&A    │  │ [A] Modify spec  │
│           │  │ [B] Add exception│
│           │  │ [C] --force      │
└───────────┘  └────────┬─────────┘
                        │
                  User choice
                  /    |    \
                 A     B     C
                 │     │     │
                 ▼     ▼     ▼
            Continue  Create  Log override
            to Q&A    Exception event
```

---

## Why This Scope?

P95 completes the "constitution lifecycle" by making conflicts **actionable**:

| Phase | Capability | P95 Addition |
|-------|------------|--------------|
| P93 | Vision capture | ✓ |
| P94 | Version drift detection | ✓ |
| **P95** | **Content drift detection** | `--deep` mode |
| **P95** | **Conflict visibility** | `/speckit.specify` integration |
| **P95** | **Sanctioned exceptions** | `ConstitutionType::Exception` |

After P95, the system can:
1. Detect when specs are out of alignment (version AND content)
2. Show conflicts to users at refinement time
3. Track sanctioned exceptions as first-class data
4. Log all reconciliation decisions for audit/learning

P96 can then add **AI-powered suggestions** on top of this foundation.
