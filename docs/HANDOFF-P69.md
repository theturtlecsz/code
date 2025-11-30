# P69 Session Handoff

**Previous**: P68 | **Commit**: (pending) | **Status**: Ready

---

## P68 Completed

### SPEC-KIT-971: Interactive Clarify Command
- `/speckit.clarify` now scans for `[NEEDS CLARIFICATION: ...]` markers
- Modal-based Q&A for resolving markers (freeform text input)
- Falls back to ambiguity heuristics when no markers found
- Files created: `clarify_modal.rs`, `clarify_handler.rs`
- Files modified: `clarify_native.rs`, `quality.rs`, `app.rs`, `app_event.rs`, `mod.rs` (multiple)
- Build verified, tests passing

---

## P69 Priorities

| # | Task | Effort | Description |
|---|------|--------|-------------|
| 1 | Pre-commit cleanup | 10 min | Commit P68, fix command registry test assertion |
| 2 | Minimal clarify tests | 15 min | Add marker regex test to clarify_native |
| 3 | Question customization | 45 min | Project-type-aware questions for BOTH PRD & Clarify modals |
| 4 | Ferris benchmark | 30 min | Benchmark against reference, track in SPEC.md |

---

## Priority 1: Pre-commit Cleanup

### Tasks
1. Commit P68 changes (SPEC-KIT-971)
2. Fix command registry test assertion in `command_registry.rs`
   - Test `test_global_registry_populated` expects 30 commands
   - Test `test_all_names_count` expects 44 total names
   - Verify counts are still correct (no new commands added, just modal)

### Commit Message Template
```
feat(spec-kit): Interactive clarify modal (SPEC-KIT-971)

- Add [NEEDS CLARIFICATION: ...] marker scanner
- Modal-based Q&A for resolving markers
- Falls back to ambiguity heuristics when no markers
```

---

## Priority 2: Minimal Clarify Tests

### Tasks
Add to `clarify_native.rs` tests:
```rust
#[test]
fn test_clarification_marker_regex() {
    // Test the [NEEDS CLARIFICATION: ...] pattern matching
    // Test extraction of question text
    // Test multi-marker detection in single file
}
```

Location: `tui/src/chatwidget/spec_kit/clarify_native.rs` (after existing tests)

---

## Priority 3: Question Customization (Extended Scope)

### Goal
Make BOTH PRD builder AND clarify modal project-type-aware.

### Design
```rust
enum ProjectType {
    Rust,       // Cargo.toml exists
    Python,     // pyproject.toml or setup.py
    TypeScript, // package.json with typescript
    Go,         // go.mod
    Generic,    // Fallback
}

fn detect_project_type(cwd: &Path) -> ProjectType {
    if cwd.join("Cargo.toml").exists() { return ProjectType::Rust; }
    if cwd.join("pyproject.toml").exists() { return ProjectType::Python; }
    // ...
}
```

### PRD Builder Questions (per type)
| Type | Question 1 | Question 2 | Question 3 |
|------|-----------|------------|------------|
| Rust | "Crate or binary?" | "Workspace member?" | (existing) |
| Python | "Library or CLI?" | "Async required?" | (existing) |
| TypeScript | "Frontend or backend?" | "Framework?" | (existing) |
| Generic | (existing) | (existing) | (existing) |

### Clarify Modal Enhancement
- Add project-type context to question display
- Show relevant hints based on detected type
- Example: For Rust projects, hint about `#[cfg(...)]` for conditional features

### Implementation Steps
1. Create `project_native.rs::detect_project_type()` (may already exist - check first)
2. Add project-specific question sets to `prd_builder_modal.rs`
3. Add contextual hints to `clarify_modal.rs`
4. Pass project type when constructing modals

### Files to Modify
| File | Changes |
|------|---------|
| `project_native.rs` | Add/verify `detect_project_type()` |
| `prd_builder_modal.rs` | Project-specific questions |
| `clarify_modal.rs` | Contextual hints per project type |
| `commands/special.rs` | Pass project type to PRD builder |
| `commands/quality.rs` | Pass project type to clarify modal |

---

## Priority 4: Ferris Benchmark

### Goal
Benchmark spec-kit against the ferris-says reference implementation.

### Reference Location
```
/home/thetu/benchmark/reference/ferris-says
```

### Tasks
1. Examine reference implementation structure
2. Create SPEC using `/speckit.new` with descriptive feature request
3. Run `/speckit.auto SPEC-ID` through all stages
4. Compare output quality against reference
5. Track results in SPEC.md (pass/fail, notes)

### SPEC.md Tracking Format
```markdown
## Benchmarks

| Reference | SPEC-ID | Result | Notes |
|-----------|---------|--------|-------|
| ferris-says | SPEC-KIT-XXX | PASS/FAIL | Brief notes |
```

---

## Quick Reference

### Build
```bash
~/code/build-fast.sh run  # Build and run TUI
```

### Test
```bash
cargo test -p codex-tui clarify  # Clarify tests
cargo test -p codex-tui -- command_registry  # Registry tests
```

### Key Files
```
tui/src/bottom_pane/clarify_modal.rs       # Clarify modal UI
tui/src/bottom_pane/prd_builder_modal.rs   # PRD builder UI
tui/src/chatwidget/spec_kit/clarify_native.rs  # Marker scanner
tui/src/chatwidget/spec_kit/project_native.rs  # Project detection (check)
```

---

## Continuation Prompt

```
load docs/HANDOFF-P69.md

Begin Priority 1: Pre-commit cleanup
1. Run tests to verify command registry counts are still correct
2. Commit P68 changes (SPEC-KIT-971: Interactive clarify modal)

Then proceed to Priority 2 (minimal tests), Priority 3 (question customization
for BOTH modals), and Priority 4 (Ferris benchmark with SPEC.md tracking).
```

---

## Known State

- Branch: `main`
- Tests: All passing (8 clarify-related, full suite green)
- Build: Verified
- Pre-commit hooks: Active
