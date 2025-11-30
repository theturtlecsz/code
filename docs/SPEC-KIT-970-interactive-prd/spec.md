# SPEC-KIT-970: Interactive PRD Builder

**Created**: 2025-11-30
**Priority**: P2 - Enhancement
**Status**: Implemented
**Branch**: feat/interactive-prd-builder

---

## Problem Statement

The current `/speckit.new` command takes a description string and generates a PRD instantly with no user interaction. This produces lower quality specs because:

1. User's intent is often ambiguous from a one-line description
2. Target users, success criteria, and constraints are guessed or omitted
3. No opportunity to clarify scope before generation

### Current Behavior
```
/speckit.new Add user authentication
→ Generates PRD immediately with assumptions
```

### Desired Behavior
```
/speckit.new Add user authentication
→ Modal dialog appears with 3 required questions
→ User answers each question
→ PRD generated incorporating answers
```

---

## Solution

Add a modal-based interactive Q&A flow to `/speckit.new` that asks 3 required questions before generating the PRD.

### Modal UI

```
┌─────────────────────────────────────┐
│  Interactive PRD Builder      [1/3] │
├─────────────────────────────────────┤
│                                     │
│  What problem does this solve?      │
│                                     │
│  [A] Performance issue              │
│  [B] Missing functionality          │
│  [C] Developer experience           │
│  [D] Custom...                      │
│                                     │
├─────────────────────────────────────┤
│  ESC: Cancel  │  Enter: Confirm     │
└─────────────────────────────────────┘
```

### Required Questions (No Skip)

| # | Category | Question | Options |
|---|----------|----------|---------|
| 1 | Problem | What problem does this solve? | A-D + Custom |
| 2 | Target | Who is the primary user? | Developer, End-user, Admin, Custom |
| 3 | Success | How will you know it's complete? | Freeform input |

---

## Technical Design

### Phase 1: Modal Infrastructure

New TUI component: `PrdBuilderModal`

```rust
struct PrdBuilderModal {
    questions: Vec<ClarificationQuestion>,
    current_index: usize,
    answers: HashMap<usize, String>,
    state: ModalState,
}

enum ModalState {
    Asking,
    Confirmed,
    Cancelled,
}

impl PrdBuilderModal {
    fn render(&self, frame: &mut Frame, area: Rect);
    fn handle_input(&mut self, key: KeyEvent) -> ModalAction;
}
```

### Phase 2: Question Engine

```rust
struct ClarificationQuestion {
    category: QuestionCategory,
    question: String,
    options: Vec<QuestionOption>,
}

struct QuestionOption {
    label: char,      // A, B, C, D
    answer: String,
    is_custom: bool,
}

enum QuestionCategory {
    Problem,
    TargetUser,
    Success,
}
```

### Phase 3: Integration

Modify `/speckit.new` handler in `tui/src/chatwidget/spec_kit/`:

1. Parse description from command
2. Launch `PrdBuilderModal`
3. Collect all 3 answers (required)
4. Pass answers to PRD generation template
5. Generate spec with enriched context

---

## File Changes

| File | Change |
|------|--------|
| `tui/src/modal/mod.rs` | New: Modal infrastructure |
| `tui/src/modal/prd_builder.rs` | New: PRD builder modal |
| `tui/src/chatwidget/spec_kit/new.rs` | Modify: Integrate modal |
| `tui/src/app.rs` | Modify: Handle modal events |

---

## Acceptance Criteria

- [ ] Modal renders correctly with bordered frame
- [ ] Progress indicator shows [1/3], [2/3], [3/3]
- [ ] All 3 questions are required (no skip)
- [ ] A-D selection works with keyboard
- [ ] Custom option (D) allows text input
- [ ] ESC cancels entire flow
- [ ] Enter confirms current answer and advances
- [ ] Answers incorporated into generated PRD
- [ ] Works with existing `/speckit.new` command syntax

---

## Out of Scope

- Editing answers after submission (use `/speckit.clarify`)
- Configurable question sets
- Saving answer templates for reuse

---

## Research

See `docs/SPEC-KIT-970-interactive-prd/research.md` for:
- Upstream spec-kit analysis
- Design decision rationale
- Question category priorities
