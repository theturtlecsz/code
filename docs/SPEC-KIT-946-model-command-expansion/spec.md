**SPEC-ID**: SPEC-KIT-946
**Feature**: /model Command Expansion to All 13 Supported Models
**Status**: Backlog
**Created**: 2025-11-19
**Branch**: TBD
**Owner**: Code

**Context**: The `/model` command currently only shows 7 GPT-5 presets, limiting users to expensive OpenAI models. Users cannot select Gemini (Flash/2.5 Pro/3 Pro) or Claude (Haiku/Sonnet/Opus) models despite these being available in the spec-kit pipeline configurator. This creates an inconsistent UX where pipeline automation has access to all 13 models, but chat sessions are restricted to GPT-5 variants only.

---

## Requirements

### Functional Requirements

**FR1**: Model Registry Expansion
- Add Gemini 2.5 Flash, 2.5 Pro, 3 Pro presets with pricing ($0.30-$12 per 1M tokens)
- Add Claude Haiku 4.5, Sonnet 4.5, Opus 4.1 presets with pricing ($1-$75 per 1M tokens)
- Add GPT-5.1 Mini preset (cheapest OpenAI option at $0.25/$2)
- Update all existing "gpt-5" references to "gpt-5.1" (accurate model versioning)
- Total: Expand from 7 to ~18-20 presets (13 models × reasoning variants)

**FR2**: Reasoning Level Support
- GPT-5.1 models: Support reasoning levels (minimal/low/medium/high)
- Gemini models: NO reasoning levels (Deep Think is separate product)
- Claude models: NO reasoning levels (not publicly available)
- Reasoning effort correctly passed to API for GPT-5.1 only

**FR3**: Model Selection UI Enhancement
- Display all ~18-20 presets in modal with keyboard navigation (↑↓ + Enter)
- Group by family (Gemini / Claude / GPT-5.1) with clear headers
- Show current selection highlighting
- Load modal <100ms (performance requirement)

**FR4**: Pricing Transparency
- Each preset description includes pricing tier and actual cost
- Format: "— tier ($input/$output per 1M tokens)"
- Pricing matches `ModelPricing::for_model()` exactly
- Enables cost-informed decisions during model selection

### Non-Functional Requirements

**Performance**:
- Model picker loads <100ms
- Model switching completes <500ms
- No lag during keyboard navigation

**Reliability**:
- 100% model switching success rate
- Graceful error handling if model unavailable
- No crashes when selecting any model

**Consistency**:
- Display names match pipeline configurator exactly (single source of truth)
- Pricing matches cost tracker exactly
- Model IDs match spec-kit registry exactly

**Maintainability**:
- Import model definitions from spec-kit registry (DRY principle)
- Adding new models requires minimal changes (single location update)
- No duplicate model metadata across codebase

---

## Success Criteria

- ✅ All 13 models selectable via `/model` command (Gemini, Claude, GPT-5.1)
- ✅ Display names match pipeline configurator (consistency)
- ✅ Pricing information visible for each model (transparency)
- ✅ Reasoning levels work for GPT-5.1 models only
- ✅ Model switching works without errors (reliability)
- ✅ Modal loads <100ms, switching <500ms (performance)
- ✅ Grouped by family for clear organization (UX quality)

---

## User Stories

### P1: As a developer, I want to access cost-effective Gemini Flash for quick queries
**Why P1**: Immediate cost savings (12x cheaper than GPT-5 at $0.30 vs $3.75 input)
**Verification**: Select "Gemini 2.5 Flash" from `/model` → verify chat uses Gemini → verify cost tracking shows lower cost

**Scenario 1 (Happy Path)**:
- **Given**: User types `/model` command
- **When**: User navigates to "Gemini 2.5 Flash — cheap/fast ($0.30/$2.50)"
- **Then**: Model switches successfully, confirmation shows "Switched to Gemini 2.5 Flash"

**Scenario 2 (Error Handling)**:
- **Given**: Gemini model temporarily unavailable
- **When**: User selects Gemini Flash
- **Then**: Error message: "Model Gemini 2.5 Flash is currently unavailable. Please try another model."

### P1: As a power user, I want to select specialized models for specific tasks
**Why P1**: Task-specific optimization (Claude Opus for creative tasks, Gemini 3 Pro for coding)
**Verification**: Switch to Claude Opus → verify creative responses | Switch to Gemini 3 Pro → verify coding assistance

**Scenario**: Cost-Aware Model Selection
- **Given**: User wants best model within budget constraint
- **When**: User opens `/model` modal and compares pricing
- **Then**: User sees: Gemini 3 Pro ($2/$12), Claude Sonnet ($3/$15), GPT-5.1 Medium ($1.25/$10)
- **Outcome**: User makes informed decision based on cost/capability tradeoff

### P2: As a user, I want clear organization of 18-20 model presets
**Why P2**: Findability important but not blocking (still usable with flat list)
**Verification**: Modal shows family groups (Gemini / Claude / GPT-5.1) with clear headers

### P3: As a developer, I want consistent model selection across chat and pipeline
**Why P3**: Nice to have, improves UX consistency
**Verification**: Compare display names between `/model` modal and pipeline configurator → verify identical

---

## Edge Cases & Boundary Conditions

**Boundary 1**: Empty Model Selection
- **Condition**: User cancels modal without selecting (presses Esc)
- **Handling**: Retain current model, no changes applied

**Boundary 2**: Model API Unavailable
- **Condition**: Selected model fails API validation check
- **Handling**: Show error "Model [name] currently unavailable", revert to previous model

**Boundary 3**: Invalid Reasoning Level for Non-GPT Model
- **Condition**: Reasoning effort somehow set for Gemini/Claude
- **Handling**: Ignore reasoning parameter, log warning, model switches successfully

**Boundary 4**: Very Long Model Description
- **Condition**: Description exceeds modal width (>80 chars)
- **Handling**: Truncate or wrap text gracefully in UI

**Performance Edge Case**: 20+ Presets Rendering
- **Condition**: All reasoning variants shown simultaneously
- **Expected**: Modal still loads <100ms, navigation remains responsive

**Concurrent Access**: Model Switch During Active Request
- **Condition**: User switches model while message is generating
- **Handling**: Complete current request with old model, apply new model to next request

---

## Non-Functional Requirements

**NFR1**: Performance <100ms modal load
- **Metric**: Modal render time from `/model` command to display
- **Acceptance**: 95th percentile <100ms

**NFR2**: Reliability 100% switching success
- **Metric**: Model switching success rate
- **Acceptance**: Zero crashes, graceful error handling for API failures

**NFR3**: Security - Follow project security guidelines
- **Standard**: OWASP secure coding practices
- **Audit**: Standard code review process (no special security audit needed)

**NFR4**: Scale - Handle 20+ presets without performance degradation
- **Test**: Render modal with full preset list (18-20 items)
- **Metric**: No lag during keyboard navigation, instant selection

**NFR5**: Uptime - No service interruptions
- **Target**: 99.9% availability (inherits from TUI availability)
- **Monitoring**: Local testing (no production dashboard needed)

---

## Acceptance Criteria Summary

| Requirement | Verification Method | Acceptance Criteria |
|-------------|---------------------|---------------------|
| All 13 models selectable | Manual testing of `/model` command | Gemini (3), Claude (3), GPT-5.1 (7+) all present |
| Pricing visible | Visual inspection of modal | Each preset shows "($X/$Y)" format |
| Reasoning for GPT-5.1 | API call validation | reasoning_effort parameter passed for GPT-5.1 only |
| Modal performance | Timer measurement | <100ms load time, <500ms switching |
| Error handling | Simulate API failure | Graceful error messages, no crashes |
| Consistency | Side-by-side comparison | `/model` names match pipeline configurator names |

---

## Open Questions & Decisions

### Resolved Decisions

**Q1**: Should reasoning levels be separate presets or dynamic picker?
**A**: Separate presets (simpler, matches current UX pattern)

**Q2**: Should modal be grouped or flat?
**A**: Grouped by family (better UX, more scalable as catalog grows)

**Q3**: Should we import from spec-kit registry or duplicate?
**A**: Import from registry (DRY principle, single source of truth)

**Q4**: Should we include search/filter in initial version?
**A**: No, defer to Phase 2 (simplify initial scope)

### Remaining Questions (Non-Blocking)

None - all critical decisions resolved. Minor UX refinements can be addressed during implementation.

---

## Dependencies

### Prerequisites

- ✅ **SPEC-950**: Model registry validation (provides 13-model foundation) - COMPLETE
- ✅ **Cost Tracker**: Pricing for all models (`ModelPricing::for_model()`) - COMPLETE
- ✅ **Display Functions**: Consistent naming (`get_model_display_name()`) - COMPLETE
- ✅ **Model Selection UI**: Bottom pane modal infrastructure - EXISTS

**All dependencies satisfied** - no blockers to implementation.

### Upstream Dependencies

None - self-contained enhancement to existing `/model` command.

### Related SPECs

- SPEC-950: Model registry validation (foundation)
- SPEC-947: Pipeline configurator (reference UI)

---

## Changelog

**2025-11-19**: Initial spec created from comprehensive implementation prompt. All requirements defined, dependencies validated, implementation approach documented.

---

## Notes

**Implementation Approach**: Dynamic registry recommended over static presets.
- Import from `get_all_available_models()` in spec-kit registry
- Eliminates duplication, ensures consistency
- Estimated 4-7 hours vs 2-3 hours for static approach
- Long-term maintainability justifies slightly higher initial cost

**Scope Management**: Start with core functionality (all 13 models, pricing, grouping).
- Defer search/filter to Phase 2 (future enhancement)
- Focus on immediate value: model access and cost transparency
- Can iterate based on user feedback

**Risk Level**: LOW
- Well-scoped enhancement to existing command
- All dependencies satisfied
- Clear requirements with measurable acceptance criteria
- No architectural changes required

**Next Steps**:
1. Run `/speckit.clarify SPEC-KIT-946` to resolve any ambiguities
2. Run `/speckit.auto SPEC-KIT-946` to generate full implementation
3. OR manually implement following PRD.md detailed approach
