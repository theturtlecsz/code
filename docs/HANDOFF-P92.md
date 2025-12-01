# P92 Session Handoff: Constitution Enforcement Complete (SPEC-KIT-105 Phase 5)

## Session Lineage
P89 (Data Model) → P90 (TASK_BRIEF + Tier-2) → P91 (Conflict Detection + Gate + Command) → **P92** (Plan Command + Block Mode + Cache Invalidation)

## P92 Scope (Agreed)

### Core Deliverables

#### Task 1: Tier 2 Cache Invalidation on Constitution Changes
**Files**: `codex-rs/stage0/src/overlay_db.rs`, `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`

**Why Core P92**: Closes the constitution lifecycle loop. Without this, a constitution change leaves NotebookLM's cached Divine Truth out of sync.

**Implementation**:
1. Add `invalidate_tier2_by_constitution()` to overlay_db.rs
   - Query all Tier 2 cache entries with constitution memory dependencies
   - Delete stale entries when constitution version changes
2. Wire invalidation into `/speckit.constitution sync`
   - After regenerating NL_CONSTITUTION.md, call invalidation
3. Wire invalidation into `upsert_constitution_memory()`
   - Any constitution memory change triggers cache invalidation

**Acceptance Criteria**:
- [ ] Constitution change → Tier 2 cache entries with `domain:constitution` dependencies dropped
- [ ] Next Stage 0 run forces fresh Tier 2 synthesis
- [ ] Test: verify cache miss after constitution sync

---

#### Task 2: `/speckit.plan` Command
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`, `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`

**Purpose**: Stage 0 + planning agents only, stops before implementation. Gives spec-kit parity with `/plan + /tasks`.

**Subcommands**:
- Default: Run Stage 0 → Specify → Plan → Tasks → STOP (no Implement/Validate/Audit)

**Implementation**:
1. Create `SpecKitPlanCommand` struct in special.rs
2. Create `handle_spec_plan()` in pipeline_coordinator.rs
   - Reuse `handle_spec_auto` logic but with `SpecStage::Tasks` as terminal stage
   - Call `run_constitution_readiness_gate()` before starting
3. Register command in command_registry.rs and slash_command.rs

**Acceptance Criteria**:
- [ ] `/speckit.plan SPEC-XXX` runs Stage 0 → planning stages only
- [ ] Implementation/Validate/Audit stages are NOT invoked
- [ ] Constitution gate runs before planning starts
- [ ] Output shows Divine Truth with Constitution Alignment section

---

#### Task 3: `GateMode::Block` Implementation
**Files**: `codex-rs/stage0/src/config.rs`, `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`

**Purpose**: Turn warn-only gates into enforcement. Block pipeline when constitution not ready.

**Implementation**:
1. Add `Block` variant to `GateMode` enum in config.rs
2. Update `run_constitution_readiness_gate()` in pipeline_coordinator.rs:
   - If `GateMode::Warn`: show warnings, continue (existing)
   - If `GateMode::Skip`: do nothing (existing)
   - If `GateMode::Block`: show warnings AND return early (abort pipeline)
3. Update `/speckit.auto` and `/speckit.plan` to respect Block mode
4. Keep default as `Warn` for backwards compatibility

**Acceptance Criteria**:
- [ ] `phase1_gate_mode = "block"` in config aborts `/speckit.auto` when constitution missing
- [ ] `phase1_gate_mode = "block"` aborts `/speckit.plan` when constitution missing
- [ ] Clear error message shown to user explaining why blocked
- [ ] Default remains `Warn` (non-breaking)

---

#### Task 4: Unit + Integration Tests
**Files**: `codex-rs/stage0/src/lib.rs`, `codex-rs/tui/tests/spec_kit_commands_tests.rs` (new)

**Unit Tests (Stage0)**:
- Gate function: missing vs present constitution
- Block vs Warn vs Skip decision logic
- Cache invalidation function

**Integration Tests (TUI)**:
- `plan_respects_block_mode_when_no_constitution`:
  - Arrange: zero constitution memories, `phase1_gate_mode=Block`
  - Act: `/speckit.plan SPEC-...`
  - Assert: fails with clear message, Stage 0/agents not invoked
- `plan_runs_when_constitution_ready`:
  - Arrange: at least one principle + one guardrail, `phase1_gate_mode=Block`
  - Act: `/speckit.plan SPEC-...`
  - Assert: Stage 0 invoked, planning runs, implementation does NOT
- `auto_is_blocked_by_constitution_in_block_mode`:
  - Mirror test for `/speckit.auto`

---

## Key APIs from P91

```rust
// Stage0 exports
pub use config::{GateMode, Stage0Config, VectorIndexConfig};
pub use overlay_db::{ConstitutionType, OverlayDb, ...};

// Constitution readiness check
pub fn check_constitution_readiness(db: &OverlayDb) -> Vec<String>;

// Constitution DB methods
db.get_constitution_meta() -> (version, hash, updated_at)
db.get_constitution_memories(limit) -> Vec<ConstitutionMemory>
db.upsert_constitution_memory(id, type, content)
db.increment_constitution_version(hash)
db.constitution_memory_count() -> usize

// Gate function
run_constitution_readiness_gate(widget: &mut ChatWidget)
```

---

## Implementation Order

1. **Task 3: GateMode::Block** (smallest, enables testing of other tasks)
2. **Task 1: Cache Invalidation** (critical correctness fix)
3. **Task 2: /speckit.plan** (builds on gate infrastructure)
4. **Task 4: Tests** (verify all behavior)

---

## Out of Scope (P93+)

- `/speckit.vision` command (Q&A → NL_VISION.md)
- Constitution drift detection
- NotebookLM artifact auto-seeding on constitution change
- Interactive constitution editor modal

---

## Test Commands

```bash
cd codex-rs
cargo test -p codex-stage0 -- --test-threads=1
cargo test -p codex-tui --test spec_kit_commands_tests
~/code/build-fast.sh
```

---

## Success Metrics

1. Constitution change → stale Tier 2 cache automatically dropped
2. `/speckit.plan` runs Stage 0 + planning only (no implementation)
3. `GateMode::Block` prevents pipeline when constitution incomplete
4. All new tests pass
5. Default behavior unchanged (Warn mode)
