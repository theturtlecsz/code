# SPEC-KIT-978 — Implementer.Reflex Mode via SGLang + Bakeoff
**Date:** 2026-01-12 (Updated)
**Status:** NOT STARTED (0%)
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

### stage0.toml (or speckit.toml)

```toml
[reflex]
enabled = false
endpoint = "http://127.0.0.1:3009/v1"
model = "qwen2.5-coder-7b-instruct"
timeout_ms = 1500
json_schema = true
capture_mode = "prompts_only"  # inherited from PolicySnapshot

[reflex.fallback]
to_cloud = true
max_failures = 3

[reflex.thresholds]
p95_latency_ms = 2000
success_parity_percent = 85
json_schema_compliance_percent = 100
```

### PolicySnapshot.routing Integration

The reflex config is captured in PolicySnapshot:
```json
{
  "routing": {
    "reflex": {
      "enabled": false,
      "endpoint": "http://127.0.0.1:3009/v1",
      "model": "qwen2.5-coder-7b-instruct",
      "thresholds": { ... }
    }
  }
}
```

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

### Health Check
| Command | Description |
|---------|-------------|
| `code reflex health` | Check reflex server status |
| `code reflex models` | List available models |
| `/speckit.reflex health` | TUI equivalent |

### Bakeoff
| Command | Description |
|---------|-------------|
| `code reflex bakeoff [--suite <name>]` | Run bakeoff harness |
| `code reflex status` | Show current reflex config + thresholds |

---

## Deliverables

- [ ] Reflex config in stage0.toml
- [ ] Health check command
- [ ] OpenAI-compatible client adapter
- [ ] Routing mode implementation (`Implementer(mode=reflex)`)
- [ ] Fallback logic (reflex -> cloud)
- [ ] Capsule events for routing decisions
- [ ] Bakeoff harness extension
- [ ] Gate threshold enforcement in CI

---

## Acceptance Criteria (Testable)

### 978-A1: JSON Schema Compliance
- Reflex patch loop produces valid JSON args (schema enforced)
- No markdown preamble in output

### 978-A2: Latency Target
- TTFT and throughput meet targets (P95 < 2000ms)
- Measured on representative task set

### 978-A3: Fallback Behavior
- Fallback to cloud triggers after N failures
- Fallback is logged with reason

### 978-A4: Routing Evidence
- Every reflex call emits RoutingDecision event
- Event includes mode, reason, latency

### 978-A5: Health Check
- `code reflex health` returns 0 when server healthy
- Returns non-zero with actionable error when unhealthy

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
- Roll out with `reflex.enabled = false` (default)
- Enable via config after bakeoff passes
- Roll back by setting `reflex.enabled = false`

## Risks & Mitigations
- **Quality variance** -> Bakeoff gates before promotion
- **Server instability** -> Health checks + fallback to cloud
- **Latency spikes** -> P95 threshold enforcement
- **Model updates** -> Re-run bakeoff on model changes
