# Validation Plan: SPEC-KIT-900

**SPEC**: SPEC-KIT-900-generic-smoke
**Stage**: Validate
**Generated**: 2025-11-04

---

## Test Strategy

### Unit Tests
- Consensus artifact storage (SQLite)
- Evidence export functionality
- Cost summary schema validation
- run_id tracking accuracy

### Integration Tests
- Multi-agent pipeline execution (Plan → Tasks → Implement)
- Sequential agent collaboration
- Parallel agent execution
- Quality gate integration

### System Tests
- Full `/speckit.auto` pipeline (6 stages)
- Automated verification reports
- Evidence directory population
- Cost tracking accuracy

---

## Monitoring

### Key Metrics
- Agent completion rate (target: ≥90%)
- run_id coverage (target: 100%)
- Evidence file generation (all stages)
- Cost tracking accuracy (±10%)

### Alerts
- Agent failure rate >10%
- Missing evidence files
- Cost summary schema violations
- SQLite query failures

---

## Rollback Plan

If issues detected:
1. Revert to commit bf0d7afd4 (Part 1/3, known stable)
2. Archive failed run evidence
3. Review logs for root cause
4. Fix and re-test

---

## Cost Estimate

**Per Run**:
- Plan stage: ~$0.08 (3 agents, low-tier)
- Tasks stage: ~$0.10 (3 agents, low-tier)
- Implement stage: ~$0.11 (4 agents, code-tier)
- Validate stage: ~$0.35 (3 agents, medium-tier)
- Audit stage: ~$0.80 (3 agents, high-tier)
- Unlock stage: ~$0.80 (3 agents, high-tier)

**Total**: ~$2.24 per full pipeline

**Budget**: $2.00 (slightly over, acceptable for testing)

---

## Acceptance Criteria

- [ ] All agents complete successfully (≥90% success rate)
- [ ] Evidence directories populated (commands/, consensus/, costs/)
- [ ] Cost summary matches schema v1
- [ ] run_id tracked for all agents
- [ ] Verification report shows ✅ PASS
- [ ] No missing consensus artifacts
- [ ] Logs filterable by run_id

---

## Test Execution Log

**Latest Run**: 2025-11-04 02:00-02:45
- Plan: ✅ Completed (3 agents)
- Tasks: ✅ Completed (3 agents)
- Implement: ✅ Completed (4 agents, but tiny output - bug fixed)
- Validate: ⏸️ Not executed
- Audit: ⏸️ Not executed
- Unlock: ⏸️ Not executed

**Issues Found**:
- implement.md was 191 bytes (collected 23 agents instead of 4)
- Fixed with agent collection filtering (commit 78deeeb8f)

**Current Status**: Ready for full pipeline test

---

## Evidence Verification

**Required Evidence** (per PRD):
- ✅ consensus/SPEC-KIT-900/*_synthesis.json
- ✅ consensus/SPEC-KIT-900/*_verdict.json
- ✅ costs/SPEC-KIT-900_cost_summary.json
- ✅ commands/SPEC-KIT-900/guardrail-*.json

**Status**: All evidence files present after export

---

**Prepared**: 2025-11-04
**Status**: Validation plan complete, evidence exported
**Next**: Re-run `/speckit.checklist SPEC-KIT-900` for PASS verification
