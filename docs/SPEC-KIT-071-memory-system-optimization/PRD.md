# PRD: Memory System Optimization & Cleanup

**SPEC-ID**: SPEC-KIT-071
**Created**: 2025-10-24
**Status**: Draft - **HIGH PRIORITY**
**Priority**: **P1** (Infrastructure Hygiene - Blocks Efficiency)
**Owner**: Code

---

## 🔥 Executive Summary

**Current State**: Local-memory contains **574 memories** with **552 unique tags** (96% tag-to-memory ratio). System polluted with 50+ deprecated byterover references, inconsistent tagging, excessive storage, and analysis tools **breaking due to data bloat** (35,906 tokens exceeds 25k limit).

**Proposed State**: Clean, organized memory system with <300 high-value memories, ~50-100 well-structured tags, domain organization, and efficient retrieval. **Enable 70-90% reduction in memory bloat** while improving findability and usefulness.

**Impact**: Faster searches, lower token costs, better organization, scalable long-term growth.

---

## 1. Problem Statement - The Memory Crisis

### Critical Discovery: Analysis Tool Breaks

**Attempted Query**: "What are the top 10 most common types of information?"
**Result**: `ERROR: response (35,906 tokens) exceeds maximum (25,000)`

**This means**: We have SO MUCH data that even basic analysis is impossible. The system is drowning in its own memories.

---

### Issue #1: Tag Explosion (CRITICAL)

**Current State**: 552 unique tags for 574 memories

**Math**: 96% tag-to-memory ratio means tags are useless for organization
- Ideal ratio: 10-20 memories per tag (30-60 tags for 574 memories)
- Actual ratio: 1.04 memories per tag (chaos!)

**Examples of Tag Chaos**:
```
Most Common Tags (from stats):
production-testing, rust, prd, SPEC-KIT-070, sandbox, cleanup, plan-stage, blocker,
integration-testing, t84, doc-audit, guardrails, resolved, config-validation, handoff,
2025-10-21, spec-plan, HAL, accessibility, refactoring, stage:validate, rebase-strategy,
t21, gemini, schema-v1, t78, aggregator, agent:claude, security, followup, policy-prefilter,
agent-resilience, SPEC-KIT-067, codex, policy-final, spec-kit-tasks, new-spec,
slash-commands, t18, rebase, shell-lite, spec-validate, acceptance-criteria, SPEC.md,
session-complete, tasks, tasks-stage, production-ready, native-tools, telemetry, roster,
stage:unlock, t13, SPEC-KIT-DEMO, gpt, testing-framework, consensus-evidence, mock-hal,
byterover-sync, hooks, session-summary, local-memory, spec-md, cost-optimization,
environment, SPEC-KIT-030, implementation, checklist, testing, 2025-10-24, workflow,
policy-layer, hal-summary, tasks-table, landlock, stage:quality-gate-checklist, spec-kit,
final-check, SPEC-OPS-004, docker, t87, doc-10, tests, policy, fork-maintenance,
stage:audit, t14, 52-lines-removed, plan-review, 2025-10-14, requirements,
architecture-review, phase-4, mcp, integration-tests, test-coverage, rebase-safety, PRD,
stage:tasks, T12, chatwidget, consensus-metadata, speckit-namespace, evidence, webhooks,
isolation, spec-066, restart-plan, arbiter, SPEC-KIT-035...
(continues for 300+ more tags)
```

**Problems**:
- Inconsistent naming: "SPEC-KIT-069" vs "spec:SPEC-KIT-069" vs "spec-069" vs "spec-kit-069"
- Redundant tags: "session-summary" AND "session-complete" AND "complete"
- Overly specific: "52-lines-removed", "policy-final-check", "2025-10-14"
- Task IDs as tags: "t84", "t87", "T12" (ephemeral, not useful)
- Duplicate concepts: "testing" AND "test-coverage" AND "tests" AND "testing-framework"

**Impact**:
- Tags don't help find related content
- Search results are noisy
- Can't filter effectively
- New memories can't find good tags to use

---

### Issue #2: Byterover Pollution (HIGH)

**Finding**: 50+ memories reference deprecated byterover system

**Byterover was deprecated**: 2025-10-18 (MEMORY-POLICY.md)
**Current date**: 2025-10-24 (6 days later)
**Pollution**: 8.7% of all memories (50/574)

**Examples**:
- "From Byterover memory layer: ..." (historical references)
- "byterover" tag on old consensus artifacts
- "byterover-sync", "byterover-mirror", "byterover-gap"
- Memories about byterover migration itself

**Impact**:
- Clutters search results
- Confuses about which system to use
- Historical cruft that's no longer relevant
- Wastes storage and search time

**Solution**:
- Archive or delete byterover-tagged memories
- Keep only migration documentation (1-2 memories)
- Clean up references in remaining memories

---

### Issue #3: Excessive Session Summaries

**Current Practice**: Store session summary after every session
**Result**: Lots of session-summary memories

**Problem**: Session summaries are **redundant with**:
- Git commits (already capture what was done)
- SPEC.md (tracks progress)
- Evidence files (detailed artifacts)
- Individual memories from session (store key decisions already)

**Example Redundancy**:
Today alone (2025-10-24) we stored:
1. SPEC-KIT-069 validation completion
2. Borrow checker pattern
3. SPEC-KIT-070 cost crisis
4. Model pricing comparison
5. Phase 1A deployment
6. Phase 1 complete
7. Native SPEC-ID implementation
8. Phase 1 paused handoff
9. Session complete summary

**Memory #9 duplicates #1-8!** The session summary just aggregates what's already stored individually.

**Waste**: 9 memories when 7-8 would suffice (eliminate aggregate session summary)

---

### Issue #4: High Average Importance (MEDIUM)

**Stat**: Average importance 7.88/10

**What this means**: We're marking almost everything as highly important
- Range should be 1-10 with average ~5
- Actual average 7.88 means we overuse 7-10 range
- Inflation: If everything is important (7+), nothing is

**Impact**:
- Can't filter by importance effectively
- Critical memories don't stand out
- Query results don't prioritize well

**Examples of Over-Importance**:
- Session summaries: importance 10 (really? Every session is critical?)
- Minor bug fixes: importance 9
- Small config changes: importance 8

**Calibration Needed**:
- 10: Critical architecture decisions, major discoveries, crisis events (5-10% of memories)
- 8-9: Important milestones, significant patterns, reusable solutions (15-20%)
- 6-7: Useful context, decent findings, reference material (30-40%)
- 4-5: Minor details, transient notes, session context (30-40%)
- 1-3: Rarely used, could archive (5-10%)

---

### Issue #5: No Domain Organization (MEDIUM)

**Stat**: Domains list returned `null`

**What this means**: We're not using the domain feature at all!

**Lost Opportunity**: Domains provide hierarchical organization
- `spec-kit` domain for all spec-kit work
- `infrastructure` domain for cost, testing, architecture
- `documentation` domain for doc work
- `rust` domain for language-specific patterns

**Current Workaround**: Trying to use tags for domains ("spec-kit", "rust", "infrastructure")
- Wastes tag namespace
- Less structured than proper domains
- Harder to query by domain

---

### Issue #6: Inconsistent Tag Naming (MEDIUM)

**Patterns Found**:

**SPEC References** (4 different formats!):
- `SPEC-KIT-069` (uppercase, full)
- `spec:SPEC-KIT-069` (namespaced)
- `spec-069` (abbreviated)
- `spec-kit-069` (lowercased)

**Dates** (3 formats):
- `2025-10-24` (ISO format)
- `session-2025-10-24` (prefixed)
- `20251024` (compact)

**Status** (5 formats!):
- `complete`
- `completed`
- `session-complete`
- `done`
- `validation-complete`

**Impact**:
- Same concept has multiple tags
- Can't find all related memories
- Duplicate tag maintenance

---

### Issue #7: Over-Storage of Transient Info (MEDIUM)

**What we store**: Everything, including:
- Session summaries (redundant with git commits)
- Task IDs (t84, t87) that are ephemeral
- Specific dates (2025-10-14) that don't add value
- Status updates ("in progress", "blocked") that become stale
- Minor config tweaks that aren't reusable

**What we should store**: Only high-value, reusable knowledge:
- Architectural decisions and rationale
- Critical bug fixes and workarounds
- Reusable patterns and code examples
- Major milestones with outcomes
- Important discoveries

**Storage Criteria** (proposed):
- ✅ Will this be useful in 30+ days?
- ✅ Is this reusable knowledge?
- ✅ Does this explain WHY, not just WHAT?
- ✅ Is this unique (not in docs/code)?
- ❌ Is this already in git/SPEC.md/evidence?

---

### Issue #8: Search Performance Unknown (LOW)

**With 574 memories and 552 tags**: Search performance degrades
- Semantic search across 574 documents
- Tag filtering across 552 tag values
- Analysis queries break (35k+ tokens)

**No Metrics On**:
- Query latency (how long does search take?)
- Index size (how much disk/memory?)
- Retrieval accuracy (do we find what we need?)

**Impact**: Unknown if this will scale to 1,000+ memories

---

## 2. Proposed Solution - The Great Memory Cleanup

### Phase 1: Immediate Cleanup (Week 1)

**Goal**: Reduce 574 memories → ~300 high-value memories (48% reduction)

#### Cleanup Task 1.1: Purge Byterover Pollution (50+ memories)

**Action**: Delete or archive all byterover-tagged memories
**Keep**: 1-2 memories about migration decision (importance 8+)
**Delete**: ~48 memories (all the "From Byterover..." historical cruft)

**Savings**: 50/574 = 8.7% immediate reduction

#### Cleanup Task 1.2: Deduplicate Session Summaries (30+ memories)

**Current**: Every session stores aggregate summary
**Redundancy**: Session summaries duplicate individual memories from same session

**Action**:
- Keep individual high-value memories (architecture decisions, discoveries)
- Delete aggregate session summaries (redundant with git commits + SPEC.md)
- Keep only exceptional session summaries (major milestones)

**Criteria**: Delete session summary if:
- Content is duplicated in individual memories
- Session details already in git commits
- Nothing exceptional happened
- Created <30 days ago (still in recent git history)

**Savings**: ~30-40 memories

#### Cleanup Task 1.3: Remove Transient Tags (200+ tags)

**Delete Tag Types**:
- Task IDs: t84, t87, T12, etc. (~50 tags)
- Specific dates: 2025-10-14, 2025-10-21, etc. (~30 tags)
- Status tags: in-progress, blocked, resolved, complete, done (~10 tags)
- Overly specific: 52-lines-removed, policy-final-check, etc. (~50 tags)
- Duplicates: testing/tests/test-coverage, complete/completed/done (~30 tags)

**Keep Tag Types**:
- SPECs: spec:SPEC-KIT-### (standardized format)
- Domains: spec-kit, infrastructure, rust, documentation
- Stages: stage:plan, stage:implement, etc.
- Agents: agent:claude, agent:gemini, etc.
- Categories: bug-fix, pattern, architecture, milestone

**Result**: 552 tags → ~80-100 well-organized tags (85% reduction)

#### Cleanup Task 1.4: Recalibrate Importance (400+ memories)

**Current**: Average 7.88 (too high)
**Target**: Average 5.5-6.0 (properly distributed)

**Recalibration**:
- Downgrade session summaries: 10 → 5-6
- Downgrade minor fixes: 9 → 6-7
- Downgrade routine updates: 8 → 5-6
- Keep crisis discoveries: 10 (rate limits, cost crisis)
- Keep critical patterns: 9 (borrow checker, architecture)

**Action**: Bulk update ~200-300 memories to lower importance

---

### Phase 2: Reorganization (Week 2)

**Goal**: Organize remaining ~300 memories for optimal retrieval

#### Task 2.1: Implement Domain Structure

**Create Domains**:
- `spec-kit`: Spec-kit automation, consensus, quality gates (~150 memories)
- `infrastructure`: Cost, testing, CI/CD, architecture (~50 memories)
- `rust`: Language patterns, borrow checker, performance (~40 memories)
- `documentation`: Doc strategy, templates, writing (~30 memories)
- `debugging`: Bug fixes, workarounds, error patterns (~30 memories)

**Migrate**: Assign domain to each memory based on content

#### Task 2.2: Standardize Tag Schema

**Namespaced Tags** (enforce consistency):
```
spec:SPEC-KIT-###     (SPEC references, ~50 tags)
stage:<name>          (plan, tasks, implement, validate, audit, unlock)
agent:<name>          (claude, gemini, gpt_pro, code)
type:<category>       (bug-fix, pattern, architecture, milestone, discovery)
priority:<level>      (critical, high, medium, low)
status:<state>        (active, archived, superseded)
```

**Eliminate**: All non-namespaced ephemeral tags

#### Task 2.3: Create Memory Categories

**Use local-memory categories feature** (currently unused):
- `critical-decisions`: Architecture, security, compliance (~20 memories)
- `reusable-patterns`: Code patterns, best practices (~50 memories)
- `spec-kit-system`: Consensus, automation, workflows (~100 memories)
- `bugs-and-fixes`: Important bug discoveries (~40 memories)
- `session-context`: Recent work, transient (~50 memories, auto-archive after 30d)
- `historical`: Completed SPECs, old decisions (~40 memories)

---

### Phase 3: Ongoing Maintenance (Continuous)

**Goal**: Keep memory system healthy long-term

#### Task 3.1: Storage Policy (Prevent Re-Bloat)

**New Storage Rules** (enforce before storing):

**DO Store** (importance ≥7):
- Architectural decisions with rationale
- Critical bug fixes and workarounds
- Reusable patterns and code examples
- Major milestones with outcomes
- Important discoveries (rate limits, cost crisis)
- Non-obvious solutions to complex problems

**DON'T Store** (use git/docs instead):
- Session summaries (redundant with git commits)
- Task progress updates (use SPEC.md)
- Transient status ("in progress", "blocked")
- Information already in documentation
- Minor config changes
- Routine operations

**Importance Calibration**:
- 10: Crisis events, critical architecture (use sparingly, <5% of stores)
- 9: Major discoveries, significant patterns (10-15%)
- 8: Important milestones, reusable solutions (15-20%)
- 7: Useful context, good reference (20-30%)
- 6: Decent findings, minor patterns (25-35%)
- 5: Nice-to-have context (10-15%)
- 1-4: Rarely, consider not storing (<5%)

#### Task 3.2: Auto-Archival Strategy

**Lifecycle Management**:
```
Active (0-30 days):    All memories searchable
Archived (30-90 days): Moved to archive category, lower importance -2
Purged (90+ days):     Delete if importance <6, archive if ≥6
```

**Categories Auto-Transition**:
- `session-context` → archive after 30 days
- `spec:SPEC-KIT-###` → archive after SPEC completed + 30 days
- `bug-fix` → archive after code merged + 60 days
- `critical-decisions` → never archive (permanent)

#### Task 3.3: Periodic Cleanup Tasks

**Monthly Maintenance** (~1 hour):
- Run memory stats, review growth
- Identify redundant/duplicate memories
- Recalibrate importance if average drifts
- Consolidate similar memories
- Archive old session context

**Quarterly Maintenance** (~2-3 hours):
- Deep cleanup: Review all memories
- Update outdated information
- Merge duplicate patterns
- Validate tag usage distribution
- Reorganize domains if needed

---

## 3. Acceptance Criteria

### Phase 1 Cleanup Success

- ✅ Total memories: 574 → <300 (48%+ reduction)
- ✅ Unique tags: 552 → <100 (82% reduction)
- ✅ Byterover memories: 50 → 1-2 (96% reduction)
- ✅ Average importance: 7.88 → 5.5-6.5 (proper distribution)
- ✅ Analysis queries work (response <25k tokens)
- ✅ All domains populated (5 domains, ~60 memories each)

### Phase 2 Reorganization Success

- ✅ Tag schema standardized (namespaced tags only)
- ✅ Domain assignment: 100% of memories have domain
- ✅ Categories used: 6 categories, balanced distribution
- ✅ Tag-to-memory ratio: 5-10 memories per tag (healthy)
- ✅ Search performance: Queries return in <2 seconds

### Phase 3 Ongoing Success

- ✅ Storage policy enforced (documented in MEMORY-POLICY.md)
- ✅ Monthly cleanup automated (script or checklist)
- ✅ Growth rate: <20 new memories/month (sustainable)
- ✅ Quality maintained: High-value content only

---

## 4. Technical Implementation

### Cleanup Script Design

```bash
#!/bin/bash
# memory_cleanup.sh - SPEC-KIT-071 Phase 1

# 1. List all byterover memories
local-memory search --tags byterover --format ids --limit 100 > byterover_ids.txt

# 2. Review and selectively delete
while read -r id; do
    # Show content
    local-memory get "$id"

    # Prompt for action
    read -p "Delete this memory? [y/N] " response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        local-memory delete "$id"
        echo "Deleted: $id"
    fi
done < byterover_ids.txt

# 3. Tag consolidation (manual review needed)
local-memory stats --tags > tag_distribution.txt

# 4. Importance recalibration
local-memory search --importance-min 9 --limit 100 --format detailed > high_importance.json
# Review and update importance for each

# 5. Generate cleanup report
cat <<EOF > cleanup_report.md
# Memory Cleanup Report

Before:
- Memories: 574
- Tags: 552
- Byterover: 50+

After:
- Memories: <count>
- Tags: <count>
- Byterover: 1-2

Deleted: <count> memories
Recalibrated: <count> importance values
Tags removed: <count>
EOF
```

### Domain Migration Script

```bash
#!/bin/bash
# assign_domains.sh - SPEC-KIT-071 Phase 2

# Spec-kit domain
local-memory search --tags spec-kit --limit 200 --format ids | while read id; do
    local-memory update "$id" --domain spec-kit
done

# Infrastructure domain
local-memory search --tags infrastructure,cost-optimization,testing --limit 100 --format ids | while read id; do
    local-memory update "$id" --domain infrastructure
done

# Rust domain
local-memory search --tags rust,borrow-checker,pattern --limit 100 --format ids | while read id; do
    local-memory update "$id" --domain rust
done

# Documentation domain
local-memory search --tags documentation,docs,writing --limit 50 --format ids | while read id; do
    local-memory update "$id" --domain documentation
done

# Debugging domain
local-memory search --tags bug-fix,debugging,error --limit 50 --format ids | while read id; do
    local-memory update "$id" --domain debugging
done
```

### Tag Standardization

**Bulk Tag Updates** (via script or manual):
```bash
# Standardize SPEC references
local-memory search --tags spec-069,SPEC-069,spec-kit-069 | \
    xargs -I {} local-memory update {} --remove-tags spec-069,SPEC-069,spec-kit-069 --add-tags spec:SPEC-KIT-069

# Consolidate status tags
local-memory search --tags complete,completed,done | \
    xargs -I {} local-memory update {} --remove-tags complete,done --add-tags status:completed

# Consolidate testing tags
local-memory search --tags tests,test-coverage,testing-framework | \
    xargs -I {} local-memory update {} --remove-tags tests,testing-framework --add-tags testing

# Remove transient tags
local-memory search --tags t84,t87,T12 | \
    xargs -I {} local-memory update {} --remove-tags t84,t87,T12

# Remove specific dates (keep date ranges in content if needed)
local-memory search --tags 2025-10-14,2025-10-21 | \
    xargs -I {} local-memory update {} --remove-tags 2025-10-14,2025-10-21
```

---

## 5. Storage Policy Updates

### Update MEMORY-POLICY.md

**Add New Sections**:

#### Storage Guidelines

**When to Store**:
- Decisions that explain WHY (not just WHAT was done)
- Patterns that will be reused (borrow checker, architecture)
- Discoveries that change approach (rate limits, cost crisis)
- Milestones with evidence (Phase 1 complete, tests passing)
- Critical fixes with context (why it was hard, how we solved it)

**When NOT to Store**:
- Progress updates (use SPEC.md instead)
- Session summaries (git commits cover this)
- Information in documentation (link to docs instead)
- Transient status (in-progress, blocked)
- Routine operations (normal workflow)

#### Tag Schema Standard

**Required Tag Format**:
```
spec:<SPEC-ID>          Required for SPEC-related memories
stage:<stage-name>      For stage-specific (plan, tasks, implement, etc.)
agent:<agent-name>      For agent-specific
type:<category>         For classification (bug-fix, pattern, milestone, etc.)
domain:<area>           Use domain field, not tag
```

**Forbidden Tag Patterns**:
- Task IDs (t##, T##)
- Specific dates (2025-MM-DD) - use date range filters instead
- Status values (in-progress, blocked, done)
- Overly specific (52-lines-removed, policy-final-check)

#### Importance Calibration

**Scoring Guide**:
- **10**: Crisis events, critical architecture (examples: rate limit discovery, cost crisis, system-breaking bugs)
- **9**: Major discoveries, architectural patterns (examples: native > AI for deterministic, borrow checker patterns)
- **8**: Significant milestones, important solutions (examples: Phase 1 complete, major refactor complete)
- **7**: Useful context, good reference (examples: config changes, test additions)
- **6**: Decent findings, minor patterns (examples: small optimizations, clarifications)
- **5**: Nice-to-have context (examples: session notes, minor discoveries)
- **1-4**: Rarely used (examples: transient notes, low-value observations)

**Target Distribution**:
- 10: 5-10% of memories
- 9: 10-15%
- 8: 15-20%
- 7: 20-30%
- 6: 25-35%
- 5: 10-20%
- 1-4: <10%

---

## 6. Migration Plan

### Week 1: Cleanup (8-12 hours)

**Day 1: Byterover Purge** (2-3 hours)
- Export list of 50 byterover memories
- Review each (keep 1-2, delete rest)
- Verify deletion
- Test searches work without them

**Day 2: Session Summary Dedup** (2-3 hours)
- List all session-summary memories
- Identify duplicates with individual memories
- Delete redundant aggregates
- Keep exceptional summaries only

**Day 3: Tag Consolidation** (3-4 hours)
- Generate tag distribution report
- Identify redundant tag groups
- Create tag migration mapping
- Execute bulk tag updates
- Verify tag count reduced to <100

**Day 4: Importance Recalibration** (2-3 hours)
- Export high-importance memories (≥9)
- Review and downgrade inflated scores
- Verify new average 5.5-6.5
- Test importance-based filtering works

**Validation**:
- Memory count <300
- Tag count <100
- Average importance 5.5-6.5
- Analysis queries work (<25k tokens)
- Byterover memories 1-2
- Search quality maintained or improved

---

### Week 2: Reorganization (6-8 hours)

**Day 5-6: Domain Assignment** (3-4 hours)
- Create 5 domains (spec-kit, infrastructure, rust, docs, debugging)
- Migrate memories to appropriate domains
- Verify distribution balanced
- Test domain-filtered searches

**Day 7: Category Assignment** (2-3 hours)
- Create 6 categories (critical-decisions, reusable-patterns, etc.)
- Assign categories to memories
- Test category-based retrieval

**Day 8: Tag Standardization** (2-3 hours)
- Enforce namespaced tag schema
- Bulk update to standard format
- Remove all non-compliant tags
- Document tag schema in MEMORY-POLICY.md

---

### Ongoing: Maintenance (1-2 hours/month)

**Monthly Tasks**:
- Review memory growth (should be <20/month)
- Archive old session-context memories (30+ days)
- Check tag proliferation (flag if >120 tags)
- Recalibrate importance if average drifts
- Run cleanup report

**Quarterly Tasks**:
- Deep review of all memories
- Merge duplicates
- Update outdated information
- Reorganize if needed
- Validate storage policy compliance

---

## 7. Success Metrics

### Immediate (Phase 1)

| Metric | Before | Target | Reduction |
|--------|--------|--------|-----------|
| Total Memories | 574 | <300 | 48%+ |
| Unique Tags | 552 | <100 | 82% |
| Byterover Pollution | 50+ | 1-2 | 96% |
| Avg Importance | 7.88 | 5.5-6.5 | Proper calibration |
| Analysis Queries | BROKEN | WORKING | Fixed |

### Medium-term (Phase 2)

| Metric | Before | Target | Result |
|--------|--------|--------|--------|
| Domains Used | 0 | 5 | 100% coverage |
| Categories Used | 0 | 6 | Organized |
| Tag-to-Memory Ratio | 0.96 | 5-10 | Meaningful tags |
| Search Precision | Unknown | High | Validated |

### Long-term (Phase 3)

| Metric | Target | Monitoring |
|--------|--------|------------|
| Monthly Growth | <20 memories | Track trend |
| Tag Proliferation | <120 total | Alert if exceeded |
| Importance Avg | 5.5-6.5 | Recalibrate quarterly |
| Archive Rate | 30-day lifecycle | Auto-archive |

---

## 8. Risk Analysis

### Risk 1: Deleting Valuable Information

**Likelihood**: MEDIUM
**Impact**: HIGH
**Mitigation**:
- Export backup before bulk deletion
- Manual review of each byterover memory
- Keep anything referenced in active SPECs
- Archive instead of delete (soft delete)

### Risk 2: Breaking Existing Queries

**Likelihood**: LOW
**Impact**: MEDIUM
**Mitigation**:
- Test searches after tag changes
- Document tag migrations
- Keep old tags temporarily during transition
- Gradual rollout

### Risk 3: Time Investment

**Likelihood**: CERTAIN
**Impact**: MEDIUM
**Mitigation**:
- Phase approach spreads work over 2 weeks
- Automate where possible (scripts)
- Prioritize high-impact cleanup (byterover, tags)
- Can pause between phases

---

## 9. Cost/Benefit Analysis

### Costs

**Time Investment**:
- Phase 1 Cleanup: 8-12 hours
- Phase 2 Reorganization: 6-8 hours
- Documentation: 2-3 hours
- **Total: 16-23 hours**

**Risk**:
- Might accidentally delete something useful
- Queries might break temporarily
- Learning curve for new schema

### Benefits

**Immediate**:
- Analysis queries work again (unblocked)
- 48% less memory bloat (faster searches)
- 82% fewer tags (findability improves)
- No more byterover confusion (clarity)

**Medium-term**:
- Domain organization (structured knowledge)
- Better search precision (find what you need)
- Scalable system (can grow to 1,000+ memories)
- Lower maintenance burden

**Long-term**:
- Sustainable growth (clear policies)
- Higher value per memory (quality over quantity)
- Better knowledge reuse (properly organized)
- Reduced cognitive load (less noise)

**ROI**: 20 hours investment → permanent improvement in knowledge management

---

## 10. Open Questions

1. **Deletion vs Archival**: Soft delete (move to archive) or hard delete?
   - **Recommendation**: Soft delete with 90-day purge

2. **Auto-archival**: Automated or manual?
   - **Recommendation**: Manual for Phase 1, automated for Phase 3

3. **Tag enforcement**: How to prevent re-proliferation?
   - **Recommendation**: Update MEMORY-POLICY.md, code review

4. **Domain granularity**: 5 domains enough or need more?
   - **Recommendation**: Start with 5, expand if needed

5. **Category auto-assignment**: Use AI to categorize?
   - **Recommendation**: Manual for Phase 2, consider AI for future

---

## 11. Comparison to SPEC-KIT-070

### Similarities

Both are **infrastructure hygiene** issues:
- SPEC-KIT-070: Model cost bloat ($11/run)
- SPEC-KIT-071: Memory data bloat (574 memories, 552 tags)

Both have **similar solutions**:
- SPEC-KIT-070: Eliminate waste through smart routing
- SPEC-KIT-071: Eliminate waste through cleanup + policy

Both are **P1 priority** (block efficiency):
- SPEC-KIT-070: Blocks cost sustainability
- SPEC-KIT-071: Blocks memory scalability

### Differences

**SPEC-KIT-070**: Ongoing cost (every run costs money)
- **Urgency**: CRITICAL (bleeding money daily)
- **Impact**: $6,500/year savings

**SPEC-KIT-071**: One-time cleanup + ongoing policy
- **Urgency**: HIGH (degrading over time, not immediate crisis)
- **Impact**: Efficiency gain, enabler for scaling

### Priority Ordering

**Current**: SPEC-KIT-070 > SPEC-KIT-071
**Rationale**: Cost crisis burns money daily, memory bloat degrades slowly

**But**: SPEC-KIT-071 is **good use of 24-hour GPT downtime**!
- Can execute cleanup without TUI access
- Doesn't depend on OpenAI APIs
- Prepares system for Phase 2 cost tracking (needs clean memory)
- Sets foundation for long-term sustainability

---

## 12. Implementation Strategy

### This Session (While GPT Blocked)

**Can Do Without TUI** (4-6 hours):
- ✅ Analyze current state (done)
- ✅ Create comprehensive PRD (this document)
- ⏸️ Review byterover memories for deletion
- ⏸️ Create tag migration mapping
- ⏸️ Draft updated MEMORY-POLICY.md
- ⏸️ Create cleanup scripts

**Cannot Do** (needs TUI):
- Bulk operations (local-memory CLI may not support bulk updates)
- Test search performance after cleanup
- Validate improved findability

### Next Session (After GPT Access Returns)

**Decision**: Validate SPEC-KIT-070 first OR cleanup memory first?

**Option A**: SPEC-KIT-070 validation first (4-6 hours)
- Pro: Cost optimization more urgent
- Con: Leaves memory bloated

**Option B**: SPEC-KIT-071 cleanup first (8-12 hours)
- Pro: Cleans system before more memories added
- Pro: Good use of focus time for tedious work
- Con: Delays cost validation

**Option C**: Parallel (mix both)
- Morning: SPEC-KIT-070 validation (4h)
- Afternoon: SPEC-KIT-071 cleanup (4h)

**Recommendation**: **Option A** (cost first, memory second)
- Cost crisis more urgent
- Memory cleanup can happen anytime
- SPEC-KIT-070 Phase 2 might generate insights to store (want clean system first)

---

## 13. Deliverables

### Phase 1 Deliverables

- [ ] Cleanup report (before/after stats)
- [ ] Deleted memory list (audit trail)
- [ ] Tag migration mapping
- [ ] Updated MEMORY-POLICY.md
- [ ] Cleanup scripts (byterover purge, tag consolidation)

### Phase 2 Deliverables

- [ ] Domain assignment report
- [ ] Category structure documentation
- [ ] Tag schema specification
- [ ] Search performance benchmarks
- [ ] Reorganization validation report

### Phase 3 Deliverables

- [ ] Monthly maintenance checklist
- [ ] Quarterly cleanup procedure
- [ ] Auto-archival script (if automated)
- [ ] Memory growth dashboard
- [ ] Policy compliance monitoring

---

## 14. Validation Plan

### Pre-Cleanup Baseline

**Capture Current State**:
```bash
# Stats
local-memory stats > baseline_stats.txt

# Tag distribution
local-memory stats --tags > baseline_tags.txt

# Sample queries for comparison
local-memory search "consensus" --limit 20 > baseline_consensus.txt
local-memory search "bug fix" --limit 20 > baseline_bugfix.txt

# Export full backup
local-memory export > memory_backup_$(date +%Y%m%d).json
```

### Post-Cleanup Validation

**Verify Improvement**:
```bash
# Stats comparison
local-memory stats > post_cleanup_stats.txt
diff baseline_stats.txt post_cleanup_stats.txt

# Tag reduction
local-memory stats --tags > post_cleanup_tags.txt
# Verify <100 unique tags

# Query quality
local-memory search "consensus" --limit 20 > post_cleanup_consensus.txt
# Compare precision/recall vs baseline

# Analysis queries work
local-memory analysis --type summarize --timeframe month
# Should not exceed 25k tokens
```

**Success Criteria**:
- Memory count reduced 40-60%
- Tag count reduced 80-90%
- Query quality maintained or improved
- Analysis queries work
- Search feels faster (subjective)

---

## 15. Conclusion

**Memory System is Bloated and Chaotic**:
- 574 memories (too many)
- 552 tags (completely defeats purpose)
- 50+ deprecated byterover memories (9% pollution)
- Analysis tools break (35k token overflow)
- No organization (domains unused, tags chaos)

**Cleanup is Essential**:
- Blocks scalability (can't grow indefinitely)
- Degrades findability (tag chaos = noise)
- Wastes storage (redundant, outdated content)
- Impacts performance (large dataset, inefficient queries)

**Solution is Straightforward**:
- Delete byterover pollution (50 memories)
- Remove redundant session summaries (30-40 memories)
- Consolidate 552 tags → 80-100 organized tags
- Recalibrate importance (average 7.88 → 5.5-6.5)
- Implement domains and categories
- Establish ongoing policy

**Expected Outcome**:
- Cleaner system (300 high-value memories)
- Better organized (5 domains, 6 categories, <100 tags)
- Scalable growth (policies prevent re-bloat)
- Faster searches (less noise, better structure)
- Higher value (quality over quantity)

**Priority**: **P1 HIGH** - Not as urgent as SPEC-KIT-070 (cost crisis), but important for long-term health and good use of GPT downtime for cleanup work.

**Effort**: 16-23 hours over 2 weeks, mostly manual review (can't fully automate without risking deletion of valuable content)
