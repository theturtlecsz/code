# HANDOFF-P94: Drift Detection Foundation (SPEC-KIT-105 Phase 7)

## Session Lineage
P89 (Data Model) → P90 (TASK_BRIEF + Tier-2) → P91 (Conflict Detection + Gate + Command) → P92 (Block + Cache + Plan) → P93 (Vision Q&A) → **P94** (Drift Detection)

## P93 Completion Summary

### Implemented
1. **`/speckit.vision` Command** - Q&A wizard for guided constitution creation
   - Interactive modal with 5 questions: target users, problem, goals, non-goals, principles
   - Option selection (A-D) or custom input mode
   - Files: `vision_builder_modal.rs`, `SpecKitVisionCommand` in `special.rs`

2. **Constitution Data Model Mapping**
   - Goals → `ConstitutionType::Goal` (priority 8)
   - Non-goals → `ConstitutionType::NonGoal` (priority 8)
   - Principles → `ConstitutionType::Principle` (priority 9)
   - File: `vision_builder_handler.rs`

3. **NL_VISION.md Generation**
   - Creates `memory/NL_VISION.md` artifact
   - Aggregates all vision answers into structured markdown

4. **Tests** - 7 new tests (169 total Stage0 tests pass)
   - Vision memory types and priorities
   - Constitution version increment
   - Tier 2 cache invalidation
   - Gate behavior validation

5. **P93 Refinement (post-review)**
   - `/speckit.constitution view` now separates Goals/Non-Goals
   - Shows vision source count
   - Vision completion hints about adding guardrails

### Commits
- `0c9b7c3a6` - feat(tui): add /speckit.vision Q&A wizard (P93 Tasks 1-3)
- `0b59e12be` - test(stage0): add P93 vision front door tests (P93 Task 4)
- `071725ec8` - docs(SPEC-KIT-105): add P94 session handoff
- `e80a57705` - refactor(tui): enhance constitution view (P93 refinement)

---

## P94 Scope: Drift Detection Foundation

### Design Decisions (from P93 review)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Vision guardrails | **Add 6th question** | Single wizard achieves gate-ready state |
| Report format | **TUI + JSON** | Human dashboard + CI automation |
| Drift depth | **Version-only (fast)** | Cheap/scalable; content analysis as future `--deep` mode |
| Task scope | **Tasks 1-4** | Core + logging; `/speckit.specify` deferred to P95 |

### Task 1: Add Guardrail Question to `/speckit.vision`
**Files**: `codex-rs/tui/src/bottom_pane/vision_builder_modal.rs`, `vision_builder_handler.rs`
- Add 6th question: "What are your hard constraints? (security, privacy, data residency, etc.)"
- Map answers to `ConstitutionType::Guardrail` (priority 10)
- Single `/speckit.vision` run now achieves "constitution-ready" state
- Update tests to cover new flow

### Task 2: Track `constitution_version` at Spec Creation
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs`
- Add `constitution_version_at_creation` field to spec metadata
- Store in spec header (SPEC.md frontmatter) or sidecar JSON
- Query current version from `get_constitution_version()`
- Ensure Task Brief metadata includes both versions

### Task 3: Implement `/speckit.check-alignment`
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`, `command_registry.rs`

**Default mode (TUI table):**
```
SPEC ID          | Created Ver | Current Ver | Status
-----------------|-------------|-------------|--------
SPEC-KIT-102     | 1           | 3           | stale
SPEC-KIT-103     | 3           | 3           | fresh
SPEC-OLD-001     | -           | 3           | unknown
```

**JSON mode (`--json` flag):**
```json
[
  {
    "spec_id": "SPEC-KIT-102",
    "project": "codex-rs",
    "constitution_version_at_creation": 1,
    "current_constitution_version": 3,
    "staleness": "stale"
  }
]
```

- Version-only comparison: `fresh | stale | unknown`
- No Tier-2 calls (fast, cheap, CI-friendly)
- Content analysis deferred to future `--deep` mode (P95+)

### Task 4: Add Event Logging
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/vision_builder_handler.rs`, `handler.rs`

**VisionDefined event:**
```rust
tracing::info!(
    event_type = "VisionDefined",
    constitution_version = new_version,
    goals_count = goals.len(),
    nongoals_count = nongoals.len(),
    principles_count = principles.len(),
    guardrails_count = guardrails.len(),  // New in P94
    "Vision defined for project"
);
```

**AlignmentCheckRun event:**
```rust
tracing::info!(
    event_type = "AlignmentCheckRun",
    total_specs = specs.len(),
    fresh_count,
    stale_count,
    unknown_count,
    "Alignment check completed"
);
```

---

## Out of Scope (P95)

- Constitution-aware `/speckit.specify` with conflict prompts
- Content-level drift detection (`--deep` mode)
- Auto-fix for drifted specs
- Constitution sync from external files

---

## Tests

```bash
cd codex-rs && cargo test -p codex-stage0 -- --test-threads=1
cargo test -p codex-tui --lib command_registry
~/code/build-fast.sh
```

### New Tests to Add
- `test_vision_creates_guardrail_memories` - Guardrail question → priority 10
- `test_spec_records_constitution_version` - Version tracking at creation
- `test_check_alignment_detects_stale_specs` - Drift detection logic
- `test_check_alignment_json_output` - JSON export format

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `codex-rs/stage0/src/overlay_db.rs` | Constitution CRUD, version tracking |
| `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs` | Spec creation logic |
| `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs` | Commands |
| `codex-rs/tui/src/bottom_pane/vision_builder_modal.rs` | Vision Q&A modal |
| `codex-rs/tui/src/chatwidget/spec_kit/vision_builder_handler.rs` | Vision handler |

## Constitution Type Reference

| Type | Priority | Source |
|------|----------|--------|
| Guardrail | 10 | `/speckit.vision` Q6 or `/speckit.constitution add` |
| Principle | 9 | `/speckit.vision` Q5 |
| Goal | 8 | `/speckit.vision` Q3 |
| NonGoal | 8 | `/speckit.vision` Q4 |

---

## Why This Scope?

With P89–P93 complete:
- Constitution data model is stable ✓
- Vision capture feeds real goals/principles ✓
- Gates and conflict detection are wired ✓
- NL_VISION/NL_CONSTITUTION stay in sync ✓

P94 completes the "constitution lifecycle":
1. **Capture** - `/speckit.vision` now creates gate-ready constitution (with guardrails)
2. **Track** - Specs record which constitution version they were created under
3. **Monitor** - `/speckit.check-alignment` shows drift status across all specs
4. **Audit** - Event logging provides telemetry for CI and dashboards

After P94, SPEC-KIT-105 is functionally complete. P95 can focus on UX polish (`/speckit.specify` with conflict prompts) or move to the next spec (SPEC-KIT-103/104).
