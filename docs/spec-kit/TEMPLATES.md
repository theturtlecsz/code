# Spec-Kit Template Ecosystem

**Version**: 20251129-templates-a
**Maintained by**: Multi-agent consensus infrastructure

---

## Overview

The spec-kit template ecosystem provides 11 standardized templates for multi-agent PRD workflows. These templates ensure consistent output structure across all three agent types (Gemini, Claude, Code/GPT).

**Key Principle**: All agents receive identical template context through `prompts.json`. This ensures agent parity - no agent has more context than another.

---

## Template Inventory

| Template | File | Stage(s) | Purpose |
|----------|------|----------|---------|
| **PRD** | `templates/PRD-template.md` | specify | Product Requirements Document structure |
| **Spec** | `templates/spec-template.md` | new | SPEC directory scaffold |
| **Plan** | `templates/plan-template.md` | plan | Work breakdown and acceptance mapping |
| **Tasks** | `templates/tasks-template.md` | tasks | Task decomposition format |
| **Implement** | `templates/implement-template.md` | implement | Implementation checklist |
| **Validate** | `templates/validate-template.md` | validate | Test strategy template |
| **Audit** | `templates/audit-template.md` | audit | Compliance checklist |
| **Unlock** | `templates/unlock-template.md` | unlock | Ship decision criteria |
| **Clarify** | `templates/clarify-template.md` | clarify, quality-gate-clarify | Ambiguity detection output |
| **Analyze** | `templates/analyze-template.md` | analyze, quality-gate-analyze | Consistency analysis output |
| **Checklist** | `templates/checklist-template.md` | checklist, quality-gate-checklist | Quality scoring rubric |

---

## Variable Schema

Templates use placeholder variables that are substituted at runtime:

| Variable | Type | Source | Example |
|----------|------|--------|---------|
| `SPEC_ID` | string | Command argument | `SPEC-KIT-961` |
| `PROJECT_NAME` | string | cwd or argument | `my-project` |
| `DATE` | string | System date | `2025-11-29` |
| `DESCRIPTION` | string | User input | `Add authentication` |
| `VERSION` | string | Generated | `v20251129-auth-a` |
| `AUTHOR` | string | Git config | `Claude (P60)` |
| `PROMPT_VERSION` | string | prompts.json | `20251129-clarify-b` |
| `MODEL_ID` | string | Runtime | `gemini-2.5-flash` |
| `CONTEXT` | string | Loaded files | PRD/spec content |
| `ARTIFACTS` | string | Stage inputs | Previous stage outputs |
| `PREVIOUS_OUTPUTS` | object | Agent chain | Earlier agent responses |

### Variable Expansion

Variables are expanded in two locations:

1. **Native commands** (`/speckit.new`, `/speckit.project`): Substituted by Rust code in `new_native.rs`
2. **Agent prompts**: Substituted by the TUI orchestrator before sending to model

---

## Stage-to-Template Mapping

```
Stage                   Template              Agents
─────────────────────────────────────────────────────
spec-specify        →   PRD-template.md       gpt_pro
spec-plan           →   plan-template.md      gemini, claude, gpt_pro
spec-tasks          →   tasks-template.md     gemini, claude, gpt_pro
spec-implement      →   implement-template.md gemini, claude, gpt_codex, gpt_pro
spec-validate       →   validate-template.md  gemini, claude, gpt_pro
spec-audit          →   audit-template.md     gemini, claude, gpt_pro
spec-unlock         →   unlock-template.md    gemini, claude, gpt_pro
spec-clarify        →   clarify-template.md   gemini, claude, code
spec-analyze        →   analyze-template.md   gemini, claude, code
spec-checklist      →   checklist-template.md gemini, claude, code
quality-gate-*      →   (mirrors spec-*)      gemini, claude, code
```

---

## Agent Parity Architecture

### The Problem (Pre-SPEC-KIT-961)

```
┌─────────────────────────────────────────────────────────────────┐
│ BROKEN PARITY (Pre-fix)                                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Claude Sessions:                                               │
│  ┌─────────────┐    ┌─────────────┐                            │
│  │ CLAUDE.md   │ +  │ prompts.json│ = Full template context    │
│  │ (auto-load) │    │ (prompts)   │                            │
│  └─────────────┘    └─────────────┘                            │
│                                                                 │
│  Spawned Agents (Gemini, Code):                                 │
│  ┌─────────────┐    ┌─────────────┐                            │
│  │ NO SESSION  │    │ prompts.json│ = Partial template refs    │
│  │ FILE LOADED │    │ (prompts)   │   (4 stages broken)        │
│  └─────────────┘    └─────────────┘                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Key insight**: Spawned agents don't load session files like CLAUDE.md. Creating GEMINI.md or CODE.md would NOT help because these files are never loaded by spawned agents.

### The Solution (SPEC-KIT-961)

```
┌─────────────────────────────────────────────────────────────────┐
│ FULL PARITY (Post-fix)                                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                    prompts.json                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ template_context: {                                      │   │
│  │   templates: { plan: "...", tasks: "...", ... },        │   │
│  │   variables: { SPEC_ID: "...", DATE: "...", ... },      │   │
│  │   stage_mapping: { spec-plan: ["plan"], ... }           │   │
│  │ }                                                        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                           │                                     │
│      ┌────────────────────┼────────────────────┐               │
│      ▼                    ▼                    ▼               │
│  ┌────────┐          ┌────────┐          ┌────────┐           │
│  │ Gemini │          │ Claude │          │ Code   │           │
│  └────────┘          └────────┘          └────────┘           │
│      ALL receive IDENTICAL template context                    │
└─────────────────────────────────────────────────────────────────┘
```

**Solution**: The `template_context` block in `prompts.json` is the single source of truth for all agents. Every agent prompt includes a `Template:` reference line pointing to the relevant template file.

---

## Template Structure Standards

All templates follow consistent structure:

### Header Block
```markdown
# [Stage Name]: [FEATURE_NAME]

**SPEC-ID**: [SPEC_ID]
**[Stage] Version**: [VERSION]
**Created**: [DATE]

---

## Inputs
[Source artifacts with paths/hashes]
```

### Content Sections
- Stage-specific content organized by topic
- Quality scores where applicable (0-10 scale)
- Multi-agent consensus section

### Footer Block
```markdown
## Multi-Agent Consensus

### Agreements
- [Consensus items]

### Divergent Assessments
[Resolution of disagreements]

---

## Evidence References
[Paths to consensus artifacts]
```

---

## Adding New Templates

When adding a new stage or template:

1. **Create template file**: `templates/[stage]-template.md`
2. **Update prompts.json**:
   - Add to `template_context.templates`
   - Add to `template_context.stage_mapping`
   - Add `Template:` reference to ALL agent prompts
3. **Update validation**: Add to `scripts/validate-template-parity.sh`
4. **Update /speckit.project**: Add to `create_templates_dir()` in `project_native.rs`

### Checklist for New Templates

- [ ] Template file created in `templates/`
- [ ] Added to `template_context.templates` in prompts.json
- [ ] Added to `template_context.stage_mapping` in prompts.json
- [ ] ALL agents in the stage have `Template:` line in prompt
- [ ] Version bumped for affected stage prompts
- [ ] Validation script updated
- [ ] /speckit.project includes new template

---

## Validation

Run the parity validation script:

```bash
./scripts/validate-template-parity.sh
```

Checks performed:
1. All 11 templates exist in `templates/`
2. All stages in prompts.json have `template_context` refs
3. ALL agent types have `Template:` line per stage
4. No parity gaps between agent types

---

## References

- **prompts.json**: `docs/spec-kit/prompts.json` - Agent prompt definitions
- **Template source**: `templates/` - All 11 templates
- **Variable expansion**: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs`
- **Project scaffold**: `codex-rs/tui/src/chatwidget/spec_kit/project_native.rs`
- **SPEC-KIT-961 PRD**: `docs/SPEC-KIT-961-template-ecosystem/PRD.md`

---

## Changelog

### 20251129-templates-a
- Added `template_context` block to prompts.json
- Fixed parity for 6 stages (18 agent prompts):
  - spec-clarify (gemini, claude, code)
  - spec-analyze (gemini, claude, code)
  - spec-checklist (gemini, claude, code)
  - quality-gate-clarify (gemini, claude, code)
  - quality-gate-analyze (gemini, claude, code)
  - quality-gate-checklist (gemini, claude, code)
- Created TEMPLATES.md documentation
