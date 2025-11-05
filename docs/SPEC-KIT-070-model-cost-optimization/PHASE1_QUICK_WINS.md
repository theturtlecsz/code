# SPEC-KIT-070 Phase 1: Immediate Quick Wins

**Target**: Reduce /speckit.auto from $11 → $5-6 (45-55% savings)
**Timeline**: Week 1 (5-8 hours)
**Risk**: LOW (simple model swaps, no architectural changes)

---

## Quick Win #1: Replace Gemini Pro → Flash (17x cheaper)

**Current State**:
- All Tier 2/3/4 commands use `gemini-pro-1.5` ($1.25/1M input, $5/1M output)
- 3-5 Gemini calls per /speckit.auto run
- Cost: ~$3-4 per auto run for Gemini alone

**Proposed Change**:
- Replace with `gemini-flash-2.0` ($0.10/1M input, $0.40/1M output)
- Same API, same prompts, 12x cheaper
- Flash 2.0 is latest, fast, supports structured output

**Implementation**:
```bash
# Update config.toml
sed -i 's/gemini-pro-1.5/gemini-flash-2.0/g' ~/.code/config.toml

# Or Flash 1.5 (even cheaper: $0.075/1M)
sed -i 's/gemini-pro-1.5/gemini-flash-1.5/g' ~/.code/config.toml
```

**Validation**:
- Run /speckit.auto SPEC-KIT-070 with Flash
- Compare consensus quality vs Pro
- Verify 604 tests still pass
- Measure cost difference

**Expected Savings**: $3-4 per run → $0.25-0.35 per run (**~$3 savings, 88% reduction**)

---

## Quick Win #2: Replace GPT-4 Turbo → GPT-4o (4x cheaper)

**Current State**:
- Aggregator uses `gpt-4-turbo` ($10/1M input, $30/1M output)
- 1 call per consensus stage (6 per auto run)
- Cost: ~$2-3 per auto run

**Proposed Change**:
- Replace with `gpt-4o` ($2.50/1M input, $10/1M output)
- Better performance, 4x cheaper
- Same context window (128k)

**Implementation**:
```bash
# Update config.toml
sed -i 's/gpt-4-turbo/gpt-4o/g' ~/.code/config.toml
sed -i 's/gpt_pro/gpt-4o/g' ~/.code/config.toml
```

**Expected Savings**: $2-3 per run → $0.50-0.75 per run (**~$2 savings, 75% reduction**)

---

## Quick Win #3: Replace Claude Sonnet → Haiku for Non-Critical Tasks

**Current State**:
- `claude-sonnet-4` used for all consensus ($3/1M input, $15/1M output)
- 3-4 Claude calls per auto run
- Cost: ~$2-3 per auto run

**Proposed Change** (Conservative):
- Keep Sonnet for: /implement (code gen), /unlock (critical)
- Use `claude-haiku-3.5` ($0.25/1M input, $1.25/1M output) for:
  - /clarify, /analyze, /checklist (structured analysis)
  - /plan, /tasks (decomposition)
  - /validate (test planning)

**Expected Savings**: $2-3 per run → $0.50-1.00 per run (**~$1.50 savings, 60% reduction**)

---

## Quick Win #4: Eliminate Consensus for Deterministic Tasks

**Current State**:
- /speckit.new uses 3 agents to generate SPEC-ID (find max, increment)
- Template filling uses multi-agent consensus
- Cost: $2.40 for simple increment operation!

**Proposed Change**:
- SPEC-ID generation: **Native Rust** in TUI ($0)
  ```rust
  fn generate_next_spec_id() -> String {
      let pattern = "SPEC-KIT-*";
      let files = glob::glob(pattern);
      let max = files.filter_map(|p| parse_spec_number(p)).max().unwrap_or(0);
      format!("SPEC-KIT-{:03}", max + 1)
  }
  ```
- Template operations: Single Haiku call ($0.02)

**Expected Savings**: $2.40 → $0 (native) or $0.02 (Haiku) (**~$2.38 savings, 99% reduction**)

---

## Phase 1 Combined Impact

| Operation | Current | Phase 1 | Savings | % Reduction |
|-----------|---------|---------|---------|-------------|
| Gemini calls | $3-4 | $0.25-0.35 | $3 | 88% |
| GPT aggregation | $2-3 | $0.50-0.75 | $2 | 75% |
| Claude consensus | $2-3 | $0.50-1.00 | $1.50 | 60% |
| Deterministic | $2.40 | $0-0.02 | $2.38 | 99% |
| **TOTAL /speckit.auto** | **$11** | **$4-6** | **$5-7** | **50-65%** |

**At 100 SPECs/month**:
- Current: $1,100/month
- Phase 1: $400-600/month
- **Savings: $500-700/month**

---

## Implementation Checklist - Week 1

### Day 1: Research & Validation (2 hours)
- [ ] Test Gemini Flash 2.0 with existing prompts
- [ ] Test Claude Haiku 3.5 with existing prompts
- [ ] Test GPT-4o with aggregation prompts
- [ ] Verify API availability and rate limits
- [ ] Document any prompt modifications needed

### Day 2: Config Updates (1 hour)
- [ ] Backup current config.toml
- [ ] Replace gemini-pro → gemini-flash-2.0
- [ ] Replace gpt-4-turbo → gpt-4o
- [ ] Add model cost metadata to config
- [ ] Test config parsing

### Day 3: Implement Native SPEC-ID Generation (2 hours)
- [ ] Add `generate_next_spec_id()` to TUI
- [ ] Update /speckit.new routing to skip agent for ID gen
- [ ] Test SPEC-ID increment logic
- [ ] Verify thread safety (multiple concurrent calls)

### Day 4: Validation Testing (2 hours)
- [ ] Run /speckit.auto SPEC-KIT-070 with cheap models
- [ ] Compare output quality vs premium models (blind review)
- [ ] Run full test suite (604 tests must pass)
- [ ] Measure actual cost via API logs
- [ ] Document any quality differences

### Day 5: Cost Tracking Foundation (1 hour)
- [ ] Add basic cost calculation helpers
- [ ] Log estimated costs to telemetry
- [ ] Create cost summary script
- [ ] Validate cost estimates vs actual bills (±10%)

### Rollout Criteria
- ✅ Flash/Haiku/4o work with existing prompts (no regressions)
- ✅ Native SPEC-ID generation passes 100 test runs
- ✅ 604 tests maintain 100% pass rate
- ✅ Consensus quality ≥90% (compare 10 outputs)
- ✅ Cost tracking accurate (±10% of actual)

---

## Risk Mitigation

### Risk 1: Cheap Models Produce Lower Quality Output
**Likelihood**: Medium
**Impact**: High
**Mitigation**:
- A/B test first (run same SPEC with both)
- Keep fallback to premium models
- Monitor consensus agreement rates
- Rollback trigger: <85% agreement

### Risk 2: Prompt Engineering Needed
**Likelihood**: High
**Impact**: Low
**Mitigation**:
- Test prompts with cheap models first
- Adjust for model-specific quirks
- Document prompt modifications
- Budget 2-3 hours for tuning

### Risk 3: API Rate Limits Different
**Likelihood**: Low
**Impact**: Medium
**Mitigation**:
- Check Flash/Haiku rate limits
- Add retry logic with backoff
- Distribute load across providers

---

## Success Metrics - Phase 1

**Primary**:
- ✅ Cost reduced by ≥45% ($11 → $6 or less)
- ✅ Quality maintained (consensus ≥90%, tests 100%)
- ✅ Zero production incidents

**Secondary**:
- ✅ Native SPEC-ID generation works (0 failures in 100 runs)
- ✅ Cost tracking implemented and accurate
- ✅ Documentation updated

**Validation**:
- Run 10 real SPECs with new config
- Blind quality review by 2 developers
- Cost analysis shows ≥45% reduction
- 604 tests remain at 100% pass rate

---

## Quick Reference

### Pricing Comparison

| Model | Input $/1M | Output $/1M | Typical Use | Cost per 10k/2k call |
|-------|-----------|-------------|-------------|---------------------|
| **Gemini Flash 2.0** | $0.10 | $0.40 | Consensus | $0.0018 |
| **Gemini Flash 1.5** | $0.075 | $0.30 | Analysis | $0.0014 |
| Gemini Pro 1.5 | $1.25 | $5.00 | - | $0.0225 |
| **Claude Haiku 3.5** | $0.25 | $1.25 | Tasks | $0.0050 |
| Claude Sonnet 4 | $3.00 | $15.00 | Code gen | $0.0600 |
| **GPT-4o-mini** | $0.15 | $0.60 | Validation | $0.0027 |
| **GPT-4o** | $2.50 | $10.00 | Aggregation | $0.0450 |
| GPT-4 Turbo | $10.00 | $30.00 | - | $0.1600 |

**Typical /speckit.auto call**: 10k input, 2k output per agent

**Current** (3 Pro calls): 3 × $0.0225 = $0.0675
**Phase 1** (3 Flash calls): 3 × $0.0018 = $0.0054
**Savings per consensus**: **$0.062 (92% reduction)**

**At 6 consensus stages**: $0.40 → $0.03 (**$0.37 savings**)

---

## Next Steps

1. **Validate PRD** - Review SPEC-KIT-070/PRD.md for completeness
2. **Get Approval** - Confirm this is the right direction
3. **Start Phase 1** - Week 1 implementation (5-8 hours)
4. **Monitor Impact** - Track costs and quality continuously
5. **Iterate** - Phase 2/3 based on Phase 1 learnings

**Critical Decision Point**: This SPEC should be prioritized ABOVE all others. At current burn rate, we're wasting hundreds of dollars on operations that could cost pennies.
