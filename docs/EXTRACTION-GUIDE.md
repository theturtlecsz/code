# ChatWidget Extraction Guide

A step-by-step guide for extracting functionality from `chatwidget/mod.rs` into dedicated modules, based on patterns established in MAINT-11 Phases 1-7.

---

## Overview

The ChatWidget monolith (`mod.rs`) contains 20K+ LOC. This guide documents the process for safely extracting cohesive functionality into separate modules.

---

## Step 1: Investigation

### Find Extraction Candidates
```bash
cd codex-rs

# Find handler functions by pattern
grep -n "fn.*handler_name\|HandlerName" tui/src/chatwidget/mod.rs | head -40

# Find function groups by prefix
grep -n "pub(crate) fn prefix_" tui/src/chatwidget/mod.rs

# Count potential extraction size
awk '/^    pub\(crate\) fn target_func/,/^    pub\(crate\) fn |^    fn |^impl |^}$/{print NR": "$0}' \
    tui/src/chatwidget/mod.rs | head -100
```

### Check External Callers
```bash
# Find all references outside mod.rs
grep -rn "function_name\|TypeName" tui/src/ --include="*.rs" | grep -v mod.rs

# Check app.rs specifically (common caller)
grep -n "widget.function_name" tui/src/app.rs
```

### Identify Dependencies
- What imports does the code need?
- What struct fields does it access?
- What other ChatWidget methods does it call?

---

## Step 2: Create New Module

### File Structure
```rust
// chatwidget/new_module.rs

// MAINT-11 Phase N: Description of extracted functionality

// Required imports from external crates
use codex_core::protocol::SomeType;
use codex_core::other::AnotherType;

// Required imports from crate
use crate::app_event::AppEvent;
use crate::history_cell;

// Import ChatWidget from parent module
use super::ChatWidget;

impl ChatWidget<'_> {
    // Extracted functions go here
    pub(crate) fn extracted_function(&mut self) {
        // Implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Add tests for the extracted code
    }
}
```

### Visibility Guidelines
- `pub(crate)` for functions called from `app.rs` or other modules
- `pub(crate)` for helper functions used by event handlers in `mod.rs`
- Private (`fn`) for internal helpers only used within the new module

---

## Step 3: Update mod.rs

### Add Module Declaration
```rust
// In mod.rs, add to the MAINT-11 section
// MAINT-11: Extracted rendering helpers
mod agent_status;
mod command_render;
mod input_helpers;
mod new_module;  // <- Add here
mod review_handlers;
mod submit_helpers;
```

### Remove Extracted Code
Replace the extracted impl block with a comment:
```rust
impl ChatWidget<'_> {
    // FunctionName handlers extracted to new_module.rs (MAINT-11 Phase N)

    // Keep next function that wasn't extracted
    pub(crate) fn next_function(&mut self) {
```

### Clean Up Unused Imports
Remove imports that were only used by extracted code:
```rust
// Before: use codex_core::some_type::SomeType;
// After:  // SomeType moved to new_module.rs (MAINT-11 Phase N)
```

---

## Step 4: Verification

### Build Check
```bash
cargo build -p codex-tui 2>&1 | head -50
```

### Clippy Check (Zero Warnings)
```bash
cargo clippy -p codex-tui -- -D warnings
```

### Run Tests
```bash
# Module-specific tests
cargo test -p codex-tui new_module

# Full TUI tests
cargo test -p codex-tui

# Workspace tests
cargo test --workspace
```

### Verify LOC Reduction
```bash
echo "Previous: N LOC"
wc -l tui/src/chatwidget/mod.rs
wc -l tui/src/chatwidget/new_module.rs
```

---

## Step 5: Documentation

### Update MAINT-11-EXTRACTION-PLAN.md
- Add phase to Completed Phases table
- Update Current LOC count
- Update Metrics section
- Move phase from Planned to Completed

### Update HANDOFF.md
- Document what was extracted
- Update module structure diagram
- Prepare next session guidance

---

## Common Patterns

### Pattern A: Handler Functions
Functions that handle specific commands or events.

```rust
// In new_module.rs
impl ChatWidget<'_> {
    pub(crate) fn handle_x_command(&mut self, args: String) {
        // Validation
        if self.is_task_running() {
            self.history_push(history_cell::new_error_event(/* ... */));
            return;
        }
        // Main logic
        self.do_something();
    }
}
```

### Pattern B: UI Presenter Functions
Functions that display UI elements (modals, pickers, etc.).

```rust
impl ChatWidget<'_> {
    pub(crate) fn show_x_picker(&mut self, items: Vec<Item>) {
        let view = ListSelectionView::new(/* ... */);
        self.bottom_pane.show_list_selection(/* ... */);
    }
}
```

### Pattern C: State Check Functions
Small functions that check widget state.

```rust
impl ChatWidget<'_> {
    pub(crate) fn is_x_active(&self) -> bool {
        self.some_field.is_some() || self.other_field.is_some()
    }
}
```

---

## Common Pitfalls

### 1. Circular Imports
**Problem**: Module A imports from Module B which imports from Module A.
**Solution**: Keep shared types in mod.rs or create a separate types module.

### 2. Private Field Access
**Problem**: Extracted code needs access to private ChatWidget fields.
**Solution**: Fields accessed via `self` work automatically since the extracted code is still `impl ChatWidget`.

### 3. Forgetting Re-exports
**Problem**: External callers can't find the moved functions.
**Solution**: Functions are methods on `impl ChatWidget`, so they're automatically accessible via `widget.function_name()`.

### 4. Breaking Event Handlers
**Problem**: Event handlers in mod.rs call extracted functions.
**Solution**: Keep helper functions `pub(crate)` so event handlers can call them.

---

## Checklist

```markdown
- [ ] Investigation complete
  - [ ] Identified all functions to extract
  - [ ] Found all external callers
  - [ ] Listed required imports
- [ ] New module created
  - [ ] File created with proper header
  - [ ] All functions moved
  - [ ] Tests added
- [ ] mod.rs updated
  - [ ] Module declaration added
  - [ ] Extracted code removed
  - [ ] Comment added noting extraction
  - [ ] Unused imports removed
- [ ] Verification complete
  - [ ] Build passes
  - [ ] Clippy clean (0 warnings)
  - [ ] TUI tests pass
  - [ ] Module tests pass
- [ ] Documentation updated
  - [ ] MAINT-11-EXTRACTION-PLAN.md updated
  - [ ] HANDOFF.md updated
  - [ ] LOC reduction documented
```

---

## Extraction History (MAINT-11)

| Phase | Module | LOC | Key Pattern |
|-------|--------|-----|-------------|
| 1 | command_render.rs | ~200 | Rendering helpers |
| 2 | agent_status.rs | ~65 | Types + helpers |
| 3 | submit_helpers.rs | ~300 | Message submission |
| 4 | (cleanup) | ~5 | Dead code removal |
| 5 | input_helpers.rs | ~54 | Input normalization |
| 6 | (removal) | ~2,094 | Browser dead code |
| 7 | review_handlers.rs | ~408 | Handler functions + UI |

---

_Created: 2025-12-16 (P118)_
