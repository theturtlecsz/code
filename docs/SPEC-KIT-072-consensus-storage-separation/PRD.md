# PRD: Separate Consensus Artifact Storage from Knowledge Base

**SPEC-ID**: SPEC-KIT-072
**Created**: 2025-10-24
**Status**: Draft - **ARCHITECTURAL**
**Priority**: **P1** (Enables clean separation of concerns)
**Owner**: Code
**Depends On**: SPEC-KIT-071 (memory cleanup)

---

## 🎯 Executive Summary

**Current State**: Consensus artifacts (structured JSON) and curated knowledge (insights/patterns) both stored in local-memory MCP, causing 300-400 low-value memories to pollute the knowledge base.

**Proposed State**: Separate database for consensus artifacts (queryable structured data) with local-memory reserved for high-value curated knowledge only.

**Impact**:
- Local-memory: 574 → ~200-250 memories (56-65% reduction from separation alone!)
- Consensus: Fast SQL queries on structured data
- Knowledge: Clean, curated, findable patterns and decisions

---

## 1. Problem Statement

### Current Architecture Problem

**We're mixing two different types of data in one system**:

**Type 1: Consensus Artifacts** (~300-350 memories, 52-61% of total)
- Structured JSON from agents (gemini, claude, gpt_pro outputs)
- Each SPEC generates: 4 agents × 6 stages = 24 consensus memories
- Stored at importance: 8 (inflates average)
- **Low value after synthesis**: Raw artifacts rarely referenced
- **High volume**: Every /speckit.auto adds 24 memories

**Type 2: Curated Knowledge** (~200-250 memories, 35-44% of total)
- Architecture decisions and rationale
- Reusable patterns (borrow checker, cost optimization)
- Bug fixes with context
- Critical discoveries (rate limits, integration issues)
- **High value, reusable**: Referenced across sessions

**The Conflict**: Mixing structured data with knowledge base
- Pollutes search results (noise vs signal)
- Wrong tool for the job (semantic search for structured data?)
- Bloats knowledge base (574 memories, 552 tags)
- Different query patterns (SQL vs semantic)

---

## 2. Proposed Solution

### Separation of Concerns

**Consensus Database** (new, SQLite):
```
Purpose: Store structured consensus artifacts
Format:  Relational/JSON database
Queries: SQL (fast, structured)
Volume:  High (24 per SPEC, ~2,400/year at 100 SPECs)
Retention: 90 days after SPEC complete, then purge
Access: Via consensus_db module in Rust
```

**Local-Memory** (existing, MCP):
```
Purpose: Curated knowledge base
Format:  Semantic memory with tags
Queries: Semantic search, AI analysis
Volume:  Low (20-30/month quality storage)
Retention: Permanent for high-value (importance ≥8)
Access: Via MCP tools (existing)
```

---

## 3. Architecture Design

### Database Schema (Consensus DB)

```sql
CREATE TABLE consensus_runs (
    id TEXT PRIMARY KEY,           -- UUID
    spec_id TEXT NOT NULL,         -- SPEC-KIT-069
    stage TEXT NOT NULL,           -- plan, tasks, implement, etc.
    run_timestamp TIMESTAMP,
    synthesis_ok BOOLEAN,
    degraded BOOLEAN,
    conflicts INTEGER,
    INDEX idx_spec_stage (spec_id, stage)
);

CREATE TABLE agent_artifacts (
    id TEXT PRIMARY KEY,           -- UUID
    run_id TEXT REFERENCES consensus_runs(id),
    agent TEXT NOT NULL,           -- gemini, claude, gpt_pro, code
    model TEXT,                    -- gemini-2.5-flash, etc.
    prompt_version TEXT,
    artifact_json TEXT,            -- Full JSON output
    importance INTEGER,            -- Still track, but for filtering
    created_at TIMESTAMP,
    INDEX idx_run_agent (run_id, agent)
);

CREATE TABLE synthesis_results (
    id TEXT PRIMARY KEY,
    run_id TEXT REFERENCES consensus_runs(id),
    agreements TEXT,               -- JSON array
    conflicts TEXT,                -- JSON array
    verdict TEXT,                  -- Final decision
    quality_score REAL,
    created_at TIMESTAMP
);

-- Auto-purge old data
CREATE TRIGGER auto_archive_old_consensus
AFTER INSERT ON consensus_runs
BEGIN
    DELETE FROM agent_artifacts
    WHERE created_at < datetime('now', '-90 days');
END;
```

### Query Patterns

**Consensus Queries** (SQL):
```rust
// Find all consensus for a SPEC
db.query("SELECT * FROM consensus_runs WHERE spec_id = ?", spec_id);

// Get agent artifacts for a stage
db.query("SELECT * FROM agent_artifacts WHERE run_id = ? AND agent = ?", run_id, "gemini");

// Find conflicts
db.query("SELECT * FROM synthesis_results WHERE conflicts != '[]'");

// Purge old data
db.execute("DELETE FROM agent_artifacts WHERE created_at < date('now', '-90 days')");
```

**Knowledge Queries** (local-memory, unchanged):
```rust
// Find patterns about a topic
mcp.search("borrow checker patterns", domain: "rust");

// Get architecture decisions
mcp.search("architecture", importance_min: 9);

// Find related knowledge
mcp.relationships(memory_id, relationship_type: "find_related");
```

---

## 4. Migration Impact on SPEC-KIT-071

### Massive Simplification!

**BEFORE** (if we kept consensus in local-memory):
- 574 memories to clean up
- 552 tags to consolidate
- Consensus artifacts mixed with knowledge
- Hard to distinguish value

**AFTER** (with separation):
- ~200-250 knowledge memories to curate (much more manageable!)
- ~150-200 tags (consensus tags eliminated)
- Clean knowledge base
- Easy to identify valuable content

**SPEC-KIT-071 Scope Reduction**:
- Cleanup effort: 8-12h → 4-6h (50% reduction!)
- Much easier to curate 250 memories than 574
- Consensus artifacts handled by database (auto-purge)

---

## 5. Implementation Approach

### Phase 1: Database Setup (4-6 hours)
- Create SQLite schema
- Add consensus_db module to TUI
- Migration script: Extract consensus from local-memory → DB
- Verify data integrity

### Phase 2: Update Consensus Module (6-8 hours)
- Modify consensus.rs to use DB instead of local-memory
- Update fetch_memory_entries() → fetch_consensus_artifacts()
- Keep synthesis in local-memory (high-value summary)
- Update agent prompts (store to DB, not local-memory)

### Phase 3: Cleanup (2-3 hours)
- Delete consensus artifacts from local-memory
- Verify knowledge-only remains
- Much simpler than original cleanup!

**Total**: 12-17 hours for SPEC-KIT-072
**Enables**: SPEC-KIT-071 becomes 4-6h instead of 14-20h

---

## Question 3 (continuing SPEC-KIT-071): With consensus separated, what's your cleanup target?

**New context**: If we move consensus artifacts out, local-memory will have ~200-250 **knowledge** memories only.

**Of those 200-250 knowledge memories**:

**A. Light Cleanup** → Keep most (~180-200 memories)
- Just remove obvious waste (byterover, duplicate sessions)
- Keep everything else
- 25-35% reduction from current knowledge subset

**B. Moderate Cleanup** → Curate to quality (~120-150 memories)
- Remove redundant knowledge
- Keep only reusable patterns
- 40-50% reduction

**C. Aggressive Curation** → Elite knowledge base (~80-100 memories)
- Keep only high-value, frequently referenced
- Archive everything else
- 50-60% reduction

**MY RECOMMENDATION**: Start with **B (Moderate)** after SPEC-KIT-072
- Consensus DB handles high-volume structured data
- Local-memory becomes curated knowledge (120-150 quality memories)
- Clean, manageable, high-value

**Your preference for knowledge base size?** A, B, or C?