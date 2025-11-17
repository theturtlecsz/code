# Workflow Patterns

Common usage scenarios and best practices.

---

## Overview

**Workflow patterns** document common Spec-Kit usage scenarios:

- **Full automation**: `/speckit.auto` from PRD to unlock
- **Manual step-by-step**: Individual stage execution
- **Iterative development**: Resume from failed stage
- **Quality-focused**: Multiple quality gates
- **Cost-optimized**: Selective stage execution
- **Hybrid approach**: Mix automation and manual work

**Goal**: Help users choose the right workflow for their use case

---

## Pattern 1: Full Automation

### Use Case

**When**: New feature, comprehensive automation, team consensus needed

**Characteristics**:
- Hands-off execution (6 stages + 3 quality gates)
- 45-50 minutes total
- ~$2.70 cost
- High confidence in output quality

---

### Workflow

```bash
# Step 1: Create SPEC
/speckit.new Add OAuth 2.0 authentication with JWT tokens

# Output:
# ✅ SPEC-KIT-071 created
# 📝 docs/SPEC-KIT-071-oauth-authentication/PRD.md
# Next: Edit PRD or run /speckit.auto

# Step 2: Edit PRD (optional)
# Manually refine requirements, acceptance criteria

# Step 3: Run full automation
/speckit.auto SPEC-KIT-071

# Pipeline executes:
# ✅ Quality Gate: BeforeSpecify (Clarify) - PASS
# ✅ Plan stage (10min, $0.35)
# ✅ Quality Gate: AfterSpecify (Checklist) - PASS (score: 95/100)
# ✅ Tasks stage (3min, $0.10)
# ✅ Quality Gate: AfterTasks (Analyze) - PASS
# ✅ Implement stage (8min, $0.11)
# ✅ Validate stage (10min, $0.35)
# ✅ Audit stage (10min, $0.80)
# ✅ Unlock stage (10min, $0.80)

# Total: 51min, $2.70

# Step 4: Review outputs
ls docs/SPEC-KIT-071-oauth-authentication/
# PRD.md
# plan.md
# tasks.md
# implementation_notes.md
# test_plan.md
# audit_report.md
# unlock_approval.md

# Step 5: Implement code (if approved)
# Follow tasks.md to build the feature
```

---

### When to Use

**✅ GOOD FOR**:
- New features (greenfield)
- Team wants multi-agent consensus
- Quality assurance required
- Budget comfortable (~$3 per SPEC)
- Time available (45-50 minutes)

**❌ NOT FOR**:
- Simple bug fixes (overkill)
- Tight budget (<$1)
- Urgent fixes (too slow)
- Well-understood tasks (no consensus needed)

---

## Pattern 2: Manual Step-by-Step

### Use Case

**When**: Incremental development, review between stages, learning Spec-Kit

**Characteristics**:
- Full control over each stage
- Review outputs before proceeding
- Can skip unnecessary stages
- ~$2.70 cost (same as auto)
- Longer timeline (spread across days)

---

### Workflow

```bash
# Step 1: Create SPEC
/speckit.new Implement caching layer with Redis

# ✅ SPEC-KIT-072 created

# Step 2: Clarify PRD
/speckit.clarify SPEC-KIT-072

# Found 3 ambiguities:
# - "fast cache" (no metric)
# - "TBD expiration policy"
# - etc.

# Fix ambiguities manually in PRD.md

# Step 3: Run plan
/speckit.plan SPEC-KIT-072

# Review plan.md:
# - Work breakdown looks good
# - Acceptance mapping complete
# - Risks identified

# Approve and continue

# Step 4: Generate tasks
/speckit.tasks SPEC-KIT-072

# Review tasks.md:
# - 12 tasks identified
# - Dependencies clear
# - Estimated 2 weeks total

# Step 5: Check quality before implementation
/speckit.checklist SPEC-KIT-072

# Score: 88/100 (B)
# Issues:
# - 1 acceptance criterion missing
# - Fix and re-run

# Step 6: Analyze consistency
/speckit.analyze SPEC-KIT-072

# Found 2 issues:
# - plan references FR-005 (PRD only has FR-001 to FR-004)
# - Fix plan.md

# Step 7: Implement
/speckit.implement SPEC-KIT-072

# Review implementation_notes.md:
# - Code structure proposed
# - Files to create
# - Integration points

# Manually code the feature

# Step 8: Validate
/speckit.validate SPEC-KIT-072

# Review test_plan.md:
# - Test scenarios defined
# - Coverage requirements
# - Edge cases identified

# Write tests

# Step 9: Audit
/speckit.audit SPEC-KIT-072

# Review audit_report.md:
# - OWASP Top 10: PASS
# - Dependencies: PASS
# - Licenses: PASS

# Step 10: Unlock
/speckit.unlock SPEC-KIT-072

# Decision: APPROVED
# Ready to merge
```

---

### When to Use

**✅ GOOD FOR**:
- Learning Spec-Kit (understand each stage)
- Complex features (review between stages)
- Team collaboration (discuss outputs before proceeding)
- Custom workflows (skip some stages)

**❌ NOT FOR**:
- Repetitive tasks (automation better)
- Tight deadlines (too slow manually)
- Solo development (less review needed)

---

## Pattern 3: Iterative Development

### Use Case

**When**: First attempt failed, resuming from specific stage, fixing issues

**Characteristics**:
- Resume from failed stage
- Skip completed work
- Fix issues and retry
- Variable cost (only re-run stages)

---

### Workflow

```bash
# Initial attempt fails at Implement
/speckit.auto SPEC-KIT-073

# ✅ Plan stage - PASS
# ✅ Quality Gate: AfterSpecify - PASS
# ✅ Tasks stage - PASS
# ✅ Quality Gate: AfterTasks - PASS
# ❌ Implement stage - FAIL (git tree not clean)

# Fix issue: commit pending changes
git add .
git commit -m "WIP: prepare for Spec-Kit"

# Resume from implement
/speckit.auto SPEC-KIT-073 --from implement

# Skipped stages:
# - Plan (already complete)
# - Tasks (already complete)
# - Quality gates (memoized)

# Running:
# ✅ Implement stage - PASS
# ✅ Validate stage - PASS
# ✅ Audit stage - PASS
# ✅ Unlock stage - PASS

# Total resumed cost: ~$2.17 (saved $0.45 on skipped stages)
```

---

### When to Use

**✅ GOOD FOR**:
- Recovering from failures
- Iterating on specific stage
- Fixing quality gate failures
- Budget-conscious (avoid redundant work)

**❌ NOT FOR**:
- First-time execution (no prior work to skip)
- Major PRD changes (invalidates prior stages)

---

## Pattern 4: Quality-Focused

### Use Case

**When**: High-quality requirements, sensitive features, compliance needed

**Characteristics**:
- Run all quality gates
- Manual review of each gate
- Fix issues immediately
- Higher time investment (quality > speed)

---

### Workflow

```bash
# Step 1: Create SPEC
/speckit.new Implement payment processing with Stripe

# ✅ SPEC-KIT-074 created

# Step 2: Clarify PRD (quality gate)
/speckit.clarify SPEC-KIT-074

# Found 5 ambiguities (2 critical):
# - "secure payment" (no security standard specified)
# - "TBD error handling"

# Fix all issues before proceeding

# Step 3: Checklist (quality gate)
/speckit.checklist SPEC-KIT-074

# Score: 75/100 (C) - FAIL
# Issues:
# - Missing NFR for PCI compliance
# - No acceptance criteria for error scenarios
# - Add and re-run

# Re-run after fixes
/speckit.checklist SPEC-KIT-074

# Score: 92/100 (A) - PASS

# Step 4: Run plan
/speckit.plan SPEC-KIT-074

# Step 5: Analyze (quality gate)
/speckit.analyze SPEC-KIT-074

# Found 1 issue:
# - plan mentions "credit card storage" (out of scope per PRD)
# - Fix plan.md

# Re-run after fix
/speckit.analyze SPEC-KIT-074

# 0 issues - PASS

# Step 6: Continue pipeline
/speckit.auto SPEC-KIT-074 --from tasks

# (Skips plan, quality gates already passed)

# Manual review at each stage:
# - Tasks: Review for security concerns
# - Implement: Code review for PCI compliance
# - Validate: Verify error handling tests
# - Audit: Extra scrutiny on security checks
# - Unlock: Final approval with team
```

---

### When to Use

**✅ GOOD FOR**:
- Payment processing, auth, security features
- Compliance requirements (HIPAA, PCI, GDPR)
- Production-critical features
- Team wants high confidence

**❌ NOT FOR**:
- Experimental features (lower quality acceptable)
- Internal tools (less risk)
- Prototypes (speed > quality)

---

## Pattern 5: Cost-Optimized

### Use Case

**When**: Tight budget, simple features, manual implementation preferred

**Characteristics**:
- Use native operations (FREE)
- Skip expensive stages
- Manual implementation
- ~$0-0.50 cost

---

### Workflow

```bash
# Step 1: Create SPEC (native, FREE)
/speckit.new Add tooltip to settings button

# ✅ SPEC-KIT-075 created

# Step 2: Clarify (native, FREE)
/speckit.clarify SPEC-KIT-075

# 0 ambiguities - PASS

# Step 3: Checklist (native, FREE)
/speckit.checklist SPEC-KIT-075

# Score: 85/100 (B) - PASS

# Step 4: Analyze (native, FREE)
/speckit.analyze SPEC-KIT-075

# 0 issues - PASS

# Step 5: Manual plan
# Write plan.md by hand
# Cost: $0 (manual work)

# Step 6: Manual tasks
# Write tasks.md by hand
# Cost: $0

# Step 7: Manual implementation
# Code the tooltip
# Cost: $0

# Step 8: Skip validate, audit, unlock
# (Simple feature, low risk, manual testing sufficient)

# Total cost: $0
# Time: 2 hours (mostly manual work)
```

---

### When to Use

**✅ GOOD FOR**:
- Simple UI changes (tooltips, labels, colors)
- Bug fixes (known solution)
- Tight budget (<$1)
- Developer prefers manual work

**❌ NOT FOR**:
- Complex features (manual planning error-prone)
- Team consensus needed (no multi-agent)
- Quality assurance required (no validation)

---

## Pattern 6: Hybrid Approach

### Use Case

**When**: Mix automation and manual work, selective stage execution

**Characteristics**:
- Automate strategic stages (plan, validate)
- Manual implementation (code quality preference)
- Skip stages not needed
- ~$0.70-1.50 cost

---

### Workflow

```bash
# Step 1: Create SPEC (native, FREE)
/speckit.new Refactor database query optimization

# ✅ SPEC-KIT-076 created

# Step 2: Quality gates (native, FREE)
/speckit.clarify SPEC-KIT-076
/speckit.checklist SPEC-KIT-076
/speckit.analyze SPEC-KIT-076

# All PASS

# Step 3: Automate plan (multi-agent, $0.35)
/speckit.plan SPEC-KIT-076

# Multi-agent consensus on optimization strategy

# Step 4: Manual tasks
# Break down plan into implementation tasks
# Cost: $0 (manual)

# Step 5: Manual implementation
# Code the optimizations
# Cost: $0

# Step 6: Automate validate (multi-agent, $0.35)
/speckit.validate SPEC-KIT-076

# Multi-agent consensus on test coverage

# Step 7: Manual testing
# Write and run performance tests
# Cost: $0

# Step 8: Skip audit (low security risk)

# Step 9: Manual unlock
# Review and approve for merge
# Cost: $0

# Total cost: $0.70 (2 stages automated)
# Time: 1 day (including manual work)
```

---

### When to Use

**✅ GOOD FOR**:
- Teams with strong manual coding preference
- Budget-conscious but want strategic automation
- Specific stages benefit from consensus (plan, validate)
- Other stages simple enough for manual (tasks, implement)

**❌ NOT FOR**:
- All-or-nothing preference (use Pattern 1 or 5)
- Inconsistent quality (automation ensures standards)

---

## Comparison Table

| Pattern | Cost | Time | Quality | Use Case |
|---------|------|------|---------|----------|
| **1. Full Automation** | ~$2.70 | 45-50min | Highest | Comprehensive, team consensus |
| **2. Manual Step-by-Step** | ~$2.70 | 1-3 days | High | Learning, review between stages |
| **3. Iterative Development** | Variable | Variable | High | Resume from failures |
| **4. Quality-Focused** | ~$2.70+ | 2-5 days | Highest | Security, compliance, critical |
| **5. Cost-Optimized** | ~$0 | 2-8 hours | Medium | Simple features, tight budget |
| **6. Hybrid Approach** | ~$0.70-1.50 | 1-2 days | High | Strategic automation, manual code |

---

## Decision Tree

```
Start: What's your priority?

Speed?
  ├─ Complex feature? → Pattern 1 (Full Automation)
  └─ Simple feature? → Pattern 5 (Cost-Optimized)

Quality?
  ├─ Critical feature? → Pattern 4 (Quality-Focused)
  └─ Standard feature? → Pattern 1 (Full Automation)

Cost?
  ├─ $0 budget? → Pattern 5 (Cost-Optimized)
  ├─ <$1 budget? → Pattern 6 (Hybrid Approach)
  └─ <$3 budget? → Pattern 1 (Full Automation)

Learning?
  └─ Understand Spec-Kit? → Pattern 2 (Manual Step-by-Step)

Recovery?
  └─ Prior attempt failed? → Pattern 3 (Iterative Development)
```

---

## Best Practices

### General Guidelines

**DO**:
- ✅ Use native operations first (clarify, checklist, analyze) - FREE
- ✅ Run quality gates before expensive stages
- ✅ Review outputs before proceeding to next stage
- ✅ Resume from failed stage (don't restart from scratch)
- ✅ Monitor cost with `/speckit.status`

**DON'T**:
- ❌ Skip quality gates for critical features
- ❌ Run full automation for simple fixes
- ❌ Ignore warnings from quality gates
- ❌ Re-run successful stages unnecessarily

---

### Stage Selection

**Always Run**:
- ✅ PRD creation (`/speckit.new`) - FREE, instant
- ✅ Clarify (`/speckit.clarify`) - FREE, catches ambiguities
- ✅ Checklist (`/speckit.checklist`) - FREE, quality scoring

**Usually Run**:
- ✅ Plan (`/speckit.plan`) - $0.35, strategic value
- ✅ Validate (`/speckit.validate`) - $0.35, test coverage

**Sometimes Run**:
- 🤔 Tasks (`/speckit.tasks`) - $0.10, simple breakdown (can do manually)
- 🤔 Implement (`/speckit.implement`) - $0.11, code hints (manual coding common)

**Rarely Run**:
- 🤔 Audit (`/speckit.audit`) - $0.80, expensive (skip for low-risk)
- 🤔 Unlock (`/speckit.unlock`) - $0.80, expensive (manual approval common)

---

### Quality Gate Strategy

**Run All Gates** (recommended):
```bash
/speckit.clarify SPEC-ID    # Before plan
/speckit.plan SPEC-ID
/speckit.checklist SPEC-ID  # Before tasks
/speckit.tasks SPEC-ID
/speckit.analyze SPEC-ID    # Before implement
/speckit.implement SPEC-ID
```

**Cost**: $0 (all native)
**Benefit**: Catch issues early, avoid wasted agent costs

**Skip Gates** (not recommended):
```bash
/speckit.auto SPEC-ID --skip-quality-gates
```

**Cost**: Save ~1-2 minutes
**Risk**: Miss issues, potential rework later

---

## Common Scenarios

### Scenario 1: New Feature (Standard)

```bash
# PRD already exists, want full automation
/speckit.auto SPEC-KIT-070

# Cost: ~$2.70
# Time: 45-50 minutes
# Output: plan, tasks, implementation notes, tests, audit, approval
```

---

### Scenario 2: Bug Fix (Simple)

```bash
# Known issue, manual implementation
/speckit.new Fix null pointer in parser
/speckit.clarify SPEC-KIT-071  # 0 issues
/speckit.checklist SPEC-KIT-071  # 92/100 (A)

# Manual:
# - Write fix
# - Test
# - Merge

# Cost: $0
# Time: 1-2 hours
```

---

### Scenario 3: Failed Implementation

```bash
# Implement stage failed (git tree dirty)
# Fix issue, resume from implement

git add . && git commit -m "WIP"
/speckit.auto SPEC-KIT-072 --from implement

# Cost: ~$2.17 (saved $0.45 on skipped stages)
# Time: ~35 minutes
```

---

### Scenario 4: Experimental Prototype

```bash
# Quick prototype, minimal quality gates
/speckit.new Experiment with WebGL renderer
/speckit.clarify SPEC-KIT-073  # 0 issues

# Manual:
# - Write prototype code
# - Test in sandbox
# - Iterate

# Skip: plan, tasks, validate, audit, unlock (not needed for prototype)

# Cost: $0
# Time: 4-6 hours (manual coding)
```

---

### Scenario 5: Production-Critical Feature

```bash
# Payment processing, maximum quality
/speckit.new Implement Stripe payment integration

# Quality gates:
/speckit.clarify SPEC-KIT-074
/speckit.checklist SPEC-KIT-074

# Fix all issues (iterate until 95+ score)

# Automation:
/speckit.auto SPEC-KIT-074

# Manual review:
# - Review plan.md (team discussion)
# - Review implementation_notes.md (architecture approval)
# - Review audit_report.md (security team approval)

# Cost: ~$2.70
# Time: 2-3 days (including reviews)
```

---

## Summary

**Workflow Patterns Highlights**:

1. **6 Patterns**: Full automation, manual, iterative, quality-focused, cost-optimized, hybrid
2. **Decision Tree**: Choose pattern by priority (speed, quality, cost, learning)
3. **Best Practices**: Always run native gates, review outputs, resume from failures
4. **Stage Selection**: Always (clarify, checklist), usually (plan, validate), sometimes (tasks, implement), rarely (audit, unlock)
5. **Common Scenarios**: New feature, bug fix, failed implementation, prototype, production-critical

**Pattern Selection**:
- **Speed + Comprehensive**: Pattern 1 (Full Automation)
- **Learning**: Pattern 2 (Manual Step-by-Step)
- **Recovery**: Pattern 3 (Iterative Development)
- **Critical**: Pattern 4 (Quality-Focused)
- **Budget**: Pattern 5 (Cost-Optimized)
- **Balanced**: Pattern 6 (Hybrid Approach)

---

**End of SPEC-DOC-003 (Spec-Kit Framework)**

Total Deliverables: 10/10 complete
- command-reference.md ✅
- pipeline-architecture.md ✅
- consensus-system.md ✅
- quality-gates.md ✅
- native-operations.md ✅
- evidence-repository.md ✅
- cost-tracking.md ✅
- agent-orchestration.md ✅
- template-system.md ✅
- workflow-patterns.md ✅
