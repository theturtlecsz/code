# Consensus Degradation Playbook – SPEC-KIT-900

When fewer than three agents produce actionable output, follow this playbook to
restore consensus and capture evidence for the audit packet (Task T6).

---

## 1. Detection Matrix

| Scenario | Agreement Ratio | Immediate Action |
| --- | --- | --- |
| Healthy run | ≥ 0.90 | Record in telemetry and proceed. |
| Partial degradation | 0.67 – 0.89 | Run recovery loop (section 2) once. |
| Severe degradation | < 0.67 | Halt stage, escalate to Spec Kit Operator. |

- Always record `agreementRatio`, `missingAgents`, and `conflicts` in telemetry
  (see `telemetry-cost-schema.md`).
- Flag degraded outcomes in `notes` array and in the adoption dashboard.

---

## 2. Recovery Loop
1. **Refresh context** – Ensure the SPEC docs are committed and regenerate the
   context kit (`docs/SPEC-KIT-900-generic-smoke/telemetry-cost-schema.md` &
   `tasks.md`).
2. **Re-run stage** – Execute the same `/speckit.*` command once more.
3. **Force alternative routing** – If the second attempt fails, switch to the
   backup routing profile (cheap ↔ premium) and rerun.
4. **Capture diagnostics** – Save the degraded telemetry payload and the router
   logs under `evidence/commands/SPEC-KIT-900/` with a suffix describing the
   attempt (e.g. `_retry1`).
5. **Update adoption dashboard** – Mark the run as degraded and log follow-up
   actions (Task T7).

If consensus remains <0.67 after the loop, escalate to manual review.

---

## 3. Escalation Procedure

| Step | Owner | Description |
| --- | --- | --- |
| Notify | Spec Kit Operator | Post summary in #spec-kit-maintainers with evidence links. |
| Analyse | Telemetry Engineer | Inspect agent logs for errors (timeouts, invalid JSON). |
| Mitigate | Routing Owner | Adjust agent roster or model versions if systemic. |
| Approve | Security Program Manager | Sign off in `security-review-template.md` if data handling changed. |

Escalations must complete before the audit packet is finalised.

---

## 4. Postmortem Checklist
- [ ] Telemetry payload archived with `degradedReason` populated.
- [ ] Adoption dashboard entry updated (include rerun outcome).
- [ ] Audit packet section “Consensus Variance” filled in with ratios and links.
- [ ] Lessons learned appended to `docs/spec-kit/risk-register.md`.

---

## 5. References
- `docs/spec-kit/security-review-template.md`
- `docs/spec-kit/adoption-dashboard.md`
- `docs/spec-kit/consensus-cost-audit-packet.md`
- `scripts/spec_ops_004/evidence_stats.sh`
