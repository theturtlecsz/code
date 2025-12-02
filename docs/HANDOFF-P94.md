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
- `078488d41` - refactor(tui): enhance constitution view (P93 refinement)

## P94 Scope: Drift Detection Foundation

### Primary Goal
Pivot from "front door and enforcement" to **maintenance and drift**. Add tooling to detect when specs are out of alignment with the current constitution version.

### Task 1: Track constitution_version at spec creation
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs`
- Add `constitution_version` field to spec metadata at `/speckit.new` time
- Store in spec header (SPEC.md) or separate metadata file
- Query current version from `get_constitution_version()`

### Task 2: Implement `/speckit.check-alignment`
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`, `command_registry.rs`
- New command that scans specs vs current constitution_version
- Report specs created under earlier versions
- Surface specs with recorded `constitution_conflicts` from Stage0Result
- Read-only dashboard/report (no auto-fixes)

### Task 3: Add VisionDefined event logging
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/vision_builder_handler.rs`
- Log distinct event when vision is (re)defined:
  - `event_type=VisionDefined`
  - `constitution_version`
  - counts of goals/non-goals/principles
- Prepares for future drift detection tooling

### Task 4: Constitution-aware `/speckit.specify` (stretch)
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs`
- Pull relevant principles/guardrails into spec refinement
- Dynamic questions like: "This spec proposes X; Guardrail G2 says Y. How will you reconcile?"
- Uses conflict detection wired in P91–P92

## Out of Scope (P95+)
- Auto-fix for drifted specs
- Constitution sync from external files
- Multi-project constitution federation

## Tests

```bash
cd codex-rs && cargo test -p codex-stage0 -- --test-threads=1
~/code/build-fast.sh
```

## Key Files Reference

| File | Purpose |
|------|---------|
| `codex-rs/stage0/src/overlay_db.rs` | Constitution CRUD, cache invalidation |
| `codex-rs/stage0/src/tier2.rs` | Alignment checking, conflict detection |
| `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs` | Stage0 context injection |
| `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs` | Command implementations |
| `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs` | Spec creation logic |
| `codex-rs/tui/src/bottom_pane/vision_builder_modal.rs` | Vision Q&A modal |
| `codex-rs/tui/src/chatwidget/spec_kit/vision_builder_handler.rs` | Vision event handler |

## Constitution Type Reference

| Type | Priority | Description |
|------|----------|-------------|
| Guardrail | 10 | Hard constraints that must never be violated |
| Principle | 9 | Architectural values and design principles |
| Goal | 8 | Mid-term objectives and success criteria |
| NonGoal | 8 | Explicit exclusions - what we don't build |

## Why Drift Detection Now?

With P89–P93 complete:
- Constitution data model is stable
- Vision capture feeds real goals/principles
- Gates and conflict detection are wired
- NL_VISION/NL_CONSTITUTION stay in sync

P94 is the natural pivot point to answer: **"Which specs are out of date with the constitution?"** This makes SPEC-KIT-105 feel complete:
- Vision and constitution can be captured/refined ✓
- Gates and alignment enforced at plan/auto time ✓
- Tools to see which specs are drifting ← P94
