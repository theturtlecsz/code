# Documentation Mapping for Consolidation Review

_Generated: 2026-01-21_

## Executive Summary

| Destination | Files | Lines | Action |
|-------------|-------|-------|--------|
| ARCHIVE_PACK | 615 | 294,657 | Pack into tar.zst |
| MERGE_INTO | 59 | 18,958 | Merge content |
| KEEP_SEPARATE | 62 | 11,423 | Keep separate |
| REVIEW | 54 | 10,988 | Manual review |
| DELETE | 110 | 562 | Remove |
| CANONICAL | 3 | 478 | Keep as canonical |
| PROTECTED | 1 | 23 | Do not touch |

## Proposed Canonical Docs (9)

| # | File | Purpose | Sources |
|---|------|---------|---------|
| 1 | `docs/INDEX.md` | Navigation hub | docs/INDEX.md, docs/KEY_DOCS.md |
| 2 | `docs/ARCHITECTURE.md` | System design | docs/ARCHITECTURE.md, docs/TUI.md |
| 3 | `docs/POLICY.md` | All policies consolidated | docs/MODEL-POLICY.md, docs/spec-kit/GATE_POLICY.md, docs/spec-kit/evidence-policy.md +1 more |
| 4 | `docs/DECISIONS.md` | Decision register | docs/DECISION_REGISTER.md |
| 5 | `docs/PROGRAM.md` | Active program | docs/PROGRAM_2026Q1_ACTIVE.md |
| 6 | `docs/GOLDEN_PATH.md` | Primary workflow (may replace STAGE0 + SPEC-KIT) | docs/GOLDEN_PATH.md, docs/stage0/*, docs/spec-kit/spec-auto* |
| 7 | `docs/OPERATIONS.md` | Operational guidance | docs/OPERATIONAL-PLAYBOOK.md, docs/config.md, HANDOFF.md |
| 8 | `docs/CONTRIBUTING.md` | Contributor guide | docs/GETTING_STARTED.md, CONTRIBUTING.md |
| 9 | `docs/SPEC-KIT-REFERENCE.md` | Framework reference | docs/spec-kit/*.md |

## Protected Files

- `SPEC.md` (23 lines): Repo root - entrypoint contract

## Archive Pack: docs-pack-archive-specs

**100% duplicate content** - mirror of active specs

- **Files**: 262
- **Lines**: 124,245
- **Action**: Pack and delete directory

Sample files:
- `docs/archive/specs/SPEC-KIT-102-notebooklm-integration/artifacts/HISTORY_ROLLUP.md` (42653 lines)
- `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945F-policy-compliance-oauth2.md` (2216 lines)
- `docs/archive/specs/SPEC-KIT-099-context-bridge/spec.md` (2211 lines)
- `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945B-sqlite-transactions.md` (1976 lines)
- `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945D-config-hot-reload.md` (1729 lines)
- `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945A-async-orchestration.md` (1693 lines)
- `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945E-benchmarking-instrumentation.md` (1452 lines)
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/MASTER-QUESTIONS.md` (1415 lines)
- `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/phase1-database.md` (1283 lines)
- `docs/archive/specs/SPEC-KIT-945-implementation-research/SPEC-945C-retry-error-handling.md` (1258 lines)
- ... and 252 more

## Archive Pack: docs-pack-specs

**Completed SPEC directories** - archive for historical reference

- **Files**: 246
- **Lines**: 130,391
- **Action**: Pack after extracting durable truth

## Files for Manual Review

Total: 54 files, 10,988 lines

| File | Lines | Category |
|------|-------|----------|
| `CHANGELOG.md` | 934 | changelog |
| `docs/MEMVID_FIRST_WORKBENCH.md` | 522 | general |
| `docs/architecture/async-sync-boundaries.md` | 517 | general |
| `docs/LOCAL-MEMORY-ENVIRONMENT.md` | 457 | general |
| `docs/PROMPT-P109.md` | 383 | general |
| `docs/testing/TEST-ARCHITECTURE.md` | 343 | general |
| `PLANNING.md` | 339 | general |
| `docs/retry-strategy.md` | 333 | general |
| `docs/archive/design-docs/COMMAND_NAMING_AND_MODEL_STRATEGY.md` | 324 | archive-candidate |
| `docs/archive/design-docs/OPTIMIZATION_ANALYSIS.md` | 314 | archive-candidate |
| `docs/DOCUMENTATION_CLEANUP_PLAN.md` | 303 | general |
| `docs/archive/design-docs/PHASE_3_STANDARDIZATION_PLAN.md` | 299 | archive-candidate |
| `docs/authentication.md` | 285 | general |
| `docs/EXTRACTION-GUIDE.md` | 279 | guide |
| `product-requirements.md` | 279 | general |
| `docs/PROJECT_STATUS.md` | 271 | general |
| `docs/report/docs-report.md` | 256 | general |
| `docs/FORK-DIVERGENCES.md` | 236 | general |
| `docs/archive/design-docs/model.md` | 223 | archive-candidate |
| `docs/PROMPT-P108.md` | 222 | general |
| `docs/PROMPT-P107.md` | 218 | general |
| `docs/archive/design-docs/SPEC_AUTO_ORCHESTRATOR_DESIGN.md` | 196 | archive-candidate |
| `docs/DOGFOODING-BACKLOG.md` | 188 | general |
| `docs/handoffs/P119-PROMPT.md` | 186 | session-ephemeral |
| `docs/MAINT-11-EXTRACTION-PLAN.md` | 183 | general |
| `docs/MODEL-GUIDANCE.md` | 173 | general |
| `docs/PROMPT-P110.md` | 154 | general |
| `docs/handoffs/P120-PROMPT.md` | 152 | session-ephemeral |
| `docs/handoffs/P121-PROMPT.md` | 150 | session-ephemeral |
| `docs/architecture/chatwidget-structure.md` | 145 | general |
| ... | | 24 more files |

## Files to Delete (Stubs)

Total: 110 files

- `docs/SPEC-KIT-018-hal-http-mcp/tasks.md` (9 lines)
- `docs/SPEC-KIT-045-mini/telemetry/anchors_2025-10-13T19-22-33Z.md` (9 lines)
- `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-001-clean/plan.md` (9 lines)
- `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-001-conflict/plan.md` (9 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T03:39:12Z-92930885.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T18:50:20Z-1721217636.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T20:45:45Z-652528800.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T00:25:58Z-971518482.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T00:58:25Z-1588216078.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-14T15:58:30Z-79323873.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-12T21:11:24Z-222161205.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T03:17:09Z-85944173.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T14:31:04Z-2052213612.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-12T21:06:47Z-975517254.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T15:16:57Z-293799580.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-12T21:19:19Z-1871230763.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-12T20:50:45Z-301386052.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T02:47:17Z-218242802.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-13T17:06:06Z-147197217.md` (8 lines)
- `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/baselines/baseline_2025-10-12T20:56:37Z-168709888.md` (8 lines)
- ... and 90 more

## Merge Targets

### → ARCHITECTURE
Files to merge: 1, 801 lines

- `docs/TUI.md` (801 lines)

### → CONTRIBUTING
Files to merge: 1, 25 lines

- `docs/GETTING_STARTED.md` (25 lines)

### → DECISIONS
Files to merge: 1, 197 lines

- `docs/DECISION_REGISTER.md` (197 lines)

### → GOLDEN_PATH
Files to merge: 11, 3,119 lines

- `docs/stage0/STAGE0_SPECKITAUTO_INTEGRATION.md` (638 lines)
- `docs/stage0/STAGE0_TIER2_PROMPT.md` (627 lines)
- `docs/stage0/STAGE0_IQO_PROMPT.md` (377 lines)
- `docs/stage0/STAGE0_IMPLEMENTATION_GUIDE.md` (233 lines)
- `docs/stage0/STAGE0_TASK_BRIEF_TEMPLATE.md` (204 lines)
- `docs/stage0/STAGE0_SCORING_AND_DCC.md` (200 lines)
- `docs/stage0/STAGE0_ERROR_TAXONOMY.md` (195 lines)
- `docs/stage0/STAGE0_CONFIG_AND_PROMPTS.md` (172 lines)
- `docs/stage0/STAGE0_GUARDIANS_AND_ORCHESTRATION.md` (170 lines)
- `docs/stage0/STAGE0_METRICS.md` (163 lines)
- ... and 1 more

### → INDEX
Files to merge: 1, 57 lines

- `docs/KEY_DOCS.md` (57 lines)

### → OPERATIONS
Files to merge: 2, 981 lines

- `docs/config.md` (814 lines)
- `docs/OPERATIONAL-PLAYBOOK.md` (167 lines)

### → POLICY
Files to merge: 1, 43 lines

- `docs/MODEL-POLICY.md` (43 lines)

### → PROGRAM
Files to merge: 1, 77 lines

- `docs/PROGRAM_2026Q1_ACTIVE.md` (77 lines)

### → SPEC-KIT-REFERENCE
Files to merge: 40, 13,658 lines

- `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` (1200 lines)
- `docs/spec-kit/QUALITY_GATES_DESIGN.md` (1132 lines)
- `docs/spec-kit/spec-auto-full-automation-plan.md` (1012 lines)
- `docs/spec-kit/PROVIDER_SETUP_GUIDE.md` (791 lines)
- `docs/spec-kit/QUALITY_GATES_SPECIFICATION.md` (684 lines)
- `docs/spec-kit/COMMAND_INVENTORY.md` (643 lines)
- `docs/spec-kit/REBASE_SAFETY_MATRIX_T80-T90.md` (583 lines)
- `docs/spec-kit/QUALITY_GATE_EXPERIMENT.md` (546 lines)
- `docs/spec-kit/testing-policy.md` (525 lines)
- `docs/spec-kit/GPT5_MIGRATION_GUIDE.md` (494 lines)
- ... and 30 more

## Full Mapping Table

| Path | Lines | Destination | Note |
|------|-------|-------------|------|
| `AGENTS.md` | 133 | REVIEW | Root file - needs manual review |
| `ARB_HANDOFF.md` | 189 | ARCHIVE_PACK | docs-pack-ephemeral |
| `ARCHITECT_QUESTIONS.md` | 42 | ARCHIVE_PACK | docs-pack-research (one-time) |
| `ARCHITECT_REVIEW_BOARD_OUTPUT.md` | 485 | ARCHIVE_PACK | docs-pack-research (one-time) |
| `ARCHITECT_REVIEW_RESEARCH.md` | 2523 | ARCHIVE_PACK | docs-pack-research (one-time) |
| `CHANGELOG.md` | 934 | REVIEW | Root file - needs manual review |
| `CLAUDE.md` | 133 | REVIEW | Root file - needs manual review |
| `CONTRIBUTING.md` | 91 | REVIEW | Root file - needs manual review |
| `GEMINI.md` | 133 | REVIEW | Root file - needs manual review |
| `HANDOFF.md` | 204 | ARCHIVE_PACK | docs-pack-ephemeral |
| `MAIEUTIC_HANDOFF.md` | 195 | ARCHIVE_PACK | docs-pack-ephemeral |
| `Model Upgrade.md` | 1204 | ARCHIVE_PACK | docs-pack-ephemeral |
| `PLANNING.md` | 339 | REVIEW | Root file - needs manual review |
| `README.md` | 74 | REVIEW | Root file - needs manual review |
| `SHIP_GATE_HANDOFF.md` | 144 | ARCHIVE_PACK | docs-pack-ephemeral |
| `SPEC.md` | 23 | PROTECTED | Repo root - entrypoint contract |
| `claude/prompts/spec-932-review-prompt.md` | 0 | DELETE | Stub file (<10 lines) |
| `codex-rs/.code/EXECUTION_LOGGING_DELIVERABLES.md` | 565 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/.code/execution_logging_implementation_report.md` | 496 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/.serena/memories/code_style.md` | 4 | DELETE | Stub file (<10 lines) |
| `codex-rs/.serena/memories/project_overview.md` | 4 | DELETE | Stub file (<10 lines) |
| `codex-rs/.serena/memories/suggested_commands.md` | 7 | DELETE | Stub file (<10 lines) |
| `codex-rs/.serena/memories/task_completion.md` | 4 | DELETE | Stub file (<10 lines) |
| `codex-rs/.speckit/eval/ab-report-20260121_040256.md` | 40 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/.speckit/eval/golden-path-20260112_161328.md` | 20 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/.speckit/eval/golden-path-20260112_174819.md` | 20 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/HANDOFF.md` | 224 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/MEMORY-POLICY.md` | 388 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/README.md` | 105 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/REVIEW.md` | 953 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/SPEC.md` | 189 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/UPSTREAM_SYNC.md` | 116 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/ansi-escape/README.md` | 15 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/apply-patch/apply_patch_tool_instructions.md` | 75 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/chatgpt/README.md` | 5 | DELETE | Stub file (<10 lines) |
| `codex-rs/common/README.md` | 5 | DELETE | Stub file (<10 lines) |
| `codex-rs/config.md` | 601 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/core/README.md` | 22 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/core/gpt_5_codex_prompt.md` | 104 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/core/prompt.md` | 310 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/core/prompt_coder.md` | 83 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/core/review_prompt.md` | 87 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/core/src/prompt_for_pro_observer.md` | 34 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/core/templates/compact/history_bridge.md` | 7 | DELETE | Stub file (<10 lines) |
| `codex-rs/core/templates/compact/prompt.md` | 5 | DELETE | Stub file (<10 lines) |
| `codex-rs/core/tests/README.md` | 280 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/DECISION_REGISTER.md` | 209 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/GOLD_RUN_PLAYBOOK.md` | 257 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/HANDOFF.md` | 576 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/MODEL-POLICY.md` | 210 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/NEXT_FOCUS_ROADMAP.md` | 207 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/NEXT_SESSION_PROMPT.md` | 123 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/PROGRAM_2026Q1_ACTIVE.md` | 115 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/SPEC-923-DATA-FLOW.md` | 436 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-959-streamcontroller-per-id-buffers/spec.` | 168 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-900-gold-run/spec.md` | 108 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-922-AUTO-COMMIT.md` | 251 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-923-OUTPUT-FIX.md` | 350 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-933-database-integrity-hygiene/COMPON` | 175 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-933-database-integrity-hygiene/CONTEX` | 172 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md` | 40 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-971-memvid-capsule-foundation/spec.md` | 287 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-972-hybrid-retrieval-eval/spec.md` | 63 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-973-time-travel-ui/spec.md` | 160 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-974-capsule-export-import-encryption/` | 80 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-975-replayable-audits/spec.md` | 94 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-976-logic-mesh-graph/spec.md` | 165 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-977-model-policy-v2/spec.md` | 353 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-978-local-reflex-sglang/spec.md` | 367 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-979-local-memory-sunset/spec.md` | 20 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-980-multimodal-ingestion/spec.md` | 72 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/SPEC-KIT-DEMO/plan.md` | 35 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/docs/adr/ADR-001-tui2-local-api-adaptation.md` | 143 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/architecture/phase0-baseline.md` | 81 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/archive/completed-specs/SPEC-947-pipeline-ui-c` | 1 | DELETE | Stub file (<10 lines) |
| `codex-rs/docs/codex_mcp_interface.md` | 124 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/convergence/MEMO_codex-rs.md` | 73 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/convergence/PROMPT_codex-rs.md` | 36 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/convergence/README.md` | 30 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/protocol_v1.md` | 172 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/docs/spec-kit/CONTINUATION-PROMPT-P1.md` | 155 | MERGE_INTO:SPEC-KIT-REFERENCE | docs/SPEC-KIT-REFERENCE.md |
| `codex-rs/docs/spec-kit/CONTINUATION-PROMPT-P2.md` | 276 | MERGE_INTO:SPEC-KIT-REFERENCE | docs/SPEC-KIT-REFERENCE.md |
| `codex-rs/docs/spec-kit/REVIEW-CONTRACT.md` | 167 | MERGE_INTO:SPEC-KIT-REFERENCE | docs/SPEC-KIT-REFERENCE.md |
| `codex-rs/docs/upstream/TYPE_MAPPING.md` | 298 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/execpolicy/README.md` | 180 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/file-search/README.md` | 5 | DELETE | Stub file (<10 lines) |
| `codex-rs/git-tooling/README.md` | 21 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/linux-sandbox/README.md` | 8 | DELETE | Stub file (<10 lines) |
| `codex-rs/mcp-types/README.md` | 8 | DELETE | Stub file (<10 lines) |
| `codex-rs/protocol/README.md` | 7 | DELETE | Stub file (<10 lines) |
| `codex-rs/scripts/TMUX-AUTOMATION-README.md` | 422 | KEEP_SEPARATE | Rust codebase docs - review later |
| `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/README.md` | 50 | ARCHIVE_PACK | docs-pack-specs (completed specs) |
| `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-00` | 7 | DELETE | Stub file (<10 lines) |
| `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-00` | 7 | DELETE | Stub file (<10 lines) |
| `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-00` | 9 | DELETE | Stub file (<10 lines) |
| `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-00` | 7 | DELETE | Stub file (<10 lines) |
| `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-00` | 7 | DELETE | Stub file (<10 lines) |
| `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-00` | 9 | DELETE | Stub file (<10 lines) |
| `codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/docs/SPEC-CI-00` | 8 | DELETE | Stub file (<10 lines) |
| `codex-rs/tui/ACE_LEARNING_USAGE.md` | 153 | KEEP_SEPARATE | Rust codebase docs - review later |
| ... | | | 804 more rows |

---

_This mapping requires review before execution._

_See `docs/_work/docs_mapping.json` for machine-readable version._