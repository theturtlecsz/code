# MCP Servers

MCP server configuration, custom servers, and lifecycle management.

---

## Overview

**MCP** (Model Context Protocol) enables AI agents to access external tools and resources through standardized servers.

**Use Cases**:
- Memory systems (local-memory for knowledge persistence)
- Git operations (git-status for repository inspection)
- Custom tools (HAL for policy validation)
- External services (databases, APIs, file systems)

**Configuration**: `[mcp_servers.<name>]` sections in `config.toml`

---

## MCP Server Configuration

### Basic Configuration

```toml
# ~/.code/config.toml

[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]
startup_timeout_ms = 10000  # 10 seconds
```

---

### Configuration Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command` | string | Yes | Executable command |
| `args` | array | No | Command arguments |
| `env` | table | No | Environment variables |
| `startup_timeout_ms` | integer | No | Startup timeout (default: 10000ms) |

---

## Built-in MCP Servers

### local-memory (Knowledge Persistence)

**Purpose**: Store and retrieve high-value knowledge (architecture decisions, patterns, bug fixes)

**Configuration**:
```toml
[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]
startup_timeout_ms = 10000
```

**Installation**:
```bash
# Auto-installed on first use via npx -y
# Or install globally:
npm install -g @modelcontextprotocol/server-memory
```

**Tools Provided**:
- `mcp__local-memory__store_memory` - Store knowledge
- `mcp__local-memory__search` - Search knowledge
- `mcp__local-memory__analysis` - Analyze patterns

**Usage**:
```markdown
Use mcp__local-memory__store_memory:
- content: "Routing bug fixed: SpecKitCommand wasn't passing config..."
- domain: "debugging"
- tags: ["type:bug-fix", "spec:SPEC-KIT-066"]
- importance: 9
```

**Storage**: `~/.code/mcp-memory/` (SQLite database)

---

### git-status (Repository Inspection)

**Purpose**: Inspect Git repository state, history, changes

**Configuration**:
```toml
[mcp_servers.git-status]
command = "npx"
args = ["-y", "@just-every/mcp-server-git"]
env = { LOG_LEVEL = "info" }
```

**Tools Provided**:
- `mcp__git-status__status` - Get git status
- `mcp__git-status__diff` - Get diff for files
- `mcp__git-status__log` - Get commit history

**Use Case**: Automated commit message generation, change analysis

---

### HAL (Policy Validation)

**Purpose**: Validate spec-kit policies (storage policy, tag schema, quality gates)

**Configuration**:
```toml
[mcp_servers.hal]
command = "/path/to/hal-server"
args = ["--mode", "strict"]
env = { HAL_SECRET_KAVEDARR_API_KEY = "..." }
startup_timeout_ms = 15000
```

**Tools Provided**:
- `mcp__hal__validate_storage_policy` - Check local-memory usage
- `mcp__hal__validate_tag_schema` - Check tag naming
- `mcp__hal__validate_quality_gates` - Check consensus

**Note**: HAL server is project-specific (not publicly available)

---

## Custom MCP Servers

### Creating a Custom Server

**Example**: Database query server

```toml
[mcp_servers.database]
command = "/path/to/db-mcp-server"
args = ["--connection-string", "postgres://localhost/mydb"]
env = { DB_PASSWORD = "secret" }
startup_timeout_ms = 20000  # Longer timeout for DB connection
```

**Server Implementation**: See [MCP Server SDK](https://github.com/modelcontextprotocol/typescript-sdk)

---

### Custom Server Example (Node.js)

```javascript
// db-mcp-server.js
const { MCPServer } = require('@modelcontextprotocol/sdk');
const { Pool } = require('pg');

const server = new MCPServer({
  name: 'database',
  version: '1.0.0',
});

const pool = new Pool({
  connectionString: process.argv[2],
});

server.tool({
  name: 'query',
  description: 'Execute SQL query',
  parameters: {
    sql: { type: 'string', description: 'SQL query to execute' },
  },
  async handler({ sql }) {
    const result = await pool.query(sql);
    return { rows: result.rows };
  },
});

server.start();
```

**Configuration**:
```toml
[mcp_servers.database]
command = "node"
args = ["/path/to/db-mcp-server.js", "postgres://localhost/mydb"]
```

---

## MCP Server Lifecycle

### Startup Process

```
1. Config loaded → Parse [mcp_servers.*] sections
2. Spawn process → Execute command with args
3. Handshake → Initialize MCP protocol
4. List tools → Request tools/list from server
5. Cache tools → Store tool metadata
6. Ready → Server available for use
```

**Timeout**: `startup_timeout_ms` (default: 10000ms)

---

### Lazy Loading

**Default Behavior**: MCP servers are **not** started until first use

**Benefit**: Save resources by only starting needed servers

**Example**:
```toml
# Configured but not started
[mcp_servers.database]
command = "node"
args = ["/path/to/db-server.js"]

# Only started when tool is called:
# Use mcp__database__query: "SELECT * FROM users"
```

---

### Startup Optimization

**Cache Tool List**:
```
First session:
  1. Start MCP server (~500ms)
  2. Request tools/list (~100ms)
  3. Cache to ~/.code/mcp-cache/database.json
  4. Use tools

Subsequent sessions:
  1. Load cached tools from ~/.code/mcp-cache/database.json (~10ms)
  2. Lazy-start server only when tool is called
```

**Benefit**: Faster session startup (no waiting for MCP servers)

---

### Shutdown Process

```
1. Session end → Send shutdown signal to all MCP servers
2. Wait for clean shutdown (max 5s)
3. Force kill if timeout
4. Clean up temp files
```

---

## Environment Variables

### Server-Specific Environment

```toml
[mcp_servers.custom]
command = "/path/to/server"
env = {
    API_KEY = "secret",
    LOG_LEVEL = "debug",
    FEATURE_FLAG = "experimental"
}
```

**Scope**: Only available to the MCP server process

---

### Global Environment Variables

```bash
# Available to all MCP servers
export MCP_LOG_LEVEL="debug"
export MCP_TIMEOUT="30000"
```

**Use Case**: Global MCP debugging settings

---

## Timeouts and Retries

### Startup Timeout

**Default**: 10000ms (10 seconds)

**Configuration**:
```toml
[mcp_servers.slow-server]
command = "/path/to/slow-server"
startup_timeout_ms = 30000  # 30 seconds for slow startup
```

**Behavior**: If server doesn't respond within timeout, startup fails

---

### Tool Call Timeout

**Default**: Inherited from validation.timeout_seconds

**Override**:
```toml
[validation]
timeout_seconds = 60  # 60 seconds for all MCP tool calls
```

---

### Retry Logic

**Startup Failures**: No automatic retry (manual restart required)

**Tool Call Failures**: Retry up to 3 times with exponential backoff

**Example**:
```
1. Tool call fails (network error)
2. Wait 1s
3. Retry (1/3)
4. Wait 2s
5. Retry (2/3)
6. Wait 4s
7. Retry (3/3)
8. Give up, report error to agent
```

---

## Debugging MCP Servers

### Enable MCP Logging

```bash
export RUST_LOG=codex_mcp_client=debug
code
```

**Log Output**:
```
[DEBUG] Starting MCP server: local-memory
[DEBUG] Command: npx -y @modelcontextprotocol/server-memory
[DEBUG] Handshake complete
[DEBUG] Requesting tools/list...
[DEBUG] Received 3 tools: store_memory, search, analysis
[DEBUG] MCP server ready: local-memory
```

---

### Test MCP Server Manually

**MCP Inspector** (official debugging tool):
```bash
npm install -g @modelcontextprotocol/inspector

# Test local-memory server
npx @modelcontextprotocol/inspector npx -y @modelcontextprotocol/server-memory
```

**Features**:
- Test tool calls
- Inspect responses
- Debug connection issues

---

### Check MCP Server Status

```bash
code --mcp-status
```

**Output**:
```
MCP Servers (3 configured):

local-memory:
  Status: Running (PID: 12345)
  Command: npx -y @modelcontextprotocol/server-memory
  Uptime: 2h 15m
  Tools: 3 (store_memory, search, analysis)

git-status:
  Status: Not started (lazy-load)
  Command: npx -y @just-every/mcp-server-git
  Tools: 3 (cached)

database:
  Status: Failed (startup timeout)
  Command: /path/to/db-server
  Error: Connection timeout after 20000ms
```

---

### Force Restart MCP Server

```bash
code --mcp-restart local-memory
```

**Use Case**: Server crashed, hung, or behaving incorrectly

---

## Common MCP Servers

### Filesystem Server

```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/allowed/path"]
```

**Tools**: Read/write files in allowed directory

---

### HTTP Server

```toml
[mcp_servers.http]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-http"]
```

**Tools**: Make HTTP requests

---

### Database Servers

**PostgreSQL**:
```toml
[mcp_servers.postgres]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "postgres://localhost/mydb"]
```

**SQLite**:
```toml
[mcp_servers.sqlite]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-sqlite", "/path/to/db.sqlite"]
```

---

### Custom API Server

```toml
[mcp_servers.custom-api]
command = "/path/to/custom-mcp-server"
args = ["--api-url", "https://api.example.com"]
env = { API_TOKEN = "secret" }
```

---

## Security Considerations

### 1. Validate Command Paths

**Good**:
```toml
[mcp_servers.trusted]
command = "npx"  # Well-known command
args = ["-y", "@modelcontextprotocol/server-memory"]
```

**Bad**:
```toml
[mcp_servers.untrusted]
command = "/tmp/random-script.sh"  # ❌ Untrusted source
```

---

### 2. Avoid Secrets in Config

**Good**:
```toml
[mcp_servers.database]
command = "/path/to/db-server"
env = { DB_PASSWORD = "secret" }  # ⚠️ Still visible in config

# Better: Use environment variable
env = { DB_PASSWORD = "$DB_PASSWORD_FROM_ENV" }
```

**Best**:
```bash
# Store secret in environment
export DB_PASSWORD="secret"
```

```toml
[mcp_servers.database]
command = "/path/to/db-server"
# Server reads $DB_PASSWORD from environment
```

---

### 3. Restrict Network Access

**Sandbox Mode**: MCP servers inherit sandbox restrictions

```toml
sandbox_mode = "read-only"  # MCP servers also read-only

[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/safe/path"]
```

---

## Best Practices

### 1. Use Lazy Loading

**Default behavior** (don't change):
- Servers start on first use
- Faster session startup
- Lower resource usage

---

### 2. Set Appropriate Timeouts

**Fast servers** (in-memory):
```toml
[mcp_servers.memory]
startup_timeout_ms = 5000  # 5s
```

**Slow servers** (database, network):
```toml
[mcp_servers.database]
startup_timeout_ms = 30000  # 30s
```

---

### 3. Monitor MCP Server Logs

```bash
export RUST_LOG=debug
code

# Check logs for MCP errors
tail -f ~/.code/debug.log | grep MCP
```

---

### 4. Test Servers with MCP Inspector

```bash
npx @modelcontextprotocol/inspector <command> <args>
```

**Benefit**: Catch configuration errors before using in production

---

## Summary

**MCP Servers** enable:
- Knowledge persistence (local-memory)
- Git operations (git-status)
- Custom tools (database, API, filesystem)
- External service integration

**Configuration**:
```toml
[mcp_servers.<name>]
command = "executable"
args = ["arg1", "arg2"]
env = { KEY = "value" }
startup_timeout_ms = 10000
```

**Features**:
- Lazy loading (start on first use)
- Tool caching (faster startup)
- Automatic retry (tool call failures)
- Hot-reload support (config changes)

**Debugging**:
- MCP Inspector (test servers)
- `--mcp-status` (check status)
- `--mcp-restart` (force restart)
- Debug logging (`RUST_LOG=debug`)

**Next**: [Environment Variables](environment-variables.md)
