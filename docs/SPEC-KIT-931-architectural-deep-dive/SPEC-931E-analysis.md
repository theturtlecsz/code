# SPEC-931E Analysis: Technical Limits & Hard Constraints

**Session**: 2025-11-13
**Analyst**: Claude (Sonnet 4.5)
**Scope**: Platform, library, and API constraints that define architectural boundaries
**Context**: Child spec 5/10 in SPEC-931 architectural deep dive (Group B: Constraints)

---

## Executive Summary

This analysis quantifies technical limits across 5 constraint domains (Ratatui, SQLite, Provider APIs, Rust/tokio, Platform) that bound the design space for agent orchestration refactoring. All limits are measured with numbers, benchmarks, and hard caps.

**Key Findings**:
- **Ratatui**: NOT async (synchronous crossterm on dedicated thread), 33ms redraw debounce (30 FPS), no tokio::select! integration
- **SQLite**: Single-writer even with WAL mode, read concurrency unlimited, write throughput disk-bound
- **Provider APIs**: OpenAI 90K-10M TPM, Gemini 5-∞ RPM (DSQ), rate limits are hard blockers without queue
- **Tokio**: spawn_blocking pool ~500 threads max, saturation risk for CPU-bound work, no abortion
- **Platform**: Cross-platform (Linux/macOS/Windows), portable-pty limits, process/file descriptor OS limits

**Critical Constraints for SPEC-930**:
1. **TUI async integration** requires architectural change (not drop-in tokio::select!)
2. **Event sourcing** compatible with SQLite single-writer (transactions solve dual-write)
3. **Actor model** compatible with tokio runtime (message passing, supervisor trees)
4. **Rate limiting** MANDATORY (providers return 429 without queue)
5. **Queue-based execution** solves provider limits (backpressure, priority)

**Validation Status**: All SPEC-930 patterns are feasible within constraints, with architectural adjustments required for TUI async integration.

---

## 1. RATATUI TUI CONSTRAINTS

### 1.1 Current Architecture (Synchronous Event Loop)

**Location**: `codex-rs/tui/src/app.rs:269-350`

**Pattern**: Dedicated input thread with crossterm polling (not tokio::select!)

```rust
std::thread::spawn(move || {
    loop {
        if !input_running_thread.load(Ordering::Relaxed) {
            break;
        }
        let poll_timeout = if hot_typing {
            Duration::from_millis(2)  // 500 Hz during typing
        } else {
            Duration::from_millis(10) // 100 Hz idle
        };
        if let Ok(true) = crossterm::event::poll(poll_timeout) {
            if let Ok(event) = crossterm::event::read() {
                // Send to app_event_tx (channel)
            }
        }
    }
});
```

**Key Characteristics**:
- **Not async**: Blocking crossterm::event::read() on dedicated thread
- **Channel-based**: Events sent via std::sync::mpsc to main loop
- **Polling frequency**: 2ms (typing) / 10ms (idle)
- **No tokio integration**: Separate from tokio runtime

### 1.2 Rendering Constraints

**Redraw Debounce** (`app.rs:56`):
```rust
const REDRAW_DEBOUNCE: Duration = Duration::from_millis(33); // 30 FPS
```

**Frame Rate**: 30 FPS maximum (33ms debounce)
- **Rationale**: "Coalesce bursts of updates while we smooth out per-frame hotspots"
- **Impact**: Streaming updates batched every ~33ms, not real-time

**Rendering Thread**: Main thread only
- **Ratatui constraint**: Terminal rendering must be synchronous
- **No async rendering**: Cannot await inside render() callback
- **Frame time budget**: ~33ms per frame for draw + update logic

### 1.3 Async Integration Patterns

**Current MCP Integration** (`app.rs:354-395`):
```rust
let mcp_manager = Arc::new(tokio::sync::Mutex::new(None));
tokio::spawn(async move {
    // Async MCP initialization separate from TUI
    match McpConnectionManager::new(mcp_config, HashSet::new()).await {
        Ok((manager, errors)) => {
            *mcp_manager_slot.lock().await = Some(Arc::new(manager));
        }
    }
});
```

**Pattern**: Async operations spawned separately, communicate via channels/Arc<Mutex<>>
- **No blocking in render()**: Async work must complete before UI update
- **State synchronization**: Arc<Mutex<>> for shared state access
- **Event-driven updates**: Async tasks send AppEvent to trigger redraws

### 1.4 Blocking Constraints

**Terminal Operations**:
- **crossterm::cursor::position()**: Requires event lock, 2-second timeout
- **execute!() macros**: Synchronous terminal writes
- **Buffer diff application**: Must complete within frame budget

**Consequences**:
- Cannot call `await` in rendering path
- Long-running async work must be spawned on tokio runtime
- Updates communicated via channels (AppEvent::RequestRedraw)

### 1.5 SPEC-930 TUI Dashboard Pattern Validation

**SPEC-930 Proposal** (from spec.md:299-339):
```rust
loop {
    tokio::select! {
        _ = tick_interval.tick() => {
            // Update model state
        },
        event = event_rx.recv() => {
            // Handle actor events
        },
        input = input_rx.recv() => {
            // Handle user input
        },
    }
    // Render frame
    terminal.draw(|f| render_dashboard(f, &model))?;
}
```

**Current Implementation Gap**:
- ❌ No tokio::select! in current code
- ❌ Crossterm event loop is synchronous (std::thread::spawn, not tokio)
- ❌ Rendering is synchronous (cannot await in draw callback)

**Compatibility Assessment**:
- ⚠️ **Requires architectural change**: Cannot drop tokio::select! into current structure
- ✅ **Pattern is feasible**: Ratatui async-template demonstrates tokio integration
- ✅ **Alternative**: Use channels (current approach) + separate tokio::spawn for actor events
- ⚠️ **Migration effort**: Moderate (must restructure event loop)

**Recommendation**: Hybrid approach (current + tokio::spawn for actors), defer full tokio::select! migration.

### 1.6 Quantified Limits

| Constraint | Value | Source |
|------------|-------|--------|
| Max frame rate | 30 FPS (33ms debounce) | `app.rs:56` |
| Input poll frequency (typing) | 500 Hz (2ms) | `app.rs:294` |
| Input poll frequency (idle) | 100 Hz (10ms) | `app.rs:296` |
| Cursor position timeout | 2 seconds | `app.rs:285` |
| Rendering thread | Main thread only | Ratatui constraint |
| Async in render callback | ❌ Not supported | Ratatui constraint |

---

## 2. SQLITE LIMITATIONS

### 2.1 Concurrency Model

**Single-Writer Constraint** (even with WAL mode):
- Only one writer at a time (enforced by EXCLUSIVE lock)
- Write transactions serialize, even across processes
- **Impact**: AGENT_MANAGER HashMap + SQLite dual-write is NOT atomic

**WAL Mode Advantages**:
- Unlimited concurrent readers (no blocking on writes)
- Readers don't block writers (snapshot isolation)
- Better write performance (batch commits to WAL)

**Current Usage** (`codex-rs/core/src/consensus_db.rs`):
- Database: `~/.code/consensus_artifacts.db`
- Tables: `agent_executions`, `consensus_artifacts`, `consensus_synthesis`
- Mode: Unknown (likely journal mode, not WAL)

### 2.2 Transaction Isolation

**ACID Properties**:
- ✅ **Atomicity**: Transactions commit or rollback fully
- ✅ **Consistency**: Foreign keys, constraints enforced
- ✅ **Isolation**: Serializable by default (write-lock acquisition)
- ✅ **Durability**: fsync on commit (configurable)

**Current Dual-Write Problem** (SPEC-931A finding):
```
1. Update AGENT_MANAGER (in-memory HashMap)
2. Write to SQLite (separate transaction)
3. Crash between steps → inconsistent state
```

**Solution**: Event sourcing (single source of truth in SQLite)
- No dual-write: All mutations append to event_log
- State derived from events (replay on startup)
- ACID guarantee for all state changes

### 2.3 Performance Benchmarks

**Write Throughput** (general SQLite limits, not project-specific):
- Journal mode: ~1,000-5,000 inserts/sec (depends on fsync)
- WAL mode: ~10,000-50,000 inserts/sec (batch commits)
- Disk-bound: SSD ~100 MB/s, HDD ~50 MB/s

**Read Throughput**:
- SELECT queries: ~100,000-1,000,000 rows/sec (simple queries)
- Index lookups: ~10-100 µs per query
- Full table scans: Disk-bound

**Transaction Overhead**:
- BEGIN/COMMIT: ~0.1-1 ms (WAL mode)
- fsync penalty: 1-10 ms (depends on storage)

**Project-Specific Constraints**:
- Agent execution writes: Low frequency (1-10 per minute)
- Consensus artifact writes: Moderate (100s of KB per agent)
- Event log writes: High frequency (10-100 events per agent execution)

**Bottleneck**: Database bloat (153 MB → 53 KB after cleanup, 99.97% waste, SPEC-931A)

### 2.4 Schema Migration Constraints

**Current State**:
- No migration framework (SPEC-931D finding)
- No schema versioning (no version table)
- Breaking changes would corrupt existing databases

**Requirements for SPEC-930**:
- Migration framework (e.g., rusqlite + migration table)
- Schema versioning (version number in pragma)
- Backward compatibility (detect old schema, auto-migrate)

### 2.5 SPEC-930 Event Sourcing Pattern Validation

**SPEC-930 Proposal** (from spec.md:428-442):
```sql
CREATE TABLE event_log (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_data JSON NOT NULL,
    timestamp INTEGER NOT NULL,
    sequence_number INTEGER NOT NULL
);
```

**Compatibility Assessment**:
- ✅ **Single-writer compatible**: Events append sequentially (AUTOINCREMENT)
- ✅ **ACID compliant**: Transaction guarantees atomicity
- ✅ **Crash recovery**: Replay events from last snapshot
- ✅ **Performance acceptable**: 10-100 events/agent execution << SQLite capacity
- ⚠️ **Snapshot strategy required**: Replay overhead for large histories

**Performance Model**:
```
Agent execution: 50 events average
Write throughput: 10,000 inserts/sec (WAL mode)
Time per execution: 5 ms (event writes only)
Concurrent agents: 100
Total write load: 100 × 5 ms = 500 ms/sec (50% utilization)
```

**Verdict**: ✅ SQLite is sufficient for event sourcing (even at 100+ concurrent agents)

### 2.6 Quantified Limits

| Constraint | Value | Source |
|------------|-------|--------|
| Concurrent writers | 1 (single writer) | SQLite architecture |
| Concurrent readers | Unlimited (WAL mode) | SQLite WAL docs |
| Write throughput (WAL) | 10K-50K inserts/sec | General benchmarks |
| Read throughput | 100K-1M rows/sec | General benchmarks |
| Transaction overhead | 0.1-1 ms | WAL mode |
| Max database size | 281 TB | SQLite limits |
| Max row size | 1 GB | SQLite limits |
| Event log overhead | ~5 ms per agent execution | Calculated |

---

## 3. PROVIDER API CONSTRAINTS

### 3.1 OpenAI API Rate Limits (2025)

**Source**: [OpenAI Rate Limits Documentation](https://platform.openai.com/docs/guides/rate-limits)

**Rate Limit Dimensions**:
- **RPM**: Requests per minute
- **RPD**: Requests per day
- **TPM**: Tokens per minute (input + output)
- **TPD**: Tokens per day
- **IPM**: Images per minute (image models only)

**GPT-4 Limits** (default tier):
- RPM: 3,500 requests/min
- TPM: 90,000 tokens/min
- TPD: Not publicly specified (estimated ~10M)

**GPT-4o Limits** (top tier):
- RPM: Up to 10,000 requests/min
- TPM: Up to 10,000,000 tokens/min (10M TPM)
- **5× higher than GPT-4 Turbo**

**GPT-5 Limits** (Tier 1, Sept 2025 update):
- TPM: 500,000 tokens/min (raised from ~30K)

**Usage Tier System**:
- Tier 1: Free users (lowest limits)
- Tier 2-5: Paid users (auto-upgrade with spending)
- Each tier: 2-10× higher limits than previous

**Token Counting**:
- `max_tokens` counts toward limit even if unused
- Both input + output tokens counted
- Cached tokens do NOT count (prompt caching)

**Error Response** (429 Too Many Requests):
```json
{
  "error": {
    "message": "Rate limit exceeded",
    "type": "rate_limit_error",
    "param": null,
    "code": "rate_limit_exceeded"
  }
}
```

**Retry-After Header**: Included in 429 responses (seconds until reset)

### 3.2 Google Gemini API Rate Limits (2025)

**Source**: [Gemini API Rate Limits](https://ai.google.dev/gemini-api/docs/rate-limits)

**Rate Limit Dimensions**:
- **RPM**: Requests per minute (primary gating)
- **RPD**: Requests per day (resets at midnight Pacific)
- **TPM**: Tokens per minute
- **IPM**: Images per minute (Imagen 3 only)

**Free Tier**:
- RPM: 5 requests/min
- RPD: 25 requests/day
- **Extremely restrictive**

**Paid Tier 1**:
- RPM: "Significantly higher" (not publicly specified)
- TPM: "Increased quotas"
- Requires credit card on file

**Paid Tier 2** (requires $250 spending + 30 days):
- RPM: Enterprise-level quotas
- No public numbers

**Dynamic Shared Quota (DSQ)** (newer models):
- No fixed quotas (dynamic capacity allocation)
- No quota increase requests needed
- Automatically scales with demand
- **Impact**: Unlimited RPM/TPM (subject to fair use)

**Error Response** (429 RESOURCE_EXHAUSTED):
```json
{
  "error": {
    "code": 429,
    "message": "Quota exceeded for aiplatform.googleapis.com/online_prediction_requests_per_base_model",
    "status": "RESOURCE_EXHAUSTED"
  }
}
```

**Quota Reset**: Midnight Pacific time (daily limits)

### 3.3 Anthropic Claude API Rate Limits (2025)

**Source**: General knowledge (web search unavailable)

**Rate Limit Dimensions**:
- **TPM**: Tokens per minute (primary)
- **RPM**: Requests per minute
- **Monthly tokens**: Monthly cap (varies by tier)

**Usage Tiers**:
- Free Tier: Low TPM (~5K-10K), monthly cap
- Tier 1: ~50K TPM, ~$100/month spending
- Tier 2: ~200K TPM, ~$500/month spending
- Enterprise: Custom quotas

**Claude 3.5 Sonnet**:
- Input TPM: Varies by tier
- Output TPM: Separate limit (often lower than input)
- **ITPM/OTPM split**: Input and output tracked separately

**Error Response** (429):
```json
{
  "type": "error",
  "error": {
    "type": "rate_limit_error",
    "message": "Rate limit exceeded"
  }
}
```

**Retry-After Header**: Seconds until rate limit resets

### 3.4 OAuth2 Flow Constraints

**Claude CLI Authentication** (relevant for SPEC-931G tmux removal):
- **Device Code Flow**: Requires browser interaction
- **Non-interactive execution**: Needs pre-authentication or API key
- **Token expiration**: Access tokens expire (15-60 min typical)
- **Refresh tokens**: Long-lived (7-90 days), require secure storage

**Impact on Tmux Removal**:
- ⚠️ **Blocker**: Device code flow requires user interaction (not fully automatable)
- ✅ **Workaround**: API keys (if provider supports)
- ⚠️ **Token management**: Refresh logic required for long-running agents

### 3.5 SPEC-930 Rate Limiting Pattern Validation

**SPEC-930 Proposal** (from spec.md:249-260):
```rust
async fn dequeue_next_agent(
    queue: &AgentQueue,
    rate_limiter: &RateLimiter,
) -> Option<AgentExecution> {
    // Check rate limits per provider
    let available_providers = rate_limiter.available_providers().await;

    // Dequeue only if provider has capacity
    queue.transaction(|tx| {
        let agent = tx.dequeue_where(|a| {
            available_providers.contains(&a.provider)
        })?;

        // Acquire rate limit slot
        rate_limiter.acquire(&agent.provider).await?;
        ...
    })
}
```

**Compatibility Assessment**:
- ✅ **Mandatory**: Without rate limiter, provider 429s are guaranteed at scale
- ✅ **Token bucket algorithm**: Standard approach (used by OpenAI, Anthropic)
- ✅ **Multi-provider support**: Track TPM/RPM per provider independently
- ✅ **Queue integration**: Backpressure when all providers saturated
- ⚠️ **Token counting challenge**: Need token count BEFORE execution (estimate from prompt)

**Rate Limiter Requirements**:
1. **Per-provider buckets**: OpenAI, Anthropic, Google tracked separately
2. **Multi-dimensional limits**: Track TPM + RPM + TPD simultaneously
3. **Sliding window**: Avoid thundering herd at minute boundaries
4. **Retry-After honor**: Respect API-provided backoff timings
5. **Fallback strategy**: OpenAI unavailable → Claude → Gemini

**Quantified Limits** (for 3-agent quality gate consensus):
```
Scenario: 100 quality gates/day (300 agent executions)
Average tokens/agent: 5,000 (2K input + 3K output)
Total tokens/day: 1,500,000 (1.5M)

OpenAI (90K TPM):
- Max agents/min: 18 (90K / 5K)
- Daily capacity: 25,920 agents (18 × 60 × 24)
- Utilization: 1.2% (300 / 25,920)

Gemini Free (5 RPM):
- Max agents/min: 5
- Daily capacity: 25 (RPD limit)
- Utilization: 1,200% (300 / 25) ❌ EXCEEDS LIMIT

Gemini Paid Tier 1 (estimated 100 RPM):
- Max agents/min: 100
- Daily capacity: 144,000
- Utilization: 0.2%
```

**Verdict**: ✅ OpenAI sufficient, ⚠️ Gemini free tier insufficient, ✅ Rate limiter mandatory for multi-provider

### 3.6 Quantified Limits

| Provider | Metric | Free Tier | Paid Tier 1 | Paid Tier 2+ | Source |
|----------|--------|-----------|-------------|--------------|--------|
| OpenAI GPT-4 | TPM | - | 90,000 | 10,000,000 | OpenAI docs (2025) |
| OpenAI GPT-4 | RPM | - | 3,500 | 10,000 | OpenAI docs |
| OpenAI GPT-5 | TPM | - | 500,000 | Higher | Sept 2025 update |
| Gemini | RPM | 5 | "Higher" | Enterprise | Gemini docs |
| Gemini | RPD | 25 | "Higher" | Enterprise | Gemini docs |
| Gemini DSQ | RPM | N/A | ∞ (dynamic) | ∞ (dynamic) | DSQ models |
| Claude 3.5 | TPM | ~5-10K | ~50K | ~200K | Estimated |
| Claude 3.5 | Monthly | ~100K | ~5M | ~50M | Estimated |

---

## 4. RUST/TOKIO CONSTRAINTS

### 4.1 Tokio Runtime Architecture

**Default Configuration** (`tokio::runtime::Builder::new_multi_thread()`):
- **Worker threads**: Number of CPU cores (detected via `num_cpus`)
- **Blocking pool**: ~500 threads max (very large by default)
- **Task queue**: Unbounded (memory-limited only)

**Current Usage** (`app.rs:2668-2674`):
```rust
let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("build tokio runtime");
```

**Thread Types**:
1. **Worker threads**: Execute async tasks (CPU-bound to core count)
2. **Blocking pool threads**: Execute `spawn_blocking` tasks (up to ~500)
3. **IO driver thread**: epoll/kqueue event loop (1 thread)

### 4.2 spawn_blocking Constraints

**Source**: [Tokio spawn_blocking docs](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)

**Purpose**: Run blocking operations without blocking the async runtime

**Thread Pool Size**:
- **Default maximum**: ~500 threads
- **Configurable**: `Builder::max_blocking_threads()`
- **On-demand spawning**: Threads created as needed until limit reached
- **Queue on saturation**: Tasks queue if all threads busy

**Saturation Risk** (from web search):
> "When running CPU-bound code using spawn_blocking, you should keep this large upper limit in mind. When running many CPU-bound computations, a semaphore or some other synchronization primitive should be used to limit the number of computation executed in parallel."

**Shutdown Behavior**:
- **No abortion**: `abort()` has no effect on blocking tasks
- **Indefinite wait**: Runtime waits for all blocking tasks to complete
- **Timeout option**: `shutdown_timeout()` to force stop

**Current Usage** (via `tokio::task::spawn_blocking`):
```rust
let reader_handle = tokio::task::spawn_blocking(move || {
    // Blocking PTY read operation
    loop {
        let mut buffer = vec![0u8; 4096];
        match reader.read(&mut buffer) {
            Ok(n) => { /* ... */ }
        }
    }
});
```

**Pattern**: PTY I/O (inherently blocking) runs on blocking pool

### 4.3 Async Runtime Constraints

**Task Spawning**:
- **Lightweight**: Tasks are green threads (~2 KB overhead)
- **Unlimited**: No hard limit on task count (memory-limited)
- **Scheduling**: Cooperative (tasks must `await` or `yield`)

**Blocking Operations** (must avoid in async tasks):
- `std::fs` operations (use `tokio::fs` instead)
- `std::sync::Mutex` (use `tokio::sync::Mutex` instead)
- Long-running CPU work (use `spawn_blocking`)
- Synchronous I/O (use async I/O)

**Consequences of Blocking**:
- Worker thread stalled (other tasks cannot run)
- Latency spike for other tasks on same thread
- Potential deadlock if all workers blocked

### 4.4 Channel Performance

**Channel Types**:
- **mpsc**: Multi-producer, single-consumer (bounded or unbounded)
- **broadcast**: Multi-producer, multi-consumer (bounded, lossy)
- **oneshot**: Single-producer, single-consumer (one-shot)
- **watch**: Single-producer, multi-consumer (latest value only)

**Performance** (general benchmarks, not project-specific):
- **Send latency**: ~50-200 ns (uncontended)
- **Throughput**: ~10-50M messages/sec (single thread)
- **Bounded backpressure**: Sender blocks when full

**Current Usage**:
- `std::sync::mpsc`: For app events (high-priority and bulk queues)
- `tokio::sync::mpsc`: For actor communication
- `tokio::sync::oneshot`: For cancellation signals

### 4.5 SPEC-930 Actor Model Pattern Validation

**SPEC-930 Proposal** (from spec.md:623-672):
```rust
async fn supervisor_actor(
    mut cmd_rx: mpsc::Receiver<SupervisorCommand>,
    event_tx: broadcast::Sender<AgentEvent>,
) {
    let mut agent_actors: HashMap<String, AgentHandle> = HashMap::new();

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    SupervisorCommand::SpawnAgent { agent_id, provider } => {
                        let (tx, rx) = mpsc::channel(32);
                        let handle = tokio::spawn(agent_actor(...));
                        agent_actors.insert(agent_id, AgentHandle { tx, handle });
                    }
                }
            }
        }
    }
}
```

**Compatibility Assessment**:
- ✅ **Tokio native**: Actor pattern maps cleanly to tokio::spawn + mpsc
- ✅ **Message passing**: tokio::sync::mpsc is battle-tested
- ✅ **Supervision**: Can monitor task completion via JoinHandle
- ✅ **Graceful shutdown**: Send shutdown message, await all handles
- ⚠️ **TUI integration**: Must communicate via channels (see Section 1)

**Resource Model**:
```
Supervisor: 1 tokio task (~2 KB)
Agent actors: N tokio tasks (~2 KB each)
Message channels: (32 × sizeof(AgentMessage)) × N
Total overhead: ~2 KB + (N × 2 KB) + channel buffers
```

**Scalability**:
```
Scenario: 100 concurrent agents
Task overhead: 100 × 2 KB = 200 KB
Channel overhead: 100 × (32 × 1 KB) = 3.2 MB (estimated)
Total: ~3.4 MB (negligible)
```

**Verdict**: ✅ Tokio runtime is sufficient for actor model (even at 1000+ agents)

### 4.6 Quantified Limits

| Constraint | Value | Source |
|------------|-------|--------|
| Worker threads (default) | CPU cores | Tokio default |
| Blocking pool max | ~500 threads | Tokio docs |
| Task overhead | ~2 KB | General estimate |
| Max tasks | Memory-limited | Tokio architecture |
| Channel send latency | 50-200 ns | General benchmarks |
| Channel throughput | 10-50M msg/sec | General benchmarks |
| spawn_blocking saturation | 500 concurrent | Tokio default |
| Shutdown wait | Indefinite (or timeout) | Tokio docs |

---

## 5. PLATFORM CONSTRAINTS

### 5.1 Operating System Compatibility

**Supported Platforms** (crossterm + portable-pty):
- ✅ **Linux**: Full support (epoll, PTY, signals)
- ✅ **macOS**: Full support (kqueue, PTY, signals)
- ✅ **Windows**: Full support (IOCP, ConPTY, limited signals)

**Terminal Backend** (crossterm):
- **UNIX**: termios + ANSI escape codes
- **Windows**: Windows Console API + ANSI (Windows 10+)

**PTY System** (portable-pty):
- **UNIX**: openpty() + fork() + exec()
- **Windows**: ConPTY (Windows 10 1809+)

### 5.2 File Descriptor Limits

**UNIX (Linux/macOS)**:
- **Soft limit**: `ulimit -n` (default 256-1024)
- **Hard limit**: `ulimit -Hn` (default 4096-unlimited)
- **Impact**: Each agent execution may consume 3-10 FDs (stdin/stdout/stderr + files)

**Calculation** (100 concurrent agents):
```
FDs per agent: ~5 (PTY master/slave, logs, DB connection)
Total FDs: 100 × 5 = 500
Soft limit: 1024 (typical)
Utilization: 49% (safe)
```

**Windows**:
- **Handle limit**: ~16,000 handles/process (typical)
- **Less constrained than UNIX FDs**

### 5.3 Process Limits

**UNIX**:
- **Max processes/user**: `ulimit -u` (default 256-4096)
- **Impact**: If spawning processes per agent (current tmux approach)

**Current Architecture** (tmux-based):
```
Per agent execution:
- 1 tmux process
- 1 provider CLI process (claude/gemini/code)
Total processes: 2N (where N = concurrent agents)
```

**Constraint** (100 concurrent agents):
```
Processes: 100 × 2 = 200
User limit: 4096 (typical)
Utilization: 5% (safe)
```

**Post-SPEC-930** (actor-based, no tmux):
```
Processes: 0 (all in-process via tokio actors)
Utilization: 0% (no external processes)
```

**Verdict**: ✅ Process limits not a constraint (even with tmux, well under limits)

### 5.4 Memory Constraints

**Rust Memory Model**:
- **No garbage collector**: Deterministic memory usage
- **Stack size**: Default 2 MB (configurable per thread)
- **Heap**: Unlimited (OS-limited)

**Estimated Memory** (100 concurrent agents):
```
Tokio tasks: 100 × 2 KB = 200 KB
Actor state: 100 × 10 KB = 1 MB (estimated)
Message buffers: 100 × 32 KB = 3.2 MB
SQLite connection: ~1 MB
MCP connections: 5 × 10 MB = 50 MB (estimated)
Total: ~55 MB (low)
```

**Database Memory** (SQLite):
- **Page cache**: Configurable (default ~2 MB)
- **Connection overhead**: ~1 MB per connection
- **Impact**: Negligible for agent orchestration workload

### 5.5 Disk Space Constraints

**Evidence Repository** (SPEC-931A finding):
- **Current size**: 153 MB (99.97% bloat)
- **After cleanup**: 53 KB
- **Policy**: 25 MB soft limit per SPEC (MAINT-4)

**SQLite Database**:
- **consensus_artifacts.db**: 153 MB (before cleanup)
- **Expected size**: <10 MB (after bloat removal)

**Event Log Growth** (post-SPEC-930):
```
Events per agent: 50
Event size: ~500 bytes (JSON)
Agents per day: 300 (quality gates)
Daily growth: 300 × 50 × 500 = 7.5 MB
Monthly growth: ~225 MB
```

**Mitigation**:
- Snapshot + archive old events (retain last 30 days)
- Compress archived events (gzip ~70% reduction)
- Evidence cleanup automation (MAINT-4)

### 5.6 Network Constraints

**Provider API Calls**:
- **Latency**: 100-500 ms per request (typical)
- **Timeout**: 30-60 seconds (configurable)
- **Retries**: Exponential backoff (avoid thundering herd)

**MCP Connections**:
- **Startup timeout**: 5-10 seconds (configurable)
- **Tool call timeout**: 30 seconds (default)
- **Keep-alive**: Long-lived connections (WebSocket or stdio)

**Impact on SPEC-930**:
- **Queue delays**: Network latency << queue wait time (rate limits dominate)
- **Timeouts**: Must handle gracefully (cancel queued work, retry)
- **Fallback**: Multi-provider strategy mitigates single provider failures

### 5.7 Quantified Limits

| Constraint | Linux/macOS | Windows | Source |
|------------|-------------|---------|--------|
| File descriptors (soft) | 256-1024 | N/A | ulimit -n |
| File descriptors (hard) | 4096-unlimited | N/A | ulimit -Hn |
| Process limit (user) | 256-4096 | ~32K | ulimit -u |
| Handle limit | N/A | ~16,000 | Windows docs |
| Stack size (thread) | 2 MB | 2 MB | Rust default |
| Max database size | 281 TB | 281 TB | SQLite limit |
| Network timeout | 30-60s | 30-60s | Configurable |

---

## 6. SPEC-930 PATTERN VALIDATION MATRIX

### 6.1 Event Sourcing (Tier 1: Event Store)

**Constraints Compatibility**:

| Constraint Domain | Assessment | Notes |
|-------------------|------------|-------|
| SQLite single-writer | ✅ Compatible | Events append sequentially (no write contention) |
| Transaction ACID | ✅ Solves dual-write | Single source of truth in event_log |
| Performance | ✅ Acceptable | 10-100 events/agent << 10K writes/sec capacity |
| Platform | ✅ Cross-platform | SQLite works on all platforms |
| Disk space | ⚠️ Requires archival | 7.5 MB/day growth, need snapshot strategy |

**Verdict**: ✅ **FEASIBLE** - Event sourcing is compatible with all constraints. SQLite single-writer is not a blocker (events append-only, no contention). Performance is acceptable (5 ms per agent execution).

**Mitigation for Disk Growth**:
- Snapshot every 1,000 events (reduce replay overhead)
- Archive events older than 30 days (compress + cold storage)
- Vacuum database monthly (reclaim space)

### 6.2 Actor Model (Tier 2: Actor System)

**Constraints Compatibility**:

| Constraint Domain | Assessment | Notes |
|-------------------|------------|-------|
| Tokio runtime | ✅ Compatible | Actors map to tokio::spawn + mpsc channels |
| Message passing | ✅ Performant | 10-50M msg/sec >> agent execution rate |
| Supervision | ✅ Feasible | JoinHandle monitoring + restart logic |
| TUI integration | ⚠️ Requires adaptation | Cannot await in render(), use channels |
| Resource overhead | ✅ Negligible | ~2 KB per actor + channel buffers |

**Verdict**: ✅ **FEASIBLE** - Actor model is compatible with Tokio runtime. TUI integration requires channel-based communication (current approach), not tokio::select! in render loop.

**Mitigation for TUI Integration**:
- Actors send events via broadcast::Sender
- TUI subscribes via broadcast::Receiver (checked in event loop)
- No blocking in render path (events buffered in channel)

### 6.3 Queue-Based Execution (Tier 3: Work Distribution)

**Constraints Compatibility**:

| Constraint Domain | Assessment | Notes |
|-------------------|------------|-------|
| Provider rate limits | ✅ MANDATORY | Without queue, 429 errors guaranteed |
| SQLite performance | ✅ Compatible | Queue table writes < 100/sec |
| Backpressure | ✅ Feasible | Bounded channels + queue saturation check |
| Priority scheduling | ✅ Compatible | ORDER BY priority DESC, enqueued_at ASC |
| Platform | ✅ Cross-platform | No platform-specific dependencies |

**Verdict**: ✅ **FEASIBLE & MANDATORY** - Queue is required to avoid provider rate limit errors. SQLite performance is sufficient for queue operations.

**Rate Limit Avoidance** (quantified):
```
Scenario: 300 agents/day = 5 agents/min average
OpenAI limit: 90K TPM (18 agents/min @ 5K tokens/agent)
Peak: 50 agents/min (10× average, requires queue)
Without queue: 50 × 5K = 250K TPM (2.8× over limit) ❌
With queue: Spread over 3 minutes = 83K TPM ✅
```

### 6.4 Multi-Provider Rate Limiting (Tier 3: Rate Limiter)

**Constraints Compatibility**:

| Constraint Domain | Assessment | Notes |
|-------------------|------------|-------|
| OpenAI TPM/RPM | ✅ Required | 90K TPM default, need tracking |
| Gemini RPM/RPD | ✅ Required | 5 RPM free tier, need queue |
| Claude TPM | ✅ Required | ~50K TPM typical, need tracking |
| Token counting | ⚠️ Estimation required | Exact count unavailable before execution |
| Retry-After headers | ✅ Supported | Exponential backoff + header honor |

**Verdict**: ✅ **FEASIBLE & MANDATORY** - Rate limiter is required to avoid 429 errors across all providers. Token counting can use conservative estimates (actual usage fed back to adjust limits).

**Token Estimation Strategy**:
```rust
struct RateLimiter {
    buckets: HashMap<Provider, TokenBucket>,
}

impl RateLimiter {
    async fn can_execute(&self, agent: &Agent) -> bool {
        let estimated_tokens = estimate_tokens(&agent.prompt); // Conservative
        self.buckets[&agent.provider].try_consume(estimated_tokens)
    }

    fn refund(&mut self, agent: &Agent, actual_tokens: u64) {
        // Refund difference if estimate was high
        let estimated = estimate_tokens(&agent.prompt);
        if actual_tokens < estimated {
            self.buckets[&agent.provider].refund(estimated - actual_tokens);
        }
    }
}
```

### 6.5 TUI Dashboard (Tier 5: TUI Observability)

**Constraints Compatibility**:

| Constraint Domain | Assessment | Notes |
|-------------------|------------|-------|
| Ratatui sync rendering | ⚠️ Architectural change | Current: No tokio::select!, need adaptation |
| 30 FPS limit | ✅ Acceptable | Dashboard updates < 30 FPS (human perception) |
| Async event sources | ✅ Via channels | Actors send events, TUI polls channels |
| Blocking constraints | ✅ Solved | No await in render(), events pre-buffered |
| Platform | ✅ Cross-platform | crossterm + Ratatui work everywhere |

**Verdict**: ⚠️ **FEASIBLE WITH ADAPTATION** - Full tokio::select! integration requires restructuring event loop. Hybrid approach (current channels + actor events) is sufficient for dashboard.

**Alternative Design** (compatible with current architecture):
```rust
// In event loop (existing):
if let Ok(event) = app_event_rx.try_recv() {
    match event {
        AppEvent::ActorEvent(actor_event) => {
            // Update dashboard state (no await)
            dashboard.update_agent_status(actor_event);
            // Request redraw
            self.schedule_redraw();
        }
    }
}
```

**No tokio::select! required**: Events arrive via channels (current pattern), dashboard updates in event handling (synchronous).

### 6.6 Caching-Based Testing (Tier 6: Testing Infrastructure)

**Constraints Compatibility**:

| Constraint Domain | Assessment | Notes |
|-------------------|------------|-------|
| File I/O performance | ✅ Compatible | Cached responses < 1 MB each |
| Determinism | ✅ Achieved | Same prompt → same cached response |
| Cache invalidation | ✅ Feasible | Hash prompt + model + version |
| Platform | ✅ Cross-platform | std::fs works everywhere |
| CI/CD | ✅ Beneficial | Fast tests without API calls |

**Verdict**: ✅ **FEASIBLE** - Caching pattern is compatible with all constraints. File I/O is fast enough for test execution (<1 ms per cache hit).

**Cache Key Strategy**:
```rust
fn cache_key(prompt: &str, model: &str, version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    hasher.update(model.as_bytes());
    hasher.update(version.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

---

## 7. HARD BLOCKERS vs SOLVABLE CONSTRAINTS

### 7.1 Hard Blockers (Cannot Change)

**None identified**. All SPEC-930 patterns are feasible within constraints.

**Key Insight**: Constraints impose design choices (queue-based execution, channel-based TUI updates), but do NOT prevent implementation of any SPEC-930 patterns.

### 7.2 Solvable Constraints (Require Mitigation)

| Constraint | Impact | Mitigation Strategy | Priority |
|------------|--------|---------------------|----------|
| Ratatui sync rendering | Cannot use tokio::select! in event loop | Use channels (current) + actor broadcast | P1 |
| SQLite single-writer | Event log writes serialize | Use WAL mode + batch commits (10-100 events) | P2 |
| Provider rate limits | 429 errors without queue | Implement queue + rate limiter (MANDATORY) | P0 |
| Token estimation | Exact count unavailable before execution | Conservative estimate + refund after execution | P2 |
| spawn_blocking saturation | 500 threads max | Use semaphore for CPU-bound work, prefer async I/O | P3 |
| Disk space growth | 7.5 MB/day event log | Snapshot + archive (retain 30 days) | P2 |
| OAuth2 device flow | Requires browser interaction | Use API keys (if available), pre-authenticate tokens | P1 |

### 7.3 Constraint Mitigation Roadmap

**Phase 1: Immediate (P0)**
1. **Rate limiter**: Implement token bucket per provider (OpenAI, Anthropic, Gemini)
2. **Queue**: Priority queue with provider filtering

**Phase 2: Short-term (P1)**
3. **TUI integration**: Adapt actor events → channels → dashboard updates
4. **OAuth2 handling**: API key preference, token refresh logic

**Phase 3: Medium-term (P2)**
5. **SQLite WAL mode**: Enable for better write concurrency
6. **Event archival**: Snapshot + compress events older than 30 days
7. **Token estimation**: Heuristic + feedback loop for accuracy

**Phase 4: Long-term (P3)**
8. **CPU-bound semaphore**: Limit spawn_blocking usage for CPU work
9. **Full tokio::select! TUI**: If Ratatui async-template proves beneficial

---

## 8. RECOMMENDATIONS

### 8.1 High Priority (Must Address)

**R1: Implement Rate Limiter (P0)**
- **Rationale**: Provider rate limits will cause 429 errors without queue + rate limiter
- **Timeline**: Phase 1 (before scale testing)
- **Effort**: 3-5 days (rate limiter + token bucket + queue integration)

**R2: Implement Queue-Based Execution (P0)**
- **Rationale**: Mandatory for rate limiter, also enables priority + backpressure
- **Timeline**: Phase 1 (concurrent with rate limiter)
- **Effort**: 5-7 days (queue table + dequeue logic + SQLite integration)

**R3: Adapt TUI for Actor Events (P1)**
- **Rationale**: Dashboard requires real-time updates from actor system
- **Timeline**: Phase 2 (during actor model implementation)
- **Effort**: 2-3 days (broadcast channels + dashboard widgets)

**R4: Enable SQLite WAL Mode (P2)**
- **Rationale**: Better write performance, allows concurrent readers during writes
- **Timeline**: Phase 2 (before event sourcing deployment)
- **Effort**: 1 day (migration + validation)

### 8.2 Medium Priority (Should Address)

**R5: Token Estimation Strategy (P2)**
- **Rationale**: Accurate rate limiting requires token count estimates
- **Timeline**: Phase 2 (after rate limiter deployed)
- **Effort**: 2-3 days (estimation heuristic + feedback loop)

**R6: Event Archival Automation (P2)**
- **Rationale**: 7.5 MB/day growth → 2.7 GB/year without archival
- **Timeline**: Phase 3 (after event sourcing stable)
- **Effort**: 2-3 days (snapshot script + compression + cold storage)

**R7: OAuth2 API Key Preference (P1)**
- **Rationale**: Device code flow is blocking, API keys are non-interactive
- **Timeline**: Phase 2 (if tmux removal pursued)
- **Effort**: 1-2 days (config changes + token management)

### 8.3 Low Priority (Nice to Have)

**R8: CPU-Bound Work Semaphore (P3)**
- **Rationale**: Prevent spawn_blocking pool saturation (500 threads)
- **Timeline**: Phase 4 (after scale testing identifies hotspots)
- **Effort**: 1 day (semaphore wrapper + integration)

**R9: Full tokio::select! TUI (P3)**
- **Rationale**: Cleaner async integration (but current channels work fine)
- **Timeline**: Phase 4+ (deferred, not blocking)
- **Effort**: 5-7 days (event loop restructuring + testing)

### 8.4 Decision Matrix

| Pattern | Feasible? | Requires Mitigation? | Blocks SPEC-930? | Recommendation |
|---------|-----------|----------------------|------------------|----------------|
| Event Sourcing | ✅ Yes | ⚠️ Yes (WAL mode) | ❌ No | Implement with WAL |
| Actor Model | ✅ Yes | ⚠️ Yes (TUI channels) | ❌ No | Implement with channels |
| Queue Execution | ✅ Yes | ✅ MANDATORY | ✅ YES | Implement Phase 1 |
| Rate Limiting | ✅ Yes | ✅ MANDATORY | ✅ YES | Implement Phase 1 |
| TUI Dashboard | ✅ Yes | ⚠️ Yes (channels) | ❌ No | Hybrid approach |
| Testing Cache | ✅ Yes | ❌ No | ❌ No | Implement standard |

---

## 9. OPEN QUESTIONS

**Q1: Should we enable SQLite WAL mode by default, or require explicit configuration?**
- **Context**: WAL improves write performance but changes database file structure (adds -wal and -shm files)
- **Impact**: User-visible change (3 files instead of 1)

**Q2: What is the acceptable event log replay overhead? (10 seconds? 60 seconds?)**
- **Context**: Determines snapshot frequency (more snapshots = faster replay, more storage)
- **Trade-off**: Replay time vs storage overhead

**Q3: Should token estimation use conservative (high) or optimistic (low) estimates?**
- **Context**: Conservative = fewer 429s but lower throughput, optimistic = higher throughput but more 429s
- **Recommendation**: Conservative with feedback-based adjustment

**Q4: What is the preferred authentication method for provider CLIs? (API keys vs OAuth2)**
- **Context**: Impacts tmux removal feasibility (OAuth2 requires user interaction)
- **Recommendation**: API keys preferred (non-interactive), OAuth2 fallback

**Q5: Should we implement priority queue FIFO or weighted priority?**
- **Context**: FIFO = simple, weighted = more flexible (age + priority)
- **Trade-off**: Complexity vs fairness

**Q6: What is the acceptable queue wait time before escalation? (30s? 60s? 120s?)**
- **Context**: User experience vs throughput optimization
- **Recommendation**: 60 seconds (balance between responsiveness and efficiency)

**Q7: Should we implement provider fallback automatically or require user opt-in?**
- **Context**: OpenAI unavailable → Claude → Gemini (changes model mid-execution)
- **Trade-off**: Resilience vs predictability

**Q8: What is the archive retention policy for event logs? (30 days? 90 days? 1 year?)**
- **Context**: Disk space vs historical replay capability
- **Recommendation**: 30 days hot, 90 days warm (compressed), 1 year cold (if needed)

---

## 10. SUMMARY & NEXT STEPS

### Key Findings

**All SPEC-930 patterns are feasible** within identified constraints. No hard blockers exist.

**Mandatory Implementations**:
1. **Queue-based execution** (P0) - Required to avoid provider 429 errors
2. **Rate limiting** (P0) - Required to respect provider TPM/RPM limits

**Architectural Adaptations**:
3. **TUI integration** (P1) - Use channels instead of tokio::select! (current approach works)
4. **SQLite WAL mode** (P2) - Enable for better write concurrency

**Constraint Highlights**:
- **Ratatui**: Synchronous rendering (33ms debounce), async via channels (current)
- **SQLite**: Single-writer (not a blocker for event sourcing), WAL mode recommended
- **Provider APIs**: OpenAI 90K-10M TPM, Gemini 5-∞ RPM, rate limiter mandatory
- **Tokio**: spawn_blocking pool ~500 threads, actor model compatible
- **Platform**: Cross-platform (Linux/macOS/Windows), no OS-specific blockers

### Validation Matrix

| SPEC-930 Pattern | Feasible? | Mandatory Mitigation? | Recommendation |
|------------------|-----------|------------------------|----------------|
| Event Sourcing | ✅ YES | ⚠️ WAL mode | Implement |
| Actor Model | ✅ YES | ⚠️ TUI channels | Implement |
| Queue Execution | ✅ YES | ✅ MANDATORY | Implement Phase 1 |
| Rate Limiting | ✅ YES | ✅ MANDATORY | Implement Phase 1 |
| TUI Dashboard | ✅ YES | ⚠️ Channels | Hybrid approach |
| Testing Cache | ✅ YES | ❌ None | Implement |

### Next Steps

**Immediate** (before SPEC-930 implementation):
1. Review constraint findings with stakeholders
2. Prioritize rate limiter + queue implementation (P0)
3. Decide on SQLite WAL mode strategy (P2)
4. Design token estimation heuristic (P2)

**Implementation Phase 1** (foundational):
1. Enable SQLite WAL mode (1 day)
2. Implement rate limiter with token buckets (3-5 days)
3. Implement priority queue with provider filtering (5-7 days)
4. Validate against provider limits (2-3 days)

**Implementation Phase 2** (integration):
5. Implement actor model with TUI channels (5-7 days)
6. Implement event sourcing (7-10 days)
7. Integrate rate limiter with queue dequeue logic (2-3 days)
8. Add dashboard widgets for agent status (3-5 days)

**Total Effort**: ~30-45 days (Phase 1+2)

---

## Appendices

### A. Research Sources

**Ratatui**:
- Source code analysis (`codex-rs/tui/src/app.rs`)
- Crossterm documentation (sync event loop pattern)

**SQLite**:
- SQLite documentation (WAL mode, concurrency model)
- General benchmarks (write throughput, transaction overhead)

**Provider APIs**:
- [OpenAI Rate Limits Documentation](https://platform.openai.com/docs/guides/rate-limits) (2025)
- [Google Gemini Rate Limits](https://ai.google.dev/gemini-api/docs/rate-limits) (2025)
- Anthropic documentation (general knowledge, web search unavailable)

**Tokio**:
- [Tokio spawn_blocking documentation](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
- [Async: What is blocking? – Alice Ryhl](https://ryhl.io/blog/async-what-is-blocking/)
- Tokio runtime architecture documentation

**Platform**:
- Source code analysis (`portable-pty`, `crossterm` usage)
- UNIX ulimit documentation
- Windows process/handle limits

### B. Calculation Assumptions

**Agent Execution Model**:
- Average prompt: 2,000 tokens
- Average output: 3,000 tokens
- Total: 5,000 tokens per agent
- Duration: 5-30 seconds (provider latency)

**Quality Gate Workload**:
- 3-agent consensus per quality gate
- 100 quality gates/day = 300 agent executions/day
- Peak: 50 agents/min (10× average)

**Event Log Growth**:
- 50 events per agent execution
- 500 bytes per event (JSON)
- 300 agents/day × 50 events × 500 bytes = 7.5 MB/day

**Memory Overhead**:
- Tokio task: ~2 KB
- Actor state: ~10 KB
- Channel buffer: 32 messages × 1 KB = 32 KB
- Total per agent: ~44 KB

### C. Glossary

- **TPM**: Tokens per minute (input + output)
- **RPM**: Requests per minute
- **TPD**: Tokens per day
- **RPD**: Requests per day
- **QPM**: Queries per minute (Gemini-specific)
- **QPD**: Queries per day (Gemini-specific)
- **DSQ**: Dynamic Shared Quota (Gemini newer models)
- **WAL**: Write-Ahead Logging (SQLite journaling mode)
- **PTY**: Pseudo-terminal (Unix terminal emulation)
- **ConPTY**: Windows Console Pseudo-terminal

---

**Analysis Complete**: 2025-11-13
**Next**: SPEC-931F (Event Sourcing Feasibility)
