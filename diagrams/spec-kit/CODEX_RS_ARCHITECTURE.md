# Codex-RS Workspace Architecture Report
**Repository**: https://github.com/theturtlecsz/code (FORK)  
**Upstream**: https://github.com/just-every/code  
**Analysis Date**: 2025-10-30  
**Purpose**: Senior architecture review

---

## Executive Summary

The codex-rs workspace is a Rust-based AI coding assistant with **fork-specific spec-kit automation**. The architecture consists of **24 crates** organized into 6 major layers:
1. **Entry Points** (CLI/TUI binaries)
2. **Protocol Layer** (event-driven SQ/EQ pattern)
3. **Core Business Logic** (conversation management, agent orchestration)
4. **MCP Integration** (Model Context Protocol for tool extensibility)
5. **Execution Layer** (sandboxed command execution)
6. **Fork-Specific Features** (spec-kit multi-agent automation)

**Key Innovation**: Native multi-agent consensus system with tiered model allocation (0-4 agents per operation) achieving 40% cost reduction and 5.3x performance improvement through MCP optimization.

---

## 1. Workspace Structure

### 1.1 All 24 Crates

| Crate | Type | Purpose | Fork-Specific |
|-------|------|---------|---------------|
| **cli** | Binary | Main CLI entry point (`code` command) | No |
| **tui** | Lib+Binary | Interactive TUI (`code-tui`, `spec-status-dump`) | **Modified** |
| **exec** | Lib+Binary | Non-interactive execution mode | No |
| **core** | Library | Business logic, conversation management | No |
| **protocol** | Library | SQ/EQ event definitions | No |
| **mcp-client** | Lib+Binary | MCP client implementation | No |
| **mcp-server** | Lib+Binary | MCP server for external tools | No |
| **mcp-types** | Library | MCP protocol types | No |
| **spec-kit** | Library | **Multi-agent automation framework** | **YES** |
| **common** | Library | Shared utilities (CLI config, elapsed time, sandbox summary) | No |
| **git-tooling** | Library | Git operations (ghost commits, worktrees) | No |
| **file-search** | Lib+Binary | File search functionality | No |
| **apply-patch** | Lib+Binary | Patch application logic | No |
| **linux-sandbox** | Lib+Binary | Linux sandboxing (landlock/seccomp) | No |
| **browser** | Library | Browser automation (CDP integration) | No |
| **chatgpt** | Library | ChatGPT API integration | No |
| **ollama** | Library | Ollama local model support | No |
| **login** | Library | Authentication management | No |
| **protocol-ts** | Lib+Binary | TypeScript protocol bindings generator | No |
| **ansi-escape** | Library | ANSI escape code handling | No |
| **arg0** | Library | Argument parsing utilities | No |
| **execpolicy** | Lib+Binary | Execution policy enforcement | No |
| **codex-version** | Library | Version information | No |
| **utils/readiness** | Library | Readiness checks | No |

**Binaries**: 10 total (cli, tui, exec, mcp-server, mcp-client, file-search, apply-patch, linux-sandbox, protocol-ts, execpolicy)  
**Libraries**: 21 total (some crates have both lib and bin)

---

## 2. Entry Points & Data Flow

### 2.1 CLI Entry Point (`codex-rs/cli/src/main.rs`)

**Primary Command**: `code` (also `coder` to avoid VS Code conflicts)

**Subcommands**:
```rust
- code              → TUI (interactive mode)
- code exec         → Non-interactive execution
- code login        → Authentication management
- code mcp          → MCP server management
- code proto        → Protocol stream via stdin/stdout
- code apply        → Apply latest diff to working tree
- code resume       → Resume previous session
- code doctor       → Diagnose PATH/binary conflicts
- code preview      → Download/run preview builds
- code llm          → Side-channel LLM utilities
```

**Initialization Flow**:
```
1. Parse CLI args (clap)
2. Load config from ~/.code/config.toml (or legacy ~/.codex/)
3. Apply CLI overrides (-c key=value, --model, --sandbox, etc.)
4. Dispatch to subcommand OR launch TUI
5. Initialize MCP connection manager (shared across all features)
6. Start conversation manager (codex-core)
```

### 2.2 TUI Entry Point (`codex-rs/tui/src/main.rs`)

**Binary**: `code-tui` (minimal wrapper)

**Actual Implementation**: `codex-tui::run_main()` in `lib.rs`

**Architecture**:
```
App (event loop coordinator)
├── ChatWidget (main UI component)
│   ├── BottomPane (input composer, modals, popups)
│   ├── HistoryCell[] (message rendering)
│   ├── spec_kit/ (fork-specific automation - 15 modules)
│   └── Terminal overlays (command execution, browser)
├── OnboardingScreen (first-run setup)
├── FileSearchManager (fuzzy file finder)
└── ConversationManager (core business logic)
```

**Event System**:
- **AppEvent** - High-level UI events (user input, terminal runs, spec-kit commands)
- **Protocol Events** - From conversation manager (AgentMessage, ExecCommand, etc.)
- **Rendering** - Debounced at 30 FPS (33ms window) to coalesce updates

---

## 3. Protocol Layer (Event-Driven Architecture)

### 3.1 SQ/EQ Pattern

**Submission Queue (SQ)** - User → Agent:
```rust
pub enum Op {
    UserInput { items: Vec<InputItem> },
    UserTurn { items, cwd, approval_policy, sandbox_policy, model, effort, summary },
    ExecApproval { id, decision },
    PatchApproval { id, decision },
    Interrupt,
    Review { review_request },
    Compact,
    Shutdown,
    // ... 10+ more operations
}
```

**Event Queue (EQ)** - Agent → User:
```rust
pub enum EventMsg {
    // Core events
    TaskStarted, TaskComplete, TokenCount,
    AgentMessage, AgentMessageDelta,
    UserMessage,
    
    // Execution events
    ExecCommandBegin, ExecCommandEnd, ExecCommandOutputDelta,
    ExecApprovalRequest,
    
    // Patch events
    PatchApplyBegin, PatchApplyEnd, ApplyPatchApprovalRequest,
    
    // MCP tool events
    McpToolCallBegin, McpToolCallEnd,
    
    // Reasoning events (for advanced models)
    AgentReasoning, AgentReasoningDelta,
    AgentReasoningRawContent, AgentReasoningRawContentDelta,
    
    // Status events
    Error, StreamError, TurnAborted, ShutdownComplete,
    
    // ... 30+ event types total
}
```

**Key Insight**: Fully asynchronous design - user can interrupt, agent can request approval, events stream incrementally.

### 3.2 Content Types

**InputItem** (user to agent):
- Text
- Image (URL or local path)
- LocalImage (auto-encoded to data URL)

**ContentItem** (agent responses):
- InputText, InputImage (echoed back)
- Text (agent response)
- ToolUse, ToolResult (function calls)

---

## 4. Core Business Logic Layer

### 4.1 Conversation Management (`codex-core`)

**Primary Type**: `ConversationManager`

**Responsibilities**:
- Manage conversation lifecycle (create, resume, archive)
- Coordinate agent turns
- Handle tool calls (Bash, Read, Write, Edit, Git, MCP, Browser, etc.)
- Apply approval policies
- Enforce sandbox policies
- Track token usage
- Persist conversation history

**Key Modules**:
```
codex_core/
├── conversation_manager.rs  → Session orchestration
├── codex.rs                 → Core agent coordination
├── config.rs                → Configuration loading (5-layer precedence)
├── mcp_connection_manager.rs → MCP client pool
├── exec.rs, exec_command.rs → Sandboxed execution
├── auth.rs, auth_accounts.rs → Authentication
├── git_info.rs, git_worktree.rs → Git operations
├── agent_tool.rs            → Tool implementations
└── protocol.rs              → Protocol helpers
```

### 4.2 Configuration System (5-Layer Precedence)

**Precedence** (highest to lowest):
1. **CLI flags** - `--model`, `--sandbox`, `-c key=value`
2. **Shell environment** - `CODE_HOME`, `OPENAI_API_KEY`, `SPEC_OPS_*`
3. **Profile overrides** - `~/.code/config.toml` `[profile.myprofile]`
4. **Base config** - `~/.code/config.toml` (primary) or `~/.codex/config.toml` (legacy)
5. **Built-in defaults** - Hardcoded in code

**Key Settings**:
```toml
model = "gpt-5-codex"
model_provider_id = "openai"
approval_policy = "on-request"  # untrusted, on-failure, on-request, never
sandbox_policy = { mode = "workspace-write" }  # read-only, workspace-write, danger-full-access

[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]
env = { "MEMORY_STORAGE_PATH" = "~/.code/mcp-memory.db" }

[agents]  # Multi-agent configurations
[[agents]]
name = "gemini"
model = "gemini-2.0-pro-exp"
# ... (5 agents: gemini, claude, gpt_pro, gpt_codex, code)
```

---

## 5. MCP Integration Layer

### 5.1 Architecture

**MCP (Model Context Protocol)** - Extensibility via external tools

**Components**:
- **mcp-client** - Spawns and communicates with MCP servers via stdio
- **mcp-server** - Exposes codex functionality as MCP tools
- **mcp-types** - Shared protocol types (from MCP spec)

**Connection Management**:
```rust
pub struct McpConnectionManager {
    clients: HashMap<String, Arc<McpClient>>,  // server_name -> client
    tools: HashMap<String, ToolInfo>,          // qualified_name -> tool
}
```

**Tool Naming**: `{server}__{tool}` (e.g., `local-memory__store_memory`)

**Startup**:
1. Parse `mcp_servers` from config.toml
2. Spawn each server as subprocess (stdio transport)
3. Call `tools/list` on each server (10s timeout)
4. Aggregate tools with qualified names
5. Register with agent for tool use

### 5.2 Used MCP Servers (Fork-Specific)

| Server | Purpose | Fork-Specific |
|--------|---------|---------------|
| **local-memory** | Knowledge base (conversation history, decisions, patterns) | **Critical for spec-kit** |
| **hal** | API endpoint validation (Kavedarr project example) | Optional |
| **git-status** | Repository state monitoring | Optional |

**Note**: **local-memory is the ONLY knowledge persistence system** (byterover deprecated 2025-10-18)

### 5.3 Native MCP Optimization (ARCH-002)

**Problem**: Subprocess calls to MCP were slow (46ms per consensus check)

**Solution**: Direct Rust integration with auto-fallback
```rust
// Try native first, fallback to subprocess
match mcp_manager.native_call("local-memory__search", params).await {
    Ok(result) => result,
    Err(_) => mcp_manager.subprocess_call("local-memory__search", params).await?
}
```

**Performance**: **5.3x faster** (46ms → 8.7ms, validated via benchmarks)

---

## 6. Execution Layer (Sandboxing)

### 6.1 Sandbox Modes

**read-only**:
- Read filesystem anywhere
- Network blocked
- No writes (except TMPDIR)

**workspace-write**:
- Read anywhere
- Write to CWD + specified paths
- Network blocked

**danger-full-access**:
- No restrictions (use with caution)

### 6.2 Platform-Specific Implementation

**Linux** (`codex-linux-sandbox`):
- **Landlock** - Filesystem access control
- **seccomp** - Syscall filtering
- Enforced via separate binary invoked by core

**macOS** (`codex_core::seatbelt`):
- Apple Seatbelt API
- Configured via profile strings

**Windows**:
- Limited sandboxing (work in progress)

### 6.3 Approval Flow

```
1. Agent requests tool execution (local_shell)
2. Core checks approval_policy:
   - untrusted → Ask if not safe command
   - on-request → Agent decides (user_review=true/false)
   - on-failure → Auto-run sandboxed, escalate if fails
   - never → Auto-run, never ask
3. If asking, emit ExecApprovalRequest event
4. TUI shows modal, waits for user decision
5. User submits ExecApproval op
6. Core executes or cancels
```

---

## 7. Fork-Specific Features (Spec-Kit)

### 7.1 Multi-Agent Automation Framework

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/` (15 modules, 98.2% isolation)

**Purpose**: AI-driven feature development through 6-stage consensus workflow

**Commands** (13 total, all `/speckit.*` namespace):
```
Intake:     /speckit.new, /speckit.specify
Quality:    /speckit.clarify, /speckit.analyze, /speckit.checklist
Stages:     /speckit.plan, /speckit.tasks, /speckit.implement, 
            /speckit.validate, /speckit.audit, /speckit.unlock
Automation: /speckit.auto
Diagnostic: /speckit.status (native, 0 agents, <1s)
```

### 7.2 Tiered Model Strategy

**Tier 0 - Native** (0 agents, instant, $0):
- `/speckit.status` - Pure Rust, reads evidence directory

**Tier 2-lite - Dual Agent** (2 agents, 5-8 min, $0.35):
- `/speckit.checklist` - Quality scoring (claude, code)

**Tier 2 - Triple Agent** (3 agents, 8-12 min, $0.80-1.00):
- Most commands (gemini, claude, code/gpt_pro)

**Tier 3 - Quad Agent** (4 agents, 15-20 min, $2.00):
- `/speckit.implement` - Code generation (gemini, claude, gpt_codex, gpt_pro)

**Tier 4 - Dynamic** (3-5 agents, 60 min, $11):
- `/speckit.auto` - Full pipeline, adaptive arbiter

**Cost Reduction**: 40% savings vs uniform 5-agent approach ($15 → $11 per pipeline)

### 7.3 Spec-Kit Module Architecture

```
spec_kit/
├── mod.rs                    → Module coordination, re-exports
├── command_handlers.rs       → Entry points for slash commands
├── routing.rs                → Dispatch /speckit.* to handlers
├── state.rs                  → Pipeline state machine, quality gates
├── consensus_coordinator.rs  → Multi-agent consensus orchestration
├── pipeline_coordinator.rs   → 6-stage workflow coordination
├── validation_lifecycle.rs   → /speckit.validate lifecycle tracking
├── handler.rs                → Core automation logic
├── evidence.rs               → Telemetry and artifact management
├── cost_tracker.rs           → Budget tracking (SPEC-KIT-070)
├── spec_id_generator.rs      → Native SPEC-ID generation (eliminates $2.40 cost)
├── quality*.rs (3 files)     → Quality gate logic, issue resolution
├── ace_*.rs (7 files)        → ACE (Agentic Context Engine) integration
├── config_validator.rs       → Validate spec-kit configuration
└── subagent_defaults.rs      → Default agent configurations
```

**Key Features**:
- **Consensus synthesis** - Automatic comparison, conflict detection, arbiter invocation
- **Evidence tracking** - All outputs captured in `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`
- **Quality gates** - T85 quality issue detection/resolution system
- **Cost tracking** - Budget limits, overage alerts (SPEC-KIT-070)
- **Agent resilience** - 30-min timeout, 3 retries, empty result detection (AR-1 through AR-4)

### 7.4 Template System

**Location**: `~/.code/templates/` (11 templates)

**Performance**: **55% faster generation** vs baseline (validated SPEC-KIT-060)

**Templates**: GitHub-inspired format with P1/P2/P3 user scenarios
- PRD-template.md, spec-template.md
- plan-template.md, tasks-template.md
- implement-template.md, validate-template.md, audit-template.md, unlock-template.md
- clarify-template.md, analyze-template.md, checklist-template.md

### 7.5 Evidence & Telemetry

**Schema v1** (JSON per stage):
```json
{
  "command": "spec-plan",
  "specId": "SPEC-KIT-065",
  "sessionId": "uuid",
  "timestamp": "2025-10-30T12:34:56Z",
  "schemaVersion": 1,
  "artifacts": [
    {"path": "docs/SPEC-KIT-065-feature/plan.md", "status": "created"}
  ],
  "baseline": {
    "mode": "spec-only",
    "artifact": "docs/SPEC-KIT-065-feature/spec.md",
    "status": "exists"
  },
  "hooks": {
    "session": {"start": "2025-10-30T12:34:00Z"}
  }
}
```

**Storage**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/<SPEC-ID>/`

**Consensus**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/<SPEC-ID>/<stage>/`

---

## 8. Component Boundaries & Integration Points

### 8.1 Layer Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│ Entry Layer (CLI/TUI)                                       │
│ - Parse args, load config, dispatch to mode                │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│ Protocol Layer (codex-protocol)                             │
│ - Op definitions (user → agent)                             │
│ - EventMsg definitions (agent → user)                       │
│ - Content types (InputItem, ContentItem)                    │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│ Core Logic Layer (codex-core)                               │
│ - ConversationManager (session lifecycle)                   │
│ - Codex (agent coordination, tool calls)                    │
│ - Config (5-layer precedence)                               │
│ - Auth (login, accounts, token management)                  │
└────┬────────────────┬────────────────┬────────────────┬─────┘
     │                │                │                │
┌────▼────┐     ┌─────▼─────┐    ┌────▼────┐    ┌─────▼──────┐
│ MCP     │     │ Execution │    │ Git     │    │ Spec-Kit   │
│ Layer   │     │ Layer     │    │ Tooling │    │ (FORK)     │
│         │     │           │    │         │    │            │
│ Client  │     │ Sandbox   │    │ Commits │    │ Multi-     │
│ Server  │     │ Landlock  │    │ Worktree│    │ Agent      │
│ Types   │     │ Seatbelt  │    │         │    │ Consensus  │
└─────────┘     └───────────┘    └─────────┘    └────────────┘
```

### 8.2 Key Integration Points

**TUI → Core**:
- `App` creates `ConversationManager`
- `ChatWidget` submits `Op` via channel
- `ChatWidget` receives `EventMsg` via stream
- `App` coordinates `FileSearchManager`, `McpConnectionManager`

**Core → MCP**:
- `mcp_connection_manager::McpConnectionManager` pools clients
- Tool calls routed by qualified name (`server__tool`)
- Results returned as `CallToolResult`

**Core → Execution**:
- `exec::exec_command()` spawns sandboxed processes
- Platform-specific: Linux (landlock+seccomp), macOS (seatbelt), Windows (limited)
- Output streamed via `ExecCommandOutputDelta` events

**TUI → Spec-Kit**:
- `routing::try_dispatch_spec_kit_command()` recognizes `/speckit.*`
- `command_handlers` orchestrate multi-agent consensus
- `pipeline_coordinator` manages 6-stage workflow
- Results appear as standard `AgentMessage` events in history

**Spec-Kit → MCP (local-memory)**:
- Store consensus synthesis, agent outputs, decisions
- Search prior patterns, solutions, architecture state
- **Critical**: local-memory is ONLY knowledge persistence (MEMORY-POLICY.md)

---

## 9. Data Model & Key Types

### 9.1 Core Types

**Config** (comprehensive):
```rust
pub struct Config {
    model: String,                          // e.g., "gpt-5-codex"
    model_provider: ModelProviderInfo,      // API endpoint, auth
    approval_policy: AskForApproval,        // untrusted, on-failure, on-request, never
    sandbox_policy: SandboxPolicy,          // read-only, workspace-write, danger-full-access
    mcp_servers: HashMap<String, McpServerConfig>,
    agents: Vec<AgentConfig>,               // Multi-agent definitions (fork-specific)
    ace: AceConfig,                         // ACE learning config (fork-specific)
    cwd: PathBuf,
    codex_home: PathBuf,                    // ~/.code/ (primary) or ~/.codex/ (legacy)
    // ... 30+ fields total
}
```

**AgentConfig** (fork-specific):
```rust
pub struct AgentConfig {
    name: String,              // "gemini", "claude", "gpt_pro", "gpt_codex", "code"
    model: String,             // "gemini-2.0-pro-exp", "claude-4.5-sonnet", etc.
    provider: String,          // "google", "anthropic", "openai", etc.
    reasoning_mode: Option<String>,
    write_mode: bool,
    // ...
}
```

**SlashCommand** (TUI enum):
```rust
pub enum SlashCommand {
    // Fork-specific spec-kit (13 variants)
    SpecKitNew { description: String },
    SpecKitStatus { spec_id: String },
    SpecKitPlan { spec_id: String },
    // ... 10+ more SpecKit* variants
    
    // Upstream commands
    Model, New, Resume, Chrome, Browser, Themes,
    // ... 20+ more variants
}
```

### 9.2 Spec-Kit Types

**SpecAutoState** (pipeline state machine):
```rust
pub struct SpecAutoState {
    pub spec_id: String,
    pub stage: SpecStage,              // Plan, Tasks, Implement, Validate, Audit, Unlock
    pub task_count: usize,
    pub tasks_completed: usize,
    pub stage_begin: Option<Instant>,
    pub consensus_data: HashMap<String, ConsensusSynthesisSummary>,
    pub quality_gates: Vec<QualityCheckpoint>,
    // ...
}
```

**SpecStage**:
```rust
pub enum SpecStage {
    Plan, Tasks, Implement, Validate, Audit, Unlock
}
```

**SpecAgent** (type-safe agent enum):
```rust
pub enum SpecAgent {
    Gemini, Claude, GptPro, GptCodex, Code
}

impl SpecAgent {
    pub fn canonical_name(&self) -> &'static str { /* ... */ }
    pub fn from_string(s: &str) -> Option<Self> { /* ... */ }
}
```

**QualityIssue** (T85 quality gates):
```rust
pub struct QualityIssue {
    pub id: String,
    pub issue_type: QualityGateType,   // Ambiguity, Inconsistency, MissingRequirement, etc.
    pub severity: Magnitude,            // Critical, Major, Minor, Info
    pub resolvability: Resolvability,   // AutoFix, UserInput, Escalate
    pub description: String,
    pub resolution: Option<Resolution>,
    // ...
}
```

---

## 10. Cross-Cutting Concerns

### 10.1 Configuration

**5-Layer Precedence** (see section 4.2):
1. CLI flags (highest)
2. Shell environment (`CODE_HOME`, `OPENAI_API_KEY`)
3. Profile overrides (`~/.code/config.toml` `[profile.name]`)
4. Base config (`~/.code/config.toml`)
5. Built-in defaults (lowest)

**Backwards Compatibility**:
- Reads both `~/.code/*` (primary) and `~/.codex/*` (legacy)
- Writes only to `~/.code/*`

### 10.2 Error Handling

**Protocol Errors**:
- `EventMsg::Error { error_message }` - Surfaces to user in TUI
- `EventMsg::StreamError` - Connection issues, retries with backoff

**Agent Errors** (fork-specific resilience):
- **AR-1**: 30-minute total timeout on all operations
- **AR-2**: Auto-retry on failures (3 attempts with context injection)
- **AR-3**: Empty/invalid result detection with retry guidance
- **AR-4**: JSON schema enforcement (reduces malformed output ~80%)

**Tool Errors**:
- Exec failures → Return to agent with error context
- MCP tool failures → Fallback to subprocess, then error
- File operation failures → Surface via error events

### 10.3 Logging & Observability

**Tracing**:
- `tracing` crate throughout (`info!`, `warn!`, `error!`)
- Configured via `RUST_LOG` environment variable
- TUI logs: `~/.code/log/codex-tui.log` (or legacy `~/.codex/log/`)

**Spec-Kit Telemetry** (fork-specific):
- JSON schema v1 per stage
- Stored in evidence directory
- Queryable via `/spec-evidence-stats`

**Debug Mode**:
- `--debug` flag captures API requests/responses
- Written to `~/.code/debug_logs/` (or legacy `~/.codex/debug_logs/`)

### 10.4 Security

**Sandboxing**:
- Linux: Landlock + seccomp
- macOS: Seatbelt
- Windows: Limited (work in progress)

**Approval Policies**:
- `untrusted` - Ask for unsafe commands
- `on-request` - Agent decides when to ask
- `on-failure` - Try sandboxed first, escalate if fails
- `never` - Auto-run, never ask (dangerous)

**Secret Management**:
- API keys via environment variables (never committed)
- `scripts/env_run.sh` ensures `.env` respected
- MCP servers configured with env vars

---

## 11. External Dependencies

### 11.1 AI Providers (via Core)

| Provider | Models | Purpose |
|----------|--------|---------|
| **OpenAI** | gpt-5-codex, gpt-5, gpt-4o-mini | General, validation, arbitration |
| **Anthropic** | claude-4.5-sonnet | Synthesis, precision |
| **Google** | gemini-2.0-pro-exp, gemini-flash | Research, breadth, cost-efficient |

**API Communication**:
- `codex_core::client::ModelClient` - Unified interface
- `reqwest` - HTTP client
- SSE streaming for incremental responses

### 11.2 MCP Servers (External Processes)

| Server | Command | Transport |
|--------|---------|-----------|
| **local-memory** | `npx -y @modelcontextprotocol/server-memory` | stdio |
| **hal** | Custom HTTP wrapper | stdio |
| **git-status** | Custom (if configured) | stdio |

**Note**: All MCP servers spawn as subprocesses, communicate via JSON-RPC over stdio.

### 11.3 Platform Dependencies

**Linux**:
- `landlock` crate - Filesystem access control
- `seccompiler` crate - Syscall filtering

**macOS**:
- Apple Seatbelt API (via FFI)

**All Platforms**:
- `ratatui` - TUI rendering
- `crossterm` - Terminal abstraction
- `tokio` - Async runtime
- `serde`/`serde_json` - Serialization
- `clap` - CLI parsing

---

## 12. Performance & Scalability

### 12.1 Performance Optimizations

**MCP Native Integration** (ARCH-002):
- 5.3x faster consensus checks (46ms → 8.7ms)
- Auto-fallback to subprocess on failure

**Rendering Optimizations**:
- 30 FPS debouncing (33ms window) to coalesce updates
- Incremental rendering (only changed cells)
- Buffer diff profiling to identify hotspots

**Spec-Kit Cost Reduction** (40% savings):
- Tiered model strategy (0-4 agents vs uniform 5)
- Native SPEC-ID generation (eliminates $2.40 per `/speckit.new`)
- Template-based generation (55% faster)

### 12.2 Scalability Limits

**Conversation History**:
- Auto-compaction at token threshold (configurable)
- Archived sessions stored separately

**MCP Connection Pool**:
- One client per server (configured in `mcp_servers`)
- Tool calls serialized per client (future: parallel)

**Evidence Storage** (fork-specific):
- 25 MB soft limit per SPEC
- Offload old evidence to archive as needed

---

## 13. Deployment & Build

### 13.1 Build System

**Cargo Workspace**:
- 24 member crates
- Shared dependencies via `[workspace.dependencies]`
- Release profile: LTO, stripped symbols, single codegen unit

**Edition**: Rust 2024 (all crates)

**Profiles**:
```toml
[profile.release]
lto = "fat"
strip = "symbols"
codegen-units = 1

[profile.dev-fast]  # For fast iteration
opt-level = 1
debug = 1
codegen-units = 256
lto = "off"
```

### 13.2 Distribution

**Platforms**:
- Linux: x86_64-unknown-linux-musl, aarch64-unknown-linux-musl
- macOS: x86_64-apple-darwin, aarch64-apple-darwin
- Windows: x86_64-pc-windows-msvc

**Packaging**:
- npm: `@just-every/code` (upstream), `@theturtlecsz/code` (fork TBD)
- Homebrew (upstream only)
- Direct binary download

### 13.3 Installation

```bash
# NPM (global)
npm install -g @just-every/code

# Run without install
npx -y @just-every/code

# Manual binary
curl -L https://github.com/just-every/code/releases/latest/download/code-{platform}.tar.gz | tar xz
```

---

## 14. Testing Strategy

### 14.1 Test Coverage

**Upstream Crates**:
- Unit tests throughout (`#[cfg(test)]`)
- Integration tests in `tests/` directories
- Snapshot tests via `insta` crate

**Fork-Specific (spec-kit)**:
- **100% test coverage maintained** (MAINT-10 requirement)
- Integration tests in `codex-rs/tui/tests/spec_kit/`
- Mock framework: `MockSpecKitContext` (feature gated)
- Phase 3 test expansion (workflow, error recovery, state persistence, quality gates, concurrent ops)

**Test Utilities**:
- `core_test_support` - Shared test helpers for core
- `mcp_test_support` - Shared test helpers for MCP
- Feature flag: `test-utils` - Expose test APIs

### 14.2 CI/CD

**Pre-commit Hooks** (`.githooks/pre-commit`):
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --no-run` (compilation check)
- `scripts/doc-structure-validate.sh --mode=templates`

**Pre-push Hooks** (`.githooks/pre-push`):
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo build --workspace --all-features`

**Environment**:
- `scripts/env_run.sh` ensures `.env` secrets respected
- `SPEC_OPS_CARGO_MANIFEST` for workspace awareness

---

## 15. Future Directions

### 15.1 Planned Features (from PLANNING.md)

**ARCH Tasks** (7/13 complete):
- ARCH-008: ACE playbook SQLite integration (Phase 4)
- ARCH-010: Agent config validation (Phase 4)
- ARCH-011: Quality gate persistence (Phase 4)
- ARCH-012: Cost tracking integration (Phase 4)
- ARCH-013: Evidence compression strategy (Phase 5)

**Spec-Kit Enhancements**:
- SPEC-KIT-072: Separate consensus database (not local-memory)
- SPEC-KIT-073: Advanced conflict resolution strategies
- SPEC-KIT-074: Parallel agent execution (30% faster)
- SPEC-KIT-075: Evidence archival automation

### 15.2 Known Limitations

**Windows Sandboxing**:
- Limited compared to Linux/macOS
- Work in progress for full parity

**MCP Server Discovery**:
- Manual configuration in `config.toml`
- No auto-discovery yet

**Spec-Kit Cost Tracking**:
- Budget limits implemented, but no hard stop
- Alerts only (SPEC-KIT-070)

---

## 16. Appendix: File Counts & LOC Estimates

**Rust Source Files**: 534 total

**Major Crate Sizes** (estimated):
- `tui`: ~120-150 files (largest - includes spec-kit)
- `core`: ~80-100 files
- `protocol`: ~10-15 files
- `mcp-client`: ~5-10 files
- `spec-kit`: ~15-20 files (fork-specific)

**Lines of Code** (rough estimate):
- Total Rust: ~150,000-200,000 LOC
- Spec-kit isolation: ~5,000-8,000 LOC (98.2% isolated from upstream)

---

## 17. Conclusion

The codex-rs workspace is a **well-structured, modular architecture** with clear layer boundaries:

**Strengths**:
1. **Event-driven protocol** - Fully asynchronous, supports interrupts, streaming
2. **Extensible via MCP** - External tools integrate seamlessly
3. **Platform-specific sandboxing** - Strong security model
4. **Fork isolation** - Spec-kit changes isolated to minimize rebase conflicts (98.2%)
5. **Performance optimizations** - Native MCP, tiered models, template-based generation
6. **Comprehensive testing** - 100% test coverage in fork-specific code

**Architecture Quality**:
- **Single Responsibility**: Each crate has focused purpose
- **Dependency Inversion**: Abstractions (protocol, config) separate from implementations
- **Open/Closed**: MCP servers extend functionality without core changes
- **Clean boundaries**: Protocol layer separates UI from business logic

**Recommendations for Architecture Review**:
1. **Continue spec-kit isolation** - Maintain 98%+ separation for easy rebasing
2. **Formalize MCP native interface** - Document contract for future optimizations
3. **Consider GraphViz diagrams** - Visualize data flow for onboarding
4. **Monitor evidence storage** - Implement automated archival (ARCH-013)
5. **Expand test coverage** - Add integration tests for multi-agent edge cases

**Overall Assessment**: **Production-ready architecture** with strong separation of concerns, clear extension points, and thoughtful fork-specific additions that minimize upstream conflicts.

---

**End of Architecture Report**
