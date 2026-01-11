# PRD: Template Ecosystem Documentation & Multi-Agent Parity (SPEC-KIT-961)

**Version**: v20251129-templates-b
**Status**: Draft
**Author**: Claude (P59)
**Created**: 2025-11-29

## 1. Overview

### Problem Statement

The spec-kit template ecosystem has grown to 11 templates, but **agent parity is broken**:

1. **Claude-centric documentation**: CLAUDE.md is loaded only by Claude sessions - Gemini and Code/GPT agents never see it
2. **Inconsistent template context**: prompts.json references templates differently per agent, some miss template refs entirely
3. **No shared context infrastructure**: Each agent type operates with different levels of template awareness
4. **Variable schema undocumented**: Substitution patterns are implicit in `new_native.rs`

**Critical insight**: CLAUDE.md is a Claude-specific file. **Gemini and Code are first-class citizens** and must receive equal template context through the shared `prompts.json` infrastructure.

### Solution

Create comprehensive template documentation and **standardize template context delivery across ALL agent types** through prompts.json - the only shared prompt infrastructure.

### Success Criteria
- [ ] All three agent types (gemini, claude, code) receive identical template context
- [ ] prompts.json updated with standardized `template_context` blocks
- [ ] Complete template inventory documented in `docs/spec-kit/TEMPLATES.md`
- [ ] Variable substitution schema formalized
- [ ] `/speckit.project` includes all 11 templates
- [ ] Validation script ensures agent parity

## 2. Multi-Agent Architecture Analysis

### Current Agent Infrastructure

| Agent | Prompt Source | Template Access | Session Context |
|-------|--------------|-----------------|-----------------|
| **Gemini** | prompts.json | Template refs in prompts | None (stateless) |
| **Claude** | prompts.json + CLAUDE.md | Template refs + CLAUDE.md | Session memory |
| **Code/GPT** | prompts.json | Template refs in prompts | None (stateless) |

**Problem**: Claude gets CLAUDE.md context that Gemini and Code never see.

### Target Architecture (Agent Parity)

| Agent | Prompt Source | Template Access | Parity Status |
|-------|--------------|-----------------|---------------|
| **Gemini** | prompts.json | Standardized template_context block | ✅ Equal |
| **Claude** | prompts.json | Standardized template_context block | ✅ Equal |
| **Code/GPT** | prompts.json | Standardized template_context block | ✅ Equal |

**Solution**: Move all template context into prompts.json where ALL agents can access it.

### prompts.json as Shared Infrastructure

```
┌─────────────────────────────────────────────────────────────┐
│                     prompts.json                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ template_context: {                                      ││
│  │   templates: [...],  // All 11 templates                ││
│  │   variables: {...},  // Substitution schema             ││
│  │   usage: {...}       // Stage → template mapping        ││
│  │ }                                                        ││
│  └─────────────────────────────────────────────────────────┘│
│                           │                                  │
│      ┌────────────────────┼────────────────────┐            │
│      ▼                    ▼                    ▼            │
│  ┌────────┐          ┌────────┐          ┌────────┐        │
│  │ Gemini │          │ Claude │          │ Code   │        │
│  │ Agent  │          │ Agent  │          │ Agent  │        │
│  └────────┘          └────────┘          └────────┘        │
│      │                    │                    │            │
│      └────────────────────┴────────────────────┘            │
│              ALL receive identical context                   │
└─────────────────────────────────────────────────────────────┘
```

## 3. Template Inventory

### Current Templates (~/code/templates/)

| Template | Stage | Purpose | Used By |
|----------|-------|---------|---------|
| `PRD-template.md` | Intake | Product Requirements Document | specify |
| `spec-template.md` | Intake | SPEC directory structure | new |
| `plan-template.md` | Plan | Work breakdown structure | plan (gemini, claude, gpt_pro) |
| `tasks-template.md` | Tasks | Task decomposition format | tasks (gemini, claude, gpt_pro) |
| `implement-template.md` | Implement | Implementation checklist | implement (gemini, claude, gpt_codex, gpt_pro) |
| `validate-template.md` | Validate | Test strategy template | validate (gemini, claude, gpt_pro) |
| `audit-template.md` | Audit | Compliance checklist | audit (gemini, claude, gpt_pro) |
| `unlock-template.md` | Unlock | Ship decision criteria | unlock (gemini, claude, gpt_pro) |
| `clarify-template.md` | Quality | Ambiguity detection output | clarify (gemini, claude, code) |
| `analyze-template.md` | Quality | Consistency analysis output | analyze (gemini, claude, code) |
| `checklist-template.md` | Quality | Quality scoring rubric | checklist (gemini, claude, code) |

### Current prompts.json Template References (Audit)

| Stage | Gemini | Claude | Code/GPT | Parity? |
|-------|--------|--------|----------|---------|
| spec-plan | ✅ refs plan-template | ✅ refs plan-template | ✅ refs plan-template | ✅ |
| spec-tasks | ✅ refs tasks-template | ✅ refs tasks-template | ❌ NO REF | ❌ BROKEN |
| spec-implement | ✅ refs implement-template | ✅ refs implement-template | ✅ refs implement-template | ✅ |
| spec-validate | ✅ refs validate-template | ✅ refs validate-template | ✅ refs validate-template | ✅ |
| spec-audit | ✅ refs audit-template | ✅ refs audit-template | ✅ refs audit-template | ✅ |
| spec-unlock | ✅ refs unlock-template | ✅ refs unlock-template | ✅ refs unlock-template | ✅ |
| spec-clarify | ❌ NO REF | ❌ NO REF | ❌ NO REF | ❌ BROKEN |
| spec-analyze | ❌ NO REF | ❌ NO REF | ❌ NO REF | ❌ BROKEN |
| spec-checklist | ❌ NO REF | ❌ NO REF | ❌ NO REF | ❌ BROKEN |

**Finding**: 4 stages have broken parity (spec-tasks for code, and all quality-gate stages).

## 4. Functional Requirements

### FR-1: Standardized Template Context Block

Add top-level `template_context` to prompts.json:

```json
{
  "template_context": {
    "version": "20251129-templates-a",
    "templates": {
      "plan": "~/.code/templates/plan-template.md",
      "tasks": "~/.code/templates/tasks-template.md",
      "implement": "~/.code/templates/implement-template.md",
      "validate": "~/.code/templates/validate-template.md",
      "audit": "~/.code/templates/audit-template.md",
      "unlock": "~/.code/templates/unlock-template.md",
      "clarify": "~/.code/templates/clarify-template.md",
      "analyze": "~/.code/templates/analyze-template.md",
      "checklist": "~/.code/templates/checklist-template.md",
      "PRD": "~/.code/templates/PRD-template.md",
      "spec": "~/.code/templates/spec-template.md"
    },
    "variables": {
      "SPEC_ID": "SPEC identifier (e.g., SPEC-KIT-961)",
      "PROJECT_NAME": "Project name from cwd or argument",
      "DATE": "Current date (YYYY-MM-DD)",
      "DESCRIPTION": "Feature description",
      "VERSION": "PRD version (v{date}-{slug}-a)",
      "AUTHOR": "Author name"
    },
    "stage_mapping": {
      "spec-specify": ["PRD"],
      "spec-plan": ["plan"],
      "spec-tasks": ["tasks"],
      "spec-implement": ["implement"],
      "spec-validate": ["validate"],
      "spec-audit": ["audit"],
      "spec-unlock": ["unlock"],
      "spec-clarify": ["clarify"],
      "spec-analyze": ["analyze"],
      "spec-checklist": ["checklist"]
    }
  }
}
```

### FR-2: Agent Prompt Standardization

Update ALL agent prompts with consistent template reference format:

**Before** (inconsistent):
```
"prompt": "Template: ~/.code/templates/plan-template.md\n\nTask:..."
```

**After** (standardized):
```
"prompt": "Template Reference: ${TEMPLATE_CONTEXT.templates.plan}\nTemplate Variables: SPEC_ID, DATE\n\nTask:..."
```

### FR-3: Fix Parity Gaps

Update these stages to include template refs for ALL agents:

1. **spec-tasks**: Add template ref to `code` agent prompt
2. **spec-clarify**: Add clarify-template ref to ALL agents (gemini, claude, code)
3. **spec-analyze**: Add analyze-template ref to ALL agents
4. **spec-checklist**: Add checklist-template ref to ALL agents
5. **quality-gate-***: Mirror updates to quality gate variants

### FR-4: Template Documentation

Create `docs/spec-kit/TEMPLATES.md`:
- Complete inventory with file paths
- Variable schema with examples
- Stage-to-template mapping
- Agent usage patterns
- Cross-reference to prompts.json

### FR-5: /speckit.project Full Templates

Update `project_native.rs` to scaffold all 11 templates:
- Currently: PRD-template.md, spec-template.md (2 of 11)
- Target: All 11 templates
- Creates `templates/` directory with complete set

### FR-6: Validation Script

Create `scripts/validate-template-parity.sh`:
```bash
#!/bin/bash
# Checks:
# 1. All 11 templates exist in templates/
# 2. All stages in prompts.json reference appropriate template
# 3. ALL agent types (gemini, claude, code) have template ref
# 4. Template variables are consistent
```

## 5. Non-Functional Requirements

### NFR-1: Agent Equality
- No agent receives more template context than another
- prompts.json is the single source of truth for multi-agent context
- CLAUDE.md may duplicate info but must not be sole source

### NFR-2: Backward Compatibility
- Existing prompts continue to work
- Template refs are additive, not breaking
- Version field tracks changes

### NFR-3: Maintainability
- Single source: `template_context` block in prompts.json
- Validation script prevents drift
- Template changes propagate through variable expansion

## 6. Multi-Agent Validation Tasks (Non-Authoritative)

> **Policy Compliance (GR-001)**: These agents are *non-authoritative*. They do not vote, merge, or decide.
> They only produce critiques/checklists. The phase owner remains responsible for incorporating feedback.
> See `docs/MODEL-POLICY.md` for current model routing policy.

### Task 1: Template Structure Audit (Critic-Only)
**Agents**: gemini, claude, code (parallel validation - not consensus)
**Output**: Each agent produces independent critique containing:
- Inconsistent section headers across templates
- Variable naming pattern violations
- Missing sections compared to stage requirements
- Misalignment with expected JSON output schemas

### Task 2: Agent Parity Verification (Validator-Only)
**Agents**: gemini, claude, code (independent analysis)
**Output**: Each agent reports its own template access:
- What template context do I receive?
- What stages am I missing template refs?
- What would make my template awareness equal to other agents?

### Task 3: prompts.json Enhancement Review (Critic-Only)
**Agents**: gemini, claude, code (independent proposals)
**Output**: Each agent proposes `template_context` structure:
- What fields should be included?
- How should stage-to-template mapping work?
- What variable schema format is most useful?

**Note**: No synthesis or voting. Phase owner reviews all critiques and decides.

## 7. Implementation Phases

### Phase 1: Audit & Documentation (30 min)
- Create `docs/spec-kit/TEMPLATES.md`
- Document current parity gaps
- Formalize variable schema

### Phase 2: prompts.json Enhancement (45 min)
- Add `template_context` top-level block
- Fix parity gaps (spec-tasks/code, quality-gates)
- Standardize template reference format
- Version bump all affected stages

### Phase 3: Validation Infrastructure (30 min)
- Create `scripts/validate-template-parity.sh`
- Add to CI/pre-commit (optional)
- Document validation process

### Phase 4: /speckit.project Update (45 min)
- Add all 11 templates to `project_native.rs`
- Update tests to verify full template set
- Update PRD-960 notes

### Phase 5: Multi-Agent Testing (30 min)
- Run consensus tasks with all three agents
- Verify each agent reports equal template access
- Document any remaining gaps

## 8. Testing Plan

### Parity Tests
- [ ] `test_gemini_template_access()` - verify template refs in all gemini prompts
- [ ] `test_claude_template_access()` - verify template refs in all claude prompts
- [ ] `test_code_template_access()` - verify template refs in all code prompts
- [ ] `test_template_refs_identical()` - compare refs across agents per stage

### Template Tests
- [ ] All 11 templates exist in `templates/`
- [ ] All templates have consistent header format
- [ ] All variables documented in schema
- [ ] Variables used consistently across templates

### Integration Tests
- [ ] `/speckit.project` creates all 11 templates
- [ ] `/speckit.plan` produces output matching plan-template.md
- [ ] Validation script passes with no gaps

## 9. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing prompts | High | Version field, additive changes only |
| Agent performance impact from larger prompts | Low | Template refs are paths, not content |
| Template drift from documentation | Medium | Validation script, CI enforcement |
| Inconsistent variable names | Medium | Single schema in template_context |

## 10. Future Considerations

- **Runtime template injection**: Load template content into prompts, not just paths
- **Agent-specific template variants**: If agents need different output formats
- **Template versioning**: Track template changes for migration
- **Custom templates**: User-defined templates override defaults

## 11. References

- SPEC-KIT-960: /speckit.project command
- `docs/spec-kit/prompts.json`: Agent prompt definitions (THE shared infrastructure)
- `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs`: Variable substitution
- `templates/`: Source templates directory
- CLAUDE.md: Claude-specific context (NOT shared with other agents)

---

## Appendix A: Current prompts.json Parity Analysis

### Stages with FULL Parity (all agents have template ref)
- spec-plan ✅
- spec-implement ✅
- spec-validate ✅
- spec-audit ✅
- spec-unlock ✅

### Stages with BROKEN Parity
- spec-tasks: code agent missing template ref
- spec-clarify: ALL agents missing template ref
- spec-analyze: ALL agents missing template ref
- spec-checklist: ALL agents missing template ref
- quality-gate-clarify: ALL agents missing template ref
- quality-gate-analyze: ALL agents missing template ref
- quality-gate-checklist: ALL agents missing template ref

### Fix Priority
1. **Critical**: spec-clarify, spec-analyze, spec-checklist (used frequently)
2. **Important**: quality-gate-* variants (mirror of above)
3. **Minor**: spec-tasks/code (less impactful)

---

## Appendix B: Model & Runtime (Spec Overrides)

Policy: docs/MODEL-POLICY.md (version: 1.0.0)

Roles exercised by this spec:
- Stage0 Tier2 (NotebookLM): NO
- Architect/Planner: NO
- Implementer/Rust Ace: NO
- Librarian: NO
- Tutor: NO
- Auditor/Judge: NO

This spec is **primarily infrastructure** (template ecosystem, prompts.json parity).

Section 6 "Multi-Agent Validation Tasks" uses **non-authoritative sidecars** only:
- Agents produce independent critiques (no voting/consensus)
- Phase owner incorporates feedback (single owner decides)
- Pattern compliant with GR-001 (no consensus)

Privacy:
- local_only = true (template operations are local)

High-risk:
- HR = NO (template changes are reversible)

Overrides:
- None

---

Back to [Key Docs](../KEY_DOCS.md)
