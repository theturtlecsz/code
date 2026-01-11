# SPEC-KIT-930: Comprehensive Agent Orchestration Refactor

**Status**: RESEARCH COMPLETE - Master Reference Spec
**Priority**: P0 (Critical - blocks production scale)
**Created**: 2025-11-12
**Updated**: 2025-11-12 (Research phase complete)
**Parent**: SPEC-KIT-928 (orchestration chaos), SPEC-KIT-929 (closed/deferred)
**Next**: SPEC-KIT-931 (architectural deep dive analysis)
**Type**: Master Spec (research, patterns, industry standards)
**Effort**: 2-3 weeks implementation (after analysis validates approach)

---

## Purpose & Scope

**This is a MASTER REFERENCE SPEC** - serves as comprehensive research and pattern documentation for the agent orchestration refactor.

**Role**:
- Industry research (LangGraph, Temporal, Tokio patterns)
- Pattern library (event sourcing, actor model, rate limiting)
- Architecture vision (what industry-proven systems look like)
- Reference for implementation decisions

**NOT for direct implementation** - see SPEC-KIT-931 for architectural analysis that validates these patterns against our actual system constraints.

**Use**: Consult when making architecture decisions, understanding industry standards, or evaluating trade-offs during implementation.

---

## Problem Statement

Current agent orchestration system is **architecturally fragile** and **failing repeatedly**:

1. **Not ACID-compliant**: Dual-write pattern (in-memory HashMap + SQLite) with no transaction coordination
2. **Not resilient**: 10 bugs fixed in SPEC-928, but tmux-based architecture remains brittle
3. **Not testable**: Requires bash hacks, tmux sessions, manual validation - no proper unit tests
4. **Not observable**: Limited visibility into agent state, no real-time tracking, debugging requires SQLite inspection
5. **Not scalable**: No queueing, backpressure, or rate limiting - will hit API limits at scale
6. **Async/sync mismatch**: Tmux (sync) + tokio (async) creates impedance problems (Claude hang, SPEC-929)

**Evidence** (SPEC-928 session):
- 10 bugs discovered and fixed in single session
- Code agent: 0% → 100% success rate (was completely broken)
- Claude async hang remains unsolved
- Architecture exploration reveals 6 critical gaps (ACID, queueing, error categorization, observability, testing, graceful degradation)

**Current state**: Production-ready for 2-agent quality gates, NOT ready for high-volume, 24/7, or SLA-critical use.

---

## Vision

**Modern Rust async/await agent orchestration** with:

### Core Capabilities
1. **ACID Compliance**: Transaction-based state management, guaranteed consistency
2. **Resilient Execution**: Queue-based work distribution, retry logic, circuit breakers
3. **Observable Lifecycle**: Real-time state machine visibility, telemetry, dashboards
4. **Fully Testable**: Mock-friendly architecture, zero bash/tmux dependencies in tests
5. **Error Resilience**: Categorized errors, graceful degradation, comprehensive recovery

### Architecture Principles
- **Event-sourced state**: All state changes recorded as events (Temporal/LangGraph pattern)
- **Actor-based concurrency**: Supervisor pattern coordinates agent actors via message passing
- **Queue-based distribution**: Work distribution with backpressure and rate limiting
- **Observable lifecycle**: Real-time TUI dashboard using Ratatui async patterns
- **Async-first**: Pure tokio async/await with actor model (no shared mutable state)

### Industry Patterns Adopted

**From Temporal** (Durable Execution):
- Event sourcing for state persistence (all transitions recorded)
- Automatic crash recovery via event replay
- Deterministic execution through event log
- Time-travel debugging (replay to any point)

**From LangGraph** (Multi-Agent Orchestration):
- Supervisor pattern: coordinator agent manages specialist agents
- Graph-based workflows with conditional branching
- Shared scratchpad for agent coordination
- Persistent state across workflow steps

**From Rust Ecosystem** (Tokio Actor Model):
- Actor isolation: each agent has private state, communicates via channels
- Message passing: tokio::sync::mpsc for actor communication
- Supervision trees: supervisor restarts failed agents
- Graceful shutdown: coordinated actor termination

---

## Requirements

### Functional Requirements

**FR-1: Event-Sourced State Management** (Temporal Pattern)
- All state changes recorded as immutable events in event log
- Current state derived by replaying events from log
- Crash recovery: replay events from last checkpoint
- Time-travel debugging: replay to any historical point
- Snapshots for performance: periodic state snapshots reduce replay time
- ACID guarantees: events written transactionally to SQLite

**FR-2: Agent Lifecycle State Machine**
```
Pending → Queued → Running → [Validating] → Completed
   ↓         ↓         ↓                        ↓
   Failed    Failed    Failed                   Failed
                                                 ↓
                                              Retrying → Running
```

**FR-3: Queue-Based Execution**
- Work queue with priority support
- Backpressure when queue full
- Rate limiting per provider (OpenAI, Anthropic, Google)
- Configurable concurrency limits (default: 3 per provider)

**FR-4: Observable State** (Ratatui Async Pattern)
- Real-time agent status dashboard (Ratatui TUI widgets)
- Lifecycle event stream via tokio::sync::broadcast channels
- Elm Architecture pattern: Actions → Model updates → View rendering
- tokio::select! for event handling (ticks, renders, input)
- mpsc channels for async actor-to-UI communication
- Immediate-mode rendering with tick intervals

**FR-5: Error Categorization**
```rust
enum AgentError {
    Timeout { duration: Duration },
    RateLimitExceeded { provider: String, retry_after: Duration },
    ValidationFailed { reason: String },
    ExecutionCrash { exit_code: i32, stderr: String },
    OutputExtractionFailed { raw: String },
    NetworkError { source: reqwest::Error },
    Cancelled { reason: String },
}
```

**FR-6: AI-Specific Retry Logic** (OpenAI/Anthropic Best Practices)
- Exponential backoff with jitter (avoid thundering herd)
- Honor Retry-After headers from API responses
- Per-provider rate limit tracking (TPM, RPM, daily limits)
- Token counting: max_tokens counts toward limit even if unused
- Circuit breaker opens after consecutive failures (5x threshold)
- Provider-specific error handling (429 vs 500 vs timeout)

**FR-7: Multi-Provider Rate Limiting** (2025 Standards)
- **OpenAI**: Track TPM (tokens/min), RPM (requests/min), daily caps
- **Anthropic**: Track ITPM (input tokens/min), OTPM (output), weekly limits
- **Google**: Track QPM (queries/min), QPD (queries/day)
- Token bucket algorithm with per-provider buckets
- Sliding window for burst tolerance
- Request queuing when limits approached (not exceeded)

**FR-8: Graceful Degradation**
- Dynamic consensus threshold (3/3 ideal, 2/3 acceptable, 1/3 emergency)
- Escalation after timeout (60s: wait, 90s: continue with partial, 120s: fail)
- Provider fallback (OpenAI unavailable → Claude → Gemini)
- Quality gate auto-resolution: unanimous → auto-accept, 2/3 → user review

**FR-9: Caching-Based Testing** (Scenario Pattern)
- Record/replay: First run hits real API, subsequent runs use cache
- Deterministic responses without mocking LLM logic
- Cache invalidation: version changes, prompt changes trigger re-record
- Integration tests use real provider responses (realistic)
- Unit tests can use mock actors for speed
- Fixture management: store successful responses as test fixtures

### Non-Functional Requirements

**NFR-1: Performance**
- Agent spawn latency: <100ms (vs current ~1s tmux overhead)
- State update latency: <10ms (vs current ~50-100ms)
- Queue throughput: 100+ agents/minute
- Memory overhead: <50MB for 100 concurrent agents

**NFR-2: Reliability**
- 99.9% success rate for agent completion (excluding API failures)
- 0% state corruption on crash/restart
- 100% test coverage for state transitions
- Zero data loss on power failure

**NFR-3: Observability**
- All state transitions logged with context
- Metrics exported every 15s
- Traces for every agent execution
- Queryable state at any point

**NFR-4: Maintainability**
- Modular architecture (<500 LOC per module)
- Comprehensive documentation
- Type-safe state transitions
- Property-based tests for invariants

---

## Architecture Design

### Module Structure

**Tier 1: Event Store** (Event Sourcing Layer)
```
event_store/
├── mod.rs              # Public API
├── event_log.rs        # Immutable event append-only log
├── snapshots.rs        # State snapshots for fast recovery
├── replay.rs           # Event replay engine
├── projections.rs      # Event → state projections
└── schema.sql          # Event store schema

Responsibilities:
- Event persistence (immutable, append-only)
- Snapshot management (periodic state captures)
- Event replay (crash recovery, time-travel)
- Projection updates (event → current state)

Events:
- AgentQueued, AgentStarted, AgentCompleted, AgentFailed
- AgentRetrying, AgentCancelled, StateTransitioned
- Each event: timestamp, agent_id, event_data (JSON)
```

**Tier 2: Actor System** (Supervisor + Agent Actors)
```
actor_system/
├── mod.rs              # Public API
├── supervisor.rs       # Supervisor actor (LangGraph pattern)
├── agent_actor.rs      # Individual agent actors
├── messages.rs         # Actor message types
├── channels.rs         # Message passing infrastructure
└── supervision.rs      # Restart policies, failure handling

Responsibilities:
- Supervisor coordinates agent actors
- Agent actors: isolated state, message-driven
- Message passing via tokio::sync::mpsc
- Supervision trees (restart failed actors)
- Actor lifecycle management

Actor Messages:
- StartAgent, AgentProgress, AgentComplete, AgentFailed
- CancelAgent, RestartAgent, QueryStatus
```

**Tier 3: Work Queue & Rate Limiting** (Distribution Layer)
```
work_distribution/
├── mod.rs              # Public API
├── priority_queue.rs   # Priority-based work queue
├── rate_limiter.rs     # Multi-provider rate limiting
├── token_tracker.rs    # Token counting (TPM, RPM, daily)
├── backpressure.rs     # Queue saturation handling
└── circuit_breaker.rs  # Per-provider circuit breakers

Responsibilities:
- Queue management (priority, FIFO)
- Rate limit enforcement (OpenAI, Anthropic, Google)
- Token accounting (input + output tokens)
- Backpressure signals (queue full → reject)
- Circuit breaker logic (5x failures → open)
```

**Tier 3: Integration Layer** (external APIs)
```
agent_provider/
├── mod.rs              # Provider abstraction
├── openai.rs           # OpenAI integration
├── anthropic.rs        # Anthropic integration
├── google.rs           # Google Gemini integration
├── local.rs            # Local model support
└── mock.rs             # Testing mock

Responsibilities:
- Provider abstraction
- API communication
- Response parsing
- Error mapping
```

**Tier 4: Provider Integration** (External APIs)
```
agent_provider/
├── mod.rs              # Provider trait abstraction
├── openai.rs           # OpenAI API client
├── anthropic.rs        # Anthropic Claude API client
├── google.rs           # Google Gemini API client
├── local.rs            # Local model support (ollama, etc.)
├── mock.rs             # Testing mock provider
└── response_cache.rs   # Response caching for testing

Responsibilities:
- Provider abstraction (trait-based)
- API communication (async reqwest)
- Response parsing (JSON → AgentResult)
- Error mapping (API errors → AgentError)
- Response caching (record/replay testing)
```

**Tier 5: TUI Observability** (Ratatui Async Pattern)
```
tui_dashboard/
├── mod.rs              # Dashboard entry point
├── widgets/
│   ├── agent_status.rs  # Real-time agent status table
│   ├── queue_viz.rs     # Queue depth + rate limits chart
│   ├── event_log.rs     # Scrolling event stream
│   └── metrics.rs       # Success rate, duration charts
├── event_loop.rs       # tokio::select! event handling
├── actions.rs          # Action enum (Elm Architecture)
├── model.rs            # Application state
└── channels.rs         # Actor → UI mpsc channels

Responsibilities:
- Real-time TUI rendering (Ratatui immediate mode)
- Event handling (tick, render, input via tokio::select!)
- Actor communication (mpsc receiver for updates)
- Elm Architecture: Actions → Model → View
- Tick interval (UI refresh rate)

Pattern:
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
```

**Tier 6: Testing Infrastructure** (Caching-Based)
```
agent_testing/
├── mod.rs              # Test utilities
├── cache_manager.rs    # Response cache (record/replay)
├── mock_actors.rs      # Mock supervisor + agent actors
├── mock_provider.rs    # Mock API provider (deterministic)
├── fixtures/           # Cached real API responses
│   ├── openai_gpt4.json
│   ├── anthropic_claude.json
│   └── google_gemini.json
├── test_harness.rs     # Integration test setup
└── assertions.rs       # Custom assertions

Responsibilities:
- Response caching (first run real, subsequent cached)
- Mock actor implementations (fast unit tests)
- Fixture management (store successful responses)
- Cache invalidation (version/prompt changes)
- Integration test harness (real providers in CI)

Testing Strategy:
- Unit tests: Mock actors, fast (<1s)
- Integration tests: Cached responses, deterministic
- E2E tests: Real providers (gated, CI only)
```

### State Machine

**AgentState enum**:
```rust
pub enum AgentState {
    Pending {
        queued_at: DateTime<Utc>,
    },
    Queued {
        queued_at: DateTime<Utc>,
        queue_position: usize,
    },
    Running {
        started_at: DateTime<Utc>,
        provider: String,
        timeout_at: DateTime<Utc>,
    },
    Validating {
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        raw_output: String,
    },
    Completed {
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        result: AgentResult,
        duration: Duration,
    },
    Failed {
        started_at: Option<DateTime<Utc>>,
        failed_at: DateTime<Utc>,
        error: AgentError,
        retries_remaining: u32,
    },
    Retrying {
        previous_error: AgentError,
        retry_count: u32,
        next_retry_at: DateTime<Utc>,
    },
    Cancelled {
        cancelled_at: DateTime<Utc>,
        reason: String,
    },
}
```

**State Transitions**:
```
Pending → Queued (on queue push)
Queued → Running (on dequeue + execution start)
Running → Validating (on completion, before extraction)
Running → Failed (on timeout/crash/error)
Validating → Completed (on successful extraction)
Validating → Failed (on extraction failure)
Failed → Retrying (if retries remaining)
Retrying → Queued (after backoff delay)
Any → Cancelled (on explicit cancellation)
```

### Database Schema

**event_log** (immutable append-only event store):
```sql
CREATE TABLE event_log (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL, -- 'AgentQueued' | 'AgentStarted' | 'AgentCompleted' | 'AgentFailed' | etc.
    event_data JSON NOT NULL, -- Event-specific payload
    timestamp INTEGER NOT NULL, -- Unix epoch milliseconds
    sequence_number INTEGER NOT NULL, -- Per-agent sequence for ordering

    INDEX idx_agent_events (agent_id, sequence_number),
    INDEX idx_event_timestamp (timestamp),
    INDEX idx_event_type (event_type)
);
```

**agent_snapshots** (periodic state snapshots for fast recovery):
```sql
CREATE TABLE agent_snapshots (
    snapshot_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    state_json JSON NOT NULL, -- Full AgentState serialized
    event_id INTEGER NOT NULL, -- Last event_id included in snapshot
    timestamp INTEGER NOT NULL,

    FOREIGN KEY (event_id) REFERENCES event_log(event_id),
    INDEX idx_snapshot_agent (agent_id, timestamp DESC)
);
```

**agent_executions** (current state projection from events):
```sql
CREATE TABLE agent_executions (
    agent_id TEXT PRIMARY KEY,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    provider TEXT NOT NULL,

    -- State machine
    state TEXT NOT NULL, -- 'Pending' | 'Queued' | 'Running' | 'Validating' | 'Completed' | 'Failed' | 'Retrying' | 'Cancelled'
    state_data JSON NOT NULL, -- State-specific fields

    -- Timestamps
    created_at INTEGER NOT NULL,
    queued_at INTEGER,
    started_at INTEGER,
    completed_at INTEGER,
    failed_at INTEGER,
    cancelled_at INTEGER,

    -- Result
    result_json TEXT,
    error_json TEXT,

    -- Retry
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,

    -- Telemetry
    duration_ms INTEGER,
    queue_wait_ms INTEGER,

    FOREIGN KEY (spec_id) REFERENCES specs(spec_id)
);

CREATE INDEX idx_executions_spec_stage ON agent_executions(spec_id, stage);
CREATE INDEX idx_executions_state ON agent_executions(state);
CREATE INDEX idx_executions_created ON agent_executions(created_at);
```

**agent_queue** (work queue):
```sql
CREATE TABLE agent_queue (
    queue_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL UNIQUE,
    priority INTEGER NOT NULL DEFAULT 0,
    enqueued_at INTEGER NOT NULL,
    provider TEXT NOT NULL,

    FOREIGN KEY (agent_id) REFERENCES agent_executions(agent_id)
);

CREATE INDEX idx_queue_priority ON agent_queue(priority DESC, enqueued_at ASC);
CREATE INDEX idx_queue_provider ON agent_queue(provider);
```

**agent_metrics** (telemetry):
```sql
CREATE TABLE agent_metrics (
    metric_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL,
    labels JSON,

    FOREIGN KEY (agent_id) REFERENCES agent_executions(agent_id)
);

CREATE INDEX idx_metrics_timestamp ON agent_metrics(timestamp);
CREATE INDEX idx_metrics_name ON agent_metrics(metric_name, timestamp);
```

### Event Sourcing Patterns (Temporal-Inspired)

**Event Append (State Transition)**:
```rust
async fn transition_to_running(
    event_store: &EventStore,
    agent_id: &str,
) -> Result<(), AgentError> {
    event_store.transaction(|tx| {
        // 1. Replay events to get current state
        let current_state = tx.replay_events(agent_id)?;

        // 2. Validate transition
        if !current_state.can_transition_to(AgentState::Running) {
            return Err(InvalidTransition);
        }

        // 3. Create event
        let event = AgentEvent::AgentStarted {
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            provider: current_state.provider.clone(),
            timeout_at: Utc::now() + Duration::from_secs(1800),
        };

        // 4. Append event (immutable, append-only)
        tx.append_event(&event)?;

        // 5. Update projection (agent_executions table)
        let new_state = current_state.apply_event(&event);
        tx.update_projection(agent_id, &new_state)?;

        // 6. Commit (ACID guarantee)
        tx.commit()?;

        // 7. Broadcast event to actors
        event_bus.publish(event).await;

        Ok(())
    }).await
}
```

**Snapshot Creation (Performance Optimization)**:
```rust
async fn create_snapshot(
    event_store: &EventStore,
    agent_id: &str,
) -> Result<(), AgentError> {
    // 1. Replay all events
    let events = event_store.get_events(agent_id).await?;
    let state = AgentState::from_events(&events)?;

    // 2. Save snapshot
    event_store.save_snapshot(agent_id, &state, events.last().id).await?;

    // Fast recovery: load snapshot + replay events after snapshot
    Ok(())
}
```

**Crash Recovery (Event Replay)**:
```rust
async fn recover_agent_state(
    event_store: &EventStore,
    agent_id: &str,
) -> Result<AgentState, AgentError> {
    // 1. Load most recent snapshot (if exists)
    let snapshot = event_store.get_latest_snapshot(agent_id).await?;

    // 2. Replay events after snapshot
    let events = if let Some(snapshot) = snapshot {
        // Fast path: snapshot + recent events
        event_store.get_events_after(agent_id, snapshot.event_id).await?
    } else {
        // Slow path: replay all events
        event_store.get_all_events(agent_id).await?
    };

    // 3. Apply events to get current state
    let mut state = snapshot.map(|s| s.state).unwrap_or_default();
    for event in events {
        state = state.apply_event(&event);
    }

    Ok(state)
}
```

### Actor Communication Patterns (Tokio + LangGraph)

**Supervisor Actor (Coordinator)**:
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
                        // 1. Create agent actor
                        let (tx, rx) = mpsc::channel(32);
                        let handle = tokio::spawn(agent_actor(agent_id.clone(), provider, rx));

                        // 2. Track actor
                        agent_actors.insert(agent_id.clone(), AgentHandle { tx, handle });

                        // 3. Send start message
                        tx.send(AgentMessage::Start).await;
                    },
                    SupervisorCommand::CancelAgent { agent_id } => {
                        if let Some(handle) = agent_actors.get(&agent_id) {
                            handle.tx.send(AgentMessage::Cancel).await;
                        }
                    },
                    SupervisorCommand::Shutdown => {
                        // Graceful shutdown: notify all actors
                        for (_, handle) in agent_actors.drain() {
                            let _ = handle.tx.send(AgentMessage::Shutdown).await;
                            let _ = handle.handle.await;
                        }
                        break;
                    }
                }
            },
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                // Periodic health check
                for (agent_id, handle) in &agent_actors {
                    if handle.handle.is_finished() {
                        // Actor crashed, restart if needed
                        warn!("Agent {} crashed, restarting", agent_id);
                    }
                }
            }
        }
    }
}
```

**Agent Actor (Worker)**:
```rust
async fn agent_actor(
    agent_id: String,
    provider: String,
    mut msg_rx: mpsc::Receiver<AgentMessage>,
) {
    let mut state = AgentActorState::Idle;

    loop {
        tokio::select! {
            Some(msg) = msg_rx.recv() => {
                match (state, msg) {
                    (AgentActorState::Idle, AgentMessage::Start) => {
                        // Execute agent
                        state = AgentActorState::Running;
                        let result = execute_agent(&agent_id, &provider).await;

                        // Report completion
                        match result {
                            Ok(output) => {
                                event_tx.send(AgentEvent::AgentCompleted { agent_id, output });
                                state = AgentActorState::Completed;
                            },
                            Err(error) => {
                                event_tx.send(AgentEvent::AgentFailed { agent_id, error });
                                state = AgentActorState::Failed;
                            }
                        }
                    },
                    (_, AgentMessage::Cancel) => {
                        state = AgentActorState::Cancelled;
                        break;
                    },
                    (_, AgentMessage::Shutdown) => {
                        break;
                    },
                    _ => {}
                }
            }
        }
    }
}
```

### Queue Algorithm

**Dequeue with Rate Limiting**:
```rust
async fn dequeue_next_agent(
    queue: &AgentQueue,
    rate_limiter: &RateLimiter,
) -> Option<AgentExecution> {
    // 1. Check rate limits per provider
    let available_providers = rate_limiter.available_providers().await;

    // 2. Query queue for highest priority agent from available provider
    queue.transaction(|tx| {
        let agent = tx.dequeue_where(|a| {
            available_providers.contains(&a.provider)
        })?;

        if let Some(agent) = agent {
            // 3. Acquire rate limit slot
            rate_limiter.acquire(&agent.provider).await?;

            // 4. Transition to Running
            tx.update_agent_state(&agent.agent_id, AgentState::Running { ... })?;

            // 5. Remove from queue
            tx.remove_from_queue(&agent.agent_id)?;

            tx.commit()?;

            Some(agent)
        } else {
            None
        }
    }).await.ok().flatten()
}
```

---

## Implementation Plan

### Phase 1: Foundation (Week 1)

**Goal**: Core state management + minimal executor

**Tasks**:
1. **agent_store module** (2 days)
   - SQLite schema + migrations
   - Transaction API
   - State queries
   - Memory cache
   - Tests: ACID properties, crash recovery, concurrent access

2. **AgentState state machine** (1 day)
   - Enum definition
   - Transition validation
   - Serialization (to JSON for SQLite)
   - Tests: All valid transitions, reject invalid, state-specific data

3. **Minimal executor** (2 days)
   - Basic async execution (no queue)
   - State transitions (Pending → Running → Completed/Failed)
   - OpenAI provider integration
   - Tests: Happy path, timeout, error handling

**Acceptance**:
- Can spawn agent, execute, store result in SQLite
- State machine enforces valid transitions
- Crash mid-execution → restart from SQLite state
- 100% test coverage for state transitions

### Phase 2: Queue + Rate Limiting (Week 2)

**Goal**: Production-grade work distribution

**Tasks**:
1. **agent_queue module** (2 days)
   - Queue table + operations
   - Priority-based dequeue
   - Backpressure (queue full → reject)
   - Tests: FIFO, priority ordering, backpressure

2. **Rate limiter** (1 day)
   - Per-provider limits (configurable)
   - Token bucket algorithm
   - Sliding window
   - Tests: Enforce limits, release on completion

3. **Executor integration** (2 days)
   - Queue push on agent spawn
   - Dequeue loop with rate limiting
   - Graceful shutdown
   - Tests: Multi-agent execution, rate limit enforcement

**Acceptance**:
- Can queue 100 agents, execute with rate limits
- No more than N concurrent per provider
- Queue backpressure works
- Graceful shutdown (wait for in-flight agents)

### Phase 3: Error Resilience (Week 2)

**Goal**: Retry logic + circuit breakers

**Tasks**:
1. **AgentError enum** (1 day)
   - Error categories
   - Error context (provider, duration, etc.)
   - Serialization
   - Tests: All error types, context preservation

2. **Retry logic** (2 days)
   - Configurable retry policies
   - Exponential backoff with jitter
   - Retry budget
   - Failed → Retrying → Queued transition
   - Tests: Retry on transient errors, give up on permanent, backoff timing

3. **Circuit breaker** (1 day)
   - Failure threshold detection
   - Open/half-open/closed states
   - Provider fallback
   - Tests: Circuit opens, recovers, fallback works

**Acceptance**:
- Transient errors retry automatically (up to 3x)
- Circuit breaker opens after 5 consecutive failures
- Provider fallback works (OpenAI → Claude → Gemini)
- Permanent errors fail immediately (no retry)

### Phase 4: Observability (Week 3)

**Goal**: Full telemetry + dashboards

**Tasks**:
1. **Metrics** (1 day)
   - Prometheus integration
   - Key metrics (success rate, duration, queue depth, retry rate)
   - SQLite storage
   - Tests: Metrics recorded, queryable

2. **Events** (1 day)
   - Lifecycle event stream
   - Tokio broadcast channel
   - Event filtering
   - Tests: Events broadcast, subscribers receive

3. **TUI dashboard** (2 days)
   - Real-time agent status widget
   - Queue depth / rate limit visualization
   - Success/failure charts
   - Tests: Widget renders correctly, updates on state change

4. **Traces** (1 day)
   - OpenTelemetry integration
   - Trace full execution path
   - Span hierarchy
   - Tests: Traces recorded, spans linked

**Acceptance**:
- TUI shows real-time agent status
- Prometheus metrics exported
- Can query: "Show all agents stuck >5 min"
- Trace shows full lifecycle (queue → execute → validate → complete)

### Phase 5: Testing Infrastructure (Week 3)

**Goal**: 100% test coverage, no bash hacks

**Tasks**:
1. **Mock executor** (1 day)
   - Simulated execution (no API calls)
   - Configurable latency/errors
   - Deterministic output
   - Tests: All execution paths work with mock

2. **In-memory store** (1 day)
   - SQLite alternative for tests
   - Same API as SqliteStore
   - Faster, no file I/O
   - Tests: All store operations work in-memory

3. **Test harness** (1 day)
   - Integration test helpers
   - Fixture management
   - Custom assertions
   - Example tests demonstrating patterns

**Acceptance**:
- Can run full integration tests in <1s
- No external dependencies (SQLite, tmux, bash)
- Deterministic tests (no flakiness)
- 100% test coverage for new modules

### Phase 6: Migration + Validation (Week 3)

**Goal**: Replace old system, validate production readiness

**Tasks**:
1. **Adapter layer** (2 days)
   - Wrap new system with old API
   - Feature flag for gradual rollout
   - Parallel execution (old + new, compare)
   - Tests: Old API calls new implementation

2. **Production validation** (1 day)
   - Run SPEC-KIT-900 with new system
   - Compare results with old system
   - Validate telemetry
   - Performance benchmarks

3. **Documentation** (1 day)
   - Architecture guide
   - API documentation
   - Migration guide
   - Operations runbook

**Acceptance**:
- SPEC-KIT-900 completes successfully with new system
- Results identical to old system
- Performance metrics met (NFR-1)
- Zero regressions to SPEC-928 fixes

---

## Success Criteria

### Must Achieve

1. ✅ **ACID Compliance**: Crash mid-execution → restart from consistent state, zero corruption
2. ✅ **Queue-Based Execution**: 100 agents queued, execute with rate limits, no provider overload
3. ✅ **Observable State**: TUI dashboard shows real-time agent status, clear lifecycle
4. ✅ **Fully Testable**: Integration tests run in <5s, no bash/tmux dependencies, 100% coverage
5. ✅ **Error Resilience**: Retry on transient errors, circuit breaker on repeated failures, graceful degradation
6. ✅ **Performance**: Agent spawn <100ms, state update <10ms, queue throughput 100+ agents/min
7. ✅ **Zero Regressions**: All 10 SPEC-928 bugs stay fixed, code agent 100% success rate maintained

### Optional Goals

1. Prometheus dashboard (Grafana)
2. Distributed tracing UI (Jaeger)
3. Multi-tenant support (isolated queues per user)
4. Streaming agent output (SSE to TUI)
5. Agent result caching (avoid re-execution)

---

## Risks & Mitigations

### Risk 1: 3-Week Timeline Too Ambitious

**Probability**: Medium
**Impact**: High (delays production use)

**Mitigation**:
- Phased rollout (Phase 1-3 minimum viable)
- Parallel old system (feature flag)
- Timebox each phase, cut scope if needed

---

### Risk 2: Breaking Changes to Agent API

**Probability**: High
**Impact**: Medium (client code changes required)

**Mitigation**:
- Adapter layer preserves old API
- Gradual migration path
- Comprehensive documentation

---

### Risk 3: Performance Regression

**Probability**: Low
**Impact**: High (slower than current system)

**Mitigation**:
- Benchmark early and often
- Profile hot paths
- In-memory cache for reads
- Async all the way (no blocking)

---

### Risk 4: Test Coverage Gaps

**Probability**: Medium
**Impact**: Medium (bugs in production)

**Mitigation**:
- Property-based tests (proptest)
- Mutation testing (cargo-mutants)
- Fuzzing critical paths
- Integration tests with real providers (gated)

---

## Dependencies

**Upstream**:
- ✅ SPEC-KIT-928 complete (bugs fixed, diagnostic logging)
- ✅ Architecture exploration complete (pain points identified)
- ⏸️ SPEC-KIT-929 deferred (Claude hang addressed by refactor)

**External**:
- tokio 1.x (async runtime)
- rusqlite (SQLite binding)
- prometheus (metrics)
- opentelemetry (traces)
- proptest (property-based testing)

**Blocks**:
- SPEC-KIT-900 (end-to-end validation)
- Production scale deployment (high-volume use)
- 3-agent quality gates (Claude re-enabled)

---

## Effort Estimate

**Total**: 2-3 weeks full-time (80-120 hours)

**Breakdown**:
- Phase 1 (Foundation): 5 days
- Phase 2 (Queue): 5 days
- Phase 3 (Resilience): 4 days
- Phase 4 (Observability): 5 days
- Phase 5 (Testing): 3 days
- Phase 6 (Migration): 4 days

**Confidence**: Medium (±5 days)
- New architecture (unknown unknowns)
- SQLite expertise needed
- Async Rust complexity

---

## Alternatives Considered

### Alternative A: Incremental Fixes to Current System

**Approach**: Fix Claude hang, add queueing, improve error handling

**Pros**: Faster (1 week), less risk
**Cons**: Still tmux-based, still dual-write, still hard to test, technical debt accumulates

**Decision**: REJECTED - Band-aids don't fix architecture

---

### Alternative B: Adopt Existing Workflow Engine

**Approach**: Use Temporal, Prefect, or similar

**Pros**: Battle-tested, full-featured
**Cons**: Heavy dependency, overkill for agent orchestration, learning curve, Rust integration unclear

**Decision**: REJECTED - Too heavyweight, better to build fit-for-purpose

---

### Alternative C: Keep 2-Agent Consensus Permanently

**Approach**: Accept Claude limitation, standardize on Gemini + Code

**Pros**: Zero effort, works today
**Cons**: Waste of Claude capabilities, less diverse consensus, doesn't solve scale/testing/observability

**Decision**: REJECTED - Doesn't address root problems

---

## References

**Parent SPECs**:
- SPEC-KIT-928: Orchestration chaos - 10 bugs fixed
- SPEC-KIT-929: Claude async hang - deferred

**Architecture Analysis**:
- Agent spawning system exploration (Explore agent, 2025-11-12)
- 12-step execution flow documented
- Pain points identified (ACID, queueing, observability, testing)

**Files**:
- core/src/agent_tool.rs (1,853 LOC - to be split)
- core/src/tmux.rs (500+ LOC - to be replaced)
- tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs (319 LOC - to be refactored)
- core/src/consensus_db.rs (400+ LOC - to be reused)

**Related**:
- async-sync-boundaries.md (impedance mismatch documentation)
- UPSTREAM-SYNC.md (fork management)
- testing-policy.md (coverage targets)

---

### Industry Research & Best Practices (2025)

**AI Agent Orchestration Frameworks**:
- **LangGraph**: Graph-based multi-agent workflows, supervisor pattern, persistent state, conditional branching
- **Temporal**: Durable execution via event sourcing, automatic crash recovery, state management, time-travel debugging
- **CrewAI**: Role-based agent systems, simpler abstractions, rapid iteration
- **AutoGen**: Conversation-based agent collaboration, research-grade flexibility

**Key Insights**:
- LangGraph supervisor pattern maps perfectly to quality gate orchestration
- Temporal event sourcing provides battle-tested state persistence
- All frameworks emphasize state management and crash recovery
- Industry trend: Move from stateless to stateful agent workflows

**Rust Async & Actor Model**:
- **Tokio Actor Pattern**: Alice Ryhl's canonical blog post "Actors with Tokio"
- **Actor frameworks**: Actix (Tokio-based), Ractor (Erlang gen_server model)
- **Message passing**: tokio::sync::mpsc for actor communication
- **Isolation**: Each actor private state, communicate exclusively via channels

**Key Patterns**:
- Supervision trees: supervisor restarts failed actors
- Graceful shutdown: coordinated actor termination via messages
- Backpressure: bounded channels prevent memory explosion

**AI API Rate Limiting (2025)**:
- **OpenAI**: TPM (tokens/min), RPM (requests/min), daily caps. max_tokens counts even if unused.
- **Anthropic**: ITPM (input tokens/min), OTPM (output), weekly limits (Aug 2025 update for Claude Code)
- **Google Gemini**: QPM (queries/min), QPD (queries/day)

**Best Practices**:
- Exponential backoff with jitter (avoid thundering herd)
- Honor Retry-After headers (API-provided guidance)
- Token bucket algorithm per provider
- Track both request and token limits simultaneously
- Queue requests when approaching limits (not when exceeded)

**Testing AI Agents**:
- **Caching-based determinism**: Scenario pattern (first run real API, subsequent cached)
- **Mock approaches**: Mock external tools, NOT core LLM logic
- **DeepEval**: pytest-like framework for LLM evaluation
- **Fixtures**: Store successful API responses for replay

**Key Insight**: Caching provides deterministic tests while testing real integration code.

**Ratatui Async Patterns**:
- **Official async-template**: tokio::select! for event handling
- **Elm Architecture**: Actions → Model updates → View rendering
- **Event loop**: tick_interval + render_interval + input stream
- **Actor communication**: mpsc channels from actors to UI
- **Immediate-mode rendering**: Fast, efficient, declarative

**Pattern**:
```rust
tokio::select! {
    _ = tick_interval.tick() => { /* update model */ },
    event = event_rx.recv() => { /* handle actor event */ },
    input = input_rx.recv() => { /* handle user input */ },
}
terminal.draw(|f| render(f, &model))?;
```

---

### Research Sources (Web Search 2025-11-12)

1. **LangGraph Multi-Agent Orchestration** (Latenode, AWS, Medium)
   - Supervisor pattern, graph-based workflows, persistent state
   - Conditional branching, subgraphs, time-travel debugging

2. **Temporal for AI Workflows** (Temporal.io blog, IntuitionLabs)
   - Event sourcing, durable execution, automatic recovery
   - Code-first state management, retry semantics

3. **Framework Comparisons** (Turing, Langfuse, DataCamp, Composio)
   - LangGraph vs AutoGen vs CrewAI detailed analysis
   - Use case recommendations, trade-offs

4. **Rust Tokio Actor Model** (Medium, eo.rs, Alice Ryhl blog)
   - Actor pattern implementation with Tokio
   - Message passing, supervision, graceful shutdown

5. **OpenAI/Anthropic Rate Limits** (Claude Docs, Apidog, TechCrunch)
   - 2025 rate limit structures, best practices
   - Token counting, Retry-After headers, exponential backoff

6. **Testing LLM Agents** (Scenario, Medium, MLOps Community)
   - Caching vs mocking approaches
   - Deterministic testing strategies, DeepEval framework

7. **Ratatui Async** (Official docs, GitHub async-template)
   - tokio::select! patterns, Elm Architecture
   - Event handling, immediate-mode rendering

---

**Next Steps**:
1. Review plan with stakeholders
2. Prototype Phase 1 (state machine + minimal executor)
3. Validate performance assumptions
4. Begin implementation
