# PRD: /model Command Expansion to All 13 Supported Models

**SPEC-ID**: SPEC-KIT-946
**Status**: Draft
**Created**: 2025-11-19
**Author**: Manual SPEC creation from comprehensive prompt
**Priority**: P2 - MEDIUM (UX enhancement)

---

## Problem Statement

**Current State**:
The `/model` command only shows **7 GPT-5 presets** (all with reasoning levels).

- Implementation: `common/src/model_presets.rs` - hardcoded GPT-5 variants
- Defined presets:
  1. gpt-5-codex-low
  2. gpt-5-codex-medium
  3. gpt-5-codex-high
  4. gpt-5 minimal
  5. gpt-5 low
  6. gpt-5 medium (default)
  7. gpt-5 high

**Pain Points**:
- Users cannot select Gemini or Claude models via `/model` command
- Missing 6 model families: Gemini 2.5 Flash, 2.5 Pro, 3 Pro, Claude Haiku 4.5, Sonnet 4.5, Opus 4.1
- Spec-kit pipeline configurator has 13 models, but TUI `/model` has only 7
- Inconsistent: Pipeline uses all models, but user can't select them for chat
- Manual workarounds required (edit config or use pipeline configurator)

**Impact**:
Reduced user experience and productivity. Users cannot easily access the full range of available models for chat sessions, limiting their ability to choose cost-effective or specialized models for different tasks.

---

## User Personas

### Persona 1: Developer
**Description**: Software engineer using the spec-kit system for daily development tasks.

**Current Workflow**:
- Uses pipeline configurator to select models for spec-kit stages
- Cannot easily switch chat model to match pipeline model
- Must manually edit config files to use non-GPT-5 models

**Frustrations**:
- Inconsistent model availability between pipeline and chat
- Lack of access to cost-effective Gemini Flash for quick queries
- Cannot use Claude models for specialized tasks (code review, creative tasks)

**Desired Outcome**:
- Quick access to all 13 models via `/model` command
- Consistent experience across TUI
- Clear pricing information to make informed choices

### Persona 2: Power User
**Description**: Advanced user optimizing costs and leveraging specialized models.

**Current Workflow**:
- Researches optimal models for different task types
- Uses pipeline configurator for automated workflows
- Wants to use same model selection logic for chat

**Frustrations**:
- Limited to expensive GPT-5 models for chat
- Cannot experiment with different providers
- No visibility into model costs during selection

**Desired Outcome**:
- Access to full model catalog
- Cost transparency during selection
- Ability to switch models mid-conversation

---

## Goals & Success Metrics

### Goal 1: Complete Model Coverage
Implement full access to all 13 models in the `/model` command.

**Measurable Criteria**:
- All 3 Gemini models selectable (Flash, 2.5 Pro, 3 Pro)
- All 3 Claude models selectable (Haiku 4.5, Sonnet 4.5, Opus 4.1)
- All GPT-5.1 variants selectable (Mini, Standard, Codex with reasoning levels)
- Total: ~18-20 presets available (13 models × reasoning variants)

### Goal 2: User Experience Quality
Maintain high-quality UX with organized, clear model selection.

**Measurable Criteria**:
- Model picker loads <100ms
- Clear grouping by family (Gemini / Claude / GPT-5.1)
- Pricing information visible for each model
- 100% keyboard navigation support (↑↓ + Enter)
- Display names match pipeline configurator (consistency)

### Goal 3: Registry Consistency
Ensure single source of truth for model definitions.

**Measurable Criteria**:
- Zero duplicate model definitions
- Pricing updates propagate automatically
- Adding new models requires 1 place to update
- All model metadata consistent across TUI

---

## Non-Goals

**What We Won't Do**:
- Search/filter functionality (future enhancement)
- Model comparison view (separate feature)
- Performance benchmarks in modal (too much complexity)
- Custom model configurations (advanced feature)
- Multi-model selection (conversation uses 1 model at a time)

**Why These Are Non-Goals**:
Focus on core functionality first. Advanced features can be added in future iterations based on user feedback. Current scope provides immediate value while minimizing complexity.

---

## In-Scope Features

### Phase 1: Model Registry Expansion
- Add Gemini 2.5 Flash, 2.5 Pro, 3 Pro presets
- Add Claude Haiku 4.5, Sonnet 4.5, Opus 4.1 presets
- Add GPT-5.1 Mini preset (currently missing)
- Update all "gpt-5" → "gpt-5.1" in existing presets
- Include pricing hints in descriptions

### Phase 2: Reasoning Level Handling
- GPT-5.1: Support reasoning (minimal/low/medium/high)
- Gemini: No reasoning levels (Deep Think is separate product)
- Claude: No public reasoning levels
- Organize presets: Flat list vs grouped by family

### Phase 3: UI/UX Enhancements
- Group headers (if grouped approach chosen)
- Show pricing tier in each item
- Current selection highlighted
- Cost estimate per model
- Type-ahead search (optional)

### Phase 4: Configuration Integration
- Sync with spec-kit registry
- Import from `get_all_available_models()`
- Shared registry module (DRY principle)
- Pricing display using `ModelPricing::for_model()`

---

## Assumptions & Constraints

### Assumptions
- Required dependencies are available (spec-kit registry, cost tracker)
- All 13 models remain available via API
- Pricing structure remains stable
- No breaking changes in upstream model APIs
- SPEC-950 model registry validation is complete

### Technical Constraints
- Must maintain backward compatibility with existing `/model` command
- Reasoning levels only for GPT-5.1 (not Gemini/Claude)
- Model names must match spec-kit registry exactly
- UI modal has size constraints (max ~20 visible items)

### Resource Constraints
- Development time: 5-8 hours estimated
- No breaking changes to existing functionality
- Must work with current TUI architecture

### Time Constraints
- Target completion: After SPEC-950 validation, before next major feature
- No hard deadline (quality-of-life improvement)

---

## Requirements

### Functional Requirements

#### FR1: Model Registry Expansion
**Description**: Expand ModelPreset array to include all 13 models.

**Acceptance Criteria**:
- Gemini 3 Pro preset added with correct pricing ($2/$12)
- Gemini 2.5 Pro preset added with correct pricing ($1.25/$10)
- Gemini 2.5 Flash preset added with correct pricing ($0.30/$2.50)
- Claude Opus 4.1 preset added with correct pricing ($15/$75)
- Claude Sonnet 4.5 preset added with correct pricing ($3/$15)
- Claude Haiku 4.5 preset added with correct pricing ($1/$5)
- GPT-5.1 Mini preset added with correct pricing ($0.25/$2)
- All GPT-5 presets updated to GPT-5.1
- Each preset includes: id, label, description, model, effort (if applicable)

#### FR2: Reasoning Level Support
**Description**: Handle reasoning levels appropriately for each model family.

**Acceptance Criteria**:
- GPT-5.1 models show reasoning variants (minimal/low/medium/high)
- Gemini models show NO reasoning variants
- Claude models show NO reasoning variants
- Reasoning effort correctly passed to API for GPT-5.1
- Non-reasoning models ignore effort parameter

#### FR3: Model Selection UI
**Description**: Update modal to display all models with clear organization.

**Acceptance Criteria**:
- All ~18-20 presets visible in modal
- Keyboard navigation works (↑↓ + Enter)
- Current selection highlighted clearly
- Model switching works without errors
- Modal loads <100ms

#### FR4: Pricing Display
**Description**: Show pricing information for each model.

**Acceptance Criteria**:
- Each preset description includes pricing tier
- Format: "— tier ($input/$output per 1M tokens)"
- Pricing matches `ModelPricing::for_model()` exactly
- Cost information visible before selection

### Non-Functional Requirements

#### NFR1: Performance
- Model picker loads <100ms
- No lag during keyboard navigation
- Model switching completes <500ms

#### NFR2: Reliability
- 100% model switching success rate
- No crashes when selecting any model
- Graceful error handling if model unavailable

#### NFR3: Consistency
- Display names match pipeline configurator exactly
- Pricing matches cost tracker exactly
- Model IDs match spec-kit registry exactly

#### NFR4: Maintainability
- Single source of truth for model definitions
- Adding new models requires minimal changes
- No duplicate model metadata

---

## User Flows

### Flow 1: Quick Model Selection (Primary)

**Context**: User wants to switch to Gemini Flash for quick questions.

**Steps**:
1. User types `/model`
2. TUI shows modal with 18-20 models grouped by family
3. User navigates to "Gemini 2.5 Flash — cheap/fast ($0.30/$2.50)"
4. User presses Enter
5. TUI switches to Gemini Flash
6. Confirmation: "Switched to Gemini 2.5 Flash"

**Expected Outcome**: Model switched successfully, user can now chat with Gemini Flash at lower cost.

### Flow 2: Reasoning Level Selection

**Context**: User wants GPT-5.1 with high reasoning for complex task.

**Steps**:
1. User types `/model`
2. TUI shows modal
3. User navigates to "GPT-5.1 High — deep reasoning ($1.25/$10)"
4. User presses Enter
5. TUI switches to GPT-5.1 with high reasoning effort

**Expected Outcome**: Model switched with correct reasoning level applied.

### Flow 3: Cost-Aware Selection

**Context**: User wants most capable model within budget.

**Steps**:
1. User types `/model`
2. TUI shows modal with pricing for each model
3. User compares:
   - Gemini 3 Pro: $2/$12 (LMArena #1)
   - Claude Sonnet 4.5: $3/$15 (excellent for agents)
   - GPT-5.1 Medium: $1.25/$10 (balanced)
4. User selects based on cost/capability tradeoff

**Expected Outcome**: User makes informed decision based on visible pricing.

### Error Handling

**Error 1: Model Unavailable**
- **Condition**: Selected model fails API validation
- **Handling**: Show error "Model [name] is currently unavailable. Please try another model."
- **Recovery**: User can select different model

**Error 2: Invalid Reasoning Level**
- **Condition**: Reasoning level set for non-GPT model
- **Handling**: Ignore reasoning parameter, log warning
- **Recovery**: Model switches successfully without reasoning

---

## Dependencies

### Internal Dependencies

#### 1. Spec-Kit Model Registry (SPEC-950 ✅ COMPLETE)
- **What**: Master registry of all 13 models with metadata
- **Function**: `get_all_available_models()`
- **Status**: Complete, validated
- **Impact**: Source of truth for model definitions

#### 2. Cost Tracker (SPEC-950 ✅ COMPLETE)
- **What**: Pricing information for all models
- **Function**: `ModelPricing::for_model(model)`
- **Status**: Complete with all pricing
- **Impact**: Provides accurate cost information

#### 3. Display Name Functions (SPEC-950 ✅ COMPLETE)
- **What**: Consistent model name formatting
- **Functions**: `get_model_display_name()`, `get_model_tier_public()`
- **Status**: Complete
- **Impact**: Ensures naming consistency

#### 4. Model Selection UI (Exists)
- **What**: Bottom pane modal for model selection
- **Location**: `tui/src/bottom_pane/`
- **Status**: Exists, needs minor updates
- **Impact**: Renders model picker

### External Dependencies

None - all dependencies are internal and already satisfied.

### Approval Requirements

- PRD acceptance by code owner
- Architecture review (minor change, low risk)
- Standard code review process

---

## Risks & Mitigation

### Risk 1: UI Crowding (MEDIUM)
**Description**: 18-20 presets might overwhelm the modal UI.

**Impact**: HIGH - Poor UX if users can't find desired model.

**Mitigation Strategy**:
- Group by family (Gemini / Claude / GPT-5.1)
- Use clear headers and spacing
- Consider search/filter in future (Phase 2)

**Owner**: UI implementation team

### Risk 2: Reasoning Level Complexity (LOW)
**Description**: GPT-5.1 × 4 reasoning levels = many preset variants.

**Impact**: MEDIUM - More items in list, but organized clearly.

**Mitigation Strategy**:
- Group reasoning variants together
- Clear labeling (minimal/low/medium/high)
- Default to "medium" for simplicity

**Owner**: Registry implementation

### Risk 3: Registry Sync Drift (MEDIUM)
**Description**: Model definitions could diverge between `/model` and pipeline configurator.

**Impact**: HIGH - Inconsistent user experience.

**Mitigation Strategy**:
- Import from single source of truth (spec-kit registry)
- Automated tests to verify consistency
- Regular audits during model additions

**Owner**: Implementation team

### Risk 4: Model API Changes (LOW)
**Description**: Model providers could deprecate or change models.

**Impact**: MEDIUM - Models become unavailable.

**Mitigation Strategy**:
- Graceful error handling
- Regular monitoring of model availability
- Update registry when changes occur

**Owner**: Maintenance team

---

## Success Criteria

### Launch Criteria

1. **Functionality Complete**:
   - ✅ All 13 models selectable via `/model`
   - ✅ Gemini 2.5 Flash, 2.5 Pro, 3 Pro available
   - ✅ Claude Haiku 4.5, Sonnet 4.5, Opus 4.1 available
   - ✅ GPT-5.1 Mini, Standard, Codex available
   - ✅ Reasoning levels work for GPT-5.1

2. **Quality Gates**:
   - ✅ Display names match pipeline configurator
   - ✅ Pricing information visible and accurate
   - ✅ Model switching works (no errors)
   - ✅ 100% keyboard navigation support
   - ✅ Performance <100ms modal load

3. **Testing Complete**:
   - ✅ All models tested (switching works)
   - ✅ Reasoning levels validated (GPT-5.1)
   - ✅ Error handling verified
   - ✅ Integration tests pass

4. **Documentation**:
   - ✅ `/model` command docs updated
   - ✅ Model selection guide created
   - ✅ Changelog entry added

### Key Performance Indicators (KPIs)

#### KPI 1: Model Adoption Rate
- **Metric**: % of users using non-GPT-5 models
- **Target**: ≥30% within first week
- **Measurement**: Telemetry tracking model selection

#### KPI 2: User Satisfaction
- **Metric**: Positive feedback on model selection UX
- **Target**: ≥4/5 satisfaction score
- **Measurement**: User surveys, feedback comments

#### KPI 3: Cost Optimization
- **Metric**: Average cost per chat session
- **Target**: 15-20% reduction (users selecting cheaper models)
- **Measurement**: Cost tracker analytics

---

## Testing Requirements

### Unit Tests

1. **Model Registry Tests**:
   - All 13 models present in registry
   - Pricing information correct for each model
   - Display names formatted correctly
   - Reasoning support flags accurate

2. **Preset Generation Tests**:
   - Gemini presets created correctly
   - Claude presets created correctly
   - GPT-5.1 presets created with reasoning variants
   - No duplicate IDs or names

### Integration Tests

1. **Model Switching Tests**:
   - Switch to Gemini 3 Pro → verify uses Gemini
   - Switch to Claude Opus → verify uses Claude
   - Switch to GPT-5.1 High → verify reasoning applied
   - Cost tracking reflects actual model used

2. **UI Tests**:
   - Modal displays all models
   - Keyboard navigation works for 18-20 items
   - Current selection highlighted
   - Pricing visible in descriptions

3. **Error Handling Tests**:
   - Invalid model selection → error shown
   - Model unavailable → fallback works
   - Reasoning on non-GPT → ignored gracefully

### Manual Testing

1. Run `/model` command → verify all 13+ models appear
2. Select each model → verify switching works
3. Test reasoning levels → verify applied correctly
4. Verify pricing info accurate
5. Test keyboard navigation
6. Verify no crashes or errors

### Performance Tests

- Model picker loads <100ms (verified with timer)
- No lag during keyboard navigation
- Model switching <500ms

---

## Stakeholder Review

### Stakeholders
- Code owners (review PRD and architecture)
- TUI development team (implementation)
- Users (UX validation)

### Reviewers
- Architecture team (ensure consistency with spec-kit)
- QA team (test coverage validation)

### Review Process
- Standard code review process
- Architecture review (minor change, low risk)
- User testing (optional, can validate with early users)

### Audit Requirements (if applicable)
- No security audit needed (UI-only change)
- No compliance requirements

---

## Implementation Approach

### Recommended Strategy: Dynamic Registry Approach

#### Component 1: Shared Model Registry (2-3 hours)

Create `common/src/model_registry.rs`:

```rust
/// Master model registry (single source of truth)
pub struct ModelInfo {
    pub id: &'static str,
    pub display_name: &'static str,
    pub family: ModelFamily, // Gemini, Claude, GPT
    pub tier: Tier,          // Cheap, Medium, Premium
    pub supports_reasoning: bool,
    pub input_cost: f64,
    pub output_cost: f64,
}

pub enum ModelFamily {
    Gemini,
    Claude,
    GPT,
}

pub enum Tier {
    Cheap,    // $0-1 input
    Medium,   // $1-3 input
    Premium,  // $3-15 input
    Ultra,    // $15+ input
}

pub fn get_all_models() -> Vec<ModelInfo> {
    vec![
        // Gemini family (3 models)
        ModelInfo {
            id: "gemini-3-pro",
            display_name: "Gemini 3 Pro (LMArena #1)",
            family: ModelFamily::Gemini,
            tier: Tier::Medium,
            supports_reasoning: false,
            input_cost: 2.0,
            output_cost: 12.0,
        },
        // ... all 13 models
    ]
}
```

#### Component 2: Update model_presets.rs (1-2 hours)

Refactor to use shared registry:

```rust
pub fn builtin_model_presets(auth_mode: Option<AuthMode>) -> Vec<ModelPreset> {
    let registry = model_registry::get_all_models();

    let mut presets = Vec::new();

    for model_info in registry {
        // Base preset (no reasoning)
        presets.push(ModelPreset {
            id: model_info.id,
            label: model_info.display_name,
            description: format!("{:?} tier (${:.2}/${:.2})",
                model_info.tier,
                model_info.input_cost,
                model_info.output_cost
            ),
            model: model_info.id,
            effort: None,
        });

        // Add reasoning variants if supported
        if model_info.supports_reasoning {
            for level in ["minimal", "low", "medium", "high"] {
                presets.push(ModelPreset {
                    id: format!("{}-{}", model_info.id, level),
                    label: format!("{} {}", model_info.display_name, level),
                    description: format!("— {} reasoning", level),
                    model: model_info.id,
                    effort: Some(reasoning_level_to_effort(level)),
                });
            }
        }
    }

    presets
}
```

#### Component 3: Update Modal UI (1-2 hours)

Add grouping and pricing display:
- Group presets by family
- Show headers: "--- Gemini Family ---"
- Show pricing in each item
- Highlight current selection

**Total Estimated**: 4-7 hours

### Alternative: Simple Expansion Approach

If dynamic registry too complex, simply expand PRESETS array (~150 LOC):

**Add to model_presets.rs**:
```rust
const PRESETS: &[ModelPreset] = &[
    // Gemini family (3 models)
    ModelPreset {
        id: "gemini-3-pro",
        label: "Gemini 3 Pro",
        description: "— #1 LMArena (1501 Elo), PhD-level reasoning ($2/$12)",
        model: "gemini-3-pro",
        effort: None,
    },
    // ... 12 more models
];
```

**Total Estimated**: 2-3 hours (simpler but less maintainable)

---

## Files to Modify

| File | Estimated LOC | Purpose |
|------|---------------|---------|
| common/src/model_presets.rs | ~150 | Add 10+ new presets, update existing |
| tui/src/chatwidget/mod.rs | ~20 | Update model command handling (if needed) |
| bottom_pane/model_selector_view.rs | ~50-100 | Add grouping, pricing display |
| docs/commands/model.md | ~50 | Documentation |

**Total**: ~270-320 LOC

---

## Timeline Estimates

### Phase 1: Registry Expansion (3-4 hours)
- Add Gemini presets (3 models)
- Add Claude presets (3 models)
- Add GPT-5.1 Mini preset
- Update GPT-5 → GPT-5.1
- Add pricing to descriptions

### Phase 2: UI Updates (2-3 hours)
- Add family grouping
- Update modal width for longer descriptions
- Add pricing display
- Test keyboard navigation

### Phase 3: Testing & Validation (1 hour)
- Test switching to each model
- Verify reasoning levels
- Verify pricing info
- Test edge cases

### Phase 4: Documentation (30 min)
- Update /model command docs
- Add model selection guide
- Update changelog

**Total Estimated**: 5-8 hours

---

## Design Decisions

### Decision 1: Grouping Strategy
**Options**:
- A) Flat list (all 18-20 presets in one list)
- B) Grouped by family (Gemini / Claude / GPT-5.1)

**Recommendation**: **B - Grouped** (better UX, scalable)

**Rationale**: As model catalog grows, grouping provides better organization and findability.

### Decision 2: Reasoning Presets
**Options**:
- A) Separate entries for each reasoning level
- B) Dynamic picker after model selection

**Recommendation**: **A - Separate entries** (simpler, matches current UX)

**Rationale**: Consistent with existing `/model` behavior, no UI complexity for dynamic picker.

### Decision 3: Model Aliases
**Options**:
- A) Show both "gemini" alias and "gemini-flash" explicit
- B) Show explicit names only

**Recommendation**: **B - Explicit only** (less confusion)

**Rationale**: Aliases useful for CLI, but clear explicit names better for UI selection.

### Decision 4: Pricing Display
**Options**:
- A) Show in description
- B) Separate column
- C) Tooltip on hover

**Recommendation**: **A - In description** (cleaner, existing UX pattern)

**Rationale**: Fits current modal format, no UI changes required.

### Decision 5: Registry Sync
**Options**:
- A) Duplicate model definitions (simple)
- B) Import from spec-kit registry (DRY)

**Recommendation**: **B - Import** (DRY principle, single source of truth)

**Rationale**: Prevents drift, easier maintenance, consistent with spec-kit architecture.

---

## Example Preset Definitions

### Gemini Family
```rust
ModelPreset {
    id: "gemini-3-pro",
    label: "Gemini 3 Pro",
    description: "— #1 LMArena (1501 Elo), PhD-level reasoning, best coding/math (premium: $2/$12)",
    model: "gemini-3-pro",
    effort: None,
},
ModelPreset {
    id: "gemini-2.5-pro",
    label: "Gemini 2.5 Pro",
    description: "— strong reasoning and multimodal, cost-effective premium (medium: $1.25/$10)",
    model: "gemini-2.5-pro",
    effort: None,
},
ModelPreset {
    id: "gemini-2.5-flash",
    label: "Gemini 2.5 Flash",
    description: "— fastest Google model, great for quick tasks and prototyping (cheap: $0.30/$2.50)",
    model: "gemini-2.5-flash",
    effort: None,
},
```

### Claude Family
```rust
ModelPreset {
    id: "claude-opus-4.1",
    label: "Claude Opus 4.1",
    description: "— most capable Claude, best for complex/creative tasks (ultra: $15/$75)",
    model: "claude-opus-4.1",
    effort: None,
},
ModelPreset {
    id: "claude-sonnet-4.5",
    label: "Claude Sonnet 4.5",
    description: "— balanced performance, excellent for agents and coding (medium: $3/$15)",
    model: "claude-sonnet-4.5",
    effort: None,
},
ModelPreset {
    id: "claude-haiku-4.5",
    label: "Claude Haiku 4.5",
    description: "— fast and efficient, good for simpler tasks (cheap: $1/$5)",
    model: "claude-haiku-4.5",
    effort: None,
},
```

### GPT-5.1 Family
```rust
ModelPreset {
    id: "gpt-5.1-mini",
    label: "GPT-5.1 Mini",
    description: "— cheapest OpenAI model, good for high-volume tasks (cheap: $0.25/$2)",
    model: "gpt-5.1-mini",
    effort: None,
},
// Then: gpt-5.1 × 4 reasoning levels
// Then: gpt-5.1-codex × 3 reasoning levels
```

---

## UI/UX Mockup

### Current Modal
```
┌─────────────────────────────────────────┐
│  Select Model                           │
├─────────────────────────────────────────┤
│  > gpt-5 medium                         │
│    gpt-5 low                            │
│    gpt-5 high                           │
│    gpt-5-codex medium                   │
│    gpt-5-codex high                     │
│                                         │
│  [↑↓ Navigate | Enter Select | Esc Cancel] │
└─────────────────────────────────────────┘
```

### Proposed Modal (Grouped)
```
┌──────────────────────────────────────────────────────────┐
│  Select Model (13 available)                             │
├──────────────────────────────────────────────────────────┤
│  Gemini Family (Google)                                  │
│  > Gemini 3 Pro (LMArena #1) — premium ($2/$12)          │
│    Gemini 2.5 Pro — balanced ($1.25/$10)                 │
│    Gemini 2.5 Flash — cheap/fast ($0.30/$2.50)           │
│                                                          │
│  Claude Family (Anthropic)                               │
│    Claude Opus 4.1 — ultra premium ($15/$75)             │
│    Claude Sonnet 4.5 — balanced ($3/$15)                 │
│    Claude Haiku 4.5 — cheap/fast ($1/$5)                 │
│                                                          │
│  GPT-5.1 Family (OpenAI) — with reasoning levels         │
│    GPT-5.1 Mini — cheapest ($0.25/$2)                    │
│    GPT-5.1 Minimal — fast reasoning ($1.25/$10)          │
│    GPT-5.1 Low — light reasoning ($1.25/$10)             │
│    GPT-5.1 Medium — balanced (default) ($1.25/$10)       │
│    GPT-5.1 High — deep reasoning ($1.25/$10)             │
│    GPT-5.1 Codex Low — code specialist ($1.25/$10)       │
│    GPT-5.1 Codex Medium — code specialist ($1.25/$10)    │
│    GPT-5.1 Codex High — code specialist ($1.25/$10)      │
│                                                          │
│  [↑↓ Navigate | Enter Select | Esc Cancel]               │
└──────────────────────────────────────────────────────────┘
```

---

## Scope Decisions

### In Scope
- ✅ Expand presets to all 13 models
- ✅ Update GPT-5 → GPT-5.1 in labels
- ✅ Add pricing/tier info
- ✅ Group by family (recommended)
- ✅ Reasoning levels for GPT-5.1

### Out of Scope (Future Enhancement)
- ❌ Search/filter functionality
- ❌ Model comparison view
- ❌ Performance benchmarks in modal
- ❌ Custom model configurations
- ❌ Multi-model selection

---

## References

- SPEC-950: Model registry validation (provides 13-model foundation) ✅ COMPLETE
- SPEC-947: Pipeline configurator (reference UI for model selection)
- common/src/model_presets.rs: Current preset definitions
- tui/src/chatwidget/spec_kit/cost_tracker.rs: Pricing data
- tui/src/chatwidget/spec_kit/stage_details.rs: Display name functions

---

## PRD Completeness Checklist

- ✅ Problem statement clear and specific
- ✅ User personas defined with current workflows
- ✅ Goals measurable with success criteria
- ✅ Requirements unambiguous and testable
- ✅ Dependencies identified (all satisfied)
- ✅ Risks documented with mitigation
- ✅ Timeline estimated (5-8 hours)
- ✅ Testing strategy comprehensive
- ✅ Implementation approach detailed
- ✅ Design decisions documented with rationale

---

## Open Questions

### Question 1: Should we implement search/filter in initial version?
**Context**: With 18-20 presets, search could improve UX.

**What Needs Clarification**:
- User demand for search (is typing 3-5 arrow keys too much?)
- Development effort vs value (adds 2-3 hours)
- Scope creep risk

**Priority**: LOW
**Blocker**: NO

**How to Resolve**: Start without search, add in Phase 2 if users request it.

### Question 2: Should reasoning levels be sub-items or separate entries?
**Context**: GPT-5.1 × 4 reasoning levels creates many entries.

**What Needs Clarification**:
- UI complexity of sub-menu picker
- User preference (flat list vs nested)

**Priority**: MEDIUM
**Blocker**: NO

**How to Resolve**: Implement as separate entries (simpler), can refactor later if needed.

---

## Multi-Agent Consensus & Quality

### Consensus Summary
**Status**: N/A - Manual PRD creation from comprehensive prompt

**Agreement**: This PRD is based on detailed specification prompt provided by user. All sections filled with complete requirements, clear acceptance criteria, and measurable success metrics.

**Disagreements & Resolution**: None - single source document

### Quality Assessment

**Clarity Score**: 9/10
- Problem clearly stated
- Requirements specific and testable
- Implementation approach detailed

**Completeness Score**: 10/10
- All PRD sections filled
- User personas defined
- Success criteria measurable
- Risks and mitigation documented

**Testability Score**: 9/10
- Clear acceptance criteria for each requirement
- Comprehensive testing strategy
- Performance benchmarks defined

**Feasibility Score**: 10/10
- All dependencies satisfied (SPEC-950 complete)
- Clear implementation path
- Reasonable timeline (5-8 hours)
- Low technical risk

### Ambiguity Flags

**Resolved**:
- Grouping strategy: Grouped by family (recommended)
- Reasoning presets: Separate entries (simpler)
- Registry sync: Import from spec-kit (DRY)
- Pricing display: In description (cleaner)

**Remaining** (low priority, non-blocking):
- Search/filter in initial version (defer to Phase 2)
- Reasoning sub-menu vs flat list (use flat, simpler)

---

## Major Decisions Made

1. **Registry Approach**: Import from spec-kit registry (single source of truth) over duplicate definitions
2. **Grouping**: Grouped by family (Gemini / Claude / GPT-5.1) over flat list
3. **Reasoning**: Separate presets for each level over dynamic picker
4. **Scope**: Core functionality first, defer search/filter to Phase 2
5. **Priority**: P2 - MEDIUM (UX enhancement, not blocking)
