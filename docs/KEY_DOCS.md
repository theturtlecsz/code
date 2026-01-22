# Key Documentation Files

This repo has a small set of canonical documents. If you’re unsure what to read or update, start here.

| File                                                        | Type               | Purpose                                                            |
| ----------------------------------------------------------- | ------------------ | ------------------------------------------------------------------ |
| [`docs/KEY_DOCS.md`](KEY_DOCS.md)                           | Map                | Canonical doc map (this file)                                      |
| [`docs/POLICY.md`](POLICY.md)                               | Policy             | Consolidated policy (model, gates, evidence, testing)              |
| [`docs/OPERATIONS.md`](OPERATIONS.md)                       | Operations         | Consolidated operations (playbook + config reference)              |
| [`docs/ARCHITECTURE.md`](ARCHITECTURE.md)                   | Architecture       | System architecture (TUI, async/sync, pipeline, consensus)         |
| [`docs/CONTRIBUTING.md`](CONTRIBUTING.md)                   | Contributing       | Development workflow, fork management, rebase strategy             |
| [`docs/STAGE0-REFERENCE.md`](STAGE0-REFERENCE.md)           | Reference          | Stage 0 engine: integration, DCC, scoring, configuration           |
| [`docs/DECISIONS.md`](DECISIONS.md)                         | Decisions          | Locked decisions register (D1-D134)                                |
| [`docs/PROGRAM.md`](PROGRAM.md)                             | Program            | Active specs, dependency DAG, sequencing gates (2026-Q1)           |
| [`docs/SPEC-KIT.md`](SPEC-KIT.md)                           | Spec-Kit Reference | Commands, execution model, quality gates, multi-agent architecture |
| [`memory/constitution.md`](../memory/constitution.md)       | Charter            | Guardrails, principles, governance rules (ACE-compatible bullets)  |
| [`docs/VISION.md`](VISION.md)                               | Vision             | Product identity — Planner (terminal TUI) for Spec‑Kit workflows   |
| [`product-requirements.md`](../product-requirements.md)     | PRD                | Planner scope and requirements (Spec‑Kit workflows)                |
| [`SPEC.md`](../SPEC.md)                                     | Tracker            | Canonical task tracking (single source of truth)                   |
| [`CLAUDE.md`](../CLAUDE.md)                                 | Agent Instructions | Build commands, style, quirks                                      |
| [Feature PRDs](#feature-prds-spec-kit)                      | Feature PRDs       | Per‑feature requirements (Spec‑Kit)                                |
| [`templates/PRD-template.md`](../templates/PRD-template.md) | Template           | PRD template — standard format                                     |
| [`memory/local-notes.md`](../memory/local-notes.md)         | Notes              | Project-specific notes                                             |

## Feature PRDs (Spec-Kit)

These PRDs are canonical and editable.

* [`docs/SPEC-KIT-010-local-memory-migration/PRD.md`](SPEC-KIT-010-local-memory-migration/PRD.md)
* [`docs/SPEC-KIT-013-telemetry-schema-guard/PRD.md`](SPEC-KIT-013-telemetry-schema-guard/PRD.md)
* [`docs/SPEC-KIT-014-docs-refresh/PRD.md`](SPEC-KIT-014-docs-refresh/PRD.md)
* [`docs/SPEC-KIT-015-nightly-sync/PRD.md`](SPEC-KIT-015-nightly-sync/PRD.md)
* [`docs/SPEC-KIT-025-add-automated-conflict-resolution-with/PRD.md`](SPEC-KIT-025-add-automated-conflict-resolution-with/PRD.md)
* [`docs/SPEC-KIT-030-add-documentation-for-rebasing-from/PRD.md`](SPEC-KIT-030-add-documentation-for-rebasing-from/PRD.md)
* [`docs/SPEC-KIT-035-spec-status-diagnostics/PRD.md`](SPEC-KIT-035-spec-status-diagnostics/PRD.md)
* [`docs/SPEC-KIT-040-add-simple-config-validation-utility/PRD.md`](SPEC-KIT-040-add-simple-config-validation-utility/PRD.md)
* [`docs/SPEC-KIT-045-design-systematic-testing-framework-for/PRD.md`](SPEC-KIT-045-design-systematic-testing-framework-for/PRD.md)
* [`docs/SPEC-KIT-066-native-tool-migration/PRD.md`](SPEC-KIT-066-native-tool-migration/PRD.md)
* [`docs/SPEC-KIT-067-add-search-command-to-find-text-in-conversation-history/PRD.md`](SPEC-KIT-067-add-search-command-to-find-text-in-conversation-history/PRD.md)
* [`docs/SPEC-KIT-068-analyze-and-fix-quality-gates/PRD.md`](SPEC-KIT-068-analyze-and-fix-quality-gates/PRD.md)
* [`docs/SPEC-KIT-069-address-speckit-validate-multiple-agent-calls-and-incorrect-spawning/PRD.md`](SPEC-KIT-069-address-speckit-validate-multiple-agent-calls-and-incorrect-spawning/PRD.md)
* [`docs/SPEC-KIT-070-model-cost-optimization/PRD.md`](SPEC-KIT-070-model-cost-optimization/PRD.md)
* [`docs/SPEC-KIT-071-memory-system-optimization/PRD.md`](SPEC-KIT-071-memory-system-optimization/PRD.md)
* [`docs/SPEC-KIT-072-consensus-storage-separation/PRD.md`](SPEC-KIT-072-consensus-storage-separation/PRD.md)
* [`docs/SPEC-KIT-900/PRD.md`](SPEC-KIT-900/PRD.md)
* [`docs/SPEC-KIT-902-nativize-guardrails/PRD.md`](SPEC-KIT-902-nativize-guardrails/PRD.md)
* [`docs/SPEC-KIT-909-evidence-cleanup-automation/PRD.md`](SPEC-KIT-909-evidence-cleanup-automation/PRD.md)
* [`docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md`](SPEC-KIT-933-database-integrity-hygiene/PRD.md)
* [`docs/SPEC-KIT-934-storage-consolidation/PRD.md`](SPEC-KIT-934-storage-consolidation/PRD.md)
* [`docs/SPEC-KIT-936-tmux-elimination/PRD.md`](SPEC-KIT-936-tmux-elimination/PRD.md)
* [`docs/SPEC-KIT-938-enhanced-agent-retry/PRD.md`](SPEC-KIT-938-enhanced-agent-retry/PRD.md)
* [`docs/SPEC-KIT-939-configuration-management/PRD.md`](SPEC-KIT-939-configuration-management/PRD.md)
* [`docs/SPEC-KIT-940-performance-instrumentation/PRD.md`](SPEC-KIT-940-performance-instrumentation/PRD.md)
* [`docs/SPEC-KIT-941-automated-policy-compliance/PRD.md`](SPEC-KIT-941-automated-policy-compliance/PRD.md)
* [`docs/SPEC-KIT-946-model-command-expansion/PRD.md`](SPEC-KIT-946-model-command-expansion/PRD.md)
* [`docs/SPEC-KIT-947-multi-provider-oauth-architecture/PRD.md`](SPEC-KIT-947-multi-provider-oauth-architecture/PRD.md)
* [`docs/SPEC-KIT-951-multi-provider-oauth-research/PRD.md`](SPEC-KIT-951-multi-provider-oauth-research/PRD.md)
* [`docs/SPEC-KIT-952-cli-routing-multi-provider/PRD.md`](SPEC-KIT-952-cli-routing-multi-provider/PRD.md)
* [`docs/SPEC-KIT-956-config-cleanup/PRD.md`](SPEC-KIT-956-config-cleanup/PRD.md)
* [`docs/SPEC-KIT-957-specify-nativization/PRD.md`](SPEC-KIT-957-specify-nativization/PRD.md)
* [`docs/SPEC-KIT-960-speckit-project/PRD.md`](SPEC-KIT-960-speckit-project/PRD.md)
* [`docs/SPEC-KIT-961-template-ecosystem/PRD.md`](SPEC-KIT-961-template-ecosystem/PRD.md)
* [`docs/SPEC-KIT-962-template-installation/PRD.md`](SPEC-KIT-962-template-installation/PRD.md)
* [`docs/SPEC-KIT-963-upstream-deprecation/PRD.md`](SPEC-KIT-963-upstream-deprecation/PRD.md)
* [`docs/SPEC-KIT-964-config-isolation/PRD.md`](SPEC-KIT-964-config-isolation/PRD.md)
