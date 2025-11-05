# PRD: SPEC-KIT-910 - Separate Consensus Database

**Priority**: P1 (High Priority)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

Consensus artifacts (agent outputs, synthesis results, metadata) are currently stored in local-memory MCP, causing operational problems:

**Current Issues**:
1. **Bloat**: Every agent stores full output (importance: 8) → 100s of MB possible in local-memory DB
2. **Performance Degradation**: Query performance degrades as DB grows (full-text search on large dataset)
3. **Data Duplication**: Same content in both local-memory AND evidence repository
4. **Wrong Abstraction**: local-memory designed for curated knowledge (≥8 importance), not structured consensus data
5. **Cleanup Complexity**: Manual cleanup needed, no automated retention policy

**Why This Matters**:
- Local-memory is knowledge base for human-curated insights (architecture decisions, patterns, critical discoveries)
- Consensus artifacts are structured, high-volume, machine-generated data
- Mixing these creates semantic confusion and operational burden

The solution: Separate database for consensus artifacts, reserve local-memory for its intended purpose (curated knowledge).

---

## Goals

### Primary Goal
Migrate consensus artifacts from local-memory MCP to dedicated SQLite database, improving performance and semantic clarity while preserving all consensus functionality.

### Secondary Goals
- Reduce local-memory DB size by 80-90% (remove consensus artifacts)
- Enable fast consensus queries via indexed SQLite (vs full-text search)
- Prepare for advanced consensus analytics (pattern detection, degradation tracking)
- Preserve local-memory for curated knowledge only (importance ≥8)

---

## Requirements

### Functional Requirements

1. **Database Schema Design**
   - Create `consensus.db` SQLite database
   - Schema includes: consensus runs, agent artifacts, synthesis results, metadata
   - Indexes for fast queries: (spec_id, stage), (run_id), (timestamp)

2. **Schema Definition**
   ```sql
   CREATE TABLE consensus_runs (
       run_id TEXT PRIMARY KEY,
       spec_id TEXT NOT NULL,
       stage TEXT NOT NULL,
       timestamp INTEGER NOT NULL,
       consensus_ok BOOLEAN,
       degraded BOOLEAN,
       missing_agents TEXT,  -- JSON array
       conflicts TEXT        -- JSON array
   );

   CREATE TABLE agent_artifacts (
       artifact_id TEXT PRIMARY KEY,
       run_id TEXT NOT NULL,
       agent TEXT NOT NULL,
       content TEXT NOT NULL,
       metadata TEXT,        -- JSON
       created_at INTEGER,
       FOREIGN KEY (run_id) REFERENCES consensus_runs(run_id)
   );

   CREATE TABLE synthesis_results (
       synthesis_id TEXT PRIMARY KEY,
       run_id TEXT NOT NULL,
       synthesized_content TEXT NOT NULL,
       consensus_verdict TEXT,  -- "unanimous", "degraded", "failed"
       metadata TEXT,            -- JSON
       created_at INTEGER,
       FOREIGN KEY (run_id) REFERENCES consensus_runs(run_id)
   );

   CREATE INDEX idx_consensus_spec_stage ON consensus_runs(spec_id, stage);
   CREATE INDEX idx_consensus_timestamp ON consensus_runs(timestamp);
   CREATE INDEX idx_artifacts_run ON agent_artifacts(run_id);
   CREATE INDEX idx_synthesis_run ON synthesis_results(run_id);
   ```

3. **Consensus Storage Module**
   - Create `spec_kit/consensus_db.rs` module
   - Implement `ConsensusDatabase` struct with CRUD operations
   - Methods: `store_run()`, `store_artifact()`, `store_synthesis()`, `query_run()`, `query_artifacts()`

4. **Migration from local-memory**
   - Update `consensus_coordinator.rs` to use `ConsensusDatabase` instead of local-memory
   - Replace `mcp__local-memory__store_memory` calls with native DB writes
   - Preserve query interface (consensus queries still work, faster)

5. **Backward Compatibility**
   - Existing consensus artifacts in local-memory remain (no deletion)
   - New artifacts go to consensus.db only
   - Optional: Migration script to copy old artifacts to consensus.db (separate SPEC)

6. **Query Performance**
   - Fast queries: "Get all artifacts for SPEC-KIT-065 stage:plan" (<50ms)
   - Fast queries: "Get last 10 consensus runs" (<10ms)
   - Replace full-text search with indexed queries

7. **Retention Policy** (integrated with SPEC-KIT-909)
   - Apply same lifecycle as evidence: 30 days active, 90 days archive, delete after
   - Consensus DB cleanup via `/speckit.evidence-cleanup`

### Non-Functional Requirements

1. **Performance Targets**
   - Consensus storage: <10ms per artifact write
   - Consensus query: <50ms for single run retrieval
   - DB size: ~100-200MB per 1000 consensus runs (vs >1GB in local-memory)

2. **Data Integrity**
   - Foreign key constraints enforced (run_id linkage)
   - Atomic transactions (all artifacts for run succeed or rollback)
   - Backup strategy (SQLite file copied periodically)

3. **Maintainability**
   - Clear separation: local-memory (curated knowledge) vs consensus.db (structured data)
   - Easy to query for analytics (standard SQL)
   - Migration path documented for existing data

---

## Technical Approach

### Database Module Implementation

```rust
// spec_kit/consensus_db.rs
use rusqlite::{Connection, params};
use std::path::Path;

pub struct ConsensusDatabase {
    conn: Connection,
}

impl ConsensusDatabase {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;
        Ok(Self { conn })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS consensus_runs (
                run_id TEXT PRIMARY KEY,
                spec_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                consensus_ok BOOLEAN,
                degraded BOOLEAN,
                missing_agents TEXT,
                conflicts TEXT
            );

            CREATE TABLE IF NOT EXISTS agent_artifacts (
                artifact_id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                agent TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                created_at INTEGER,
                FOREIGN KEY (run_id) REFERENCES consensus_runs(run_id)
            );

            CREATE TABLE IF NOT EXISTS synthesis_results (
                synthesis_id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                synthesized_content TEXT NOT NULL,
                consensus_verdict TEXT,
                metadata TEXT,
                created_at INTEGER,
                FOREIGN KEY (run_id) REFERENCES consensus_runs(run_id)
            );

            CREATE INDEX IF NOT EXISTS idx_consensus_spec_stage ON consensus_runs(spec_id, stage);
            CREATE INDEX IF NOT EXISTS idx_consensus_timestamp ON consensus_runs(timestamp);
            CREATE INDEX IF NOT EXISTS idx_artifacts_run ON agent_artifacts(run_id);
            CREATE INDEX IF NOT EXISTS idx_synthesis_run ON synthesis_results(run_id);"
        )?;
        Ok(())
    }

    pub fn store_consensus_run(&self, run: &ConsensusRun) -> Result<()> {
        self.conn.execute(
            "INSERT INTO consensus_runs (run_id, spec_id, stage, timestamp, consensus_ok, degraded, missing_agents, conflicts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                run.run_id,
                run.spec_id,
                run.stage.to_string(),
                run.timestamp.timestamp(),
                run.consensus_ok,
                run.degraded,
                serde_json::to_string(&run.missing_agents)?,
                serde_json::to_string(&run.conflicts)?,
            ],
        )?;
        Ok(())
    }

    pub fn store_agent_artifact(&self, artifact: &AgentArtifact) -> Result<()> {
        self.conn.execute(
            "INSERT INTO agent_artifacts (artifact_id, run_id, agent, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                artifact.artifact_id,
                artifact.run_id,
                artifact.agent,
                artifact.content,
                serde_json::to_string(&artifact.metadata)?,
                artifact.created_at.timestamp(),
            ],
        )?;
        Ok(())
    }

    pub fn store_synthesis_result(&self, synthesis: &SynthesisResult) -> Result<()> {
        self.conn.execute(
            "INSERT INTO synthesis_results (synthesis_id, run_id, synthesized_content, consensus_verdict, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                synthesis.synthesis_id,
                synthesis.run_id,
                synthesis.synthesized_content,
                synthesis.consensus_verdict,
                serde_json::to_string(&synthesis.metadata)?,
                synthesis.created_at.timestamp(),
            ],
        )?;
        Ok(())
    }

    pub fn query_consensus_run(&self, spec_id: &str, stage: &SpecStage) -> Result<Option<ConsensusRun>> {
        let mut stmt = self.conn.prepare(
            "SELECT run_id, spec_id, stage, timestamp, consensus_ok, degraded, missing_agents, conflicts
             FROM consensus_runs
             WHERE spec_id = ?1 AND stage = ?2
             ORDER BY timestamp DESC
             LIMIT 1"
        )?;

        let run = stmt.query_row(params![spec_id, stage.to_string()], |row| {
            Ok(ConsensusRun {
                run_id: row.get(0)?,
                spec_id: row.get(1)?,
                stage: SpecStage::from_str(&row.get::<_, String>(2)?)?,
                timestamp: Utc.timestamp(row.get(3)?, 0),
                consensus_ok: row.get(4)?,
                degraded: row.get(5)?,
                missing_agents: serde_json::from_str(&row.get::<_, String>(6)?)?,
                conflicts: serde_json::from_str(&row.get::<_, String>(7)?)?,
            })
        }).optional()?;

        Ok(run)
    }

    pub fn query_agent_artifacts(&self, run_id: &str) -> Result<Vec<AgentArtifact>> {
        let mut stmt = self.conn.prepare(
            "SELECT artifact_id, run_id, agent, content, metadata, created_at
             FROM agent_artifacts
             WHERE run_id = ?1"
        )?;

        let artifacts = stmt.query_map(params![run_id], |row| {
            Ok(AgentArtifact {
                artifact_id: row.get(0)?,
                run_id: row.get(1)?,
                agent: row.get(2)?,
                content: row.get(3)?,
                metadata: serde_json::from_str(&row.get::<_, String>(4)?)?,
                created_at: Utc.timestamp(row.get(5)?, 0),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(artifacts)
    }
}
```

### Migration in Consensus Coordinator

```rust
// consensus_coordinator.rs (updated)
impl ConsensusCoordinator {
    pub async fn store_consensus_artifacts(
        &self,
        spec_id: &str,
        stage: SpecStage,
        agents: &[AgentOutput],
        synthesis: &SynthesisResult,
    ) -> Result<()> {
        let consensus_db = ConsensusDatabase::new(&self.db_path)?;

        // Store run metadata
        let run = ConsensusRun {
            run_id: Uuid::new_v4().to_string(),
            spec_id: spec_id.to_string(),
            stage,
            timestamp: Utc::now(),
            consensus_ok: synthesis.consensus_ok,
            degraded: synthesis.degraded,
            missing_agents: synthesis.missing_agents.clone(),
            conflicts: synthesis.conflicts.clone(),
        };
        consensus_db.store_consensus_run(&run)?;

        // Store agent artifacts
        for agent_output in agents {
            let artifact = AgentArtifact {
                artifact_id: Uuid::new_v4().to_string(),
                run_id: run.run_id.clone(),
                agent: agent_output.agent.clone(),
                content: agent_output.content.clone(),
                metadata: agent_output.metadata.clone(),
                created_at: Utc::now(),
            };
            consensus_db.store_agent_artifact(&artifact)?;
        }

        // Store synthesis result
        consensus_db.store_synthesis_result(synthesis)?;

        Ok(())
    }

    // OLD (remove after migration):
    // pub async fn store_to_local_memory(&self, ...) { ... }
}
```

---

## Acceptance Criteria

- [ ] `consensus_db.rs` module created with SQLite integration
- [ ] Database schema defined (3 tables: consensus_runs, agent_artifacts, synthesis_results)
- [ ] Indexes created for fast queries (spec_id+stage, run_id, timestamp)
- [ ] `ConsensusDatabase` struct implemented with CRUD methods
- [ ] `consensus_coordinator.rs` updated to use consensus.db instead of local-memory
- [ ] Query performance verified: <50ms for run retrieval, <10ms for inserts
- [ ] Backward compatibility: existing local-memory artifacts remain
- [ ] Unit tests for database operations
- [ ] Integration tests verify consensus storage/retrieval works
- [ ] Documentation updated (`CLAUDE.md`, consensus design doc)
- [ ] Migration guide for existing data (optional cleanup script)

---

## Out of Scope

- **Retroactive migration**: Existing local-memory artifacts not automatically migrated (manual cleanup optional)
- **Local-memory removal**: local-memory still used for curated knowledge (≥8 importance)
- **Advanced analytics**: This SPEC implements storage, not analytics queries
- **Compression**: Consensus data stored uncompressed (can be added later)

---

## Success Metrics

1. **Performance**: Consensus queries 5-10x faster (indexed SQL vs full-text search)
2. **DB Size**: local-memory DB reduced by 80-90% (remove consensus artifacts)
3. **Clarity**: Semantic separation clear (knowledge vs structured data)
4. **Scalability**: Consensus DB handles 10,000+ runs without degradation

---

## Dependencies

### Prerequisites
- ACE framework operational (done ✅ 2025-10-29)
- Consensus coordinator stable

### Downstream Dependencies
- Evidence lifecycle management (SPEC-KIT-909) can apply to consensus.db
- Future consensus analytics rely on this database structure

---

## Estimated Effort

**1-2 days** (as per architecture review)

**Breakdown**:
- Schema design: 2 hours
- Database module implementation: 4 hours
- Consensus coordinator migration: 3 hours
- Unit + integration tests: 3 hours
- Documentation: 2 hours

---

## Priority

**P1 (High Priority)** - Important for operational sustainability and semantic clarity, fits within 30-day action window. Reduces local-memory bloat and improves performance.

---

## Related Documents

- Architecture Review: Section "30-Day Actions, Task 2" (SPEC-KIT-072 reference)
- `spec_kit/consensus_coordinator.rs` - Current consensus logic
- `spec_kit/consensus.rs` - Synthesis implementation
- MEMORY-POLICY.md: Local-memory usage policy (curated knowledge only)
- SPEC-KIT-909: Evidence lifecycle management (retention policy applies here too)
