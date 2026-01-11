# SPEC-KIT-900 Telemetry & Cost Schema

This document defines the canonical telemetry payload and cost summary expected
from `/speckit.plan`, `/speckit.tasks`, `/speckit.validate`, and
`/speckit.implement` runs of SPEC-KIT-900.  It exists to keep stage evidence in
sync with the acceptance criteria listed in `spec.md` and to ensure downstream
analysis scripts can parse artifacts without reverse-engineering ad-hoc fields.

---

## Command Telemetry Envelope

Every stage must emit a telemetry JSON object with the following keys.  All
fields are strings unless noted otherwise.

| Field | Required | Description |
| --- | --- | --- |
| `schemaVersion` | ✓ | Always `"3.0"` for the SPEC-KIT-900 benchmark profile. |
| `command` | ✓ | Slash command executed, e.g. `/speckit.tasks`. |
| `specId` | ✓ | Always `"SPEC-KIT-900"`. |
| `stage` | ✓ | One of `plan`, `tasks`, `validate`, `implement`. |
| `sessionId` | ✓ | UUID generated per automation run. |
| `runProfile` | ✓ | Routing tier identifier (`cheap-tier`, `premium-tier`, etc.). |
| `retryAttempt` | ✓ | Integer; 0 for first attempt, incremented on retries. |
| `timestamp` | ✓ | ISO 8601 UTC timestamp captured when consensus finishes. |
| `evidenceFootprintBytes` | ✓ | Total bytes of consensus + command evidence for the stage. |
| `notes` | – | Optional array of free-form strings (degraded mode, manual overrides). |

### Agent Metrics

Embed an `agents` array that tracks per-agent resource usage and outcomes.

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `agent` | string | ✓ | Logical name (`gemini`, `claude`, `gpt_pro`, `gpt_codex`, `code`). |
| `modelId` | string | ✓ | Full provider model identifier. |
| `reasoningMode` | string | ✓ | `low`, `medium`, or `high`. |
| `promptTokens` | integer | ✓ | Tokens sent to the model. |
| `completionTokens` | integer | ✓ | Tokens returned from the model. |
| `latencyMs` | integer | ✓ | End-to-end latency per agent call. |
| `costUsd` | number | ✓ | Cost billed for the agent call. |
| `cacheHit` | boolean | – | Mark when router served a cached response. |
| `arbiterOverride` | boolean | – | True iff the arbiter replaced an agent response. |
| `overrideReason` | string | – | Human-readable justification for overrides. |

### Consensus Summary Fields

Stages must also surface a `consensus` object:

| Field | Required | Description |
| --- | --- | --- |
| `consensusOk` | ✓ | Boolean; true when ≥90% participation with no conflicts. |
| `agreementRatio` | ✓ | Decimal between 0 and 1 summarising agent agreement. |
| `missingAgents` | ✓ | Array of agent identifiers that failed to produce actionable output. |
| `conflicts` | ✓ | Array of structured conflict notes (empty when consensus succeeds). |
| `degradedReason` | – | Optional string explaining degraded runs. |

---

## Cost Summary Schema

`docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/SPEC-KIT-900_cost_summary.json`
must conform to the following shape:

```json
{
  "specId": "SPEC-KIT-900",
  "schemaVersion": "1.0",
  "generatedAt": "2025-10-28T18:45:31Z",
  "runProfile": "cheap-tier",
  "perStage": {
    "plan": {"usd": 0.82, "tokens": 5123},
    "tasks": {"usd": 0.94, "tokens": 4670},
    "validate": {"usd": 0.61, "tokens": 4895},
    "implement": {"usd": 0.37, "tokens": 2441}
  },
  "totalUsd": 2.74,
  "totalTokens": 17129,
  "notes": ["consensus_ok"]
}
```

- Amounts are expressed in USD to two decimal places; token counts are integers.
- If a stage is skipped, set its `usd` and `tokens` to `0` and include `notes`
  explaining the omission.

---

## Guardrails & Thresholds

- **Cost guardrail**: keep total spend below **$3.00** for a complete run.
- **Token guardrail**: each stage should emit between **4 k** and **6 k** tokens.
- **Evidence footprint**: total size of `commands/` + `consensus/` for the SPEC
  should remain below **20 MB**; exceeding **25 MB** requires archival or prune
  (see `docs/spec-kit/evidence-baseline.md`).

`scripts/spec_ops_004/evidence_stats.sh --spec SPEC-KIT-900` now reports both the
20 MB warning and 25 MB failure thresholds and prints an actionable remediation
hint when they are exceeded.

---

## Validation Procedure

1. After each stage concludes, append the command telemetry payload to
   `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-900/`.
2. Regenerate `SPEC-KIT-900_cost_summary.json` using the router output or cost
   tracker and confirm the JSON matches the schema above.
3. Run `scripts/spec_ops_004/evidence_stats.sh --spec SPEC-KIT-900` and capture
   the output in the audit packet; remediate if warnings appear.
4. Record the consensus verdict alongside the telemetry payload so that the
   audit packet (T9) can reconcile agreement ratios with the cost report.

All deviations (missing agents, degraded consensus, cost overages) must be
documented in the audit packet template prior to `/speckit.validate` sign-off.
