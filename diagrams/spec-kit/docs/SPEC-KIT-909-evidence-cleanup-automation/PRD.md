# PRD: SPEC-KIT-909 - Evidence Lifecycle Management

**Priority**: P1 (High Priority)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

The spec-kit evidence repository can grow indefinitely with no automated cleanup or lifecycle management:

**Current Issues**:
1. **Unbounded Growth**: Evidence files accumulate without deletion (soft limit 25MB per SPEC, no enforcement)
2. **Manual Monitoring**: `/spec-evidence-stats` requires manual invocation to check size
3. **No Lifecycle Policy**: No automated archival or deletion strategy
4. **Git Bloat Risk**: If evidence committed, slows Git operations over time
5. **Disk Pressure**: Large evidence directories impact CI/CD performance

**Current Behavior**:
- Telemetry JSON written on every stage (~1-5MB per run)
- No automatic cleanup or archival
- Soft limit (25MB per SPEC) documented but not enforced
- Manual cleanup only, no scheduled automation

Without lifecycle management, evidence storage will become operational burden requiring frequent manual intervention.

---

## Goals

### Primary Goal
Implement automated evidence lifecycle management with archival and cleanup policies, enforcing hard limits to prevent unbounded growth.

### Secondary Goals
- Auto-archive evidence older than 30 days to separate tier
- Auto-delete archived evidence older than 90 days
- Enforce 50MB hard limit per SPEC (halt if exceeded)
- Provide `/speckit.evidence-cleanup` command for manual triggering
- Generate cleanup summary reports for audit trail

---

## Requirements

### Functional Requirements

1. **Lifecycle Policy Configuration**
   - Define retention periods: Active (30 days), Archive (90 days), Delete (after 90d in archive)
   - Define size limits: Soft (25MB warning), Hard (50MB block)
   - Configurable via `Config.evidence_lifecycle` section

2. **Automatic Archival**
   - Move evidence files >30 days old to `evidence/archive/<SPEC-ID>/`
   - Preserve directory structure
   - Compress archived files (optional: gzip for space savings)
   - Log archival actions to `evidence/lifecycle.log`

3. **Automatic Deletion**
   - Delete archived files >90 days old
   - Never delete active evidence (<30 days)
   - Log deletions to `evidence/lifecycle.log`
   - Optional: Require confirmation flag for deletions

4. **Size Enforcement**
   - Check total evidence size for SPEC before writing new telemetry
   - If >50MB (hard limit): Block write, return error with cleanup suggestion
   - If >25MB (soft limit): Log warning, continue write
   - Report size in cleanup summary

5. **Manual Cleanup Command**
   - Add `/speckit.evidence-cleanup [--spec SPEC-ID] [--dry-run]` command
   - If `--spec` provided: Clean single SPEC only
   - If omitted: Clean all SPECs project-wide
   - `--dry-run`: Show what would be cleaned without executing
   - Display cleanup summary: files archived, deleted, space freed

6. **Cleanup Summary Report**
   - Output: Files processed, space freed, retention applied
   - Example:
     ```
     Evidence Cleanup Summary
     ========================
     SPEC-KIT-065:
       - Archived: 12 files (8.3 MB) [>30 days]
       - Deleted: 3 files (2.1 MB) [>90 days in archive]
       - Remaining: 156 files (18.7 MB)

     SPEC-KIT-070:
       - Archived: 8 files (5.2 MB)
       - Deleted: 1 file (0.8 MB)
       - Remaining: 89 files (12.4 MB)

     Total space freed: 16.4 MB
     ```

### Non-Functional Requirements

1. **Performance**
   - Cleanup operation completes in <30 seconds for 100 SPECs
   - Size check overhead <50ms per telemetry write

2. **Safety**
   - Never delete active evidence (<30 days)
   - Archive before delete (two-tier safety)
   - Atomic operations (move + verify before delete original)
   - Rollback on failure (restore from archive)

3. **Auditability**
   - All lifecycle actions logged to `evidence/lifecycle.log`
   - Log format: timestamp, action, spec_id, file_path, size
   - Parseable for compliance audits

---

## Technical Approach

### Lifecycle Module Structure

```rust
// spec_kit/evidence_lifecycle.rs
pub struct EvidenceLifecycle {
    retention_days_active: u32,      // 30 default
    retention_days_archive: u32,     // 90 default
    soft_limit_bytes: usize,         // 25MB
    hard_limit_bytes: usize,         // 50MB
    log_path: PathBuf,               // evidence/lifecycle.log
}

impl EvidenceLifecycle {
    pub fn cleanup_spec(&self, spec_id: &str, dry_run: bool) -> Result<CleanupSummary> {
        let evidence_dir = format!("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/{}", spec_id);
        let archive_dir = format!("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/{}", spec_id);

        let mut summary = CleanupSummary::default();

        // 1. Find files older than retention_days_active
        let active_files = self.find_old_active_files(&evidence_dir)?;
        if !active_files.is_empty() {
            summary.archived_count = active_files.len();
            summary.archived_bytes = self.archive_files(&active_files, &archive_dir, dry_run)?;
        }

        // 2. Find archived files older than retention_days_archive
        let old_archive_files = self.find_old_archived_files(&archive_dir)?;
        if !old_archive_files.is_empty() {
            summary.deleted_count = old_archive_files.len();
            summary.deleted_bytes = self.delete_files(&old_archive_files, dry_run)?;
        }

        // 3. Calculate remaining size
        summary.remaining_bytes = self.calculate_directory_size(&evidence_dir)?;
        summary.remaining_count = self.count_files(&evidence_dir)?;

        Ok(summary)
    }

    pub fn check_before_write(&self, spec_id: &str) -> Result<()> {
        let size = self.calculate_spec_size(spec_id)?;

        if size > self.hard_limit_bytes {
            return Err(SpecKitError::EvidenceOversize {
                spec_id: spec_id.to_string(),
                size,
                limit: self.hard_limit_bytes,
                message: format!(
                    "Evidence size {}MB exceeds hard limit {}MB. Run `/speckit.evidence-cleanup --spec {}` to free space.",
                    size / 1_000_000,
                    self.hard_limit_bytes / 1_000_000,
                    spec_id
                ),
            });
        } else if size > self.soft_limit_bytes {
            warn!(
                "Evidence size {}MB approaching limit for {} (soft: {}MB, hard: {}MB)",
                size / 1_000_000,
                spec_id,
                self.soft_limit_bytes / 1_000_000,
                self.hard_limit_bytes / 1_000_000
            );
        }

        Ok(())
    }

    fn find_old_active_files(&self, evidence_dir: &str) -> Result<Vec<PathBuf>> {
        let cutoff = Utc::now() - Duration::days(self.retention_days_active as i64);

        let mut old_files = Vec::new();
        for entry in fs::read_dir(evidence_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let metadata = fs::metadata(&path)?;
                let modified = metadata.modified()?.into();

                if modified < cutoff {
                    old_files.push(path);
                }
            }
        }

        Ok(old_files)
    }

    fn archive_files(&self, files: &[PathBuf], archive_dir: &str, dry_run: bool) -> Result<usize> {
        fs::create_dir_all(archive_dir)?;
        let mut total_bytes = 0;

        for file in files {
            let size = fs::metadata(file)?.len() as usize;
            total_bytes += size;

            if !dry_run {
                let dest = Path::new(archive_dir).join(file.file_name().unwrap());
                fs::rename(file, &dest)?;
                self.log_action("archive", file, size)?;
            }
        }

        Ok(total_bytes)
    }

    fn delete_files(&self, files: &[PathBuf], dry_run: bool) -> Result<usize> {
        let mut total_bytes = 0;

        for file in files {
            let size = fs::metadata(file)?.len() as usize;
            total_bytes += size;

            if !dry_run {
                fs::remove_file(file)?;
                self.log_action("delete", file, size)?;
            }
        }

        Ok(total_bytes)
    }

    fn log_action(&self, action: &str, file: &Path, size: usize) -> Result<()> {
        let log_entry = format!(
            "{} | {} | {} | {}\n",
            Utc::now().to_rfc3339(),
            action,
            file.display(),
            size
        );

        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        log_file.write_all(log_entry.as_bytes())?;
        Ok(())
    }
}

pub struct CleanupSummary {
    pub archived_count: usize,
    pub archived_bytes: usize,
    pub deleted_count: usize,
    pub deleted_bytes: usize,
    pub remaining_count: usize,
    pub remaining_bytes: usize,
}
```

### Integration with Evidence Writer

```rust
// evidence.rs (updated)
impl EvidenceRepository {
    pub fn write_telemetry_bundle(&self, spec_id: &str, telemetry: &TelemetryBundle) -> Result<()> {
        // Check size before write (enforces hard limit)
        let lifecycle = EvidenceLifecycle::default();
        lifecycle.check_before_write(spec_id)?;

        // Existing write logic
        self.write_json(spec_id, telemetry)?;

        Ok(())
    }
}
```

### Manual Cleanup Command

```rust
// commands/evidence_cleanup.rs
pub struct SpecKitEvidenceCleanupCommand;

impl SpecKitCommand for SpecKitEvidenceCleanupCommand {
    fn execute(&self, widget: &mut ChatWidget, args: CleanupArgs) {
        let lifecycle = EvidenceLifecycle::default();

        let specs = if let Some(spec_id) = args.spec {
            vec![spec_id]
        } else {
            self.find_all_specs(&widget.config.cwd)?
        };

        widget.append_markdown_cell(format!("🧹 **Evidence Cleanup**\n\nMode: {}\n",
            if args.dry_run { "Dry Run" } else { "Execute" }
        ));

        let mut total_archived = 0;
        let mut total_deleted = 0;
        let mut total_freed = 0;

        for spec_id in specs {
            let summary = lifecycle.cleanup_spec(&spec_id, args.dry_run)?;

            widget.append_markdown_cell(format!(
                "**{}**:\n- Archived: {} files ({:.1} MB)\n- Deleted: {} files ({:.1} MB)\n- Remaining: {} files ({:.1} MB)\n",
                spec_id,
                summary.archived_count,
                summary.archived_bytes as f64 / 1_000_000.0,
                summary.deleted_count,
                summary.deleted_bytes as f64 / 1_000_000.0,
                summary.remaining_count,
                summary.remaining_bytes as f64 / 1_000_000.0,
            ));

            total_archived += summary.archived_count;
            total_deleted += summary.deleted_count;
            total_freed += summary.archived_bytes + summary.deleted_bytes;
        }

        widget.append_markdown_cell(format!(
            "✅ **Total**: Archived {} files, deleted {} files, freed {:.1} MB",
            total_archived,
            total_deleted,
            total_freed as f64 / 1_000_000.0
        ));
    }
}
```

---

## Acceptance Criteria

- [ ] `EvidenceLifecycle` module created in `spec_kit/evidence_lifecycle.rs`
- [ ] Lifecycle policy configurable (retention days, size limits)
- [ ] Automatic archival logic implemented (move files >30 days to archive/)
- [ ] Automatic deletion logic implemented (delete archived files >90 days)
- [ ] Size enforcement implemented (50MB hard limit blocks writes)
- [ ] `/speckit.evidence-cleanup` command added
- [ ] `--spec` flag for single-SPEC cleanup
- [ ] `--dry-run` flag for preview without execution
- [ ] Cleanup summary report generated and displayed
- [ ] Lifecycle log file created (`evidence/lifecycle.log`)
- [ ] Integration with evidence writer (check before write)
- [ ] Unit tests for lifecycle logic (archival, deletion, size checks)
- [ ] Integration tests for cleanup command
- [ ] Documentation updated (`CLAUDE.md`, evidence policy doc)

---

## Out of Scope

- **Automated scheduling**: This SPEC implements logic, not cron/scheduled execution
- **Compression**: Evidence files not compressed (can be added later)
- **Remote storage**: Evidence remains local, no cloud upload/archival
- **Retention customization**: Uses fixed policy (30/90 days), not per-SPEC custom

---

## Success Metrics

1. **Automated Cleanup**: Evidence size stabilizes below 50MB per SPEC
2. **Manual Intervention**: Cleanup needed <1x per month (vs current ad-hoc)
3. **Disk Pressure**: Evidence directory total size <2GB project-wide
4. **Performance**: Cleanup completes in <30 seconds for 100 SPECs

---

## Dependencies

### Prerequisites
- None (standalone evidence system enhancement)

### Downstream Dependencies
- Evidence footprint monitoring (`/spec-evidence-stats`) enhanced by lifecycle management
- CI/CD performance improves (smaller evidence directories)

---

## Estimated Effort

**4-6 hours** (as per architecture review)

**Breakdown**:
- Lifecycle module implementation: 2 hours
- Integration with evidence writer: 1 hour
- Manual cleanup command: 1 hour
- Unit + integration tests: 1.5 hours
- Documentation: 30 min

---

## Priority

**P1 (High Priority)** - Critical for operational sustainability, fits within 30-day action window. Prevents future disk pressure and manual cleanup burden.

---

## Related Documents

- Architecture Review: Section "30-Day Actions, Task 1" (ARCH-013)
- `spec_kit/evidence.rs` - Evidence repository implementation
- `scripts/spec_ops_004/evidence_stats.sh` - Current manual monitoring
- Evidence policy doc (to be updated with lifecycle details)
