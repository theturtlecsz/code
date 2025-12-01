# HANDOFF-P91: Constitution Enforcement Foundation (SPEC-KIT-105 Phase 4)

**Session**: P91
**Previous**: P90 (Constitution TASK_BRIEF Integration)
**Status**: Ready to Start
**Primary Goal**: Implement constitution enforcement primitives: conflict detection, readiness gates, `/speckit.constitution`

---

## What P90 Accomplished

### 1. TASK_BRIEF Section 0 (dcc.rs)
- Always renders Section 0: Project Constitution (Summary)
- Groups by type: Principles → Guardrails → Goals
- Includes memory IDs for traceability (`const-001`)
- Placeholder if constitution empty with `/speckit.constitution` hint
- Logs warning if constitution missing

### 2. TASK_BRIEF Metadata (dcc.rs)
- Added `constitution_version` to JSON metadata
- Added `constitution_hash` to JSON metadata
- Added `constitution_aligned_ids` array for P91 conflict detection

### 3. Tier-2 Prompt (tier2.rs)
- Added CONSTITUTION AWARENESS clause
- Explains Section 0 (principles, guardrails, goals)
- Instructs to treat guardrails as hard constraints
- Instructs to call out conflicts in new Section 2

### 4. Divine Truth Output (tier2.rs)
- New Section 2: Constitution Alignment
- Extracts `aligned_ids` (e.g., ["P1", "G2"])
- Stores `conflicts_raw` for P91 processing
- Sections renumbered: 3=Guardrails, 4=History, 5=Risks, 6=Links
- New `ConstitutionAlignment` struct

### Test Coverage
- 150 tests passing (+6 new)
- Full Section 0 rendering tests
- Constitution alignment parsing tests

---

## P91 Scope (Refined)

### 1. Structured Conflict Detection

**File**: `codex-rs/stage0/src/lib.rs`

Parse Divine Truth's Constitution Alignment section and record conflicts:

```rust
// After divine_truth = parse_divine_truth(response)
let conflict_summary = if let Some(conflicts) = &divine_truth.constitution_alignment.conflicts_raw {
    if !conflicts.trim().is_empty() && conflicts != "None identified." {
        tracing::warn!(
            target: "stage0",
            spec_id = spec_id,
            conflicts = %conflicts,
            "Constitution conflict detected in spec"
        );
        Some(conflicts.clone())
    } else {
        None
    }
} else {
    None
};

// Store in Stage0Result for UI consumption
```

**File**: `codex-rs/stage0/src/lib.rs` (Stage0Result)

Add field to result:
```rust
pub struct Stage0Result {
    // ...existing fields
    pub constitution_conflicts: Option<String>,  // P91: Raw conflict text
    pub constitution_aligned_ids: Vec<String>,   // P91: Aligned principle/guardrail IDs
}
```

### 2. Phase -1 Readiness Gate (Warn-Only)

**File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` (or new gate module)

Implement soft pre-flight check:
```rust
/// Check constitution readiness before pipeline runs
/// Returns warnings but does not block execution
pub fn check_constitution_readiness(db: &OverlayDb) -> Vec<String> {
    let mut warnings = Vec::new();

    let count = db.constitution_memory_count().unwrap_or(0);
    if count == 0 {
        warnings.push("⚠ No constitution defined for this project. Run /speckit.constitution.".into());
        tracing::warn!(target: "stage0", "stage0.gate.constitution_missing=true");
    }

    // Check for minimum types
    let mems = db.get_constitution_memories().unwrap_or_default();
    let has_principle = mems.iter().any(|m| m.const_type == ConstitutionType::Principle);
    let has_guardrail = mems.iter().any(|m| m.const_type == ConstitutionType::Guardrail);

    if count > 0 && !has_principle {
        warnings.push("⚠ No principles defined. Consider adding at least one.".into());
    }
    if count > 0 && !has_guardrail {
        warnings.push("⚠ No guardrails defined. Consider adding at least one.".into());
    }

    warnings
}
```

**Config key**: Add `phase1_gate_mode` supporting `warn|skip` (reserve `block` for P92):
```rust
// In config.rs or Stage0Config
pub enum GateMode {
    Warn,   // Log warning, continue
    Skip,   // No check
    // Block, // P92+: Stop pipeline
}
```

**Integration point**: Call gate before `/speckit.auto` and `/speckit.new` in pipeline coordinator.

### 3. `/speckit.constitution` Command

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/commands/constitution.rs` (new)
- `codex-rs/tui/src/chatwidget/spec_kit/commands/mod.rs` (register)
- `codex-rs/tui/src/slash_command.rs` (dispatch)

**Functionality**:
1. **View mode** (no args): Display current constitution
   ```
   /speckit.constitution

   📜 Project Constitution (v3)

   ## Principles
   - [P1] Optimize for developer ergonomics (const-001)
   - [P2] All APIs must be documented (const-002)

   ## Guardrails
   - [G1] Never store secrets in plain text (const-003)

   ## Goals
   - [Goal] Support 3 cloud providers by Q3 (const-004)

   Total: 4 items | Hash: sha256:abc123
   ```

2. **Add mode**: Interactive constitution entry
   ```
   /speckit.constitution add

   What type? [principle/guardrail/goal/non-goal]: guardrail
   Enter the guardrail: All file operations must be sandboxed

   ✅ Added guardrail (const-005). Constitution v4.
   ```

3. **Regenerate**: Update `NL_CONSTITUTION.md` and `memory/constitution.md`
   ```
   /speckit.constitution sync

   ✅ Regenerated NL_CONSTITUTION.md (4 items)
   ✅ Updated memory/constitution.md
   ```

**Implementation pattern** (follow existing commands like `/speckit.new`):
```rust
pub async fn handle_constitution_command(
    args: &str,
    state: &mut SpecKitState,
    db: &OverlayDb,
) -> Result<String, SpecKitError> {
    match args.trim() {
        "" | "view" => view_constitution(db),
        "add" => add_constitution_entry(state, db).await,
        "sync" => sync_constitution_files(state, db).await,
        _ => Err(SpecKitError::InvalidArgs("constitution".into())),
    }
}
```

---

## Key Files

| File | Purpose |
|------|---------|
| `stage0/src/lib.rs` | Stage0Result with conflict fields |
| `tui/src/chatwidget/spec_kit/commands/constitution.rs` | NEW: /speckit.constitution handler |
| `tui/src/chatwidget/spec_kit/commands/mod.rs` | Register constitution command |
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Readiness gate integration |
| `stage0/src/config.rs` | GateMode enum and config |

---

## Out of Scope (P92+)

- `/speckit.vision` - Vision/Articles Q&A wizard
- `/speckit.plan` - Plan-only CLI mode
- Hard blocking on missing constitution or conflicts
- Auto-remediation suggestions
- Constitution versioning UI
- NotebookLM artifact seeding for constitution
- Constitution diff/history tracking
- `GateMode::Block` implementation

---

## Test Commands

```bash
# Run stage0 tests
cd codex-rs && cargo test -p codex-stage0 -- --test-threads=1

# Build TUI
~/code/build-fast.sh

# Run full check
cargo fmt --all -- --check
cargo clippy -p codex-stage0 --all-targets -- -D warnings
```

---

## Commits from P90

```
9f8224a56 docs(SPEC-KIT-102): add orphaned session artifacts and research
79461a646 feat(stage0): implement constitution TASK_BRIEF integration (SPEC-KIT-105 Phase 3)
ce483e9b8 docs: Add P91 session handoff for conflict detection
4179f9719 style: apply cargo fmt to workspace
```

---

## Session Lineage

```
P89 (Constitution Data Model)
    ↓
P90 (TASK_BRIEF + Tier-2) ← COMPLETED
    ↓
P91 (Conflict Detection + Gate + /speckit.constitution) ← NEXT
    ↓
P92 (Vision, Plan, Blocking Enforcement)
```

---

## Acceptance Criteria

1. **Conflict Detection**: `Stage0Result` contains `constitution_conflicts` and `constitution_aligned_ids` populated from Divine Truth Section 2
2. **Readiness Gate**: Warning printed before `/speckit.auto` when no constitution exists
3. **Slash Command**: `/speckit.constitution` displays current constitution, allows adding entries, syncs to files
4. **Tests**: New tests for gate logic and command parsing
5. **Config**: `phase1_gate_mode` config key with `warn`/`skip` support
