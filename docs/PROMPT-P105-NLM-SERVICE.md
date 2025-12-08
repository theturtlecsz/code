# P105 Continuation Prompt - NotebookLM Service Integration

## Context

- **P103 Complete** (commit 4042be45d): Native Rust harvester modules
- **P104 Complete** (this session): Mermaid.js call graph + graph_bridge cleanup
- **Key Discovery**: CodeGraphContext MCP only parses Python, not Rust
- **Architecture Shift**: notebooklm-mcp now supports HTTP Service mode (port 3456)

---

## Primary Objective: NotebookLM HTTP Service Bridge

Implement `nlm_service.rs` with full HTTP API integration, replacing CLI spawning.

### Scope Decisions (User Confirmed)

| Decision | Choice | Implication |
|----------|--------|-------------|
| API Scope | **Full research** | Include /api/ask, /api/sources, /api/research/fast, /api/research/deep |
| Service Mgmt | **Full lifecycle** | Add `code architect service start/stop/status` commands |
| Auto-upload | **Auto on refresh** | `code architect refresh` uploads artifacts if service running |

---

## Pre-flight Checklist

```bash
# 1. Verify P104 build
./build-fast.sh

# 2. Test existing architect
./codex-rs/target/dev-fast/code architect refresh --graph --mermaid

# 3. Check notebooklm-mcp version (need v2+ for service mode)
notebooklm --version

# 4. Verify service mode works
notebooklm service start
curl http://127.0.0.1:3456/health
notebooklm service stop
```

---

## Implementation Tasks

### Phase 1: Core HTTP Client (`nlm_service.rs`)

Create `core/src/architect/nlm_service.rs`:

```rust
//! NotebookLM HTTP Service client.
//!
//! Connects to notebooklm-mcp service (default: http://127.0.0.1:3456)
//! with lazy service spawning and health monitoring.

pub struct NlmService {
    base_url: String,
    client: reqwest::Client,
}

impl NlmService {
    /// Ensure service is running, spawn if needed.
    pub async fn ensure_running() -> Result<Self>;

    /// Health check with queue/session stats.
    pub async fn health(&self) -> Result<HealthStatus>;

    /// Ask a question to a notebook.
    pub async fn ask(&self, notebook: &str, question: &str) -> Result<AskResponse>;

    /// Add a source (website, file, text) to a notebook.
    pub async fn add_source(&self, notebook: &str, source: Source) -> Result<()>;

    /// Fast research (parallel web search).
    pub async fn fast_research(&self, notebook: &str, query: &str) -> Result<ResearchResults>;

    /// Deep research (multi-step autonomous).
    pub async fn deep_research(&self, notebook: &str, query: &str) -> Result<ResearchResults>;
}
```

### Phase 2: Service Lifecycle Commands

Add to `cli/src/architect_cmd.rs`:

```rust
#[derive(Debug, clap::Subcommand)]
pub enum ArchitectCommand {
    // ... existing commands ...

    /// Manage NotebookLM service daemon.
    Service(ServiceArgs),
}

#[derive(Debug, clap::Subcommand)]
pub enum ServiceCommand {
    /// Start the NotebookLM service daemon.
    Start {
        #[arg(long, default_value = "3456")]
        port: u16,
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the running service.
    Stop,
    /// Check service status.
    Status,
}
```

### Phase 3: Auto-Upload on Refresh

Modify `run_refresh_native()` to:

1. Check if NLM service is running (non-blocking health check)
2. If running, upload new artifacts as sources:
   - `churn_matrix.md` → text source
   - `complexity_map.json` → text source
   - `repo_skeleton.xml` → text source
   - `call_graph.mmd` → text source (if --graph)
3. Log upload status, continue even if upload fails

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `core/src/architect/nlm_service.rs` | CREATE | HTTP client + service manager |
| `core/src/architect/mod.rs` | MODIFY | Add `pub mod nlm_service;` |
| `cli/src/architect_cmd.rs` | MODIFY | Add Service subcommand, auto-upload |
| `core/Cargo.toml` | MODIFY | Add `reqwest` dependency if missing |

---

## API Reference (from notebooklm-mcp README)

### Health Check
```bash
GET http://127.0.0.1:3456/health
# Returns: { status, queue: { pending, processing }, sessions: { active, max } }
```

### Ask Question
```bash
POST http://127.0.0.1:3456/api/ask
Content-Type: application/json
{ "question": "...", "notebook": "notebook-id" }
# Returns: { success, data: { answer, sessionId } }
```

### Add Source
```bash
POST http://127.0.0.1:3456/api/sources
Content-Type: application/json
{ "source_type": "text|website|file", "content": "...", "notebook": "notebook-id" }
```

### Fast Research
```bash
POST http://127.0.0.1:3456/api/research/fast
Content-Type: application/json
{ "query": "...", "notebook": "notebook-id", "wait": true, "timeout_ms": 120000 }
```

---

## Success Criteria

1. `code architect service start` spawns notebooklm daemon
2. `code architect service status` shows health + queue stats
3. `code architect ask "question"` uses HTTP API (no CLI spawn)
4. `code architect refresh` uploads artifacts when service running
5. All existing functionality preserved (cache-first, legacy mode)

---

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Service not running | Prompt user to run `code architect service start` |
| Service start fails | Show error, suggest `notebooklm service start --foreground` |
| Network timeout | Retry once, then fail with actionable error |
| Upload fails | Log warning, continue with refresh (non-blocking) |

---

## Test Plan

```bash
# 1. Service lifecycle
code architect service start
code architect service status  # Should show "running"
code architect service stop

# 2. Ask with service
code architect service start
code architect ask "What is the architecture?"  # Should use HTTP
code architect ask "Same question"              # Should be cached

# 3. Auto-upload
code architect service start
code architect refresh  # Should show "Uploading artifacts..."
# Verify sources appear in NotebookLM web UI

# 4. Graceful degradation
code architect service stop
code architect ask "Question"  # Should prompt to start service
```

---

## Key References

| Document | Purpose |
|----------|---------|
| `~/notebooklm-mcp/README.md` | Full API reference |
| `docs/ARCHITECT-FEATURE-PARITY.md` | Gap analysis |
| `core/src/architect/mod.rs` | Current module structure |
| P104 session | Mermaid + graph_bridge implementation |
