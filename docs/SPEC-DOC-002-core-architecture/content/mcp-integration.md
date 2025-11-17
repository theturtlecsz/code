# MCP Integration

Native Model Context Protocol client and server integration.

---

## Overview

**MCP (Model Context Protocol)** allows AI models to access external tools:
- File operations
- Database queries
- API calls
- Custom tools

**Native Implementation**: 5.3× faster than subprocess approach (8.7ms typical vs 46ms)

**Location**: `codex-rs/mcp-client/`, `codex-rs/mcp-types/`, `codex-rs/core/src/mcp_connection_manager.rs`

---

## Architecture

```
Application (TUI/Core)
    ↓
McpConnectionManager (singleton, shared across widgets)
    ├→ McpClient("filesystem")
    │   ├→ Writer Task (outgoing → stdin)
    │   ├→ Reader Task (stdout → pending HashMap)
    │   └→ Subprocess (MCP server)
    ├→ McpClient("git-status")
    └→ McpClient("local-memory")

MCP Servers (subprocesses)
├→ @modelcontextprotocol/server-filesystem
├→ @modelcontextprotocol/server-git-status
└→ @modelcontextprotocol/server-local-memory
```

---

## McpClient Implementation

### Core Structure

**Location**: `codex-rs/mcp-client/src/mcp_client.rs:63-150`

```rust
pub struct McpClient {
    child: tokio::process::Child,                    // Subprocess handle
    outgoing_tx: mpsc::Sender<JSONRPCMessage>,      // Send requests (128 capacity)
    pending: Arc<Mutex<HashMap<i64, PendingSender>>>, // Request ID → response oneshot
    id_counter: AtomicI64,                          // Monotonic ID generator
}

type PendingSender = oneshot::Sender<JSONRPCMessage>;

impl McpClient {
    pub async fn new_stdio_client(
        program: OsString,
        args: Vec<OsString>,
        env: Option<HashMap<String, String>>,
    ) -> io::Result<Self> {
        // Spawn MCP server subprocess
        let mut child = Command::new(program)
            .args(args)
            .env_clear()
            .envs(env.unwrap_or_default())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let (outgoing_tx, mut outgoing_rx) = mpsc::channel(128);
        let pending = Arc::new(Mutex::new(HashMap::new()));

        // Writer task: outgoing_rx → stdin (JSON-RPC requests)
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = outgoing_rx.recv().await {
                let json = serde_json::to_string(&msg)?;
                stdin.write_all(json.as_bytes()).await?;
                stdin.write_all(b"\n").await?;  // Newline-delimited
            }
            Ok::<(), io::Error>(())
        });

        // Reader task: stdout → pending HashMap (JSON-RPC responses)
        let pending_clone = Arc::clone(&pending);
        tokio::spawn(async move {
            let mut lines = BufReader::with_capacity(1024 * 1024, stdout).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let msg: JSONRPCMessage = serde_json::from_str(&line)?;

                // Match response to request
                if let Some(id) = msg.id {
                    if let Some(tx) = pending_clone.lock().unwrap().remove(&id) {
                        let _ = tx.send(msg);
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        });

        Ok(Self {
            child,
            outgoing_tx,
            pending,
            id_counter: AtomicI64::new(1),
        })
    }
}
```

**Key Design Choices**:
- **1MB buffer**: `BufReader::with_capacity(1024 * 1024)` handles large tool responses
- **Line-delimited JSON**: Each JSON-RPC message on one line
- **Concurrent I/O**: Separate reader/writer tasks prevent deadlock
- **kill_on_drop**: Subprocess cleaned up automatically

---

### JSON-RPC Protocol

**Types**: `codex-rs/mcp-types/src/jsonrpc.rs`

```rust
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum JSONRPCMessage {
    Request {
        jsonrpc: String,  // "2.0"
        id: i64,
        method: String,
        params: Option<Value>,
    },
    Response {
        jsonrpc: String,
        id: i64,
        result: Option<Value>,
        error: Option<JSONRPCError>,
    },
    Notification {
        jsonrpc: String,
        method: String,
        params: Option<Value>,
    },
}
```

**Wire Format** (newline-delimited):
```json
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":null}
{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"read_file","description":"..."}]}}
```

---

### Tool Invocation

```rust
impl McpClient {
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value> {
        // Generate unique request ID
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);

        // Send request
        let request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id,
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": tool_name,
                "arguments": arguments,
            })),
        };

        self.outgoing_tx.send(request).await?;

        // Wait for response (with timeout)
        let response = tokio::time::timeout(
            Duration::from_secs(60),
            rx
        ).await??;

        // Extract result
        match response {
            JSONRPCMessage::Response { result: Some(result), .. } => Ok(result),
            JSONRPCMessage::Response { error: Some(err), .. } => Err(err.into()),
            _ => Err(Error::InvalidResponse),
        }
    }
}
```

**Flow**:
1. Generate unique ID (atomic counter)
2. Create oneshot channel, store in pending HashMap
3. Send JSON-RPC request to server (via outgoing_tx)
4. Writer task writes to stdin
5. Server processes request, writes to stdout
6. Reader task parses response
7. Match ID, send via oneshot channel
8. Remove from pending HashMap
9. Return result to caller

---

## McpConnectionManager

**Purpose**: Central hub for all MCP servers, tool aggregation

**Location**: `codex-rs/core/src/mcp_connection_manager.rs:84-150`

```rust
pub struct McpConnectionManager {
    clients: HashMap<String, Arc<McpClient>>,  // Server name → client
    tools: HashMap<String, ToolInfo>,          // Qualified tool name → metadata
}

pub struct ToolInfo {
    pub server_name: String,
    pub tool_name: String,
    pub qualified_name: String,  // "server__tool_name"
    pub description: String,
    pub input_schema: Value,
}

impl McpConnectionManager {
    pub async fn new(
        mcp_servers: HashMap<String, McpServerConfig>,
        excluded_tools: HashSet<(String, String)>,
    ) -> Result<(Self, ClientStartErrors)> {
        let mut clients = HashMap::new();
        let mut tools = HashMap::new();
        let mut errors = ClientStartErrors::default();

        // Spawn all servers concurrently
        let mut join_set = JoinSet::new();

        for (server_name, config) in mcp_servers {
            let server_name_clone = server_name.clone();
            join_set.spawn(async move {
                let client = McpClient::new_stdio_client(
                    config.command.into(),
                    config.args.into_iter().map(OsString::from).collect(),
                    config.env,
                ).await?;

                // Initialize server
                client.call_method("initialize", json!({"protocolVersion": "1.0"})).await?;

                // List tools
                let response = client.call_method("tools/list", None).await?;
                let server_tools: Vec<Tool> = serde_json::from_value(response["tools"].clone())?;

                Ok::<(String, Arc<McpClient>, Vec<Tool>), Error>((
                    server_name_clone,
                    Arc::new(client),
                    server_tools,
                ))
            });
        }

        // Collect results
        while let Some(result) = join_set.join_next().await {
            match result? {
                Ok((server_name, client, server_tools)) => {
                    clients.insert(server_name.clone(), client);

                    // Qualify and aggregate tools
                    for tool in server_tools {
                        let qualified_name = qualify_tool_name(&server_name, &tool.name);

                        if !excluded_tools.contains(&(server_name.clone(), tool.name.clone())) {
                            tools.insert(qualified_name.clone(), ToolInfo {
                                server_name: server_name.clone(),
                                tool_name: tool.name.clone(),
                                qualified_name,
                                description: tool.description,
                                input_schema: tool.input_schema,
                            });
                        }
                    }
                },
                Err(e) => {
                    errors.add(server_name, e);
                },
            }
        }

        Ok((Self { clients, tools }, errors))
    }

    pub async fn invoke_tool(
        &self,
        qualified_name: &str,
        arguments: Value,
    ) -> Result<Value> {
        // Look up tool info
        let tool_info = self.tools.get(qualified_name)
            .ok_or(Error::ToolNotFound(qualified_name.to_string()))?;

        // Get client for server
        let client = self.clients.get(&tool_info.server_name)
            .ok_or(Error::ServerNotFound(tool_info.server_name.clone()))?;

        // Invoke tool on server
        client.call_tool(&tool_info.tool_name, arguments).await
    }
}
```

**Key Features**:
- **Concurrent initialization**: Spawn all servers in parallel (JoinSet)
- **Tool qualification**: `filesystem__read_file` (prevents name collisions)
- **Excluded tools**: Filter out unwanted tools
- **Error collection**: Track which servers failed to start
- **Shared clients**: `Arc<McpClient>` allows concurrent access

---

### Tool Name Qualification

**Purpose**: Prevent tool name collisions across servers

**Strategy**: Prefix with server name

```rust
fn qualify_tool_name(server_name: &str, tool_name: &str) -> String {
    let combined = format!("{}_{}", server_name, tool_name);

    // Limit to 64 chars (OpenAI tool name limit)
    if combined.len() > 64 {
        // Hash collision avoidance for long names
        let hash = sha1::Sha1::digest(combined.as_bytes());
        let prefix = &combined[..40];
        format!("{}_{:x}", prefix, hash)
    } else {
        combined.replace('-', "_")  // Normalize separators
    }
}
```

**Examples**:
- `read_file` (filesystem) → `filesystem__read_file`
- `query` (database) → `database__query`
- `very-long-server-name__very-long-tool-name` → `very-long-server-name__very-long-tool_<hash>`

---

### App-Level Shared Connection Manager

**Purpose**: Prevent MCP server process multiplication

**Problem**: Each ChatWidget spawning its own MCP connections would create N×M processes (N widgets × M servers)

**Solution**: Singleton shared manager

**Location**: `codex-rs/tui/src/app.rs:105-107`

```rust
pub(crate) struct App<'a> {
    chat_widgets: Vec<ChatWidget<'a>>,
    mcp_manager: Arc<tokio::sync::Mutex<Option<Arc<McpConnectionManager>>>>,
    // ↑ Shared singleton across all ChatWidgets
}

impl App {
    pub fn new() -> Self {
        // Initialize MCP manager once
        let mcp_manager = Arc::new(tokio::sync::Mutex::new(None));

        tokio::spawn({
            let mcp_manager = Arc::clone(&mcp_manager);
            let config = load_config();

            async move {
                let manager = McpConnectionManager::new(
                    config.mcp_servers,
                    HashSet::new(),
                ).await?;

                *mcp_manager.lock().await = Some(Arc::new(manager));
            }
        });

        Self {
            chat_widgets: Vec::new(),
            mcp_manager,
        }
    }
}
```

**Result**: Only M MCP server processes (one per configured server), regardless of widget count

---

## Performance: Native vs Subprocess

### Benchmark Results

**Subprocess approach** (pre-2025-10-18):
```
Tool invocation via local-memory MCP:
- Spawn process: ~20ms
- Execute tool: ~15ms
- Parse output: ~5ms
- Process cleanup: ~6ms
Total: ~46ms per tool call
```

**Native approach** (current):
```
Tool invocation via native client:
- Subprocess already running
- JSON-RPC roundtrip: ~7ms
- Parse response: ~1.7ms
Total: ~8.7ms per tool call
```

**Speedup**: **5.3× faster** (8.7ms vs 46ms)

**Additional benefits**:
- No process spawn overhead
- Persistent connection (reuse stdout/stdin)
- Lower memory footprint (shared subprocess)

---

## Server Lifecycle

### Initialization Sequence

```
1. Spawn subprocess
   └→ Command::new(program).spawn()

2. Send initialize request
   └→ {"method": "initialize", "params": {"protocolVersion": "1.0"}}

3. Receive initialize response
   └→ {"result": {"capabilities": {...}, "serverInfo": {...}}}

4. Send initialized notification
   └→ {"method": "notifications/initialized"}

5. List tools
   └→ {"method": "tools/list"}
   └→ {"result": {"tools": [...]}}

6. Ready for tool calls
```

---

### Shutdown Sequence

```
1. Send shutdown request
   └→ {"method": "shutdown"}

2. Receive shutdown response
   └→ {"result": null}

3. Send exit notification
   └→ {"method": "exit"}

4. Wait for process exit
   └→ child.wait().await

5. Cleanup (automatic via kill_on_drop)
```

---

### Health Monitoring

**Configurable timeouts**:
```toml
# ~/.code/config.toml

[mcp_servers.filesystem]
startup_timeout_sec = 10   # Server initialization timeout
tool_timeout_sec = 60      # Per-tool call timeout
```

**Timeout enforcement**:
```rust
// Startup timeout
let client = tokio::time::timeout(
    Duration::from_secs(config.startup_timeout_sec),
    McpClient::new_stdio_client(...)
).await??;

// Tool call timeout
let result = tokio::time::timeout(
    Duration::from_secs(config.tool_timeout_sec),
    client.call_tool(name, args)
).await??;
```

---

## Error Handling

### Error Types

```rust
pub enum McpError {
    // Connection errors
    ServerSpawnFailed(io::Error),
    ServerNotResponding,
    ServerCrashed(i32),         // Exit code

    // Protocol errors
    InvalidMessage(serde_json::Error),
    UnexpectedResponse,
    RequestTimeout,

    // Tool errors
    ToolNotFound(String),
    ToolExecutionFailed(String),
    InvalidArguments(String),
}
```

---

### Retry Logic

**Transient errors** (retry):
- Server not responding (may be overloaded)
- Request timeout (network issue)

**Permanent errors** (don't retry):
- Tool not found (invalid tool name)
- Invalid arguments (schema mismatch)
- Server crashed (exit code non-zero)

```rust
let result = execute_with_backoff(
    || client.call_tool(name, args),
    &RetryConfig::default(),
).await?;
```

---

### Graceful Degradation

**If MCP server fails**:
```rust
match mcp_manager.invoke_tool(name, args).await {
    Ok(result) => {
        // Tool executed successfully
        use_tool_result(result);
    },
    Err(McpError::ToolNotFound(_)) => {
        // Tool doesn't exist, inform model
        return "Tool not available. Please try another approach.";
    },
    Err(McpError::ServerCrashed(_)) => {
        // Server crashed, disable MCP for this conversation
        disable_mcp_for_conversation();
        return "MCP server unavailable. Continuing without tools.";
    },
    Err(e) => {
        // Other error, retry or fail
        return format!("Tool execution failed: {}", e);
    },
}
```

---

## Tool Schema Validation

### Input Schema

**From MCP server**:
```json
{
  "name": "read_file",
  "description": "Read contents of a file",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path to read"
      }
    },
    "required": ["path"]
  }
}
```

**Validation before invocation**:
```rust
pub fn validate_tool_arguments(
    tool_info: &ToolInfo,
    arguments: &Value,
) -> Result<()> {
    let schema = &tool_info.input_schema;

    // Use jsonschema crate for validation
    let compiled_schema = JSONSchema::compile(schema)?;

    compiled_schema.validate(arguments)
        .map_err(|errors| {
            let messages: Vec<_> = errors.map(|e| e.to_string()).collect();
            Error::InvalidArguments(messages.join(", "))
        })
}
```

---

## Configuration

### Per-Server Config

```toml
# ~/.code/config.toml

[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/project"]
env = { "NODE_ENV" = "production" }
startup_timeout_sec = 10
tool_timeout_sec = 60

[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-local-memory"]
startup_timeout_sec = 10
tool_timeout_sec = 30

[mcp_servers.git-status]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-git-status"]
```

---

### Excluded Tools

**Filter out specific tools**:
```toml
[mcp_servers.filesystem]
excluded_tools = ["write_file", "delete_file"]  # Read-only mode
```

**Programmatic exclusion**:
```rust
let excluded = HashSet::from([
    ("filesystem".to_string(), "write_file".to_string()),
    ("filesystem".to_string(), "delete_file".to_string()),
]);

let (manager, errors) = McpConnectionManager::new(
    config.mcp_servers,
    excluded,
).await?;
```

---

## Summary

**MCP Integration Highlights**:

1. **Native Client**: 5.3× faster than subprocess (8.7ms vs 46ms)
2. **Concurrent I/O**: Separate reader/writer tasks prevent deadlock
3. **Shared Manager**: App-level singleton prevents process multiplication
4. **Tool Qualification**: `server__tool_name` prevents collisions
5. **Timeout Enforcement**: Per-server startup and per-tool timeouts
6. **Error Handling**: Retry transient errors, fail fast on permanent
7. **Schema Validation**: Validate arguments before invocation
8. **Graceful Degradation**: Continue without MCP if servers fail

**Next Steps**:
- [Database Layer](database-layer.md) - SQLite optimization
- [Configuration System](configuration-system.md) - Hot-reload

---

**File References**:
- MCP client: `codex-rs/mcp-client/src/mcp_client.rs:63-150`
- Connection manager: `codex-rs/core/src/mcp_connection_manager.rs:84-150`
- JSON-RPC types: `codex-rs/mcp-types/src/jsonrpc.rs`
- App integration: `codex-rs/tui/src/app.rs:105-107`
