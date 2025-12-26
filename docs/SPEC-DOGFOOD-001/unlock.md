# Plan: SPEC-DOGFOOD-001

**Stage**: Unlock
**Agents**: 1
**Generated**: 2025-12-26 15:02 UTC

## Agent Responses (Raw)

*Note: Structured extraction failed, displaying raw agent data*

### claude

**stage**:
spec-unlock

**prompt_version**:
20251002-unlock-a

**model**:
claude-4.5-sonnet

**model_release**:
2025-09-29

**reasoning_mode**:
balanced

**unlock_justification**:
SPEC-DOGFOOD-001 is NOT ready to ship. Critical blocking issue: Stage0 silent skip bug prevents 3/6 acceptance criteria from being validated (A2: Tier2 Used, A3: Evidence Exists, A4: System Pointer). Despite infrastructure being healthy (code doctor [OK], local-memory healthy, NotebookLM authenticated), /speckit.auto produces no TUI output and generates no evidence artifacts. Root cause analysis: `/speckit.auto` command is routing through an unidentified third code path that bypasses both ProcessedCommand::SpecAuto and SPEC_KIT_REGISTRY dispatch. Debug symbols exist in binary (strings shows 'handle_spec_auto' and skip-reason messages), but neither documented entry point fires. Sessions 25-26 added tracing at pipeline_coordinator.rs and commands/special.rs with no output, confirming the routing bypass. The golden path objective—validating Tier1+Tier2 integration for dogfooding—cannot be achieved until Stage0 is wired correctly. Passing criteria (A0: No Surprise Fan-Out, A1: Doctor Ready, A5: GR-001 Enforcement, A6: Slash Dispatch Single-Shot) demonstrate system readiness in supporting infrastructure, but the core spec objective hinges on Stage0 execution. Code quality is excellent: ~5,422 LOC dead code deleted, 0 warnings, 543+ tests passing. However, shipping with an unresolved routing bug would make dogfooding impossible—users running /speckit.auto would see no output, consistent with the bug observed. Recommend: Continue Session 26+ investigation focusing on third routing path discovery (check ChatWidget.handle_message, AppEvent enum alternatives, config-driven command overrides).

**readiness_assessment**:
NOT READY. 4/6 acceptance criteria pass (67%), but 2/6 are blocked by critical infrastructure bug. Core objective (validate Tier2 integration) is unreachable without Stage0 fix. System prerequisites and supporting infrastructure healthy. Code quality excellent. Blocking factor is deterministic and reproducible—fix is achievable but not yet resolved.

**safeguards**:
- "Do not merge to main until Stage0 routing issue resolved and A2, A3, A4 validated in full pipeline run"
- "Do not dogfood with /speckit.auto until evidence artifacts are confirmed generated"
- "If Stage0 fix requires non-trivial routing refactor, require code review + integration test before merge"
- "Validate NotebookLM session freshness before marking Tier2 complete (re-auth if cookie >24h old)"
- "Add regression test case for /speckit.auto routing to prevent future silent skip bugs"

**followups**:
- "Session 26+: Complete Stage0 routing trace. Check ChatWidget::handle_message for alternative dispatch, verify SlashCommand::SpecKitAuto enum matching, audit Config-driven command overrides, search for 'speckit.auto' string outside known dispatch paths"
- "Once Stage0 routed correctly: Run /speckit.auto SPEC-DOGFOOD-001 and verify tier2_used=true in output + TASK_BRIEF.md + DIVINE_TRUTH.md artifacts generated"
- "Validate system pointer storage: lm search 'SPEC-DOGFOOD-001' should return memory entry with system:true tag"
- "Add integration test for golden path: spawn TUI subprocess, send /speckit.auto command, verify Stage0 output and artifacts within 30s"
- "Document Stage0 architecture in runbook for future debugging (routing flow diagram, entry points, debug symbol locations)"


## Consensus Summary

- Synthesized from 1 agent responses
- All agents completed successfully
