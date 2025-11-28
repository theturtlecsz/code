**SPEC-ID**: SYNC-011
**Feature**: OpenTelemetry Observability Crate
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-011
**Owner**: Code

**Context**: Port the `codex-otel` crate from upstream providing OpenTelemetry integration for distributed tracing and metrics. This enables production monitoring, performance analysis, and debugging of multi-agent pipelines. Critical for observability of spec-kit orchestration and DirectProcessExecutor performance.

**Source**: `~/old/code/codex-rs/otel/` (otel_event_manager.rs 19KB, otel_provider.rs 9KB)

---

## User Scenarios

### P1: Distributed Tracing for Agent Execution

**Story**: As an operator, I want distributed tracing so that I can debug slow or failing multi-agent pipelines.

**Priority Rationale**: Multi-agent orchestration creates complex execution flows; tracing is essential for debugging.

**Testability**: Execute spec-kit pipeline and verify traces appear in OTLP backend.

**Acceptance Scenarios**:
- Given a spec-kit pipeline runs, when traced, then each stage appears as a span
- Given an agent call fails, when traced, then error details are attached to span
- Given multiple agents run in parallel, when traced, then parent-child relationships are correct

### P2: Performance Metrics Collection

**Story**: As an operator, I want performance metrics so that I can identify bottlenecks and track SLOs.

**Priority Rationale**: Metrics enable capacity planning and performance optimization.

**Testability**: Generate load and verify metrics in OTLP backend.

**Acceptance Scenarios**:
- Given API calls, when metrics collected, then latency histograms are populated
- Given agent execution, when metrics collected, then duration and success rate are tracked
- Given token usage, when metrics collected, then consumption is recorded

### P3: Custom Event Recording

**Story**: As a developer, I want to record custom events so that application-specific signals are captured.

**Priority Rationale**: Custom events enable domain-specific observability beyond standard traces.

**Testability**: Emit custom event and verify it appears in traces.

**Acceptance Scenarios**:
- Given a quality gate check, when recorded, then event appears with pass/fail status
- Given consensus synthesis, when recorded, then agent responses and conflicts are captured
- Given model selection, when recorded, then provider and model are logged

---

## Edge Cases

- OTLP endpoint unavailable (buffer locally, warn, don't block execution)
- Very high event volume (sampling, aggregation)
- Sensitive data in spans (implement attribute filtering)
- Span context propagation across process boundaries (W3C trace context)
- Graceful shutdown with pending spans (flush with timeout)

---

## Requirements

### Functional Requirements

- **FR1**: Implement `OtelEventManager` for span and event management
- **FR2**: Implement `OtelProvider` for OTLP exporter configuration
- **FR3**: Support trace context propagation (W3C Trace Context headers)
- **FR4**: Integrate with DirectProcessExecutor for command execution tracing
- **FR5**: Integrate with spec-kit pipeline for stage tracing
- **FR6**: Support configurable sampling rates
- **FR7**: Provide attribute filtering for sensitive data

### Non-Functional Requirements

- **Performance**: Tracing overhead should be <5% of execution time
- **Reliability**: Tracing failures must not affect application execution
- **Scalability**: Support high-volume event streams with sampling
- **Compatibility**: Work with standard OTLP backends (Jaeger, Tempo, etc.)

---

## Success Criteria

- Crate compiles and is added to workspace
- OTLP exporter connects to configured endpoint
- Traces appear in OTLP backend for test executions
- Span hierarchy correctly represents execution flow
- Sensitive data is filtered from exported attributes
- Integration points documented for DirectProcessExecutor and spec-kit

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-otel
cd codex-rs && cargo test -p codex-otel

# Local testing with Jaeger
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest

# Run with OTLP enabled
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 ./target/debug/codex-tui

# View traces at http://localhost:16686
```

---

## Dependencies

- `opentelemetry` crate
- `opentelemetry-otlp` for OTLP export
- `tracing-opentelemetry` for tracing integration
- OTLP-compatible backend for production use (Jaeger, Tempo, Honeycomb, etc.)

---

## Notes

- Large crate (~28KB total) - most complex of the sync items
- Estimated 8-12h for full port and integration
- Consider feature flag to disable OTEL for minimal builds
- Fork's spec-kit has existing telemetry - may need to bridge or replace
- DirectProcessExecutor spans should capture command, duration, exit code
- Agent calls should capture provider, model, token count, latency
