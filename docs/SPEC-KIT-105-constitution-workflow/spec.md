# SPEC-KIT-105: Constitution & Vision Workflow Enhancement

**Status**: RESEARCH
**Created**: 2025-12-01
**Session**: P87
**Priority**: P1 (Should Have)
**Depends On**: SPEC-KIT-900 (Integration Test Harness)

---

## 1. Problem Statement

### 1.1 Current Gap

When a user runs `/speckit.project`, they get a scaffolded project with placeholder content in `memory/constitution.md`:

```markdown
## Mission
[Define the project's core purpose]

## Principles
1. [Core principle 1]
...
```

There is no guided workflow to:
1. **Capture product vision** - What are we building and why?
2. **Establish architectural principles** - How do we build things here?
3. **Enforce constraints** - What rules must all specs follow?

This means `/speckit.new` and `/speckit.auto` operate without foundational context.

### 1.2 GitHub Spec-Kit Reference

[GitHub's spec-kit](https://github.com/github/spec-kit) addresses this with:
- **9 Articles Framework** - Immutable architectural principles (Library-First, Test-First, etc.)
- **Phase -1 Gates** - Specs must pass constitutional checks before implementation
- **Template embedding** - Constitution constraints are injected into spec templates

See: [spec-driven.md](https://github.com/github/spec-kit/blob/main/spec-driven.md)

---

## 2. Proposed Solution

### 2.1 New Commands

| Command | Purpose | Output |
|---------|---------|--------|
| `/speckit.vision` | Capture product mission via Q&A | Updates `memory/constitution.md` Mission section |
| `/speckit.constitution` | Establish architectural principles via Q&A | Updates `memory/constitution.md` Principles/Constraints |

### 2.2 Enhanced `/speckit.project`

```bash
# Option A: Flags
/speckit.project rust myapp --mission "A library for X"

# Option B: Interactive (preferred)
/speckit.project rust myapp
> What is the core mission? [Q&A flow]
> What are your architectural principles? [Q&A flow]
```

### 2.3 Integration Points

1. **Stage 0 (DCC)** - Include constitution in TASK_BRIEF context
2. **Divine Truth** - NotebookLM synthesis references constitution
3. **Spec Templates** - Embed constitutional constraints as "Phase -1 Gates"
4. **Agent Prompts** - Inject constitution into all stage agents

---

## 3. Research Required

Before implementation, analyze GitHub spec-kit deeper:

### 3.1 Research Tasks

- [ ] Clone and explore `github/spec-kit` repository structure
- [ ] Document the 9 Articles framework in detail
- [ ] Understand how Phase -1 Gates are enforced
- [ ] Analyze their template embedding mechanism
- [ ] Identify what to adopt vs. adapt for our use case

### 3.2 Key Questions

1. Should we adopt GitHub's 9 Articles or create our own?
2. How prescriptive vs. flexible should constitution be?
3. Should constitution be per-project or have global defaults?
4. How do we handle projects without constitution (backward compat)?

---

## 4. Acceptance Criteria

- [ ] `/speckit.vision` command implemented with Q&A modal
- [ ] `/speckit.constitution` command implemented with Q&A modal
- [ ] `memory/constitution.md` populated with real content after commands
- [ ] Stage 0 DCC includes constitution in context
- [ ] `/speckit.new` templates reference constitution
- [ ] Backward compatible (works without constitution)
- [ ] Documentation updated

---

## 5. Test Plan

### 5.1 Integration Test (SPEC-KIT-900)

Use `ferris-test` benchmark project at `/home/thetu/benchmark/ferris-test/`:

1. Run `/speckit.vision` with mission: "A library for printing text with Ferris as the mascot"
2. Run `/speckit.constitution` to establish principles
3. Run `/speckit.new "Add ANSI color support"`
4. Verify spec template includes constitutional constraints
5. Run `/speckit.auto` and verify Stage 0 context includes constitution

### 5.2 Validation Points

| Checkpoint | Verification |
|------------|--------------|
| Vision captured | `memory/constitution.md` Mission section filled |
| Principles set | `memory/constitution.md` Principles section filled |
| Stage 0 context | TASK_BRIEF.md includes constitution |
| Agent awareness | Agents log constitution reference |

---

## 6. Implementation Phases

### Phase 1: Research (This Session → Next)
- Deep dive into GitHub spec-kit
- Document findings in `research/` subdirectory
- Decide on framework approach

### Phase 2: Core Commands
- Implement `/speckit.vision`
- Implement `/speckit.constitution`
- Update `memory/constitution.md` template

### Phase 3: Integration
- Stage 0 DCC constitution injection
- Template embedding
- Agent prompt updates

### Phase 4: Validation
- E2E test with ferris-test benchmark
- Documentation

---

## 7. References

- [GitHub Spec-Kit Repository](https://github.com/github/spec-kit)
- [Spec-Driven Development Guide](https://github.com/github/spec-kit/blob/main/spec-driven.md)
- [GitHub Blog: Spec-driven development with AI](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)
- SPEC-KIT-900: Integration Test Harness (this repo)
- SPEC-KIT-102R: Stage 0 Implementation Report

---

## 8. Session Log

| Session | Date | Status | Notes |
|---------|------|--------|-------|
| P87 | 2025-12-01 | RESEARCH | Initial spec created, gap identified, GitHub spec-kit researched |

---

*This spec improves the foundational workflow for spec-driven development.*
