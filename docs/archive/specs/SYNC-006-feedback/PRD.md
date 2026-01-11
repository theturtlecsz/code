**SPEC-ID**: SYNC-006
**Feature**: Feedback Crate
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-006
**Owner**: Code

**Context**: Port the `feedback` crate from upstream providing user feedback/bug reporting with Sentry integration. This enables users to report issues with attached logs and session context, improving bug triage and resolution. The crate provides a ring buffer for log collection with automatic size limits.

**Source**: `~/old/code/codex-rs/feedback/` (~250 LOC)

---

## User Scenarios

### P1: Bug Report Submission

**Story**: As a user experiencing an issue, I want to submit a bug report so that developers can investigate with relevant context.

**Priority Rationale**: User feedback is critical for identifying and fixing issues in production.

**Testability**: Submit test bug report and verify it reaches Sentry/logging backend.

**Acceptance Scenarios**:
- Given a user encounters an error, when they submit feedback, then logs and session info are attached
- Given feedback submission, when sent to Sentry, then issue is created with attachments
- Given offline mode, when feedback submitted, then it's queued for later submission

### P2: Automatic Log Collection

**Story**: As a developer, I want automatic log collection with size limits so that bug reports include relevant context without excessive data.

**Priority Rationale**: Context is essential for debugging but must be bounded to avoid memory/bandwidth issues.

**Testability**: Fill ring buffer beyond limit and verify oldest entries are dropped.

**Acceptance Scenarios**:
- Given continuous logging, when buffer exceeds 4MB, then oldest entries are dropped
- Given a crash, when logs are collected, then recent entries are preserved
- Given sensitive data in logs, when collected, then it's redacted before submission

### P3: Session Classification

**Story**: As a user, I want to classify my feedback (bug, bad result, good result) so that reports are properly categorized.

**Priority Rationale**: Classification helps prioritize and route feedback appropriately.

**Testability**: Submit feedback with each classification type.

**Acceptance Scenarios**:
- Given feedback classified as "bug", when submitted, then it's tagged as bug in Sentry
- Given feedback classified as "bad_result", when submitted, then it's tagged for quality review
- Given feedback classified as "good_result", when submitted, then it's tracked as positive signal

---

## Edge Cases

- Sentry unavailable (queue locally, warn user)
- Very large log entries (truncate individual entries)
- Rapid logging filling buffer (drop oldest, maintain most recent)
- Sensitive data in logs (implement redaction patterns)
- Feedback during shutdown (attempt synchronous send)

---

## Requirements

### Functional Requirements

- **FR1**: Implement ring buffer log collector with 4MB default cap
- **FR2**: Support feedback classification: bug, bad_result, good_result
- **FR3**: Integrate with Sentry for issue creation and attachment upload
- **FR4**: Provide log entry redaction for sensitive patterns (API keys, tokens)
- **FR5**: Queue feedback when offline, retry on connectivity

### Non-Functional Requirements

- **Performance**: Log collection must not block main thread (async buffer)
- **Memory**: Ring buffer hard limit of 4MB (configurable)
- **Reliability**: Feedback submission should not crash the application on failure
- **Privacy**: Redact API keys, tokens, and PII from submissions

---

## Success Criteria

- Crate compiles and is added to workspace
- Ring buffer logging works with size limits
- Sentry integration sends feedback (requires Sentry account)
- Log redaction removes sensitive patterns
- TUI integration documented (separate task)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-feedback
cd codex-rs && cargo test -p codex-feedback

# Verify ring buffer behavior
# (requires test that fills buffer and checks size)
```

---

## Dependencies

- `sentry` crate for issue submission
- Sentry account and DSN for full integration (optional for crate-only port)
- `tracing` for log capture integration

---

## Notes

- Crate port is ~1h; full TUI integration is 4-6h (separate task)
- Sentry account required for production feedback - can be fork-specific
- Consider local-only mode for users who don't want telemetry
- Redaction patterns should be configurable via config file
