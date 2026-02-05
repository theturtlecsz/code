# Deprecations Register

Single canonical list of deprecated/superseded documentation and their replacements.

**Why this exists**

- Reduce roadmap/spec drift by making deprecations explicit and discoverable.
- Preserve historical context without letting legacy docs override current truth.

**Policy**

- When a doc is no longer canonical, add a **top-of-file banner** in the doc and an entry here.
- Deprecated PRDs are **archived into zip packs under `archive/` and removed from the working tree** (no stubs); this register is the canonical pointer.
- For historical/frozen docs (notably under `docs/SPEC-KIT-*`), prefer adding an entry here and only add minimal banner text when needed.

**Status meanings**

- **Deprecated**: Retained for history; do not treat as current guidance.
- **Superseded**: Replaced by a newer doc (link provided); old doc should not be updated further.
- **Needs refresh**: Still useful, but not safe to treat as canonical until updated.

## Register

| Document | Status | Replacement / Canonical Reference | Notes | Deprecated On |
| --- | --- | --- | --- | --- |
| `codex-rs/docs/NEXT_FOCUS_ROADMAP.md` | Deprecated | `SPEC.md` → `codex-rs/SPEC.md` | Historical roadmap; conflicts resolved by `codex-rs/SPEC.md` doc precedence order. | 2026-02-05 |
| `codex-rs/docs/SPEC-KIT-900-gold-run/spec.md` | Superseded | `docs/SPEC-DOGFOOD-002/spec.md` + `codex-rs/SPEC.md` (Planned) | `SPEC-KIT-900` is completed work; gold-run validation is tracked separately as `SPEC-DOGFOOD-002`. | 2026-02-05 |
| `codex-rs/docs/GOLD_RUN_PLAYBOOK.md` | Needs refresh | `docs/SPEC-DOGFOOD-002/spec.md` + `codex-rs/SPEC.md` (Planned) | Keep as playbook, but acceptance criteria lives in the SPEC and tracker. | 2026-02-05 |
| `ARB_HANDOFF.md` | Deprecated | `docs/DECISIONS.md` + `codex-rs/SPEC.md` | ARB Pass 1/2 handoff doc; superseded by locked decisions + canonical tracker; archived in `tree-pack-20260205-legacy-arb-docs.zip`. | 2026-02-05 |
| `ARCHITECT_REVIEW_RESEARCH.md` | Deprecated | `docs/DECISIONS.md` + `codex-rs/SPEC.md` | ARB Pass 1 research notes; superseded by locked decisions + canonical tracker; archived in `tree-pack-20260205-legacy-arb-docs.zip`. | 2026-02-05 |
| `ARCHITECT_QUESTIONS.md` | Deprecated | `docs/DECISIONS.md` + `codex-rs/SPEC.md` | ARB question definitions; superseded by locked decisions + canonical tracker; archived in `tree-pack-20260205-legacy-arb-docs.zip`. | 2026-02-05 |
| `ARCHITECT_REVIEW_BOARD_OUTPUT.md` | Superseded | `docs/DECISIONS.md` | ARB decision output; superseded by locked decisions register; archived in `tree-pack-20260205-legacy-arb-docs.zip`. | 2026-02-05 |
| `plan.md` | Deprecated | `docs/DECISIONS.md` + `codex-rs/SPEC.md` | ARB Pass 2 planning doc; superseded by locked decisions + canonical tracker; archived in `tree-pack-20260205-legacy-arb-docs.zip`. | 2026-02-05 |

| `codex-rs/docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-010-local-memory-migration/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-013-telemetry-schema-guard/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-014-docs-refresh/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-015-nightly-sync/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-025-add-automated-conflict-resolution-with/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-030-add-documentation-for-rebasing-from/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-035-spec-status-diagnostics/PRD.md` | Superseded | `codex-rs/SPEC.md` (Planned: SPEC-PM-001) | Legacy status diagnostics superseded by capsule-backed PM tracker/status surfaces; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-040-add-simple-config-validation-utility/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-045-design-systematic-testing-framework-for/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-066-native-tool-migration/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-067-add-search-command-to-find-text-in-conversation-history/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-068-analyze-and-fix-quality-gates/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-069-address-speckit-validate-multiple-agent-calls-and-incorrect-spawning/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-070-model-cost-optimization/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-071-memory-system-optimization/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-072-consensus-storage-separation/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-900/PRD.md` | Superseded | `docs/SPEC-DOGFOOD-002/spec.md` + `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002) | Legacy smoke scenario superseded by canonical Gold Run; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-909-evidence-cleanup-automation/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-934-storage-consolidation/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-936-tmux-elimination/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-938-enhanced-agent-retry/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-939-configuration-management/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-940-performance-instrumentation/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-941-automated-policy-compliance/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-946-model-command-expansion/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-947-multi-provider-oauth-architecture/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-951-multi-provider-oauth-research/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-956-config-cleanup/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-957-specify-nativization/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-961-template-ecosystem/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-962-template-installation/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-963-upstream-deprecation/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SPEC-KIT-964-config-isolation/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Doc Precedence + Planned A/B/C) | Legacy SPEC-KIT PRD not aligned to current vision scope; archived in `tree-pack-20260205-legacy-prds-spec-kit.zip`. | 2026-02-05 |
| `docs/SYNC-002-process-hardening/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-003-cargo-deny/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-004-async-utils/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-005-keyring-store/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-006-feedback/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-007-api-error-bridge/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-008-ascii-animation/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-009-footer-improvements/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-010-auto-drive-patterns/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-011-opentelemetry/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-012-typescript-sdk/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-013-shell-mcp/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-014-prompt-management/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-015-character-encoding/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-016-device-code-auth/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-017-review-merge-workflows/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-018-branch-aware-resume/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-019-features-registry/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-020-skills-v1/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-021-skills-v2/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-022-code-bridge/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-023-tui-perf/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-024-tui-ps/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-025-exec-hardening/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-026-retention-compaction/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-027-models-manager/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-028-tui2-scaffold/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-029-tui2-parity/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-030-requirements-policy/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |
| `docs/SYNC-031-otel-minimal/PRD.md` | Deprecated | `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002 / SPEC-PK-001 / SPEC-PM-001) | SYNC feature removed; archived in `tree-pack-20260205-legacy-prds-sync.zip` per hard rule. | 2026-02-05 |

## Planned: Capsule-backed tracking

Long-term, deprecations should be emitted as capsule events and projected into this register. Track design/implementation in `codex-rs/SPEC.md` (Planned: `SPEC-PM-001`).
