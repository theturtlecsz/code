# SPEC-931D Analysis: External Contracts & API Stability

**Session**: 2025-11-13
**Analyst**: Claude (Sonnet 4.5)
**Scope**: External API contracts, breaking change impact, versioning strategy
**Context**: Child spec 4/10 in SPEC-931 architectural deep dive (Group B: Constraints)

---

## Executive Summary

This analysis catalogs all external contracts in the codex-rs agent orchestration system, identifying 4 contract categories with 72+ individual API surfaces that must remain stable or version gracefully.

**Key Findings**:
- 23 slash commands (41 total names with aliases) exposed to users
- 2 major protocol enums (Op, EventMsg) with 20+ variants each
- 3 evidence formats (consensus, commands, quality gates)
- Extensive backward compatibility via command aliases (18 legacy names supported)
- Critical breaking change risk: Evidence schema, consensus database, slash command names

**Critical Stability Requirements**:
1. Command names must remain stable (backward compat via aliases)
2. Evidence JSON schemas cannot break without migration
3. Protocol enums cannot reorder variants (serde serialization)
4. SQLite schema requires versioned migrations
5. MCP protocol changes require coordination

---

## 1. USER-FACING CONTRACTS

### 1.1 Slash Commands (23 commands, 41 total names)

**Primary Interface**: Command registry pattern with aliases for backward compatibility.

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs:148-187`

#### Command Inventory

**Special Commands (8)**:
- `speckit.new` (alias: `new-spec`) - Native SPEC creation
- `speckit.specify` - PRD generation (single-agent)
- `speckit.auto` - Full pipeline orchestration
- `speckit.status` (alias: `spec-status`) - Status dashboard
- `speckit.verify` - Verification report
- `spec-consensus` - Consensus inspection
- `speckit.constitution` - ACE constitution pinning
- `speckit.ace-status` (alias: `ace-status`) - ACE playbook status

**Stage Commands (6)** - prompt-expanding:
- `speckit.plan` (aliases: `spec-plan`, `spec-ops-plan`)
- `speckit.tasks` (aliases: `spec-tasks`, `spec-ops-tasks`)
- `speckit.implement` (aliases: `spec-implement`, `spec-ops-implement`)
- `speckit.validate` (aliases: `spec-validate`, `spec-ops-validate`)
- `speckit.audit` (aliases: `spec-audit`, `spec-ops-audit`)
- `speckit.unlock` (aliases: `spec-unlock`, `spec-ops-unlock`)

**Quality Commands (3)** - native heuristics:
- `speckit.clarify` - Ambiguity detection (FREE, <1s)
- `speckit.analyze` - Consistency checking (FREE, <1s)
- `speckit.checklist` - Quality scoring (FREE, <1s)

**Guardrail Commands (6)** - shell wrappers:
- `guardrail.plan` (alias: `spec-ops-plan`)
- `guardrail.tasks` (alias: `spec-ops-tasks`)
- `guardrail.implement` (alias: `spec-ops-implement`)
- `guardrail.validate` (alias: `spec-ops-validate`)
- `guardrail.audit` (alias: `spec-ops-audit`)
- `guardrail.unlock` (alias: `spec-ops-unlock`)

**Meta Commands (2)**:
- `guardrail.auto` (alias: `spec-ops-auto`) - Redirects to `speckit.auto`
- `spec-evidence-stats` - Evidence footprint monitoring

**Contract Details**:
- **Signature**: `/<command-name> <SPEC-ID> [options]`
- **Returns**: TUI history cells (Notice, Error types)
- **Backward Compat**: 18 legacy aliases maintained
- **Versioning**: No version field in command syntax (implicit v1)

**Consumer**:
- `routing.rs:70-155` (command dispatcher)
- User scripts/automation
- Documentation examples in `docs/spec-kit/`

**Breaking Change Impact**:
- Renaming primary command breaks documentation/examples
- Removing aliases breaks existing user scripts
- Changing argument syntax breaks automation
- Changing output format breaks script parsers

#### Questions

Q1: What is the deprecation policy for legacy aliases? Can we remove `spec-ops-*` in a future major version?

Q2: Should command names be versioned? (e.g., `/speckit.v2.plan`)

Q3: What happens if a user script calls a removed command? Error message? Automatic upgrade suggestion?

Q4: Are there telemetry metrics on alias usage to inform deprecation decisions?

Q5: Should we formalize a command contract test suite? (e.g., "these 41 names must resolve")

### 1.2 Configuration Format

**Location**: `codex-rs/core/src/config_types.rs`, `config.toml` in projects

**Contract**: TOML-based configuration with structured sections

#### Key Configuration Sections

**Subagent Commands** (used by prompt-expanding commands):
```toml
[[subagent-commands]]
name = "speckit.plan"
agents = ["gemini-flash", "claude-haiku", "gpt5-medium"]
prompt = "..."
```

**Agents Configuration**:
```toml
[[agents]]
id = "gemini-flash"
provider = "gemini"
model = "gemini-2.0-flash-exp"
```

**ACE Configuration**:
```toml
[ace]
server = "ace-playbook"
enabled = true
```

**Breaking Change Impact**:
- Renaming config keys breaks existing config.toml files
- Changing config structure requires migration tooling
- Removing config options breaks backward compat

#### Questions

Q6: Is there a config schema version field? How do we detect outdated configs?

Q7: What happens when a user upgrades with an old config.toml? Silent failures? Warnings?

Q8: Should we provide a config migration tool? (e.g., `code config upgrade`)

Q9: Are there default fallbacks for missing config keys?

### 1.3 File Outputs

#### SPEC.md Tracker Format

**Location**: `SPEC.md` in project root

**Format**:
```markdown
| Status | SPEC ID | Description | Tasks | Evidence | Priority |
|--------|---------|-------------|-------|----------|----------|
| In Progress | SPEC-KIT-### | Feature description | T12, T15 | [📊](docs/...) | High |
```

**Contract Fields**:
- Status: `Backlog | In Progress | In Review | Blocked | Done`
- SPEC ID: `SPEC-KIT-###` pattern
- Tasks: Comma-separated task IDs
- Evidence: Markdown link to evidence directory

**Consumer**:
- Users reading project status
- CI/CD scripts parsing SPEC status
- Documentation generators

**Breaking Change Impact**:
- Changing status values breaks parsers
- Removing columns breaks automation
- Changing SPEC ID format breaks evidence path resolution

#### Questions

Q10: Is the SPEC.md format versioned? What if we need to add/remove columns?

Q11: Should we provide a SPEC.md migration script for format changes?

Q12: Are there tools that depend on parsing SPEC.md? (CI/CD, dashboards)

#### Task Files (tasks.md)

**Location**: `docs/SPEC-<id>-<slug>/tasks.md`

**Format**:
```markdown
| Order | Task ID | Title | Status | PRD | Branch | PR | Notes |
|-------|---------|-------|--------|-----|--------|----|----|
| 1 | T12 | Task title | Done | R1, R2 | feat/xyz | #123 | ... |
```

**Contract**: Markdown table with 8 required columns

**Breaking Change Impact**:
- Column reordering breaks parsers
- Renaming columns breaks automation
- Changing status values breaks scripts

#### Questions

Q13: Are task files machine-parsed anywhere? (dashboards, metrics)

Q14: Should we standardize on a parseable format? (JSON, YAML)

#### Plan Files (plan.md)

**Location**: `docs/SPEC-<id>-<slug>/plan.md`

**Format**: Semi-structured Markdown with required sections

**Required Sections**:
```markdown
# Plan: <feature / spec-id>
## Inputs
## Work Breakdown
## Acceptance Mapping
## Risks & Unknowns
## Consensus & Risks (Multi-AI)
## Exit Criteria (Done)
```

**Consumer**:
- Users reviewing plans
- Consensus agents parsing for context
- Quality gates validating structure

**Breaking Change Impact**:
- Removing required sections breaks validation
- Renaming sections breaks parsers
- Changing section order may break tooling

#### Questions

Q15: Is the plan.md structure documented as a contract? Or implementation detail?

Q16: Should we validate plan.md structure programmatically? (schema validation)

Q17: Do consensus agents depend on specific section names/order?

---

## 2. SYSTEM-FACING CONTRACTS

### 2.1 Protocol API (Op & EventMsg enums)

**Location**: `codex-rs/core/src/protocol.rs`

#### Submission Operations (Op enum)

**Key Variants**:
- `ConfigureSession` - Session initialization with model config
- `UserInput` - User message submission
- `Interrupt` - Task abortion
- `ExecApproval` / `PatchApproval` - Approval flows
- `RunProjectCommand` - Custom command execution
- `Review` - Code review requests
- `Shutdown` - Graceful termination

**Contract**: Tagged union with serde serialization

```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Op { ... }
```

**Stability**: `#[non_exhaustive]` prevents external exhaustive matching

**Breaking Change Impact**:
- Reordering variants breaks serde deserialization
- Renaming variants breaks JSON protocol
- Removing variants breaks existing clients
- Adding required fields breaks old clients

#### Response Events (EventMsg enum)

**Key Variants**:
- `TaskStarted` / `TaskComplete` - Lifecycle events
- `AgentMessage` / `AgentMessageDelta` - Text output
- `AgentReasoning` / `AgentReasoningDelta` - Reasoning traces
- `SessionConfigured` - Config ack
- `McpToolCallBegin` / `McpToolCallEnd` - MCP tool events
- `ExecCommandBegin` / `ExecCommandEnd` - Command execution
- `PatchApplyBegin` / `PatchApplyEnd` - Code patching
- `Pro(ProEvent)` - Pro mode events (agent spawning, status)

**Contract**: Tagged union, streaming-compatible

```rust
#[derive(Debug, Clone, Deserialize, Serialize, Display)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventMsg { ... }
```

**Consumer**:
- TUI event loop (`codex-rs/tui/src/app.rs`)
- Future web/API clients
- Event logging/telemetry

**Breaking Change Impact**:
- Reordering breaks binary serialization
- Renaming breaks JSON consumers
- Removing events breaks UI logic
- Adding required fields breaks parsers

#### Questions

Q18: Is the protocol versioned? How do we detect protocol mismatches?

Q19: Should we add a protocol version handshake? (ConfigureSession response includes version)

Q20: Are there protocol evolution strategies? (optional fields, backward-compat extensions)

Q21: Do we support multiple protocol versions simultaneously? (v1, v2 clients)

### 2.2 MCP Protocol Contracts

**Location**: MCP server implementations (ACE Playbook, local-memory, etc.)

**Key MCP Contracts**:

#### ACE Playbook MCP

**Tools**:
- `playbook_slice` - Retrieve bullets by scope
- `playbook_pin` - Pin bullets to ensure inclusion
- `learn` - Update playbook from execution feedback

**Schema**: Defined in `mcp__ace-playbook__*` tool definitions

**Consumer**:
- `ace_client.rs` - ACE MCP client
- Consensus orchestrator
- Learning loop

**Breaking Change Impact**:
- Changing tool names breaks client code
- Renaming parameters breaks agent prompts
- Changing response schemas breaks parsers

#### Questions

Q22: Are MCP tool schemas versioned?

Q23: What happens if MCP server returns incompatible schema? (version mismatch)

Q24: Should we validate MCP responses against schemas?

Q25: Do agent prompts hardcode MCP tool names? (breaking if renamed)

#### Local-Memory MCP

**Tools**:
- `store_memory` - Store knowledge artifacts
- `search` - Semantic/tag/date search
- `analysis` - Question answering, summarization
- `relationships` - Find/create/map relationships
- `domains` / `categories` / `sessions` - Metadata management

**Consumer**:
- Consensus storage (`consensus_db.rs` as consumer, but also stores to local-memory)
- Quality gate artifacts
- User documentation

**Breaking Change Impact**:
- Changing search API breaks consensus lookups
- Removing tools breaks dependent features
- Schema changes break stored data

#### Questions

Q26: Is consensus_db.rs replacing local-memory for consensus artifacts? (SPEC-KIT-072 migration)

Q27: What is the migration path from local-memory to consensus_db?

Q28: Are there other MCP servers in use? (Context7, Readwise, etc.)

### 2.3 Evidence Format Contracts

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/evidence.rs`

#### Evidence Directory Structure

**Layout**:
```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
├── consensus/
│   └── <SPEC-ID>/
│       ├── spec-plan_synthesis.json
│       ├── spec-plan_<timestamp>_<agent>.json
│       └── ...
├── commands/
│   └── <SPEC-ID>/
│       ├── spec-plan_<timestamp>-<session-id>.json
│       └── ...
└── .locks/
    └── <SPEC-ID>.lock
```

**Contract**: Fixed directory hierarchy, JSON file naming conventions

**Breaking Change Impact**:
- Changing directory names breaks file lookups
- Changing file naming breaks pattern matching
- Moving evidence root breaks all references

#### Questions

Q29: Is the evidence directory structure a stable contract?

Q30: Should we version the evidence directory layout? (v1/, v2/)

Q31: What if we need to migrate evidence to a different location? (S3, database)

Q32: Are evidence paths hardcoded in multiple places? (grep for DEFAULT_EVIDENCE_BASE)

#### Consensus Synthesis Schema

**Location**: Example at `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-DEMO/spec-plan_synthesis.json`

**Schema**:
```json
{
  "specId": "SPEC-KIT-###",
  "stage": "spec-plan",
  "timestamp": "2025-10-12T14:44:00Z",
  "status": "ok",
  "agents": {
    "active": [{"model": "...", "role": "..."}],
    "degraded": ["gpt_pro", "gpt_codex"]
  },
  "consensus": {
    "agreements": ["..."],
    "conflicts_resolved": ["..."]
  }
}
```

**Required Fields**:
- `specId` (string)
- `stage` (string)
- `timestamp` (ISO 8601)
- `status` ("ok" | "error")
- `agents.active` (array)
- `consensus.agreements` (array)

**Consumer**:
- Consensus coordinator reading past decisions
- Verification reports
- Telemetry aggregation

**Breaking Change Impact**:
- Removing required fields breaks parsers
- Changing field names breaks deserialization
- Changing enum values breaks validation

#### Questions

Q33: Is the consensus schema versioned? (add `schemaVersion` field?)

Q34: What if we need to extend the schema? (optional fields, nested objects)

Q35: Are there schema validation tests? (JSON Schema, serde checks)

Q36: Do old evidence files get migrated? Or do we support multiple schema versions?

#### Command Telemetry Schema

**Location**: Example at `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-DEMO/spec-implement_*.json`

**Schema**:
```json
{
  "schemaVersion": 1,
  "command": "spec-ops-implement",
  "specId": "SPEC-KIT-###",
  "sessionId": "2025-10-12T15:02:56Z-194826352",
  "timestamp": "2025-10-12T15:10:31Z",
  "lock_status": "locked",
  "hook_status": "ok",
  "policy": {
    "prefilter": {"model": "...", "status": "passed", "note": "..."},
    "final": {"model": "...", "status": "passed", "note": "..."}
  },
  "artifacts": [{"path": "/full/path/to/log"}]
}
```

**Required Fields**:
- `schemaVersion` (integer) - **Good: versioning present!**
- `command` (string)
- `specId` (string)
- `sessionId` (string)
- `timestamp` (ISO 8601)

**Stage-Specific Fields** (documented in CLAUDE.md):
- Plan: `baseline.mode`, `baseline.artifact`, `hooks.session.start`
- Tasks: `tool.status`
- Implement: `lock_status`, `hook_status`
- Validate/Audit: `scenarios[{name, status}]`
- Unlock: `unlock_status`

**Consumer**:
- Guardrail result validation
- `/speckit.verify` report generation
- Evidence stats aggregation

**Breaking Change Impact**:
- Removing stage-specific fields breaks validation
- Changing schema version requires migration
- Changing enum values breaks parsers

#### Questions

Q37: Is schemaVersion respected? (do we have schema v1, v2 parsers?)

Q38: What is the schema evolution policy? (when to bump version?)

Q39: Are stage-specific fields documented in code? (or only in CLAUDE.md?)

Q40: Should we validate telemetry against JSON Schema?

### 2.4 Database Schemas

#### Consensus Database (SQLite)

**Location**: `~/.code/consensus_artifacts.db` (per SPEC-KIT-072)

**Schema** (inferred from `verify.rs:77-82`):

**Table: `agent_executions`**:
```sql
CREATE TABLE agent_executions (
  spec_id TEXT NOT NULL,
  run_id TEXT,
  stage TEXT,
  agent_name TEXT,
  phase_type TEXT,
  spawned_at TIMESTAMP,
  completed_at TIMESTAMP
);
```

**Consumer**:
- `/speckit.verify` command
- Consensus artifact storage (replacing local-memory)
- Audit trail queries

**Breaking Change Impact**:
- Renaming columns breaks SQL queries
- Removing columns breaks existing code
- Adding NOT NULL columns breaks inserts
- Schema changes require migrations

#### Questions

Q41: Is the database schema versioned? (schema_version table?)

Q42: How do we handle schema migrations? (ALTER TABLE scripts, migration framework)

Q43: What happens if a user has old schema? (automatic migration, manual upgrade)

Q44: Are there database schema tests? (migration testing, rollback testing)

Q45: Is the database path stable? (~/.code/consensus_artifacts.db)

#### ACE Playbook Database (SQLite)

**Location**: `~/.code/ace/playbooks_normalized.sqlite3`

**Schema** (inferred from `speckit.ace-status` query):

**Table: `playbook_bullet`**:
```sql
CREATE TABLE playbook_bullet (
  scope TEXT,
  pinned INTEGER,
  score REAL,
  -- other fields...
);
```

**Consumer**:
- `/speckit.constitution` command
- `/speckit.ace-status` command
- ACE MCP server

**Breaking Change Impact**:
- Schema changes break ACE MCP server
- Renaming columns breaks status queries
- Database path change breaks all ACE features

#### Questions

Q46: Who owns the ACE database schema? (codex-rs or ACE MCP server?)

Q47: What if ACE schema changes? (version coordination between repos)

Q48: Should ACE database location be configurable? (currently hardcoded)

---

## 3. BREAKING CHANGE IMPACT MATRIX

| Contract | Consumers | Breaking Change Risk | Migration Cost | Stability Requirement |
|----------|-----------|---------------------|----------------|----------------------|
| **Slash command names** | Users, scripts, docs | HIGH | Medium (alias transition) | STABLE - use aliases for evolution |
| **Command argument syntax** | Users, scripts | HIGH | High (script rewrites) | STABLE - extend only, never change |
| **Command output format** | Script parsers | MEDIUM | Medium (parser updates) | VERSIONED - support multiple formats |
| **SPEC.md format** | CI/CD, parsers | MEDIUM | Medium (migration script) | STABLE - version field recommended |
| **Evidence directory layout** | Evidence lookups | HIGH | High (file migrations) | STABLE - never change paths |
| **Consensus JSON schema** | Parsers, validators | HIGH | Medium (schema migration) | VERSIONED - schemaVersion respected |
| **Command telemetry schema** | Guardrail scripts | MEDIUM | Low (already versioned!) | VERSIONED - schema v1, v2 supported |
| **Protocol Op enum** | All clients | CRITICAL | Very High (protocol break) | STABLE - extend only, never reorder |
| **Protocol EventMsg enum** | TUI, future clients | CRITICAL | Very High (UI rewrite) | STABLE - extend only, never reorder |
| **MCP tool names** | Agent prompts, code | HIGH | High (prompt rewrites) | STABLE - never rename tools |
| **MCP schemas** | MCP clients | HIGH | Medium (client updates) | VERSIONED - version negotiations |
| **SQLite schema** | SQL queries, code | HIGH | High (data migrations) | VERSIONED - schema_version table required |
| **Config.toml format** | User configs | MEDIUM | Medium (config migration tool) | BACKWARD COMPAT - defaults for missing keys |
| **File output formats** (plan.md, tasks.md) | Parsers, validators | MEDIUM | Medium (format migration) | VERSIONED - detect format version |

---

## 4. CONSUMER ANALYSIS

### 4.1 Internal Consumers

**Command Registry Consumers**:
- `routing.rs:70-155` - Command dispatcher (reads registry, dispatches commands)
- `command_registry.rs` tests - Contract validation (41 names, aliases, properties)

**Protocol Consumers**:
- `codex-rs/tui/src/app.rs` - Event loop (processes all EventMsg variants)
- `codex-rs/tui/src/chatwidget/` - Widget handlers (Op submission, event display)
- Future: Web clients, API servers, CLI tools

**Evidence Consumers**:
- `evidence.rs` - Evidence repository implementation
- `handler.rs` - Guardrail result validation
- `verify.rs` - Verification report generation
- `guardrail.rs` - Script wrapper execution

**Database Consumers**:
- `verify.rs:72-89` - Agent execution queries
- `ace_constitution.rs` - ACE playbook queries
- Future: Analytics dashboards, audit reports

### 4.2 External Consumers

**Users**:
- Manual command execution (23 commands)
- Config.toml editing (agents, subagent-commands)
- Evidence file inspection (consensus, telemetry)
- SPEC.md reading (project status)

**Automation Scripts**:
- CI/CD pipelines parsing SPEC.md
- Monitoring scripts checking telemetry
- Dashboard parsers reading evidence JSON
- Build scripts calling slash commands

**Documentation**:
- Examples in `docs/spec-kit/`
- CLAUDE.md command reference
- Tutorial content

**Future Consumers** (anticipated):
- Web dashboard (protocol EventMsg stream)
- REST API (wrapping slash commands)
- IDE plugins (status queries, evidence display)
- Analytics platforms (telemetry ingestion)

---

## 5. VERSIONING & DEPRECATION STRATEGY RECOMMENDATIONS

### 5.1 Command Versioning

**Current**: Implicit v1 (no version in command syntax)

**Recommendation**:
- Maintain current approach (aliases for evolution)
- Document deprecation timeline for legacy aliases
- Add version field to command registry metadata (not in user syntax)

**Migration Path**:
1. Mark old aliases as deprecated in help text
2. Log usage metrics for deprecated commands
3. After 6 months, add deprecation warnings
4. After 12 months, remove aliases (major version bump)

### 5.2 Schema Versioning

**Current**: Mixed (command telemetry has `schemaVersion`, consensus does not)

**Recommendation**:
- Add `schemaVersion` to all JSON schemas
- Implement schema validation with version checks
- Support reading old schemas with migration logic

**Example**:
```rust
match telemetry.schema_version {
    1 => parse_v1(telemetry),
    2 => parse_v2(telemetry),
    _ => return Err(UnsupportedSchemaVersion),
}
```

### 5.3 Protocol Versioning

**Current**: No version handshake, implicit v1

**Recommendation**:
- Add protocol version to `SessionConfigured` event
- Client sends `desired_protocol_version` in `ConfigureSession`
- Server responds with `negotiated_protocol_version`
- Support multiple protocol versions simultaneously

### 5.4 Database Migrations

**Current**: No migration framework detected

**Recommendation**:
- Add `schema_version` table to both databases
- Implement migration runner (e.g., `refinery`, `diesel_migrations`)
- Run migrations on startup before queries
- Test migrations with real data (no data loss)

**Example**:
```sql
CREATE TABLE schema_version (
  version INTEGER PRIMARY KEY,
  applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Migration 001: Initial schema
INSERT INTO schema_version (version) VALUES (1);
```

### 5.5 Deprecation Policy

**Recommendation**:

**Phase 1: Mark Deprecated** (0-6 months)
- Add deprecation notice to help text
- Log usage to telemetry
- No functional change

**Phase 2: Warn Users** (6-12 months)
- Display warning on command execution
- Point to migration guide
- Continue functioning normally

**Phase 3: Remove** (12+ months, major version)
- Remove deprecated features
- Provide migration tooling
- Update documentation

**Example**:
```
$ /spec-ops-plan SPEC-123
⚠️  Warning: /spec-ops-plan is deprecated. Use /speckit.plan instead.
   This alias will be removed in v2.0 (releasing 2026-01-01).
   See migration guide: docs/migration/v1-to-v2.md
```

---

## 6. MIGRATION COMPATIBILITY REQUIREMENTS

### 6.1 Command Migration Requirements

**Requirement**: Users can upgrade without script breakage

**Implementation**:
- Maintain all aliases for 12 months minimum
- Provide command alias mapping tool
- Log deprecated command usage for metrics

**Migration Tool**:
```bash
$ code migrate check-commands script.sh
Found 3 deprecated commands:
  - /spec-ops-plan → /speckit.plan (line 12)
  - /new-spec → /speckit.new (line 24)
  - /spec-auto → /speckit.auto (line 45)

$ code migrate update-commands script.sh --dry-run
$ code migrate update-commands script.sh --in-place
```

### 6.2 Evidence Migration Requirements

**Requirement**: Old evidence files remain readable after schema changes

**Implementation**:
- Add `schemaVersion` to all JSON
- Implement schema migration on read
- Never delete/overwrite old evidence

**Migration Strategy**:
```rust
fn read_evidence(path: &Path) -> Result<Evidence> {
    let raw: Value = serde_json::from_reader(File::open(path)?)?;
    let version = raw.get("schemaVersion").and_then(|v| v.as_u64()).unwrap_or(1);

    match version {
        1 => migrate_v1_to_v2(raw)?.try_into(),
        2 => serde_json::from_value(raw),
        _ => Err(UnsupportedSchema),
    }
}
```

### 6.3 Database Migration Requirements

**Requirement**: Databases upgrade automatically on version change

**Implementation**:
- Detect schema version on startup
- Run pending migrations automatically
- Backup database before migrations
- Rollback on failure

**Example**:
```rust
fn ensure_database_schema(db: &Connection) -> Result<()> {
    let current = get_schema_version(db)?;
    let target = LATEST_SCHEMA_VERSION;

    if current < target {
        backup_database(db)?;
        run_migrations(db, current, target)?;
    }

    Ok(())
}
```

### 6.4 Config Migration Requirements

**Requirement**: Old config.toml files work with new versions

**Implementation**:
- Provide defaults for missing keys
- Warn about deprecated keys
- Auto-migrate on load

**Example**:
```rust
fn load_config(path: &Path) -> Result<Config> {
    let mut config: Config = toml::from_str(&fs::read_to_string(path)?)?;

    // Migrate deprecated keys
    if let Some(old_key) = config.old_key.take() {
        warn!("Config key 'old_key' is deprecated, use 'new_key' instead");
        config.new_key = Some(old_key);
    }

    // Provide defaults
    config.ace = config.ace.or_else(|| Some(AceConfig::default()));

    Ok(config)
}
```

---

## 7. OPEN QUESTIONS SUMMARY

**Commands & Configuration** (15 questions):
- Q1-Q5: Command deprecation policy, versioning, error handling, telemetry
- Q6-Q9: Config schema versioning, migration tooling, defaults
- Q10-Q17: File output format versioning, migration, parsing dependencies

**Protocol & MCP** (11 questions):
- Q18-Q21: Protocol versioning, handshake, evolution strategies
- Q22-Q28: MCP schema versioning, validation, migration paths

**Evidence & Schemas** (13 questions):
- Q29-Q36: Evidence directory stability, schema versioning, migration
- Q37-Q40: Schema version enforcement, evolution policy, validation

**Database** (8 questions):
- Q41-Q48: Schema versioning, migrations, path stability, ownership

**Total**: 47 open questions requiring architectural decisions

---

## 8. RECOMMENDATIONS

### Immediate Actions (High Priority)

1. **Add schema versioning to all JSON outputs**
   - Add `schemaVersion` field to consensus synthesis
   - Document schema version policy
   - Implement version validation

2. **Implement database migration framework**
   - Add `schema_version` table to both databases
   - Create migration runner
   - Test with real data

3. **Document contract stability guarantees**
   - Mark stable vs unstable APIs
   - Define deprecation policy
   - Create migration guides

4. **Add protocol version handshake**
   - Version field in `SessionConfigured`
   - Client version negotiation
   - Multi-version support

### Medium Priority

5. **Create command migration tooling**
   - Automated script updates
   - Deprecation warnings
   - Usage telemetry

6. **Implement config migration**
   - Auto-upgrade on load
   - Deprecation warnings
   - Default fallbacks

7. **Evidence schema validation**
   - JSON Schema definitions
   - Runtime validation
   - Migration testing

### Low Priority

8. **Consumer documentation**
   - Contract stability matrix
   - Breaking change guidelines
   - Migration cookbook

9. **Versioning automation**
   - Schema version bumps in CI
   - Breaking change detection
   - Compatibility testing

---

## 9. CONCLUSION

The codex-rs agent orchestration system has **72+ external API surfaces** across 4 major categories:

1. **User-Facing**: 23 commands (41 names), config format, file outputs
2. **System-Facing**: Protocol (40+ enum variants), MCP contracts
3. **Data Formats**: Evidence schemas (3 types), database schemas (2 databases)
4. **Consumers**: Internal (4 modules), external (users, scripts, docs, future tools)

**Critical Finding**: Most contracts lack explicit versioning, creating high risk of silent breakage on evolution.

**Key Stability Requirements**:
- Commands: Alias-based evolution (already implemented ✅)
- Evidence: Schema versioning needed (partially implemented)
- Protocol: Version handshake needed (not implemented)
- Database: Migration framework needed (not implemented)

**Next Steps**:
1. Answer 47 open questions through stakeholder discussion
2. Implement schema versioning for all JSON outputs
3. Create database migration framework
4. Document contract stability guarantees
5. Build migration tooling for users

**Impact**: Proper contract management enables confident evolution without breaking existing users, automation, or integrations.
