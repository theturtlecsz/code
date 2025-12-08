# P105 Continuation Prompt - NotebookLM HTTP Service Integration

## Context

### Completed Work
| Phase | Commit | Deliverables |
|-------|--------|--------------|
| P103 | 4042be45d | churn.rs, complexity.rs, skeleton.rs (native Rust harvester) |
| P104 | ee8bf5b48 | mermaid.rs (call graph), graph_bridge.rs (Python-only docs) |

### Key Discovery (P104)
CodeGraphContext MCP only parses Python, not Rust. We use native tree-sitter for Rust analysis.

### Architecture Shift
notebooklm-mcp v2+ has HTTP Service mode (port 3456). Eliminates cold-start latency.

---

## Scope Decisions (User Confirmed)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Module Location | `core/src/architect/` | Quota gatekeeper, cohesive with harvester |
| Budget Tracking | **Hourly breakdown** | Maximum visibility for usage patterns |
| Research API | **Defer to P106** | Focus on core ask/sources/budget first |
| Source Strategy | **Full Replace (Atomic Swap)** | Prevents context rot, fixed slot usage |

---

## Pre-flight Checklist

```bash
# 1. Build and verify P104 work
./build-fast.sh
./codex-rs/target/dev-fast/code architect refresh --graph

# 2. Check notebooklm-mcp version (need v2+ for service mode)
notebooklm --version
# Expected: 1.3.0 or higher

# 3. Verify service mode works
notebooklm service start
curl http://127.0.0.1:3456/health
notebooklm service stop

# 4. Check existing architect structure
ls -la codex-rs/core/src/architect/
# Should see: churn.rs, complexity.rs, skeleton.rs, mermaid.rs, graph_bridge.rs, mod.rs
```

---

## Implementation Tasks

### Phase 1: HTTP Client (`nlm_service.rs`)

Create `core/src/architect/nlm_service.rs`:

```rust
//! NotebookLM HTTP Service client.
//!
//! Connects to notebooklm-mcp service (default: http://127.0.0.1:3456)
//! with lazy service spawning and budget tracking.
//!
//! # Architecture
//! This module is the SOLE GATEKEEPER for NotebookLM quota.
//! Other parts of codex-rs should not access NotebookLM directly.

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

const DEFAULT_PORT: u16 = 3456;
const DEFAULT_HOST: &str = "127.0.0.1";
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

pub struct NlmService {
    base_url: String,
    client: Client,
    budget: BudgetTracker,
}

impl NlmService {
    /// Ensure service is running, spawn if needed.
    pub async fn ensure_running(vault_path: &Path) -> Result<Self>;

    /// Health check with queue/session stats.
    pub async fn health(&self) -> Result<HealthStatus>;

    /// Ask a question (budget-tracked, cached).
    pub async fn ask(&mut self, notebook: &str, question: &str) -> Result<AskResponse>;

    /// List sources in a notebook.
    pub async fn list_sources(&self, notebook: &str) -> Result<Vec<Source>>;

    /// Delete a source by index.
    pub async fn delete_source(&self, notebook: &str, index: usize) -> Result<()>;

    /// Add sources (text content).
    pub async fn add_text_source(&self, notebook: &str, title: &str, content: &str) -> Result<()>;

    /// Atomic swap: delete [ARCH] sources, upload fresh artifacts.
    pub async fn refresh_context(&mut self, notebook: &str, artifacts: &[Artifact]) -> Result<()>;
}
```

### Phase 2: Budget Tracking (`budget.rs`)

Create `core/src/architect/budget.rs`:

```rust
//! Budget tracking with hourly granularity.
//!
//! Stores usage in .codex/architect/usage.json

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageData {
    /// Current date (YYYY-MM-DD)
    pub date: String,
    /// Queries per hour (0-23 keys)
    pub hourly: HashMap<u8, u32>,
    /// Total queries today
    pub total: u32,
    /// 7-day history for trends
    pub history: Vec<DailyUsage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String,
    pub total: u32,
}

pub struct BudgetTracker {
    usage: UsageData,
    file_path: std::path::PathBuf,
    daily_limit: u32,  // 500 for Plus tier
    warn_threshold: u32,  // 400 (80%)
}

impl BudgetTracker {
    pub fn load(vault_path: &Path) -> Result<Self>;
    pub fn record_query(&mut self) -> Result<()>;
    pub fn remaining(&self) -> u32;
    pub fn needs_confirmation(&self) -> bool;  // true if > 400 used
    pub fn format_status(&self) -> String;     // "Used: 23/500 (4.6%)"
    pub fn hourly_breakdown(&self) -> String;  // For status display
}
```

### Phase 3: Source Management (Atomic Swap)

The "[ARCH]" prefix protocol:

```rust
/// Artifact with [ARCH] prefix for namespace isolation.
pub struct Artifact {
    /// Title with [ARCH] prefix (e.g., "[ARCH] Churn Metrics")
    pub title: String,
    /// Content to upload
    pub content: String,
}

impl Artifact {
    pub fn new(name: &str, content: String) -> Self {
        Self {
            title: format!("[ARCH] {}", name),
            content,
        }
    }
}

/// Atomic swap implementation
pub async fn refresh_context(&mut self, notebook: &str, artifacts: &[Artifact]) -> Result<()> {
    // 1. List all sources
    let sources = self.list_sources(notebook).await?;

    // 2. Identify managed sources (stale artifacts)
    let stale_indices: Vec<usize> = sources
        .iter()
        .filter(|s| s.title.starts_with("[ARCH]"))
        .map(|s| s.index)
        .collect();

    // 3. Delete highest index first (avoids shifting)
    for index in stale_indices.iter().rev() {
        self.delete_source(notebook, *index).await?;
    }

    // 4. Upload fresh artifacts
    for artifact in artifacts {
        self.add_text_source(notebook, &artifact.title, &artifact.content).await?;
    }

    Ok(())
}
```

### Phase 4: CLI Commands

Update `cli/src/architect_cmd.rs`:

```rust
#[derive(Debug, clap::Subcommand)]
pub enum ArchitectCommand {
    /// Update local forensic maps (churn, complexity, skeleton).
    Refresh(RefreshArgs),

    /// Ask a question (cached, budget-tracked).
    Ask(AskArgs),

    /// Audit a Rust crate.
    Audit(AuditArgs),

    /// Show vault status and budget.
    Status,

    /// Manage NotebookLM service daemon.
    Service(ServiceArgs),

    /// Manage notebook sources.
    Sources(SourcesArgs),

    /// Clear cached answers.
    ClearCache,
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
    /// Check service status and health.
    Status,
}

#[derive(Debug, clap::Subcommand)]
pub enum SourcesCommand {
    /// List sources in notebook.
    List,
    /// Upload artifacts (atomic swap with [ARCH] prefix).
    Upload {
        #[arg(long)]
        force: bool,  // Skip confirmation
    },
}
```

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `core/src/architect/nlm_service.rs` | CREATE | HTTP client + service manager |
| `core/src/architect/budget.rs` | CREATE | Hourly usage tracking |
| `core/src/architect/mod.rs` | MODIFY | Add `pub mod nlm_service; pub mod budget;` |
| `cli/src/architect_cmd.rs` | MODIFY | Add Service, Sources subcommands |
| `core/Cargo.toml` | MODIFY | Add `reqwest = { version = "0.11", features = ["json"] }` |

---

## API Reference (notebooklm-mcp)

### Health Check
```bash
GET http://127.0.0.1:3456/health
# Returns: { status, version, uptime, queue: { pending, processing }, sessions: { active, max } }
```

### Ask Question
```bash
POST http://127.0.0.1:3456/api/ask
Content-Type: application/json
{ "question": "...", "notebook": "notebook-id-or-name" }
# Returns: { success, data: { answer, sessionId } }
```

### List Sources
```bash
GET http://127.0.0.1:3456/api/sources?notebook=notebook-id
# Returns: { success, data: { sources: [{ index, title, status }], sourceCount } }
```

### Add Source (Text)
```bash
POST http://127.0.0.1:3456/api/sources
Content-Type: application/json
{ "source_type": "text", "content": "...", "notebook": "notebook-id" }
```

### Delete Source
```bash
# Note: Check API for exact endpoint - may need index in body or URL
DELETE http://127.0.0.1:3456/api/sources/:index?notebook=notebook-id
```

---

## Success Criteria

1. **Service Lifecycle**
   - `code architect service start` spawns daemon (or confirms running)
   - `code architect service status` shows health + queue stats
   - `code architect service stop` cleanly shuts down

2. **Budget Tracking**
   - `code architect status` shows hourly breakdown
   - Warning at 80% (400 queries)
   - Blocks at 100% with clear message

3. **Ask Flow**
   - Cache hit → instant, no query used
   - Cache miss → HTTP request, budget decremented
   - Confirmation prompt if >400 used

4. **Source Management**
   - `code architect sources list` shows current sources
   - `code architect sources upload` performs atomic swap
   - Only [ARCH] prefixed sources are touched

---

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Service not running | Prompt: "Start service with `code architect service start`" |
| Service start fails | Show error, suggest `--foreground` for debugging |
| Budget exceeded | Block with: "Daily limit (500) reached. Resets at midnight UTC." |
| Network timeout | Retry once (2s), then fail with actionable error |
| Source delete fails | Log warning, continue with uploads |

---

## Test Plan

```bash
# 1. Service lifecycle
code architect service start
code architect service status  # Health: ok, Queries: 0/500
code architect service stop

# 2. Budget tracking
code architect service start
code architect ask "What is the main architecture?"
code architect status  # Should show 1 query used, hourly breakdown

# 3. Atomic swap
code architect sources list              # Show current sources
code architect refresh                   # Generate artifacts
code architect sources upload            # Atomic swap
code architect sources list              # Verify [ARCH] sources updated

# 4. Cache behavior
code architect ask "Same question"       # Should be cached (0 queries)
code architect ask "Same question" -f    # Force fresh (1 query)
```

---

## Deferred to P106

- `/api/research/fast` - Quick parallel web search
- `/api/research/deep` - Multi-step autonomous research
- Research result import to notebook
- Research caching and history

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         codex-rs CLI                            │
│  code architect ask / refresh / sources / service / status      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   core/src/architect/                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  churn.rs    │  │ skeleton.rs  │  │    nlm_service.rs    │  │
│  │  (P103) ✓    │  │  (P103) ✓    │  │    (P105) ← NEW      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ complexity.rs│  │  mermaid.rs  │  │     budget.rs        │  │
│  │  (P103) ✓    │  │  (P104) ✓    │  │    (P105) ← NEW      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼ HTTP (localhost:3456)
┌─────────────────────────────────────────────────────────────────┐
│                    notebooklm-mcp service                       │
│                    (Browser automation)                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Google NotebookLM (Plus)                      │
│                   500 queries/day, 300 sources                  │
└─────────────────────────────────────────────────────────────────┘
```
