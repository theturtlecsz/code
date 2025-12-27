# Session 34 Prompt - Source Management

**Last updated:** 2025-12-27
**Status:** S33 Complete - Source-based Tier2 E2E validated
**Primary SPEC:** SPEC-SOURCE-MGMT (New)

---

## Session 33 Accomplishments

| Item | Status | Commit |
|------|--------|--------|
| Source-based Tier2 E2E | ✅ VALIDATED | `fbd254241` |
| Session close fix | ✅ FIXED | `fbd254241` |
| Polling fix (CommitAnimation) | ✅ FIXED | `fbd254241` |
| DIVINE_TRUTH.md with citations | ✅ CONFIRMED | Evidence updated |
| SPEC-DOGFOOD-001 A2/A3/A4/A5 | ✅ ALL PASS | See SPEC.md |

---

## Problem: Source Proliferation

After multiple `/speckit.auto` runs, the NotebookLM notebook has accumulated redundant sources:

**Current state (13 sources, 6 redundant):**
```
1. Divine Truth Tier 2 SPEC Analysis Framework [KEEP - template]
2. Golden Path Dogfooding Validation: Stage0 System Integration [REDUNDANT]
3. Golden Path Dogfooding Validation: Stage0 Verification Spec [REDUNDANT]
4. NotebookLM Tier2 Architectural Decisions and Milestone Log [KEEP - static]
5. Protocol for Active Testing Specifications [KEEP - static]
6. Golden Path Dogfooding Validation: SPEC-DOGFOOD-001 [REDUNDANT]
7. S33 Smoke Test Protocols [DELETE - test artifact]
8. SPEC-DOGFOOD-001: Golden Path Dogfooding Validation Brief [KEEP - CURRENT_TASK_BRIEF]
9. TUI v2 Port Stub and Compatibility Tracking Document [KEEP - static]
10. The Codex TUI Dogfooding Protocol [KEEP - static]
11. The Essence of New Source Testing [KEEP - static]
12. The Golden Path Dogfooding Validation Specification [REDUNDANT]
13. Golden Path Dogfooding Validation: SPEC-DOGFOOD-001 Project Brief [REDUNDANT]
```

**Root cause:** The upsert uses fuzzy title matching, but NotebookLM renames sources based on content. Each run with slightly different content creates a new source.

---

## SPEC-SOURCE-MGMT: NotebookLM Source Lifecycle Management

### Problem Statement

The current source-based Tier2 architecture creates orphan sources over time because:
1. NotebookLM generates semantic titles from content (unpredictable)
2. Fuzzy matching often fails to find the "same" source
3. No tracking of which sources belong to which spec
4. No cleanup of stale sources
5. NotebookLM has a 50-source limit per notebook

### Proposed Architecture

**Option A: Local Source Registry (Recommended)**

```
~/.config/code/source-registry.db  (SQLite)

Tables:
- sources (id, spec_id, source_type, notebooklm_title, created_at, last_used)
- notebooks (id, notebook_id, name, source_count)

Flow:
1. Before upsert: lookup spec_id in registry
2. If found: delete existing source by stored title
3. Upsert new source
4. After upsert: update registry with new title
5. Periodic cleanup: delete sources not used in N days
```

**Option B: Naming Convention**

Use deterministic prefixes that survive NotebookLM renaming:
```
CURRENT_SPEC_{SPEC_ID}_{HASH_8}
CURRENT_BRIEF_{SPEC_ID}_{HASH_8}
```

**Option C: Source Pinning**

Keep only 2 dynamic sources (CURRENT_SPEC, CURRENT_BRIEF) and always overwrite them regardless of title changes.

### Acceptance Criteria

| ID | Criterion | Verification |
|----|-----------|--------------|
| A1 | No orphan sources after 10 runs | `list-sources` shows constant count |
| A2 | Cleanup command exists | `/stage0.cleanup-sources` or similar |
| A3 | Source count tracked | Registry shows per-spec history |
| A4 | Manual cleanup works | Can delete by spec_id |

### Implementation Tasks

1. **Phase 1: Registry Design**
   - Create SQLite schema for source tracking
   - Add registry lookup to Tier2HttpAdapter
   - Store NotebookLM-generated title after upsert

2. **Phase 2: Upsert Enhancement**
   - Delete old source before upserting new
   - Handle title mismatch gracefully
   - Update registry with new title

3. **Phase 3: Cleanup Command**
   - Add `/stage0.cleanup-sources` command
   - Delete sources older than N days
   - Report cleanup stats

---

## Session 34 Tasks

### Phase 1: Manual Cleanup (Immediate)

The delete-source CLI isn't working properly. Manual cleanup via NotebookLM UI:

1. Open https://notebooklm.google.com
2. Select `code-project-docs` notebook
3. Delete sources 2, 3, 6, 7, 12, 13 (the redundant ones)
4. Verify 7 sources remain

### Phase 2: Design Source Registry

Create `docs/SPEC-SOURCE-MGMT/spec.md` with:
- Problem statement
- Architecture options (A/B/C above)
- Chosen approach with rationale
- Implementation plan

### Phase 3: Implement Registry

If time permits, implement Option A (Local Source Registry).

---

## Key Files

| Location | Purpose |
|----------|---------|
| `~/code/codex-rs/tui/src/stage0_adapters.rs` | Tier2HttpAdapter - where to add registry |
| `~/.config/code/source-registry.db` | Proposed registry location |
| `~/notebooklm-client/src/service/handlers/sources.ts` | Upsert API |

---

## Commits (S32-S33)

| Session | Commit | Description |
|---------|--------|-------------|
| S32 | `3f4464b` | notebooklm-client: POST /api/sources/upsert |
| S32 | `04d042a47` | codex-rs: Tier2HttpAdapter upsert flow |
| S32 | `82b52ce20` | docs: S32 evidence and SPEC.md |
| S33 | `fbd254241` | fix: Session close + polling fixes |
| S33 | `a68a6b8cc` | docs: S33 milestone |
