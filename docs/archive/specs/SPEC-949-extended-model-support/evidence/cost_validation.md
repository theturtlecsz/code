# SPEC-949 Cost Validation Report

**SPEC**: SPEC-949 (Extended Model Support)
**Validation Date**: TBD (awaiting GPT-5 access)
**Status**: 🟡 Template (validation pending)

---

## Executive Summary

**Objective**: Validate -13% cost reduction claim ($2.71 → $2.36 per /speckit.auto run)

**Methodology**:
- Baseline: GPT-4 era cost structure (from SPEC-070)
- Test: Run n≥3 complete SPEC pipelines with GPT-5 agents
- Measure: Per-stage cost from telemetry, total pipeline cost
- Compare: Actual vs expected cost reduction

**Results**: ⏳ Pending validation run

---

## Cost Baseline (GPT-4 Era)

**Source**: SPEC-070 tiered model strategy (2025-10-15)
**Context**: Pre-SPEC-949 agent routing

| Stage | Agent(s) | Tier | Expected Cost | Notes |
|-------|----------|------|---------------|-------|
| **specify** | gpt5-low (hypothetical) | 1 | $0.10 | Single-agent, simple transformation |
| **plan** | gemini-flash + claude-haiku + gpt5-medium | 2 | $0.35 | Multi-agent consensus |
| **tasks** | gpt5-low | 1 | $0.10 | Single-agent, task breakdown |
| **implement** | gpt_codex + claude-haiku | 2 | $0.11 | Code generation + validation |
| **validate** | gemini-flash + claude-haiku + gpt5-medium | 2 | $0.35 | Test strategy consensus |
| **audit** | gpt5-high + claude-sonnet + gemini-pro | 3 | $0.80 | Security/compliance, premium |
| **unlock** | gpt5-high + claude-sonnet + gemini-pro | 3 | $0.80 | Ship decision, premium |
| **Total** | | | **$2.71** | Full /speckit.auto pipeline |

---

## Cost Target (GPT-5 Era)

**Source**: SPEC-949 implementation plan (lines 420-432)
**Context**: Post-SPEC-949 agent routing

| Stage | Agent(s) | GPT-5 Model | Expected Cost | Reduction | Notes |
|-------|----------|-------------|---------------|-----------|-------|
| **specify** | gpt5_1_mini | gpt-5.1-codex-mini | $0.08 | -20% | Cost-optimized variant |
| **plan** | gemini-flash + claude-haiku + gpt5_1 | gpt-5.1 (adaptive) | $0.30 | -14% | Adaptive reasoning speedup |
| **tasks** | gpt5_1_mini | gpt-5.1-codex-mini | $0.08 | -20% | Cost-optimized variant |
| **implement** | gpt5_1_codex + claude-haiku | gpt-5.1-codex | $0.10 | -9% | Enhanced agentic + tool use |
| **validate** | gemini-flash + claude-haiku + gpt5_1 | gpt-5.1 (adaptive) | $0.30 | -14% | Adaptive reasoning speedup |
| **audit** | gpt5_codex + claude-sonnet + gemini-pro | gpt-5-codex | $0.80 | 0% | Quality over cost |
| **unlock** | gpt5_codex + claude-sonnet + gemini-pro | gpt-5-codex | $0.80 | 0% | Quality over cost |
| **Total** | | | **$2.36** | **-13%** | **Target: $0.35 savings per run** |

**Acceptable Range**: $2.30-$2.42 (±2.5% of target)

---

## Validation Methodology

### Test SPECs

**Primary**: SPEC-900 (generic smoke test)
- Neutral multi-stage workload
- Plan → tasks → validate (3 stages)
- Avoids bias from domain-specific complexity

**Secondary**: Create minimal test SPEC
- Small feature scope (<100 LOC changes)
- All 7 stages executed
- Representative of typical spec-kit usage

**Sample Size**: n ≥ 3 complete pipeline runs
- Reduces variance from API latency, caching effects
- Calculate mean and standard deviation
- Report median as primary metric

### Measurement Approach

**1. Telemetry Extraction**:
```bash
# Per-stage cost extraction
for stage in specify plan tasks implement validate audit unlock; do
    cat docs/SPEC-XXX/evidence/$stage/telemetry_*.json \
        | jq -r '.cost' 2>/dev/null || echo "0"
done | awk '{sum += $1} END {print "Total: $" sum}'
```

**2. Consensus Artifacts**:
```bash
# Verify which agents were actually used
cat docs/SPEC-XXX/evidence/*/consensus_*.json \
    | jq -r '{stage: .stage, agent: .agent, cost: .cost}'
```

**3. Cost Breakdown Analysis**:
- Group by stage
- Group by agent
- Group by model family (GPT-5 vs Gemini vs Claude)
- Identify outliers (>2 standard deviations from mean)

### Success Criteria

| Metric | Target | Acceptable Range | Failure Threshold |
|--------|--------|------------------|-------------------|
| **Total Pipeline Cost** | $2.36 | $2.30-$2.42 | >$2.71 (worse than baseline) |
| **Per-Stage Reduction** | -13% avg | -10% to -15% | Any stage >+5% (cost increase) |
| **Consistency** | σ <$0.10 | σ <$0.15 | σ >$0.20 (high variance) |
| **Caching Benefit** | 70-90% on follow-up | 50-90% | <30% (caching not working) |

---

## Actual Results

### Run 1: [SPEC-ID] [Date]

**Test SPEC**: TBD
**Pipeline**: /speckit.auto [SPEC-ID]
**Timestamp**: TBD

**Per-Stage Costs**:
| Stage | Agent Used | Model | Actual Cost | Expected | Δ | Status |
|-------|------------|-------|-------------|----------|---|--------|
| specify | TBD | TBD | TBD | $0.08 | TBD | ⏳ |
| plan | TBD | TBD | TBD | $0.30 | TBD | ⏳ |
| tasks | TBD | TBD | TBD | $0.08 | TBD | ⏳ |
| implement | TBD | TBD | TBD | $0.10 | TBD | ⏳ |
| validate | TBD | TBD | TBD | $0.30 | TBD | ⏳ |
| audit | TBD | TBD | TBD | $0.80 | TBD | ⏳ |
| unlock | TBD | TBD | TBD | $0.80 | TBD | ⏳ |
| **Total** | | | **TBD** | **$2.36** | **TBD** | ⏳ |

**Notes**: TBD

**Evidence Files**:
- Telemetry: `docs/[SPEC-ID]/evidence/*/telemetry_*.json`
- Consensus: `docs/[SPEC-ID]/evidence/*/consensus_*.json`

---

### Run 2: [SPEC-ID] [Date]

*(Same structure as Run 1)*

**Per-Stage Costs**: TBD

---

### Run 3: [SPEC-ID] [Date]

*(Same structure as Run 1)*

**Per-Stage Costs**: TBD

---

## Aggregate Analysis

### Cost Summary (n=3 runs)

| Stage | Mean Cost | Median | Std Dev (σ) | Min | Max | Expected | Δ from Expected | Status |
|-------|-----------|--------|-------------|-----|-----|----------|-----------------|--------|
| specify | TBD | TBD | TBD | TBD | TBD | $0.08 | TBD | ⏳ |
| plan | TBD | TBD | TBD | TBD | TBD | $0.30 | TBD | ⏳ |
| tasks | TBD | TBD | TBD | TBD | TBD | $0.08 | TBD | ⏳ |
| implement | TBD | TBD | TBD | TBD | TBD | $0.10 | TBD | ⏳ |
| validate | TBD | TBD | TBD | TBD | TBD | $0.30 | TBD | ⏳ |
| audit | TBD | TBD | TBD | TBD | TBD | $0.80 | TBD | ⏳ |
| unlock | TBD | TBD | TBD | TBD | TBD | $0.80 | TBD | ⏳ |
| **Total** | **TBD** | **TBD** | **TBD** | **TBD** | **TBD** | **$2.36** | **TBD** | ⏳ |

### Comparison to Baseline

| Metric | Baseline (GPT-4) | Actual (GPT-5) | Reduction | Target | Status |
|--------|------------------|----------------|-----------|--------|--------|
| **Total Cost** | $2.71 | TBD | TBD | -13% ($0.35) | ⏳ |
| **Simple Stages** (specify, tasks) | $0.20 | TBD | TBD | -20% ($0.04) | ⏳ |
| **Multi-Agent** (plan, validate) | $0.70 | TBD | TBD | -14% ($0.10) | ⏳ |
| **Premium** (audit, unlock) | $1.60 | TBD | TBD | 0% ($0.00) | ⏳ |

---

## Performance Metrics

### Execution Time (if SPEC-940 timing available)

| Stage | Baseline (GPT-4) | Actual (GPT-5) | Speedup | Expected Speedup | Status |
|-------|------------------|----------------|---------|------------------|--------|
| specify (simple) | ~4 min | TBD | TBD | 2-3× (1.5-2 min) | ⏳ |
| plan (complex) | ~12 min | TBD | TBD | 1.3× (8-10 min) | ⏳ |
| tasks (simple) | ~4 min | TBD | TBD | 2-3× (1.5-2 min) | ⏳ |
| implement | ~15 min | TBD | TBD | 1-1.5× | ⏳ |
| validate (complex) | ~12 min | TBD | TBD | 1.3× (8-10 min) | ⏳ |
| audit | ~12 min | TBD | TBD | 1× (same) | ⏳ |
| unlock | ~10 min | TBD | TBD | 1× (same) | ⏳ |
| **Total Pipeline** | ~60 min | TBD | TBD | 1.2× (50 min, -17%) | ⏳ |

**Adaptive Reasoning Hypothesis**: 2-3× speedup on simple tasks (specify, tasks) due to dynamic compute allocation

---

## Caching Effectiveness

### Test: Follow-up Query Cost Reduction

**Methodology**:
1. Run /speckit.plan [SPEC-ID] (first run, no cache)
2. Wait 1 minute
3. Re-run /speckit.plan [SPEC-ID] (24h cache hit expected)
4. Compare costs

**Results**:

| Run | Timestamp | Cost | Cache Hit | Reduction | Status |
|-----|-----------|------|-----------|-----------|--------|
| First | TBD | TBD | No | N/A | ⏳ |
| Follow-up | TBD | TBD | TBD | TBD | ⏳ |

**Expected**: 70-90% cost reduction on follow-up (24h cache active)
**Actual**: TBD

**Comparison to GPT-4**:
- GPT-4: 5-minute cache (expires before most follow-ups)
- GPT-5: 24-hour cache (persists across work sessions)
- **Benefit**: Enables iterative refinement without cost penalty

---

## Findings & Analysis

### Cost Reduction Achievement

**Primary Question**: Did we achieve -13% cost reduction ($2.71 → $2.36)?

**Answer**: ⏳ Pending validation

**Analysis (when complete)**:
- [ ] **Success**: Median cost within $2.30-$2.42 range
- [ ] **Partial Success**: Cost reduced but not to target (e.g., $2.50)
- [ ] **Failure**: Cost equal to or higher than baseline ($2.71+)

**Root Cause Analysis (if target not met)**:
- TBD: Pricing changes?
- TBD: Model usage incorrect?
- TBD: Caching not active?
- TBD: Higher token usage than expected?

### Performance Improvement

**Primary Question**: Did adaptive reasoning deliver 2-3× speedup on simple tasks?

**Answer**: ⏳ Pending validation

**Analysis (when complete)**:
- [ ] **Hypothesis Confirmed**: Simple stages 2-3× faster
- [ ] **Partial Confirmation**: 1.5-2× faster (still beneficial)
- [ ] **Hypothesis Rejected**: No significant speedup

### Caching Benefit

**Primary Question**: Does 24h cache provide 70-90% follow-up savings?

**Answer**: ⏳ Pending validation

**Analysis (when complete)**:
- [ ] **Confirmed**: 70-90% reduction on follow-ups
- [ ] **Partial**: 50-70% reduction (still valuable)
- [ ] **Minimal**: <50% reduction (caching may not be active)

---

## Recommendations

### If Validation Succeeds (Cost Target Met)

1. ✅ **Production Deployment**: GPT-5 agents are cost-effective
2. ✅ **Update Default Config**: Keep GPT-5 as default routing
3. 📊 **Monitor Costs**: Track monthly spend, compare to baseline
4. 📘 **Document Actual Costs**: Update guides with real numbers
5. 🚀 **Communicate**: Inform team of cost savings achieved

### If Partial Success (Cost Reduced but <-13%)

1. 🔍 **Investigate Gap**: Identify which stages exceeded expected cost
2. 🔧 **Selective Optimization**: Use cheaper models for expensive stages
3. 📊 **Measure Long-Term**: May average out over more runs
4. 🔄 **Adjust Targets**: Update documentation with realistic numbers
5. ✅ **Proceed Cautiously**: Deploy if cost still lower than baseline

### If Validation Fails (Cost ≥ Baseline)

1. 🚨 **Rollback**: Revert to GPT-4 agents (see GPT5_MIGRATION_GUIDE.md)
2. 🔍 **Root Cause Analysis**: Diagnose why costs higher
3. 🆘 **Contact OpenAI**: Verify pricing, check for account issues
4. 📋 **Document Failure**: Preserve evidence for future investigation
5. ⏸️ **Pause SPEC-948/947**: Delay dependent SPECs until resolved

---

## Validation Checklist

### Prerequisites
- [ ] GPT-5 API access confirmed (test with curl)
- [ ] SPEC-949 Phases 1-3 complete (git log shows 3 commits)
- [ ] Test SPEC available (SPEC-900 or create new)
- [ ] Telemetry collection enabled

### Execution
- [ ] Run 1 complete (n=1)
- [ ] Run 2 complete (n=2)
- [ ] Run 3 complete (n=3)
- [ ] Caching test complete (follow-up query)

### Analysis
- [ ] Per-stage costs extracted from telemetry
- [ ] Agent usage verified from consensus artifacts
- [ ] Mean/median/σ calculated
- [ ] Comparison to baseline completed
- [ ] Performance metrics captured (if available)

### Documentation
- [ ] All tables filled with actual data
- [ ] Findings section completed
- [ ] Recommendations selected
- [ ] Evidence files referenced
- [ ] Report reviewed and validated

---

## Appendix: Telemetry Commands

### Extract Per-Stage Costs

```bash
#!/bin/bash
# Usage: ./extract_costs.sh SPEC-ID

SPEC_ID=$1
EVIDENCE_DIR="../docs/${SPEC_ID}/evidence"

echo "SPEC: $SPEC_ID"
echo "Stage,Cost"

for stage in specify plan tasks implement validate audit unlock; do
    COST=$(cat $EVIDENCE_DIR/$stage/telemetry_*.json 2>/dev/null | jq -r '.cost // 0')
    echo "$stage,$COST"
done

echo ""
echo -n "Total,"
cat $EVIDENCE_DIR/*/telemetry_*.json 2>/dev/null \
    | jq -r '.cost // 0' \
    | awk '{sum += $1} END {print sum}'
```

### Verify Agent Usage

```bash
#!/bin/bash
# Usage: ./verify_agents.sh SPEC-ID

SPEC_ID=$1
EVIDENCE_DIR="../docs/${SPEC_ID}/evidence"

echo "SPEC: $SPEC_ID"
echo "Stage,Agent,Model"

for stage in specify plan tasks implement validate audit unlock; do
    AGENT=$(cat $EVIDENCE_DIR/$stage/consensus_*.json 2>/dev/null \
        | jq -r '.agent // "N/A"' \
        | head -1)
    echo "$stage,$AGENT"
done
```

### Compare Multiple Runs

```bash
#!/bin/bash
# Usage: ./compare_runs.sh SPEC-ID-1 SPEC-ID-2 SPEC-ID-3

echo "Stage,Run1,Run2,Run3,Mean,Median"

for stage in specify plan tasks implement validate audit unlock; do
    COST1=$(cat ../docs/$1/evidence/$stage/telemetry_*.json 2>/dev/null | jq -r '.cost // 0')
    COST2=$(cat ../docs/$2/evidence/$stage/telemetry_*.json 2>/dev/null | jq -r '.cost // 0')
    COST3=$(cat ../docs/$3/evidence/$stage/telemetry_*.json 2>/dev/null | jq -r '.cost // 0')

    MEAN=$(echo "$COST1 $COST2 $COST3" | awk '{print ($1 + $2 + $3) / 3}')
    MEDIAN=$(echo "$COST1 $COST2 $COST3" | tr ' ' '\n' | sort -n | sed -n '2p')

    echo "$stage,$COST1,$COST2,$COST3,$MEAN,$MEDIAN"
done
```

---

**Report Status**: 🟡 Template Complete (Awaiting Validation Data)
**Next Action**: Run n≥3 test SPECs with GPT-5 agents, populate results
**Last Updated**: 2025-11-16
