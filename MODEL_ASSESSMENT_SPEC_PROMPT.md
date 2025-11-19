# Model Assessment & Validation SPEC - Creation Prompt

## Objective

Create comprehensive SPEC for validating and updating the spec-kit model registry with accurate, available, and cost-effective models for multi-agent consensus workflows.

## Background Context

**Current State** (2025-11-18):
- Model registry in `cost_tracker.rs` and `pipeline_configurator.rs` contains outdated/invalid models
- GPT-4, GPT-5 models included but OpenAI API keys NOT used in this project
- Gemini 3 released TODAY (2025-11-18) but not in registry
- No validation that listed models are actually available through our MCP integrations
- Pricing data potentially stale (last updated 2025-10-24)

**Critical Issues**:
1. **Invalid Models**: GPT-4, GPT-4-turbo, GPT-5 not available (no OpenAI keys)
2. **Missing Models**: Gemini 3 Pro, Gemini 3 Deep Think (released today)
3. **Unvalidated Availability**: No check that models work with our MCP setup
4. **Stale Pricing**: Need current rates for cost estimation accuracy
5. **Consensus Architecture**: Unclear which model acts as aggregator/orchestrator

## SPEC Requirements

### Phase 1: Discovery & Validation

**Task 1.1: Model Availability Audit**
- Query MCP servers for available models (if supported)
- Test model access through existing integrations
- Document which models are actually callable
- Identify models to REMOVE (GPT-4, GPT-5, etc.)

**Task 1.2: Current Model Research** (Web Search)
- Gemini 3 variants (Pro, Deep Think, Flash 3.0?)
- Latest Claude models (Sonnet 4.5, Haiku 4.0?)
- Latest Gemini 2.x variants still available
- Pricing updates for all models
- Performance benchmarks (LMArena leaderboard as of Nov 2025)

**Task 1.3: Consensus Architecture Documentation**
- Identify which model serves as aggregator/orchestrator
- Document: Do N agents → 1 orchestrator synthesize → final output?
- Or: Do N agents vote programmatically without orchestrator model?
- Find code evidence in `consensus.rs`, `agent_orchestrator.rs`
- Clarify in UI: Show orchestrator as separate role

### Phase 2: Registry Cleanup

**Task 2.1: Remove Invalid Models**
- Delete all GPT-4, GPT-5 references from:
  - `cost_tracker.rs` (ModelPricing::for_model)
  - `pipeline_configurator.rs` (get_all_available_models)
  - `stage_details.rs` (get_model_tier_public)
- Verify no broken references remain

**Task 2.2: Add New Models**
- Gemini 3 Pro (pricing, tier classification)
- Gemini 3 Deep Think (if available)
- Any new Claude 4.x variants
- Update tier classifications (cheap/medium/premium)

**Task 2.3: Update Pricing Data**
- Fetch current pricing from official pages:
  - https://ai.google.dev/pricing (Gemini)
  - https://www.anthropic.com/pricing (Claude)
- Update `cost_tracker.rs` with accurate rates
- Add data source comments and last-updated dates

### Phase 3: Architecture Clarification

**Task 3.1: Document Orchestrator Model**
- Find default orchestrator model in code
- Add to UI as separate role (e.g., "[Orchestrator] claude-sonnet - synthesis")
- Allow user to configure orchestrator separately from consensus agents
- Update role descriptions to clarify: "consensus agent 1 (input)" vs "orchestrator (synthesis)"

**Task 3.2: Update Model Role Descriptions**
- Clarify which models generate outputs vs which synthesize
- Add tooltips/help text explaining consensus workflow:
  ```
  Plan Stage (4 models total):
    [1] gemini-flash - consensus agent 1 (generates plan)
    [2] claude-haiku - consensus agent 2 (generates plan)
    [3] gpt5_1 - consensus agent 3 (generates plan)
    [Orchestrator] claude-sonnet - synthesizes consensus
  ```

### Phase 4: Validation & Testing

**Task 4.1: Integration Testing**
- Test each model in registry is callable
- Verify pricing calculations accurate
- Ensure orchestrator model configurable
- Validate TOML serialization with new models

**Task 4.2: Cost Impact Analysis**
- Calculate cost difference with Gemini 3 models
- Identify cost optimization opportunities
- Document recommended configurations by budget

**Task 4.3: Documentation Updates**
- Update `docs/spec-kit/model-strategy.md`
- Add model compatibility matrix
- Document orchestrator architecture
- Create model selection best practices guide

## Success Criteria

- [ ] All invalid models removed (GPT-4, GPT-5)
- [ ] Gemini 3 models added with accurate pricing
- [ ] All models validated as available through MCP
- [ ] Orchestrator model visible and configurable in UI
- [ ] Pricing data current (as of Nov 2025)
- [ ] Consensus architecture documented and clear
- [ ] Cost calculations accurate
- [ ] User can confidently select models knowing they work

## Research Questions to Answer

1. **Gemini 3 variants**: Pro, Deep Think, Flash 3.0 - which are available via API?
2. **Gemini 3 pricing**: What are the rates? Better than Gemini 2.5?
3. **Claude 4.x**: Any new models since Sonnet 4.5?
4. **Orchestrator default**: Which model currently synthesizes consensus?
5. **Model aliases**: Map user-friendly names (e.g., "gemini-3-flash") to API names
6. **Rate limits**: Any models with strict rate limits to warn about?
7. **Context windows**: Relevant for large specs - document per model
8. **Deprecations**: Any models being sunset?

## Estimated Scope

- **Duration**: 3-4 hours (research-heavy)
- **LOC**: ~200 (registry updates, UI enhancements, docs)
- **Testing**: Integration tests for model availability
- **Deliverables**:
  - Updated model registry (15-20 valid models)
  - Orchestrator UI and configuration
  - Current pricing data
  - Validation test suite
  - Architecture documentation

## Dependencies

- Web access for pricing/release research
- MCP servers operational for availability testing
- Access to model provider documentation

## Priority

**HIGH** - Invalid models in production code, missing new releases, unclear architecture

---

**Use this prompt to create**: `/speckit.new Model registry validation and Gemini 3 integration`
