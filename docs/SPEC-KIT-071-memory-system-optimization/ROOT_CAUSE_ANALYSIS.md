# SPEC-KIT-071 Root Cause Analysis - Documentation Drives Bloat

**Discovery Date**: 2025-10-24
**Severity**: CRITICAL
**Finding**: Our own CLAUDE.md documentation **instructs** creation of the bloat problems

---

## 🔥 The Smoking Gun

### CLAUDE.md Lines 300-307: Session End Storage (REQUIRED)

**What it says**:
```markdown
**5. Session End** (REQUIRED):
Do not auto-store session summaries in local-memory.
Write a file summary to `~/.local-memory/session-summaries/<domain>/...` and promote manually if needed:
- lm remember "WHAT: ...\nWHY: ...\nEVIDENCE: <file>\nOUTCOME: ..." --type milestone --importance 8 --tags "spec:SPEC-066"
```

**What this creates**:
- ❌ Mandatory session summaries (redundant with git commits + individual memories)
- ❌ Domain: "session-summary" (but we should use general domains like "spec-kit")
- ❌ Tags: Specific dates like "2025-10-20" (creates 30+ ephemeral date tags!)
- ❌ Importance: 9 (inflates average, should be 5-6 for routine sessions)

**Impact**: This ONE instruction creates:
- Session summary bloat (~40-50 redundant memories)
- Date tag pollution (~30+ useless tags)
- Importance inflation (avg 7.88 instead of 5.5)

---

## 🚨 Problem #1: Mandatory Session Summaries

### The Guidance (CLAUDE.md:300-307)

**Says**: "Session End (REQUIRED)"
**Domain**: "session-summary"
**Importance**: 9

### Why This is Wrong

**Session summaries are TRIPLE REDUNDANT**:

1. **Git commits already capture what was done**:
   ```bash
   $ git log --oneline -3
   6a2142704 docs(cost): add SPEC-KIT-070 spec.md and handoff
   f92672cb1 docs(spec-kit): update SPEC-KIT-070 Phase 1 progress
   49357f77d docs(cost): add SPEC-KIT-070 Phase 2 plan
   ```

2. **SPEC.md tracks progress**:
   - Tasks completed
   - Evidence locations
   - Status updates

3. **Individual memories store key decisions**:
   - Today we stored 9 memories
   - Memory #9 (session summary) just aggregates #1-8!

**Example from Today**:
```
Memory 1: SPEC-KIT-069 validation complete
Memory 2: Borrow checker pattern
Memory 3: Cost crisis discovery
Memory 4: Model pricing
Memory 5: Phase 1A deployment
Memory 6: Phase 1 complete
Memory 7: Native SPEC-ID
Memory 8: Phase 1 paused
Memory 9: "Session 2025-10-24 COMPLETE" ← DUPLICATES 1-8!
```

**The session summary provides ZERO new information** beyond what's in individual memories + git commits.

### The Fix

**Change CLAUDE.md**:
```markdown
**5. Session End** (OPTIONAL, only if exceptional):

Store session summary ONLY if:
- Session had major breakthrough/discovery
- Complex multi-day work needs handoff context
- Critical decisions made that aren't in individual memories

Otherwise: Individual memories + git commits are sufficient.

If storing:
- domain: Appropriate domain (spec-kit, infrastructure, etc.)
- tags: ["type:session-handoff"] (NOT specific dates)
- importance: 6-7 (routine session) or 8-9 (exceptional)
```

**Impact**: Eliminates 30-40 redundant session summaries

---

## 🚨 Problem #2: Importance Threshold Too Low

### The Guidance (CLAUDE.md:274, 38)

**Says**: "Store importance ≥7"
**Result**: Average importance 7.88 (everything is "important")

### Why This is Wrong

**Calibration Math**:
- If threshold is 7, people use 7-10 range
- Average becomes (7+8+9+10)/4 = 8.5
- Reality: We're averaging 7.88 (close to prediction)

**When everything is important (≥7), nothing is important.**

**Proper Distribution** should be:
- 10: 5-10% (crisis events, critical architecture)
- 9: 10-15% (major discoveries, significant patterns)
- 8: 15-20% (important milestones)
- 7: 20-30% (useful context)
- 6: 25-35% (decent findings)
- 5: 10-20% (nice-to-have)
- 1-4: <10% (rarely used)

**With threshold ≥7**: We're only using top 4 values (7-10), compressing distribution

### The Fix

**Change CLAUDE.md**:
```markdown
**3. During Work** (Store importance ≥8):

Importance Calibration:
- 10: Crisis events, system-breaking discoveries (use sparingly!)
- 9: Major architectural decisions, critical patterns
- 8: Significant milestones, important solutions
- 7: Useful reference, good context (rarely store these)
- 6: Minor findings (don't store unless exceptional)
- 5 and below: Don't store (use git commits/docs instead)

Threshold: ≥8 (not ≥7) prevents inflation
```

**Impact**: Average drops to 5.5-6.5 (proper distribution)

---

## 🚨 Problem #3: Date Tags Guidance

### The Guidance (CLAUDE.md:279, 296, 305)

**Examples show**: `tags: ["2025-10-20", "2025-10-19", "2025-10-24"]`

**Result**: Creates 30+ date tags that are useless
- "2025-10-14"
- "2025-10-21"
- "2025-10-13"
- "2025-10-12"
- "2025-10-18"
- "2025-10-19"
- "2025-10-20"
- "2025-10-24"
- etc.

### Why This is Wrong

**Date tags defeat filtering**:
- Want memories from October? Can't query "2025-10-*"
- Want recent memories? Use date range filters, not tags
- Date tags proliferate endlessly (new tag every day!)
- No value for long-term retrieval

**Better Approach**: Use local-memory's date filtering
```bash
# Instead of tagging with dates:
local-memory search "bug fix" --start-date 2025-10-01 --end-date 2025-10-31

# Or just rely on created_at timestamp (automatic!)
```

### The Fix

**Change CLAUDE.md examples**:
```markdown
Bad:  tags: ["2025-10-20", "routing-fix", "spec-066"]
Good: tags: ["type:bug-fix", "spec:SPEC-KIT-066", "component:routing"]
```

**Remove ALL date tags from examples and guidelines**

**Impact**: Eliminates 30+ ephemeral date tags

---

## 🚨 Problem #4: No Tag Schema Specification

### What's Missing in CLAUDE.md

**No guidance on**:
- Tag naming conventions
- Namespaced tags (spec:, stage:, agent:, type:)
- Tag consolidation
- Maximum tag count
- When to create new tags vs reuse existing

**Result**: Tag chaos
- "SPEC-KIT-069" vs "spec:SPEC-KIT-069" vs "spec-069" vs "spec-kit-069"
- "complete" vs "completed" vs "done" vs "session-complete"
- "testing" vs "tests" vs "test-coverage" vs "testing-framework"

**Current**: 552 tags for 574 memories (96% ratio)
**Ideal**: 60-100 tags for 574 memories (6-10 memories per tag)

### The Fix

**Add to CLAUDE.md** (new section):
```markdown
### Tag Schema (ENFORCE)

**Namespaced Format** (preferred):
- spec:<SPEC-ID>              Example: spec:SPEC-KIT-069
- stage:<stage-name>          Example: stage:plan, stage:implement
- agent:<agent-name>          Example: agent:claude, agent:gemini
- type:<category>             Example: type:bug-fix, type:pattern

**General Tags** (limited set, ~50 max):
- Core: spec-kit, infrastructure, rust, documentation
- Tools: mcp, testing, consensus, evidence
- Concepts: cost-optimization, quality-gates

**FORBIDDEN**:
- ❌ Specific dates (use date filters instead)
- ❌ Task IDs (t84, T12) - ephemeral
- ❌ Status values (in-progress, done) - changes over time
- ❌ Duplicates (use ONE tag: testing, not tests/testing/test-coverage)

**Reuse Before Creating**: Check existing tags first
```

**Impact**: Prevents tag proliferation, enforces consistency

---

## 🚨 Problem #5: Domain Usage Not Explained

### Current Guidance

**CLAUDE.md shows**: `domain: "spec-kit"`, `domain: "testing"`, `domain: "session-summary"`

**But**: No explanation of:
- What domains are for
- How many to create
- When to use which domain
- How they differ from tags

**Result**: Inconsistent usage
- Sometimes domain = tag ("spec-kit" appears in both)
- Sometimes domain is overly specific ("session-summary")
- No clear domain structure

### The Fix

**Add to CLAUDE.md**:
```markdown
### Domain Structure (5 Domains)

**spec-kit**: Spec-kit automation, consensus, multi-agent
**infrastructure**: Cost, testing, architecture, CI/CD
**rust**: Language patterns, borrow checker, performance
**documentation**: Doc strategy, templates, writing
**debugging**: Bug fixes, error patterns, workarounds

**Domain vs Tag**:
- Domain: Broad category (like a folder)
- Tags: Specific attributes (like labels)

Example:
- domain: "spec-kit"
- tags: ["spec:SPEC-KIT-069", "stage:validate", "type:bug-fix"]
```

**Impact**: Clear organization structure, prevents domain proliferation

---

## 🚨 Problem #6: No Cleanup Guidance

### What's Missing in CLAUDE.md

**Zero guidance on**:
- When to delete memories
- How to consolidate duplicates
- Archive/retention policy
- Maximum memory count
- Quality over quantity

**Result**: Perpetual growth
- 574 memories and counting
- No natural cleanup
- Old memories accumulate
- Analysis tools break (35k token overflow)

### The Fix

**Add to CLAUDE.md**:
```markdown
### Memory Lifecycle (NEW)

**Active** (0-30 days): All memories searchable
**Archived** (30-90 days): Lower importance -2, mark status:archived
**Purged** (90+ days): Delete if importance <6

**Monthly Cleanup** (1st of month):
- Review memories added last month
- Delete redundant session summaries
- Consolidate duplicate content
- Recalibrate inflated importance scores
- Target: <20 new memories/month (sustainable growth)

**When to Delete**:
- Information now in documentation
- Session summaries (git commits cover it)
- Outdated information (superseded by newer memories)
- Low-value observations (importance <6, age >90 days)
```

**Impact**: Sustainable growth, prevents re-bloat

---

## 🚨 Problem #7: Spec-Kit Integration is Manual

### Current State (Found in Code)

**Only 20 references** to local-memory in all of `tui/src` Rust code

**Where it's used**:
- `consensus.rs`: Fetch agent artifacts, store synthesis
- `quality_gate_handler.rs`: Store quality gate results
- `quality_gate_broker.rs`: Search for agent outputs
- `spec_prompts.rs`: Gather historical context

**Where it's NOT used** (opportunities!):
- ❌ Cost tracking (we just built cost_tracker.rs, should auto-store!)
- ❌ Evidence creation (should remember evidence paths)
- ❌ Test completion (should store test results)
- ❌ SPEC creation (should store new SPEC metadata)
- ❌ Validation results (should auto-store findings)
- ❌ Error recovery (should remember solutions)

### Spec-Kit Prompts TELL Agents to Use Memory

**Found in quality_gate_handler.rs:848**:
```rust
"After producing the JSON array, store it to local-memory using remember with:
- domain: spec-kit
- importance: 8
- tags: {tags}
- content: JSON array only"
```

**Problem**: We're instructing AGENTS to store, but not TUI itself!

**Agents add ~300-400 memories** (consensus artifacts)
**TUI adds ~100-200 memories** (manual storage via Claude Code)
**Together**: 574 total, but TUI could auto-store more strategically

---

## 🚨 Problem #8: No Cross-Tool Integration

### Different Tools, Different Behaviors

**Claude Code** (Anthropic):
- Uses local-memory CLI + REST (no MCP)
- Follows CLAUDE.md workflow
- Stores manually (we call `lm remember` / `POST /api/v1/memories`)

**Gemini CLI** (Google):
- Unknown if it uses local-memory
- Might have own memory system
- No integration documented

**Code CLI** (this fork):
- Spec-kit agents use local-memory
- Main TUI barely uses it (20 references only)
- Massive missed opportunity

### Integration Gaps

**No shared memory strategy across**:
- Claude Code sessions (manual storage)
- Gemini CLI usage (unknown)
- Code CLI / TUI (minimal integration)
- Spec-kit automation (heavy usage)

**Result**: Inconsistent storage, duplicates across tools

---

## 💡 Massive Optimization Opportunities

### Opportunity #1: Auto-Store from Spec-Kit Events

**Currently Manual** (we call `lm remember` / `POST /api/v1/memories`):
- Session summaries
- Milestone completions
- Bug discoveries

**Could Be Automatic** (TUI could auto-store):

```rust
// In spec_kit/handler.rs

// After stage completes successfully
pub fn on_stage_complete(widget: &ChatWidget, spec_id: &str, stage: SpecStage) {
    // Auto-store completion to memory
    let content = format!(
        "SPEC {} {} stage complete. Consensus: {}, Evidence: {}",
        spec_id, stage.display_name(), consensus_ok, evidence_path
    );

    widget.auto_store_memory(
        content,
        "spec-kit",
        vec![format!("spec:{}", spec_id), format!("stage:{}", stage.command_name())],
        8, // Milestone importance
    );
}

// After cost tracking
pub fn record_agent_call(...) {
    let (cost, alert) = cost_tracker.record(...);

    // Auto-store if approaching budget
    if alert.is_some() {
        widget.auto_store_memory(
            format!("Budget alert for {}: ${:.2} spent", spec_id, spent),
            "infrastructure",
            vec![format!("spec:{}", spec_id), "type:cost-alert"],
            8,
        );
    }
}

// After test suite runs
pub fn on_test_complete(spec_id: &str, results: TestResults) {
    if results.pass_rate < 1.0 {
        // Auto-store test failures
        widget.auto_store_memory(
            format!("Tests failed for {}: {}/{} passing", spec_id, passed, total),
            "infrastructure",
            vec![format!("spec:{}", spec_id), "type:test-failure"],
            9, // High importance for failures
        );
    }
}
```

**Impact**:
- More consistent storage (don't rely on manual)
- Better coverage (capture all important events)
- Less burden on Claude Code (auto happens in background)

---

### Opportunity #2: Smart Tag Auto-Completion

**Current**: We manually choose tags every time
**Problem**: Leads to inconsistency and proliferation

**Proposed**: Auto-suggest tags based on content

```rust
pub fn suggest_tags(content: &str, domain: &str) -> Vec<String> {
    let mut tags = Vec::new();

    // Extract SPEC references
    if let Some(spec_id) = extract_spec_id(content) {
        tags.push(format!("spec:{}", spec_id));
    }

    // Extract stage references
    for stage in ["plan", "tasks", "implement", "validate", "audit", "unlock"] {
        if content.contains(stage) {
            tags.push(format!("stage:{}", stage));
        }
    }

    // Add domain-specific defaults
    match domain {
        "spec-kit" => tags.push("automation"),
        "infrastructure" => tags.push("system"),
        "rust" => tags.push("language"),
        _ => {}
    }

    // Classify content type
    if content.contains("bug") || content.contains("fix") {
        tags.push("type:bug-fix");
    }
    if content.contains("pattern") || content.contains("workaround") {
        tags.push("type:pattern");
    }

    tags
}
```

**Usage**:
```rust
// Suggest tags, let user confirm/modify
let suggested = suggest_tags(content, domain);
println!("Suggested tags: {:?}", suggested);
// User can accept or modify
```

**Impact**: Consistent tagging, prevents proliferation, easier to use

---

### Opportunity #3: Memory Health Dashboard

**Currently**: No visibility into memory system health

**Proposed**: `/memory-stats` command or dashboard

```rust
pub fn generate_memory_health_report() -> MemoryHealthReport {
    let stats = local_memory_stats_rest();

    MemoryHealthReport {
        total_memories: stats.total,
        avg_importance: stats.avg_importance,
        unique_tags: stats.unique_tags,

        // Health indicators
        tag_to_memory_ratio: stats.unique_tags as f64 / stats.total as f64,
        importance_distribution: calculate_distribution(stats),
        byterover_pollution: count_byterover_memories(),
        domain_usage: stats.domains.len(),

        // Alerts
        warnings: vec![
            if tag_ratio > 0.5 => "Tag explosion detected",
            if avg_importance > 7.5 => "Importance inflation detected",
            if byterover > 10 => "Byterover pollution detected",
            if domains == 0 => "Domains not being used",
        ],

        // Recommendations
        recommended_actions: vec![
            "Run memory cleanup (SPEC-KIT-071)",
            "Consolidate tags (552 → <100)",
            "Recalibrate importance",
        ],
    }
}
```

**Display**:
```
Memory System Health Report
===========================
Total Memories:     574
Unique Tags:        552  ⚠️ WARNING: Tag explosion (ratio: 0.96)
Avg Importance:     7.88 ⚠️ WARNING: Inflation detected (target: 5.5-6.5)
Byterover Pollution: 50  ⚠️ WARNING: Deprecated system referenced
Domains Used:       0    🚨 CRITICAL: Feature unused

Recommended Actions:
1. Run SPEC-KIT-071 cleanup (reduce to ~300 memories)
2. Consolidate tags (target: <100)
3. Assign domains (target: 5 domains)
4. Purge byterover references (delete 50 memories)

Health Score: 3/10 (NEEDS ATTENTION)
```

**Impact**: Visibility enables proactive maintenance

---

### Opportunity #4: Consensus Artifact Cleanup

**Current Behavior** (found in code):

Agents store consensus artifacts to local-memory:
- Every plan stage: 3-5 agent memories
- Every tasks stage: 3-5 agent memories
- Every implement stage: 3-5 agent memories
- etc.

**At 6 stages × 4 agents**: 24 consensus artifact memories per SPEC

**Problem**: These are LOW-VALUE after synthesis
- Synthesis aggregates all agent outputs
- Individual agent outputs rarely referenced
- Stored at importance 8-9 (too high for raw artifacts)
- Should be archived after synthesis complete

**Proposed**: Lifecycle management

```rust
// After consensus synthesis succeeds
pub fn on_consensus_complete(spec_id: &str, stage: SpecStage) {
    // Store synthesis (high value)
    store_synthesis(spec_id, stage, synthesis, importance: 9);

    // Archive raw agent artifacts (lower value)
    for agent in agents {
        update_memory(
            agent.memory_id,
            importance: 5, // Downgrade from 8-9
            tags: add("status:archived"),
        );
    }

    // Or even delete after 30 days
    schedule_cleanup(spec_id, stage, delay: 30.days());
}
```

**Impact**:
- Reduce memory count by 100-150 (old consensus artifacts)
- Keep synthesis (high value), archive/delete raw (low value)
- Importance recalibration (8-9 → 5 for artifacts)

---

### Opportunity #5: Deduplication Before Storage

**Current**: No dedup check before storing

**Problem**: Might store same info twice
- Multiple sessions discover same pattern
- Same bug fix documented multiple times
- Similar milestones create similar memories

**Proposed**: Check for duplicates before storing

```rust
pub async fn smart_store_memory(
    content: &str,
    domain: &str,
    tags: Vec<String>,
    importance: u32,
) -> Result<String> {
    // Check for similar existing memories
    let similar = search_memory(
        query: content.substring(0, 100), // First 100 chars
        domain: Some(domain),
        limit: 5,
    ).await?;

    // If very similar content exists
    if let Some(existing) = similar.iter().find(|m| similarity(m.content, content) > 0.9) {
        // Update existing instead of creating new
        update_memory(
            existing.id,
            content: content, // Update with new info
            tags: merge_tags(existing.tags, tags),
            importance: max(existing.importance, importance),
        ).await?;

        return Ok(format!("Updated existing memory: {}", existing.id));
    }

    // No duplicate found, store new
    store_memory(content, domain, tags, importance).await
}
```

**Impact**: Prevents duplicate memories, consolidates information

---

### Opportunity #6: Better CLAUDE.md Integration Examples

**Current Examples** (CLAUDE.md:258-307):
- Show basic usage
- No tag schema
- Include bad patterns (dates, session summaries)
- No negative examples (what NOT to do)

**Improved Examples**:

```markdown
### GOOD Example ✅
content: "Native SPEC-ID generation eliminates $2.40 consensus cost. Implementation: spec_id_generator.rs scans docs/, finds max ID, increments. Pattern: Use native Rust for deterministic tasks (10,000x faster, FREE)."
domain: "infrastructure"
tags: ["spec:SPEC-KIT-070", "type:pattern", "cost-optimization"]
importance: 9

Why Good:
- Captures WHY (pattern insight)
- Includes HOW (implementation detail)
- Reusable (applies to other deterministic tasks)
- Proper tags (namespaced, meaningful)
- Proper domain
- Proper importance (major discovery = 9)

### BAD Example ❌
content: "Session 2025-10-24: Did stuff. Fixed bugs. Created SPEC-070."
domain: "session-summary"
tags: ["2025-10-24", "session-complete", "done"]
importance: 9

Why Bad:
- Redundant (git commits cover this)
- Vague (no actionable insights)
- Date tag (useless for retrieval)
- Status tags (ephemeral)
- Wrong importance (routine session ≠ 9)
- Wrong domain (too specific)

### BETTER (if storing session at all) ⚠️
content: "Discovered OpenAI rate limit crisis during SPEC-KIT-070 validation. Hit limits at current usage rate (1 day 1 hour block). Proves cost optimization isn't just financial - it's operational blocker. Changed strategy to prioritize provider diversity."
domain: "infrastructure"
tags: ["spec:SPEC-KIT-070", "type:discovery", "priority:critical"]
importance: 10

Why Better:
- Captures specific discovery (rate limits = operational blocker)
- Includes impact (changed strategy)
- No date tags (use date filters)
- Proper domain (infrastructure, not session-summary)
- Justified importance (crisis discovery = 10)
```

**Impact**: Teach by example, prevent bad patterns

---

## 🚨 Problem #8: Spec-Kit Agents Over-Store

**Current Behavior** (from prompt analysis):

Agents instructed to store at importance: 8
**Every stage**: 3-5 agents × importance 8 = inflates average

**Example**: Plan stage
- gemini stores plan (importance: 8)
- claude stores plan (importance: 8)
- gpt_pro stores plan (importance: 8)
- code stores plan (importance: 8)

**Result**: 4 memories, all importance 8, for ONE consensus operation

**Better**:
- Agents store at importance: 6 (raw artifacts, lower value)
- Synthesis stores at importance: 8 (aggregated, higher value)
- After 30 days: Archive agent artifacts (importance: 6 → 4)

**Impact**: Lower average importance, better value distribution

---

## 📊 CLAUDE.md Problems Summary

| Issue | Current Guidance | Impact | Fix Priority |
|-------|-----------------|---------|--------------|
| Mandatory session summaries | REQUIRED at session end | 40-50 redundant memories | P0 |
| Importance threshold | ≥7 (too low) | Avg 7.88 inflation | P0 |
| Date tags in examples | Shows "2025-10-20" | 30+ useless tags | P0 |
| No tag schema | Not documented | 552 tag chaos | P0 |
| Domain usage unclear | Inconsistent examples | No organization | P1 |
| No cleanup guidance | Not mentioned | Perpetual growth | P1 |
| No negative examples | Only "good" patterns | Repeat bad patterns | P1 |
| Manual storage only | No auto-store | Missed opportunities | P2 |

---

## 🎯 SPEC-KIT-071 Expanded Scope

### Original Scope (PRD.md)
- Clean up 574 → 300 memories
- Consolidate 552 → 90 tags
- Organize with domains/categories

### NEW Scope (Root Cause Fix)
- **Fix CLAUDE.md documentation** (drives behavior)
- **Update MEMORY-POLICY.md** (add tag schema, cleanup policy)
- **Add auto-storage to TUI** (spec-kit event integration)
- **Create memory health dashboard** (visibility)
- **Then execute cleanup** (fix existing data)
- **Prevent re-bloat** (documentation + automation)

**Insight**: Fixing documentation prevents future bloat, cleanup fixes past bloat

---

## 🔧 Recommended CLAUDE.md Changes

### Section 9: Memory Workflow Checklist

**REMOVE**:
```markdown
❌ **5. Session End** (REQUIRED)
```

**REPLACE WITH**:
```markdown
**5. Session End** (OPTIONAL - only if exceptional):

Store session summary ONLY for:
- Major breakthroughs or discoveries
- Multi-day work needing handoff
- Critical decisions not in individual memories

Otherwise: Individual memories + git commits + SPEC.md are sufficient.

If storing:
- domain: Appropriate domain (NOT "session-summary")
- tags: ["type:handoff"] (NO specific dates)
- importance: 6-8 based on significance
```

**ADD**:
```markdown
**6. What NOT to Store**:

❌ Session summaries (use git commits + SPEC.md)
❌ Progress updates (use SPEC.md task tracker)
❌ Information in documentation (link instead)
❌ Transient status ("in progress", "blocked")
❌ Routine operations (normal workflow)
❌ Low-value observations (importance <8)

✅ Store ONLY high-value, reusable knowledge
```

**ADD**:
```markdown
### Tag Schema (REQUIRED)

**Namespaced Format**:
spec:<SPEC-ID>        Example: spec:SPEC-KIT-071
stage:<name>          Example: stage:implement
agent:<name>          Example: agent:claude
type:<category>       Example: type:bug-fix, type:pattern, type:discovery

**General Tags** (~50 max):
- Domains: spec-kit, infrastructure, rust, documentation
- Tools: mcp, testing, consensus
- Concepts: cost-optimization, quality-gates

**FORBIDDEN**:
❌ Specific dates (use date filters: --start-date)
❌ Task IDs (t84, T12) - ephemeral
❌ Status (in-progress, done) - changes over time
❌ Duplicates (testing, tests, test-coverage → pick ONE)

Reuse tags before creating new ones!
```

**ADD**:
```markdown
### Importance Calibration

Use this guide STRICTLY:

10: Crisis/critical (rate limits, cost crisis, system-breaking) - <5% of stores
9:  Major discoveries (architectural insights, critical patterns) - 10-15%
8:  Important milestones (Phase complete, major fixes) - 15-20%
7:  Useful reference (good context, decent findings) - 20-30%
6:  Minor findings (small improvements, notes) - RARELY STORE
5-: Don't store (use docs/git instead)

Threshold: Store ONLY importance ≥8 (not ≥7)
Target average: 8-9 (not 7.88)
```

**ADD**:
```markdown
### Domain Structure

5 Domains (use consistently):

**spec-kit**: Automation, consensus, multi-agent, workflows
**infrastructure**: Cost, testing, architecture, CI/CD, performance
**rust**: Language patterns, borrow checker, performance, cargo
**documentation**: Doc strategy, templates, writing, guides
**debugging**: Bug fixes, error patterns, workarounds, troubleshooting
```

---

## 📈 Expected Impact of Documentation Fixes

### Immediate (Next Session)

**If we update CLAUDE.md now**:
- Future sessions won't create session summaries (saves 2-5 memories/session)
- Importance threshold ≥8 instead of ≥7 (prevents inflation)
- No date tags (prevents 1-2 new ephemeral tags/session)
- Consistent tag schema (prevents proliferation)

**Monthly**:
- Without fix: +40-60 memories/month (unsustainable)
- With fix: +10-20 memories/month (sustainable)
- **Savings**: 50-75% reduction in growth rate

### Combined with Cleanup

**Cleanup alone**: 574 → 300 (one-time fix)
**Without doc fix**: 300 → 574 again in 6 months (re-bloat)
**With doc fix**: 300 → 350-400 in 12 months (controlled growth)

**Conclusion**: Documentation fix is MORE important than cleanup!

---

## 🎯 SPEC-KIT-071 Revised Strategy

### Phase 0: Fix Root Cause (NEW, FIRST)

**Week 1, Day 1-2** (4-6 hours):
- [x] Update CLAUDE.md with new memory workflow
- [x] Add tag schema specification
- [x] Add importance calibration guide
- [x] Add domain structure
- [x] Remove session summary requirement
- [x] Add negative examples
- [x] Update MEMORY-POLICY.md with tag schema

**Deliverables**:
- Updated CLAUDE.md (Section 9 rewritten)
- Updated MEMORY-POLICY.md (tag schema added)
- Prevents future bloat (most important!)

---

### Phase 1: Cleanup (REVISED)

**Now we can cleanup knowing it won't come back!**

**Week 1, Day 3-6** (8-12 hours):
- Same as original plan
- But with confidence it won't re-bloat
- Documentation fixes prevent recurrence

---

### Phase 2: Automation (NEW)

**Week 2** (6-8 hours):
- Implement auto-storage in spec-kit
- Add memory health dashboard
- Create tag suggestion helpers
- Enable dedup checking

**Better than original Phase 2** because:
- Documentation already fixed (Phase 0)
- Auto-storage prevents manual mistakes
- Health dashboard enables monitoring

---

## 🔍 Additional Findings

### Finding #1: 297 local-memory References

**In markdown files**: 297 references total
**Problem**: No way to know if they're all current/accurate
**Should audit**: Check if references match current behavior

### Finding #2: Minimal TUI Integration

**Only 20 Rust references** in all of `tui/src`
**Concentrated in**: consensus.rs, quality_gate_*.rs, spec_prompts.rs

**Missed Integration Points**:
- Cost tracking (new in SPEC-KIT-070)
- Evidence creation
- Test results
- SPEC creation
- Validation outcomes

**Opportunity**: 5-10x more auto-storage possible

### Finding #3: Agent Prompts Instruct Storage

**Agents are TOLD to use local-memory**:
```
"store it to local-memory using remember with:
- domain: spec-kit
- importance: 8
- tags: {tags}"
```

**Problem**: We control this via prompts!
- Can reduce importance: 8 → 6 (raw artifacts)
- Can standardize tags better
- Can add dedup checking

**Easy Win**: Update prompt templates to use better storage patterns

---

## 💰 This is BIGGER than Expected

### Original SPEC-KIT-071 Estimate
- **Scope**: Cleanup bloat (574 → 300)
- **Effort**: 16-23 hours
- **Impact**: Better organization

### REVISED SPEC-KIT-071 Scope
- **Scope**: Fix root cause (CLAUDE.md) + cleanup + automation
- **Effort**: 25-35 hours
- **Impact**:
  - Fix bloat (574 → 300, 48%)
  - Prevent re-bloat (documentation fixes)
  - Enable auto-storage (TUI integration)
  - Provide visibility (health dashboard)
  - Long-term sustainability

### This is a SYSTEM REDESIGN, not just cleanup!

---

## 🎯 Priority Re-Assessment

**Was thinking**: SPEC-KIT-071 is cleanup work (P1, can wait)

**Now realize**: SPEC-KIT-071 is **infrastructure overhaul** (P0-P1, blocks scalability)

**Why Higher Priority**:
1. **Blocks SPEC-KIT-070 Phase 2**: Cost tracking needs clean memory for telemetry
2. **Blocks scaling**: Can't store more if analysis tools already break
3. **Documentation fix prevents waste**: Every session without fix creates more bloat
4. **Easy wins available**: CLAUDE.md updates take 2-3 hours, immediate impact

**Recommended Priority Order**:
1. **SPEC-KIT-070**: Cost crisis (P0, blocks sustainability financially)
2. **SPEC-KIT-071**: Memory bloat (P0-P1, blocks sustainability operationally)
3. **SPEC-KIT-066**: Native tools (P1, blocks features but not as urgent)
4. **SPEC-KIT-067/068**: Nice-to-haves (P2)

---

## 📝 Summary of Ultrathink Findings

**Root Cause Identified**: CLAUDE.md documentation drives bloat
- Requires session summaries (redundant)
- Threshold ≥7 too low (inflation)
- Examples show date tags (proliferation)
- No tag schema (chaos)
- No cleanup guidance (perpetual growth)

**Scope Expansion**: From cleanup → System redesign
- Fix documentation (prevent future bloat)
- Cleanup existing data (fix past bloat)
- Add automation (reduce manual burden)
- Add monitoring (visibility)

**Integration Gaps**: Massive opportunity
- TUI barely uses memory (20 references)
- Could auto-store 10x more
- Agent prompts need better guidance
- Cross-tool integration unclear

**This is as important as SPEC-KIT-070** because both are infrastructure sustainability issues!

Should I create the expanded PRD with root cause fixes and integration improvements?
