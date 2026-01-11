# PRD: Enhanced Agent Retry Logic

**SPEC-ID**: SPEC-KIT-938
**Created**: 2025-11-13
**Status**: Draft - **MEDIUM PRIORITY**
**Priority**: **P2** (Reliability + User Experience)
**Owner**: Code
**Estimated Effort**: 4-6 hours (1 day)
**Dependencies**: None (enhances existing AR-2/3/4 retry implementation)
**Blocks**: None

---

## 🔥 Executive Summary

**Current State**: Basic retry logic exists (AR-2/3/4 tasks added foundational retry). Agents that fail due to transient errors (timeout, 429 rate limit, 503 service unavailable) treated as permanent failures. No exponential backoff (immediate retry). Quality gates don't retry failed agents, just mark as degraded and continue with 2/3 consensus.

**Proposed State**: Enhanced retry logic with error classification (retryable vs permanent), exponential backoff with jitter (avoid thundering herd), max retry limits (3 attempts then fail), and quality gate integration (retry within gate, not re-spawn entire gate). Comprehensive telemetry (log retry attempts, success rate, backoff delays).

**Impact**:
- ✅ Higher reliability (transient failures auto-recover)
- ✅ Better user experience (fewer manual retries)
- ✅ Smarter retry strategy (exponential backoff prevents API overload)
- ✅ Quality gate resilience (2/3 → 3/3 consensus via retry)

**Source**: SPEC-931A architectural analysis (Q43 answered in QUESTION-CONSOLIDATION-ANALYSIS.md).

**Alternative Rejected**: Infinite retry (NO-GO - could cause infinite loops, max 3 attempts sufficient).

---

## 1. Problem Statement

### Issue #1: Transient Failures Treated as Permanent (HIGH)

**Current Behavior** (agent_orchestrator.rs):
```rust
async fn execute_agent(agent_name: &str) -> Result<AgentResponse> {
    let response = agent_cli.execute().await?;

    // Any error = permanent failure
    match response {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::error!("Agent {} failed: {}", agent_name, e);
            return Err(e); // No retry
        }
    }
}
```

**Transient Error Examples**:
- **Timeout**: Network latency spike (temporary)
- **Rate Limit (429)**: API quota exceeded (wait and retry)
- **Service Unavailable (503)**: Provider temporary outage (retry after backoff)
- **Network Error**: DNS resolution failure (temporary)

**Impact**:
- User sees "Agent failed: Timeout" → Must manually re-run quality gate
- Wasted time: User could have been working on other tasks while auto-retry happens
- Degraded consensus: 2/3 agents succeed (acceptable but suboptimal)

**Frequency**: ~5-10% of agent executions encounter transient errors (based on provider SLA: 99.9% uptime = 0.1% errors, plus network variance).

---

### Issue #2: AR-2/3/4 Retry Lacks Sophistication (MEDIUM)

**Existing Implementation** (AR-2/3/4 tasks):
```rust
// AR-2: Basic retry on agent failure (up to 3 attempts)
for attempt in 1..=3 {
    match spawn_agent(agent_name).await {
        Ok(result) => return Ok(result),
        Err(e) => {
            if attempt == 3 {
                return Err(e); // Final failure
            }
            // Immediate retry (no backoff)
        }
    }
}
```

**Gaps**:
1. **No Error Classification**: Retries permanent errors (e.g., invalid API key) wastefully
2. **No Backoff**: Immediate retry can trigger rate limiting (make problem worse)
3. **No Jitter**: All agents retry simultaneously (thundering herd)
4. **No Telemetry**: No visibility into retry attempts, success rate, failure patterns

**Example Problem** (thundering herd):
```
T=0s: 3 agents hit rate limit (429)
T=0.1s: All 3 retry immediately
T=0.1s: All 3 hit rate limit again (429)
T=0.2s: All 3 retry immediately
... infinite loop until rate limit resets (60s later)
```

**Better Approach** (exponential backoff with jitter):
```
T=0s: 3 agents hit rate limit (429)
T=1s + rand(0-0.5s): Agent 1 retries (1s backoff)
T=2s + rand(0-1s): Agent 2 retries (2s backoff)
T=4s + rand(0-2s): Agent 3 retries (4s backoff)
... staggered retries avoid thundering herd
```

---

### Issue #3: Quality Gates Don't Retry (MEDIUM)

**Current Behavior** (quality_gate_handler.rs):
```rust
// Spawn 3 agents (gemini, claude, code)
let results = spawn_all_agents(["gemini", "claude", "code"]).await;

// Auto-resolution: 2/3 agents succeed → consensus OK
if successful_agents >= 2 {
    return Ok(Consensus::Approved);
}

// If only 1 agent succeeds → manual intervention
return Err("Insufficient agents for consensus");
```

**Problem**:
- Agent fails due to timeout → Quality gate continues with 2/3 (degraded)
- User doesn't know failure was retryable
- No attempt to recover 3/3 consensus

**Better Approach**:
```rust
// Retry failed agent before degrading to 2/3
if failed_agents.len() > 0 {
    for agent in failed_agents {
        if is_retryable(&agent.error) {
            match retry_agent_with_backoff(&agent).await {
                Ok(result) => {
                    successful_agents.push(result);
                }
                Err(e) => {
                    tracing::warn!("Retry failed: {}", e);
                }
            }
        }
    }
}

// Re-check consensus after retries
if successful_agents >= 3 {
    return Ok(Consensus::Approved); // Full 3/3 consensus
}
```

---

## 2. Proposed Solution

### Component 1: Error Classification (CRITICAL - 1-2h)

**Implementation**:
```rust
// error_classifier.rs
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    Retryable,       // Timeout, rate limit, service unavailable
    Permanent,       // Invalid API key, model not found
    Unknown,         // Unclear errors (treat as retryable once)
}

pub fn classify_agent_error(error: &AgentError) -> ErrorCategory {
    match error {
        // Retryable errors
        AgentError::Timeout(_) => ErrorCategory::Retryable,
        AgentError::RateLimit(_) => ErrorCategory::Retryable,
        AgentError::ServiceUnavailable(_) => ErrorCategory::Retryable,
        AgentError::NetworkError(_) => ErrorCategory::Retryable,
        AgentError::HttpStatus(status) if status.is_server_error() => ErrorCategory::Retryable, // 5xx

        // Permanent errors
        AgentError::InvalidApiKey(_) => ErrorCategory::Permanent,
        AgentError::ModelNotFound(_) => ErrorCategory::Permanent,
        AgentError::QuotaExceeded(_) => ErrorCategory::Permanent, // Daily quota, not rate limit
        AgentError::HttpStatus(status) if status.is_client_error() => ErrorCategory::Permanent, // 4xx (except 429)

        // Unknown (default to retryable once)
        _ => ErrorCategory::Unknown,
    }
}
```

**Classification Rules**:
| Error Type | Category | Reason |
|------------|----------|--------|
| Timeout | Retryable | Network latency temporary |
| 429 Rate Limit | Retryable | Wait and retry after backoff |
| 503 Service Unavailable | Retryable | Provider temporary outage |
| Network Error | Retryable | DNS/connection temporary |
| 5xx Server Error | Retryable | Provider internal error |
| Invalid API Key | Permanent | Configuration error (user must fix) |
| Model Not Found | Permanent | Configuration error (wrong model name) |
| 4xx Client Error | Permanent | Bad request (retry won't help) |
| Quota Exceeded | Permanent | Daily limit (retry tomorrow) |

---

### Component 2: Exponential Backoff with Jitter (CRITICAL - 2-3h)

**Implementation**:
```rust
// retry.rs
pub struct RetryConfig {
    max_attempts: u32,           // Default: 3
    initial_backoff_ms: u64,     // Default: 1000 (1s)
    max_backoff_ms: u64,         // Default: 16000 (16s)
    backoff_multiplier: f64,     // Default: 2.0 (exponential)
    jitter_factor: f64,          // Default: 0.5 (50% jitter)
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 16000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.5,
        }
    }
}

pub async fn retry_with_backoff<F, T>(
    operation: F,
    config: &RetryConfig,
) -> Result<T, AgentError>
where
    F: Fn() -> BoxFuture<'static, Result<T, AgentError>>,
{
    let mut attempt = 1;
    let mut backoff_ms = config.initial_backoff_ms;

    loop {
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    tracing::info!("Retry successful on attempt {}", attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                // Check if retryable
                let category = classify_agent_error(&e);
                if category == ErrorCategory::Permanent {
                    tracing::error!("Permanent error, not retrying: {}", e);
                    return Err(e);
                }

                // Check max attempts
                if attempt >= config.max_attempts {
                    tracing::error!("Max retries ({}) exceeded: {}", config.max_attempts, e);
                    return Err(e);
                }

                // Calculate backoff with jitter
                let jitter_range = (backoff_ms as f64 * config.jitter_factor) as u64;
                let jitter = rand::thread_rng().gen_range(0..jitter_range);
                let actual_backoff = backoff_ms + jitter;

                tracing::warn!(
                    attempt = attempt,
                    max_attempts = config.max_attempts,
                    backoff_ms = actual_backoff,
                    error = %e,
                    "Retrying after backoff"
                );

                // Sleep with backoff
                tokio::time::sleep(Duration::from_millis(actual_backoff)).await;

                // Exponential backoff for next attempt
                backoff_ms = (backoff_ms as f64 * config.backoff_multiplier) as u64;
                backoff_ms = backoff_ms.min(config.max_backoff_ms);

                attempt += 1;
            }
        }
    }
}
```

**Backoff Progression**:
```
Attempt 1: Immediate (0ms)
Attempt 2: 1000ms + rand(0-500ms) = 1000-1500ms
Attempt 3: 2000ms + rand(0-1000ms) = 2000-3000ms
Final Failure: After 3 attempts
```

---

### Component 3: Quality Gate Integration (MEDIUM - 1h)

**Implementation** (quality_gate_handler.rs):
```rust
async fn execute_quality_gate_with_retry(
    checkpoint: QualityCheckpoint,
    config: &Config,
) -> Result<ConsensusResult> {
    // Spawn all agents
    let agent_futures: Vec<_> = checkpoint.required_agents()
        .iter()
        .map(|agent_name| {
            let name = agent_name.clone();
            let cfg = config.clone();
            tokio::spawn(async move {
                retry_with_backoff(
                    || Box::pin(spawn_single_agent(&name, &cfg)),
                    &RetryConfig::default(),
                ).await
            })
        })
        .collect();

    // Wait for all agents (with retry)
    let results = join_all(agent_futures).await;

    // Count successes
    let successful: Vec<_> = results.into_iter()
        .filter_map(|r| r.ok().and_then(|r| r.ok()))
        .collect();

    // Auto-resolution (prefer 3/3, accept 2/3)
    if successful.len() >= 3 {
        Ok(ConsensusResult::FullConsensus(successful)) // 3/3 agents
    } else if successful.len() >= 2 {
        Ok(ConsensusResult::DegradedConsensus(successful)) // 2/3 agents
    } else {
        Err(SpecKitError::InsufficientAgents)
    }
}
```

---

### Component 4: Telemetry (LOW - 1h)

**Implementation**:
```rust
// In retry_with_backoff()
tracing::info!(
    agent_name = %agent,
    attempt = attempt,
    max_attempts = config.max_attempts,
    backoff_ms = actual_backoff,
    error_category = ?category,
    "Retrying agent"
);

// On final success
tracing::info!(
    agent_name = %agent,
    total_attempts = attempt,
    total_time_ms = %start.elapsed().as_millis(),
    "Agent succeeded after retries"
);

// On final failure
tracing::error!(
    agent_name = %agent,
    total_attempts = config.max_attempts,
    final_error = %e,
    "Agent failed after max retries"
);
```

**Metrics to Capture**:
- Retry attempts per agent execution
- Success rate after retries (e.g., 80% succeed on retry 2)
- Average backoff delay
- Error category distribution (retryable vs permanent)

---

## 3. Acceptance Criteria

### AC1: Error Classification ✅
- [ ] All agent errors classified (retryable, permanent, unknown)
- [ ] Classification logic covers all error types (timeout, rate limit, 5xx, 4xx)
- [ ] Unit tests verify classification correctness

### AC2: Exponential Backoff ✅
- [ ] Backoff follows exponential progression (1s, 2s, 4s, ...)
- [ ] Jitter added to prevent thundering herd (±50% variance)
- [ ] Max backoff limit enforced (16s cap)
- [ ] Max retry attempts enforced (3 attempts)

### AC3: Quality Gate Integration ✅
- [ ] Quality gates retry failed agents before degrading to 2/3
- [ ] Full 3/3 consensus achieved via retry (when possible)
- [ ] Telemetry logs retry attempts with context

### AC4: Telemetry ✅
- [ ] Retry attempts logged with backoff delays
- [ ] Success/failure after retries logged
- [ ] Error categories logged (retryable vs permanent)

---

## 4. Technical Implementation

### Day 1: Error Classification + Backoff (3-4h)

**Morning (2h)**:
- Create `error_classifier.rs` with classification logic
- Unit tests for all error types
- Verify coverage (timeout, rate limit, 5xx, 4xx, network, API key, quota)

**Afternoon (1-2h)**:
- Create `retry.rs` with exponential backoff
- Implement `retry_with_backoff()` function
- Unit tests (verify backoff progression, jitter, max attempts)

**Files**:
- `codex-core/src/error_classifier.rs` (~100 LOC)
- `codex-core/src/retry.rs` (~150 LOC)
- `codex-core/src/tests/retry_tests.rs` (~200 LOC)

---

### Day 2: Quality Gate Integration + Telemetry (2h)

**Morning (1h)**:
- Update `quality_gate_handler.rs` to use retry logic
- Integrate `retry_with_backoff()` into agent spawning
- Integration tests (multi-agent with retry)

**Afternoon (1h)**:
- Add telemetry (tracing::info! calls)
- Verify logs capture retry context
- Final testing (all quality gate tests pass)

**Files**:
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (+50 LOC)
- `codex-tui/src/chatwidget/spec_kit/agent_orchestrator.rs` (+30 LOC)

---

## 5. Success Metrics

### Reliability Metrics
- **Transient Error Recovery**: 80%+ success rate on retry (timeout, rate limit, 503)
- **Full Consensus Rate**: 90%+ quality gates achieve 3/3 (up from 70% 2/3)

### Performance Metrics
- **Retry Overhead**: <5s added latency (1s + 2s backoff typical)
- **Thundering Herd**: 0 occurrences (jitter prevents simultaneous retry)

### Telemetry Metrics
- **Retry Visibility**: 100% retry attempts logged with context
- **Error Classification Accuracy**: 100% (all known errors classified correctly)

---

## 6. Risk Analysis

### Risk 1: Infinite Retry Loops (LOW)

**Scenario**: Bug in error classification causes permanent errors to be retried infinitely.

**Mitigation**:
- Max attempts hardcoded (3 attempts maximum)
- Comprehensive unit tests for error classification
- Unknown errors default to retryable ONCE (not infinite)

**Likelihood**: Very Low (multiple safeguards)

---

### Risk 2: Excessive Backoff Delays (LOW)

**Scenario**: Users perceive quality gates as "hanging" during retry backoff.

**Mitigation**:
- TUI shows "Retrying agent X (attempt 2/3, backoff 2s)..."
- Max backoff capped at 16s (reasonable wait)
- User can cancel quality gate anytime (Ctrl+C)

**Likelihood**: Low (transparent feedback)

---

## 7. Open Questions

### Q1: Should max attempts be configurable?

**Context**: Some users may want more aggressive retry (5 attempts) or less (1 attempt).

**Decision**: YES - Add to config.json:
```json
{
  "retry": {
    "max_attempts": 3,
    "initial_backoff_ms": 1000
  }
}
```

---

### Q2: Should we retry on provider-specific errors?

**Context**: Some providers return custom error codes (e.g., Anthropic "overloaded_error").

**Decision**: YES - Extend classification to cover provider-specific codes (document in error_classifier.rs).

---

## 8. Implementation Strategy

### Day 1: Core Retry Logic (4h)
- **Hour 1-2**: Error classification (error_classifier.rs)
- **Hour 3-4**: Exponential backoff (retry.rs)
- **Testing**: Unit tests for classification + backoff

### Day 2: Integration + Telemetry (2h)
- **Hour 1**: Quality gate integration (quality_gate_handler.rs)
- **Hour 2**: Telemetry + final testing
- **Deliverable**: All quality gate tests pass with retry

**Total**: 6h (within 4-6h estimate, upper bound)

---

## 9. Deliverables

1. **Code Changes**:
   - `codex-core/src/error_classifier.rs` - Error classification logic
   - `codex-core/src/retry.rs` - Exponential backoff implementation
   - `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` - Retry integration

2. **Tests**:
   - Unit tests (error classification, backoff progression)
   - Integration tests (multi-agent with retry, quality gate resilience)

3. **Documentation**:
   - `docs/retry-strategy.md` - Error classification matrix, backoff algorithm

---

## 10. Validation Plan

### Unit Tests (15 tests)
- Error classification (all error types: timeout, rate limit, 5xx, 4xx, network, API key)
- Backoff progression (verify exponential, jitter, max cap)
- Max attempts enforcement (fail after 3 attempts)

### Integration Tests (5 tests)
- Quality gate with transient errors (verify retry recovers 3/3)
- Quality gate with permanent errors (verify no retry)
- Thundering herd prevention (verify jitter staggers retries)
- Telemetry verification (logs capture retry context)

**Total**: 20 tests

---

## 11. Conclusion

SPEC-938 enhances agent retry logic with error classification, exponential backoff, and quality gate integration. **Estimated effort: 4-6 hours over 1 day.**

**Key Benefits**:
- ✅ Higher reliability (transient errors auto-recover)
- ✅ Smarter retry (exponential backoff, jitter, max attempts)
- ✅ Better quality gates (3/3 consensus via retry)
- ✅ Comprehensive telemetry (visibility into retry patterns)

**Next Steps**:
1. Review and approve SPEC-938
2. Schedule 1-day implementation sprint
3. Coordinate with SPEC-933 (transactions enable safe retry within quality gates)

---

Back to [Key Docs](../KEY_DOCS.md)
