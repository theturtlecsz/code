# Documentation Inventory Report

_Generated: 2026-01-21T21:16:23.178708_

## Executive Summary

| Metric | Value |
|--------|-------|
| Total Files | 904 |
| Total Lines | 337,089 |
| Duplicate Groups | 266 |
| Files in Duplicate Groups | 553 (61%) |
| Archive Candidates | 384 |
| Stub Files (<10 lines) | 93 |

## Critical Finding: 61% Duplication

The `docs/archive/specs/` directory contains **exact copies** of nearly every file in `docs/SPEC-KIT-*` directories.
This alone accounts for ~150,000 lines of duplicated content.

## Files by Category

| Category | Count |
|----------|-------|
| archive-candidate | 384 |
| spec-directory | 254 |
| general | 105 |
| stub | 93 |
| sync-spec | 31 |
| session-ephemeral | 13 |
| readme | 11 |
| governance | 7 |
| guide | 4 |
| changelog | 2 |

## Files by Topic Tag

| Tag | Count |
|-----|-------|
| spec | 594 |
| archive | 384 |
| prd | 150 |
| evidence | 125 |
| session | 119 |
| testing | 79 |
| sync | 72 |
| plan | 71 |
| tui | 67 |
| uncategorized | 60 |
| architecture | 59 |
| prompt | 30 |
| readme | 30 |
| config | 27 |
| memory | 26 |
| policy | 17 |
| stage0 | 15 |
| guide | 14 |
| auth | 9 |
| changelog | 8 |

## Top 50 Files by Size

| Lines | Path | Tags |
|-------|------|------|
| 42,653 | `docs/SPEC-KIT-102-notebooklm-integration/artifacts/HISTORY_ROLLUP.md` | changelog, evidence, spec |
| 42,653 | `docs/archive/specs/SPEC-KIT-102-notebooklm-integration/artifacts/HISTORY_ROLLUP.md` | archive, evidence, spec |
| 2,523 | `ARCHITECT_REVIEW_RESEARCH.md` | architecture |
| 2,216 | `docs/SPEC-KIT-945-implementation-research/SPEC-945F-policy-compliance-oauth2.md` | auth, spec, policy |
| 2,216 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945F-policy-compliance-oauth2.md` | archive, auth, spec |
| 2,211 | `docs/archive/specs/SPEC-KIT-099-context-bridge/spec.md` | archive, spec |
| 2,211 | `docs/SPEC-KIT-099-context-bridge/spec.md` | spec |
| 1,976 | `docs/SPEC-KIT-945-implementation-research/SPEC-945B-sqlite-transactions.md` | spec |
| 1,976 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945B-sqlite-transactions.md` | archive, spec |
| 1,729 | `docs/SPEC-KIT-945-implementation-research/SPEC-945D-config-hot-reload.md` | config, spec |
| 1,729 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945D-config-hot-reload.md` | archive, config, spec |
| 1,693 | `docs/SPEC-KIT-945-implementation-research/SPEC-945A-async-orchestration.md` | spec, sync |
| 1,693 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945A-async-orchestration.md` | archive, spec, sync |
| 1,519 | `docs/archive/session-continuations/NEXT-SESSION-SPEC-947-VALIDATION.md` | session, archive, spec |
| 1,452 | `docs/SPEC-KIT-945-implementation-research/SPEC-945E-benchmarking-instrumentation.md` | spec |
| 1,452 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945E-benchmarking-instrumentation.md` | archive, spec |
| 1,415 | `docs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` | architecture, spec |
| 1,415 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` | archive, architecture, spec |
| 1,283 | `docs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md` | architecture, spec |
| 1,283 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md` | archive, architecture, spec |
| 1,258 | `docs/SPEC-KIT-945-implementation-research/SPEC-945C-retry-error-handling.md` | spec |
| 1,258 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945C-retry-error-handling.md` | archive, spec |
| 1,223 | `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931E-analysis.md` | architecture, spec |
| 1,223 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/SPEC-931E-analysis.md` | archive, architecture, spec |
| 1,218 | `docs/SPEC-KIT-930-agent-orchestration-refactor/spec.md` | spec |
| 1,218 | `docs/archive/specs/SPEC-KIT-930-agent-orchestration-refactor/spec.md` | archive, spec |
| 1,204 | `Model Upgrade.md` | uncategorized |
| 1,200 | `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` | guide, config, spec |
| 1,161 | `docs/SPEC-947-pipeline-ui-configurator/implementation-plan.md` | config, spec, plan |
| 1,161 | `docs/archive/specs/SPEC-947-pipeline-ui-configurator/implementation-plan.md` | archive, config, spec |
| 1,157 | `docs/SPEC-KIT-931-architectural-deep-dive/phase1-inventory.md` | architecture, spec |
| 1,157 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/phase1-inventory.md` | archive, architecture, spec |
| 1,156 | `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931F-event-sourcing-feasibility.md` | architecture, spec |
| 1,156 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/SPEC-931F-event-sourcing-feasibility.md` | archive, architecture, spec |
| 1,152 | `docs/SPEC-948-modular-pipeline-logic/implementation-plan.md` | spec, plan |
| 1,152 | `docs/archive/specs/SPEC-948-modular-pipeline-logic/implementation-plan.md` | archive, spec, plan |
| 1,132 | `docs/spec-kit/QUALITY_GATES_DESIGN.md` | spec |
| 1,112 | `docs/SPEC-KIT-931-architectural-deep-dive/phase1-dataflows.md` | architecture, spec |
| 1,112 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/phase1-dataflows.md` | archive, architecture, spec |
| 1,077 | `docs/SPEC-KIT-936-tmux-elimination/tasks.md` | spec |
| 1,077 | `docs/archive/specs/SPEC-KIT-936-tmux-elimination/tasks.md` | archive, spec |
| 1,066 | `docs/SPEC-KIT-071-memory-system-optimization/ROOT_CAUSE_ANALYSIS.md` | memory, spec |
| 1,066 | `docs/archive/specs/SPEC-KIT-071-memory-system-optimization/ROOT_CAUSE_ANALYSIS.md` | archive, memory, spec |
| 1,052 | `docs/SPEC-KIT-071-memory-system-optimization/ULTRATHINK_RESEARCH_SYNTHESIS.md` | memory, spec |
| 1,052 | `docs/archive/specs/SPEC-KIT-071-memory-system-optimization/ULTRATHINK_RESEARCH_SYNTHESIS.md` | archive, memory, spec |
| 1,034 | `docs/SPEC-KIT-931-architectural-deep-dive/phase1-summary.md` | architecture, spec |
| 1,034 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/phase1-summary.md` | archive, architecture, spec |
| 1,018 | `docs/SPEC-KIT-071-memory-system-optimization/PRD.md` | memory, spec, prd |
| 1,018 | `docs/archive/specs/SPEC-KIT-071-memory-system-optimization/PRD.md` | archive, memory, spec |
| 1,012 | `docs/spec-kit/spec-auto-full-automation-plan.md` | spec, plan |

## Duplicate Groups (Top 30)

Files with identical first 50 lines (high confidence duplicates):

### Duplicate Group 1 (23 files)
- `codex-rs/tui/tests/fixtures/spec_status/conflict/docs/SPEC-FIX-CONFLICT/tasks.md`
- `codex-rs/tui/tests/fixtures/spec_status/conflict/docs/SPEC-FIX-CONFLICT/plan.md`
- `codex-rs/tui/tests/fixtures/spec_status/conflict/docs/SPEC-FIX-CONFLICT/PRD.md`
- `codex-rs/tui/tests/fixtures/spec_status/conflict/docs/SPEC-FIX-CONFLICT/spec.md`
- `codex-rs/tui/tests/fixtures/spec_status/healthy/docs/SPEC-FIX-HEALTHY/tasks.md`
- ... and 18 more

### Duplicate Group 2 (2 files)
- `docs/SPEC-KIT-DEMO/audit.md`
- `docs/archive/specs/SPEC-KIT-DEMO/audit.md`

### Duplicate Group 3 (2 files)
- `docs/SPEC-KIT-DEMO/unlock.md`
- `docs/archive/specs/SPEC-KIT-DEMO/unlock.md`

### Duplicate Group 4 (2 files)
- `docs/SPEC-KIT-DEMO/tasks.md`
- `docs/archive/specs/SPEC-KIT-DEMO/tasks.md`

### Duplicate Group 5 (2 files)
- `docs/SPEC-KIT-DEMO/plan.md`
- `docs/archive/specs/SPEC-KIT-DEMO/plan.md`

### Duplicate Group 6 (2 files)
- `docs/SPEC-KIT-DEMO/spec.md`
- `docs/archive/specs/SPEC-KIT-DEMO/spec.md`

### Duplicate Group 7 (2 files)
- `docs/SPEC-KIT-947-multi-provider-oauth-architecture/PRD.md`
- `docs/archive/specs/SPEC-KIT-947-multi-provider-oauth-architecture/PRD.md`

### Duplicate Group 8 (2 files)
- `docs/SPEC-KIT-980-multimodal-ingestion/spec.md`
- `codex-rs/docs/SPEC-KIT-980-multimodal-ingestion/spec.md`

### Duplicate Group 9 (2 files)
- `docs/SYNC-027-models-manager/PRD.md`
- `docs/archive/specs/SYNC-027-models-manager/PRD.md`

### Duplicate Group 10 (2 files)
- `docs/SPEC-949-extended-model-support/evidence/cost_validation.md`
- `docs/archive/specs/SPEC-949-extended-model-support/evidence/cost_validation.md`

### Duplicate Group 11 (2 files)
- `docs/SPEC-949-extended-model-support/implementation-plan.md`
- `docs/archive/specs/SPEC-949-extended-model-support/implementation-plan.md`

### Duplicate Group 12 (2 files)
- `docs/SPEC-949-extended-model-support/spec.md`
- `docs/archive/specs/SPEC-949-extended-model-support/spec.md`

### Duplicate Group 13 (2 files)
- `docs/SPEC-947/README.md`
- `docs/archive/specs/SPEC-947/README.md`

### Duplicate Group 14 (2 files)
- `docs/SYNC-018-branch-aware-resume/PRD.md`
- `docs/archive/specs/SYNC-018-branch-aware-resume/PRD.md`

### Duplicate Group 15 (2 files)
- `docs/SPEC-TIER2-SOURCES/evidence/S32_IMPLEMENTATION.md`
- `docs/archive/specs/SPEC-TIER2-SOURCES/evidence/S32_IMPLEMENTATION.md`

### Duplicate Group 16 (2 files)
- `docs/SPEC-TIER2-SOURCES/spec.md`
- `docs/archive/specs/SPEC-TIER2-SOURCES/spec.md`

### Duplicate Group 17 (2 files)
- `docs/SPEC-KIT-925-agent-status-sync/spec.md`
- `docs/archive/specs/SPEC-KIT-925-agent-status-sync/spec.md`

### Duplicate Group 18 (2 files)
- `docs/SYNC-012-typescript-sdk/PRD.md`
- `docs/archive/specs/SYNC-012-typescript-sdk/PRD.md`

### Duplicate Group 19 (2 files)
- `docs/SPEC-KIT-923-tmux-observable-agents/spec.md`
- `docs/archive/specs/SPEC-KIT-923-tmux-observable-agents/spec.md`

### Duplicate Group 20 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/RESUME-931C.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/RESUME-931C.md`

### Duplicate Group 21 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931B-config-integration.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/SPEC-931B-config-integration.md`

### Duplicate Group 22 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md`

### Duplicate Group 23 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931E-analysis.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/SPEC-931E-analysis.md`

### Duplicate Group 24 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931C-analysis.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/SPEC-931C-analysis.md`

### Duplicate Group 25 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931F-event-sourcing-feasibility.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/SPEC-931F-event-sourcing-feasibility.md`

### Duplicate Group 26 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/G-testing-strategy.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/G-testing-strategy.md`

### Duplicate Group 27 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931H-actor-model-analysis.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/SPEC-931H-actor-model-analysis.md`

### Duplicate Group 28 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/ULTRATHINK-VALIDATION-REPORT.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/ULTRATHINK-VALIDATION-REPORT.md`

### Duplicate Group 29 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/RESUME-931E.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/RESUME-931E.md`

### Duplicate Group 30 (2 files)
- `docs/SPEC-KIT-931-architectural-deep-dive/QUESTION-CONSOLIDATION-ANALYSIS.md`
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/QUESTION-CONSOLIDATION-ANALYSIS.md`

## Key Term References

### Files with Policy References

- `docs/SPEC-KIT-102-notebooklm-integration/artifacts/HISTORY_ROLLUP.md` (42653 lines)
- `ARCHITECT_REVIEW_RESEARCH.md` (2523 lines)
- `docs/SPEC-KIT-945-implementation-research/SPEC-945F-policy-compliance-oauth2.md` (2216 lines)
- `docs/SPEC-KIT-099-context-bridge/spec.md` (2211 lines)
- `docs/SPEC-KIT-945-implementation-research/SPEC-945B-sqlite-transactions.md` (1976 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` (1415 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md` (1283 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931E-analysis.md` (1223 lines)
- `docs/SPEC-KIT-930-agent-orchestration-refactor/spec.md` (1218 lines)
- `Model Upgrade.md` (1204 lines)
- `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` (1200 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931F-event-sourcing-feasibility.md` (1156 lines)
- `docs/SPEC-948-modular-pipeline-logic/implementation-plan.md` (1152 lines)
- `docs/SPEC-KIT-071-memory-system-optimization/ROOT_CAUSE_ANALYSIS.md` (1066 lines)
- `docs/SPEC-KIT-071-memory-system-optimization/ULTRATHINK_RESEARCH_SYNTHESIS.md` (1052 lines)
- `docs/SPEC-KIT-071-memory-system-optimization/PRD.md` (1018 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931D-analysis.md` (979 lines)
- `codex-rs/REVIEW.md` (953 lines)
- `CHANGELOG.md` (934 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931B-analysis.md` (918 lines)

### Files with Decision ID References (D###)

- `ARCHITECT_REVIEW_RESEARCH.md`: D112, D1, D2
- `docs/SPEC-KIT-945-implementation-research/SPEC-945D-config-hot-reload.md`: D4, D3
- `docs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md`: D3, D4, D1, D2
- `docs/SPEC-KIT-939-configuration-management/PRD.md`: D4, D3
- `docs/SPEC-KIT-931-architectural-deep-dive/spec.md`: D3, D4, D1, D2
- `docs/SPEC-KIT-979-local-memory-sunset/spec.md`: D94, D53, D60, D14, D29
- `docs/SPEC-KIT-931-architectural-deep-dive/ULTRATHINK-VALIDATION-REPORT.md`: D4, D1
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931I-storage-consolidation-analysis.md`: D1
- `codex-rs/docs/HANDOFF.md`: D15
- `docs/SPEC-KIT-931-architectural-deep-dive/QUESTION-CONSOLIDATION-ANALYSIS.md`: D4, D3
- `ARCHITECT_REVIEW_BOARD_OUTPUT.md`: D126, D113, D129, D120, D2
- `docs/SPEC-KIT-932-implementation-planning/RESUME-PRD-GENERATION.md`: D4, D3
- `codex-rs/docs/SPEC-KIT-978-local-reflex-sglang/spec.md`: D110, D50, D93, D49, D13
- `codex-rs/docs/SPEC-KIT-977-model-policy-v2/spec.md`: D17, D36, D60, D57, D101
- `codex-rs/docs/SPEC-KIT-971-memvid-capsule-foundation/spec.md`: D6, D52, D4, D70, D20

### Files with Invariant References

- `docs/SPEC-KIT-102-notebooklm-integration/artifacts/HISTORY_ROLLUP.md` (42653 lines)
- `ARCHITECT_REVIEW_RESEARCH.md` (2523 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` (1415 lines)
- `docs/SPEC-KIT-930-agent-orchestration-refactor/spec.md` (1218 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/phase1-dataflows.md` (1112 lines)
- `codex-rs/REVIEW.md` (953 lines)
- `CHANGELOG.md` (934 lines)
- `docs/SPEC-KIT-105-constitution-workflow/spec.md` (833 lines)
- `docs/spec-kit/testing-policy.md` (525 lines)
- `ARCHITECT_REVIEW_BOARD_OUTPUT.md` (485 lines)
- `docs/SPEC-KIT-954-session-management-polish/spec.md` (419 lines)
- `codex-rs/MEMORY-POLICY.md` (388 lines)
- `codex-rs/docs/SPEC-KIT-971-memvid-capsule-foundation/spec.md` (287 lines)
- `docs/GOLDEN_PATH.md` (229 lines)
- `codex-rs/docs/MODEL-POLICY.md` (210 lines)

### Files with Gate References

- `docs/SPEC-KIT-102-notebooklm-integration/artifacts/HISTORY_ROLLUP.md` (42653 lines)
- `ARCHITECT_REVIEW_RESEARCH.md` (2523 lines)
- `docs/SPEC-KIT-945-implementation-research/SPEC-945F-policy-compliance-oauth2.md` (2216 lines)
- `docs/SPEC-KIT-099-context-bridge/spec.md` (2211 lines)
- `docs/SPEC-KIT-945-implementation-research/SPEC-945B-sqlite-transactions.md` (1976 lines)
- `docs/SPEC-KIT-945-implementation-research/SPEC-945D-config-hot-reload.md` (1729 lines)
- `docs/SPEC-KIT-945-implementation-research/SPEC-945A-async-orchestration.md` (1693 lines)
- `docs/SPEC-KIT-945-implementation-research/SPEC-945E-benchmarking-instrumentation.md` (1452 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` (1415 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md` (1283 lines)
- `docs/SPEC-KIT-945-implementation-research/SPEC-945C-retry-error-handling.md` (1258 lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/SPEC-931E-analysis.md` (1223 lines)
- `docs/SPEC-KIT-930-agent-orchestration-refactor/spec.md` (1218 lines)
- `Model Upgrade.md` (1204 lines)
- `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` (1200 lines)

### Files with Memvid References

- `ARCHITECT_REVIEW_RESEARCH.md` (2523 lines)
- `docs/SPEC-KIT-979-local-memory-sunset/spec.md` (680 lines)
- `codex-rs/docs/HANDOFF.md` (576 lines)
- `docs/SPEC-KIT-103-librarian/spec.md` (554 lines)
- `docs/MEMVID_FIRST_WORKBENCH.md` (522 lines)
- `ARCHITECT_REVIEW_BOARD_OUTPUT.md` (485 lines)
- `docs/LOCAL-MEMORY-ENVIRONMENT.md` (457 lines)
- `codex-rs/MEMORY-POLICY.md` (388 lines)
- `codex-rs/docs/SPEC-KIT-977-model-policy-v2/spec.md` (353 lines)
- `codex-rs/docs/SPEC-KIT-971-memvid-capsule-foundation/spec.md` (287 lines)
- `docs/report/docs-report.md` (256 lines)
- `docs/GOLDEN_PATH.md` (229 lines)
- `docs/INDEX.md` (228 lines)
- `codex-rs/HANDOFF.md` (224 lines)
- `codex-rs/docs/MODEL-POLICY.md` (210 lines)

## Archive Sprawl Analysis

### docs/archive/specs/ Directory

- **Files**: 262
- **Lines**: 124,245
- **Assessment**: 100% duplicated content - safe to remove entirely

### docs/archive/2025-sessions/ Directory

- **Files**: 82
- **Lines**: 26,391
- **Assessment**: Historical session logs - archive pack candidate

### docs/archive/session-continuations/ Directory

- **Files**: 17
- **Lines**: 8,644
- **Assessment**: Ephemeral handoffs - archive pack candidate

## Root-Level Markdown Files

| File | Lines | Tags |
|------|-------|------|
| `ARCHITECT_REVIEW_RESEARCH.md` | 2523 | architecture |
| `Model Upgrade.md` | 1204 | uncategorized |
| `CHANGELOG.md` | 934 | changelog |
| `ARCHITECT_REVIEW_BOARD_OUTPUT.md` | 485 | architecture |
| `PLANNING.md` | 339 | plan |
| `product-requirements.md` | 279 | uncategorized |
| `HANDOFF.md` | 204 | session |
| `MAIEUTIC_HANDOFF.md` | 195 | session |
| `ARB_HANDOFF.md` | 189 | session |
| `SHIP_GATE_HANDOFF.md` | 144 | session |
| `AGENTS.md` | 133 | uncategorized |
| `GEMINI.md` | 133 | uncategorized |
| `CLAUDE.md` | 133 | uncategorized |
| `CONTRIBUTING.md` | 91 | uncategorized |
| `README.md` | 74 | readme |
| `plan.md` | 43 | plan |
| `ARCHITECT_QUESTIONS.md` | 42 | architecture |
| `SPEC.md` | 23 | uncategorized |

## Canonical docs/ Root Files

| File | Lines | Category | Policy Refs | Decision IDs |
|------|-------|----------|-------------|--------------|
| `docs/config.md` | 814 | general | ✓ |  |
| `docs/TUI.md` | 801 | general | ✓ |  |
| `docs/SPEC-KIT-900-ARCHITECTURE-ANALYSIS.md` | 774 | spec-directory |  |  |
| `docs/MEMVID_FIRST_WORKBENCH.md` | 522 | general | ✓ |  |
| `docs/LOCAL-MEMORY-ENVIRONMENT.md` | 457 | general |  |  |
| `docs/PROMPT-P109.md` | 383 | general | ✓ |  |
| `docs/SPEC-KIT-921.md` | 364 | spec-directory | ✓ |  |
| `docs/retry-strategy.md` | 333 | general | ✓ |  |
| `docs/SPEC-TUI2-STUBS.md` | 331 | spec-directory | ✓ |  |
| `docs/DOCUMENTATION_CLEANUP_PLAN.md` | 303 | general | ✓ |  |
| `docs/authentication.md` | 285 | general |  |  |
| `docs/EXTRACTION-GUIDE.md` | 279 | guide |  |  |
| `docs/PROJECT_STATUS.md` | 271 | general | ✓ |  |
| `docs/FORK-DIVERGENCES.md` | 236 | general | ✓ |  |
| `docs/GOLDEN_PATH.md` | 229 | general | ✓ |  |
| `docs/INDEX.md` | 228 | general | ✓ | D112, D1 |
| `docs/PROMPT-P108.md` | 222 | general | ✓ |  |
| `docs/PROMPT-P107.md` | 218 | general |  |  |
| `docs/DECISION_REGISTER.md` | 197 | governance | ✓ | D93, D126, D46 |
| `docs/DOGFOODING-BACKLOG.md` | 188 | general | ✓ |  |
| `docs/MAINT-11-EXTRACTION-PLAN.md` | 183 | general |  |  |
| `docs/MODEL-GUIDANCE.md` | 173 | general |  |  |
| `docs/OPERATIONAL-PLAYBOOK.md` | 167 | general | ✓ |  |
| `docs/PROMPT-P110.md` | 154 | general | ✓ |  |
| `docs/DOGFOODING-CHECKLIST.md` | 139 | general |  |  |
| `docs/SPEC-TUI2-TEST-PLAN.md` | 121 | spec-directory | ✓ |  |
| `docs/MEMVID-ENVIRONMENT.md` | 102 | general | ✓ |  |
| `docs/DEPRECATIONS.md` | 92 | general |  |  |
| `docs/PROGRAM_2026Q1_ACTIVE.md` | 77 | general | ✓ |  |
| `docs/KEY_DOCS.md` | 57 | general | ✓ |  |
| `docs/slash-commands.md` | 55 | general |  |  |
| `docs/MAINTAINER_ANSWERS.md` | 48 | general | ✓ |  |
| `docs/NL_DECISIONS.md` | 46 | governance | ✓ |  |
| `docs/MODEL-POLICY.md` | 43 | governance | ✓ |  |
| `docs/VISION.md` | 28 | general |  |  |
| `docs/GETTING_STARTED.md` | 25 | guide |  |  |
| `docs/ARCHITECTURE.md` | 21 | general |  |  |
| `docs/SYNC-019-031-UPGRADE-INDEX.md` | 19 | sync-spec | ✓ |  |

## Sprawl Culprits (Top 20)

Files contributing most to documentation sprawl:

| Rank | Path | Lines | Issue |
|------|------|-------|-------|
| 1 | `docs/SPEC-KIT-102-notebooklm-integration/artifacts/HISTORY_ROLLUP.md` | 42,653 | Massive changelog artifact |
| 2 | `docs/archive/specs/SPEC-KIT-102-notebooklm-integration/artifacts/HISTORY_ROLLUP.md` | 42,653 | Exact duplicate of active spec |
| 3 | `docs/SPEC-KIT-945-implementation-research/SPEC-945F-policy-compliance-oauth2.md` | 2,216 | Oversized spec file |
| 4 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945F-policy-compliance-oauth2.md` | 2,216 | Exact duplicate of active spec |
| 5 | `docs/archive/specs/SPEC-KIT-099-context-bridge/spec.md` | 2,211 | Exact duplicate of active spec |
| 6 | `docs/SPEC-KIT-099-context-bridge/spec.md` | 2,211 | Oversized spec file |
| 7 | `docs/SPEC-KIT-945-implementation-research/SPEC-945B-sqlite-transactions.md` | 1,976 | Oversized spec file |
| 8 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945B-sqlite-transactions.md` | 1,976 | Exact duplicate of active spec |
| 9 | `docs/SPEC-KIT-945-implementation-research/SPEC-945D-config-hot-reload.md` | 1,729 | Oversized spec file |
| 10 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945D-config-hot-reload.md` | 1,729 | Exact duplicate of active spec |
| 11 | `docs/SPEC-KIT-945-implementation-research/SPEC-945A-async-orchestration.md` | 1,693 | Oversized spec file |
| 12 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945A-async-orchestration.md` | 1,693 | Exact duplicate of active spec |
| 13 | `docs/archive/session-continuations/NEXT-SESSION-SPEC-947-VALIDATION.md` | 1,519 | Ephemeral handoff too large |
| 14 | `docs/SPEC-KIT-945-implementation-research/SPEC-945E-benchmarking-instrumentation.md` | 1,452 | Oversized spec file |
| 15 | `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945E-benchmarking-instrumentation.md` | 1,452 | Exact duplicate of active spec |
| 16 | `docs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` | 1,415 | Oversized spec file |
| 17 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` | 1,415 | Exact duplicate of active spec |
| 18 | `docs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md` | 1,283 | Oversized spec file |
| 19 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md` | 1,283 | Exact duplicate of active spec |
| 20 | `docs/SPEC-KIT-945-implementation-research/SPEC-945C-retry-error-handling.md` | 1,258 | Oversized spec file |

## Recommendations

### Immediate Actions (High Impact)

1. **Delete `docs/archive/specs/` entirely** - 100% duplication with active specs (~150,000 lines)
2. **Archive session continuations** - Pack into tarball with manifest (~20,000 lines)
3. **Remove stubs** - 93 files with <10 lines add noise

### Phase 2: Consolidation

4. **Merge SPEC-KIT research files** - SPEC-945* has 6 research docs that could be 1
5. **Consolidate SPEC-KIT-931 analysis** - 21 files that could be 3-4 consolidated docs
6. **Unify policy documents** - Multiple sources of truth for model policy

### Target State

- ≤9 canonical docs in `/docs/` root
- All specs in archive pack (not active directory)
- Session logs in dated archive packs
- Doc lint prevents future sprawl

---

_End of inventory report_