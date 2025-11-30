# SPEC-KIT-970: Interactive PRD Builder Research

**Date**: 2025-11-30
**Status**: Research Complete
**Sources**: GitHub spec-kit, Martin Fowler SDD analysis, DeepWiki documentation

---

## Executive Summary

Upstream GitHub spec-kit implements a **two-phase clarification model**:
1. `/speckit.specify` generates PRD with `[NEEDS CLARIFICATION]` markers
2. `/speckit.clarify` resolves markers through structured Q&A

Our fork currently has instant `/speckit.new` with no clarification. This research informs adding interactive Q&A capability.

---

## Upstream Implementation Analysis

### Command Workflow (8 phases)

| Phase | Command | Output | Purpose |
|-------|---------|--------|---------|
| Governance | `/speckit.constitution` | `constitution.md` | Project principles |
| **Requirements** | `/speckit.specify` | `spec.md` | User stories + criteria |
| **Clarification** | `/speckit.clarify` | Updated `spec.md` | Resolve ambiguities |
| Design | `/speckit.plan` | `plan.md` | Technical architecture |
| Validation | `/speckit.analyze` | Analysis report | Cross-check alignment |
| Decomposition | `/speckit.tasks` | `tasks.md` | Actionable work items |
| Execution | `/speckit.implement` | Source code | Generate implementation |
| Quality | `/speckit.checklist` | Domain checklists | Validate requirements |

### Clarification Mechanics

**Marker Rules**:
- Maximum **3 total** `[NEEDS CLARIFICATION: ...]` markers per spec
- Only use when:
  - Significantly impacts scope or UX
  - Multiple reasonable interpretations exist
  - No industry standard default applies

**Priority Ranking**: scope > security/privacy > UX > technical details

**Question Format** (table-based):
```markdown
| Option | Answer | Implications |
|--------|--------|--------------|
| A | [First suggestion] | [Feature impact] |
| B | [Second suggestion] | [Feature impact] |
| C | [Third suggestion] | [Feature impact] |
| Custom | User-provided | [How to submit] |
```

**Response Format**: `Q1: A, Q2: Custom - [details], Q3: B`

### Success Criteria Standards

Must be:
- **Measurable** with specific metrics (time, percentage, count)
- **Technology-agnostic** (no frameworks, languages, databases)
- **User-focused** (outcomes, not system internals)
- **Verifiable** without implementation knowledge

*Bad*: "API response under 200ms" (too technical)
*Good*: "Users see results instantly" (user-centric)

---

## Current State (Our Fork)

```
/speckit.new <description>  → Instant PRD generation (no questions)
```

- No clarification phase
- No `[NEEDS CLARIFICATION]` markers
- No structured Q&A flow

---

## Implementation Options

### Option A: Enhanced /speckit.new with Flag

```bash
/speckit.new <description>           # Instant (current behavior)
/speckit.new -i <description>        # Interactive with questions
```

**Pros**: Backward compatible, explicit opt-in
**Cons**: Flag syntax unusual for slash commands

### Option B: Separate Clarify Command (Matches Upstream)

```bash
/speckit.new <description>           # Generates with [NEEDS CLARIFICATION] markers
/speckit.clarify SPEC-ID             # Resolves markers through Q&A
```

**Pros**: Matches upstream exactly, two-phase workflow
**Cons**: Requires running two commands, markers in generated PRD

### Option C: Inline Q&A Before Generation (Recommended)

```bash
/speckit.new <description>
# System asks 1-3 key questions inline
# User responds or types "skip" to generate immediately
# PRD generated with answers incorporated
```

**Pros**: Single command, natural chat flow, skip available
**Cons**: Different from upstream pattern

---

## Design Decisions

### 1. Modal Dialog vs Inline Q&A

| Approach | Pros | Cons |
|----------|------|------|
| **Modal** | Focused, clear boundaries, professional UX | Requires TUI modal infrastructure |
| Inline | Natural chat flow | Gets lost in conversation |

**Decision**: **Modal dialog** - focused interaction, clear boundaries, better UX.

### 2. Required vs Optional Questions

| Approach | Pros | Cons |
|----------|------|------|
| **Required** | Forces thoughtful input, higher quality PRDs | Must answer all |
| Optional | Fast path available | May skip important clarifications |

**Decision**: **Required questions** - no skip, ensures comprehensive specs.

### 3. Modal Flow

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
│  Press A-D or type custom answer    │
├─────────────────────────────────────┤
│  ESC: Cancel  │  Enter: Confirm     │
└─────────────────────────────────────┘
```

### 4. Question Sequence (All Required)

1. **Problem** - What problem does this solve?
2. **Target User** - Who is the primary user?
3. **Success Criteria** - How will you know it's complete?

---

## Proposed Question Categories

### Priority 1: Scope (Always Ask)
- "What specific problem does this solve?"
- "What is the minimum viable scope?"

### Priority 2: Target User (Ask if Unclear)
- "Who is the primary user?"
- Options: Developer, End-user, Admin, API consumer

### Priority 3: Success Criteria (Ask if Missing)
- "How will you know this feature is complete?"
- Freeform or suggest measurable criteria

### Priority 4: Constraints (Ask if Complex)
- "Any hard requirements or limitations?"
- "What should this explicitly NOT do?" (anti-goals)

---

## Recommended Implementation

### Phase 1: Modal Infrastructure

```rust
// New TUI modal component
struct PrdBuilderModal {
    questions: Vec<ClarificationQuestion>,
    current_index: usize,
    answers: HashMap<usize, String>,
    state: ModalState,  // Asking, Confirmed, Cancelled
}

impl PrdBuilderModal {
    fn render(&self, frame: &mut Frame, area: Rect) {
        // Draw bordered modal with current question
        // Show progress indicator [1/3]
        // Render options A-D
        // Show ESC/Enter hints
    }

    fn handle_input(&mut self, key: KeyEvent) -> ModalAction {
        // A-D: Select option
        // Enter: Confirm and advance
        // ESC: Cancel entire flow
    }
}
```

### Phase 2: Question Engine

```rust
struct ClarificationQuestion {
    category: QuestionCategory,  // Problem, Target, Success
    question: String,
    options: Vec<QuestionOption>,
}

struct QuestionOption {
    label: char,      // A, B, C, D
    answer: String,
    is_custom: bool,  // D = custom input
}

enum QuestionCategory {
    Problem,      // What problem does this solve?
    TargetUser,   // Who is the primary user?
    Success,      // How will you know it's complete?
}
```

### Phase 3: Integrate with /speckit.new

```rust
// Modified /speckit.new flow
1. Parse description
2. Launch PrdBuilderModal with 3 required questions
3. Collect all answers (no skip)
4. Generate PRD incorporating answers
5. Create spec file and branch
```

---

## Sources

- [GitHub Spec-Kit Repository](https://github.com/github/spec-kit)
- [Spec-Driven Development Guide](https://github.com/github/spec-kit/blob/main/spec-driven.md)
- [Martin Fowler: Understanding SDD Tools](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html)
- [DeepWiki: CLI Tool Reference](https://deepwiki.com/github/spec-kit/4-cli-tool-reference)
- [GitHub Blog: Spec-Driven Development](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)
- [Visual Studio Magazine: Spec Kit Analysis](https://visualstudiomagazine.com/articles/2025/09/16/github-spec-kit-experiment-a-lot-of-questions.aspx)

---

## Next Steps

1. Create SPEC-KIT-970 with `/speckit.new`
2. Build TUI modal infrastructure (Phase 1)
3. Implement question engine with 3 required questions (Phase 2)
4. Integrate modal with `/speckit.new` command (Phase 3)
