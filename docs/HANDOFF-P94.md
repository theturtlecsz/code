# HANDOFF-P94: Constitution-Aware Pipeline (SPEC-KIT-105 Phase 7)

## Session Lineage
P89 (Data Model) → P90 (TASK_BRIEF + Tier-2) → P91 (Conflict Detection + Gate + Command) → P92 (Block + Cache + Plan) → P93 (Vision Q&A) → **P94** (Constitution-Aware Pipeline)

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

### Commits
- `0c9b7c3a6` - feat(tui): add /speckit.vision Q&A wizard (P93 Tasks 1-3)
- `0b59e12be` - test(stage0): add P93 vision front door tests (P93 Task 4)

## P94 Scope: Constitution-Aware Pipeline

### Primary Goal
Extend `/speckit.specify` and `/speckit.auto` to incorporate constitution content into prompts, ensuring agents respect project guardrails, principles, and goals.

### Task 1: `/speckit.specify` Constitution Context
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`
- Add constitution injection to `/speckit.specify` prompt template
- Include CONSTITUTION_BRIEF section before TASK_BRIEF
- Format: guardrails (priority 10), principles (priority 9), goals/non-goals (priority 8)

### Task 2: `/speckit.auto` Constitution Context
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs`
- Inject constitution context into Stage 0 prompts
- Ensure all pipeline stages see constitution content
- Use `get_constitution_memories()` from overlay DB

### Task 3: Constitution Conflict Detection
**Files**: `codex-rs/tui/src/chatwidget/spec_kit/handler.rs`
- Check SPEC content against constitution during pipeline
- Warn on potential goal/non-goal conflicts
- Use `check_constitution_alignment()` from tier2.rs

### Task 4: Eval Cases
**Files**: `codex-rs/stage0/src/eval.rs` (new)
- Create eval test cases for constitution enforcement
- Test: guardrail blocking, principle adherence, goal alignment

## Out of Scope (P95+)
- Drift detection (`/speckit.check-alignment`)
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
| `codex-rs/tui/src/bottom_pane/vision_builder_modal.rs` | Vision Q&A modal |
| `codex-rs/tui/src/chatwidget/spec_kit/vision_builder_handler.rs` | Vision event handler |

## Constitution Type Reference

| Type | Priority | Description |
|------|----------|-------------|
| Guardrail | 10 | Hard constraints that must never be violated |
| Principle | 9 | Architectural values and design principles |
| Goal | 8 | Mid-term objectives and success criteria |
| NonGoal | 8 | Explicit exclusions - what we don't build |
