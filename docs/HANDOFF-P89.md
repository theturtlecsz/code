# P89 Session Handoff: Constitution Data Model Implementation

**Previous Session**: P88
**Date**: 2025-12-01
**Status**: IMPLEMENTATION READY

---

## 1. Session Context

P88 completed the full design of SPEC-KIT-105 (Constitution Workflow Enhancement). The spec now has parity with GitHub spec-kit plus Stage 0 + NotebookLM integration advantages.

### 1.1 What Was Accomplished (P88)

| Task | Status | Artifact |
|------|--------|----------|
| Integration analysis | ✅ Done | `research/P88-integration-analysis.md` |
| SPEC-KIT-105 v1 | ✅ Done | Full design with all integration points |
| SPEC-KIT-105 v2 | ✅ Done | Added: versioning, 5-Articles, /speckit.plan, two-way sync |
| Gap analysis vs GitHub spec-kit | ✅ Done | All gaps addressed in spec |
| Design decisions documented | ✅ Done | IQO union, separate always-on pass, 5-Articles |

### 1.2 Key Design Decisions (P88)

1. **IQO Amendment**: Union (not replace) for constitution domain
2. **Constitution Retrieval**: Separate always-on pass, not just IQO filtering
3. **Always Include N**: At least 3 constitution memories in Section 0
4. **5-Articles Template**: Adapted from GitHub's 9 Articles
5. **Versioning**: constitution_version in file, spec headers, TASK_BRIEF metadata
6. **Two-Way Sync**: File is source of truth, manual refresh command
7. **Gate Location**: Phase -1 gates in TUI layer, not Stage 0

---

## 2. P89 Primary Goal: Phase 2 - Data Model

### 2.1 Scope (Explicit)

**IN SCOPE**:
- Constitution memory schema (`domain:constitution`, types)
- Overlay scoring rules (`initial_priority` 9-10)
- Versioning system (`constitution_version`)
- IQO union logic
- "Always include N" selection behavior in DCC

**OUT OF SCOPE** (defer to P90+):
- TASK_BRIEF Section 0 rendering
- New commands (/speckit.vision, /speckit.constitution, etc.)
- Phase -1 Gates
- NotebookLM artifact generation
- Constitution-aware Q&A
- Drift detection
- Auto-refresh watcher
- E2E ferris-test benchmark

### 2.2 Implementation Tasks

#### Task 1: Constitution Memory Schema

Define how constitution entries are stored in local-memory:

```
Domain: constitution
Types:
  - type:principle  (initial_priority: 9)
  - type:guardrail  (initial_priority: 10)
  - type:goal       (initial_priority: 8)
  - type:non-goal   (initial_priority: 8)
```

**Files to modify**:
- Consider adding schema documentation
- Ensure Template Guardian can handle constitution entries

#### Task 2: Overlay Scoring Rules

Update overlay_db to handle constitution priorities:

```rust
// overlay_db.rs - ensure constitution items get elevated priority
pub fn upsert_constitution_memory(
    &self,
    memory_id: &str,
    constitution_type: ConstitutionType, // principle, guardrail, goal, non-goal
    content: &str,
) -> Result<()> {
    let priority = match constitution_type {
        ConstitutionType::Guardrail => 10,
        ConstitutionType::Principle => 9,
        ConstitutionType::Goal | ConstitutionType::NonGoal => 8,
    };
    // ... upsert with priority
}
```

**Files to modify**:
- `codex-rs/stage0/src/overlay_db.rs`

#### Task 3: Versioning System

Add constitution version tracking:

```sql
-- New table or extension to existing schema
CREATE TABLE IF NOT EXISTS constitution_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    version INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL,
    hash TEXT
);
```

**Files to modify**:
- `codex-rs/stage0/STAGE0_SCHEMA.sql`
- `codex-rs/stage0/src/overlay_db.rs` (add version methods)

#### Task 4: IQO Union Logic

Ensure constitution domain is always included without clobbering:

```rust
// dcc.rs - in build_iqo or compile_context
if !iqo.domains.iter().any(|d| d == "constitution") {
    iqo.domains.push("constitution".to_string());
}
```

**Files to modify**:
- `codex-rs/stage0/src/dcc.rs`

#### Task 5: Separate Always-On Pass

Add dedicated constitution retrieval that bypasses IQO filtering:

```rust
// dcc.rs - in compile_context
async fn fetch_constitution_memories(
    local_mem: &impl LocalMemoryClient,
    limit: usize,
) -> Result<Vec<LocalMemorySummary>> {
    // Direct query for domain:constitution, ordered by priority
    // Returns top N regardless of IQO
}
```

**Files to modify**:
- `codex-rs/stage0/src/dcc.rs`

#### Task 6: "Always Include N" Selection

After MMR selection, ensure at least 3 constitution memories are present:

```rust
// dcc.rs - after select_with_mmr
fn ensure_constitution_minimum(
    selected: &mut Vec<MemoryCandidate>,
    constitution_memories: &[MemoryCandidate],
    min_count: usize,
) {
    let current_count = selected.iter()
        .filter(|m| m.domain.as_deref() == Some("constitution"))
        .count();

    if current_count < min_count {
        // Backfill from constitution_memories
    }
}
```

**Files to modify**:
- `codex-rs/stage0/src/dcc.rs`

---

## 3. Files to Reference

### 3.1 SPEC-KIT-105 (Primary Reference)

| Document | Path | Purpose |
|----------|------|---------|
| Spec | `docs/SPEC-KIT-105-constitution-workflow/spec.md` | Full design spec |
| Research | `docs/SPEC-KIT-105-constitution-workflow/research/P88-integration-analysis.md` | Design rationale |

### 3.2 Stage 0 Implementation

| Component | Path | Key Functions |
|-----------|------|---------------|
| DCC | `codex-rs/stage0/src/dcc.rs` | `compile_context()`, `build_iqo()`, `select_with_mmr()` |
| Overlay DB | `codex-rs/stage0/src/overlay_db.rs` | `upsert_overlay_memory()`, `get_memory()` |
| Schema | `codex-rs/stage0/STAGE0_SCHEMA.sql` | Database schema |
| Scoring | `codex-rs/stage0/src/scoring.rs` | `calculate_dynamic_score()` |
| Config | `codex-rs/stage0/src/config.rs` | `Stage0Config`, `ScoringConfig` |

---

## 4. Quick Start Commands

```bash
# Tests (from codex-rs/)
cargo test -p codex-stage0   # 127 tests
cargo test -p codex-tui      # 507 tests

# Specific DCC tests
cargo test -p codex-stage0 -- dcc::tests

# Build
~/code/build-fast.sh
```

---

## 5. Success Criteria for P89

- [ ] Constitution memory schema documented and implemented
- [ ] `domain:constitution` with types `principle`, `guardrail`, `goal`, `non-goal`
- [ ] Overlay priority rules: guardrail=10, principle=9, goal/non-goal=8
- [ ] `constitution_version` tracking in overlay_db
- [ ] IQO union logic: constitution domain always included
- [ ] Separate always-on pass for constitution retrieval in DCC
- [ ] "Always include at least 3" selection logic implemented
- [ ] All existing Stage 0 tests still pass
- [ ] New tests for constitution-specific behavior

---

## 6. Session Prompt for P89

Copy this prompt to start the next session:

```
P89 Session: Constitution Data Model Implementation (SPEC-KIT-105 Phase 2)

Read docs/HANDOFF-P89.md for full context.

Primary Goal: Implement the constitution data model foundation.

Scope (STRICT - do not expand):
1. Constitution memory schema (domain:constitution + types)
2. Overlay scoring rules (initial_priority 9-10 for constitution)
3. Versioning system (constitution_version in overlay_db)
4. IQO union logic (always include constitution domain)
5. Separate always-on pass for constitution retrieval
6. "Always include N" selection behavior

OUT OF SCOPE (defer to P90):
- TASK_BRIEF Section 0 rendering
- New slash commands
- Phase -1 Gates
- NotebookLM artifacts
- E2E ferris-test benchmark
- Any enhancements (Q&A, drift detection, watcher)

Key Files:
- codex-rs/stage0/src/dcc.rs (IQO, selection)
- codex-rs/stage0/src/overlay_db.rs (priority, versioning)
- codex-rs/stage0/STAGE0_SCHEMA.sql (schema)

Tests: cargo test -p codex-stage0

Session Lineage: P72-P86 (Stage 0) → P87-P88 (Design) → P89 (Data Model)
```

---

## 7. Session Lineage

```
P72-P86: Stage 0 Implementation (SPEC-KIT-102)
    └── P87: Integration Testing + Gap Discovery
        ├── SPEC-KIT-900: E2E Test Harness
        └── SPEC-KIT-105: Constitution Enhancement (created)
    └── P88: Constitution Design Complete
        ├── Integration analysis
        ├── Gap analysis vs GitHub spec-kit
        └── SPEC-KIT-105 v2 (full design)
    └── P89: Data Model Implementation ← YOU ARE HERE
        └── P90+: TASK_BRIEF, Commands, Gates, E2E
```

---

## 8. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Schema changes break existing tests | Run full test suite after each change |
| IQO union logic affects non-constitution queries | Add flag to enable/disable constitution injection |
| Versioning adds complexity | Start simple (single counter), extend later |
| "Always include N" distorts MMR results | Apply after MMR, not during |

---

## 9. Definition of Done

P89 is complete when:
1. All 6 implementation tasks have code changes
2. `cargo test -p codex-stage0` passes (127+ tests)
3. New tests added for constitution-specific behavior
4. Changes committed with clear commit messages
5. HANDOFF-P90.md created for next phase

---

*Handoff prepared by P88 session. NotebookLM authentication required at session start.*
