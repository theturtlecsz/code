# System Overview & Architecture

Comprehensive overview of the theturtlecsz/code architecture.

---

## Table of Contents

1. [High-Level Overview](#high-level-overview)
2. [Design Philosophy](#design-philosophy)
3. [Component Architecture](#component-architecture)
4. [Data Flow](#data-flow)
5. [Technology Stack](#technology-stack)
6. [Fork-Specific Additions](#fork-specific-additions)
7. [Integration Points](#integration-points)

---

## High-Level Overview

**theturtlecsz/code** is a terminal-based AI coding assistant built on a multi-layered architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                    TUI Layer (Ratatui)                      │
│  ┌─────────────────┐  ┌──────────────────────────────────┐ │
│  │   ChatWidget    │  │  Spec-Kit Framework (Fork)       │ │
│  │  (912K LOC)     │  │  - Multi-agent automation        │ │
│  │  - Conversation │  │  - Quality gates                 │ │
│  │  - Agent panels │  │  - Consensus coordination        │ │
│  └────────┬────────┘  └──────────────┬───────────────────┘ │
└───────────┼───────────────────────────┼─────────────────────┘
            │                           │
            ▼                           ▼
┌─────────────────────────────────────────────────────────────┐
│             Core Services Layer (codex-core)                │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Conversation │  │ MCP Manager  │  │  Config System  │  │
│  │   Manager    │  │ (5.3× faster)│  │  (Hot-reload)   │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬────────┘  │
└─────────┼──────────────────┼───────────────────┼───────────┘
          │                  │                   │
          ▼                  ▼                   ▼
┌─────────────────────────────────────────────────────────────┐
│              Infrastructure Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ SQLite Pool  │  │ MCP Clients  │  │ Model Providers │  │
│  │ (6.6× faster)│  │ (stdio IPC)  │  │ (OpenAI, etc.)  │  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Key Characteristics**:
- **226,607 lines of Rust** across 538 source files
- **24-crate Cargo workspace** with modular design
- **Async/sync hybrid** (Tokio async core, Ratatui sync UI)
- **Native MCP integration** (5.3× faster than subprocess)
- **98.8% upstream isolation** (fork features in isolated modules)

---

## Design Philosophy

### 1. Separation of Concerns

**Layer Boundaries**:
- **UI Layer**: Ratatui TUI, user interaction, rendering
- **Application Layer**: Business logic, workflow orchestration
- **Service Layer**: MCP, database, configuration, agents
- **Infrastructure Layer**: SQLite, stdio, HTTP clients

### 2. Async/Sync Hybrid Architecture

**Rationale**: Ratatui requires synchronous event loop, but backend services benefit from async I/O.

**Solution**: Clear async/sync boundary using channels:

```rust
// Sync UI Thread                 Async Backend Task
ChatWidget                        ConversationManager
    ↓                                    ↓
codex_op_tx.send(op)  ──────→   conversation.submit_op().await
    ↑                                    ↓
app_event_tx.recv()   ←──────   Event::CodexEvent(event)
```

**Pattern**: `UnboundedSender<Op>` bridges sync UI to async backend.

**File**: `codex-rs/tui/src/chatwidget/agent.rs:16-62`

### 3. Modularity via Cargo Workspace

**Benefits**:
- Clear dependency boundaries
- Independent compilation units
- Parallel builds (faster CI)
- Reusable components (spec-kit as separate crate)

**Workspace Size**: 24 crates, ~220K LOC Rust

### 4. Fork Isolation Strategy

**Goal**: Minimize rebase conflicts with upstream (`just-every/code`)

**Implementation**:
- Fork features in isolated modules (`tui/src/chatwidget/spec_kit/`)
- "Friend module" pattern (access ChatWidget private fields without public API)
- Dynamic command registry (avoids enum growth in SlashCommand)
- Trait-based abstractions (SpecKitContext decouples from ChatWidget)

**Result**: 98.8% isolation (55 modules under `spec_kit/`, minimal upstream changes)

**Evidence**: `docs/spec-kit/REFACTORING_COMPLETE_SUMMARY.md`

### 5. Performance-First Design

**Database**:
- SQLite with WAL mode: **6.6× read speedup** (850µs → 129µs)
- R2D2 connection pooling: Eliminates connection overhead
- Incremental auto-vacuum: Prevents unbounded growth

**MCP Integration**:
- Native client: **5.3× faster** than subprocess approach
- Shared connection manager: Prevents process multiplication
- 1MB buffer for large tool responses

**Configuration**:
- Hot-reload with 300ms debounce
- Arc<RwLock> for atomic updates (<1ms lock time)

### 6. Fault Tolerance

**Retry Logic** (SPEC-945C):
- Exponential backoff: 100ms → 200ms → 400ms → 800ms
- Jitter to prevent thundering herd
- Permanent vs transient error classification

**Graceful Degradation**:
- Multi-agent consensus works with 2/3 agents (if 1 fails)
- MCP server failures don't crash app
- Config reload failures preserve old config

---

## Component Architecture

### Core Components

#### 1. TUI Layer (`codex-tui` crate)

**Purpose**: Terminal user interface using Ratatui framework

**Key Components**:
- **App**: Top-level application state, event loop
- **ChatWidget**: Main conversation interface (912K LOC)
- **BottomPane**: Input composer, status bar
- **HistoryCell**: Message rendering (user, assistant, tool, exec)
- **StreamController**: Real-time token streaming

**Event Flow**:
```
Terminal Events → App → ChatWidget → Handle Event → Render
                                  ↓
                          Submit Op to Backend
                                  ↓
                          Receive Events from Backend
```

**File**: `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/chatwidget/mod.rs`

---

#### 2. Spec-Kit Framework (`spec-kit` crate + TUI integration)

**Purpose**: Multi-agent automation pipeline with quality gates

**Architecture**:
```
User Command (/speckit.auto)
    ↓
Command Registry (dynamic dispatch)
    ↓
Pipeline Coordinator (state machine)
    ↓
Agent Orchestrator (spawn agents)
    ↓
Consensus Coordinator (aggregate results)
    ↓
Quality Gate Broker (checkpoints)
    ↓
Evidence Repository (artifacts)
```

**Key Modules**:
- **spec-kit** (library crate): Config, retry, types
- **tui/src/chatwidget/spec_kit** (55 modules): TUI integration
  - `command_registry.rs`: Dynamic command dispatch
  - `pipeline_coordinator.rs`: Workflow state machine
  - `agent_orchestrator.rs`: Agent lifecycle
  - `consensus_coordinator.rs`: Multi-agent consensus
  - `native_*.rs`: Zero-cost operations (FREE)
  - `consensus_db.rs`: SQLite artifact storage

**File**: `codex-rs/spec-kit/src/lib.rs`, `codex-rs/tui/src/chatwidget/spec_kit/mod.rs`

---

#### 3. Core Services (`codex-core` crate)

**Purpose**: Backend services for conversation, MCP, database, config

**Key Modules**:
- **ConversationManager**: Agent conversation lifecycle
- **McpConnectionManager**: MCP server aggregation
- **Config**: Configuration loading, validation
- **Database**: SQLite connection pooling, transactions
- **Protocol**: OpenAI API client, model providers

**Responsibilities**:
- Agent spawning and orchestration
- MCP tool invocation
- Model provider communication (OpenAI, Anthropic, Google)
- Configuration hot-reload
- Consensus artifact storage

**File**: `codex-rs/core/src/lib.rs`

---

#### 4. MCP Integration (`mcp-client`, `mcp-types` crates)

**Purpose**: Model Context Protocol client and server support

**Components**:
- **McpClient**: Async client for stdio communication
- **McpConnectionManager**: Central hub for all MCP servers
- **mcp-types**: JSON-RPC protocol types

**Architecture**:
```
MCP Server (subprocess)
    ↓ stdin/stdout
McpClient
├── Writer Task: outgoing_rx → stdin (JSON-RPC requests)
├── Reader Task: stdout → pending HashMap (JSON-RPC responses)
└── Dispatcher: Request ID → oneshot::Sender (pair requests/responses)
```

**Performance**:
- **Native integration**: 5.3× faster than subprocess (8.7ms typical)
- **Concurrent I/O**: Separate reader/writer tasks prevent deadlock
- **1MB buffer**: Handles large tool responses

**File**: `codex-rs/mcp-client/src/mcp_client.rs:63-150`

---

#### 5. Database Layer (`consensus_db` in spec-kit, `db` module in core)

**Purpose**: SQLite storage for consensus artifacts, config, telemetry

**Architecture**:
```
Application
    ↓
R2D2 Connection Pool (2-8 connections)
    ↓
SQLite Connection (WAL mode)
    ↓
Database File (consensus_artifacts.db)
```

**Optimizations**:
- **WAL mode**: 6.6× read speedup (allows concurrent reads)
- **Connection pooling**: Eliminates connection overhead
- **Optimized pragmas**: 32MB cache, memory-mapped I/O (1GB)
- **Incremental auto-vacuum**: 99.95% size reduction after cleanup

**Schema**:
- `consensus_runs`: Workflow execution tracking
- `agent_outputs`: Individual agent responses
- `consensus_artifacts`: Synthesized consensus results

**File**: `codex-rs/core/src/db/connection.rs:39-105`, `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs`

---

#### 6. Configuration System (`config` module in spec-kit)

**Purpose**: Layered configuration with hot-reload

**5-Tier Precedence** (highest to lowest):
1. **CLI flags**: `--model gpt-5`, `--config key=value`
2. **Shell environment**: `export OPENAI_API_KEY=...`
3. **Profile**: `[profiles.premium]` in config.toml
4. **Config file**: `~/.code/config.toml`
5. **Defaults**: Built-in fallback values

**Hot-Reload**:
```
File Change → Debouncer (300ms) → Validate → Lock → Replace → Event
                                       ↓ Fail
                                Preserve Old Config
```

**Performance**:
- Reload latency: <100ms (p95)
- Lock contention: <1ms write locks
- CPU overhead: <0.5% idle

**File**: `codex-rs/spec-kit/src/config/hot_reload.rs:1-100`

---

#### 7. Model Providers (`codex-protocol`, `codex-chatgpt` crates)

**Purpose**: Communication with AI model APIs

**Providers**:
- **OpenAI**: GPT-5, GPT-4o, o3 (Responses API)
- **Anthropic**: Claude Sonnet, Haiku, Opus (via CLI)
- **Google**: Gemini Pro, Flash (via CLI)
- **Custom**: Any OpenAI-compatible endpoint

**Features**:
- Streaming responses (SSE)
- Retry logic (exponential backoff)
- Rate limit handling
- Zero Data Retention support (ZDR)

**File**: `codex-rs/protocol/src/lib.rs`, `codex-rs/chatgpt/src/lib.rs`

---

## Data Flow

### Conversation Flow

```
1. User Input
   └→ ChatWidget.handle_key_event()
       └→ ChatWidget.submit_prompt()
           └→ codex_op_tx.send(Op::NewMessage)

2. Async Backend
   └→ ConversationManager.submit(op)
       └→ Conversation.process()
           └→ Model Provider API (OpenAI/Anthropic/Google)

3. Response Streaming
   └→ conversation.next_event()
       └→ app_event_tx.send(AppEvent::CodexEvent)
           └→ ChatWidget.handle_event()
               └→ Render response tokens
```

### Spec-Kit Automation Flow

```
1. User Command: /speckit.auto SPEC-ID
   └→ CommandRegistry.find("speckit.auto")
       └→ AutoCommand.execute(widget, args)

2. Pipeline Initialization
   └→ PipelineCoordinator.start_pipeline()
       └→ Load SPEC from docs/SPEC-{ID}-*/spec.md

3. Stage Execution Loop
   └→ For each stage (specify, plan, tasks, implement, validate, audit, unlock):
       ├→ NativeQualityGate.check() [if native stage]
       ├→ AgentOrchestrator.spawn_agents() [if multi-agent]
       ├→ ConsensusCoordinator.synthesize() [aggregate results]
       ├→ QualityGateBroker.validate() [checkpoint]
       └→ EvidenceRepository.store() [artifacts]

4. Completion
   └→ PipelineCoordinator.complete()
       └→ Push results to ChatWidget history
```

### MCP Tool Invocation Flow

```
1. Agent requests tool
   └→ Model response: {"type": "tool_use", "name": "filesystem__read_file"}

2. Tool dispatch
   └→ McpConnectionManager.invoke_tool()
       └→ Find MCP client by tool prefix ("filesystem")
           └→ McpClient.call_tool()

3. Server communication
   └→ outgoing_tx.send(JSONRPCRequest)
       └→ Writer task → stdin (JSON)
           └→ MCP server processes request
               └→ stdout (JSON) → Reader task
                   └→ pending.get(request_id).send(response)

4. Result returned
   └→ Tool result forwarded to model
       └→ Model continues generation with tool output
```

---

## Technology Stack

### Core Technologies

**Language**: Rust (Edition 2024)
- Memory safety without garbage collection
- Zero-cost abstractions
- Fearless concurrency

**Async Runtime**: Tokio
- Multi-threaded work-stealing scheduler
- Async I/O for network, file, subprocess
- Channels for sync/async boundary

**Terminal UI**: Ratatui (v0.29.0, patched fork)
- Immediate-mode rendering
- Cross-platform terminal support
- Widget composition

**Database**: SQLite (rusqlite crate)
- Embedded database (no server)
- ACID transactions
- WAL mode for concurrency

**Serialization**: Serde (JSON, TOML, YAML)
- Compile-time serialization
- Type-safe deserialization
- Schema validation

### Supporting Libraries

**Networking**:
- `reqwest`: HTTP client (model providers)
- `eventsource-stream`: Server-Sent Events (streaming)

**Configuration**:
- `toml`: Config file parsing
- `notify`: Filesystem watching (hot-reload)

**Database**:
- `rusqlite`: SQLite bindings
- `r2d2`: Connection pooling
- `r2d2_sqlite`: SQLite adapter for r2d2

**MCP**:
- `mcp-types`: Protocol definitions (internal)
- `tokio-util`: Codec for line-delimited JSON

**Error Handling**:
- `anyhow`: Flexible error types
- `thiserror`: Custom error derive

**CLI**:
- `clap`: Command-line argument parsing
- `clap_complete`: Shell completion generation

---

## Fork-Specific Additions

### Isolation Strategy

**Goal**: Add fork features without upstream conflicts

**Implementation**:

1. **Isolated Module Tree**:
   ```
   codex-rs/tui/src/chatwidget/
   ├── mod.rs (upstream)
   └── spec_kit/ (fork, 55 modules, 98.8% isolated)
       ├── mod.rs
       ├── command_registry.rs
       ├── pipeline_coordinator.rs
       └── ... (50+ modules)
   ```

2. **Friend Module Pattern**:
   ```rust
   // In chatwidget/mod.rs (upstream file, minimal change)
   pub mod spec_kit;  // Single line addition

   // spec_kit modules can access ChatWidget private fields
   impl ChatWidget {
       fn internal_method(&mut self) { /* ... */ }
   }
   ```

3. **Dynamic Command Registry** (avoids upstream enum):
   ```rust
   // Upstream: SlashCommand enum
   pub enum SlashCommand { New, Model, Reasoning, ... }

   // Fork: Dynamic registry (no enum growth)
   pub trait SpecKitCommand { /* ... */ }
   SPEC_KIT_REGISTRY.register(Box::new(AutoCommand));
   ```

4. **Context Trait** (decouples from ChatWidget):
   ```rust
   pub trait SpecKitContext {
       fn submit_operation(&self, op: Op);
       fn push_error(&mut self, message: String);
       // ... methods spec-kit needs
   }

   impl SpecKitContext for ChatWidget { /* ... */ }
   ```

**Result**:
- **98.8% isolation**: 55 modules, 1,222 lines extracted
- **Zero upstream conflicts**: Rebases require <10 lines of merge
- **Testability**: MockSpecKitContext for unit tests
- **Maintainability**: Clear separation of concerns

**Evidence**: `docs/spec-kit/REFACTORING_COMPLETE_SUMMARY.md`

---

### New Crates

**spec-kit** (`codex-rs/spec-kit/`):
- Configuration system (hot-reload, 5-tier precedence)
- Retry logic (exponential backoff, jitter)
- Types and error handling
- Evidence management
- Cost tracking

**Purpose**: Reusable library for spec-kit automation (can extract as standalone in future per MAINT-10)

---

## Integration Points

### 1. TUI ↔ Core Services

**Boundary**: Async/sync channel boundary

**Direction**: Bidirectional
- **TUI → Core**: `UnboundedSender<Op>`
- **Core → TUI**: `AppEventSender`

**Pattern**: Message passing with typed events

---

### 2. Core ↔ MCP Servers

**Boundary**: stdio subprocess communication

**Direction**: Bidirectional (JSON-RPC over stdin/stdout)

**Protocol**: Line-delimited JSON (Model Context Protocol)

**Lifecycle**:
1. Spawn subprocess (`tokio::process::Command`)
2. Initialize with `initialize` request
3. List tools with `tools/list` request
4. Invoke tools with `tools/call` request
5. Kill on app exit (`kill_on_drop = true`)

---

### 3. Core ↔ Model Providers

**Boundary**: HTTPS API requests

**Direction**: Request/response with streaming

**Protocols**:
- **OpenAI**: Responses API (streaming SSE)
- **Anthropic**: Messages API (via CLI subprocess)
- **Google**: Gemini API (via CLI subprocess)

**Features**:
- Retry logic (exponential backoff)
- Rate limit handling (429 response)
- Streaming token delivery

---

### 4. Spec-Kit ↔ Database

**Boundary**: R2D2 connection pool

**Direction**: Read/write consensus artifacts

**Operations**:
- Store agent outputs (per-agent JSON blobs)
- Store consensus synthesis (aggregated results)
- Query historical runs (evidence retrieval)
- Atomic transactions (ACID guarantees)

---

### 5. Configuration ↔ Filesystem

**Boundary**: File watching (notify crate)

**Direction**: Read config.toml, watch for changes

**Hot-Reload**:
1. Filesystem change event
2. Debounce (300ms window)
3. Parse and validate config
4. Atomic update via Arc<RwLock>
5. Emit reload event to UI

---

## Summary

**Architecture Highlights**:

1. **Clean Layering**: TUI → Core → Infrastructure
2. **Async/Sync Hybrid**: Tokio backend, Ratatui UI
3. **Modular Design**: 24-crate workspace
4. **Fork Isolation**: 98.8% isolation via friend modules
5. **Performance-First**: 6.6× DB speedup, 5.3× MCP speedup
6. **Fault Tolerance**: Retry logic, graceful degradation

**Next Steps**:
- [Cargo Workspace Guide](cargo-workspace.md) - Detailed crate documentation
- [TUI Architecture](tui-architecture.md) - Ratatui and async/sync patterns
- [MCP Integration](mcp-integration.md) - Native client details
- [Database Layer](database-layer.md) - SQLite optimization

---

**File References**:
- Workspace: `codex-rs/Cargo.toml`
- TUI: `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/chatwidget/mod.rs`
- Spec-Kit: `codex-rs/spec-kit/src/lib.rs`, `codex-rs/tui/src/chatwidget/spec_kit/mod.rs`
- Core: `codex-rs/core/src/lib.rs`
- MCP: `codex-rs/mcp-client/src/mcp_client.rs`
- DB: `codex-rs/core/src/db/connection.rs`
- Config: `codex-rs/spec-kit/src/config/hot_reload.rs`
