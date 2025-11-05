# SPEC-KIT-900 Adoption Dashboard Specification

Use this guide to capture weekly adoption metrics for SPEC-KIT-900 (Task T7).

---

## 1. Metrics Overview

| Metric | Target | Source | Notes |
| --- | --- | --- | --- |
| Runs per week | ≥ 5 | `evidence/commands/SPEC-KIT-900/` | Count unique `sessionId` values. |
| Successful consensus ratio | ≥ 0.90 | Telemetry payload | Average of `agreementRatio`. |
| Average cost | ≤ $3.00 | Cost summary | Track per stage and total. |
| Evidence footprint | < 20 MB warn, < 25 MB hard | `evidence_stats.sh` | Log warning when `warned` emitted. |
| Mean latency (minutes) | < 12 | Adoption sheet | Derived from telemetry `latencyMs`. |

Store the metrics in `docs/spec-kit/adoption-dashboard.csv` (one row per week).

---

## 2. Update Cadence
1. Collect telemetry and cost summaries every Friday before 17:00 UTC.
2. Update the CSV and attach the snapshot to the audit packet when `/speckit.validate` finishes.
3. Post a summary message in #spec-kit-maintainers including: run count, consensus ratio, cost, and any degradation notes.

---

## 3. CSV Layout

```text
week,run_profile,runs,consensus_ratio,total_usd,avg_latency_min,evidence_mb,degraded_runs
2025-W44,cheap-tier,6,0.94,2.82,9.8,12.3,1
```

- `week` – ISO week, e.g. `2025-W44`.
- `run_profile` – routing tier sampled.
- `evidence_mb` – combined commands + consensus footprint.
- `degraded_runs` – count of runs where agreement < 0.90 after recovery loop.

---

## 4. Dashboards & Visuals
- Recommended tooling: Google Sheets or Looker Studio.
- Create a stacked column chart for weekly run counts per routing profile.
- Include a line chart for consensus ratio vs. acceptance threshold (0.90).
- Highlight weeks breaching cost or evidence guardrails in red.

---

## 5. Review & Escalation
- If runs/week < 3 for two consecutive weeks, escalate to the PMO.
- If consensus ratio drops below 0.85, trigger the degradation playbook.
- If total cost exceeds $3.50, log a risk in `docs/spec-kit/risk-register.md` and coordinate with routing owners.
