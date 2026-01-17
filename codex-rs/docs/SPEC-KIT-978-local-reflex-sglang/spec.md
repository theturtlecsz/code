# SPEC-KIT-978 — Implementer.Reflex Mode via SGLang + Bakeoff
**Date:** 2026-01-17 (Updated)
**Status:** IN PROGRESS (75%)
**Owner (role):** Infra/LLM Eng

## Summary

Wire a local OpenAI-compatible inference server for sub-second compiler loops using SGLang with RadixAttention and JSON schema enforcement. Establish bakeoff gates for reflex promotion.

## Decision IDs implemented

**Implemented by this spec:** D13, D25, D43, D55, D78, D93, D110, D112

**Referenced (must remain consistent):** D49, D50

**Explicitly out of scope:** D27

---

## Non-Negotiable: Reflex is a Routing Mode

**This is the most important requirement and cannot be compromised:**

- **Reflex is NOT a new Stage0 role** - It is expressed as `Implementer(mode=reflex)`
- **No new role name** - No "Reflex" role in Stage0
- **Routing chooses backend** based on: policy snapshot routing rules, stage context (Implement only), local server health, bakeoff thresholds
- **Stage context** - Reflex only applies to Implement stage

---

## Goals
- Sub-second inference for patch loop tasks
- Cost reduction for high-volume implementation
- Privacy for sensitive codebases
- Quality parity with cloud (enforced via bakeoff)

## Non-Goals
- Replacing cloud models for all stages
- Running inference on CPU
- Multi-tenant inference service

---

## Configuration Keys

### model_policy.toml (Authoritative Source)

Reflex configuration lives in `model_policy.toml`, NOT `stage0.toml`:

```toml
[routing.reflex]
# Reflex is a routing mode for Implementer only
enabled = false
endpoint = "http://127.0.0.1:3009/v1"
model = "qwen2.5-coder-7b-instruct"
timeout_ms = 1500
json_schema_required = true

# Fallback order when reflex is enabled
# 1. Try reflex if healthy + thresholds met
# 2. Fall back to cloud implementer
fallback_to_cloud = true

[routing.reflex.thresholds]
# Bakeoff thresholds for reflex promotion
p95_latency_ms = 2000
success_parity_percent = 85
json_schema_compliance_percent = 100
```

### PolicySnapshot.governance.routing Integration

The reflex config is captured in `PolicySnapshot.governance.routing`:

```json
{
  "governance": {
    "routing": {
      "reflex": {
        "enabled": false,
        "endpoint": "http://127.0.0.1:3009/v1",
        "model": "qwen2.5-coder-7b-instruct",
        "timeout_ms": 1500,
        "json_schema_required": true,
        "fallback_to_cloud": true,
        "thresholds": {
          "p95_latency_ms": 2000,
          "success_parity_percent": 85,
          "json_schema_compliance_percent": 100
        }
      }
    }
  }
}
```

### Config Loading (Implemented)

```rust
// In stage0/src/reflex_config.rs
pub fn load_reflex_config(config_path: Option<&PathBuf>) -> Result<ReflexConfig>
```

Loads from `model_policy.toml` at `[routing.reflex]` section.

---

## Local Server Contract (OpenAI-Compatible)

### Required Endpoints

| Endpoint | Requirement |
|----------|-------------|
| `GET /v1/models` | Returns available models |
| `POST /v1/chat/completions` | Chat completion with streaming optional |

### Required Features

- **JSON schema / constrained decoding** OR explicit "must output valid JSON" with validator+retry
- **Streaming** (optional but recommended)
- **Request timeout** handling

### Health Check

```bash
curl -s http://127.0.0.1:3009/v1/models | jq -e '.data | length > 0'
```

---

## Routing Logic

### Decision Flow

```
if stage != Implement:
    use cloud_implementer
elif not reflex.enabled:
    use cloud_implementer
elif not reflex_server_healthy():
    use cloud_implementer
elif not bakeoff_thresholds_met():
    use cloud_implementer
else:
    use reflex
    on_failure:
        increment failure_count
        if failure_count >= max_failures:
            fallback to cloud_implementer
```

### Routing Evidence

Every routing decision emits capsule event:
```json
{
  "event_type": "RoutingDecision",
  "role": "Implementer",
  "mode": "reflex",  // or "cloud"
  "reason": "healthy_and_thresholds_met",  // or "server_unhealthy", "thresholds_not_met", etc.
  "latency_ms": 145
}
```

---

## Bakeoff Harness Requirements

### Suite: `reflex_patchloop_v1`

Run with `Implementer(mode=reflex)` vs cloud implementer on same tasks:

| Metric | Measurement |
|--------|-------------|
| Success rate | % of tasks completed without error |
| P50/P95 latency | Time to first token + completion |
| Diff quality | Tests passing, lint passing |
| JSON compliance | % of responses that parse as valid JSON |

### Gate Thresholds

| Threshold | Value | Action |
|-----------|-------|--------|
| P95 latency | < 2000ms | Required for promotion |
| Success parity | >= 85% | Required for promotion |
| JSON compliance | 100% | Required for promotion |

### Harness Output

- JSON report: `.speckit/eval/reflex-bakeoff-<timestamp>.json`
- Markdown report: `.speckit/eval/reflex-bakeoff-<timestamp>.md`
- CI fails if regression exceeds thresholds

---

## Capture for Replay (SPEC-KIT-975)

Reflex calls MUST emit capsule events:

```json
{
  "event_type": "LLMCall",
  "role": "Implementer",
  "mode": "reflex",
  "model": "qwen2.5-coder-7b-instruct",
  "request": { ... },  // or hash if capture_mode != full_io
  "response": { ... }, // or hash if capture_mode != full_io
  "latency_ms": 145,
  "fallback_reason": null  // or reason if fell back
}
```

Capture mode is controlled by `PolicySnapshot.capture.mode`.

---

## CLI Commands

### Headless CLI (Implemented)

| Command | Description | Status |
|---------|-------------|--------|
| `code speckit reflex bakeoff [--since <duration>] [--json]` | Show bakeoff statistics (reflex vs cloud) | DONE |
| `code speckit reflex check [--since <duration>] [--min-samples N] [--json]` | Validate if reflex meets thresholds | DONE |

**Duration formats:** `1h`, `24h`, `7d`, `30d` (default: `24h`)

### TUI Slash Commands (Planned)

| Command | Description | Status |
|---------|-------------|--------|
| `/speckit.reflex health` | Check reflex server status | PLANNED |
| `/speckit.reflex status` | Show current reflex config + thresholds | PLANNED |
| `/speckit.reflex models` | List available models | PLANNED |

### Example Output

```bash
$ code speckit reflex bakeoff --json
{
  "period": "24h",
  "reflex": {
    "total_attempts": 45,
    "success_rate": 97.8,
    "json_compliance_rate": 100.0,
    "p95_latency_ms": 1450
  },
  "cloud": {
    "total_attempts": 12,
    "success_rate": 100.0,
    ...
  }
}
```

---

## Deliverables

### Core Infrastructure (DONE)
- [x] Reflex config in `model_policy.toml` (`[routing.reflex]`)
- [x] `ReflexConfig` struct and `load_reflex_config()` (`stage0/src/reflex_config.rs`)
- [x] OpenAI-compatible client adapter (`reflex_client.rs`)
- [x] JSON schema enforcement (`chat_completion_json()` with schema parameter)

### Routing Logic (DONE)
- [x] Routing decision module (`reflex_router.rs`)
- [x] `decide_implementer_routing()` with full decision flow
- [x] Fallback logic (reflex -> cloud on failure/thresholds)
- [x] Health check integration (server reachability + model availability)
- [x] Threshold checking against bakeoff metrics

### Capsule Integration (DONE)
- [x] `RoutingDecision` capsule event emission (`emit_routing_event()`)
- [x] `RoutingDecisionPayload` with mode, reason, latency
- [x] `RoutingMode` and `RoutingFallbackReason` types

### Metrics & CLI (DONE)
- [x] `ReflexMetricsDb` for bakeoff stats (`reflex_metrics.rs`)
- [x] `code speckit reflex bakeoff` command
- [x] `code speckit reflex check` command
- [x] `bakeoff_runner.rs` module for trial execution

### Remaining Work
- [ ] **Bakeoff report writer**: Write JSON/MD reports to `.speckit/eval/reflex-bakeoff-<timestamp>.*`
- [ ] **CI gate**: `code speckit reflex check --exit-code` exits non-zero if thresholds not met
- [ ] **LLMCall event capture**: Align with `PolicySnapshot.capture.mode` (ties into SPEC-KIT-975)
- [ ] **TUI slash commands**: `/speckit.reflex health|status|models`
- [ ] **Wire bakeoff_runner**: Execute trials via `bakeoff_runner.rs` (module exists, not wired)

---

## Acceptance Criteria (Testable)

### 978-A1: JSON Schema Compliance ✅ PASSING
- Reflex patch loop produces valid JSON args (schema enforced)
- No markdown preamble in output
- **Test**: `reflex_client.rs` - `test_json_compliance_*` tests

### 978-A2: Latency Target ✅ PASSING
- TTFT and throughput meet targets (P95 < 2000ms)
- Measured on representative task set
- **Test**: `code speckit reflex check` validates against thresholds

### 978-A3: Fallback Behavior ✅ PASSING
- Fallback to cloud triggers on health check failure or threshold miss
- Fallback is logged with reason
- **Test**: `reflex_router.rs` - `test_routing_*` tests

### 978-A4: Routing Evidence ✅ PASSING
- Every reflex routing decision emits `RoutingDecision` event
- Event includes mode, reason, latency, is_fallback
- **Implementation**: `emit_routing_event()` in `reflex_router.rs`

### 978-A5: Health Check ✅ PASSING
- Health check validates server reachability AND model availability
- Returns structured `HealthCheckResult` with detailed status
- **Implementation**: `check_reflex_health()` in `reflex_router.rs`

---

## Server Runbook

### SGLang Setup (RTX 5090)

```bash
# Install SGLang
pip install sglang[all]

# Start server
python -m sglang.launch_server \
  --model-path Qwen/Qwen2.5-Coder-7B-Instruct \
  --port 3009 \
  --host 0.0.0.0 \
  --tp 1 \
  --dtype float16
```

### vLLM Fallback

```bash
# If SGLang unavailable
pip install vllm

python -m vllm.entrypoints.openai.api_server \
  --model Qwen/Qwen2.5-Coder-7B-Instruct \
  --port 3009 \
  --host 0.0.0.0
```

---

## Dependencies
- SPEC-KIT-971: Capsule foundation (for event storage)
- SPEC-KIT-977: PolicySnapshot (for routing config)
- Decision Register: `docs/DECISION_REGISTER.md`

## Rollout / Rollback
- Roll out with `routing.reflex.enabled = false` (default in `model_policy.toml`)
- Enable via config after bakeoff passes (`code speckit reflex check` succeeds)
- Roll back by setting `routing.reflex.enabled = false` in `model_policy.toml`

## Risks & Mitigations
- **Quality variance** -> Bakeoff gates before promotion
- **Server instability** -> Health checks + fallback to cloud
- **Latency spikes** -> P95 threshold enforcement
- **Model updates** -> Re-run bakeoff on model changes
