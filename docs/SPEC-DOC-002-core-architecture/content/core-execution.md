# Core Execution System

Agent orchestration, model providers, and execution management.

---

## Overview

The Core Execution system manages:
- **Agent Lifecycle**: Spawning, tracking, cleanup
- **Model Providers**: OpenAI, Anthropic, Google integration
- **Conversation Management**: Request/response handling
- **Retry Logic**: Exponential backoff for failures
- **Timeout Management**: Per-operation deadlines

**Location**: `codex-rs/core/src/`

---

## Agent Orchestration

### ConversationManager

**Purpose**: Central hub for agent conversation lifecycle

**Location**: `codex-rs/core/src/conversation_manager.rs`

```rust
pub struct ConversationManager {
    conversations: Arc<Mutex<HashMap<ConversationId, Conversation>>>,
    provider_clients: Arc<ModelProviderClients>,
    config: Arc<RwLock<Config>>,
}

impl ConversationManager {
    pub async fn new_conversation(&self, config: Config) -> Result<NewConversation> {
        let conversation = Conversation::new(
            config,
            self.provider_clients.clone(),
        ).await?;

        Ok(NewConversation { conversation, id })
    }
}
```

**Responsibilities**:
- Create new conversations
- Manage conversation lifecycle
- Coordinate with model providers
- Handle configuration updates

---

### Agent Spawning Pattern

**From TUI**: `codex-rs/tui/src/chatwidget/agent.rs:16-62`

```rust
pub(crate) fn spawn_agent(
    config: Config,
    app_event_tx: AppEventSender,
    server: Arc<ConversationManager>,
) -> UnboundedSender<Op> {
    let (codex_op_tx, mut codex_op_rx) = unbounded_channel::<Op>();

    tokio::spawn(async move {
        // Create conversation
        let conversation = server.new_conversation(config).await?;

        // Operation processor
        tokio::spawn(async move {
            while let Some(op) = codex_op_rx.recv().await {
                conversation.submit(op).await;
            }
        });

        // Event forwarder
        while let Ok(event) = conversation.next_event().await {
            app_event_tx.send(AppEvent::CodexEvent(event))?;
        }
    });

    codex_op_tx
}
```

**Key Points**:
- **Async task**: Runs on Tokio runtime
- **Channel-based**: UnboundedSender for sync → async bridge
- **Concurrent processing**: Separate op handler and event forwarder
- **Automatic cleanup**: Tasks exit when conversation ends

---

### Multi-Agent Orchestration (Spec-Kit)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

```rust
pub struct AgentOrchestrator {
    active_agents: HashMap<String, AgentHandle>,
    agent_configs: Vec<AgentConfig>,
}

pub struct AgentHandle {
    agent_id: String,
    agent_name: String,
    process: Child,                    // Subprocess handle
    stdout_reader: tokio::task::JoinHandle<()>,
    stderr_reader: tokio::task::JoinHandle<()>,
}

impl AgentOrchestrator {
    pub async fn spawn_agents(
        &mut self,
        spec_id: &str,
        stage: SpecStage,
        prompt: &str,
    ) -> Result<Vec<String>> {
        let mut agent_ids = Vec::new();

        for agent_config in &self.agent_configs {
            let agent_id = format!("{}-{}-{}", spec_id, stage, agent_config.name);

            // Spawn subprocess
            let mut child = Command::new(&agent_config.command)
                .args(&agent_config.args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            // Read stdout/stderr asynchronously
            let stdout_reader = tokio::spawn(read_stream(child.stdout.take()));
            let stderr_reader = tokio::spawn(read_stream(child.stderr.take()));

            let handle = AgentHandle {
                agent_id: agent_id.clone(),
                agent_name: agent_config.name.clone(),
                process: child,
                stdout_reader,
                stderr_reader,
            };

            self.active_agents.insert(agent_id.clone(), handle);
            agent_ids.push(agent_id);
        }

        Ok(agent_ids)
    }

    pub async fn wait_for_completion(&mut self, timeout: Duration) -> Result<Vec<AgentOutput>> {
        let deadline = Instant::now() + timeout;
        let mut outputs = Vec::new();

        for (agent_id, handle) in self.active_agents.drain() {
            let remaining = deadline.saturating_duration_since(Instant::now());

            match tokio::time::timeout(remaining, handle.process.wait()).await {
                Ok(Ok(status)) => {
                    let stdout = handle.stdout_reader.await?;
                    outputs.push(AgentOutput {
                        agent_id,
                        agent_name: handle.agent_name,
                        stdout,
                        exit_code: status.code(),
                    });
                },
                Ok(Err(e)) => { /* process error */ },
                Err(_) => { /* timeout */ },
            }
        }

        Ok(outputs)
    }
}
```

**Features**:
- **Concurrent spawning**: Launch multiple agents in parallel
- **Timeout enforcement**: Per-agent deadlines
- **Stream capture**: Async stdout/stderr reading
- **Graceful cleanup**: Kill on timeout or error

---

## Model Providers

### Provider Architecture

```
Application
    ↓
ModelProviderClients (registry)
    ├→ OpenAIProvider (Responses API)
    ├→ AnthropicProvider (CLI subprocess)
    └→ GoogleProvider (CLI subprocess)
```

---

### OpenAI Provider

**Location**: `codex-rs/protocol/src/openai_client.rs`

```rust
pub struct OpenAIClient {
    base_url: String,
    api_key: String,
    http_client: reqwest::Client,
    retry_config: RetryConfig,
}

impl OpenAIClient {
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        // Server-Sent Events stream
        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| parse_sse_event(event));

        Ok(stream)
    }

    pub async fn responses_api(
        &self,
        request: ResponsesRequest,
    ) -> Result<impl Stream<Item = Result<ResponseEvent>>> {
        // Similar but for Responses API
    }
}
```

**Features**:
- **Streaming**: Server-Sent Events (SSE)
- **Retry logic**: Exponential backoff on failures
- **Rate limiting**: 429 response handling
- **Timeout**: Per-request deadlines

---

### Anthropic Provider (CLI)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

```rust
// Anthropic via CLI subprocess
let mut child = Command::new("claude")
    .arg("--model").arg("claude-sonnet-4-5")
    .arg(prompt)
    .env("ANTHROPIC_API_KEY", api_key)
    .stdout(Stdio::piped())
    .spawn()?;

let output = child.wait_with_output().await?;
let response = String::from_utf8(output.stdout)?;
```

**Note**: Uses CLI subprocess, not direct API (simpler integration for multi-agent)

---

### Google Provider (CLI)

```rust
// Google via CLI subprocess
let mut child = Command::new("gemini")
    .arg("-i").arg(prompt)
    .env("GOOGLE_API_KEY", api_key)
    .stdout(Stdio::piped())
    .spawn()?;

let output = child.wait_with_output().await?;
let response = String::from_utf8(output.stdout)?;
```

---

## Protocol Implementation

### Request/Response Types

**Location**: `codex-rs/protocol/src/types.rs`

```rust
// Chat Completions API
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

pub struct Message {
    pub role: Role,      // system, user, assistant, tool
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

// Responses API (newer)
pub struct ResponsesRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub reasoning: Option<ReasoningConfig>,
    pub response_format: Option<ResponseFormat>,
}

pub struct ReasoningConfig {
    pub effort: ReasoningEffort,  // minimal, low, medium, high
    pub summary: SummaryLevel,    // none, auto, concise, detailed
}
```

---

### Streaming Events

```rust
pub enum ResponseEvent {
    Start { id: String },
    Token { content: String },
    ReasoningToken { content: String },
    ToolCall { call: ToolCall },
    Complete { finish_reason: FinishReason },
    Error { error: String },
}
```

**Processing**:
```rust
while let Some(event) = stream.next().await {
    match event? {
        ResponseEvent::Token { content } => {
            // Forward to UI for rendering
            app_event_tx.send(AppEvent::Token(content))?;
        },
        ResponseEvent::ToolCall { call } => {
            // Execute tool and inject result
            let result = execute_tool(call).await?;
            conversation.submit(Op::ToolResponse(call.id, result)).await?;
        },
        ResponseEvent::Complete { finish_reason } => {
            break;
        },
        _ => {}
    }
}
```

---

## Retry Logic

### Exponential Backoff

**Location**: `codex-rs/spec-kit/src/retry/strategy.rs`

```rust
pub struct RetryConfig {
    pub max_attempts: usize,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10_000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.5,
        }
    }
}
```

**Backoff Calculation**:
```
Attempt 1: 100ms + jitter(±50ms)
Attempt 2: 200ms + jitter(±100ms)
Attempt 3: 400ms + jitter(±200ms)
Attempt 4: 800ms + jitter(±400ms)
Attempt 5: 1600ms + jitter(±800ms)
...
Max: 10,000ms (10s)
```

---

### Error Classification

```rust
pub trait RetryClassifiable {
    fn is_retryable(&self) -> bool;
}

impl RetryClassifiable for ApiError {
    fn is_retryable(&self) -> bool {
        match self {
            // Transient errors (retry)
            ApiError::RateLimitExceeded => true,
            ApiError::ServiceUnavailable => true,
            ApiError::Timeout => true,
            ApiError::NetworkError(_) => true,

            // Permanent errors (don't retry)
            ApiError::AuthenticationFailed => false,
            ApiError::InvalidRequest(_) => false,
            ApiError::InsufficientQuota => false,

            _ => false,
        }
    }
}
```

---

### Retry Execution

```rust
pub async fn execute_with_backoff<F, Fut, T, E>(
    mut operation: F,
    config: &RetryConfig,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Error + RetryClassifiable,
{
    let mut attempts = 0;
    let mut backoff_ms = config.initial_backoff_ms;

    loop {
        attempts += 1;

        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) if !err.is_retryable() => {
                return Err(RetryError::PermanentError(err.to_string()));
            },
            Err(_) if attempts >= config.max_attempts => {
                return Err(RetryError::MaxAttemptsExceeded(attempts));
            },
            Err(_) => {
                // Calculate jittered backoff
                let jitter = (rand::random::<f64>() - 0.5) * 2.0 * config.jitter_factor;
                let delay_ms = (backoff_ms as f64 * (1.0 + jitter)) as u64;

                tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                // Exponential increase
                backoff_ms = (backoff_ms as f64 * config.backoff_multiplier) as u64;
                backoff_ms = backoff_ms.min(config.max_backoff_ms);
            }
        }
    }
}
```

---

## Timeout Management

### Per-Operation Timeouts

```rust
pub async fn execute_with_timeout<F, T>(
    operation: F,
    timeout: Duration,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match tokio::time::timeout(timeout, operation).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(Error::Timeout),
    }
}
```

**Usage**:
```rust
// API request with 30s timeout
let response = execute_with_timeout(
    openai_client.chat_completion(request),
    Duration::from_secs(30),
).await?;

// Agent execution with 5min timeout
let outputs = execute_with_timeout(
    agent_orchestrator.wait_for_completion(),
    Duration::from_secs(300),
).await?;
```

---

### Configurable Timeouts

**From config.toml**:
```toml
[mcp_servers.filesystem]
startup_timeout_sec = 10   # Server initialization
tool_timeout_sec = 60      # Per-tool execution

[model_providers.openai]
stream_idle_timeout_ms = 300000  # 5min idle timeout
```

---

## Tool Execution

### Tool Call Flow

```
1. Model returns tool_use event
   └→ {"type": "tool_use", "name": "filesystem__read_file", "arguments": {...}}

2. Conversation extracts tool call
   └→ ToolCall { id, name, arguments }

3. Submit to MCP manager
   └→ mcp_manager.invoke_tool(name, arguments).await

4. MCP client executes
   └→ Server subprocess processes request

5. Result returned
   └→ ToolResult { id, content: "file contents..." }

6. Inject into conversation
   └→ conversation.submit(Op::ToolResponse(id, result)).await

7. Model continues with tool result
```

---

### Sandbox Enforcement

```rust
pub enum SandboxMode {
    ReadOnly,           // No writes, no network
    WorkspaceWrite,     // Write to workspace, no network
    DangerFullAccess,   // Full access (use in Docker)
}

pub fn execute_sandboxed(
    command: &str,
    args: &[&str],
    sandbox: SandboxMode,
) -> Result<Output> {
    match sandbox {
        SandboxMode::ReadOnly => {
            // Landlock: deny all writes
            apply_landlock_readonly()?;
        },
        SandboxMode::WorkspaceWrite => {
            // Landlock: allow workspace writes only
            apply_landlock_workspace(workspace_path)?;
        },
        SandboxMode::DangerFullAccess => {
            // No sandboxing
        },
    }

    // Execute command
    Command::new(command)
        .args(args)
        .output()
}
```

**File**: `codex-rs/linux-sandbox/src/lib.rs`

---

## Performance Optimizations

### Connection Pooling

**HTTP Client**:
```rust
let http_client = reqwest::Client::builder()
    .pool_max_idle_per_host(10)     // Reuse connections
    .timeout(Duration::from_secs(30))
    .build()?;
```

**Benefits**:
- Reuse TCP connections (avoid handshake overhead)
- Reduce latency for subsequent requests

---

### Concurrent Execution

**Multi-Agent**:
```rust
// Spawn all agents concurrently
let mut join_set = JoinSet::new();
for agent_config in agent_configs {
    join_set.spawn(spawn_agent(agent_config, prompt.clone()));
}

// Wait for all to complete
let mut outputs = Vec::new();
while let Some(result) = join_set.join_next().await {
    outputs.push(result??);
}
```

**Performance**: 3 agents finish in ~10s instead of ~30s (3× speedup)

---

## Error Handling

### Error Hierarchy

```rust
pub enum Error {
    // Network errors (retryable)
    NetworkError(reqwest::Error),
    Timeout,

    // API errors (some retryable)
    RateLimitExceeded,
    ServiceUnavailable,
    AuthenticationFailed,      // Not retryable
    InvalidRequest(String),    // Not retryable

    // Agent errors
    AgentSpawnFailed(io::Error),
    AgentTimeout,
    AgentCrashed(i32),         // Exit code

    // Tool errors
    ToolNotFound(String),
    ToolExecutionFailed(String),
}
```

---

### Graceful Degradation

**Multi-Agent Consensus**:
```rust
// If 1/3 agents fail, continue with 2/3
let outputs = agent_orchestrator.wait_for_completion().await?;

if outputs.len() >= 2 {
    // Consensus still valid with 2/3 agents
    let synthesis = synthesize_consensus(&outputs);
    return Ok((synthesis, degraded: true));
} else {
    return Err(Error::InsufficientAgents);
}
```

---

## Summary

**Core Execution Highlights**:

1. **Agent Orchestration**: Async task spawning with channel-based communication
2. **Multi-Provider**: OpenAI (HTTP), Anthropic/Google (CLI subprocess)
3. **Streaming**: Real-time token delivery via SSE
4. **Retry Logic**: Exponential backoff with jitter (100ms → 10s)
5. **Timeout Management**: Per-operation deadlines
6. **Tool Execution**: MCP integration with sandbox enforcement
7. **Error Handling**: Permanent vs transient classification
8. **Performance**: Connection pooling, concurrent agent execution

**Next Steps**:
- [MCP Integration](mcp-integration.md) - Native client details
- [Database Layer](database-layer.md) - SQLite optimization
- [Configuration System](configuration-system.md) - Hot-reload

---

**File References**:
- Conversation manager: `codex-rs/core/src/conversation_manager.rs`
- Agent spawner: `codex-rs/tui/src/chatwidget/agent.rs:16-62`
- Agent orchestrator: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
- OpenAI client: `codex-rs/protocol/src/openai_client.rs`
- Retry logic: `codex-rs/spec-kit/src/retry/strategy.rs`
- Sandbox: `codex-rs/linux-sandbox/src/lib.rs`
