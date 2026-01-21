# Documentation Consolidation - Session 2 Report

**Date**: 2026-01-21
**Phase**: Blueprint (Complete Mapping)
**Session**: 2

## Canonical Docs Set (9 files)

| # | Canonical File | Purpose | Status |
|---|----------------|---------|--------|
| 1 | `docs/INDEX.md` | Navigation hub | EXISTS - extend |
| 2 | `docs/ARCHITECTURE.md` | System design | EXISTS - extend |
| 3 | `docs/POLICY.md` | All policies consolidated | CREATE |
| 4 | `docs/DECISIONS.md` | Decision register | CREATE from DECISION_REGISTER.md |
| 5 | `docs/PROGRAM.md` | Active program | CREATE from PROGRAM_2026Q1_ACTIVE.md |
| 6 | `docs/GOLDEN_PATH.md` | Primary workflow | EXISTS - extend |
| 7 | `docs/OPERATIONS.md` | Operational guidance | CREATE |
| 8 | `docs/CONTRIBUTING.md` | Contributor guide | CREATE |
| 9 | `docs/SPEC-KIT-REFERENCE.md` | Framework reference | CREATE |

**Protected:** `SPEC.md` at repo root (entrypoint contract)

---

## Complete Mapping Table

### Legend
- **KEEP**: Canonical doc (one of the 9)
- **MERGE**: Content merges into a canonical doc
- **ARCHIVE**: Goes into archive pack (recoverable)
- **DELETE**: Provably duplicated + archived, or stub (<10 lines)

### Summary Statistics

| Decision | Files | Lines | % of Total |
|----------|-------|-------|------------|
| ARCHIVE | 615 | 294,657 | 87.4% |
| MERGE | 59 | 18,958 | 5.6% |
| KEEP | 65 | 11,901 | 3.5% |
| DELETE | 110 | 562 | 0.2% |
| PROTECTED | 1 | 23 | <0.1% |
| REVIEW | 54 | 10,988 | 3.3% |

---

## ROOT-LEVEL FILES

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `SPEC.md` | PROTECTED | — | Repo entrypoint contract |
| `CLAUDE.md` | KEEP | — | AI agent config (required) |
| `AGENTS.md` | KEEP | — | AI agent config (multi-agent) |
| `GEMINI.md` | KEEP | — | AI agent config (Gemini) |
| `README.md` | KEEP | — | Repo readme (required) |
| `CONTRIBUTING.md` | MERGE | docs/CONTRIBUTING.md | Contributor guide |
| `CHANGELOG.md` | ARCHIVE | docs-pack-changelog | Historical changelog |
| `HANDOFF.md` | ARCHIVE | docs-pack-ephemeral | Session handoff |
| `ARB_HANDOFF.md` | ARCHIVE | docs-pack-ephemeral | Session handoff |
| `MAIEUTIC_HANDOFF.md` | ARCHIVE | docs-pack-ephemeral | Session handoff |
| `SHIP_GATE_HANDOFF.md` | ARCHIVE | docs-pack-ephemeral | Session handoff |
| `PLANNING.md` | ARCHIVE | docs-pack-ephemeral | Planning artifact |
| `product-requirements.md` | ARCHIVE | docs-pack-ephemeral | Requirements doc |
| `Model Upgrade.md` | ARCHIVE | docs-pack-ephemeral | Upgrade notes |
| `ARCHITECT_QUESTIONS.md` | ARCHIVE | docs-pack-research | One-time research |
| `ARCHITECT_REVIEW_RESEARCH.md` | ARCHIVE | docs-pack-research | One-time research |
| `ARCHITECT_REVIEW_BOARD_OUTPUT.md` | ARCHIVE | docs-pack-research | One-time research |
| `plan.md` | DELETE | — | Stub (43 lines, outdated) |

---

## docs/ ROOT FILES

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/INDEX.md` | KEEP | CANONICAL #1 | Navigation hub |
| `docs/ARCHITECTURE.md` | KEEP | CANONICAL #2 | System design |
| `docs/GOLDEN_PATH.md` | KEEP | CANONICAL #6 | Primary workflow |
| `docs/KEY_DOCS.md` | MERGE | docs/INDEX.md | Nav content → INDEX |
| `docs/DECISION_REGISTER.md` | MERGE | docs/DECISIONS.md | → CANONICAL #4 |
| `docs/PROGRAM_2026Q1_ACTIVE.md` | MERGE | docs/PROGRAM.md | → CANONICAL #5 |
| `docs/MODEL-POLICY.md` | MERGE | docs/POLICY.md | → CANONICAL #3 |
| `docs/OPERATIONAL-PLAYBOOK.md` | MERGE | docs/OPERATIONS.md | → CANONICAL #7 |
| `docs/config.md` | MERGE | docs/OPERATIONS.md | → CANONICAL #7 |
| `docs/GETTING_STARTED.md` | MERGE | docs/CONTRIBUTING.md | → CANONICAL #8 |
| `docs/MODEL-GUIDANCE.md` | MERGE | docs/POLICY.md | Model guidance → POLICY |
| `docs/TUI.md` | MERGE | docs/ARCHITECTURE.md | TUI arch → ARCHITECTURE |
| `docs/NL_DECISIONS.md` | MERGE | docs/DECISIONS.md | NL decisions → DECISIONS |
| `docs/DEPRECATIONS.md` | ARCHIVE | docs-pack-reference | Reference doc |
| `docs/VISION.md` | ARCHIVE | docs-pack-reference | Vision statement |
| `docs/slash-commands.md` | MERGE | docs/SPEC-KIT-REFERENCE.md | CLI reference |
| `docs/authentication.md` | MERGE | docs/OPERATIONS.md | Auth ops → OPERATIONS |
| `docs/retry-strategy.md` | ARCHIVE | docs-pack-reference | Reference doc |
| `docs/FORK-DIVERGENCES.md` | ARCHIVE | docs-pack-reference | Fork tracking |
| `docs/PROJECT_STATUS.md` | ARCHIVE | docs-pack-ephemeral | Status snapshot |
| `docs/DOGFOODING-BACKLOG.md` | ARCHIVE | docs-pack-ephemeral | Backlog snapshot |
| `docs/DOGFOODING-CHECKLIST.md` | ARCHIVE | docs-pack-ephemeral | Checklist |
| `docs/DOCUMENTATION_CLEANUP_PLAN.md` | DELETE | — | Superseded by this work |
| `docs/MEMVID_FIRST_WORKBENCH.md` | ARCHIVE | docs-pack-reference | Memvid reference |
| `docs/LOCAL-MEMORY-ENVIRONMENT.md` | ARCHIVE | docs-pack-reference | Memory env setup |
| `docs/MEMVID-ENVIRONMENT.md` | ARCHIVE | docs-pack-reference | Memvid env |
| `docs/EXTRACTION-GUIDE.md` | ARCHIVE | docs-pack-reference | Extraction guide |
| `docs/MAINT-11-EXTRACTION-PLAN.md` | ARCHIVE | docs-pack-reference | Maint plan |
| `docs/MAINTAINER_ANSWERS.md` | ARCHIVE | docs-pack-reference | Q&A archive |
| `docs/SYNC-019-031-UPGRADE-INDEX.md` | ARCHIVE | docs-pack-specs | Sync upgrade index |
| `docs/SPEC-KIT-900-ARCHITECTURE-ANALYSIS.md` | ARCHIVE | docs-pack-specs | Analysis doc |
| `docs/SPEC-KIT-921.md` | ARCHIVE | docs-pack-specs | Spec doc |
| `docs/SPEC-TUI2-STUBS.md` | ARCHIVE | docs-pack-specs | Stub tracking |
| `docs/SPEC-TUI2-TEST-PLAN.md` | ARCHIVE | docs-pack-specs | Test plan |
| `docs/PROMPT-P107.md` | ARCHIVE | docs-pack-prompts | Prompt artifact |
| `docs/PROMPT-P108.md` | ARCHIVE | docs-pack-prompts | Prompt artifact |
| `docs/PROMPT-P109.md` | ARCHIVE | docs-pack-prompts | Prompt artifact |
| `docs/PROMPT-P110.md` | ARCHIVE | docs-pack-prompts | Prompt artifact |

---

## docs/stage0/ (11 files → MERGE into GOLDEN_PATH)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/stage0/STAGE0_SPECKITAUTO_INTEGRATION.md` | MERGE | docs/GOLDEN_PATH.md#stage0-integration | Core workflow |
| `docs/stage0/STAGE0_TIER2_PROMPT.md` | MERGE | docs/GOLDEN_PATH.md#tier2-prompt | Prompt template |
| `docs/stage0/STAGE0_IQO_PROMPT.md` | MERGE | docs/GOLDEN_PATH.md#iqo-prompt | IQO prompt |
| `docs/stage0/STAGE0_IMPLEMENTATION_GUIDE.md` | MERGE | docs/GOLDEN_PATH.md#implementation | Implementation |
| `docs/stage0/STAGE0_TASK_BRIEF_TEMPLATE.md` | MERGE | docs/GOLDEN_PATH.md#task-brief | Task template |
| `docs/stage0/STAGE0_SCORING_AND_DCC.md` | MERGE | docs/GOLDEN_PATH.md#scoring | Scoring |
| `docs/stage0/STAGE0_ERROR_TAXONOMY.md` | MERGE | docs/GOLDEN_PATH.md#errors | Error handling |
| `docs/stage0/STAGE0_CONFIG_AND_PROMPTS.md` | MERGE | docs/GOLDEN_PATH.md#config | Config |
| `docs/stage0/STAGE0_GUARDIANS_AND_ORCHESTRATION.md` | MERGE | docs/GOLDEN_PATH.md#guardians | Guardians |
| `docs/stage0/STAGE0_METRICS.md` | MERGE | docs/GOLDEN_PATH.md#metrics | Metrics |
| `docs/stage0/STAGE0_OBSERVABILITY.md` | MERGE | docs/GOLDEN_PATH.md#observability | Observability |

---

## docs/spec-kit/ (40 files → MERGE into SPEC-KIT-REFERENCE)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/spec-kit/README.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#overview | Overview |
| `docs/spec-kit/ARCHITECTURE.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#architecture | Architecture |
| `docs/spec-kit/CLI-REFERENCE.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#cli | CLI reference |
| `docs/spec-kit/COMMAND_INVENTORY.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#commands | Commands |
| `docs/spec-kit/COMMAND_REGISTRY_DESIGN.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#registry | Registry design |
| `docs/spec-kit/COMMAND_REGISTRY_TESTS.md` | ARCHIVE | docs-pack-specs | Test docs |
| `docs/spec-kit/GATE_POLICY.md` | MERGE | docs/POLICY.md#gates | Gate policy |
| `docs/spec-kit/HANDOFF.md` | ARCHIVE | docs-pack-ephemeral | Handoff |
| `docs/spec-kit/HERMETIC-ISOLATION.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#isolation | Isolation |
| `docs/spec-kit/MIGRATION_GUIDE.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#migration | Migration |
| `docs/spec-kit/MULTI-AGENT-ARCHITECTURE.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#multi-agent | Multi-agent |
| `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#pipeline | Pipeline config |
| `docs/spec-kit/PROVIDER_SETUP_GUIDE.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#providers | Provider setup |
| `docs/spec-kit/QUALITY_GATE_EXPERIMENT.md` | ARCHIVE | docs-pack-specs | Experiment |
| `docs/spec-kit/QUALITY_GATES_CONFIGURATION.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#quality-gates | QG config |
| `docs/spec-kit/QUALITY_GATES_DESIGN.md` | ARCHIVE | docs-pack-specs | Design doc |
| `docs/spec-kit/QUALITY_GATES_SPECIFICATION.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#qg-spec | QG spec |
| `docs/spec-kit/TEMPLATES.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#templates | Templates |
| `docs/spec-kit/TESTING_INFRASTRUCTURE.md` | ARCHIVE | docs-pack-specs | Test infra |
| `docs/spec-kit/adoption-dashboard.md` | ARCHIVE | docs-pack-ephemeral | Dashboard |
| `docs/spec-kit/consensus-cost-audit-packet.md` | ARCHIVE | docs-pack-specs | Audit |
| `docs/spec-kit/consensus-degradation-playbook.md` | MERGE | docs/OPERATIONS.md#consensus | Ops playbook |
| `docs/spec-kit/consensus-runner-design.md` | ARCHIVE | docs-pack-specs | Design doc |
| `docs/spec-kit/ensemble-run-checklist.md` | MERGE | docs/OPERATIONS.md#ensemble | Ops checklist |
| `docs/spec-kit/evidence-baseline.md` | ARCHIVE | docs-pack-specs | Baseline |
| `docs/spec-kit/evidence-policy.md` | MERGE | docs/POLICY.md#evidence | Evidence policy |
| `docs/spec-kit/model-strategy.md` | MERGE | docs/POLICY.md#model-strategy | Model strategy |
| `docs/spec-kit/multi-agent-data-flow-recommendations.md` | ARCHIVE | docs-pack-specs | Recommendations |
| `docs/spec-kit/new-spec-command.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#new-spec | New spec cmd |
| `docs/spec-kit/qa-sweep-checklist.md` | MERGE | docs/OPERATIONS.md#qa-sweep | Ops checklist |
| `docs/spec-kit/security-review-template.md` | MERGE | docs/OPERATIONS.md#security | Security |
| `docs/spec-kit/spec-auto-automation.md` | MERGE | docs/SPEC-KIT-REFERENCE.md#automation | Automation |
| `docs/spec-kit/spec-auto-full-automation-plan.md` | ARCHIVE | docs-pack-specs | Plan doc |
| `docs/spec-kit/testing-policy.md` | MERGE | docs/POLICY.md#testing | Testing policy |
| `docs/spec-kit/GPT5_MIGRATION_GUIDE.md` | ARCHIVE | docs-pack-specs | Migration guide |
| `docs/spec-kit/REBASE_SAFETY_MATRIX_T80-T90.md` | ARCHIVE | docs-pack-specs | Safety matrix |

---

## docs/architecture/ (2 files)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/architecture/chatwidget-structure.md` | MERGE | docs/ARCHITECTURE.md#chatwidget | Component arch |
| `docs/architecture/async-sync-boundaries.md` | MERGE | docs/ARCHITECTURE.md#async-sync | Async patterns |

---

## docs/testing/ (1 file)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/testing/TEST-ARCHITECTURE.md` | MERGE | docs/ARCHITECTURE.md#testing | Test arch |

---

## docs/adr/ (1 file)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/adr/ADR-002-tui2-purpose-and-future.md` | KEEP | — | ADR (keep separate) |

---

## docs/convergence/ (1 file)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/convergence/README.md` | ARCHIVE | docs-pack-reference | Convergence ref |

---

## docs/hal/ (1 file)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/hal/README.md` | ARCHIVE | docs-pack-reference | HAL readme |

---

## docs/handoffs/ (5 files → ARCHIVE)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/handoffs/P117-PROMPT.md` | ARCHIVE | docs-pack-prompts | Ephemeral prompt |
| `docs/handoffs/P118-PROMPT.md` | ARCHIVE | docs-pack-prompts | Ephemeral prompt |
| `docs/handoffs/P119-PROMPT.md` | ARCHIVE | docs-pack-prompts | Ephemeral prompt |
| `docs/handoffs/P120-PROMPT.md` | ARCHIVE | docs-pack-prompts | Ephemeral prompt |
| `docs/handoffs/P121-PROMPT.md` | ARCHIVE | docs-pack-prompts | Ephemeral prompt |

---

## docs/handoff/ (1 file)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/handoff/HANDOFF.md` | ARCHIVE | docs-pack-ephemeral | Session handoff |

---

## docs/report/ (1 file)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/report/docs-report.md` | ARCHIVE | docs-pack-ephemeral | Report artifact |

---

## docs/archive/ DIRECTORIES (361 files → ARCHIVE or DELETE)

### docs/archive/specs/ (262 files) - 100% DUPLICATES

| Directory | Decision | Destination | Rationale |
|-----------|----------|-------------|-----------|
| `docs/archive/specs/*` (262 files) | ARCHIVE | docs-pack-archive-specs | Exact duplicates of active specs |

**Note**: This directory mirrors `docs/SPEC-*` exactly. Pack once, then delete directory.

### docs/archive/2025-sessions/ (82 files)

| Directory | Decision | Destination | Rationale |
|-----------|----------|-------------|-----------|
| `docs/archive/2025-sessions/*` | ARCHIVE | docs-pack-sessions-2025 | Historical sessions |

### docs/archive/session-continuations/ (17 files)

| Directory | Decision | Destination | Rationale |
|-----------|----------|-------------|-----------|
| `docs/archive/session-continuations/*` | ARCHIVE | docs-pack-sessions | Session handoffs |

### docs/archive/design-docs/ (5 files)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `docs/archive/design-docs/COMMAND_NAMING_AND_MODEL_STRATEGY.md` | ARCHIVE | docs-pack-design | Design doc |
| `docs/archive/design-docs/OPTIMIZATION_ANALYSIS.md` | ARCHIVE | docs-pack-design | Design doc |
| `docs/archive/design-docs/PHASE_3_STANDARDIZATION_PLAN.md` | ARCHIVE | docs-pack-design | Design doc |
| `docs/archive/design-docs/SPEC_AUTO_ORCHESTRATOR_DESIGN.md` | ARCHIVE | docs-pack-design | Design doc |
| `docs/archive/design-docs/model.md` | ARCHIVE | docs-pack-design | Design doc |

---

## docs/SPEC-*/ DIRECTORIES (246 files → ARCHIVE)

All completed SPEC directories should be archived after extracting durable insights into canonical docs.

| Pattern | Count | Decision | Destination | Rationale |
|---------|-------|----------|-------------|-----------|
| `docs/SPEC-KIT-*/**/*.md` | ~200 | ARCHIVE | docs-pack-specs | Completed specs |
| `docs/SPEC-*/**/*.md` | ~46 | ARCHIVE | docs-pack-specs | Completed specs |
| `docs/SYNC-*/**/*.md` | ~31 | ARCHIVE | docs-pack-specs | Sync specs |

**Notable large specs to archive**:
- `docs/SPEC-KIT-102-notebooklm-integration/` (42K lines HISTORY_ROLLUP)
- `docs/SPEC-KIT-945-implementation-research/` (6 research docs, 10K lines)
- `docs/SPEC-KIT-931-architectural-deep-dive/` (21 analysis docs, 15K lines)

---

## codex-rs/ CODEBASE DOCS (62 files → KEEP SEPARATE)

These are codebase-specific docs that live with the code. Review in separate session.

| Pattern | Count | Decision | Rationale |
|---------|-------|----------|-----------|
| `codex-rs/README.md` | 1 | KEEP | Crate readme |
| `codex-rs/*/README.md` | 12 | KEEP | Crate readmes |
| `codex-rs/docs/**/*.md` | 30 | KEEP | Crate docs |
| `codex-rs/tui/**/*.md` | 8 | KEEP | TUI docs |
| `codex-rs/core/**/*.md` | 7 | KEEP | Core docs |
| `codex-rs/.code/*.md` | 2 | KEEP | Code artifacts |

**Exception - Archive these**:
- `codex-rs/docs/SPEC-KIT-*/*.md` → docs-pack-specs (completed specs)

---

## STUB FILES TO DELETE (110 files, <10 lines each)

| Pattern | Count | Lines | Rationale |
|---------|-------|-------|-----------|
| Test fixtures | 23 | ~150 | Test scaffolding |
| Baseline files | 70 | ~560 | Telemetry baselines |
| Empty stubs | 17 | <70 | Placeholder files |

---

## docs/_work/ (4 files → KEEP)

| Source Path | Decision | Rationale |
|-------------|----------|-----------|
| `docs/_work/docs_inventory.md` | KEEP | Consolidation work |
| `docs/_work/docs_inventory.json` | KEEP | Machine-readable |
| `docs/_work/docs_mapping.md` | KEEP | Mapping work |
| `docs/_work/docs_mapping.json` | KEEP | Machine-readable |

---

## memory/ (2 files)

| Source Path | Decision | Destination | Rationale |
|-------------|----------|-------------|-----------|
| `memory/constitution.md` | KEEP | — | Project charter |
| `memory/local-notes.md` | KEEP | — | Local notes |

---

## ARCHIVE PACK STRUCTURE

| Pack Name | Contents | Est. Files | Est. Lines |
|-----------|----------|------------|------------|
| `docs-pack-archive-specs` | docs/archive/specs/* | 262 | 124,245 |
| `docs-pack-specs` | docs/SPEC-*/, docs/SYNC-*/ | 246 | 130,391 |
| `docs-pack-sessions-2025` | docs/archive/2025-sessions/* | 82 | 26,391 |
| `docs-pack-sessions` | session continuations | 17 | 8,644 |
| `docs-pack-ephemeral` | handoffs, planning | 15 | 3,500 |
| `docs-pack-prompts` | PROMPT-*, handoffs/P* | 10 | 1,500 |
| `docs-pack-research` | ARCHITECT_*, one-time research | 5 | 5,000 |
| `docs-pack-design` | design-docs/* | 5 | 1,300 |
| `docs-pack-reference` | misc reference docs | 15 | 2,500 |
| `docs-pack-changelog` | CHANGELOG.md | 1 | 934 |

**Total to archive**: ~658 files, ~304,000 lines

---

## NEXT SESSION TASKS

1. [ ] Create archive pack infrastructure verification
2. [ ] Pack `docs/archive/specs/` (safest - 100% duplicate)
3. [ ] Delete `docs/archive/specs/` after verified pack
4. [ ] Pack remaining archive directories
5. [ ] Begin MERGE operations for canonical docs
6. [ ] Update doc_lint with sprawl prevention

---

## STOP CONDITIONS

Session 2 is COMPLETE when:
- [x] Canonical docs set defined (9 files)
- [x] Complete mapping table produced
- [x] Every markdown file has a disposition
- [ ] (Session 3+) Archive packs created
- [ ] (Session 3+) Merges executed
- [ ] (Session 3+) Sprawl prevention enabled

---

_Session Status: BLUEPRINT COMPLETE - Ready for execution in Session 3_
