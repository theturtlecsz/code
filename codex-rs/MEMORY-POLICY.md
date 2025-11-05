# Memory System Policy

**Effective Date**: 2025-10-18
**Status**: MANDATORY

---

## Single Memory System: local-memory MCP

**Policy**: Use **local-memory MCP** exclusively for all knowledge persistence and retrieval.

**Deprecated**:
- ~~byterover-mcp~~ (DO NOT USE)
- ~~Any other memory MCP servers~~

---

## Rationale

**Why local-memory only**:
1. **Native MCP Integration**: Validated 5.3x faster than subprocess baseline
2. **Spec-Kit Dependency**: Consensus framework requires local-memory for multi-agent synthesis
3. **Single Source of Truth**: Eliminates memory conflicts and divergence
4. **Tested & Reliable**: 141 passing tests validate MCP integration path

**Why not byterover**:
- Adds unnecessary complexity
- Potential for memory conflicts between systems
- Not integrated with spec-kit automation
- Unclear sync/merge semantics

---

## Usage Guidelines

### Store Knowledge
```bash
# Via MCP tool (in code)
mcp_manager.call_tool("local-memory", "store_memory", {
  "content": "...",
  "domain": "spec-kit",
  "tags": ["spec:SPEC-ID", "stage:plan"],
  "importance": 8
})

# Via CLI (manual)
local-memory remember "knowledge here" \
  --importance 8 \
  --domain spec-kit \
  --tags spec:SPEC-123
```

### Retrieve Knowledge
```bash
# Via MCP tool (in code)
mcp_manager.call_tool("local-memory", "search", {
  "query": "consensus plan",
  "limit": 20,
  "tags": ["spec:SPEC-ID", "stage:plan"],
  "search_type": "hybrid"
})

# Via CLI (manual)
local-memory search "consensus" --tags spec:SPEC-123
```

### Query Best Practices
1. **Before** any task: Query local-memory for relevant context
2. **During** work: Store decisions with importance ≥7
3. **After** completion: Store outcomes, evidence paths, validation results
4. **Tag Structure**: Use `spec:SPEC-ID`, `stage:STAGE`, `consensus-verdict` for spec-kit artifacts

---

## Integration Points

**Spec-Kit Consensus** (`tui/spec_kit/consensus.rs`):
- `fetch_memory_entries()`: Searches local-memory for agent artifacts
- `remember_consensus_verdict()`: Stores synthesis results
- **Fallback**: File-based evidence if MCP unavailable (see ARCH-002)

**Session Context** (`tui/spec_prompts.rs`):
- `gather_local_memory_context()`: Retrieves historical context for agents
- Used by all 6 spec-kit stages (plan, tasks, implement, validate, audit, unlock)

**Evidence Repository** (`tui/spec_kit/evidence.rs`):
- Writes artifacts to filesystem: `docs/SPEC-OPS-004.../evidence/`
- Local-memory stores metadata pointing to evidence files

---

## Migration Complete

**Status**: Byterover migration to local-memory **COMPLETE** as of 2025-10-18

**What Changed**:
- Subprocess `Command::new("local-memory")` → Native MCP
- Performance: 5.3x faster (46ms → 8.7ms measured)
- Reliability: 3-retry logic with exponential backoff
- Testing: 3 integration tests validate MCP path

**Removed**:
- ~~Byterover MCP tool calls~~
- ~~Byterover fallback logic~~
- ~~Subprocess local-memory wrapper~~ (deprecated, pending deletion)

---

## Do Not Use

**Forbidden MCP Servers** (for memory):
- ❌ `byterover-mcp`
- ❌ Any memory system other than `local-memory`

**Exception**: MCP servers for **tools** (not memory) are allowed:
- ✅ `git` (version control operations)
- ✅ `codegraphcontext` (code search)
- ✅ `ide` (editor integrations)
- ✅ etc.

**Distinction**:
- **Memory MCP**: Stores/retrieves knowledge (local-memory ONLY)
- **Tool MCP**: Provides functionality (git, codegraphcontext, etc. - allowed)

---

## Enforcement

**Code Reviews**: Flag any byterover references
**Documentation**: This policy referenced in CLAUDE.md, REVIEW.md
**Validation**: `grep -r "byterover" . --include="*.rs"` should return 0 matches

**Last Verified**: 2025-10-18 (no byterover in active codebase)

---

---

## Tag Schema (SPEC-KIT-071)

**Effective**: 2025-10-24
**Purpose**: Prevent tag proliferation (was 557 tags for 577 memories!)

### Namespaced Tags (Use When Applicable)

```
spec:<SPEC-ID>          Reference to SPEC documents (e.g., spec:SPEC-KIT-071)
type:<category>         Content type (bug-fix, pattern, discovery, milestone, architecture)
project:<name>          Project scope (codex-rs, kavedarr, etc.)
component:<area>        Code area (routing, consensus, testing, etc.)
stage:<stage>           Pipeline stage (plan, tasks, implement, validate, audit, unlock)
agent:<name>            AI agent (claude, gemini, gpt_pro, code)
```

### General Tags (Curated List, ~30-50 max)

**Core Domains** (primary categories):
- spec-kit, infrastructure, rust, documentation, debugging

**Tools & Systems**:
- mcp, testing, consensus, evidence, telemetry

**Concepts & Themes**:
- cost-optimization, quality-gates, rebase-safety, borrow-checker, native-tools

**Adding New Tags**:
- Check existing tags first (reuse over create)
- Justify unique concept
- Quarterly review consolidates duplicates

### Forbidden Tags (Auto-Reject)

```
❌ Specific dates: 2025-10-20, 2025-10-14
   → Use date range filters instead: --start-date 2025-10-01

❌ Task IDs: t84, T12, t21, T78
   → Ephemeral, not useful for long-term retrieval

❌ Status values: in-progress, blocked, done, complete, resolved
   → Status changes over time, use search filters instead

❌ Overly specific: 52-lines-removed, policy-final-check
   → Not reusable, doesn't generalize
```

---

## Importance Calibration (SPEC-KIT-071)

**Effective**: 2025-10-24
**Purpose**: Prevent importance inflation (was avg 7.88, target 8.5-9.0)

### Scoring Guide

**Use strictly to maintain quality-focused curation**:

```
10: Crisis events, system-breaking discoveries (<5% of stores)
    - Operational blockers (rate limits, data corruption)
    - Critical architecture flaws
    - Security vulnerabilities
    - Examples: OpenAI rate limit discovery, CLAUDE.md drives bloat

9:  Major architectural decisions, critical patterns (10-15%)
    - Significant refactors with lessons
    - Cost optimization strategies ($1,000+ annual impact)
    - Complex problem solutions (borrow checker workarounds)
    - Examples: Native > AI for deterministic, tiered memory architecture

8:  Important milestones, valuable solutions (15-20%)
    - Phase completions with evidence
    - Non-obvious bug fixes with context
    - Reusable code patterns
    - Examples: Phase 1 complete (180 tests), handler.rs extraction

7:  Useful context, good reference (RARELY STORE, <10%)
    - Configuration changes with detailed rationale
    - Minor optimizations with measurable impact
    - Use docs/git instead unless truly valuable

6 and below: DON'T STORE
    - Use git commits, SPEC.md, or documentation
    - Not valuable for knowledge base
```

**Threshold**: Store ONLY importance ≥8
**Target Average**: 8.5-9.0 (quality-focused curation)
**Target Distribution**:
- 10: <5%, 9: 10-15%, 8: 15-20%, 7: <10%, 6-: 0%

---

## Storage Criteria (SPEC-KIT-071)

**Effective**: 2025-10-24

### When to Store (importance ≥8)

**Store IF it meets ALL criteria**:
1. ✅ Will be useful in 30+ days (not transient)
2. ✅ Reusable knowledge (applies beyond specific case)
3. ✅ Explains WHY (rationale, not just what was done)
4. ✅ Unique (not already in docs/code/git)
5. ✅ High value (importance ≥8)

**Examples of GOOD storage**:
- Architecture decisions with trade-off analysis
- Patterns that generalize (native > AI for deterministic)
- Critical discoveries (rate limits block operations)
- Complex bug fixes (borrow checker solutions)

### When NOT to Store

**Don't store if it's**:
- Already in documentation (link to docs instead)
- Captured in git commits (what was changed)
- Progress updates (use SPEC.md task tracker)
- Transient status (in-progress, blocked)
- Routine operations (normal workflow)
- Session summaries (redundant with git + individual memories)
- Low importance (<8)

---

## Cleanup & Maintenance (SPEC-KIT-071)

**Effective**: 2025-10-24

### Quarterly Maintenance (Every 3 Months)

**Schedule**: 1st week of Jan/Apr/Jul/Oct
**Time**: 2-3 hours per quarter
**Goal**: Maintain 120-150 curated knowledge memories

**Tasks**:
1. **Review Growth**: Check total count, flag if >180
2. **Archive Old**: Mark importance -2 for memories >90 days old (unless critical)
3. **Consolidate Tags**: Identify and merge duplicate tag concepts
4. **Recalibrate Importance**: Adjust if average drifts from 8.5-9.0
5. **Remove Obsolete**: Delete outdated information (superseded decisions)
6. **Verify Quality**: Spot-check 20-30 memories for value

**Criteria-Based Cleanup**:
```
DELETE if:
- Importance <8 AND age >90 days
- Superseded by newer memory on same topic
- Information now in documentation
- Transient status or ephemeral content

ARCHIVE if:
- Importance 8 AND age >90 days AND not frequently accessed
- Historical value but not current

KEEP if:
- Importance ≥9 (always)
- Importance 8 AND frequently referenced
- Current project knowledge
```

### Health Metrics

**Monitor quarterly**:
- Total memories (target: 120-150)
- Unique tags (target: <100)
- Average importance (target: 8.5-9.0)
- Growth rate (target: 40-60/month)

**Alert thresholds**:
- 🟡 Warning: >180 memories (cleanup recommended)
- 🔴 Critical: >250 memories (cleanup required)
- 🔴 Critical: >150 tags (consolidation required)
- 🔴 Critical: Avg importance <8.0 or >9.5 (recalibration required)

---

## Lifecycle Management

**Effective**: 2025-10-24

### Memory States

```
Active (0-30 days):
- All recent memories
- Highly discoverable
- Full importance weight

Aging (30-90 days):
- Still searchable
- Consider archival if importance <9
- Quarterly review candidates

Stale (90+ days):
- Archive if importance <9
- Delete if importance <8
- Keep permanently if importance ≥9
```

### Archival Strategy

**Not yet implemented** (planned for Phase 2):
- Lower importance -2 for archived
- Move to archived category
- Still searchable but lower priority
- Can restore if needed

---

## Separation of Concerns (SPEC-KIT-072)

**Planned**: 2025-10-24

### Consensus Artifacts → Separate Database

**What moves to SQLite DB**:
- Agent outputs (gemini, claude, gpt_pro, code)
- Consensus synthesis results
- Quality gate artifacts
- Structured telemetry data
- ~300-350 memories (52-61% of current total!)

**What stays in local-memory**:
- Curated knowledge (patterns, decisions)
- Architecture rationale
- Bug fixes with context
- Reusable insights
- ~120-150 memories (focused knowledge base)

**Rationale**: Structured data (consensus) vs semantic knowledge (insights)
- Different query patterns (SQL vs semantic search)
- Different retention (90 days vs permanent)
- Different purposes (artifacts vs knowledge)

---

## Questions?

If uncertain about memory system usage:
1. Default to local-memory MCP
2. Check this policy document
3. Review CLAUDE.md Section 9 for workflow
4. Ask maintainer if edge case arises

**Maintainer**: theturtlecsz
**Repository**: https://github.com/theturtlecsz/code
