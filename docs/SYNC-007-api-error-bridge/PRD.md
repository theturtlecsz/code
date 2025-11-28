**SPEC-ID**: SYNC-007
**Feature**: API Error Bridge Logic
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-007
**Owner**: Code

**Context**: Extract and adapt rate limit parsing and error mapping logic from upstream's `api_bridge.rs`. The fork has custom API clients but lacks sophisticated error handling for rate limits, usage limits, and provider-specific errors. This improves user experience with actionable error messages and retry hints.

**Source**: `~/old/code/codex-rs/core/src/api_bridge.rs` (logic extraction, not full file copy)

---

## User Scenarios

### P1: Rate Limit Error Handling

**Story**: As a user hitting rate limits, I want clear error messages with retry timing so that I know when to try again.

**Priority Rationale**: Rate limits are common with API usage; poor handling frustrates users.

**Testability**: Trigger rate limit and verify error message includes retry-after time.

**Acceptance Scenarios**:
- Given a 429 response with Retry-After header, when error is displayed, then retry time is shown
- Given a 429 response without Retry-After, when error is displayed, then default backoff is suggested
- Given rate limit error, when automatic retry is configured, then request is retried after delay

### P2: Usage Limit Detection

**Story**: As a user approaching usage limits, I want warnings so that I can manage my API usage proactively.

**Priority Rationale**: Usage limits affect user workflow; advance warning enables planning.

**Testability**: Simulate usage limit headers and verify warning display.

**Acceptance Scenarios**:
- Given response with usage headers, when near limit, then warning is displayed
- Given usage limit exceeded, when error occurs, then plan upgrade info is shown
- Given usage tracking, when limit reset time known, then it's displayed to user

### P3: Provider-Specific Error Mapping

**Story**: As a developer, I want consistent error types across providers so that error handling code is simpler.

**Priority Rationale**: Consistency reduces code complexity but is less user-visible.

**Testability**: Trigger errors from different providers and verify unified error type.

**Acceptance Scenarios**:
- Given OpenAI error response, when mapped, then it becomes `CodexErr::RateLimit`
- Given Anthropic error response, when mapped, then it becomes appropriate `CodexErr` variant
- Given unknown error format, when mapped, then it becomes `CodexErr::ApiError` with details

---

## Edge Cases

- Malformed Retry-After header (use default backoff)
- Missing error response body (use status code only)
- Different error formats between providers (normalize to common structure)
- Nested error objects in JSON response (extract most relevant message)
- HTML error pages instead of JSON (detect and handle gracefully)

---

## Requirements

### Functional Requirements

- **FR1**: Parse `Retry-After` header (both delta-seconds and HTTP-date formats)
- **FR2**: Extract rate limit info from provider-specific error bodies (OpenAI, Anthropic, Google)
- **FR3**: Add `RateLimitInfo` struct with retry_after, limit, remaining, reset fields
- **FR4**: Add `UsageLimitInfo` struct with plan_type, usage, limit, reset fields
- **FR5**: Extend `CodexErr` enum with rate limit and usage limit variants
- **FR6**: Add error mapping helpers in `api_clients/mod.rs`

### Non-Functional Requirements

- **Performance**: Error parsing should be <1ms
- **Compatibility**: Work with existing fork error types (extend, don't replace)
- **Maintainability**: Provider-specific parsing isolated to individual modules

---

## Success Criteria

- `CodexErr` has new variants for rate limits and usage limits
- Rate limit errors include retry timing information
- Error messages are user-friendly with actionable guidance
- All three providers (OpenAI, Anthropic, Google) have error mapping
- Existing error handling continues to work (backward compatible)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-core
cd codex-rs && cargo test -p codex-core error

# Manual testing with rate limit simulation
# (requires mock server or actual API key near limits)
```

---

## Dependencies

- `codex-core` error module (existing)
- `codex-core` api_clients module (existing)
- HTTP response parsing (existing reqwest usage)

---

## Notes

- This is logic extraction, not full module copy - upstream's api_bridge.rs has tight coupling to `codex-api` crate which fork doesn't have
- Fork's error types are in `core/src/error.rs` - extend with new variants
- Consider adding telemetry for rate limit frequency to inform capacity planning
- Retry logic itself is separate concern (can use with or without this)
