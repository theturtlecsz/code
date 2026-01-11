# SPEC-KIT-071 Cleanup Quick Start Guide

**Goal**: Reduce memory bloat from 574 → ~300 memories (48% reduction)
**Timeline**: Can start NOW (doesn't need TUI/GPT access)
**Effort**: 8-12 hours over 4 days

---

## Critical Issues Discovered

### 🚨 Issue #1: Tag Explosion
- **574 memories** with **552 unique tags**
- 96% tag-to-memory ratio (chaos!)
- Should be: 5-10 memories per tag (~60-100 tags total)

### 🚨 Issue #2: Byterover Pollution
- **50+ memories** reference deprecated system (8.7% of total)
- Deprecated since: 2025-10-18
- All contain "From Byterover memory layer..." (historical cruft)

### 🚨 Issue #3: Analysis Tool Broken
- Query returned 35,906 tokens (exceeds 25k limit)
- System too bloated for basic analysis
- **This is a RED FLAG** - data is out of control

### 🚨 Issue #4: Importance Inflation
- Average: 7.88/10 (too high)
- Should be: 5.5-6.5 (proper distribution)
- If everything is important (7-10), nothing is

### 🚨 Issue #5: No Organization
- Domains: 0 used (unused feature)
- Categories: Unknown (probably unused)
- Tags: Chaos (552 unique, inconsistent naming)

---

## Immediate Actions (Day 1-2)

### Action 1: Purge Byterover Memories (2-3 hours)

**Count**: 50 memories found with byterover tag

**Strategy**: Delete 48, keep 2
- **Keep**: Migration decision documentation (importance ≥8)
- **Delete**: All "From Byterover..." historical references

**Commands**:
```bash
# List byterover memories (already done, found 50)
# IDs from search results above

# Review each and delete manually via local-memory CLI
# Or via MCP tool from code
```

**Expected**: 574 → 524 memories (50 deleted, 8.7% reduction)

---

### Action 2: Remove Redundant Session Summaries (2-3 hours)

**Problem**: Session summaries duplicate individual memories + git commits

**Example from Today** (2025-10-24):
```
Individual memories:
1. SPEC-KIT-069 validation complete
2. Borrow checker pattern
3. Cost crisis discovery
4. Model pricing
5. Phase 1A deployment
6. Phase 1 complete
7. Native SPEC-ID
8. Phase 1 paused

Session summary memory:
9. "Session 2025-10-24 COMPLETE" ← Duplicates #1-8!
```

**Strategy**:
- Find all memories tagged "session-summary" or "session-complete"
- Check if content is duplicate of individual memories
- Delete if redundant (keep individual memories, delete aggregate)
- Keep only exceptional session summaries (major milestones)

**Criteria for Deletion**:
- Content duplicated in individual memories? → DELETE
- Session details in git commits? → DELETE
- Nothing exceptional happened? → DELETE
- Created <30 days ago? → DELETE (still in git history)

**Expected**: 524 → 484 memories (40 deleted, 7% reduction)

---

### Action 3: Tag Consolidation Mapping (1-2 hours)

**Goal**: Map 552 tags → ~80-100 organized tags

**Tag Groups to Consolidate**:

**SPEC References** (consolidate to one format):
```
REMOVE: spec-069, SPEC-069, spec-kit-069, SPEC-KIT-069
KEEP:   spec:SPEC-KIT-069 (namespaced, standardized)
```

**Status Tags** (eliminate, use search filters instead):
```
DELETE: complete, completed, done, in-progress, blocked, resolved
Rationale: Status changes over time, not useful for long-term tags
```

**Testing Tags** (consolidate):
```
REMOVE: tests, test-coverage, testing-framework, integration-tests
KEEP:   testing (single tag covers all)
```

**Transient Tags** (delete):
```
DELETE: t84, t87, T12, t21, t18, t14, t78, t81, t86, t13
DELETE: 2025-10-14, 2025-10-21, 2025-10-18, 2025-10-13, 2025-10-12
DELETE: 52-lines-removed, policy-final-check, day-3-complete, day-3-done
Rationale: Ephemeral, not useful for retrieval
```

**Duplicate Concepts** (consolidate):
```
REMOVE: spec-plan, plan-stage, spec-plan-stage
KEEP:   stage:plan

REMOVE: spec-implement, implement-stage
KEEP:   stage:implement

REMOVE: agent-resilience, agents, agent:claude, agent:gemini
KEEP:   agent:<name> (namespaced)
```

**Expected**: 552 → ~80-100 tags (82-85% reduction)

---

### Action 4: Importance Recalibration (2-3 hours)

**Current Distribution** (estimated based on avg 7.88):
- 10: ~100 memories (too many!)
- 9: ~120 memories
- 8: ~100 memories
- 7: ~80 memories
- 6: ~60 memories
- 5: ~50 memories
- 1-4: ~64 memories

**Target Distribution** (healthier):
- 10: ~30 memories (5%, crisis events only)
- 9: ~70 memories (12%, major discoveries)
- 8: ~90 memories (16%, important milestones)
- 7: ~80 memories (14%, useful context)
- 6: ~110 memories (19%, decent findings)
- 5: ~20 memories (3%, session context)
- Deleted: ~174 low-value memories

**Recalibration Rules**:
- Session summaries: 10 → 5-6 (routine, not critical)
- Minor bug fixes: 9 → 6-7
- Config changes: 8 → 6
- Routine updates: 7 → 5
- Keep crisis events at 10 (rate limits, cost crisis)
- Keep critical patterns at 9 (borrow checker, architecture)

---

## Tag Schema Specification

### Standard Tag Format (Enforce Going Forward)

**Namespaced Tags**:
```
spec:<SPEC-ID>              SPEC-KIT-069, SPEC-KIT-070, etc.
stage:<stage>               plan, tasks, implement, validate, audit, unlock
agent:<name>                claude, gemini, gpt_pro, code
type:<category>             bug-fix, pattern, architecture, milestone, discovery
priority:<level>            critical, high, medium, low
```

**General Tags** (non-namespaced, limited set):
```
Core:        spec-kit, infrastructure, rust, documentation, debugging
Tools:       mcp, testing, consensus, evidence, telemetry
Concepts:    cost-optimization, rebase-safety, quality-gates
```

**Forbidden Tags**:
```
❌ Task IDs:       t84, T12, t21 (ephemeral)
❌ Dates:          2025-10-14 (use date filters instead)
❌ Status:         in-progress, blocked, complete (changes over time)
❌ Overly specific: 52-lines-removed, policy-final-check (not reusable)
❌ Duplicates:     tests/testing/test-coverage (consolidate to one)
```

---

## Domain Assignment Guide

**spec-kit** (~150-200 memories):
- Consensus, orchestration, automation
- Quality gates, validation, evidence
- Spec-kit commands, workflows
- Agent coordination

**infrastructure** (~50-70 memories):
- Cost optimization, performance
- Testing frameworks, CI/CD
- Architecture decisions
- Build systems, deployment

**rust** (~40-60 memories):
- Language patterns
- Borrow checker workarounds
- Performance optimizations
- Cargo, clippy, fmt

**documentation** (~30-40 memories):
- Writing strategy
- Template management
- Doc structure
- READMEs, guides

**debugging** (~30-40 memories):
- Bug fixes
- Error patterns
- Workarounds
- Troubleshooting

---

## Quick Reference: What to Delete

### Definite DELETE ✅
- [x] Byterover references (except 1-2 migration docs)
- [x] Redundant session summaries
- [x] Transient status memories
- [x] Task ID references (t##)
- [x] Overly specific tags

### Review Before DELETE ⚠️
- Session summaries with unique insights
- Old SPEC memories (might have useful patterns)
- Bug fix memories (might be referenced)

### KEEP ✅
- Architecture decisions with rationale
- Critical patterns (borrow checker)
- Crisis discoveries (rate limits)
- Major milestones with evidence
- Reusable solutions

---

## Cleanup Checklist

### Pre-Cleanup
- [ ] Export full backup: `local-memory export > backup_20251024.json`
- [ ] Capture baseline stats
- [ ] Document current state
- [ ] Review MEMORY-POLICY.md

### Day 1: Byterover Purge
- [ ] List 50 byterover memories
- [ ] Review each individually
- [ ] Delete 48, keep 1-2
- [ ] Verify searches work without them
- [ ] Document deleted IDs

### Day 2: Session Summary Dedup
- [ ] Find all session-summary memories
- [ ] Identify redundant aggregates
- [ ] Delete duplicates
- [ ] Keep exceptional summaries
- [ ] Verify no information lost

### Day 3: Tag Consolidation
- [ ] Generate tag distribution
- [ ] Create migration mapping
- [ ] Execute bulk tag updates
- [ ] Verify reduced to <100 tags
- [ ] Test searches still work

### Day 4: Importance Recalibration
- [ ] List importance ≥9 memories
- [ ] Review and downgrade inflated scores
- [ ] Verify average 5.5-6.5
- [ ] Test importance filtering

### Post-Cleanup
- [ ] Run analysis queries (should work now)
- [ ] Compare search quality (before/after)
- [ ] Document results
- [ ] Update MEMORY-POLICY.md

---

## Expected Results

### Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Total Memories | 574 | ~300 | 48% reduction |
| Unique Tags | 552 | ~90 | 84% reduction |
| Byterover Pollution | 50 | 1-2 | 96% reduction |
| Avg Importance | 7.88 | 5.5-6.5 | Proper calibration |
| Analysis Queries | BROKEN | WORKING | Fixed |
| Domains Used | 0 | 5 | Organized |
| Tag-to-Memory Ratio | 0.96 | 3-5 | Meaningful |

### Qualitative Benefits

- ✅ Searches return relevant results
- ✅ Tags actually help filter
- ✅ Analysis tools work
- ✅ System scales to 1,000+ memories
- ✅ Easier to find what you need
- ✅ Less cognitive load
- ✅ Sustainable long-term

---

## Start Here Tomorrow

1. **Read**: This file + PRD.md
2. **Decide**: SPEC-KIT-070 first or SPEC-KIT-071 first?
3. **Execute**: Follow checklist above
4. **Validate**: Measure before/after metrics
5. **Document**: Cleanup report with results

**Time Required**: 8-12 hours spread over 4 days
**Can Start**: Anytime (doesn't need GPT access)
**Benefits**: Permanent improvement in knowledge management
