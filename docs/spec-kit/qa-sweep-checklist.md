# SPEC-KIT-900 Telemetry QA Sweep (Task T8)

Complete this checklist after `/speckit.tasks` and before `/speckit.validate` to
ensure telemetry artifacts remain trustworthy.

---

## Pre-Run
- [ ] Validate prompts match `docs/SPEC-KIT-900-generic-smoke/PRD.md` (§4).
- [ ] Confirm telemetry schema version set to `"3.0"` in context kit.
- [ ] Ensure adoption dashboard CSV is up to date for the current week.

## During Run
- [ ] Monitor router output for agent timeouts or retries.
- [ ] Capture raw telemetry payload before any manual edits.
- [ ] Note consensus ratio and missing agents (if any).

## Post-Run Validation
| Check | Command | Status |
| --- | --- | --- |
| Telemetry schema JSON valid | `jq type evidence/commands/.../*.json` | ☐ |
| Cost summary matches schema | `jq '.perStage | keys' SPEC-KIT-900_cost_summary.json` | ☐ |
| Evidence footprint within limits | `scripts/spec_ops_004/evidence_stats.sh --spec SPEC-KIT-900` | ☐ |
| Adoption metrics updated | Update `adoption-dashboard.csv` | ☐ |
| Security template completed | `docs/spec-kit/security-review-template.md` | ☐ |

## Attachments
- Telemetry payload (`commands/` directory) ✔
- Cost summary ✔
- Evidence stats output ✔
- Adoption dashboard snapshot ✔

Store attachments under `docs/SPEC-KIT-900-generic-smoke/validation/` for audit.
