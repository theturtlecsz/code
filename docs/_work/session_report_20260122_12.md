# Session 12 Report: Archival Phase

**Date**: 2026-01-22
**Phase**: Archival
**Commit**: `ce534ecb1`

***

## Inventory Summary

| Category            | Before | After     | Change     |
| ------------------- | ------ | --------- | ---------- |
| docs/\*.md          | 48     | 11        | -37        |
| docs/spec-kit/\*.md | 35     | 0         | -35        |
| Canonical docs      | 11     | 9 (+nav)  | Merged 3→1 |
| Archive pack        | -      | 1 (116KB) | +1         |

***

## Canonical Document Set (Final)

| # | Document            | Lines | Purpose                          |
| - | ------------------- | ----- | -------------------------------- |
| 1 | POLICY.md           | 335   | Consolidated policy              |
| 2 | OPERATIONS.md       | 887   | Consolidated operations          |
| 3 | ARCHITECTURE.md     | 395   | System architecture              |
| 4 | CONTRIBUTING.md     | 387   | Development workflow             |
| 5 | STAGE0-REFERENCE.md | 1,235 | Stage 0 engine reference         |
| 6 | DECISIONS.md        | 250   | Locked decisions (D1-D134)       |
| 7 | PROGRAM.md          | 136   | Active specs + DAG               |
| 8 | SPEC-KIT.md         | 447   | CLI, architecture, quality gates |
| 9 | KEY\_DOCS.md        | 65    | Canonical doc map                |

**Navigation**: INDEX.md (247 lines), VISION.md (28 lines)

***

## Top Consolidation (This Session)

| Source Files              | Lines Before | Target      | Lines After | Reduction |
| ------------------------- | ------------ | ----------- | ----------- | --------- |
| SPEC-KIT-QUALITY-GATES.md | 828          | SPEC-KIT.md | 447         | -         |
| SPEC-KIT-CLI.md           | 532          | SPEC-KIT.md | (merged)    | -         |
| SPEC-KIT-ARCHITECTURE.md  | 419          | SPEC-KIT.md | (merged)    | -         |
| **Total**                 | 1,779        |             | 447         | 75%       |

***

## Archive Pack Created

**File**: `archive/docs-pack-20260122.tar.zst`
**Size**: 116 KB (compressed from 490 KB)
**Contents**: 72 markdown files

**Manifest**: `archive/docs-pack-20260122-manifest.json`

* Original path
* SHA256 hash
* Size in bytes
* Mapped destination or "archive-only" tag
* Date archived

**Categories archived**:

* 37 docs root files (legacy + redirect stubs)
* 35 docs/spec-kit files (reference docs)

***

## Mapping Decisions

| Old File                       | Destination  | Tag           |
| ------------------------------ | ------------ | ------------- |
| SPEC-KIT-QUALITY-GATES.md      | SPEC-KIT.md  | redirect-stub |
| SPEC-KIT-CLI.md                | SPEC-KIT.md  | redirect-stub |
| SPEC-KIT-ARCHITECTURE.md       | SPEC-KIT.md  | redirect-stub |
| DECISION\_REGISTER.md          | DECISIONS.md | legacy        |
| PROGRAM\_2026Q1\_ACTIVE.md     | PROGRAM.md   | redirect-stub |
| MODEL-POLICY.md                | POLICY.md    | legacy        |
| docs/spec-kit/\*.md (35 files) | archive-only | reference     |
| (32 other docs root files)     | archive-only | legacy        |

***

## Changes Applied

```
79 files changed, 1204 insertions(+), 14991 deletions(-)
```

**Key changes**:

1. Created `docs/SPEC-KIT.md` (447 lines) - merged canonical
2. Created `archive/docs-pack-20260122.tar.zst` (116KB)
3. Created `archive/docs-pack-20260122-manifest.json`
4. Updated `docs/INDEX.md` - references to SPEC-KIT.md
5. Updated `docs/KEY_DOCS.md` - single SPEC-KIT.md entry
6. Updated `codex-rs/scripts/doc_lint.py` - canonical enforcement
7. Deleted 72 non-canonical markdown files

***

## Verification Results

**doc\_lint.py output**:

* ✓ All canonical docs present (11 files)
* ✓ No doc sprawl detected
* ✓ Required files found (SPEC.md, PROGRAM.md, DECISIONS.md, POLICY.md, SPEC-KIT.md)

**Pre-existing warnings** (not addressed this session):

* model\_policy.toml missing
* SPEC.md sections missing
* 79 spec directories missing Decision IDs

***

## Outstanding Questions/Risks

1. **VISION.md**: Counted as canonical but small (28 lines). Consider merging into ARCHITECTURE.md?
2. **model\_policy.toml**: Required by doc\_lint but missing. Pre-existing issue.
3. **Archive recovery**: Archive pack is zstd-compressed. Ensure team has zstd installed.
4. **Redirect stubs removed**: Unlike previous sessions, redirect stubs were deleted (not kept). Archive manifest serves as mapping.

***

## Next Session Tasks (max 10)

1. ~~Create archive pack~~ ✓
2. Consider adding model\_policy.toml or removing from REQUIRED\_FILES
3. Add missing SPEC.md sections ("Doc Precedence Order", "Invariants")
4. Review VISION.md for potential merge
5. Update CLAUDE.md to reference SPEC-KIT.md for spec-kit workflows
6. Consider CI job to validate archive integrity
7. Document archive recovery procedure in OPERATIONS.md
8. Review and archive old docs/\_work/ session reports
9. Update README.md to reference new canonical structure
10. Consider INDEX.md + KEY\_DOCS.md merge (both serve navigation purpose)

***

**Phase Status**: Archival ✓ COMPLETE
**Next Phase**: Enforcement (doc\_lint CI integration, anti-sprawl hooks)
