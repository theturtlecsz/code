# Documentation Index

## Canonical Docs (Source of Truth)

* `INDEX.md` (this file)
* `KEY_DOCS.md`
* `VISION.md`
* `ARCHITECTURE.md`
* `POLICY.md`
* `DECISIONS.md`
* `PROGRAM.md`
* `OPERATIONS.md`
* `STAGE0-REFERENCE.md`
* `SPEC-KIT.md`
* `CONTRIBUTING.md`
* `PRODUCT-KNOWLEDGE-SYSTEM-DESIGN.md`

***

## 2026-Q1 Active Program (authoritative)

* `PROGRAM.md` — **pinned active specs + dependency DAG + sequencing gates**
* `DECISIONS.md` — locked decisions (D1–D134)
* `POLICY.md` — consolidated policy (model, gates, evidence, testing)
* `ARCHITECTURE.md` — architecture + ADRs for the Memvid-first workbench
* `GOLDEN_PATH.md` — end-to-end walkthrough (what “good” looks like)

**Last Updated**: 2026-01-10
**Total Active Docs**: \~45 essential documents
**Archive Packs**: see `archive/` (integrity-verified packs) and `docs/archive/README.md` (restore instructions)

***

## 🎯 Start Here (Essential Reading)

**New to the project? Read these first:**

| Document                    | Purpose                                     | Status                 |
| --------------------------- | ------------------------------------------- | ---------------------- |
| **SPEC.md**                 | Task tracker, single source of truth        | ✅ Current (2026-01-10) |
| **CLAUDE.md**               | Operating guide for Claude Code             | ✅ Current (2025-10-19) |
| **product-requirements.md** | Product scope and vision                    | ✅ Current              |
| **PLANNING.md**             | High-level architecture, goals, constraints | ✅ Current              |
| **README.md**               | Project overview, quick start               | ✅ Current              |

***

## 📋 Policies & Standards

**Governance and compliance:**

| Document                   | Purpose                                                  | Status                 |
| -------------------------- | -------------------------------------------------------- | ---------------------- |
| **POLICY.md**              | Consolidated policy (model, gates, evidence, testing)    | ✅ Current (2026-01-21) |
| **OPERATIONS.md**          | Consolidated operations (playbook + config reference)    | ✅ Current (2026-01-21) |
| **ARCHITECTURE.md**        | System architecture (TUI, async/sync, pipeline)          | ✅ Current (2026-01-21) |
| **CONTRIBUTING.md**        | Development workflow, fork management, rebase strategy   | ✅ Current (2026-01-21) |
| **STAGE0-REFERENCE.md**    | Stage 0 engine: integration, DCC, scoring, configuration | ✅ Current (2026-01-22) |
| **MODEL-GUIDANCE.md**      | Model-specific reasoning and validation tiers            | ✅ Reference            |
| **UPSTREAM-SYNC.md**       | Quarterly sync process, conflict resolution              | ✅ Current              |
| **memory/constitution.md** | Project charter and guardrails                           | ✅ Current              |

**Location**: `/docs/`

***

## 🏗️ Architecture & Design

**System architecture and design decisions:**

| Document                         | Purpose                                                          | Status                 |
| -------------------------------- | ---------------------------------------------------------------- | ---------------------- |
| **ARCHITECTURE.md**              | Consolidated architecture (TUI, async/sync, pipeline, consensus) | ✅ Current (2026-01-21) |
| **SPEC\_AUTO\_FLOW\.md**         | Pipeline flow (6 stages: Plan→Unlock)                            | ✅ Reference            |
| **IMPLEMENTATION\_CONSENSUS.md** | Implementation details                                           | ✅ Reference            |
| **CONTRIBUTING.md**              | Fork workflow, rebase strategy, deviation tracking               | ✅ Current (2026-01-21) |

**Design Documents**:

* **SPEC-KIT.md** - Canonical spec-kit reference (commands, execution model, quality gates, architecture)

***

## 🧪 Testing Documentation

**Test infrastructure and plans:**

| Document                       | Purpose                                   | Status                 |
| ------------------------------ | ----------------------------------------- | ---------------------- |
| **TESTING\_INFRASTRUCTURE.md** | MockMcpManager, fixtures, tarpaulin setup | ✅ Current              |
| **PHASE3\_TEST\_PLAN.md**      | Integration tests (W/E/S/Q/C categories)  | ✅ Complete 2025-10-19  |
| **PHASE4\_TEST\_PLAN.md**      | Edge cases + property-based tests (EC/PB) | ✅ Complete 2025-10-19  |
| **POLICY.md#4-testing-policy** | Coverage goals, module targets, roadmap   | ✅ Current (2026-01-21) |

**Test Results**: 604 tests @ 100% pass rate, 42-48% estimated coverage

***

## 🔧 Implementation & Operation Guides

**How-to guides and operational procedures:**

| Document                              | Purpose                                             |
| ------------------------------------- | --------------------------------------------------- |
| **SPEC-KIT.md**                       | Canonical spec-kit reference (commands, workflows)  |
| **OPERATIONS.md**                     | Agent behavioral guidance + configuration reference |
| **spec-auto-automation.md**           | Spec-kit automation workflows                       |
| **spec-auto-full-automation-plan.md** | Full automation implementation                      |
| **MIGRATION\_GUIDE.md**               | Migration patterns and examples                     |
| **ensemble-run-checklist.md**         | Multi-agent run checklist                           |
| **COMMAND\_REGISTRY\_DESIGN.md**      | Command registry architecture                       |
| **telemetry-schema-v2.md**            | Telemetry schema specification                      |
| **CONFLICT\_RESOLUTION.md**           | Consensus conflict handling                         |

***

## 📦 Deferred & Archive

### Deferred Tasks

| Task         | Status                | Location                   |
| ------------ | --------------------- | -------------------------- |
| **MAINT-10** | Deferred indefinitely | MAINT-10-EXECUTION-PLAN.md |

**Rationale**: No CLI/API/library consumers exist (YAGNI principle)

### Archived Documentation

Large historical doc trees are packed to keep the repo navigable:

* `archive/tree-pack-20260127-docs-archive.zip` — historical `docs/archive/**`
* `archive/tree-pack-20260127-docs-_work.zip` — transient `docs/_work/**`
* `archive/tree-pack-20260127-spec-ops-004-evidence.zip` — historical `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/**`

Browse packs via:

```bash
./scripts/docs-archive-pack.sh list archive/tree-pack-20260127-docs-archive.zip
```

***

## 🔍 Finding Documentation

### By Topic

**Testing**:

* Start: POLICY.md#4-testing-policy
* Infrastructure: TESTING\_INFRASTRUCTURE.md
* Plans: PHASE3\_TEST\_PLAN.md, PHASE4\_TEST\_PLAN.md

**Quality Gates**:

* Reference: SPEC-KIT.md (Execution Model, Policies and Capture sections)

**Evidence**:

* Policy: POLICY.md#3-evidence-policy
* Baseline: TESTING\_INFRASTRUCTURE.md (fixtures)

**Upstream Sync**:

* Process: UPSTREAM-SYNC.md
* Isolation: FORK\_DEVIATIONS.md (80 FORK-SPECIFIC markers)

**Architecture**:

* Overview: ARCHITECTURE.md (consolidated)
* Async/Sync: ARCHITECTURE.md#5-asyncsync-boundaries
* Pipeline: ARCHITECTURE.md#8-pipeline-components
* Fork Management: CONTRIBUTING.md#7-fork-deviation-tracking

### By Audience

**New Contributors**:

1. README.md
2. CLAUDE.md
3. product-requirements.md
4. PLANNING.md

**Developers**:

1. SPEC.md (current tasks)
2. ARCHITECTURE.md
3. testing-policy.md
4. CLAUDE.md (operating guide)

**AI Agents**:

1. CLAUDE.md (mandatory)
2. AGENTS.md (orchestration)
3. SPEC.md (task context)
4. Relevant policy docs

***

## 📊 Documentation Statistics

**Total Project Docs**: \~250 .md files

**Active Documentation**:

* Essential (root): 18 files
* Spec-kit: 20-25 files
* Architecture: 3 files
* Specs (SPEC-KIT-\*): \~150 directories

**Archived**: 28+ files

* Session notes: 16 files
* Design docs: 12 files

**Reduction**: 30-40% fewer active docs to maintain

***

## 🔄 Maintenance

**Update Frequency**:

* **Daily**: SPEC.md (task updates)
* **Per session**: CLAUDE.md (if prerequisites change)
* **Per release**: CHANGELOG.md
* **Quarterly**: UPSTREAM-SYNC.md (after sync)
* **As needed**: Policy docs (testing-policy, evidence-policy)

**Stale Document Policy**:

* Session summaries → Archive after 30 days
* Design docs → Archive when implemented
* Completed specs → Archive when unlocked
* Analysis docs → Archive after decisions made

***

## 🎓 Quick Reference

**Common Tasks**:

* Run spec-kit command → See CLAUDE.md section 2
* Write tests → See POLICY.md#4-testing-policy, TESTING\_INFRASTRUCTURE.md
* Handle quality gates → See POLICY.md#2-gate-policy
* Sync upstream → See UPSTREAM-SYNC.md
* Find evidence → See POLICY.md#3-evidence-policy

**Common Questions**:

* "How do I...?" → Check CLAUDE.md first
* "What's the status of...?" → Check SPEC.md
* "Why did we...?" → Check relevant policy or design doc
* "Where is...?" → Check this INDEX.md

***

**Navigation**: Return to [README.md](../README.md) | [SPEC.md](../SPEC.md) | [CLAUDE.md](../CLAUDE.md)
