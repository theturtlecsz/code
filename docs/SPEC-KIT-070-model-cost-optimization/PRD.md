# PRD: Radical Model Cost Optimization Strategy

**SPEC-ID**: SPEC-KIT-070
**Created**: 2025-10-24
**Status**: Draft - **CRITICAL PRIORITY**
**Priority**: **P0** (Infrastructure - blocks sustainable scaling)
**Owner**: Code

---

## 🔥 Executive Summary

**Current State**: Spec-Kit automation burns **$11 per /speckit.auto run** using premium models (Gemini Pro, Claude Sonnet 4, GPT-4 Pro) for ALL tasks regardless of complexity. At scale, this is **unsustainable and wasteful**.

**Proposed State**: Intelligent model routing based on task complexity, using cheap models (Haiku, Flash, 4o-mini) for 80% of operations, **reducing costs by 70-90% ($11 → $1-3 per run)** while maintaining quality through validation.

**Impact**: Enable 5-10x more automation runs with same budget, or reduce monthly costs from ~$500 to ~$50-100 for typical usage.

---

## 1. Problem Statement - The Cost Crisis

### Current Spending Analysis

**From SPEC.md Tier Strategy**:
```
Tier 0: Native TUI           - $0      (status only)
Tier 1: Single Agent         - $0.10   (not implemented!)
Tier 2-lite: Dual Agent      - $0.35   (checklist only)
Tier 2: Triple Agent         - $0.80   (most commands)
Tier 3: Quad Agent           - $2.00   (implement)
Tier 4: Full Pipeline        - $11.00  (auto)
```

**Reality Check** - /speckit.auto breakdown:
```
Plan:      3 agents × $0.80  = $2.40
Tasks:     3 agents × $1.00  = $3.00
Implement: 4 agents × $2.00  = $8.00 (CODE GENERATION!)
Validate:  3 agents × $1.00  = $3.00
Audit:     3 agents × $1.00  = $3.00
Unlock:    3 agents × $1.00  = $3.00
-----------------------------------------
TOTAL:                        $22.40 (before 40% optimization → $11)
```

### Why This Is Catastrophic

1. **Over-Engineering Everywhere**:
   - SPEC-ID generation (simple increment) uses 3 premium models
   - Template filling (string substitution) uses consensus
   - Task decomposition (structured) uses $3 of premium tokens

2. **No Price Sensitivity**:
   - Always use Gemini Pro ($1.25/1M input) when Flash ($0.075/1M) is 17x cheaper
   - Always use Claude Sonnet 4 ($3/1M input) when Haiku 3.5 ($0.25/1M) is 12x cheaper
   - Always use GPT-4 Pro when 4o-mini is 66x cheaper

3. **Wasteful Consensus**:
   - Using 3 agents for deterministic tasks (SPEC-ID: $2.40 for simple increment!)
   - Using 4 agents for code gen when 1-2 could suffice with validation
   - No "try cheap first, escalate if needed" strategy

4. **No Accountability**:
   - No per-SPEC cost tracking
   - No budget limits
   - No cost/quality tradeoff analysis
   - No measurement of model performance differences

### Cheaper Models Available NOW

| Model | Input Cost | Output Cost | vs Current | Use Case |
|-------|-----------|-------------|------------|----------|
| **Claude Haiku 3.5** | $0.25/1M | $1.25/1M | **12x cheaper** | Task decomp, validation, structured output |
| **Gemini Flash 1.5** | $0.075/1M | $0.30/1M | **17x cheaper** | Analysis, planning, consensus voting |
| **GPT-4o-mini** | $0.15/1M | $0.60/1M | **66x cheaper** | Code review, simple reasoning |
| **Gemini Flash 2.0** | $0.10/1M | $0.40/1M | **12x cheaper** | Latest, fast, structured |

**Current Premium Models** (should be RARE):
- Claude Sonnet 4: $3/1M input, $15/1M output - Use for: Complex reasoning, critical decisions
- Gemini Pro 1.5: $1.25/1M input, $5/1M output - Use for: Multi-step analysis
- GPT-4o: $2.50/1M input, $10/1M output - Use for: Aggregation, final validation

---

## 2. Strategic Rethink - Radical Changes

### Core Principle: **Task Complexity Routing**

**Stop**: Using premium models by default
**Start**: Use cheapest model that can handle task complexity

### Task Complexity Classification

#### **Tier S (Simple)** - Deterministic, structured output
**Characteristics**:
- Clear input/output format
- Minimal reasoning needed
- Template-based or rule-based
- Predictable outcomes

**Examples**:
- SPEC-ID generation (find max, increment)
- Template filling (string substitution)
- Status reporting (read state, format)
- File path construction
- Evidence statistics

**Strategy**: **Native Rust** (Tier 0, $0) or **Single Haiku/Flash** (Tier 1, $0.02-0.05)

---

#### **Tier M (Medium)** - Requires judgment, simple consensus
**Characteristics**:
- Needs understanding of context
- Benefits from validation
- Some ambiguity resolution
- Structured reasoning

**Examples**:
- Task decomposition (break PRD into steps)
- Requirement clarification (identify ambiguous points)
- Quality checklist (evaluate against criteria)
- Test planning (coverage analysis)
- Simple code review

**Strategy**: **Dual Cheap Models** (Tier 2-lite, $0.20-0.40)
- Primary: Haiku/Flash (fast, cheap)
- Validator: 4o-mini (confirms/flags issues)

---

#### **Tier C (Complex)** - Multi-step reasoning, consensus critical
**Characteristics**:
- Complex architectural decisions
- Needs diverse perspectives
- Conflict resolution important
- Quality cannot be compromised

**Examples**:
- Implementation planning (design choices)
- Consensus aggregation (resolve disagreements)
- Audit compliance (interpret standards)
- Architecture review (system design)

**Strategy**: **Mixed Tier** (Tier 2-premium, $0.60-1.00)
- 2 cheap models (Haiku + Flash): $0.20
- 1 premium model (Sonnet/Pro): $0.40
- Aggregator (4o): $0.20
- **Total: ~$0.80 (vs $2.40 current)**

---

#### **Tier X (Critical)** - Cannot fail, needs best models
**Characteristics**:
- Security/compliance implications
- Production deployment decisions
- User-facing output quality
- No room for hallucinations

**Examples**:
- Final unlock validation (ship/no-ship decision)
- Security audit findings (vulnerability assessment)
- Production incident analysis
- User-facing documentation

**Strategy**: **Premium Only** (Tier 3, $2.00-3.00) - RARE, <5% of operations

---

### Model Selection Matrix

| Task Type | Primary Model | Validator | Aggregator | Cost | Current Cost | Savings |
|-----------|--------------|-----------|------------|------|--------------|---------|
| **Simple** | Haiku/Flash | None | None | $0.02 | $0.80 | **97%** |
| **Medium** | Haiku + 4o-mini | None | None | $0.30 | $0.80 | **63%** |
| **Complex** | Haiku + Flash + Sonnet | 4o | 4o | $0.80 | $2.40 | **67%** |
| **Critical** | Sonnet + Pro + 4o | GPT-5 | GPT-4o | $2.50 | $2.00 | -25% (worth it) |

---

## 3. Implementation Strategy

### Phase 1: Immediate Wins (Week 1)

**Goal**: Reduce /speckit.auto from $11 to $4-5 (55% savings)

**Quick Wins**:
1. **Replace Gemini Pro with Flash** everywhere (17x cheaper)
   - Flash 1.5 for analysis/planning: $0.075 vs $1.25
   - Flash 2.0 for consensus voting: $0.10 vs $1.25
   - **Savings: ~$3-4 per auto run**

2. **Replace GPT-4 Pro with GPT-4o** for aggregation
   - 4o: $2.50 vs 4 Turbo: $10
   - **Savings: ~$1-2 per auto run**

3. **Implement Tier 1** (single cheap model) for simple tasks
   - SPEC-ID generation: Native Rust (FREE)
   - Status checks: Native Rust (FREE)
   - **Savings: ~$0.50 per operation**

**Expected Result**: $11 → $5-6 (45-55% reduction)

---

### Phase 2: Strategic Routing (Week 2)

**Goal**: Reduce /speckit.auto from $5-6 to $2-3 (70-80% total savings)

**Complexity Routing**:
1. **Classify all 13 /speckit.* commands** by complexity
   ```
   Simple (Tier S → Native/Tier 1):
   - /speckit.status (already Tier 0)
   - /speckit.new (SPEC-ID gen → native)

   Medium (Tier M → Tier 2-lite, cheap models):
   - /speckit.clarify (Haiku + 4o-mini)
   - /speckit.checklist (already Tier 2-lite)
   - /speckit.tasks (Haiku + Flash)

   Complex (Tier C → Mixed tier):
   - /speckit.specify (2 cheap + 1 premium)
   - /speckit.plan (2 cheap + 1 premium)
   - /speckit.analyze (2 cheap + 1 premium)
   - /speckit.validate (2 cheap + 1 premium)
   - /speckit.implement (special case - see below)

   Critical (Tier X → Premium):
   - /speckit.audit (security implications)
   - /speckit.unlock (ship/no-ship decision)
   ```

2. **Special Case: /speckit.implement** (currently $8!)
   - **Current**: 4 premium agents (gemini, claude, gpt_codex, gpt_pro)
   - **Proposed**: 1 premium coder (Sonnet 4) + cheap validator (Haiku)
   - **Rationale**: Code gen needs 1 GOOD model, not 4 mediocre consensus
   - **Savings**: $8 → $1.50 (81% reduction!)

**Expected Result**: $5-6 → $2-3 (70-80% total reduction from baseline)

---

### Phase 3: Dynamic Optimization (Week 3-4)

**Goal**: Adaptive routing with quality validation

**Features**:
1. **Try-Cheap-First Strategy**:
   ```rust
   async fn run_with_fallback(task: Task) -> Result {
       // Try cheap model first
       let result = run_cheap_model(task).await?;

       // Validate quality
       if validate_output(&result) {
           return Ok(result); // Good enough!
       }

       // Escalate to premium if quality insufficient
       warn!("Cheap model insufficient, escalating to premium");
       run_premium_model(task).await
   }
   ```

2. **Cost Tracking & Budgets**:
   ```rust
   struct SpecCostTracker {
       spec_id: String,
       budget: Decimal,       // e.g., $2.00
       spent: Decimal,        // running total
       per_stage: HashMap<SpecStage, Decimal>,
   }

   impl SpecCostTracker {
       fn record_agent_call(&mut self, stage: SpecStage, cost: Decimal) {
           self.spent += cost;
           *self.per_stage.entry(stage).or_default() += cost;

           if self.spent > self.budget * Decimal::from_str("0.8").unwrap() {
               warn!("Approaching budget: ${} / ${}", self.spent, self.budget);
           }
       }
   }
   ```

3. **Quality Monitoring**:
   - Track consensus quality by tier
   - A/B test: cheap vs expensive models
   - Measure actual output differences
   - Auto-tune routing based on data

4. **Cost-Aware Telemetry**:
   ```json
   {
     "command": "validate",
     "spec_id": "SPEC-KIT-070",
     "model_tier": "M",
     "models_used": ["haiku-3.5", "gpt-4o-mini"],
     "estimated_cost": 0.32,
     "actual_tokens": {
       "input": 12500,
       "output": 3200
     },
     "quality_score": 0.95,
     "escalated": false
   }
   ```

---

## 4. Acceptance Criteria

### Cost Targets

| Metric | Current | Phase 1 | Phase 2 | Phase 3 |
|--------|---------|---------|---------|---------|
| **/speckit.auto cost** | $11.00 | $5-6 | $2-3 | $1.50-2.50 |
| **Per-command avg** | $0.80 | $0.40 | $0.20 | $0.10-0.30 |
| **Monthly budget (100 runs)** | $1,100 | $500-600 | $200-300 | $150-250 |
| **Cost reduction** | - | 45-55% | 70-80% | 85-90% |

### Quality Assurance

**Must Not Degrade**:
- ✅ Consensus agreement rate: Maintain ≥90%
- ✅ Test pass rate: Maintain 100% (604/604 tests)
- ✅ Production readiness: All HIGH priority findings still caught
- ✅ Evidence quality: Artifacts remain comprehensive
- ✅ Telemetry completeness: No schema violations

**Validation Strategy**:
1. **A/B Test**: Run same SPEC with cheap vs expensive models
2. **Quality Comparison**: Manual review of 10 outputs per tier
3. **Regression Suite**: 604 tests must pass with cheap models
4. **Production Pilot**: 5 real SPECs with new routing before full rollout

---

## 5. Technical Implementation

### Model Provider Updates

**New Config Section** (`~/.code/config.toml`):
```toml
[models.cost_tiers]
# Simple tasks (Tier S) - Deterministic, structured
tier_s_model = "claude-haiku-3.5"
tier_s_fallback = "gemini-flash-1.5"

# Medium tasks (Tier M) - Judgment, validation
tier_m_primary = "claude-haiku-3.5"
tier_m_validator = "gpt-4o-mini"

# Complex tasks (Tier C) - Multi-step reasoning
tier_c_cheap_1 = "claude-haiku-3.5"
tier_c_cheap_2 = "gemini-flash-2.0"
tier_c_premium = "claude-sonnet-4"
tier_c_aggregator = "gpt-4o"

# Critical tasks (Tier X) - Cannot fail
tier_x_models = ["claude-sonnet-4", "gemini-pro-1.5", "gpt-4o"]
tier_x_validator = "gpt-5-oauth2"  # Keep existing

[models.cost_limits]
per_spec_budget = 2.00  # USD
per_command_budget = 0.50  # USD
monthly_budget = 200.00  # USD
alert_threshold = 0.80  # 80% of budget
```

### Command Classification

**Update command metadata** (`spec_kit/command_registry.rs`):
```rust
pub enum TaskComplexity {
    Simple,     // Deterministic, structured → Tier S
    Medium,     // Requires judgment → Tier M
    Complex,    // Multi-step reasoning → Tier C
    Critical,   // Cannot fail → Tier X
}

impl SpecKitCommand {
    pub fn complexity(&self) -> TaskComplexity {
        match self.name() {
            "status" => TaskComplexity::Simple,
            "clarify" | "checklist" | "tasks" => TaskComplexity::Medium,
            "specify" | "plan" | "analyze" | "validate" | "implement" => TaskComplexity::Complex,
            "audit" | "unlock" => TaskComplexity::Critical,
            _ => TaskComplexity::Complex, // Safe default
        }
    }

    pub fn model_tier(&self) -> ModelTier {
        self.complexity().into()
    }
}
```

### Cost Tracking Integration

**New Module** (`spec_kit/cost_tracker.rs`):
```rust
pub struct CostTracker {
    spec_costs: Arc<Mutex<HashMap<String, SpecCostTracker>>>,
    monthly_total: Arc<AtomicU64>, // Cents
    config: CostConfig,
}

impl CostTracker {
    pub async fn record_agent_call(
        &self,
        spec_id: &str,
        stage: SpecStage,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Result<()> {
        let cost = self.calculate_cost(model, input_tokens, output_tokens);

        // Update per-SPEC tracking
        let mut costs = self.spec_costs.lock().await;
        let tracker = costs.entry(spec_id.to_string()).or_default();
        tracker.record(stage, cost);

        // Check budget limits
        if tracker.spent > tracker.budget * 0.8 {
            warn!("SPEC {} approaching budget: ${:.2} / ${:.2}",
                  spec_id, tracker.spent, tracker.budget);
        }

        // Update monthly total
        self.monthly_total.fetch_add(
            (cost * 100.0) as u64,
            Ordering::SeqCst
        );

        // Write telemetry
        self.write_cost_telemetry(spec_id, stage, model, cost).await?;

        Ok(())
    }

    fn calculate_cost(&self, model: &str, input: u64, output: u64) -> f64 {
        let rates = match model {
            "claude-haiku-3.5" => (0.25, 1.25),
            "claude-sonnet-4" => (3.0, 15.0),
            "gemini-flash-1.5" => (0.075, 0.30),
            "gemini-flash-2.0" => (0.10, 0.40),
            "gemini-pro-1.5" => (1.25, 5.0),
            "gpt-4o-mini" => (0.15, 0.60),
            "gpt-4o" => (2.50, 10.0),
            "gpt-4-turbo" => (10.0, 30.0),
            _ => (3.0, 15.0), // Safe expensive default
        };

        let input_cost = (input as f64 / 1_000_000.0) * rates.0;
        let output_cost = (output as f64 / 1_000_000.0) * rates.1;
        input_cost + output_cost
    }
}
```

---

## 6. Migration Plan

### Week 1: Foundation + Quick Wins
- ✅ Add model cost configuration to config.toml
- ✅ Implement CostTracker module with telemetry
- ✅ Replace Gemini Pro → Flash everywhere (17x cheaper)
- ✅ Replace GPT-4 Turbo → GPT-4o for aggregation
- ✅ Add cost tracking to all agent spawn points
- ✅ Deploy to staging, validate with SPEC-KIT-070 self-test

**Target**: $11 → $5-6 (45-55% reduction)

### Week 2: Strategic Routing
- ✅ Add TaskComplexity classification to command registry
- ✅ Implement model tier selection based on complexity
- ✅ Refactor /speckit.implement to use single premium + validator
- ✅ Update all command prompts for cheaper models
- ✅ Add budget alerts and cost reports
- ✅ A/B test: 10 SPECs with cheap vs current models

**Target**: $5-6 → $2-3 (70-80% total reduction)

### Week 3: Dynamic Optimization
- ✅ Implement try-cheap-first with fallback
- ✅ Add quality validation and auto-escalation
- ✅ Build cost dashboard (evidence/costs/)
- ✅ Add monthly budget tracking and alerts
- ✅ Performance tuning based on real usage data

**Target**: $2-3 → $1.50-2.50 (85-90% total reduction)

### Week 4: Validation & Rollout
- ✅ Run 25 regression SPECs with new routing
- ✅ Validate 604 tests still pass
- ✅ Quality audit: Compare outputs
- ✅ Update documentation (CLAUDE.md, testing-policy.md)
- ✅ Enable for all users
- ✅ Monitor for 2 weeks, tune thresholds

---

## 7. Risk Analysis & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| **Quality degradation with cheap models** | High | Medium | A/B testing, fallback strategy, quality monitoring dashboard |
| **Consensus fails with fewer agents** | Medium | Low | Keep 2-3 agent minimum for complex tasks, mixed tier approach |
| **Budget tracking overhead** | Low | Low | Async telemetry, minimal performance impact (<1ms) |
| **Model availability/rate limits** | Medium | Medium | Multi-provider fallbacks, retry logic, provider diversity |
| **Prompt engineering needed per model** | Medium | High | Test suite validates prompts work across models, iterative tuning |
| **Cost tracking drift from actual** | Low | Medium | Periodic reconciliation with API bills, adjust rates quarterly |

---

## 8. Success Metrics

### Primary KPIs

1. **Cost Reduction**: ≥70% reduction ($11 → $3) by end of Phase 2
2. **Quality Maintained**: Consensus agreement ≥90%, test pass rate 100%
3. **Budget Compliance**: ≥95% of SPECs stay under $2 budget
4. **No Regressions**: Zero production issues caused by model changes

### Secondary Metrics

1. **Adoption**: 100% of /speckit.* commands use cost-aware routing
2. **Visibility**: Cost tracking available for all operations
3. **Optimization**: ≥80% of operations use cheap models (Tier S/M)
4. **Efficiency**: Average cost per command <$0.30 (from $0.80)

### Monitoring Dashboard

**Daily Report** (`evidence/costs/daily_summary.json`):
```json
{
  "date": "2025-10-25",
  "total_spent": 12.35,
  "specs_completed": 8,
  "avg_cost_per_spec": 1.54,
  "model_usage": {
    "haiku-3.5": 45,
    "flash-1.5": 38,
    "4o-mini": 22,
    "sonnet-4": 8,
    "4o": 5
  },
  "tier_distribution": {
    "tier_s": 25,
    "tier_m": 52,
    "tier_c": 35,
    "tier_x": 6
  },
  "budget_status": {
    "monthly_budget": 200.00,
    "spent_to_date": 145.67,
    "remaining": 54.33,
    "projected_month_end": 189.45,
    "alert_level": "yellow"
  }
}
```

---

## 9. Future Enhancements (Post-MVP)

### Phase 4: Advanced Optimization (Month 2+)

1. **Model Performance Profiling**:
   - Track quality metrics per model per task type
   - Build heat map: model × task → quality score
   - Auto-select best cheap model for each task

2. **Smart Caching**:
   - Cache expensive consensus results
   - Reuse similar task outputs
   - Template library from past runs

3. **Batch Processing**:
   - Group similar tasks for single model call
   - Reduce per-call overhead
   - Example: Batch all clarify questions

4. **Custom Fine-Tuned Models**:
   - Fine-tune Haiku on spec-kit tasks
   - Reduce prompt size (cheaper)
   - Better performance on domain-specific work

5. **Dynamic Pricing Integration**:
   - Real-time model pricing via API
   - Auto-switch to cheapest available
   - Negotiate volume discounts

---

## 10. Open Questions

1. **Quality Threshold**: What consensus agreement rate is acceptable? (Proposal: ≥90%)
2. **Budget Allocation**: How to distribute $2 budget across stages? (Proposal: 40% implement, 60% other)
3. **Escalation Triggers**: When to escalate from cheap to premium? (Proposal: Quality score <0.85)
4. **Model Versioning**: How to handle model updates/deprecations? (Proposal: Quarterly review)
5. **User Override**: Allow manual model selection for debugging? (Proposal: Yes, via --model flag)

---

## 11. Dependencies & Prerequisites

**Technical**:
- Model provider SDKs support Haiku 3.5, Flash 2.0, 4o-mini
- Config system supports model selection per complexity tier
- Cost tracking integrated with evidence pipeline
- Telemetry schema extended for cost fields

**Organizational**:
- Budget approval for month 1 ($200 for validation)
- Stakeholder buy-in for 70-90% cost reduction target
- QA resources for A/B testing and quality validation
- Documentation updates coordinated with team

---

## 12. Validation Plan

### Pre-Flight Checks (Before Phase 1)
- ✅ Verify all cheap models available via API
- ✅ Confirm pricing in model provider dashboards
- ✅ Test basic prompts with Haiku/Flash/4o-mini
- ✅ Validate telemetry schema changes

### Phase 1 Validation (Week 1)
- ✅ Run SPEC-KIT-070 (this SPEC) with cheap models
- ✅ Compare output quality vs premium models
- ✅ Verify cost tracking accuracy (±5% of actual)
- ✅ Check 604 tests still pass with Flash/Haiku

### Phase 2 Validation (Week 2)
- ✅ A/B test: 10 SPECs cheap vs premium
- ✅ Quality audit: Blind review by 2 developers
- ✅ Performance test: Latency acceptable (<10% increase)
- ✅ Budget test: Stay under $2/SPEC for 20 runs

### Phase 3 Validation (Week 3-4)
- ✅ Soak test: 50 SPECs with dynamic routing
- ✅ Edge cases: Test fallback, escalation, budget limits
- ✅ Integration: All 13 /speckit.* commands work
- ✅ Production pilot: 5 real features developed

### Rollout Criteria (Week 4)
- ✅ Cost reduction ≥70% validated
- ✅ Quality maintained (≥90% consensus, 100% tests)
- ✅ Zero panics or critical bugs
- ✅ Cost tracking accurate (±10% of actual bills)
- ✅ Documentation complete and reviewed

---

## 13. Documentation Updates Required

1. **CLAUDE.md**:
   - Update model strategy section (lines 8-30)
   - Add cost optimization principles
   - Document task complexity classification
   - Update tier descriptions with new costs

2. **~/.code/config.toml**:
   - Add `[models.cost_tiers]` section
   - Add `[models.cost_limits]` section
   - Update orchestrator instructions with cheap models

3. **docs/spec-kit/testing-policy.md**:
   - Add cost validation requirements
   - Document A/B testing for model changes
   - Add budget compliance checks

4. **New: docs/spec-kit/cost-optimization.md**:
   - Task complexity classification guide
   - Model selection decision tree
   - Cost tracking and budgeting guide
   - Troubleshooting cheap model issues

---

## 14. Rollback Plan

**If Phase 1/2 Shows Issues**:
1. Revert config to premium models immediately
2. Analyze failure patterns in evidence/costs/
3. Adjust classification or add more fallbacks
4. Retry with refined strategy

**Rollback Triggers**:
- Consensus agreement drops below 85%
- Test failures increase by >5%
- Production incidents caused by model issues
- Cost tracking shows unexpected high costs

**Rollback Procedure**:
```bash
# Emergency revert to premium models
git revert <cost-optimization-commit>
cargo build --release
# Redeploy
systemctl restart codex-tui
```

---

## Conclusion

**This is not optimization - this is SURVIVAL**. At current costs, scaling to 100 SPECs/month = $1,100. With cheap models: $150-250. **That's the difference between sustainable and dead project**.

**The strategy is simple**:
1. **Use Rust for deterministic tasks** ($0)
2. **Use cheap models for 80% of operations** ($0.02-0.40)
3. **Use premium models only when critical** ($2-3 for <5% of tasks)
4. **Track everything, optimize continuously**

**Expected Impact**:
- 70-90% cost reduction ($11 → $1.50-3.00 per /speckit.auto)
- 5-10x more automation possible with same budget
- Quality maintained through validation and fallbacks
- Sustainable scaling to 100s of SPECs

**Priority**: **P0 CRITICAL** - This blocks sustainable scaling and should be addressed BEFORE SPEC-KIT-066, 067, 068.
