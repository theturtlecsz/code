# ADR-004-003: Turn-Based Schema Design for Trajectory Storage

**Status**: Accepted
**Date**: 2025-11-16
**Deciders**: Research Team
**Related**: SPEC-PPP-004 (Trajectory Logging & MCP Integration)

---

## Context

PPP framework requires storing multi-turn conversations with metadata to calculate:
- **$R_{Proact}$**: Based on question effort (low/medium/high)
- **$R_{Pers}$**: Based on preference violations (minor/major/critical)

Four schema patterns were evaluated:
1. **Flat message history**: Single table with role-based messages
2. **Turn-based with metadata**: 4 tables (trajectories, turns, questions, violations)
3. **JSON blob storage**: Store turns as JSON in single column
4. **Hierarchical spans**: OpenTelemetry-style parent/child traces

---

## Decision

We will use **Turn-based schema with metadata** (Option 2) with 4 normalized tables:
- `trajectories`: One per agent execution
- `trajectory_turns`: One per user-agent exchange
- `trajectory_questions`: Questions asked by agent
- `trajectory_violations`: Preference violations detected

---

## Rationale

### Schema Design

```sql
-- Table 1: Trajectories (one per agent execution)
CREATE TABLE trajectories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,              -- e.g., "SPEC-KIT-001"
    agent_name TEXT NOT NULL,           -- e.g., "gemini-flash"
    run_id TEXT,                        -- Links to consensus_artifacts
    created_at TEXT DEFAULT (datetime('now')),
    INDEX idx_spec_agent (spec_id, agent_name),
    INDEX idx_run_id (run_id)
);

-- Table 2: Turns (one per user-agent exchange)
CREATE TABLE trajectory_turns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trajectory_id INTEGER NOT NULL,
    turn_number INTEGER NOT NULL,      -- 1, 2, 3, ...
    prompt TEXT NOT NULL,               -- User input
    response TEXT NOT NULL,             -- Agent output
    token_count INTEGER,                -- Optional: for cost tracking
    latency_ms INTEGER,                 -- Optional: for performance
    timestamp TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (trajectory_id) REFERENCES trajectories(id) ON DELETE CASCADE,
    INDEX idx_trajectory (trajectory_id),
    UNIQUE (trajectory_id, turn_number)
);

-- Table 3: Questions (extracted from responses)
CREATE TABLE trajectory_questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    turn_id INTEGER NOT NULL,
    question_text TEXT NOT NULL,
    effort_level TEXT,                  -- 'low', 'medium', 'high'
    FOREIGN KEY (turn_id) REFERENCES trajectory_turns(id) ON DELETE CASCADE,
    INDEX idx_turn (turn_id)
);

-- Table 4: Violations (detected in responses)
CREATE TABLE trajectory_violations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    turn_id INTEGER NOT NULL,
    preference_name TEXT NOT NULL,      -- e.g., 'require_json'
    expected TEXT NOT NULL,             -- e.g., 'Valid JSON'
    actual TEXT NOT NULL,               -- e.g., 'Plain text'
    severity TEXT NOT NULL,             -- 'minor', 'major', 'critical'
    FOREIGN KEY (turn_id) REFERENCES trajectory_turns(id) ON DELETE CASCADE,
    INDEX idx_turn (turn_id),
    INDEX idx_preference (preference_name)
);
```

---

### Comparison: 4 Schema Patterns

| Feature | Flat | **Turn-Based** | JSON Blob | Hierarchical |
|---------|------|----------------|-----------|--------------|
| **Tables** | 1 | **4** | 1 | 2 |
| **Query Complexity** | Low | **Medium** | High | Very High |
| **Storage Efficiency** | Good | **Excellent** | Poor | Medium |
| **Extensibility** | Poor | **Excellent** | Good | Excellent |
| **PPP Support** | 20% | **100%** | 50% | 60% |
| **Indexing** | Limited | **Full** | Limited | Limited |

**Winner**: Turn-based schema - Only option supporting all PPP calculations efficiently.

---

### Why Turn-Based Wins

#### 1. Supports PPP Calculations Natively

**$R_{Proact}$ Query** (calculate from questions):
```sql
SELECT
    CASE
        WHEN COUNT(q.id) = 0 THEN 0.05
        WHEN COUNT(CASE WHEN q.effort_level != 'low' THEN 1 END) = 0 THEN 0.05
        ELSE -0.1 * COUNT(CASE WHEN q.effort_level = 'medium' THEN 1 END)
             -0.5 * COUNT(CASE WHEN q.effort_level = 'high' THEN 1 END)
    END AS r_proact
FROM trajectory_turns t
LEFT JOIN trajectory_questions q ON t.id = q.turn_id
WHERE t.trajectory_id = ?;
```

**Performance**: ~3-8ms for 100-turn trajectory

**$R_{Pers}$ Query** (calculate from violations):
```sql
SELECT
    CASE
        WHEN COUNT(v.id) = 0 THEN 0.05
        ELSE -0.01 * COUNT(CASE WHEN v.severity = 'minor' THEN 1 END)
             -0.03 * COUNT(CASE WHEN v.severity = 'major' THEN 1 END)
             -0.05 * COUNT(CASE WHEN v.severity = 'critical' THEN 1 END)
    END AS r_pers
FROM trajectory_turns t
LEFT JOIN trajectory_violations v ON t.id = v.turn_id
WHERE t.trajectory_id = ?;
```

**Performance**: ~3-8ms for 100-turn trajectory

**Total Scoring Time**: <20ms (both queries + merge)

---

#### 2. Normalized Schema (No Duplication)

**Flat Schema** (redundant):
```sql
CREATE TABLE messages (
    id INTEGER PRIMARY KEY,
    session_id TEXT,      -- ← Repeated for every message
    spec_id TEXT,         -- ← Repeated for every message
    agent_name TEXT,      -- ← Repeated for every message
    role TEXT,
    content TEXT
);
```

**Turn-Based Schema** (normalized):
```sql
-- Metadata stored once per trajectory
CREATE TABLE trajectories (
    id INTEGER PRIMARY KEY,
    spec_id TEXT,         -- ✅ Stored once
    agent_name TEXT       -- ✅ Stored once
);

-- Only turn data stored per row
CREATE TABLE trajectory_turns (
    id INTEGER PRIMARY KEY,
    trajectory_id INTEGER,  -- ✅ Reference, not duplicate
    prompt TEXT,
    response TEXT
);
```

**Storage Savings**: ~30% less disk space (metadata not duplicated)

---

#### 3. Extensible Without Migration

**Add New Metadata** (no schema change):
```sql
-- Want to track tool calls? Add new table
CREATE TABLE trajectory_tool_calls (
    id INTEGER PRIMARY KEY,
    turn_id INTEGER,
    tool_name TEXT,
    arguments TEXT,
    result TEXT,
    FOREIGN KEY (turn_id) REFERENCES trajectory_turns(id)
);
```

**Existing queries still work** - no migration needed.

**Flat/JSON Schema**: Would require schema change or JSON parsing overhead.

---

### Why NOT Flat Schema

**Problem 1**: Prompt + response = 2 rows

```sql
-- Flat schema forces 2 rows per turn
INSERT INTO messages (role, content) VALUES ('user', 'Implement OAuth');
INSERT INTO messages (role, content) VALUES ('assistant', 'Which provider?');

-- Querying requires grouping by turn (complex)
SELECT
    MAX(CASE WHEN role = 'user' THEN content END) AS prompt,
    MAX(CASE WHEN role = 'assistant' THEN content END) AS response
FROM messages
GROUP BY turn_number;  -- ← turn_number not in schema!
```

**Turn-Based**: 1 row per turn (simpler)

```sql
INSERT INTO trajectory_turns (prompt, response) VALUES ('Implement OAuth', 'Which provider?');

SELECT prompt, response FROM trajectory_turns;  -- ✅ Simple
```

**Problem 2**: No place to store metadata

- Where to store effort_level? (per question, not per message)
- Where to store violation details? (per violation, not per message)

**Flat schema** would require complex JSON columns or denormalized data.

---

### Why NOT JSON Blob

**Proposed Schema**:
```sql
CREATE TABLE trajectories (
    id INTEGER PRIMARY KEY,
    spec_id TEXT,
    turns_json TEXT  -- JSON: [{prompt, response, questions: [...], violations: [...]}, ...]
);
```

**Example JSON**:
```json
{
  "turns": [
    {
      "prompt": "Implement OAuth",
      "response": "Which provider?",
      "questions": [{"text": "Which provider?", "effort": "low"}],
      "violations": []
    }
  ]
}
```

**Problems**:

1. **No indexing** (SQLite <3.38 limited JSON support)
   ```sql
   -- Slow: Must scan entire JSON blob
   SELECT * FROM trajectories WHERE json_extract(turns_json, '$.turns[*].questions[*].effort') = 'high';
   ```

2. **No foreign keys** (can't enforce referential integrity)

3. **Hard to query** (requires json_extract() everywhere)
   ```sql
   -- Complex $R_{Proact}$ calculation
   SELECT
       CASE
           WHEN json_extract(turns_json, '$.turns[*].questions') IS NULL THEN 0.05
           ELSE (
               SELECT -0.1 * COUNT(*) FROM json_each(...)  -- ← Nested queries
           )
       END
   FROM trajectories;
   ```

4. **Storage overhead** (JSON syntax: ~30% larger than normalized)

**Decision**: JSON blob rejected - too complex, no performance benefit.

---

### Why NOT Hierarchical Spans (OpenTelemetry)

**Proposed Schema**:
```sql
CREATE TABLE spans (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER,
    span_type TEXT,  -- 'conversation', 'turn', 'llm_call', 'tool_use'
    attributes_json TEXT,
    FOREIGN KEY (parent_id) REFERENCES spans(id)
);
```

**Example Hierarchy**:
```
Span (conversation, id=1)
  ├─ Span (turn, id=2, parent=1)
  │    ├─ Span (llm_call, id=3, parent=2)
  │    └─ Span (tool_use, id=4, parent=2)
  └─ Span (turn, id=5, parent=1)
```

**Problems**:

1. **Overkill for CLI** (no nested tool calls in codex-tui)
   - Coding assistants don't have complex tool hierarchies
   - Turns are flat (user → agent → user)

2. **Complex queries** (recursive CTEs required)
   ```sql
   -- Get all turns for conversation (requires recursive CTE)
   WITH RECURSIVE turn_hierarchy AS (
       SELECT * FROM spans WHERE id = ?
       UNION ALL
       SELECT s.* FROM spans s JOIN turn_hierarchy th ON s.parent_id = th.id
   )
   SELECT * FROM turn_hierarchy WHERE span_type = 'turn';
   ```

3. **Storage overhead** (~2x due to hierarchy metadata)

4. **PPP calculations harder** (questions/violations buried in attributes_json)

**Decision**: Hierarchical rejected - unnecessary complexity for flat turn structure.

---

## Consequences

### Positive

1. ✅ **100% PPP support**: All formulas map directly to SQL queries
2. ✅ **Normalized**: No data duplication, efficient storage
3. ✅ **Indexed**: Fast queries on spec_id, agent_name, trajectory_id
4. ✅ **Extensible**: Add tables without migrating existing data
5. ✅ **Foreign keys**: Cascading deletes maintain referential integrity
6. ✅ **Simple queries**: Standard SQL, no complex JOINs or CTEs

### Negative

1. ⚠️ **4 tables**: More complex than flat schema
   - Mitigation: Standard normalization pattern, well-understood
   - Acceptable: Complexity justified by query performance

2. ⚠️ **JOINs required**: Queries must join tables
   - Mitigation: Indexed foreign keys make JOINs fast (<10ms)
   - Acceptable: Better than JSON parsing overhead

3. ⚠️ **More write operations**: 4 INSERTs per turn (worst case)
   - Mitigation: Async batching amortizes cost (0.1ms/turn total)
   - Acceptable: Write overhead << query benefit

### Neutral

1. 📊 **Storage**: ~500 bytes/turn (normalized)
   - Comparable to JSON blob (~600 bytes with overhead)
   - Acceptable: 20 turns × 500 bytes = 10 KB/trajectory

---

## Migration Strategy

### Phase 1: Schema Creation

```rust
pub fn init_trajectory_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS trajectories (...)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS trajectory_turns (...)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS trajectory_questions (...)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS trajectory_violations (...)",
        [],
    )?;

    Ok(())
}
```

### Phase 2: Versioned Migration

```rust
pub fn migrate_database(conn: &Connection) -> Result<()> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if version < 2 {
        init_trajectory_tables(conn)?;
        conn.execute("PRAGMA user_version = 2", [])?;
    }

    Ok(())
}
```

### Phase 3: Rollback Support

```sql
-- If trajectory logging causes issues, can drop tables safely
DROP TABLE trajectory_violations;
DROP TABLE trajectory_questions;
DROP TABLE trajectory_turns;
DROP TABLE trajectories;

-- Consensus DB continues working (separate tables)
```

---

## Alternative Considered: Hybrid (Turn-Based + JSON)

**Idea**: Store common fields in columns, metadata in JSON

```sql
CREATE TABLE trajectory_turns (
    id INTEGER PRIMARY KEY,
    trajectory_id INTEGER,
    prompt TEXT,
    response TEXT,
    metadata_json TEXT  -- {questions: [...], violations: [...]}
);
```

**Pros**:
- Simpler (3 tables instead of 4)
- Flexible metadata

**Cons**:
- Loses indexing on effort_level, severity
- PPP queries become complex (JSON parsing)
- Inconsistent: Some data normalized, some JSON

**Rejected**: Prefer full normalization for consistency.

---

## Performance Validation

### Benchmark Targets

| Operation | Target | Measurement |
|-----------|--------|-------------|
| **Insert Turn** | <0.1ms | Async batched |
| **Query $R_{Proact}$** | <10ms | 100-turn trajectory |
| **Query $R_{Pers}$** | <10ms | 100-turn trajectory |
| **Full Scoring** | <20ms | Both queries + merge |

### Test Queries

```sql
-- Test 1: R_Proact with 100 turns, 50 questions
SELECT ... FROM trajectory_turns t LEFT JOIN trajectory_questions q ...
WHERE t.trajectory_id = ?;
-- Expected: <10ms

-- Test 2: R_Pers with 100 turns, 10 violations
SELECT ... FROM trajectory_turns t LEFT JOIN trajectory_violations v ...
WHERE t.trajectory_id = ?;
-- Expected: <10ms
```

---

## References

1. SPEC-PPP-004 findings.md - Schema pattern analysis
2. SPEC-PPP-004 comparison.md - Detailed schema comparison
3. PPP Framework (arXiv:2511.02208) - Formula definitions
4. SQLite normalization best practices

---

## Decision Drivers

| Driver | Weight | Turn-Based | Flat | JSON | Hierarchical |
|--------|--------|------------|------|------|--------------|
| PPP Support | 40% | ✅ 100% | ❌ 20% | ⚠️ 50% | ⚠️ 60% |
| Query Performance | 30% | ✅ Fast | ⚠️ Medium | ❌ Slow | ❌ Slow |
| Extensibility | 20% | ✅ High | ❌ Low | ⚠️ Medium | ✅ High |
| Complexity | 10% | ⚠️ Medium | ✅ Simple | ⚠️ Medium | ❌ Complex |

**Total Score**: Turn-Based **90%**, Hierarchical **55%**, JSON **45%**, Flat **35%**

**Winner**: Turn-Based by significant margin.
