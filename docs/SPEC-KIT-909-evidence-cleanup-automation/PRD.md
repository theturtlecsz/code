# PRD: Evidence Lifecycle Management (50MB Enforcement)

**SPEC-ID**: SPEC-KIT-909
**Priority**: P1 (blocks SPEC-KIT-910, SPEC-KIT-902)
**Created**: 2025-11-01
**Effort**: 4-6 hours

---

## 1. Problem Statement

The evidence repository grows unbounded without lifecycle management, creating:
- Disk bloat (evidence folders can exceed 100MB per SPEC)
- Slow repository operations (large binary files in git)
- Unclear retention policy (consensus artifacts persist indefinitely)
- No enforcement of the 25MB soft limit documented in evidence-policy.md

**Current State**:
- Evidence stored in `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`
- Consensus artifacts: `consensus/SPEC-ID/`
- Command telemetry: `commands/SPEC-ID/`
- No automated cleanup or archival
- `/spec-evidence-stats` shows current size but doesn't enforce limits

**Why Now**: SPEC-KIT-910 (consensus DB migration) and SPEC-KIT-902 (guardrail nativization) both require clean evidence foundation. Without automated lifecycle, manual cleanup becomes operational burden.

---

## 2. Goals

1. **Auto-archive** consensus artifacts older than 30 days
2. **Enforce 50MB hard limit** per SPEC (was 25MB soft limit)
3. **Provide cleanup tooling** for evidence repository management
4. **Maintain audit trail** for compliance/debugging

---

## 3. Functional Requirements

**FR-1**: Auto-archive consensus artifacts >30 days old
- Compress to `.tar.gz` with SHA256 checksum
- Move to `evidence/archive/YYYY-MM/`
- Preserve original timestamps and metadata
- Dry-run mode for validation

**FR-2**: Enforce 50MB hard limit per SPEC
- `/spec-evidence-stats` shows warnings when SPEC >40MB, errors >50MB
- `/speckit.auto` checks evidence size before starting
- Auto-archive oldest artifacts if approaching limit

**FR-3**: Evidence cleanup utilities
- `evidence_archive.sh` - Compress and archive old consensus
- `evidence_cleanup.sh` - Purge archived files >180 days (with safety flag)
- Update `evidence_stats.sh` with policy compliance checks

**FR-4**: Audit trail preservation
- Archive manifest: `archive_manifest.json` with checksums, dates, original paths
- Restoration script: `evidence_restore.sh SPEC-ID` to decompress archived data
- Evidence deletion log: Record what was purged and when

---

## 4. Acceptance Criteria

**AC-1**: `/spec-evidence-stats` shows policy compliance
- Warnings for SPECs >40MB
- Errors for SPECs >50MB
- List of archive candidates (>30 days old)

**AC-2**: Auto-archive works
- Consensus artifacts >30 days compressed to `.tar.gz`
- Checksums validated after compression
- Original size → compressed size logged (expect ~75% reduction)

**AC-3**: Hard limit enforcement
- `/speckit.auto` aborts with error if SPEC >50MB
- Suggests running archive before continuing
- Evidence continues to accumulate for active SPECs

**AC-4**: Restoration verified
- Archived evidence can be decompressed successfully
- Checksums match
- Original directory structure preserved

---

## 5. Implementation Plan

**Script 1**: `scripts/spec_ops_004/evidence_archive.sh` (enhance existing)
- Add `--dry-run` flag
- Add `--retention-days N` (default 30)
- Compress consensus artifacts for each SPEC
- Generate checksums and manifest
- Estimated: 2 hours

**Script 2**: `scripts/spec_ops_004/evidence_cleanup.sh` (new)
- Purge archives >180 days with `--enable-purge` safety flag
- Log deletions to `evidence/cleanup.log`
- Estimated: 1 hour

**Script 3**: Update `scripts/spec_ops_004/evidence_stats.sh`
- Add "Policy Compliance" section
- Warn if SPEC >40MB, error if >50MB
- List archive candidates
- Estimated: 1 hour

**Integration**: Update `/speckit.auto` to check evidence size pre-flight
- Call `evidence_stats.sh` before starting
- Abort if >50MB with remediation steps
- Estimated: 30 minutes

**Testing**: Validate on existing evidence
- Dry-run archive on SPEC-KIT-025, 045, 060, 900
- Verify compression ratios
- Test restoration
- Estimated: 1-2 hours

---

## 6. Non-Goals

- Do NOT purge command telemetry (needed for debugging)
- Do NOT compress evidence for SPECs with status "In Progress"
- Do NOT auto-delete without explicit `--enable-purge` flag
- Do NOT touch evidence outside consensus/ directory

---

## 7. Success Metrics

- Evidence repository stays <500MB total
- All SPECs <50MB individually
- Archived artifacts compress to ~25% original size
- Zero evidence loss (checksums validate)
- Automated enforcement (no manual intervention)

---

## 8. Dependencies

**Blocks**:
- SPEC-KIT-910 (needs clean evidence foundation for DB migration)
- SPEC-KIT-902 (guardrail nativization needs predictable evidence size)

**Depends On**: None (can start immediately)

---

Back to [Key Docs](../KEY_DOCS.md)
