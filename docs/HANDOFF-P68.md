# P68 Session Handoff

**Previous**: P67 | **Commit**: `df7373818` | **Status**: Ready

---

## P67 Completed

### SPEC-KIT-970: Interactive PRD Builder
- Modal-based Q&A for `/speckit.new` (3 required questions)
- Files: `prd_builder_modal.rs`, `prd_builder_handler.rs`, `new_native.rs`
- Build verified, committed to main

---

## P68 Priorities

| # | Task | Effort | Description |
|---|------|--------|-------------|
| 1 | `/speckit.clarify` command | 45 min | Resolve [NEEDS CLARIFICATION] markers in existing specs |
| 2 | Question customization | 30 min | Project-type-aware questions (Rust/Python/TS/Generic) |
| 3 | Ferris benchmark | 30 min | Reference implementation at `/home/thetu/benchmark/reference/ferris-says` |

---

## Priority 1: /speckit.clarify Command

### Goal
Add command to resolve ambiguities in existing SPECs (upstream pattern).

### Design
```
/speckit.clarify SPEC-KIT-###
    ↓
1. Load spec.md and PRD.md from docs/SPEC-KIT-###-*/
2. Scan for [NEEDS CLARIFICATION: ...] markers
3. Present each as modal question (reuse PrdBuilderModal pattern)
4. Update files with resolved answers
5. Remove markers
```

### Implementation Steps
1. Create `clarify_command.rs` in `commands/`
2. Add `SpecKitClarifyCommand` to registry
3. Create `ClarifyModal` (simpler than PrdBuilderModal - freeform answers)
4. Add marker scanner in `clarify_native.rs` (exists, may need enhancement)
5. Wire AppEvent for completion

### Files to Create/Modify
| File | Action |
|------|--------|
| `commands/clarify.rs` | New command |
| `command_registry.rs` | Register command |
| `bottom_pane/clarify_modal.rs` | New modal |
| `clarify_native.rs` | Enhance marker scanner |

---

## Priority 2: Question Customization

### Goal
Make PRD builder questions context-aware based on project type.

### Design
```rust
enum ProjectType {
    Rust,      // Cargo.toml exists
    Python,    // pyproject.toml or setup.py
    TypeScript, // package.json with typescript
    Go,        // go.mod
    Generic,   // Fallback
}

fn get_questions_for_type(project_type: ProjectType) -> Vec<PrdQuestion> {
    match project_type {
        Rust => rust_questions(),      // "Crate or binary?", "Workspace member?"
        Python => python_questions(),  // "Library or CLI?", "Async required?"
        TypeScript => ts_questions(),  // "Frontend or backend?", "Framework?"
        // ...
    }
}
```

### Implementation Steps
1. Add `detect_project_type()` to `project_native.rs`
2. Create question sets per type in `prd_builder_modal.rs`
3. Pass project type when constructing modal
4. Keep fallback generic questions

---

## Priority 3: Ferris Benchmark

### Goal
Benchmark spec-kit against reference implementation.

### Reference
```
/home/thetu/benchmark/reference/ferris-says
```

### Tasks
1. Examine reference implementation
2. Create SPEC using `/speckit.new`
3. Run `/speckit.auto` through all stages
4. Compare output quality
5. Document findings

---

## Quick Reference

### Build
```bash
~/code/build-fast.sh run  # Build and run TUI
```

### Test PRD Builder
```
/speckit.new Add feature X
# Modal should appear with 3 questions
```

### Key Files
```
tui/src/bottom_pane/prd_builder_modal.rs  # Modal UI
tui/src/chatwidget/spec_kit/prd_builder_handler.rs  # Event handlers
tui/src/chatwidget/spec_kit/new_native.rs  # create_spec_with_context()
```

---

## Continuation Prompt

```
load docs/HANDOFF-P68.md

Begin Priority 1: Implement /speckit.clarify command following the design
in the handoff. Create the modal, command, and wire the events.
```

---

## Known State

- Branch: `main`
- 6 commits ahead of origin
- All tests passing
- Pre-commit hooks active
