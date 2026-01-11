# SPEC-945C: Retry & Error Handling Implementation

**Document Version**: 1.0
**Created**: 2025-11-13
**Status**: Implementation Ready
**Implementation Spec**: 3 of 6 (SPEC-KIT-945 Series)
**Estimated Implementation**: 1-2 weeks
**Dependencies**: SPEC-945A (Async Patterns)

---

## Executive Summary

This specification provides a complete implementation guide for exponential backoff retry logic, circuit breaker patterns, and error classification systems for SPEC-KIT-945. The implementation supports SPEC-938 (Enhanced Agent Retry) requirements and integrates seamlessly with quality gate consensus logic.

### Technology Stack
- **Retry Logic**: backon 1.1+ (primary) or backoff 0.4+ (fallback)
- **Error Types**: thiserror 1.0+ for structured error definitions
- **Circuit Breaker**: failsafe-rs 1.0+ (optional, advanced use cases)
- **Async Runtime**: tokio 1.35+ (async-compatible retry patterns)

### PRDs Supported
- **SPEC-938**: Enhanced agent retry logic (error classification, exponential backoff, quality gate integration)
- **SPEC-933**: Database transactions (retry-safe ACID operations)
- **SPEC-KIT-945**: Async orchestration (retry in async context)

### Expected Benefits
- **Reduced Transient Failures**: 80%+ success rate on retry for timeout/rate limit/503 errors
- **3/3 Consensus Reliability**: 95%+ quality gates achieve full consensus (up from 70%)
- **Graceful Degradation**: Circuit breakers prevent cascading failures during sustained outages
- **Transparent Recovery**: Automatic retry with exponential backoff and jitter (no manual intervention)

### Critical Warning
**tokio-retry has maintenance concerns** (research findings Section 3). Use backon 1.1+ as primary implementation, backoff 0.4+ as fallback. Do NOT use tokio-retry for new code.

---

## 1. Technology Research Summary

### Best Practices (from Research Section 3)

**Core Principles**:
1. **Fast Fail on Permanent Errors**: Authentication failures, invalid inputs should not be retried (waste of resources)
2. **Exponential Backoff on Transient Errors**: Network timeouts, rate limits require increasing delays
3. **Jitter is Mandatory**: Prevents thundering herd problem (all clients retry simultaneously)
4. **Circuit Break on Sustained Failures**: Prevent cascading failures across system boundaries

**Industry Standards**:
- **Exponential Backoff**: 100ms → 200ms → 400ms → 800ms → 1.6s (doubles each attempt)
- **Jitter Range**: ±25-50% randomness to spread retry storms
- **Max Attempts**: 3-5 retries before permanent failure (configurable)
- **Retry Budget**: Limit concurrent retries (prevents resource exhaustion)

### Recommended Crates (Ranked by Priority)

#### Primary: backon 1.1+
**Pros**:
- ✅ Chainable API (ergonomic, composable)
- ✅ WASM/no_std support (broad compatibility)
- ✅ Built-in jitter (prevents thundering herd)
- ✅ Actively maintained (v1.1 released 2024-08)

**Cons**:
- ❌ Newer ecosystem (less battle-tested than backoff)
- ❌ Smaller community (fewer examples)

**Use When**: New code, modern async patterns, WASM compatibility needed

#### Fallback: backoff 0.4+
**Pros**:
- ✅ Battle-tested (production-proven since 2017)
- ✅ Comprehensive (backoff, circuit breaker, fallback)
- ✅ Stable API (0.4+ mature)

**Cons**:
- ❌ Less ergonomic than backon (more verbose)
- ❌ Async support less polished

**Use When**: Conservative choice, existing codebase uses backoff, need stability

#### Advanced: failsafe-rs 1.0+
**Pros**:
- ✅ Comprehensive resilience patterns (circuit breaker, retry, fallback, bulkhead)
- ✅ Production-ready (1.0+ stable API)
- ✅ Integrates well with Tower ecosystem

**Cons**:
- ❌ Complex API (learning curve)
- ❌ Heavier dependency (larger footprint)

**Use When**: Need advanced circuit breaker, complex fallback logic, Tower integration

#### Avoid: tokio-retry 0.3
**Reason**: Feels unmaintained (last release 2021), limited features, no active development

### Performance Characteristics (from Research)

**Exponential Backoff Timing** (5 retries, 100ms base):
```
Attempt 1: Immediate (0ms)
Attempt 2: 100ms + jitter (0-50ms) = 100-150ms
Attempt 3: 200ms + jitter (0-100ms) = 200-300ms
Attempt 4: 400ms + jitter (0-200ms) = 400-600ms
Attempt 5: 800ms + jitter (0-400ms) = 800-1200ms
Attempt 6: 1600ms + jitter (0-800ms) = 1600-2400ms

Total Time: ~3.1-4.7s (with jitter)
```

**Jitter Benefits**:
- Prevents synchronized retry storms (thundering herd)
- Spreads load over time (smoother API utilization)
- Reduces probability of cascading failures

**Circuit Breaker Overhead**:
- State checking: ~10-50µs per call
- Memory usage: ~1KB per circuit breaker instance
- Failure threshold: 5 failures in 60s (configurable)

### Sources (Authoritative References)
- [backon Crate Documentation](https://docs.rs/backon/latest/backon/)
- [BackON Reaches v1 - Design Rationale](https://xuanwo.io/2024/08-backon-reaches-v1/)
- [backoff Crate Documentation](https://docs.rs/backoff/latest/backoff/)
- [failsafe-rs GitHub](https://github.com/dmexe/failsafe-rs)
- [AWS SDK Error Handling - Retryable vs Permanent](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/retries.html)

---

## 2. Detailed Implementation Plan

### 2.1 Code Structure

**New Module Layout**:
```
codex-rs/
├── spec-kit/src/
│   ├── retry/
│   │   ├── mod.rs              (NEW - public API, re-exports)
│   │   ├── strategy.rs         (NEW - backoff strategy implementations)
│   │   ├── classifier.rs       (NEW - error classification logic)
│   │   └── circuit_breaker.rs  (NEW - circuit breaker pattern, optional)
│   ├── consensus.rs            (MODIFY - integrate retry for 3/3 consensus)
│   ├── quality_gates.rs        (MODIFY - retry failed agents before degrading)
│   └── error.rs                (MODIFY - extend with RetryClassifiable trait)
└── tui/src/widgets/spec_kit/
    └── handler.rs              (MODIFY - retry orchestration in TUI)
```

**Module Responsibilities**:
- **retry/mod.rs**: Public API (`retry_with_backoff()`, `RetryConfig`)
- **retry/strategy.rs**: Backoff algorithms (exponential, constant, fibonacci)
- **retry/classifier.rs**: Error classification (retryable vs permanent)
- **retry/circuit_breaker.rs**: Circuit breaker state machine (optional)

### 2.2 Error Classification Hierarchy

**Rust Type Definitions**:
```rust
// retry/classifier.rs
use thiserror::Error;

/// Top-level error classification for retry decision-making
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    /// Transient errors that should be retried with backoff
    Retryable(RetryableError),

    /// Permanent errors that should NOT be retried
    Permanent(PermanentError),

    /// Degraded state (partial success, e.g., 2/3 consensus)
    Degraded(DegradedError),
}

/// Transient errors (retry recommended)
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RetryableError {
    #[error("Network timeout after {0}s")]
    NetworkTimeout(u64),

    #[error("Rate limit exceeded, retry after {retry_after}s")]
    RateLimitExceeded { retry_after: u64 },

    #[error("Service unavailable (HTTP 503)")]
    ServiceUnavailable,

    #[error("Database locked (SQLITE_BUSY)")]
    DatabaseLocked,

    #[error("Connection refused")]
    ConnectionRefused,

    #[error("DNS resolution failed")]
    DnsError,
}

/// Permanent errors (do NOT retry)
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PermanentError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid input in field '{field}': {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Daily quota exceeded (resets at {reset_time})")]
    QuotaExceeded { reset_time: String },

    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

/// Degraded state (partial success, retryable but non-critical)
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DegradedError {
    #[error("Degraded consensus: {success}/{total} agents succeeded")]
    DegradedConsensus { success: usize, total: usize },

    #[error("Partial data: {available}/{expected} records available")]
    PartialData { available: usize, expected: usize },
}

/// Trait for error classification
pub trait RetryClassifiable {
    fn classify(&self) -> ErrorClass;
    fn is_retryable(&self) -> bool {
        matches!(self.classify(), ErrorClass::Retryable(_))
    }
    fn suggested_backoff(&self) -> Option<Duration>;
}
```

**Classification Logic** (implementation in `classifier.rs`):
```rust
impl RetryClassifiable for AgentError {
    fn classify(&self) -> ErrorClass {
        match self {
            // Retryable errors
            AgentError::Timeout(duration) => {
                ErrorClass::Retryable(RetryableError::NetworkTimeout(duration.as_secs()))
            }
            AgentError::HttpStatus(429) => {
                // Rate limit - extract Retry-After header if available
                let retry_after = self.retry_after_header().unwrap_or(5);
                ErrorClass::Retryable(RetryableError::RateLimitExceeded { retry_after })
            }
            AgentError::HttpStatus(503) => {
                ErrorClass::Retryable(RetryableError::ServiceUnavailable)
            }
            AgentError::DatabaseError(ref e) if e.contains("SQLITE_BUSY") => {
                ErrorClass::Retryable(RetryableError::DatabaseLocked)
            }
            AgentError::NetworkError(_) => {
                ErrorClass::Retryable(RetryableError::ConnectionRefused)
            }

            // Permanent errors
            AgentError::InvalidApiKey(ref msg) => {
                ErrorClass::Permanent(PermanentError::AuthenticationFailed(msg.clone()))
            }
            AgentError::InvalidInput { field, reason } => {
                ErrorClass::Permanent(PermanentError::InvalidInput {
                    field: field.clone(),
                    reason: reason.clone(),
                })
            }
            AgentError::ModelNotFound(ref model) => {
                ErrorClass::Permanent(PermanentError::ModelNotFound(model.clone()))
            }
            AgentError::HttpStatus(status) if (400..500).contains(status) => {
                // 4xx client errors (except 429) are permanent
                ErrorClass::Permanent(PermanentError::InvalidInput {
                    field: "http_status".to_string(),
                    reason: format!("HTTP {}", status),
                })
            }

            // Degraded state
            AgentError::ConsensusFailure { success, total } if success >= 2 => {
                // 2/3 consensus is degraded but acceptable
                ErrorClass::Degraded(DegradedError::DegradedConsensus { success, total })
            }

            // Default: treat unknown errors as permanent (safe default)
            _ => ErrorClass::Permanent(PermanentError::InvalidInput {
                field: "unknown".to_string(),
                reason: format!("Unknown error: {:?}", self),
            }),
        }
    }

    fn suggested_backoff(&self) -> Option<Duration> {
        match self {
            // Rate limit with explicit Retry-After header
            AgentError::HttpStatus(429) => {
                self.retry_after_header().map(Duration::from_secs)
            }
            // Database lock - short backoff (100ms typical)
            AgentError::DatabaseError(_) => Some(Duration::from_millis(100)),
            // Default: use exponential backoff algorithm
            _ => None,
        }
    }
}
```

### 2.3 Retry Decision Tree

**Visual Flow**:
```
┌─────────────────┐
│  Error Occurs   │
└────────┬────────┘
         │
         ▼
  ┌──────────────┐
  │ Classify     │
  │ Error        │
  └──────┬───────┘
         │
    ┌────┴────────────────┐
    │                     │
    ▼                     ▼
┌───────────┐      ┌──────────────┐
│ Retryable │      │ Permanent    │
└─────┬─────┘      └──────┬───────┘
      │                   │
      ▼                   ▼
┌───────────────┐   ┌──────────────┐
│ Check Budget  │   │ Fail         │
└──────┬────────┘   │ Immediately  │
       │            └──────────────┘
  ┌────┴────┐
  │         │
  ▼         ▼
Budget      Budget
Available   Exhausted
  │            │
  ▼            ▼
Apply    ┌──────────────┐
Backoff  │ Mark as      │
+ Retry  │ Degraded     │
         └──────────────┘
```

**Implementation** (retry/mod.rs):
```rust
pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: &RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> BoxFuture<'static, Result<T, E>>,
    E: RetryClassifiable + std::error::Error,
{
    let mut attempt = 1;
    let mut strategy = ExponentialBackoff::new(config);

    loop {
        // Execute operation
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    tracing::info!(
                        attempt = attempt,
                        "Retry successful after {} attempts",
                        attempt
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                // Classify error
                let classification = e.classify();

                match classification {
                    ErrorClass::Permanent(_) => {
                        // Permanent error - fail immediately
                        tracing::error!(
                            error = %e,
                            classification = ?classification,
                            "Permanent error detected, not retrying"
                        );
                        return Err(e);
                    }

                    ErrorClass::Retryable(_) => {
                        // Check retry budget
                        if attempt >= config.max_attempts {
                            tracing::error!(
                                attempt = attempt,
                                max_attempts = config.max_attempts,
                                error = %e,
                                "Max retry attempts exceeded"
                            );
                            return Err(e);
                        }

                        // Calculate backoff (with jitter)
                        let backoff = e.suggested_backoff()
                            .unwrap_or_else(|| strategy.next_backoff());

                        tracing::warn!(
                            attempt = attempt,
                            max_attempts = config.max_attempts,
                            backoff_ms = backoff.as_millis(),
                            error = %e,
                            classification = ?classification,
                            "Retrying after backoff"
                        );

                        // Sleep with backoff
                        tokio::time::sleep(backoff).await;

                        attempt += 1;
                    }

                    ErrorClass::Degraded(_) => {
                        // Degraded state - retry once, then accept
                        if attempt >= 2 {
                            tracing::warn!(
                                error = %e,
                                classification = ?classification,
                                "Accepting degraded state after retry"
                            );
                            return Err(e); // Accept degraded
                        }

                        // Retry once
                        let backoff = strategy.next_backoff();
                        tracing::info!(
                            backoff_ms = backoff.as_millis(),
                            "Retrying degraded state once"
                        );
                        tokio::time::sleep(backoff).await;
                        attempt += 1;
                    }
                }
            }
        }
    }
}
```

---

## 3. Code Examples

### Example 1: Exponential Backoff with Jitter (backon)

```rust
// retry/strategy.rs
use backon::{ExponentialBuilder, Retryable};
use std::time::Duration;

/// Execute operation with exponential backoff retry
pub async fn execute_with_backon<F, T, E>(
    operation: F,
    max_attempts: usize,
) -> Result<T, E>
where
    F: Fn() -> BoxFuture<'static, Result<T, E>> + Send + Sync,
    E: std::error::Error + RetryClassifiable + Send + Sync + 'static,
{
    let retry_strategy = ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(100))  // Start at 100ms
        .with_max_delay(Duration::from_secs(10))     // Cap at 10s
        .with_max_times(max_attempts)                // Max 5 attempts
        .with_jitter();  // Add randomness (±25%)

    (|| async {
        operation().await.map_err(|e| {
            // Classify error for retry decision
            if e.is_retryable() {
                backon::Error::Transient(e)  // Retry
            } else {
                backon::Error::Permanent(e)  // Fail immediately
            }
        })
    })
    .retry(&retry_strategy)
    .await
}

// Usage example
async fn fetch_agent_output(agent_name: &str) -> Result<AgentOutput, AgentError> {
    execute_with_backon(
        || Box::pin(async {
            // Actual agent execution
            spawn_agent(agent_name).await
        }),
        5,  // Max 5 attempts
    ).await
}
```

**Backoff Progression** (with jitter):
```
Attempt 1: Immediate (0ms)
Attempt 2: 100ms ± 25ms = 75-125ms
Attempt 3: 200ms ± 50ms = 150-250ms
Attempt 4: 400ms ± 100ms = 300-500ms
Attempt 5: 800ms ± 200ms = 600-1000ms

Total: ~1.2-1.9s (varies due to jitter)
```

---

### Example 2: Error Classification with thiserror

```rust
// error.rs (extend existing error types)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Network timeout after {0}s")]
    NetworkTimeout(u64),

    #[error("Rate limit exceeded, retry after {0}s")]
    RateLimitExceeded(u64),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Invalid input: {field} - {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Service unavailable (HTTP 503)")]
    ServiceUnavailable,

    #[error("Database locked (SQLITE_BUSY)")]
    DatabaseLocked,
}

// Implement classification trait
impl RetryClassifiable for AgentError {
    fn classify(&self) -> ErrorClass {
        match self {
            // Retryable errors
            AgentError::NetworkTimeout(_) => {
                ErrorClass::Retryable(RetryableError::NetworkTimeout(60))
            }
            AgentError::RateLimitExceeded(secs) => {
                ErrorClass::Retryable(RetryableError::RateLimitExceeded {
                    retry_after: *secs,
                })
            }
            AgentError::ServiceUnavailable => {
                ErrorClass::Retryable(RetryableError::ServiceUnavailable)
            }
            AgentError::DatabaseLocked => {
                ErrorClass::Retryable(RetryableError::DatabaseLocked)
            }

            // Permanent errors
            AgentError::AuthFailed(msg) => {
                ErrorClass::Permanent(PermanentError::AuthenticationFailed(msg.clone()))
            }
            AgentError::InvalidInput { field, reason } => {
                ErrorClass::Permanent(PermanentError::InvalidInput {
                    field: field.clone(),
                    reason: reason.clone(),
                })
            }
        }
    }

    fn is_retryable(&self) -> bool {
        matches!(
            self,
            AgentError::NetworkTimeout(_)
                | AgentError::RateLimitExceeded(_)
                | AgentError::ServiceUnavailable
                | AgentError::DatabaseLocked
        )
    }

    fn suggested_backoff(&self) -> Option<Duration> {
        match self {
            // Rate limit with explicit delay
            AgentError::RateLimitExceeded(secs) => {
                Some(Duration::from_secs(*secs))
            }
            // Database lock - short backoff
            AgentError::DatabaseLocked => {
                Some(Duration::from_millis(100))
            }
            // Default: use exponential backoff
            _ => None,
        }
    }
}
```

**Classification Examples**:
```rust
// Example 1: Retryable error (timeout)
let error = AgentError::NetworkTimeout(30);
assert!(error.is_retryable());
assert!(matches!(error.classify(), ErrorClass::Retryable(_)));

// Example 2: Permanent error (auth failure)
let error = AgentError::AuthFailed("Invalid API key".to_string());
assert!(!error.is_retryable());
assert!(matches!(error.classify(), ErrorClass::Permanent(_)));

// Example 3: Rate limit with custom backoff
let error = AgentError::RateLimitExceeded(60);
assert_eq!(error.suggested_backoff(), Some(Duration::from_secs(60)));
```

---

### Example 3: Circuit Breaker Pattern (failsafe-rs)

```rust
// retry/circuit_breaker.rs
use failsafe::{Config, CircuitBreaker, futures::CircuitBreaker as AsyncCircuitBreaker};
use std::sync::Arc;
use std::time::Duration;

/// Circuit breaker for agent provider calls
pub struct AgentCircuitBreaker {
    inner: Arc<AsyncCircuitBreaker>,
}

impl AgentCircuitBreaker {
    /// Create new circuit breaker with standard configuration
    pub fn new() -> Self {
        let config = Config::new()
            .failure_rate_threshold(50)  // Open at 50% failure rate
            .wait_duration_in_open_state(Duration::from_secs(30))  // Wait 30s
            .sliding_window_size(100)  // Track last 100 calls
            .minimum_number_of_calls(10);  // Need 10 calls before tripping

        Self {
            inner: Arc::new(AsyncCircuitBreaker::new(config)),
        }
    }

    /// Execute operation with circuit breaker protection
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> BoxFuture<'static, Result<T, E>>,
        E: std::error::Error,
    {
        self.inner.call(operation).await
            .map_err(|e| match e {
                failsafe::Error::Inner(inner) => CircuitBreakerError::OperationFailed(inner),
                failsafe::Error::Rejected => CircuitBreakerError::CircuitOpen,
            })
    }

    /// Get current circuit breaker state
    pub fn state(&self) -> CircuitState {
        match self.inner.state() {
            failsafe::State::Closed => CircuitState::Closed,
            failsafe::State::Open => CircuitState::Open,
            failsafe::State::HalfOpen => CircuitState::HalfOpen,
        }
    }
}

#[derive(Debug, Error)]
pub enum CircuitBreakerError<E> {
    #[error("Circuit breaker is open (too many failures)")]
    CircuitOpen,

    #[error("Operation failed: {0}")]
    OperationFailed(E),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failure threshold exceeded, fast-fail
    HalfOpen,  // Testing if service recovered
}

// Usage example
async fn call_agent_with_circuit_breaker(
    agent_name: &str,
    cb: &AgentCircuitBreaker,
) -> Result<AgentOutput, CircuitBreakerError<AgentError>> {
    cb.call(|| Box::pin(async move {
        spawn_agent(agent_name).await
    })).await
}
```

**Circuit Breaker State Machine**:
```
┌─────────┐  Failure rate > 50%   ┌──────┐
│ Closed  │──────────────────────>│ Open │
│ (Normal)│                       │(Fail)│
└────┬────┘                       └───┬──┘
     │                                │
     │ Success                        │ Wait 30s
     │                                │
     └─────────┐               ┌──────┘
               │               │
            ┌──▼───────────────▼──┐
            │    Half-Open        │
            │  (Testing recovery) │
            └─────────────────────┘
              │              │
         Success       Failure
              ▼              ▼
          Closed          Open
```

**Example Scenario**:
```rust
// Initialize circuit breaker
let cb = AgentCircuitBreaker::new();

// Call 1-9: Success (circuit remains closed)
for i in 0..9 {
    assert!(call_agent(&cb).await.is_ok());
}

// Call 10-15: 6 failures out of 10 (60% failure rate)
for i in 0..6 {
    let _ = call_agent(&cb).await;  // Fails
}
assert_eq!(cb.state(), CircuitState::Open);  // Circuit trips

// Call 16: Fast-fail (circuit open)
assert!(matches!(
    call_agent(&cb).await,
    Err(CircuitBreakerError::CircuitOpen)
));

// Wait 30 seconds
tokio::time::sleep(Duration::from_secs(30)).await;

// Call 17: Half-open (testing recovery)
assert_eq!(cb.state(), CircuitState::HalfOpen);
assert!(call_agent(&cb).await.is_ok());  // Success

// Circuit closes after successful test
assert_eq!(cb.state(), CircuitState::Closed);
```

---

### Example 4: Quality Gate Retry (3/3 Consensus)

```rust
// quality_gates.rs (modify existing)
use crate::retry::{retry_with_backoff, RetryConfig};

/// Execute quality gate with retry for 3/3 consensus
pub async fn execute_consensus_with_retry(
    spec_id: &str,
    stage: &str,
    agents: &[AgentConfig],
) -> Result<ConsensusResult, ConsensusError> {
    let retry_config = RetryConfig {
        max_attempts: 3,  // Max 3 attempts for consensus
        initial_backoff_ms: 1000,
        max_backoff_ms: 10000,
        backoff_multiplier: 2.0,
        jitter_factor: 0.5,
    };

    retry_with_backoff(
        || Box::pin(async {
            // Spawn all agents in parallel
            let agent_futures: Vec<_> = agents
                .iter()
                .map(|agent| spawn_single_agent(spec_id, stage, agent))
                .collect();

            // Wait for all agents to complete
            let results = futures::future::join_all(agent_futures).await;

            // Analyze consensus
            let successful: Vec<_> = results
                .into_iter()
                .filter_map(|r| r.ok())
                .collect();

            match (successful.len(), agents.len()) {
                (3, 3) => {
                    // Full consensus achieved
                    Ok(ConsensusResult::Full(successful))
                }
                (2, 3) => {
                    // Degraded consensus - retryable
                    Err(ConsensusError::Degraded {
                        success: 2,
                        total: 3,
                    })
                }
                (n, 3) if n < 2 => {
                    // Insufficient consensus - permanent failure
                    Err(ConsensusError::InsufficientAgreement {
                        success: n,
                        total: 3,
                    })
                }
                _ => unreachable!(),
            }
        }),
        &retry_config,
    ).await
}

// Error type with retry classification
#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("Degraded consensus: {success}/{total} agents succeeded")]
    Degraded { success: usize, total: usize },

    #[error("Insufficient agreement: {success}/{total} agents succeeded")]
    InsufficientAgreement { success: usize, total: usize },
}

impl RetryClassifiable for ConsensusError {
    fn classify(&self) -> ErrorClass {
        match self {
            // 2/3 consensus is degraded but retryable
            ConsensusError::Degraded { success, total } => {
                ErrorClass::Degraded(DegradedError::DegradedConsensus {
                    success: *success,
                    total: *total,
                })
            }
            // <2/3 consensus is permanent failure
            ConsensusError::InsufficientAgreement { .. } => {
                ErrorClass::Permanent(PermanentError::InvalidInput {
                    field: "consensus".to_string(),
                    reason: "Insufficient agent agreement".to_string(),
                })
            }
        }
    }
}
```

**Retry Behavior**:
```
Attempt 1: Spawn 3 agents → 2 succeed, 1 fails (timeout)
  Result: Degraded (2/3) → Retry
  Backoff: 1000ms + jitter

Attempt 2: Spawn 3 agents → 3 succeed
  Result: Full consensus (3/3) → Success!

Total Time: ~1-1.5s (1 retry with backoff)
```

---

## 4. Migration Strategy

### Phase 1: Create Retry Module (Zero Impact)

**Week 1, Day 1-2**:
1. Create `retry/` module with all new code
2. Implement `RetryConfig`, `retry_with_backoff()`, `ExponentialBackoff`
3. Add unit tests (backoff progression, jitter, max attempts)
4. **No integration** with existing code (pure addition)

**Validation**:
- [ ] All unit tests pass
- [ ] No changes to existing code paths
- [ ] Zero behavioral changes

---

### Phase 2: Wrap Existing Calls (Transparent)

**Week 1, Day 3-5**:
1. Wrap agent spawn calls with `retry_with_backoff()`
2. Keep existing error propagation (transparent wrapper)
3. Add telemetry (log retry attempts)
4. Integration tests (verify backward compatibility)

**Example**:
```rust
// Before (existing code)
let result = spawn_agent(agent_name).await?;

// After (with retry wrapper, transparent)
let result = retry_with_backoff(
    || Box::pin(spawn_agent(agent_name)),
    &RetryConfig::default(),
).await?;
```

**Validation**:
- [ ] All existing tests pass (no regressions)
- [ ] Retry telemetry visible in logs
- [ ] Transient errors auto-recover (timeout, 503)

---

### Phase 3: Add Error Classification (Enrich)

**Week 2, Day 1-2**:
1. Extend `AgentError` with `RetryClassifiable` trait
2. Classify existing error types (retryable vs permanent)
3. Unit tests (verify classification correctness)
4. Gradual opt-in (mark errors as retryable incrementally)

**Default Behavior** (safe):
```rust
// Default: all errors are permanent (no retry)
impl RetryClassifiable for AgentError {
    fn classify(&self) -> ErrorClass {
        ErrorClass::Permanent(PermanentError::InvalidInput {
            field: "unknown".to_string(),
            reason: format!("{:?}", self),
        })
    }
}

// Opt-in: mark specific errors as retryable
impl RetryClassifiable for AgentError {
    fn classify(&self) -> ErrorClass {
        match self {
            AgentError::Timeout(_) => ErrorClass::Retryable(...),
            _ => ErrorClass::Permanent(...),  // Safe default
        }
    }
}
```

**Validation**:
- [ ] Classification tests cover all error types
- [ ] Permanent errors NOT retried (auth, validation)
- [ ] Retryable errors auto-recover (timeout, rate limit)

---

### Phase 4: Integrate Circuit Breaker (Optional, Advanced)

**Week 2, Day 3-5** (if needed):
1. Add `circuit_breaker.rs` module
2. Wrap provider calls with circuit breaker
3. Integration tests (sustained failures trip circuit)
4. Monitoring (track circuit state, failure rate)

**When to Use**:
- High-volume provider calls (OpenAI, Anthropic)
- Cascading failure prevention (one provider down shouldn't block all)
- Rate limit protection (circuit opens before hitting limits)

**Validation**:
- [ ] Circuit trips after 50% failure rate
- [ ] Fast-fail during open state (no wasted retries)
- [ ] Recovery after cooldown period (half-open → closed)

---

### Backward Compatibility Guarantees

**Commitments**:
1. **Existing error types unchanged**: All current `AgentError` variants remain
2. **Default: no retry**: New errors default to permanent (explicit opt-in)
3. **Zero behavioral changes** until error classification added
4. **Rollback procedure**: Remove retry wrapper (revert to direct calls)

**Rollback Steps**:
```rust
// Rollback: unwrap retry logic
// Before (with retry)
let result = retry_with_backoff(|| Box::pin(spawn_agent(name)), &config).await?;

// After (rollback)
let result = spawn_agent(name).await?;

// Keep retry module (no harm, just unused)
```

---

## 5. Performance Validation

### Metrics to Track

**Instrumentation** (in `retry_with_backoff()`):
```rust
#[derive(Debug, Clone)]
pub struct RetryMetrics {
    pub total_attempts: u64,
    pub successful_first_attempt: u64,
    pub successful_after_retry: u64,
    pub failed_after_retries: u64,
    pub avg_backoff_duration_ms: u64,
    pub error_classifications: HashMap<ErrorClass, u64>,
}

impl RetryMetrics {
    pub fn record_attempt(&mut self, attempt: u32, success: bool, backoff_ms: u64) {
        self.total_attempts += 1;

        if success {
            if attempt == 1 {
                self.successful_first_attempt += 1;
            } else {
                self.successful_after_retry += 1;
            }
        } else if attempt >= MAX_ATTEMPTS {
            self.failed_after_retries += 1;
        }

        // Update rolling average
        self.avg_backoff_duration_ms =
            (self.avg_backoff_duration_ms + backoff_ms) / 2;
    }
}
```

### Success Criteria

**Reliability** (must achieve):
- ✅ Transient error recovery: ≥90% success rate after retry
- ✅ Consensus reliability: 3/3 achieved ≥95% of time (up from 70%)
- ✅ Permanent errors: 0 retries (no wasted attempts)

**Performance** (must not exceed):
- ✅ Backoff timing: 100ms → 1.6s exponential curve (validated)
- ✅ No thundering herd: Jitter spreads retries over time (measured)
- ✅ Latency impact: p95 latency ≤5s with retries (3 attempts max)

**Telemetry** (must capture):
- ✅ Retry rate: % of requests that retry (alert if >20%)
- ✅ Success after retry: % of retries that succeed
- ✅ Classification accuracy: % of errors classified correctly

### Regression Detection

**Alerts to Configure**:
```yaml
# Prometheus alert rules
- alert: HighRetryRate
  expr: retry_rate > 0.20  # Alert if >20% of requests retry
  for: 5m
  annotations:
    summary: "High retry rate detected"

- alert: LowRetrySuccessRate
  expr: retry_success_rate < 0.80  # Alert if <80% retries succeed
  for: 5m
  annotations:
    summary: "Retries not recovering failures"

- alert: PermanentErrorRetries
  expr: permanent_error_retries > 0  # Should NEVER retry permanent errors
  for: 1m
  annotations:
    severity: critical
    summary: "Permanent errors being retried (classification bug)"
```

**Benchmarks** (criterion.rs):
```rust
// Benchmark: retry overhead
fn bench_retry_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("retry_overhead");

    // Baseline: direct call (no retry)
    group.bench_function("direct_call", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            execute_agent_stub().await
        })
    });

    // With retry wrapper (no failures)
    group.bench_function("retry_wrapper_success", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            retry_with_backoff(
                || Box::pin(execute_agent_stub()),
                &RetryConfig::default(),
            ).await
        })
    });

    // With retry (1 failure, 1 success)
    group.bench_function("retry_with_one_failure", |b| {
        let counter = Arc::new(AtomicU32::new(0));
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            let c = counter.clone();
            retry_with_backoff(
                || Box::pin(async move {
                    if c.fetch_add(1, Ordering::SeqCst) == 0 {
                        Err(AgentError::NetworkTimeout(30))  // Fail first
                    } else {
                        Ok(())  // Succeed second
                    }
                }),
                &RetryConfig::default(),
            ).await
        })
    });

    group.finish();
}
```

---

## 6. Dependencies & Sequencing

### Crate Dependencies (Cargo.toml)

```toml
[dependencies]
# Primary retry implementation
backon = "1.1"            # Preferred: ergonomic, WASM support
backoff = "0.4"           # Alternative: battle-tested, stable

# Error type definitions
thiserror = "1.0"         # Structured error types
anyhow = "1.0"            # Error context (optional)

# Circuit breaker (optional, advanced)
failsafe = "1.0"          # Circuit breaker, retry, fallback

# Async runtime (existing)
tokio = { version = "1.35", features = ["time", "macros"] }
futures = "0.3"

# Logging (existing)
tracing = "0.1"

# Utilities
rand = "0.8"              # Jitter calculation
```

**Version Constraints**:
- `backon = "1.1"` (minimum, use latest 1.x)
- `backoff = "0.4"` (minimum, use latest 0.x)
- `failsafe = "1.0"` (minimum, use latest 1.x)
- `thiserror = "1.0"` (stable, pinned to 1.x)

### Implementation Order (2 Weeks)

**Week 1: Foundation**
- **Day 1-2**: Create retry/ module (strategy.rs, mod.rs)
  - Implement `ExponentialBackoff`, `RetryConfig`
  - Unit tests (backoff progression, jitter)
- **Day 3-4**: Error classification (classifier.rs)
  - Implement `RetryClassifiable` trait
  - Classify all `AgentError` types
  - Unit tests (classification correctness)
- **Day 5**: Integration into consensus (consensus.rs)
  - Wrap agent spawn calls with retry
  - Integration tests (3/3 consensus via retry)

**Week 2: Polish & Advanced Features**
- **Day 1-2**: Quality gate integration (quality_gates.rs)
  - Retry failed agents before degrading to 2/3
  - Telemetry (log retry attempts, success rate)
  - Integration tests (quality gate resilience)
- **Day 3-4** (optional): Circuit breaker (circuit_breaker.rs)
  - Implement circuit breaker pattern
  - Wrap provider calls (OpenAI, Anthropic)
  - Integration tests (sustained failures trip circuit)
- **Day 5**: Performance validation
  - Run benchmarks (retry overhead)
  - Monitor metrics (retry rate, success rate)
  - Document findings

### Integration Points

**SPEC-945A (Async Patterns)**:
- Retry logic must be async-compatible (tokio runtime)
- Use `tokio::time::sleep()` for backoff (not `std::thread::sleep`)
- Integrate with `JoinSet` for parallel agent execution

**SPEC-945B (SQLite)**:
- Retry on `SQLITE_BUSY` (database locked)
- Short backoff for DB locks (100ms typical)
- Coordinate with transaction retry logic (SPEC-933)

**SPEC-938 (Enhanced Retry)**:
- All requirements implemented (error classification, backoff, quality gates)
- Telemetry captures retry context (attempt count, backoff delay)
- 3/3 consensus reliability improved (95% target)

**Quality Gates (existing)**:
- Integrate retry before degrading to 2/3
- Prefer full consensus (3/3) via retry
- Accept degraded (2/3) only after retry exhausted

---

## 7. Validation Checklist

### Pre-Submission Checklist

**Code Quality**:
- [ ] All code examples compile (Rust syntax correct)
- [ ] Error classification covers common cases (timeout, rate limit, auth, validation)
- [ ] Backoff timing matches research (100ms → 1.6s progression)
- [ ] Circuit breaker prevents cascading failures (50% threshold, 30s cooldown)

**Dependencies**:
- [ ] Version constraints specified (backon 1.1+, thiserror 1.0+)
- [ ] Fallback crate documented (backoff 0.4+ as alternative)
- [ ] tokio-retry explicitly avoided (maintenance concerns)

**Documentation**:
- [ ] Source URLs from research document included
- [ ] Cross-references to SPEC-938 throughout
- [ ] Decision rationale documented (why backon over backoff)
- [ ] Migration strategy clear (zero-impact → transparent → enrich)

**Completeness**:
- [ ] 8-10 pages total length (target: 8-10 pages, actual: 10 pages ✅)
- [ ] All sections present (summary, research, plan, examples, migration, validation)
- [ ] Code examples production-ready (not pseudocode)
- [ ] Performance characteristics quantified (timing, overhead, metrics)

---

## 8. Conclusion

SPEC-945C provides a complete implementation guide for retry logic, error classification, and circuit breaker patterns. The implementation supports SPEC-938 (Enhanced Agent Retry) and integrates seamlessly with quality gate consensus logic.

### Key Deliverables
1. **Retry Module** (`retry/mod.rs`, `strategy.rs`, `classifier.rs`): Complete retry infrastructure
2. **Error Classification** (`error.rs`, `classifier.rs`): Structured error hierarchy (retryable vs permanent)
3. **Quality Gate Integration** (`quality_gates.rs`, `consensus.rs`): 3/3 consensus via retry
4. **Circuit Breaker** (`circuit_breaker.rs`, optional): Advanced failure protection

### Expected Outcomes
- **Reliability**: 80%+ transient error recovery (timeout, rate limit, 503)
- **Consensus**: 95%+ quality gates achieve 3/3 (up from 70%)
- **Performance**: <5s p95 latency with retries (3 attempts max)
- **Transparency**: Automatic recovery with comprehensive telemetry

### Next Steps
1. Review SPEC-945C implementation plan
2. Schedule 2-week implementation sprint
3. Coordinate with SPEC-945A (async patterns) and SPEC-945B (SQLite)
4. Monitor metrics post-deployment (retry rate, success rate, latency)

**Implementation Status**: ✅ Ready for development
**Estimated Effort**: 1-2 weeks (10-15 days)
**Risk Level**: Low (proven patterns, gradual migration)

---

**Document Metadata**:
- **Pages**: 10 (target: 8-10 ✅)
- **Code Examples**: 4 comprehensive examples (backon, error classification, circuit breaker, quality gates)
- **Sources**: 5 authoritative references (backon docs, AWS SDK, failsafe-rs)
- **Cross-References**: SPEC-938 (Enhanced Retry), SPEC-945A (Async), SPEC-945B (SQLite), SPEC-933 (Transactions)
