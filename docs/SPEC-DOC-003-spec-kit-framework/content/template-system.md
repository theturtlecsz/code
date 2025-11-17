# Template System

Comprehensive guide to PRD and document templates.

---

## Overview

The **Template System** provides standardized document structures for all Spec-Kit artifacts:

- **PRD template**: Product requirements document
- **Plan template**: Work breakdown and acceptance mapping
- **Tasks template**: Task decomposition and SPEC.md tracking
- **Evidence templates**: Telemetry JSON schemas
- **Quality gate templates**: Checkpoint results

**Purpose**:
- **Consistency**: All SPECs follow same structure
- **Completeness**: Templates include all required sections
- **Automation**: Templates enable automated validation
- **Onboarding**: Clear guidance for manual editing

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/templates/`

---

## PRD Template

### Template Structure

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs:100-200`

```markdown
# {feature_name}

**SPEC-ID**: {spec_id}
**Created**: {date}
**Status**: Draft

---

## Background

{description}

## Requirements

### Functional Requirements

- **FR-001**: [Describe first functional requirement]
- **FR-002**: [Describe second functional requirement]

### Non-Functional Requirements

- **NFR-001**: [Performance, scalability, security, etc.]

## Acceptance Criteria

### FR-001
- [ ] [Specific measurable criterion]
- [ ] [Another criterion]

### FR-002
- [ ] [Criterion]

## Constraints

- [Technical constraints]
- [Business constraints]
- [Time/resource constraints]

## Out of Scope

- [Explicitly state what's NOT included]

---

**Next Steps**: Run `/speckit.clarify {spec_id}` to detect ambiguities
```

**Variables**:
- `{feature_name}`: Capitalized description
- `{spec_id}`: Generated SPEC-ID (e.g., "SPEC-KIT-070")
- `{date}`: Current date (YYYY-MM-DD)
- `{description}`: User-provided description

---

### PRD Sections

#### Background

**Purpose**: Context and motivation

**Guidelines**:
- Explain the problem being solved
- Why this feature is needed
- Who benefits from it
- Current state vs desired state

**Example**:

```markdown
## Background

Users currently cannot customize the application's visual theme, forcing them to use the default light mode. This creates accessibility issues for users who prefer dark mode or have light sensitivity.

We need to implement a theme toggle that allows users to switch between light and dark modes, with system preference detection and persistence.

**Problem**: No theme customization
**Impact**: Poor accessibility for some users
**Solution**: Theme toggle with dark mode support
```

---

#### Requirements

**Purpose**: Detailed feature specifications

**Guidelines**:
- **Functional Requirements** (FR): What the system does
- **Non-Functional Requirements** (NFR): How well it does it
- Use numbered IDs (FR-001, FR-002, NFR-001)
- Be specific and measurable
- One requirement per ID

**Example**:

```markdown
## Requirements

### Functional Requirements

- **FR-001**: System must provide a visible toggle control for switching themes
- **FR-002**: Theme preference must persist across browser sessions
- **FR-003**: System must detect and apply OS/browser dark mode preference on first load
- **FR-004**: Manual toggle must override system preference

### Non-Functional Requirements

- **NFR-001**: Theme switching must complete within 200ms (p95)
- **NFR-002**: Dark mode must meet WCAG AA contrast ratios (4.5:1 text, 3:1 UI)
- **NFR-003**: Theme preference stored in localStorage (no server dependency)
```

---

#### Acceptance Criteria

**Purpose**: Measurable pass/fail conditions

**Guidelines**:
- One section per requirement ID
- Use checkboxes ([ ]) for tracking
- Be specific and testable
- Include edge cases

**Example**:

```markdown
## Acceptance Criteria

### FR-001
- [ ] Toggle control visible in settings menu
- [ ] Toggle shows current theme state (light/dark)
- [ ] Click toggle switches theme immediately

### FR-002
- [ ] Preference saved to localStorage on toggle
- [ ] Preference loaded and applied on page reload
- [ ] Works across browser tabs (storage event)

### FR-003
- [ ] System detects prefers-color-scheme media query
- [ ] Dark mode auto-applied if system preference is dark
- [ ] Light mode auto-applied if system preference is light

### FR-004
- [ ] Manual toggle overrides system preference
- [ ] Override persists until user toggles again
- [ ] Clear button to reset to system preference

### NFR-001
- [ ] Theme switch measured at <200ms (p95) in performance tests
- [ ] No visual flicker during transition

### NFR-002
- [ ] All text meets 4.5:1 contrast ratio in dark mode
- [ ] All UI elements meet 3:1 contrast ratio
- [ ] Automated contrast testing passes
```

---

#### Constraints

**Purpose**: Limitations and restrictions

**Guidelines**:
- Technical limitations (browser support, dependencies)
- Business constraints (budget, timeline)
- Design constraints (must match existing UI)
- Regulatory requirements (WCAG, GDPR)

**Example**:

```markdown
## Constraints

### Technical
- Must support Chrome 90+, Firefox 88+, Safari 14+
- No external dependencies (use native CSS custom properties)
- Must work without JavaScript (progressive enhancement)

### Business
- Budget: <$500 for implementation and testing
- Timeline: 2 weeks (Sprint 5)
- No breaking changes to existing UI components

### Design
- Toggle must match existing settings controls
- Dark mode palette must align with brand guidelines
- Animation duration <300ms for accessibility (prefers-reduced-motion)
```

---

#### Out of Scope

**Purpose**: Explicit exclusions

**Guidelines**:
- List features explicitly NOT included
- Clarify boundaries to prevent scope creep
- Reference future SPECs if applicable

**Example**:

```markdown
## Out of Scope

- Custom theme colors (only light/dark, no custom palettes)
- Per-component theme overrides (global theme only)
- Automatic time-based switching (no sunset/sunrise detection)
- Server-side preference storage (localStorage only)
- Mobile app theme support (web only)

**Future Work**: Custom theme colors planned for SPEC-KIT-075
```

---

## Plan Template

### Template Structure

**Location**: Agents generate this, but expected structure is defined

```markdown
# Plan: {feature_name}

## Inputs
- Spec: docs/{spec_id}-{slug}/spec.md (version/hash)
- Constitution: memory/constitution.md (version/hash)

## Work Breakdown

### Phase 1: {phase_name} ({duration})
{task_1}
{task_2}

### Phase 2: {phase_name} ({duration})
{task_1}
{task_2}

## Acceptance Mapping

| Requirement (Spec) | Validation Step | Test/Check Artifact |
| --- | --- | --- |
| {req_id}: {summary} | {validation} | {artifact} |

## Risks & Unknowns

### Risks
- **Risk**: {description}
  - Mitigation: {strategy}

### Unknowns
- **Unknown**: {question}
  - Research: {approach}

## Consensus & Risks (Multi-AI)

### Agreement
{areas_of_consensus}

### Disagreement & Resolution
{areas_of_disagreement_and_how_resolved}

## Exit Criteria (Done)

- [ ] All acceptance checks pass
- [ ] Docs updated (list files)
- [ ] Changelog/PR prepared
```

**Variables**:
- `{feature_name}`: From PRD
- `{spec_id}`: SPEC-ID
- `{slug}`: Directory slug
- `{phase_name}`: Phase description
- `{duration}`: Estimated time
- `{req_id}`: Requirement ID (FR-001, etc.)

---

### Plan Sections

#### Work Breakdown

**Purpose**: Phased task structure

**Guidelines**:
- Group tasks into logical phases
- Estimate duration for each phase
- Number tasks within phases
- Dependencies between tasks

**Example**:

```markdown
## Work Breakdown

### Phase 1: UI Components (3 days)
1.1 Create ThemeToggle component (1 day)
1.2 Add ThemeProvider context (1 day)
1.3 Update existing components for theme support (1 day)

### Phase 2: State Management (2 days)
2.1 Implement theme persistence (localStorage) (0.5 day)
2.2 Add system preference detection (0.5 day)
2.3 Create theme switching logic (1 day)

### Phase 3: Styling (2 days)
3.1 Define dark mode color palette (0.5 day)
3.2 Update CSS-in-JS styles (1 day)
3.3 Test contrast ratios (WCAG AA) (0.5 day)

**Total**: 7 days
```

---

#### Acceptance Mapping

**Purpose**: Link requirements to validation

**Guidelines**:
- One row per requirement
- Specify how to validate (manual, automated, both)
- Identify test artifact (file, tool, process)

**Example**:

```markdown
## Acceptance Mapping

| Requirement (Spec) | Validation Step | Test/Check Artifact |
| --- | --- | --- |
| FR-001: Toggle control | Manual inspection | Screenshot + accessibility audit |
| FR-002: Theme persistence | Automated test | `test_theme_persistence.rs` |
| FR-003: System preference | Manual + automated | `test_system_preference.rs` + manual check |
| FR-004: Manual override | Automated test | `test_manual_override.rs` |
| NFR-001: <200ms switch | Performance benchmark | `benchmark_theme_switch.rs` |
| NFR-002: WCAG AA contrast | Automated contrast testing | `axe-core` accessibility scan |
```

---

#### Risks & Unknowns

**Purpose**: Identify potential issues early

**Guidelines**:
- **Risks**: Known issues with mitigation strategies
- **Unknowns**: Questions requiring research
- Separate critical vs minor risks

**Example**:

```markdown
## Risks & Unknowns

### Risks

- **Risk**: Existing components may hardcode light theme colors
  - **Severity**: High
  - **Mitigation**: Audit all components, refactor to use theme context
  - **Timeline**: Add 1 day to Phase 1

- **Risk**: Browser support for prefers-color-scheme varies
  - **Severity**: Medium
  - **Mitigation**: Provide manual toggle fallback, test on target browsers
  - **Timeline**: Included in Phase 2

### Unknowns

- **Unknown**: Can localStorage events sync themes across tabs in real-time?
  - **Research**: Test storage event listeners in Chrome, Firefox, Safari
  - **Fallback**: Manual sync on tab focus if events don't work

- **Unknown**: What is acceptable color palette for dark mode?
  - **Research**: Review brand guidelines, consult design team
  - **Decision**: Defer to Phase 3, iterate on feedback
```

---

## Tasks Template

### Template Structure

**Location**: Agents generate this, structure defined

```markdown
# Tasks: {feature_name}

**SPEC-ID**: {spec_id}
**Generated**: {date}

---

## Task List

### T-001: {task_title}
- **Phase**: {phase_number}
- **Dependencies**: {dependent_task_ids}
- **Estimated Time**: {duration}
- **Assignee**: TBD
- **Description**: {detailed_description}
- **Acceptance**: {task_specific_criteria}

### T-002: {task_title}
...

---

## SPEC.md Tracker Update

Add the following rows to SPEC.md:

| Order | Task ID | Title | Status | PRD | Branch | PR | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | T-001 | {title} | Backlog | {spec_id} | - | - | - |
| 2 | T-002 | {title} | Backlog | {spec_id} | - | - | - |
```

**Variables**:
- `{task_title}`: Short task description
- `{phase_number}`: Phase from plan
- `{dependent_task_ids}`: Other tasks that must complete first
- `{duration}`: Estimated time (hours or days)

---

### Task Structure

**Purpose**: Granular implementation units

**Guidelines**:
- One task per discrete unit of work
- 1-3 days max per task (break larger into subtasks)
- Clear dependencies
- Specific acceptance criteria

**Example**:

```markdown
## Task List

### T-001: Create ThemeToggle component
- **Phase**: 1 (UI Components)
- **Dependencies**: None
- **Estimated Time**: 1 day
- **Assignee**: TBD
- **Description**:
  Create a reusable ThemeToggle component that renders a toggle switch for light/dark mode selection. Component should accept theme state and onChange callback as props.

  Implementation:
  - Create `ThemeToggle.tsx` in `src/components/`
  - Use existing Toggle component as base
  - Add icons for sun (light) and moon (dark)
  - Support keyboard navigation (Space, Enter)

- **Acceptance**:
  - [ ] Component renders correctly in both states
  - [ ] onClick triggers theme change
  - [ ] Keyboard accessible (Tab, Space, Enter)
  - [ ] Unit tests pass (>90% coverage)

### T-002: Add ThemeProvider context
- **Phase**: 1 (UI Components)
- **Dependencies**: T-001
- **Estimated Time**: 1 day
- **Assignee**: TBD
- **Description**:
  Create React context for theme state management. Provider should wrap the app and provide theme value and setter to all components.

  Implementation:
  - Create `ThemeContext.tsx` in `src/contexts/`
  - Define ThemeContext with { theme, setTheme }
  - Implement ThemeProvider with localStorage integration
  - Export useTheme hook for component consumption

- **Acceptance**:
  - [ ] Context provides current theme value
  - [ ] setTheme function updates theme globally
  - [ ] All components can access theme via useTheme()
  - [ ] Integration tests pass
```

---

## Evidence Templates

### Telemetry JSON Template

**Location**: Guardrail scripts generate this

```json
{
  "command": "{stage}",
  "specId": "{spec_id}",
  "sessionId": "{session_uuid}",
  "timestamp": "{iso_8601_timestamp}",
  "schemaVersion": "1.0",

  "baseline": {
    "mode": "file",
    "artifact": "docs/{spec_id}-{slug}/spec.md",
    "status": "exists"
  },

  "hooks": {
    "session": {
      "start": "passed"
    }
  },

  "agents": [
    {
      "name": "{agent_name}",
      "model": "{model_id}",
      "cost": {cost_float},
      "input_tokens": {input_count},
      "output_tokens": {output_count},
      "duration_ms": {duration},
      "status": "success"
    }
  ],

  "consensus": {
    "status": "ok",
    "present_agents": ["{agent1}", "{agent2}", "{agent3}"],
    "missing_agents": [],
    "conflicts": [],
    "mcp_calls": 1,
    "mcp_duration_ms": 8.7
  },

  "artifacts": [
    "docs/{spec_id}-{slug}/{stage}.md"
  ],

  "total_cost": {total_cost_float},
  "total_duration_ms": {total_duration},
  "exit_code": 0
}
```

**Variables**: All `{}` placeholders filled by guardrail scripts

---

### Quality Gate Template

**Location**: Quality gate handler generates this

```json
{
  "checkpoint": "{checkpoint_name}",
  "spec_id": "{spec_id}",
  "gate_type": "{clarify|analyze|checklist}",
  "timestamp": "{iso_8601_timestamp}",

  "native_result": {
    "overall_score": {score_0_100},
    "grade": "{A|B|C|D|F}",
    "issues": [
      {
        "id": "{issue_id}",
        "category": "{category}",
        "severity": "{CRITICAL|IMPORTANT|MINOR}",
        "description": "{issue_description}",
        "suggestion": "{fix_suggestion}"
      }
    ]
  },

  "gpt5_validations": [
    {
      "issue_id": "{issue_id}",
      "majority_answer": "{answer}",
      "gpt5_verdict": {
        "agrees_with_majority": {true|false},
        "reasoning": "{explanation}",
        "confidence": "{high|medium|low}"
      },
      "resolution": "{auto_applied|escalated}"
    }
  ],

  "user_escalations": [
    {
      "issue_id": "{issue_id}",
      "question": "{clarifying_question}",
      "user_answer": "{answer}",
      "resolution": "applied"
    }
  ],

  "outcome": {
    "status": "{passed|failed}",
    "initial_score": {score_before},
    "final_score": {score_after},
    "auto_resolved": {count},
    "gpt5_validated": {count},
    "user_escalated": {count}
  },

  "modified_files": [
    "docs/{spec_id}-{slug}/spec.md"
  ],

  "cost": {cost_float},
  "duration_ms": {duration}
}
```

---

## Template Usage

### Creating New Templates

**Steps**:
1. Identify common structure across documents
2. Extract variable placeholders (`{name}`)
3. Define default values for optional sections
4. Document template in code comments
5. Test template with sample data

**Example**:

```rust
pub fn fill_prd_template(
    spec_id: &str,
    feature_name: &str,
    description: &str,
) -> Result<String> {
    let date = Local::now().format("%Y-%m-%d").to_string();

    Ok(format!(r#"# {feature_name}

**SPEC-ID**: {spec_id}
**Created**: {date}
**Status**: Draft

---

## Background

{description}

## Requirements

### Functional Requirements

- **FR-001**: [Describe first functional requirement]

### Non-Functional Requirements

- **NFR-001**: [Performance, scalability, security, etc.]

---

**Next Steps**: Run `/speckit.clarify {spec_id}` to detect ambiguities
"#,
        feature_name = feature_name,
        spec_id = spec_id,
        date = date,
        description = description
    ))
}
```

---

### Customizing Templates

**Configuration** (future feature):

```toml
# .code/templates.toml

[prd]
sections = [
    "Background",
    "Requirements",
    "Acceptance Criteria",
    "Constraints",
    "Out of Scope"
]

[prd.requirements]
include_functional = true
include_non_functional = true
auto_number = true

[plan]
include_consensus = true
include_risks = true
table_format = "markdown"  # or "ascii"
```

**Note**: Template customization not yet implemented (planned for future release)

---

## Best Practices

### Template Design

**DO**:
- ✅ Use clear placeholder names (`{feature_name}`, not `{x}`)
- ✅ Provide inline guidance (`[Describe...]`)
- ✅ Include examples in comments
- ✅ Version schema (`"schemaVersion": "1.0"`)

**DON'T**:
- ❌ Hardcode values that should be variables
- ❌ Use ambiguous placeholders (`{data}`)
- ❌ Omit required fields
- ❌ Mix template versions in same SPEC

---

### Template Evolution

**When to Update**:
- New required field discovered
- Validation rules change
- User feedback on clarity

**How to Update**:
1. Increment schema version (`1.0` → `1.1`)
2. Document changes in migration guide
3. Support old versions temporarily
4. Provide upgrade tool

**Example**:

```rust
pub fn migrate_prd_v1_to_v2(prd_v1: &str) -> Result<String> {
    // Add new "Dependencies" section
    let sections = parse_sections(prd_v1)?;

    if !sections.contains_key("Dependencies") {
        sections.insert("Dependencies", "- None\n");
    }

    Ok(render_template(sections, "2.0"))
}
```

---

## Summary

**Template System Highlights**:

1. **Standardized Structures**: PRD, plan, tasks, telemetry, quality gates
2. **Variable Substitution**: Clear placeholders for dynamic content
3. **Inline Guidance**: Examples and descriptions for manual editing
4. **Schema Versioning**: Support for template evolution
5. **Automation-Friendly**: Enable validation and quality checks
6. **Consistency**: All SPECs follow same structure

**Next Steps**:
- [Workflow Patterns](workflow-patterns.md) - Common usage scenarios and examples

---

**File References**:
- PRD template: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs:100-200`
- Telemetry schema: Guardrail scripts (v1.0)
- Quality gate schema: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs`
