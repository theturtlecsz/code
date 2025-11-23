# Memory System: Local-Memory Knowledge Base

## Overview

Spec-Kit uses a local-memory MCP (Model Context Protocol) server as its knowledge persistence system. This creates a curated knowledge base of reusable patterns, architectural decisions, and lessons learned - NOT a complete history archive.

## Purpose

### What Local-Memory Is

- **Curated knowledge base**: High-value patterns and decisions
- **Living project handbook**: Current understanding of architecture
- **Reusable insights**: Solutions that apply to future work
- **Decision rationale**: Why choices were made

### What Local-Memory Is NOT

- **Session history**: Use git commits and SPEC.md instead
- **Progress tracker**: Use SPEC.md task table
- **Complete archive**: Store only importance ≥8
- **Consensus storage**: Moving to separate SQLite DB

## Core Operations

### Search

```rust
// Before starting work, query for relevant context
mcp__local_memory__search(SearchRequest {
    query: "OAuth implementation patterns",
    limit: 10,
    search_type: SearchType::Semantic,
    tags: Some(vec!["authentication", "security"]),
})
```

**Use Cases**:
- Session start: "project architecture recent changes"
- Before tasks: "test coverage phase 3 integration"
- Problem solving: "rate limit handling solutions"

### Store

```rust
// Store important discoveries and decisions
mcp__local_memory__store_memory(StoreRequest {
    content: "Native SPEC-ID generation eliminates $2.40 consensus cost...",
    domain: "infrastructure",
    tags: vec!["type:pattern", "spec:SPEC-KIT-070", "cost-optimization"],
    importance: 9,
})
```

**When to Store** (importance ≥8 only):
- 🏗️ Architecture decisions with rationale
- 🔧 Reusable patterns and code examples
- 🚨 Critical discoveries (rate limits, system-breaking)
- 🐛 Non-obvious bug fixes with context
- ⚠️ Important limitations and workarounds
- ✅ Major milestones with outcomes

### Analysis

```rust
// Get insights about stored knowledge
mcp__local_memory__analysis(AnalysisRequest {
    domain: Some("spec-kit"),
    time_range: Some(TimeRange::LastMonth),
})
```

---

## Domain Structure

Memories organized into 5 primary domains:

| Domain | Purpose | Examples |
|--------|---------|----------|
| `spec-kit` | Automation, consensus, workflows | Agent routing, stage configuration |
| `infrastructure` | Cost, testing, architecture, CI/CD | Performance optimizations, build issues |
| `rust` | Language patterns, cargo, performance | Borrow checker solutions, async patterns |
| `documentation` | Doc strategy, templates, guides | Writing patterns, structure decisions |
| `debugging` | Bug fixes, error patterns, workarounds | Root causes, non-obvious solutions |

---

## Tag Schema

### Namespaced Tags (Required When Applicable)

```
spec:<SPEC-ID>          # Example: spec:SPEC-KIT-070
type:<category>         # Example: type:bug-fix, type:pattern
project:<name>          # Example: project:codex-rs
component:<area>        # Example: component:routing, component:consensus
```

### General Tags (~30-50 approved)

```
Core:     testing, mcp, consensus, evidence, telemetry
Concepts: cost-optimization, quality-gates, rebase-safety
Tools:    borrow-checker, native-tools
```

### Forbidden Tags (Auto-Reject)

```
❌ Specific dates: 2025-10-20, 2025-10-14
   → Use date filters instead

❌ Task IDs: t84, T12, t21
   → Ephemeral, not useful long-term

❌ Status values: in-progress, blocked, done
   → Changes over time

❌ Overly specific: 52-lines-removed, policy-final-check
   → Not reusable
```

---

## Importance Calibration

### Importance Scale

| Score | Use For | % of Stores |
|-------|---------|-------------|
| **10** | Crisis events, system-breaking discoveries | <5% |
| **9** | Major architectural decisions, critical patterns | 10-15% |
| **8** | Important milestones, valuable solutions | 15-20% |
| **7** | Useful context (RARELY store, use docs/git instead) | 10-15% |
| **≤6** | DON'T STORE - use git commits, SPEC.md, or documentation | N/A |

### Threshold

- **Store**: importance ≥8 only
- **Target average**: 8.5-9.0
- **Current average**: Should not drop below 8.0

### Examples by Score

**Score 10** (Crisis/System-Breaking):
```
"Discovered OpenAI rate limit crisis - hit limits during testing,
blocked for 1 day 1 hour. Validates SPEC-KIT-070 urgency. Must
prioritize provider diversity and cost reduction immediately."
```

**Score 9** (Major Architecture/Pattern):
```
"Native SPEC-ID generation eliminates $2.40 consensus cost per
/speckit.new. Pattern: Use native Rust for deterministic tasks -
10,000x faster, FREE, more reliable than AI consensus."
```

**Score 8** (Important Milestone/Solution):
```
"Test coverage Phase 3 complete: 60 integration tests added
(workflow, error recovery, state persistence). Total: 555 tests,
100% pass. Pattern: IntegrationTestContext harness enables complex
multi-module testing."
```

**Score 7** (DON'T STORE - use docs instead):
```
"Updated configuration file for new feature."
→ This belongs in git commit message
```

---

## Workflow Integration

### Session Start

**Required**: Query for project context

```rust
// First action in every session
let context = mcp__local_memory__search(SearchRequest {
    query: "project architecture recent changes",
    limit: 10,
    search_type: SearchType::Semantic,
}).await?;

// Review recent decisions before starting work
for memory in context.memories {
    process_context(&memory);
}
```

### Before Major Tasks

**Required**: Search for relevant prior work

```rust
// Before implementing OAuth
let prior_work = mcp__local_memory__search(SearchRequest {
    query: "OAuth implementation authentication security",
    tags: Some(vec!["authentication"]),
    limit: 5,
}).await?;

// Apply lessons learned
for memory in prior_work.memories {
    if memory.importance >= 8 {
        apply_lesson(&memory);
    }
}
```

### During Work

**Store when importance ≥8**: Key decisions, patterns, discoveries

```rust
// Found important pattern during implementation
if discovery.importance >= 8 {
    mcp__local_memory__store_memory(StoreRequest {
        content: format!(
            "Pattern discovered: {}. Implementation: {}. Applies to: {}",
            discovery.pattern,
            discovery.implementation,
            discovery.applicability
        ),
        domain: "debugging",
        tags: vec![
            format!("type:{}", discovery.type),
            format!("spec:{}", current_spec),
            format!("component:{}", discovery.component),
        ],
        importance: discovery.importance,
    }).await?;
}
```

### After Milestones

**Store completion evidence** (importance ≥8):

```rust
// Phase complete - store outcomes
mcp__local_memory__store_memory(StoreRequest {
    content: format!(
        "{} complete: {}. Evidence: {}. Patterns: {}",
        phase_name,
        outcomes,
        evidence_locations,
        learned_patterns
    ),
    domain: "infrastructure",
    tags: vec![
        "type:milestone",
        format!("spec:{}", spec_id),
        // relevant domain tags
    ],
    importance: 8,
}).await?;
```

### Session End

**Optional** - only store session summary if exceptional:

Store ONLY if:
- Major breakthrough or discovery
- Multi-day work requiring detailed handoff
- Critical decisions NOT captured in individual memories

Otherwise: Individual memories + git commits + SPEC.md are sufficient.

---

## Storage Examples

### Good Example ✅

```rust
StoreRequest {
    content: "Routing bug fixed: SpecKitCommand wasn't passing config.
Root cause: routing.rs line 45 passed None instead of actual config.
Solution: Pass widget.config to format_subagent_command().
Pattern: Always verify config propagation in command chains.",
    domain: "debugging",
    tags: vec![
        "type:bug-fix",
        "spec:SPEC-KIT-066",
        "component:routing"
    ],
    importance: 9,
}
```

**Why Good**:
- Captures WHY (pattern: verify config propagation)
- Includes HOW (specific solution)
- Generalizable (applies beyond this case)
- Proper tags (namespaced, meaningful)
- Justified importance (pattern = 9)

### Bad Example ❌

```rust
StoreRequest {
    content: "Session 2025-10-24: Did work on SPEC-069 and SPEC-070.
Made progress. Tests passing.",
    domain: "session-summary",
    tags: vec![
        "2025-10-24",
        "session-complete",
        "done"
    ],
    importance: 9,
}
```

**Why Bad**:
- Redundant (git commits capture this)
- Vague (no actionable insights)
- Date tag (useless for retrieval)
- Status tags (ephemeral)
- Wrong importance (routine ≠ 9)
- No WHY (doesn't explain decisions)

---

## Retrieval Patterns

### Semantic Search

Find conceptually related memories:

```rust
// Find memories about cost optimization
mcp__local_memory__search(SearchRequest {
    query: "reducing API costs model selection",
    search_type: SearchType::Semantic,
    limit: 10,
})
```

### Tag Filtering

Find memories with specific tags:

```rust
// Find all bug fixes for routing
mcp__local_memory__search(SearchRequest {
    query: "routing",
    tags: Some(vec!["type:bug-fix", "component:routing"]),
    limit: 10,
})
```

### Domain Scoping

Search within specific domain:

```rust
// Find patterns in spec-kit domain
mcp__local_memory__search(SearchRequest {
    query: "consensus patterns",
    domain: Some("spec-kit"),
    limit: 10,
})
```

### Time-Based

Find recent memories:

```rust
// What was learned this week?
mcp__local_memory__search(SearchRequest {
    query: "*",
    time_range: Some(TimeRange::LastWeek),
    limit: 20,
})
```

---

## Maintenance

### Quality Targets

| Metric | Target | Current |
|--------|--------|---------|
| Total memories | 120-150 | Monitor |
| Monthly stores | 40-60 | Monitor |
| Average importance | 8.5-9.0 | Monitor |
| Domains coverage | All 5 active | Monitor |

### Quarterly Cleanup

- Remove outdated memories (deprecated patterns)
- Consolidate duplicate tags
- Archive low-importance memories
- Review domain balance

### Migration Plan

Consensus artifacts will migrate to separate database (SPEC-KIT-072):
- Agent outputs → SQLite consensus_db
- Structured data → SQLite
- Human insights → local-memory (stays)

---

## Best Practices

### DO

✅ Query before starting work
✅ Store patterns with WHY
✅ Use namespaced tags
✅ Calibrate importance honestly
✅ Include applicability scope
✅ Reference specific files/lines

### DON'T

❌ Store session summaries
❌ Store progress updates
❌ Use date tags
❌ Inflate importance scores
❌ Store info already in docs
❌ Store routine operations

### Content Quality

**Good content includes**:
- What was discovered/decided
- Why it matters
- How to apply it
- Where it applies
- Evidence/references

**Bad content includes**:
- Vague descriptions
- Status updates
- Duplicate information
- Context-free facts
- Ephemeral details
