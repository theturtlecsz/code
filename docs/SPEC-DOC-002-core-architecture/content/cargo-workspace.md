# Cargo Workspace Structure

Complete guide to the 24-crate workspace architecture.

---

## Workspace Overview

**Location**: `/home/user/code/codex-rs/Cargo.toml`

**Statistics**:
- **24 member crates**
- **226,607 lines of Rust code**
- **538 source files**
- **Edition 2024** (Rust 2024 edition features)

---

## Crate Categories

### Core Application Crates

#### **tui** - Terminal User Interface
**Purpose**: Main application binary, Ratatui-based TUI

**Key Modules**:
- `app.rs`: Application state and event loop
- `chatwidget/`: Conversation interface (912K LOC mod.rs)
- `chatwidget/spec_kit/`: Fork feature integration (55 modules)

**Dependencies**: `ratatui`, `codex-core`, `spec-kit`, `mcp-client`, `tokio`

**Binary**: `code-tui`

**File**: `codex-rs/tui/Cargo.toml`

---

#### **cli** - CLI Wrapper
**Purpose**: Command-line interface entry point

**Responsibilities**:
- Argument parsing (clap)
- Shell completion generation
- Delegates to `code-tui` binary

**Dependencies**: `codex-tui`, `codex-core`, `clap`, `clap_complete`

**Binary**: `code`

**File**: `codex-rs/cli/Cargo.toml`

---

#### **spec-kit** - Multi-Agent Automation Framework (Fork)
**Purpose**: Reusable library for spec-kit automation

**Modules**:
- `config/`: Configuration system (hot-reload, 5-tier precedence)
- `retry/`: Exponential backoff retry logic
- `types.rs`: Core types (SpecStage, QualityCheckpoint)
- `error.rs`: Error handling

**Dependencies**: `codex-core`, `mcp-types`, `rusqlite`, `notify`, `tokio`

**Note**: Can be extracted as standalone crate (MAINT-10 future work)

**File**: `codex-rs/spec-kit/Cargo.toml`

---

### Backend Service Crates

#### **core** - Backend Services
**Purpose**: Backend orchestration (conversation, MCP, DB, config)

**Key Modules**:
- `conversation_manager.rs`: Agent lifecycle
- `mcp_connection_manager.rs`: MCP server aggregation
- `db/`: SQLite connection pooling, transactions
- `config_types.rs`: Configuration data structures
- `protocol.rs`: Model provider abstraction

**Dependencies**: `mcp-client`, `codex-protocol`, `rusqlite`, `r2d2`, `tokio`

**File**: `codex-rs/core/Cargo.toml`

---

### MCP (Model Context Protocol) Crates

#### **mcp-client** - Async MCP Client
**Purpose**: Subprocess communication with MCP servers

**Key Features**:
- Line-delimited JSON-RPC over stdio
- Concurrent reader/writer tasks (prevent deadlock)
- Request/response pairing via HashMap
- 1MB buffer for large responses

**Dependencies**: `mcp-types`, `tokio`, `tokio-util`, `serde_json`

**File**: `codex-rs/mcp-client/src/mcp_client.rs:63-150`

---

#### **mcp-server** - MCP Server Implementation
**Purpose**: Implements MCP server for tool exposure

**Dependencies**: `codex-core`, `codex-protocol`, `mcp-types`, `tokio`

**File**: `codex-rs/mcp-server/Cargo.toml`

---

#### **mcp-types** - Protocol Types
**Purpose**: JSON-RPC and MCP protocol definitions

**Key Types**:
- `JSONRPCMessage`: Request/Response/Notification
- `ToolInfo`: Tool metadata
- `McpServerConfig`: Server configuration

**Dependencies**: `serde`, `serde_json`, `ts-rs` (TypeScript bindings)

**File**: `codex-rs/mcp-types/Cargo.toml`

---

### Protocol & Model Provider Crates

#### **protocol** - OpenAI Protocol
**Purpose**: OpenAI API client and types

**Features**:
- Responses API support
- Chat Completions API support
- Streaming SSE support
- Request/response types

**Dependencies**: `serde`, `serde_json`, `reqwest`

**File**: `codex-rs/protocol/Cargo.toml`

---

#### **chatgpt** - ChatGPT Auth
**Purpose**: ChatGPT authentication flow

**Dependencies**: `reqwest`, `serde`

**File**: `codex-rs/chatgpt/Cargo.toml`

---

### Utility & Tooling Crates

#### **common** - Shared Utilities
**Purpose**: Common utilities across crates

**Modules**:
- Model presets (GPT-5, Claude, Gemini)
- Elapsed time formatting
- CLI helpers

**Dependencies**: `serde`, `reqwest`, `clap`

**File**: `codex-rs/common/Cargo.toml`

---

#### **git-tooling** - Git Operations
**Purpose**: Git integration helpers

**File**: `codex-rs/git-tooling/Cargo.toml`

---

#### **file-search** - File Search
**Purpose**: Fuzzy file search (@ trigger in composer)

**Dependencies**: `nucleo-matcher` (fuzzy matching)

**File**: `codex-rs/file-search/Cargo.toml`

---

### Execution & Sandbox Crates

#### **exec** - Command Execution
**Purpose**: Sandboxed command execution

**File**: `codex-rs/exec/Cargo.toml`

---

#### **execpolicy** - Execution Policy
**Purpose**: Approval policy enforcement

**File**: `codex-rs/execpolicy/Cargo.toml`

---

#### **linux-sandbox** - Linux Sandboxing
**Purpose**: Landlock, seccomp sandboxing

**Dependencies**: `landlock`, `seccompiler`

**File**: `codex-rs/linux-sandbox/Cargo.toml`

---

### Supporting Crates

**login**: Authentication helpers
**browser**: Browser integration (CDP)
**ollama**: Ollama model support
**arg0**: Binary name detection
**apply-patch**: Diff application
**ansi-escape**: ANSI escape code handling
**codex-version**: Version utilities

---

## Dependency Graph

```
tui (main binary)
├── spec-kit (fork feature)
│   ├── mcp-types
│   ├── codex-core
│   │   ├── mcp-client
│   │   │   └── mcp-types
│   │   ├── codex-protocol
│   │   ├── rusqlite
│   │   └── r2d2
│   └── rusqlite (direct)
├── codex-core
├── mcp-client
├── codex-protocol
├── codex-common
├── ratatui (v0.29.0 patched)
└── tokio

cli (entry point)
├── codex-tui
└── clap

core (backend)
├── mcp-client
├── codex-protocol
├── rusqlite
├── r2d2
└── tokio
```

**Key Observations**:
- **spec-kit** depends on **core** (reuses DB, MCP)
- **tui** depends on **spec-kit** (friend module pattern)
- **mcp-client** is lightweight (only `mcp-types`, `tokio`)
- **core** aggregates backend services

---

## Build Profiles

### dev-fast (Default Development)
```toml
[profile.dev-fast]
inherits = "dev"
opt-level = 1              # Basic optimization
debug = 1                  # Line tables only
incremental = true         # Incremental compilation
codegen-units = 256        # Parallel codegen
lto = "off"                # No LTO (faster builds)
```

**Use**: `./build-fast.sh` or `cargo build --profile dev-fast`

**Build Time**: ~30-60 seconds (incremental)

**Binary**: `codex-rs/target/dev-fast/code`

---

### release (Production)
```toml
[profile.release]
lto = "fat"                # Full LTO (link-time optimization)
strip = "symbols"          # Remove debug symbols
codegen-units = 1          # Single codegen unit (max optimization)
```

**Use**: `cargo build --release`

**Build Time**: ~5-10 minutes (full rebuild)

**Binary Size**: ~15-20 MB (stripped)

**Binary**: `codex-rs/target/release/code`

---

### perf (Performance Profiling)
```toml
[profile.perf]
inherits = "release"
incremental = true
codegen-units = 256
lto = "off"
debug = 2                  # Full debug info
strip = "none"             # Keep symbols
split-debuginfo = "packed" # Separate debug file
```

**Use**: `cargo build --profile perf`

**Purpose**: Performance profiling with `perf`, `flamegraph`

---

## Workspace-Level Configuration

### Shared Dependencies
```toml
[workspace.dependencies]
# Internal crates (path dependencies)
codex-core = { path = "core" }
codex-tui = { path = "tui" }
spec-kit = { path = "spec-kit" }
mcp-client = { path = "mcp-client" }
mcp-types = { path = "mcp-types" }
# ... 20+ internal crates

# External crates (version pinning)
tokio = "1"
ratatui = "0.29.0"
rusqlite = { version = "0.37", features = ["bundled"] }
reqwest = "0.12"
serde = "1"
serde_json = "1"
anyhow = "1"
thiserror = "2.0.16"
# ... 100+ external dependencies
```

**Benefits**:
- Single version source of truth
- Automatic version consistency
- Easier dependency updates

---

### Workspace Lints
```toml
[workspace.lints.clippy]
expect_used = "deny"               # Forbid .expect()
unwrap_used = "deny"               # Forbid .unwrap()
manual_ok_or = "deny"              # Enforce .ok_or()
needless_borrow = "deny"           # Remove unnecessary borrows
redundant_clone = "deny"           # Remove unnecessary clones
uninlined_format_args = "deny"     # Use format!("{var}") not format!("{}", var)
# ... 30+ lints
```

**Enforcement**: All crates inherit workspace lints unless overridden

---

### Patched Dependencies

**Ratatui Fork**:
```toml
[patch.crates-io]
ratatui = { git = "https://github.com/nornagon/ratatui", branch = "nornagon-v0.29.0-patch" }
```

**Reason**: Custom patches for TUI improvements

---

## Build System

### Fast Builds (Development)

```bash
# Use build-fast.sh
./build-fast.sh

# Or directly
cd codex-rs
cargo build --profile dev-fast --bin code --bin code-tui
```

**Output**: `codex-rs/target/dev-fast/code`

---

### Release Builds (Production)

```bash
# Full release build
cd codex-rs
cargo build --release --bin code --bin code-tui --bin code-exec

# Quick build (code only)
npm run build:quick
```

**Output**: `codex-rs/target/release/code`

---

### Testing

```bash
cd codex-rs

# All tests
cargo test

# Specific crate
cargo test -p spec-kit

# Specific test
cargo test -p codex-tui spec_auto
```

---

### Code Quality

```bash
cd codex-rs

# Format
cargo fmt --all

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check (no codegen)
cargo check --workspace
```

---

## Crate Size Breakdown

| Crate | LOC (Rust) | Files | Key Responsibility |
|-------|-----------|-------|-------------------|
| **tui** | ~45,000 | 120 | UI, ChatWidget, Spec-Kit integration |
| **spec-kit** (lib) | ~8,000 | 25 | Config, retry, types (reusable) |
| **spec-kit** (tui integration) | ~35,000 | 55 | Command handlers, consensus, pipeline |
| **core** | ~30,000 | 85 | Backend services, MCP, DB, config |
| **mcp-client** | ~3,500 | 12 | Async MCP client |
| **mcp-server** | ~5,000 | 18 | MCP server implementation |
| **protocol** | ~12,000 | 35 | OpenAI protocol client |
| **Others** | ~88,000 | ~230 | Utilities, tooling, sandbox |

**Total**: ~226,607 LOC across 538 files

---

## Adding New Crates

### When to Create a New Crate

**Good reasons**:
- ✅ Reusable component (can extract to standalone library)
- ✅ Clear responsibility boundary
- ✅ Independent compilation (speeds up builds)
- ✅ Different dependency requirements
- ✅ Potential for external use (spec-kit library)

**Bad reasons**:
- ❌ Small utility module (use `common` instead)
- ❌ Tightly coupled to single crate
- ❌ No clear abstraction boundary

---

### Creating a New Crate

```bash
cd codex-rs

# Create new crate in workspace
cargo new --lib my-new-crate

# Or with specific edition
cargo new --lib --edition 2024 my-new-crate
```

**Add to workspace** (`Cargo.toml`):
```toml
[workspace]
members = [
    "ansi-escape",
    # ... existing crates
    "my-new-crate",  # Add here
]
```

**Add workspace dependency**:
```toml
[workspace.dependencies]
my-new-crate = { path = "my-new-crate" }
```

**Use in other crates**:
```toml
# In another crate's Cargo.toml
[dependencies]
my-new-crate = { workspace = true }
```

---

## Next Steps

- [TUI Architecture](tui-architecture.md) - Detailed TUI and ChatWidget design
- [Core Execution](core-execution.md) - Agent orchestration and providers
- [MCP Integration](mcp-integration.md) - Native MCP client details
- [Database Layer](database-layer.md) - SQLite optimization

---

**File References**:
- Workspace: `codex-rs/Cargo.toml:1-234`
- Build profiles: `codex-rs/Cargo.toml:201-234`
- TUI: `codex-rs/tui/Cargo.toml`
- Spec-Kit: `codex-rs/spec-kit/Cargo.toml`
- Core: `codex-rs/core/Cargo.toml`
- MCP Client: `codex-rs/mcp-client/Cargo.toml`
