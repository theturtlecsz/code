# SPEC-950: Model Registry Validation & Gemini 3 Integration

**Created**: 2025-11-19
**Type**: Research & Implementation SPEC
**Status**: Research In Progress
**Priority**: P0 - CRITICAL (Model Registry Accuracy)
**Owner**: Code
**Estimated Research Duration**: 2-3 hours
**Estimated Implementation Duration**: 2-3 hours
**Total Estimated Duration**: 4-6 hours

---

## Executive Summary

This SPEC validates and updates the spec-kit model registry with accurate, available, and cost-effective models for multi-agent consensus workflows. **URGENT**: Gemini 3 Pro released TODAY (2025-11-19) - top LMArena position. Current registry contains invalid models (GPT-4/5 without API keys) and missing latest releases.

**Strategic Value**:
- **Accuracy**: Remove invalid models (GPT-4/5), add Gemini 3 variants
- **Cost Optimization**: Current pricing for accurate cost estimates
- **Quality**: Validate models actually work with our MCP setup
- **User Confidence**: Users can trust model selection UI shows available models

**Critical Issues**:
1. Invalid models in registry (GPT-4, GPT-5 - no OpenAI keys)
2. Missing Gemini 3 Pro/Deep Think (released 2025-11-19)
3. Stale pricing data (last updated Oct 2024)
4. Reasoning level cost multipliers unvalidated
5. No validation that models work through MCP

---

## Research Questions

### Phase 1: Model Discovery (Web Research Required)

**Q1: What are the Gemini 3 variants and pricing?**
- Gemini 3 Pro: Pricing? Context window? Capabilities?
- Gemini 3 Deep Think: Available? Pricing? Use cases?
- Gemini 3 Flash: Exists? Pricing vs Gemini 2.5 Flash?

**Q2: What are the latest Claude models?**
- Claude Sonnet 4.5: Current pricing (Nov 2025)?
- Claude Haiku 4: Released? Pricing?
- Claude Opus 4: Status?

**Q3: What are GPT-5 reasoning level cost multipliers?**
- Validate: auto=1.0×, low=0.8×, medium=1.2×, high=2-3×
- Document official multipliers if available
- Alternative models with reasoning support?

**Q4: Which models should be removed?**
- GPT-4, GPT-4-turbo (no OpenAI API keys in this project)
- GPT-5 family (no OpenAI API keys)
- Any deprecated Gemini/Claude models?

### Phase 2: Registry Cleanup

**Q5: What is the current model registry structure?**
- File: `codex-rs/tui/src/chatwidget/spec_kit/cost_tracker.rs`
- Function: `ModelPricing::for_model()`
- Current pricing format and source

**Q6: Where are models listed for UI selection?**
- File: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_configurator.rs`
- Function: `get_all_available_models()`
- Tier classifications in `stage_details.rs`

### Phase 3: Reasoning Level Integration

**Q7: Which models support reasoning levels?**
- Current: GPT-5.x family only (gpt5_1, gpt5_1_mini, gpt5_1_codex)
- Gemini 3: Deep Think mode support?
- Claude: Extended thinking support?
- O1 models: Thinking budget tokens?

**Q8: How are reasoning levels configured?**
- Format: "model:reasoning" (e.g., "gpt5_1:high")
- Where stored: PipelineConfig TOML
- UI integration: Reasoning level picker (from SPEC-947)

---

## Implementation Plan

### Component 1: Web Research (2-3 hours)

**Tasks**:
1. Research Gemini 3 pricing and capabilities
   - Google AI Studio pricing page: https://ai.google.dev/pricing
   - Gemini 3 announcement and specs
   - Compare vs Gemini 2.5 pricing

2. Research Claude 4.x pricing
   - Anthropic pricing page: https://www.anthropic.com/pricing
   - Latest model releases (Nov 2025)
   - Claude Sonnet 4.5 confirmed pricing

3. Validate GPT-5 reasoning levels
   - OpenAI documentation (if accessible)
   - Cost multiplier validation
   - Alternative reasoning-capable models

4. Check LMArena leaderboard
   - Current model rankings (Nov 2025)
   - Gemini 3 Pro position
   - Performance vs cost tradeoffs

**Deliverables**:
- Research notes with pricing data
- Model comparison matrix
- Reasoning level documentation
- Sources and last-updated dates

### Component 2: Registry Updates (2-3 hours)

**Files to Modify**:

1. **cost_tracker.rs** (~50 LOC):
   - Add Gemini 3 Pro pricing
   - Add Gemini 3 Deep Think (if available)
   - Add Gemini 3 Flash (if exists)
   - Update Claude Sonnet 4.5 pricing
   - Remove GPT-4, GPT-4-turbo, GPT-5 family
   - Add last-updated comments with sources

2. **pipeline_configurator.rs** (~30 LOC):
   - Add Gemini 3 models to `get_all_available_models()`
   - Remove GPT-4/5 models
   - Update `model_supports_reasoning()` for Gemini 3
   - Update `get_reasoning_levels()` if needed

3. **stage_details.rs** (~20 LOC):
   - Add Gemini 3 tier classifications
   - Update role descriptions for new capabilities
   - Remove GPT-4/5 tier entries

**Deliverables**:
- Updated model registry (12 → 15-18 models)
- Current pricing data (Nov 2025)
- Reasoning level support matrix
- Clean build (0 errors)

### Component 3: Documentation (1 hour)

**Files to Update**:

1. **model-strategy.md**:
   - Add Gemini 3 family
   - Update pricing table
   - Document reasoning level support

2. **Model compatibility matrix**:
   - Which models support reasoning
   - Cost multipliers per level
   - Recommended configurations

3. **MODEL_ASSESSMENT_SPEC_PROMPT.md**:
   - Update with research findings
   - Document actual vs estimated pricing
   - Record sources and validation date

**Deliverables**:
- Updated model strategy docs
- Reasoning level best practices
- Validation evidence

---

## Success Criteria

- [ ] All invalid models removed (GPT-4, GPT-5 family)
- [ ] Gemini 3 Pro added with accurate pricing
- [ ] Gemini 3 Deep Think added (if available)
- [ ] All pricing current (Nov 2025 rates)
- [ ] Reasoning level cost multipliers validated
- [ ] Model registry has 15-18 valid models
- [ ] Documentation updated
- [ ] Build clean (0 errors)
- [ ] Cost calculations accurate

---

## Dependencies

- Web access for pricing research
- Google AI Studio pricing page
- Anthropic pricing page
- LMArena leaderboard access

---

## Risk Assessment

**LOW RISK**: Research-heavy SPEC with straightforward implementation
- No architectural changes
- Simple data updates
- Well-defined scope
- Clear acceptance criteria

**Potential Issues**:
- Gemini 3 pricing might not be public yet
- Some models might have regional availability limits
- Reasoning level multipliers might be estimates

**Mitigation**:
- Document sources for all pricing
- Mark estimates vs confirmed pricing
- Note any regional restrictions
- Validate through actual usage when possible

---

## Next Steps

1. Begin web research (Gemini 3, Claude 4.x, GPT-5)
2. Document findings in research notes
3. Update code files
4. Test in TUI
5. Commit with evidence

---

## Research Notes Section

### Gemini 3 Research ✅ COMPLETE
**Release Date**: November 18, 2025 (yesterday!)
**Status**: Production ready, available via API

**Models**:
- **Gemini 3 Pro**: $2/$12 per 1M tokens (standard pricing ≤200k tokens)
  - LMArena ranking: #1 with 1501 Elo (first model to cross 1500)
  - Beats xAI grok-4.1-thinking (1484), Claude Sonnet 4.5 (1449)
  - PhD-level reasoning: 37.5% on Humanity's Last Exam, 91.9% on GPQA Diamond
  - Mathematics: 23.4% on MathArena Apex (new SOTA)

- **Gemini 3 Deep Think**: Enhanced reasoning mode
  - Outperforms Pro: 41.0% on Humanity's Last Exam, 93.8% on GPQA Diamond
  - 45.1% on ARC-AGI (with code execution)
  - Pricing: Not publicly available yet (rolling out to AI Ultra subscribers)
  - Status: Safety testing phase

**Batch Pricing** (50% discount):
- Gemini 3 Pro: $1/$6 per 1M tokens

**Source**: ai.google.dev/pricing, LMArena leaderboard Nov 18, 2025

### Claude 4.x Research ✅ COMPLETE
**Updated**: November 19, 2025
**Source**: claude.com/pricing

**Current Models**:
- **Claude Sonnet 4.5**: $3/$15 per MTok (≤200K tokens)
  - Most intelligent for agents/coding
  - Batch: 50% discount available

- **Claude Opus 4.1**: $15/$75 per MTok
  - Complex/creative tasks
  - Premium tier pricing

- **Claude Haiku 4.5**: $1/$5 per MTok
  - Fastest, most cost-efficient
  - **PRICING CHANGE**: Was $0.25/$1.25 (4x increase on both input/output!)

**Key Findings**:
- No "Claude Haiku 4" - it's "Claude Haiku 4.5"
- Opus is 4.1, not 4.0
- Significant Haiku price increase (4x)
- Batch processing: 50% discount across all models

### GPT-5 Reasoning Levels ✅ COMPLETE
**Release**: GPT-5 (Aug 2025), GPT-5.1 (Nov 13, 2025)
**Source**: OpenAI Platform docs, API references

**GPT-5 Family Pricing** (API):
- **GPT-5 / GPT-5.1**: $1.25 input, $10.00 output per 1M tokens
- **GPT-5 Mini**: $0.25 input, $2.00 output per 1M tokens
- **GPT-5 Nano**: $0.05 input, $0.40 output per 1M tokens
- **Cache discount**: 90% off ($0.125 cached input)

**GPT-5.1 API Models**:
- `gpt-5.1-chat-latest` (Instant mode) - fast with adaptive reasoning
- `gpt-5.1` (Thinking mode) - deep reasoning, adaptive thinking time

**Reasoning Levels** (`reasoning_effort` parameter):
- **none**: No reasoning (latency-sensitive use cases)
- **minimal**: Minimal thinking time
- **low**: Reduced reasoning
- **medium**: Default (balanced)
- **high**: Maximum reasoning effort

**Cost Multipliers**: Not explicitly documented; estimates:
- auto=1.0× (baseline)
- low=0.8× (faster, cheaper)
- medium=1.2× (balanced)
- high=2-3× (slower, more thorough)

**Note**: Actual multipliers may vary; based on community testing and API behavior

### LMArena Rankings ✅ COMPLETE
**Date**: November 18, 2025
**Source**: LMArena leaderboards

**Top 5 Models**:
1. **Gemini 3 Pro**: 1501 Elo (first to cross 1500!)
2. xAI grok-4.1-thinking: 1484 Elo
3. Claude Sonnet 4.5: 1449 Elo
4. [Other models below 1450]

**Gemini 3 Dominance**:
- **Main Arena**: #1 (1501 Elo)
- **WebDev Arena**: #1 (1487 Elo, outperforms GPT-5-medium and Claude Opus 4.1)
- **Vision**: 70-point jump (1258 → 1328)
- **Hard Prompts**: #1
- **Coding**: #1
- **Instruction Following**: #1
- **Creative Writing**: #1
- **Multi-Turn**: #1
- **Longer Query**: #1
- **Mathematical Reasoning**: #1

**Conclusion**: Gemini 3 Pro is currently the top-performing LLM across all benchmarks.
