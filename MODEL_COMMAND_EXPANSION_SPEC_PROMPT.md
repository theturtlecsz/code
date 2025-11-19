# /model Command Expansion SPEC - Creation Prompt

## Objective

Expand the `/model` command to support selecting from ALL 13 available models (Gemini, Claude, GPT-5.1 families), not just the current 7 GPT-5/GPT-5-codex presets.

---

## Background Context

**Current State** (2025-11-19):
- `/model` command only shows **7 GPT-5 presets** (all with reasoning levels)
- Implementation: `common/src/model_presets.rs` - hardcoded GPT-5 variants
- Defined presets:
  1. gpt-5-codex-low
  2. gpt-5-codex-medium
  3. gpt-5-codex-high
  4. gpt-5 minimal
  5. gpt-5 low
  6. gpt-5 medium (default)
  7. gpt-5 high

**Problem**:
- Users cannot select Gemini or Claude models via `/model` command
- Missing 6 model families: Gemini 2.5 Flash, 2.5 Pro, 3 Pro, Claude Haiku 4.5, Sonnet 4.5, Opus 4.1
- Spec-kit pipeline configurator has 13 models, but TUI `/model` has only 7
- Inconsistent: Pipeline uses all models, but user can't select them for chat

**Desired State**:
- `/model` command shows ALL 13 models from spec-kit registry
- Organized by tier (cheap → premium)
- With reasoning levels for GPT-5.1 models
- Clear display names (same as pipeline configurator)
- Consistent experience across TUI

---

## Current Implementation Analysis

### Files Involved

1. **common/src/model_presets.rs** (74 LOC):
   - Defines `ModelPreset` struct (id, label, description, model, effort)
   - `PRESETS` constant array (7 GPT-5 presets)
   - `builtin_model_presets(auth_mode)` function
   - **Issue**: Hardcoded to GPT-5 only

2. **tui/src/chatwidget/mod.rs** (line 9809-9845):
   - `handle_model_command()` - processes /model command
   - `available_model_presets()` - calls builtin_model_presets()
   - `apply_model_selection()` - switches active model
   - Shows bottom pane modal via `show_model_selection()`

3. **tui/src/bottom_pane/** (model selection modal):
   - Renders model picker UI
   - Shows preset list with descriptions
   - Keyboard navigation (↑↓ + Enter)

### Current UX Flow

```
User: /model
  ↓
TUI: Show modal with 7 GPT-5 presets
  ↓
User: Navigate (↑↓) and select (Enter)
  ↓
TUI: Apply model + reasoning effort
  ↓
Show: "Switched to gpt-5 medium"
```

---

## SPEC Requirements

### Phase 1: Model Registry Integration

**Task 1.1: Expand ModelPreset Array**
- Add Gemini family presets:
  ```rust
  ModelPreset {
      id: "gemini-3-pro",
      label: "Gemini 3 Pro",
      description: "— #1 LMArena (1501 Elo), best for complex reasoning, coding, math ($2/$12)",
      model: "gemini-3-pro",
      effort: None,
  },
  ModelPreset {
      id: "gemini-2.5-pro",
      label: "Gemini 2.5 Pro",
      description: "— strong reasoning and coding, cost-effective premium tier ($1.25/$10)",
      model: "gemini-2.5-pro",
      effort: None,
  },
  ModelPreset {
      id: "gemini-2.5-flash",
      label: "Gemini 2.5 Flash",
      description: "— fastest and cheapest, great for quick tasks and prototyping ($0.30/$2.50)",
      model: "gemini-2.5-flash",
      effort: None,
  },
  ```

- Add Claude family presets:
  ```rust
  ModelPreset {
      id: "claude-opus-4.1",
      label: "Claude Opus 4.1",
      description: "— most capable Claude, best for complex/creative tasks, premium pricing ($15/$75)",
      model: "claude-opus-4.1",
      effort: None,
  },
  ModelPreset {
      id: "claude-sonnet-4.5",
      label: "Claude Sonnet 4.5",
      description: "— balanced performance and cost, excellent for agents and coding ($3/$15)",
      model: "claude-sonnet-4.5",
      effort: None,
  },
  ModelPreset {
      id: "claude-haiku-4.5",
      label: "Claude Haiku 4.5",
      description: "— fast and cost-efficient, good for simpler tasks ($1/$5)",
      model: "claude-haiku-4.5",
      effort: None,
  },
  ```

- Update GPT-5 presets to GPT-5.1:
  ```rust
  // Change all "gpt-5" to "gpt-5.1" or "gpt-5.1-codex"
  // Update descriptions to mention GPT-5.1 (Nov 2025)
  ```

**Task 1.2: Add GPT-5.1 Mini Preset**
- Currently missing from presets but available in spec-kit
- Add: gpt-5.1-mini (cheapest GPT-5 option)

**Deliverables**:
- `PRESETS` array: 7 → ~18-20 presets (13 models × reasoning variants)
- All families represented
- Clear descriptions with pricing hints
- Consistent naming with spec-kit registry

### Phase 2: Reasoning Level Handling

**Task 2.1: Reasoning Support Matrix**
- GPT-5.1: ✅ Supports reasoning (minimal/low/medium/high)
- Gemini: ❌ No reasoning levels (yet - Deep Think is separate product)
- Claude: ❌ No public reasoning levels

**Task 2.2: Preset Organization**
Option A: **Flat List** (Simple, ~13-18 presets)
```
Gemini 3 Pro
Gemini 2.5 Pro
Gemini 2.5 Flash
Claude Opus 4.1
Claude Sonnet 4.5
Claude Haiku 4.5
GPT-5.1 Codex Low
GPT-5.1 Codex Medium
GPT-5.1 Codex High
GPT-5.1 Minimal
GPT-5.1 Low
GPT-5.1 Medium
GPT-5.1 High
GPT-5.1 Mini
```

Option B: **Grouped by Family** (Clearer, ~20 presets)
```
--- Gemini Family ---
Gemini 3 Pro (LMArena #1)
Gemini 2.5 Pro
Gemini 2.5 Flash

--- Claude Family ---
Claude Opus 4.1 (premium)
Claude Sonnet 4.5 (balanced)
Claude Haiku 4.5 (fast/cheap)

--- GPT-5.1 Family ---
GPT-5.1 Codex (low/medium/high)
GPT-5.1 (minimal/low/medium/high)
GPT-5.1 Mini
```

**Recommendation**: Option B (grouped) for better organization as list grows

### Phase 3: UI/UX Enhancements

**Task 3.1: Model Selection Modal**
- Current: Simple list with ↑↓ navigation
- Desired:
  - Group headers (if Option B)
  - Show pricing tier in each item
  - Current selection highlighted
  - Cost estimate per model

**Task 3.2: Model Descriptions**
- Current: Brief one-liner (e.g., "— fastest responses with limited reasoning")
- Desired:
  - Include use case hints
  - Show pricing tier (cheap/medium/premium)
  - Highlight special features (LMArena #1, code specialist, etc.)

**Task 3.3: Search/Filter (Optional)**
- Type-ahead search by model name
- Filter by tier (cheap/medium/premium)
- Filter by capability (reasoning/coding/creative)

### Phase 4: Configuration Integration

**Task 4.1: Sync with Spec-Kit Registry**
- Import from `tui/src/chatwidget/spec_kit/pipeline_configurator.rs::get_all_available_models()`
- Or: Create shared registry module
- Ensure consistency: Pipeline models = /model command models

**Task 4.2: Pricing Display**
- Show estimated cost per model
- Use `cost_tracker.rs::ModelPricing::for_model()`
- Format: "Gemini 2.5 Flash ($0.30/$2.50 per 1M tokens)"

**Task 4.3: Reasoning Level Selector**
- For GPT-5.1 models only (currently)
- Show reasoning picker after model selection (if applicable)
- Or: Show all preset combinations (GPT-5.1 × 4 reasoning levels = 4 presets)

---

## Implementation Approach

### Recommended Strategy

**Phase 1** (Core Expansion): Add all models to PRESETS array
- Estimated: 2-3 hours
- LOC: ~150 (15 new presets × 10 LOC each)
- Files: model_presets.rs

**Phase 2** (UI Polish): Group by family, add pricing
- Estimated: 2-3 hours
- LOC: ~100 (group headers, pricing display)
- Files: bottom_pane/model_selector_view.rs (or similar)

**Phase 3** (Sync): Create shared registry or import spec-kit models
- Estimated: 1-2 hours
- LOC: ~50 (refactor to shared module)
- Files: New shared module or import path

**Total Estimated**: 5-8 hours

### Alternative: Dynamic Registry (Recommended!)

Instead of hardcoding presets, **import from spec-kit registry**:

```rust
// In model_presets.rs
pub fn builtin_model_presets(auth_mode: Option<AuthMode>) -> Vec<ModelPreset> {
    // Import from spec-kit registry
    use crate::spec_kit::pipeline_configurator;

    let models = pipeline_configurator::get_all_available_models();

    models.iter().map(|model| {
        let display_name = get_model_display_name(model);
        let tier = get_model_tier_public(model);
        let pricing = ModelPricing::for_model(model);

        ModelPreset {
            id: model,
            label: display_name,
            description: format!("{} (${:.2}/${:.2})", tier, pricing.input_per_million, pricing.output_per_million),
            model: model,
            effort: None,
        }
    }).collect()
}
```

**Benefits**:
- ✅ Single source of truth (spec-kit registry)
- ✅ Automatic sync (add model once, available everywhere)
- ✅ No duplication
- ✅ Consistent naming and pricing

**Estimated**: 3-4 hours (simpler than manual expansion)

---

## Success Criteria

- [ ] /model command shows ALL 13 models (not just GPT-5)
- [ ] Gemini 2.5 Flash, 2.5 Pro, 3 Pro selectable
- [ ] Claude Haiku 4.5, Sonnet 4.5, Opus 4.1 selectable
- [ ] GPT-5.1 Mini, Standard, Codex selectable (with reasoning levels)
- [ ] Display names match pipeline configurator (consistency)
- [ ] Pricing information visible (tier or actual cost)
- [ ] Reasoning levels work for GPT-5.1 models
- [ ] Model switching works (no errors)
- [ ] User can easily find and select any model

---

## Research Questions

**Q1: Should reasoning levels be separate presets or dynamic?**
- Current: Separate presets (gpt-5 low, gpt-5 medium, gpt-5 high)
- Alternative: Single preset + reasoning picker (like pipeline configurator)
- Recommendation: Keep separate presets for simplicity (each is a different experience)

**Q2: How to handle model aliases?**
- "gemini" (alias) vs "gemini-flash" (explicit) - show both or just one?
- "claude" (alias) vs "claude-haiku" (explicit) - show both or just one?
- "code" (TUI default = GPT-5.1) - keep as quick shortcut?
- Recommendation: Show explicit names only, keep aliases for CLI shorthand

**Q3: Should modal be grouped or flat?**
- Flat: All 18-20 presets in one list (simple but long)
- Grouped: By family (Gemini / Claude / GPT-5.1) with headers
- Recommendation: Grouped for clarity (especially as more models added)

**Q4: Should pricing be shown?**
- Yes: Helps users make cost-informed decisions
- Format: "$0.30/$2.50" or "cheap tier" or both?
- Recommendation: Show tier label + actual pricing in description

**Q5: Integration with spec-kit registry?**
- Option A: Duplicate model definitions (simple but maintenance overhead)
- Option B: Import from spec-kit registry (DRY, single source of truth)
- Recommendation: **Option B** - dynamic import from `get_all_available_models()`

---

## Proposed Implementation Plan

### Recommended: Dynamic Registry Approach

**Component 1: Shared Model Registry** (2-3 hours)

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
        // Gemini family
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

**Component 2: Update model_presets.rs** (1-2 hours)

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

**Component 3: Update Modal UI** (1-2 hours)

Add grouping and pricing display:
```rust
// In bottom_pane/model_selector_view.rs
// Group presets by family
// Show headers: "--- Gemini Family ---"
// Show pricing in each item
// Highlight current selection
```

**Total Estimated**: 4-7 hours

---

## Alternative: Simple Expansion Approach

**If dynamic registry too complex**, simply expand PRESETS array:

**Add to model_presets.rs** (~150 LOC):
```rust
const PRESETS: &[ModelPreset] = &[
    // Gemini family (3 models)
    ModelPreset {
        id: "gemini-3-pro",
        label: "Gemini 3 Pro",
        description: "— #1 LMArena (1501 Elo), PhD-level reasoning, top coding ($2/$12)",
        model: "gemini-3-pro",
        effort: None,
    },
    // ... 2 more Gemini models

    // Claude family (3 models)
    ModelPreset {
        id: "claude-opus-4.1",
        label: "Claude Opus 4.1",
        description: "— most capable Claude, complex/creative tasks ($15/$75)",
        model: "claude-opus-4.1",
        effort: None,
    },
    // ... 2 more Claude models

    // GPT-5.1 family (4 base + reasoning variants)
    // Update all "gpt-5" → "gpt-5.1"
    // ... existing 7 presets updated
];
```

**Total Estimated**: 2-3 hours (simpler but less maintainable)

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

## Testing Requirements

**Manual Testing**:
1. Run `/model` command
2. Verify all 13+ models appear
3. Select each model and verify it switches
4. Test reasoning levels for GPT-5.1
5. Verify pricing info accurate
6. Test keyboard navigation

**Integration Testing**:
1. Switch to Gemini 3 Pro via `/model`
2. Send message, verify uses Gemini
3. Switch to Claude Opus via `/model`
4. Send message, verify uses Claude
5. Cost tracking reflects actual model used

---

## Dependencies

- Spec-kit model registry (SPEC-950 ✅ COMPLETE)
- Cost tracker with all pricing (SPEC-950 ✅ COMPLETE)
- Display name functions (SPEC-950 ✅ COMPLETE)
- Model selection UI infrastructure (exists)

**No blockers** - all dependencies satisfied!

---

## Risk Assessment

**LOW RISK**: Well-scoped enhancement to existing command
- No architectural changes
- Existing modal UI reusable
- Clear requirements
- All model data already exists

**Potential Issues**:
- UI might get crowded with 18-20 presets (mitigate with grouping)
- Reasoning level explosion (GPT-5.1 × 4 levels = many variants)
- Need to handle model switching errors gracefully

**Mitigation**:
- Group by family (cleaner UI)
- Show reasoning as sub-items or separate picker
- Validate model availability before switching

---

## Success Metrics

**Functional**:
- ✅ All 13 models selectable via `/model`
- ✅ Model switching works for all families
- ✅ Reasoning levels work for GPT-5.1
- ✅ No errors when switching models

**User Experience**:
- ✅ Clear which model is selected
- ✅ Easy to find desired model (grouping helps)
- ✅ Pricing visible (cost-informed decisions)
- ✅ Consistent with pipeline configurator

**Maintenance**:
- ✅ Adding new models is easy (1 place to update)
- ✅ Pricing updates propagate automatically
- ✅ No duplicate definitions

---

## Scope Decisions

### In Scope
- ✅ Expand presets to all 13 models
- ✅ Update GPT-5 → GPT-5.1 in labels
- ✅ Add pricing/tier info
- ✅ Group by family (optional but recommended)
- ✅ Reasoning levels for GPT-5.1

### Out of Scope (Future Enhancement)
- ❌ Search/filter functionality
- ❌ Model comparison view
- ❌ Performance benchmarks in modal
- ❌ Custom model configurations
- ❌ Multi-model selection (conversation uses 1 model at a time)

---

## Implementation Checklist

### Phase 1: Registry Expansion (3-4 hours)
- [ ] Add Gemini 3 Pro preset
- [ ] Add Gemini 2.5 Pro preset
- [ ] Add Gemini 2.5 Flash preset
- [ ] Add Claude Opus 4.1 preset
- [ ] Add Claude Sonnet 4.5 preset
- [ ] Add Claude Haiku 4.5 preset
- [ ] Add GPT-5.1 Mini preset
- [ ] Update all "gpt-5" → "gpt-5.1" in presets
- [ ] Add pricing to descriptions
- [ ] Add use case hints to descriptions

### Phase 2: UI Updates (2-3 hours)
- [ ] Add family grouping (if chosen)
- [ ] Update modal width for longer descriptions
- [ ] Add pricing display
- [ ] Test keyboard navigation with longer list
- [ ] Add current model indicator

### Phase 3: Testing & Validation (1 hour)
- [ ] Test switching to each model
- [ ] Verify reasoning levels for GPT-5.1
- [ ] Verify pricing info accurate
- [ ] Test edge cases (invalid models, errors)
- [ ] Manual TUI testing

### Phase 4: Documentation (30 min)
- [ ] Update /model command docs
- [ ] Add model selection guide
- [ ] Document model descriptions
- [ ] Update changelog

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

## Priority & Urgency

**Priority**: MEDIUM
- Not blocking other work
- Quality-of-life improvement
- Aligns with SPEC-950 model expansion

**Urgency**: LOW
- Current workaround exists (select model in pipeline configurator, or edit config)
- But user experience would significantly improve

**Recommendation**: Schedule after SPEC-950 validation, before next major feature

---

## References

- SPEC-950: Model registry validation (provides 13-model foundation)
- SPEC-947: Pipeline configurator (reference UI for model selection)
- common/src/model_presets.rs: Current preset definitions
- tui/src/chatwidget/spec_kit/cost_tracker.rs: Pricing data
- tui/src/chatwidget/spec_kit/stage_details.rs: Display name functions

---

## Recommended SPEC Creation Command

**Title**: "/model Command: Expand to All 13 Supported Models (Gemini, Claude, GPT-5.1)"

**Description**:
"Expand the /model command model selector from 7 GPT-5 presets to all 13 available models across Gemini, Claude, and GPT-5.1 families. Include pricing information, group by family, and sync with spec-kit model registry for consistency."

**Estimated Duration**: 5-8 hours
**Priority**: P2 - MEDIUM (UX enhancement)
**Dependencies**: SPEC-950 ✅ COMPLETE

---

## Key Design Decisions Needed

1. **Grouping**: Flat list vs grouped by family?
   - Recommendation: **Grouped** (better UX, scalable)

2. **Reasoning Presets**: Separate entries vs dynamic picker?
   - Recommendation: **Separate entries** (simpler, matches current UX)

3. **Aliases**: Show "gemini" alias or only "gemini-flash" explicit?
   - Recommendation: **Explicit only** (less confusion)

4. **Pricing Display**: Show in description or separate column?
   - Recommendation: **In description** (cleaner, existing UX)

5. **Registry Sync**: Import from spec-kit or duplicate?
   - Recommendation: **Import** (DRY principle, single source of truth)

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

### GPT-5.1 Family (with reasoning)
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

## Next Steps

1. Create SPEC using this prompt
2. Decide on grouping strategy (grouped recommended)
3. Decide on registry sync approach (import recommended)
4. Implement in phases (registry → UI → testing)
5. Validate with manual TUI testing

---

**Use this prompt to create**:

```
/speckit.new /model command expansion: Support all 13 models (Gemini, Claude, GPT-5.1 families)
```

Then attach this full prompt as context for the PRD creation.
