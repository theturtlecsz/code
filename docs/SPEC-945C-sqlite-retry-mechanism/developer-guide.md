# Retry Logic Developer Guide

**Audience**: Developers adding retry logic to new operations or modifying existing retry behavior
**Prerequisites**: Understanding of Rust error handling, async/await, SQLite basics
**Related Docs**: `implementation.md` (architectural overview), `SPEC-945C-retry-error-handling.md` (original spec)

---

## Quick Start: Adding Retry to New Operations

### For Synchronous Operations (rusqlite, file I/O)

```rust
use codex_spec_kit::retry::{execute_with_backoff_sync, RetryConfig, RetryClassifiable};

pub fn my_database_operation(&self) -> Result<(), MyError> {
    let config = RetryConfig {
        max_attempts: 3,
        initial_backoff_ms: 100,
        backoff_multiplier: 1.5,
        max_backoff_ms: 2000,
        jitter_factor: 0.5,
    };

    execute_with_backoff_sync(
        || {
            // Your operation here (must return Result<T, E>)
            let conn = self.get_connection()?;
            conn.execute("INSERT INTO ...", params![...])?;
            Ok(())
        },
        &config,
    )
}
```

**Key Points**:
- Operation closure is `Fn() -> Result<T, E>` (can be called multiple times)
- Closure must NOT have side effects that can't be retried (use idempotent operations)
- Error type `E` must implement `RetryClassifiable` trait

---

### For Async Operations (tokio runtime)

```rust
use codex_spec_kit::retry::{execute_with_backoff, RetryConfig};
use futures::future::BoxFuture;

pub async fn my_async_operation(&self, data: String) -> Result<(), MyError> {
    let config = RetryConfig {
        max_attempts: 5,
        initial_backoff_ms: 100,
        backoff_multiplier: 1.5,
        max_backoff_ms: 5000,
        jitter_factor: 0.5,
    };

    execute_with_backoff(
        || {
            let data = data.clone(); // Clone for each attempt
            Box::pin(async move {
                // Your async operation here
                perform_network_request(&data).await?;
                Ok(())
            })
        },
        &config,
    )
    .await
}
```

**Key Points**:
- Operation closure returns `BoxFuture<'static, Result<T, E>>`
- Must clone captured variables (closure called multiple times)
- Use `Box::pin(async move { ... })` for the future

---

## Error Classification Guide

### Decision Tree

```
┌─────────────────────────────────────────────────────────┐
│ Step 1: Is the error transient (temporary condition)?  │
│                                                          │
│ YES: Network timeout, database lock, rate limit         │
│  → ErrorClass::Retryable                               │
│                                                          │
│ NO: Go to Step 2                                        │
└──────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│ Step 2: Is the error permanent (no point in retrying)? │
│                                                          │
│ YES: Auth failure, validation error, not found          │
│  → ErrorClass::Permanent                               │
│                                                          │
│ NO: Go to Step 3                                        │
└──────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│ Step 3: Is partial success acceptable?                 │
│                                                          │
│ YES: 2/3 consensus, partial data                        │
│  → ErrorClass::Degraded                                │
│                                                          │
│ NO: Default to Permanent (safe)                         │
└──────────────────────────────────────────────────────────┘
```

---

### Implementation Checklist

**Step 1**: Define error type with thiserror

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("Database locked (SQLITE_BUSY)")]
    DatabaseBusy,

    #[error("Network timeout after {0}s")]
    NetworkTimeout(u64),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
```

---

**Step 2**: Implement `RetryClassifiable` trait

```rust
use codex_spec_kit::retry::{
    RetryClassifiable, ErrorClass, RetryableError, PermanentError
};
use std::time::Duration;

impl RetryClassifiable for MyError {
    fn classify(&self) -> ErrorClass {
        match self {
            // Retryable errors (transient)
            MyError::DatabaseBusy => {
                ErrorClass::Retryable(RetryableError::DatabaseError(
                    "SQLITE_BUSY".to_string(),
                ))
            }
            MyError::NetworkTimeout(secs) => {
                ErrorClass::Retryable(RetryableError::NetworkTimeout(*secs))
            }

            // Permanent errors (no retry)
            MyError::AuthFailed(msg) => {
                ErrorClass::Permanent(PermanentError::AuthenticationFailed(
                    msg.clone(),
                ))
            }
            MyError::InvalidInput(reason) => {
                ErrorClass::Permanent(PermanentError::InvalidInput {
                    field: "input".to_string(),
                    reason: reason.clone(),
                })
            }
        }
    }

    fn suggested_backoff(&self) -> Option<Duration> {
        match self {
            // Custom backoff for database locks
            MyError::DatabaseBusy => Some(Duration::from_millis(100)),

            // Network timeouts may need longer backoff
            MyError::NetworkTimeout(_) => Some(Duration::from_secs(1)),

            // Default: use exponential backoff
            _ => None,
        }
    }
}
```

---

**Step 3**: Write classification tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_busy_retryable() {
        let err = MyError::DatabaseBusy;
        assert!(matches!(err.classify(), ErrorClass::Retryable(_)));
        assert_eq!(err.suggested_backoff(), Some(Duration::from_millis(100)));
    }

    #[test]
    fn test_auth_failure_permanent() {
        let err = MyError::AuthFailed("invalid key".to_string());
        assert!(matches!(err.classify(), ErrorClass::Permanent(_)));
        assert_eq!(err.suggested_backoff(), None); // No backoff
    }

    #[test]
    fn test_network_timeout_retryable() {
        let err = MyError::NetworkTimeout(30);
        assert!(matches!(err.classify(), ErrorClass::Retryable(_)));
        assert_eq!(err.suggested_backoff(), Some(Duration::from_secs(1)));
    }
}
```

---

## Configuration Guidelines

### Choosing max_attempts

| Operation Type          | Recommended | Rationale                                    |
|-------------------------|-------------|----------------------------------------------|
| Critical writes         | 5-10        | Ensure durability, tolerate longer retries   |
| Standard writes         | 3-5         | Balance retry effort vs latency             |
| Reads                   | 2-3         | Fail faster, reads less critical            |
| Idempotent operations   | 5+          | Safe to retry many times                    |
| Non-idempotent writes   | 3           | Minimize duplicate risk                     |

**Example**:
```rust
// Critical consensus data (must not lose)
let critical_config = RetryConfig {
    max_attempts: 10,
    // ... other params
};

// Read-heavy query (fail fast)
let read_config = RetryConfig {
    max_attempts: 2,
    // ... other params
};
```

---

### Choosing Backoff Parameters

#### initial_backoff_ms

| Resource Type    | Recommended | Rationale                                  |
|------------------|-------------|--------------------------------------------|
| Database locks   | 50-100ms    | Short contention windows                   |
| Network requests | 500-1000ms  | Allow network recovery                     |
| File locks       | 100-200ms   | File system contention                     |
| API rate limits  | 1000-5000ms | Respect rate limit windows                 |

---

#### backoff_multiplier

| Strategy          | Multiplier | Progression Example (100ms initial)          |
|-------------------|------------|----------------------------------------------|
| Conservative      | 1.5x       | 100 → 150 → 225 → 337ms (slow growth)      |
| Standard          | 2.0x       | 100 → 200 → 400 → 800ms (exponential)      |
| Aggressive        | 3.0x       | 100 → 300 → 900 → 2700ms (rapid backoff)   |

**Recommendation**: Use 1.5x for database operations, 2.0x for network operations

---

#### max_backoff_ms

| Resource Type    | Recommended | Rationale                                  |
|------------------|-------------|--------------------------------------------|
| Database locks   | 2000-5000ms | Prevent indefinite waiting                 |
| Network requests | 10000-30000ms| Allow service recovery                    |
| File locks       | 1000-2000ms | File system should recover quickly         |

---

#### jitter_factor

| Strategy          | Factor | Description                                  |
|-------------------|--------|----------------------------------------------|
| No jitter         | 0.0    | Deterministic backoff (testing only)         |
| Light jitter      | 0.3    | ±30% randomness, mild spread                |
| **Recommended**   | 0.5    | ±50% randomness, good spread                |
| Heavy jitter      | 1.0    | ±100% randomness, maximum spread            |

**Why jitter matters**: Prevents "thundering herd" where all clients retry simultaneously

**Example**:
```rust
// With jitter_factor = 0.5 and backoff = 100ms:
// Actual delay: 100ms ± 50ms = 50-150ms (random)
```

---

### Configuration Presets

#### Database Write (Conservative)

```rust
pub const DB_WRITE_CONFIG: RetryConfig = RetryConfig {
    max_attempts: 5,
    initial_backoff_ms: 100,
    backoff_multiplier: 1.5,
    max_backoff_ms: 5000,
    jitter_factor: 0.5,
};

// Backoff progression: 100 → 150 → 225 → 337 → 506ms
// Total time (max): ~1.3s
```

---

#### Database Read (Aggressive)

```rust
pub const DB_READ_CONFIG: RetryConfig = RetryConfig {
    max_attempts: 3,
    initial_backoff_ms: 50,
    backoff_multiplier: 2.0,
    max_backoff_ms: 2000,
    jitter_factor: 0.5,
};

// Backoff progression: 50 → 100 → 200ms
// Total time (max): ~350ms
```

---

#### Network Request (Moderate)

```rust
pub const NETWORK_CONFIG: RetryConfig = RetryConfig {
    max_attempts: 5,
    initial_backoff_ms: 1000,
    backoff_multiplier: 2.0,
    max_backoff_ms: 30000,
    jitter_factor: 0.5,
};

// Backoff progression: 1s → 2s → 4s → 8s → 16s
// Total time (max): ~31s
```

---

## Testing Patterns

### Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_retry_success_after_failure() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = execute_with_backoff_sync(
            || {
                let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    // Fail first 2 attempts
                    Err(MyError::DatabaseBusy)
                } else {
                    // Succeed on 3rd attempt
                    Ok(42)
                }
            },
            &RetryConfig {
                max_attempts: 5,
                initial_backoff_ms: 10, // Fast tests
                backoff_multiplier: 1.5,
                max_backoff_ms: 100,
                jitter_factor: 0.0, // Deterministic for testing
            },
        );

        assert_eq!(result, Ok(42));
        assert_eq!(call_count.load(Ordering::SeqCst), 3); // 2 failures + 1 success
    }

    #[test]
    fn test_permanent_error_no_retry() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = execute_with_backoff_sync(
            || {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                Err(MyError::AuthFailed("invalid".to_string()))
            },
            &RetryConfig {
                max_attempts: 5,
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1); // No retries
    }

    #[test]
    fn test_max_attempts_exhausted() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = execute_with_backoff_sync(
            || {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                Err(MyError::DatabaseBusy) // Always fail
            },
            &RetryConfig {
                max_attempts: 3,
                initial_backoff_ms: 10,
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 3); // Max attempts
    }
}
```

---

### Integration Test Template

```rust
#[tokio::test]
async fn test_concurrent_database_writes_with_retry() {
    use tokio::task::JoinSet;

    // Setup: Create database connection pool
    let db = Database::new(":memory:").await.unwrap();

    // Spawn 10 concurrent writers
    let mut tasks = JoinSet::new();
    for i in 0..10 {
        let db_clone = db.clone();
        tasks.spawn(async move {
            db_clone.write_with_retry(&format!("data_{}", i)).await
        });
    }

    // Wait for all tasks to complete
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap().is_ok() {
            success_count += 1;
        }
    }

    // All writes should succeed (retry handles contention)
    assert_eq!(success_count, 10);
}
```

---

## Common Pitfalls

### ❌ Pitfall 1: Retrying Non-Idempotent Operations

**Problem**: Operation has side effects that can't be safely retried

```rust
// BAD: Sends email on every retry
execute_with_backoff_sync(
    || {
        send_email("user@example.com", "Welcome!")?; // Side effect!
        update_database()?;
        Ok(())
    },
    &config,
)
```

**Solution**: Make operation idempotent or split into retriable/non-retriable parts

```rust
// GOOD: Check if email already sent (idempotent)
execute_with_backoff_sync(
    || {
        if !email_sent_before("user@example.com")? {
            send_email("user@example.com", "Welcome!")?;
            mark_email_sent("user@example.com")?;
        }
        Ok(())
    },
    &config,
)
```

---

### ❌ Pitfall 2: Holding Locks Across Retries

**Problem**: Lock held during retry delays causes deadlock

```rust
// BAD: Lock held across retries
let guard = mutex.lock().unwrap(); // Lock acquired
execute_with_backoff_sync(
    || {
        perform_operation(&guard)?; // Lock still held during retries
        Ok(())
    },
    &config,
)
```

**Solution**: Acquire lock inside retry operation

```rust
// GOOD: Lock acquired per attempt
execute_with_backoff_sync(
    || {
        let guard = mutex.lock().unwrap(); // Fresh lock per attempt
        perform_operation(&guard)?;
        drop(guard); // Release immediately
        Ok(())
    },
    &config,
)
```

---

### ❌ Pitfall 3: Ignoring Timeout Budgets

**Problem**: Total retry time exceeds caller's timeout

```rust
// BAD: May retry for 30+ seconds
execute_with_backoff_sync(
    || expensive_operation(),
    &RetryConfig {
        max_attempts: 10,
        initial_backoff_ms: 1000,
        backoff_multiplier: 2.0,
        max_backoff_ms: 30000, // 30 second max!
        jitter_factor: 0.5,
    },
)
```

**Solution**: Consider total timeout budget

```rust
// GOOD: Cap total time at ~5s
execute_with_backoff_sync(
    || expensive_operation(),
    &RetryConfig {
        max_attempts: 5,
        initial_backoff_ms: 500,
        backoff_multiplier: 1.5,
        max_backoff_ms: 2000, // Reasonable cap
        jitter_factor: 0.5,
    },
)
// Total: 500 + 750 + 1125 + 1687 + 2000 = ~6s max
```

---

### ❌ Pitfall 4: Classifying All Errors as Retryable

**Problem**: Permanent errors get retried (waste resources)

```rust
// BAD: Default to retryable
impl RetryClassifiable for MyError {
    fn classify(&self) -> ErrorClass {
        ErrorClass::Retryable(RetryableError::NetworkTimeout(30))
    }
}
```

**Solution**: Default to permanent (safe), opt-in to retryable

```rust
// GOOD: Explicit classification
impl RetryClassifiable for MyError {
    fn classify(&self) -> ErrorClass {
        match self {
            MyError::Timeout(_) => ErrorClass::Retryable(...),
            MyError::AuthFailed(_) => ErrorClass::Permanent(...),
            // Safe default: permanent
            _ => ErrorClass::Permanent(PermanentError::InvalidInput {
                field: "unknown".to_string(),
                reason: format!("{:?}", self),
            }),
        }
    }
}
```

---

## Advanced Patterns

### Pattern 1: Retry with Timeout Budget

```rust
use tokio::time::{timeout, Duration};

pub async fn operation_with_timeout() -> Result<(), MyError> {
    // Total budget: 5 seconds
    timeout(
        Duration::from_secs(5),
        execute_with_backoff(
            || Box::pin(async { perform_operation().await }),
            &RetryConfig {
                max_attempts: 10, // May not reach 10 due to timeout
                initial_backoff_ms: 100,
                backoff_multiplier: 2.0,
                max_backoff_ms: 1000,
                jitter_factor: 0.5,
            },
        ),
    )
    .await
    .map_err(|_| MyError::TimeoutExceeded)?
    .map_err(|e| MyError::from(e))
}
```

---

### Pattern 2: Conditional Retry Based on Context

```rust
pub fn write_with_context(
    &self,
    data: &str,
    is_critical: bool,
) -> Result<(), MyError> {
    let config = if is_critical {
        // Critical: more retries
        RetryConfig {
            max_attempts: 10,
            initial_backoff_ms: 100,
            backoff_multiplier: 1.5,
            max_backoff_ms: 5000,
            jitter_factor: 0.5,
        }
    } else {
        // Non-critical: fewer retries
        RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 50,
            backoff_multiplier: 2.0,
            max_backoff_ms: 1000,
            jitter_factor: 0.5,
        }
    };

    execute_with_backoff_sync(
        || self.perform_write(data),
        &config,
    )
}
```

---

### Pattern 3: Retry with Fallback

```rust
pub async fn operation_with_fallback() -> Result<Data, MyError> {
    // Try primary operation with retry
    let result = execute_with_backoff(
        || Box::pin(async { fetch_from_primary_source().await }),
        &RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 1000,
            jitter_factor: 0.5,
        },
    )
    .await;

    match result {
        Ok(data) => Ok(data),
        Err(_) => {
            // Primary failed, try fallback (no retry)
            fetch_from_fallback_source().await
        }
    }
}
```

---

## Quick Reference Card

### Sync Retry

```rust
use codex_spec_kit::retry::{execute_with_backoff_sync, RetryConfig};

execute_with_backoff_sync(
    || /* operation */,
    &RetryConfig { /* config */ },
)
```

---

### Async Retry

```rust
use codex_spec_kit::retry::{execute_with_backoff, RetryConfig};

execute_with_backoff(
    || Box::pin(async { /* operation */ }),
    &RetryConfig { /* config */ },
).await
```

---

### Error Classification

```rust
impl RetryClassifiable for MyError {
    fn classify(&self) -> ErrorClass {
        match self {
            MyError::Transient => ErrorClass::Retryable(...),
            MyError::Permanent => ErrorClass::Permanent(...),
            MyError::Partial => ErrorClass::Degraded(...),
        }
    }

    fn suggested_backoff(&self) -> Option<Duration> {
        // Optional: custom backoff per error type
        None // Use exponential backoff
    }
}
```

---

### Configuration Presets

| Type      | max_attempts | initial_backoff | multiplier | max_backoff | jitter |
|-----------|--------------|-----------------|------------|-------------|--------|
| DB Write  | 5            | 100ms           | 1.5x       | 5000ms      | 0.5    |
| DB Read   | 3            | 50ms            | 2.0x       | 2000ms      | 0.5    |
| Network   | 5            | 1000ms          | 2.0x       | 30000ms     | 0.5    |
| File Lock | 3            | 100ms           | 2.0x       | 1000ms      | 0.5    |

---

## Additional Resources

- **Implementation Guide**: `implementation.md` - Architecture and design decisions
- **Original Spec**: `docs/SPEC-KIT-945-implementation-research/SPEC-945C-retry-error-handling.md`
- **Test Examples**: `codex-rs/spec-kit/src/retry/strategy.rs` (18 tests)
- **Real-World Usage**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs` (11 wrapped operations)

---

## Document Metadata

- **Created**: 2025-11-14
- **Version**: 1.0
- **Target Audience**: Rust developers working on spec-kit
- **Pages**: 8
- **Code Examples**: 20+
