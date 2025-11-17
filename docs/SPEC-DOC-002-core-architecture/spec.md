# SPEC-DOC-002: Core Architecture Documentation

**Status**: Pending
**Priority**: P0 (High)
**Estimated Effort**: 16-20 hours
**Target Audience**: Contributors, system architects
**Created**: 2025-11-17

---

## Objectives

Document the complete internal architecture of the theturtlecsz/code project:
1. System overview and design philosophy
2. Cargo workspace structure (24 crates, dependencies)
3. TUI architecture (Ratatui, async/sync boundaries)
4. Core execution system (agent orchestration, tmux management)
5. MCP integration (native client, connection management)
6. Database layer (SQLite, schema, transactions)
7. Configuration system (5-tier precedence, hot-reload)

---

## Scope

### In Scope

**System Architecture**:
- High-level component diagram
- Data flow diagrams
- Integration points between subsystems
- Fork-specific additions (98.8% isolation from upstream)

**Cargo Workspace** (24 crates):
- Crate dependency graph
- Purpose of each crate
- Inter-crate dependencies
- Build profiles (dev-fast, release, perf)

**TUI System**:
- Ratatui architecture
- ChatWidget structure (912K LOC mod.rs)
- Async/sync boundary handling (Handle::block_on pattern)
- Friend module pattern for spec-kit isolation
- Widget lifecycle and rendering

**Core Execution**:
- Agent spawning and orchestration
- Tmux session management
- Model provider clients (OpenAI, Anthropic, Google)
- Protocol implementation
- Timeout and retry logic (AR-1 through AR-4)

**MCP Integration**:
- Native client implementation (5.3× speedup)
- App-level shared connection manager (ARCH-005)
- Server lifecycle management
- Tool invocation patterns

**Database Layer**:
- SQLite schema (consensus_artifacts.db)
- Transaction handling (IMMEDIATE mode)
- File locking (fs2 crate, ARCH-007)
- Retry logic and error handling
- Auto-vacuum strategy (99.95% reduction)

**Configuration System**:
- 5-tier precedence (CLI > shell > profile > TOML > defaults)
- Hot-reload mechanism (config_reload.rs, 300ms debounce)
- Profile system
- Environment variable overrides

### Out of Scope

- User-facing configuration guide (see SPEC-DOC-006)
- Spec-kit framework details (see SPEC-DOC-003)
- Testing infrastructure (see SPEC-DOC-004)
- Security implementation (see SPEC-DOC-007)

---

## Deliverables

### Primary Documentation

1. **content/system-overview.md** - Architecture overview, component diagram
2. **content/cargo-workspace.md** - Workspace structure, crate guide
3. **content/tui-architecture.md** - Ratatui, async/sync, ChatWidget
4. **content/core-execution.md** - Agent orchestration, tmux, providers
5. **content/mcp-integration.md** - Native client, connection manager
6. **content/database-layer.md** - SQLite, schema, transactions
7. **content/configuration-system.md** - 5-tier precedence, hot-reload

### Supporting Materials

- **evidence/diagrams/** - Architecture diagrams, data flow charts
- **adr/** - Key architectural decisions (if new decisions documented)

---

## Success Criteria

- [ ] Complete component diagram created
- [ ] All 24 crates documented with purposes
- [ ] Async/sync boundary patterns explained with examples
- [ ] MCP integration fully documented (connection lifecycle, retry logic)
- [ ] SQLite schema documented with ER diagram
- [ ] Configuration precedence clearly illustrated
- [ ] All file paths reference actual source code locations

---

## Related SPECs

- SPEC-DOC-000 (Master)
- SPEC-DOC-001 (User Onboarding - references architecture concepts)
- SPEC-DOC-003 (Spec-Kit - detailed spec-kit architecture)
- SPEC-DOC-004 (Testing - test infrastructure architecture)
- SPEC-DOC-005 (Development - build system details)

---

**Status**: Structure defined, content pending
