# HANDOFF-P91: Constitution Conflict Detection (SPEC-KIT-105 Phase 4)

**Session**: P91
**Previous**: P90 (Constitution TASK_BRIEF Integration)
**Status**: Ready to Start
**Primary Goal**: Implement conflict detection and slash commands

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

## P91 Scope

### 1. CONSTITUTION_CONFLICT_WARNING Error Type

**File**: `codex-rs/stage0/src/errors.rs`

Add new error variant:
```rust
pub enum Stage0Error {
    // ...existing variants

    /// Constitution conflict detected (non-fatal warning)
    ConstitutionConflict {
        spec_id: String,
        conflicts: Vec<String>,  // Conflict descriptions
        severity: ConflictSeverity,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum ConflictSeverity {
    Warning,    // Log but continue
    Blocking,   // Stop pipeline (future P92)
}
```

**Usage**: When `DivineTruth.constitution_alignment.conflicts_raw` is populated:
- Parse conflict descriptions
- Emit warning-level log
- Store in result for UI display
- Do NOT stop pipeline (P92 will add blocking)

### 2. Conflict Detection Integration

**File**: `codex-rs/stage0/src/lib.rs` (run_stage0 function)

After Tier-2 synthesis:
```rust
// After divine_truth = parse_divine_truth(response)
if let Some(conflicts) = &divine_truth.constitution_alignment.conflicts_raw {
    if !conflicts.trim().is_empty() {
        tracing::warn!(
            target: "stage0",
            spec_id = spec_id,
            conflicts = %conflicts,
            "Constitution conflict detected in spec"
        );
        // Store in result for UI
    }
}
```

### 3. Slash Commands (TUI)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/commands/`

#### `/speckit.constitution`
- Shows current constitution from overlay DB
- Lists all constitution memories with types
- Shows version and hash

#### `/speckit.vision`
- Shows spec purpose/vision (deferred from P89)
- Reads from spec.md or asks user to define

#### `/speckit.plan`
- Shows current SPEC state (planned vs implemented)
- Lists completed vs pending phases

**Implementation Pattern** (follow existing commands):
```rust
// In commands/mod.rs
pub mod constitution;

// In commands/constitution.rs
pub fn handle_constitution_command(state: &SpecKitState, db: &OverlayDb) -> String {
    let mems = db.get_constitution_memories().unwrap_or_default();
    if mems.is_empty() {
        return "No constitution defined. Use `store_memory` with domain='constitution' and type:* tags.".into();
    }
    // Format as markdown table
}
```

### 4. Phase -1 Gates (Optional)

If time permits, add pre-flight check:
```rust
fn check_constitution_readiness(db: &OverlayDb) -> Result<()> {
    let meta = db.get_constitution_meta()?;
    if meta.0 == 0 {
        tracing::warn!("No constitution defined - spec alignment will be limited");
    }
    Ok(())
}
```

---

## Key Files

| File | Purpose |
|------|---------|
| `stage0/src/errors.rs` | Add ConstitutionConflict error |
| `stage0/src/lib.rs` | Conflict detection integration |
| `tui/src/chatwidget/spec_kit/commands/` | Slash command handlers |
| `tui/src/chatwidget/spec_kit/mod.rs` | Command dispatch |

---

## Out of Scope (P92+)

- Blocking conflict enforcement
- Auto-remediation suggestions
- Constitution versioning UI
- NotebookLM artifact seeding for constitution
- Constitution diff/history tracking

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
```

---

## Session Lineage

```
P89 (Constitution Data Model)
    ↓
P90 (TASK_BRIEF + Tier-2) ← COMPLETED
    ↓
P91 (Conflict Detection + Slash Commands) ← NEXT
    ↓
P92 (Blocking Enforcement, NotebookLM Artifacts)
```
