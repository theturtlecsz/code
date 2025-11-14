# SPEC-933 Component 4 Handoff: Daily Cleanup Cron

**Priority**: P0-CRITICAL
**Effort**: 8-12 hours
**Progress**: 87.5% → 100% (final component)
**Status**: READY TO START

## Quick Start

```bash
# Context
- Component 3: ✅ COMPLETE (parallel spawning delivered)
- Components 1-2: ✅ COMPLETE (ACID + auto-vacuum via SPEC-945B)
- Component 4: ⏳ REMAINING (this document)

# Files to modify
- Create: tui/src/chatwidget/spec_kit/evidence_cleanup.rs
- Modify: tui/src/chatwidget/spec_kit/mod.rs (add module)
- Modify: tui/src/app.rs (integrate cleanup on startup)
- Create: tests (unit + integration)

# Session prompt
See: START-SPEC-933-COMPONENT-4.txt (create this for next session)
```

## Objective

Implement automated cleanup of old consensus artifacts to prevent unbounded evidence growth and enforce 50MB limit (SPEC-KIT-909).

**Target**:
- Archive artifacts >30 days old
- Purge artifacts >90 days old (optional: use 180d for safety)
- Exempt "In Progress" SPECs from cleanup
- Daily execution (<5 min runtime)
- 50MB limit monitoring with warnings

## Architecture Overview

```rust
// evidence_cleanup.rs module structure

pub struct CleanupConfig {
    pub archive_after_days: u64,    // Default: 30
    pub purge_after_days: u64,      // Default: 90 (or 180 for safety)
    pub enabled: bool,               // Default: true
    pub dry_run: bool,               // Default: false (for testing)
}

pub struct CleanupSummary {
    pub files_archived: usize,
    pub files_purged: usize,
    pub space_reclaimed_mb: f64,
    pub total_evidence_mb: f64,
    pub warnings: Vec<String>,
}

pub async fn run_daily_cleanup(
    config: &CleanupConfig
) -> Result<CleanupSummary, SpecKitError> {
    // 1. Find old artifacts
    // 2. Check if SPEC is "In Progress" (exempt)
    // 3. Archive >30d or purge >90d
    // 4. Calculate space reclaimed
    // 5. Check 50MB limit
    // 6. Return summary
}
```

## Implementation Steps

### Phase 1: Core Cleanup Logic (4-6h)

**Step 1.1: Create evidence_cleanup.rs module**
```rust
// tui/src/chatwidget/spec_kit/evidence_cleanup.rs

use std::path::{Path, PathBuf};
use std::time::{SystemTime, Duration};

pub async fn run_daily_cleanup(config: &CleanupConfig) -> Result<CleanupSummary, SpecKitError> {
    let mut summary = CleanupSummary::default();

    // Find evidence root (docs/SPEC-OPS-004-*/evidence/)
    let evidence_root = find_evidence_root()?;

    // Scan for old artifacts
    let old_artifacts = find_old_artifacts(&evidence_root, config.archive_after_days)?;
    let purge_candidates = find_old_artifacts(&evidence_root, config.purge_after_days)?;

    // Process archives (>30d)
    for artifact in old_artifacts {
        if !is_in_progress(&artifact.spec_id)? {
            archive_artifact(&artifact, &mut summary, config.dry_run)?;
        }
    }

    // Process purges (>90d or 180d)
    for artifact in purge_candidates {
        if !is_in_progress(&artifact.spec_id)? {
            purge_artifact(&artifact, &mut summary, config.dry_run)?;
        }
    }

    // Check evidence limits
    check_evidence_limits(&mut summary)?;

    Ok(summary)
}
```

**Step 1.2: Implement artifact discovery**
```rust
fn find_old_artifacts(evidence_root: &Path, age_days: u64) -> Result<Vec<ArtifactInfo>> {
    let cutoff = SystemTime::now() - Duration::from_secs(age_days * 86400);

    let mut artifacts = Vec::new();
    for entry in walkdir::WalkDir::new(evidence_root) {
        let entry = entry?;
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff {
                    artifacts.push(ArtifactInfo {
                        path: entry.path().to_path_buf(),
                        spec_id: extract_spec_id(&entry.path())?,
                        age_days: /* calculate */,
                        size_bytes: metadata.len(),
                    });
                }
            }
        }
    }

    Ok(artifacts)
}
```

**Step 1.3: Implement SPEC status checking**
```rust
fn is_in_progress(spec_id: &str) -> Result<bool> {
    // Option 1: Check SPEC.md file (if exists)
    // Option 2: Check for active .lock files
    // Option 3: Check SQLite for recent activity

    // Simplest: Check if SPEC directory has recent modifications
    let spec_dir = find_spec_directory(&std::env::current_dir()?, spec_id)?;
    let metadata = spec_dir.metadata()?;
    let modified = metadata.modified()?;
    let age = SystemTime::now().duration_since(modified)?;

    // If modified within last 7 days, consider "In Progress"
    Ok(age.as_secs() < 7 * 86400)
}
```

**Step 1.4: Implement archive/purge operations**
```rust
fn archive_artifact(
    artifact: &ArtifactInfo,
    summary: &mut CleanupSummary,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        tracing::info!("DRY RUN: Would archive {}", artifact.path.display());
        return Ok(());
    }

    // Create archive directory if needed
    let archive_dir = artifact.path.parent()
        .ok_or_else(|| SpecKitError::InvalidPath)?
        .join(".archived");

    std::fs::create_dir_all(&archive_dir)?;

    // Move to archive
    let archive_path = archive_dir.join(artifact.path.file_name().unwrap());
    std::fs::rename(&artifact.path, &archive_path)?;

    summary.files_archived += 1;
    summary.space_reclaimed_mb += artifact.size_bytes as f64 / 1_000_000.0;

    tracing::info!("Archived: {}", artifact.path.display());
    Ok(())
}

fn purge_artifact(
    artifact: &ArtifactInfo,
    summary: &mut CleanupSummary,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        tracing::info!("DRY RUN: Would purge {}", artifact.path.display());
        return Ok(());
    }

    std::fs::remove_file(&artifact.path)?;

    summary.files_purged += 1;
    summary.space_reclaimed_mb += artifact.size_bytes as f64 / 1_000_000.0;

    tracing::info!("Purged: {}", artifact.path.display());
    Ok(())
}
```

### Phase 2: Integration & Scheduling (2-3h)

**Step 2.1: Add module to mod.rs**
```rust
// tui/src/chatwidget/spec_kit/mod.rs
pub mod evidence_cleanup; // SPEC-933 Component 4: Evidence cleanup cron
```

**Step 2.2: Integrate with TUI startup**
```rust
// tui/src/app.rs (or appropriate initialization location)

use codex_tui::chatwidget::spec_kit::evidence_cleanup;

async fn run_startup_cleanup() {
    let config = evidence_cleanup::CleanupConfig {
        archive_after_days: 30,
        purge_after_days: 90,  // Or 180 for safety
        enabled: true,
        dry_run: false,
    };

    match evidence_cleanup::run_daily_cleanup(&config).await {
        Ok(summary) => {
            tracing::info!("Evidence cleanup complete: {} archived, {} purged, {:.2}MB reclaimed",
                summary.files_archived, summary.files_purged, summary.space_reclaimed_mb);

            // Warn if approaching 50MB limit
            if summary.total_evidence_mb > 45.0 {
                tracing::warn!("Evidence approaching 50MB limit: {:.2}MB", summary.total_evidence_mb);
            }
        }
        Err(e) => {
            tracing::error!("Evidence cleanup failed: {}", e);
        }
    }
}
```

**Step 2.3: Daily scheduling (optional)**
```rust
// Option 1: Run on TUI startup (simplest)
// → Already implemented above

// Option 2: Background tokio task (if TUI runs continuously)
tokio::spawn(async {
    let mut interval = tokio::time::interval(Duration::from_secs(86400)); // 24h
    loop {
        interval.tick().await;
        run_startup_cleanup().await;
    }
});

// Option 3: System cron (external)
// → Not recommended (requires user setup)
```

### Phase 3: Testing (2-3h)

**Step 3.1: Unit tests**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_find_old_artifacts() {
        // Create temp directory with old files
        // Verify correct artifacts identified
    }

    #[test]
    fn test_is_in_progress_exemption() {
        // Create temp SPEC directory
        // Touch file to make it "recent"
        // Verify exemption works
    }

    #[test]
    fn test_archive_creation() {
        // Archive artifact
        // Verify moved to .archived/
        // Verify original deleted
    }

    #[test]
    fn test_dry_run_mode() {
        // Run cleanup with dry_run=true
        // Verify no files modified
        // Verify summary shows would-be actions
    }
}
```

**Step 3.2: Integration tests**
```rust
#[tokio::test]
async fn test_full_cleanup_execution() {
    // Create test evidence structure
    // Add old + new + in-progress artifacts
    // Run cleanup
    // Verify:
    //   - Old artifacts archived
    //   - Very old purged
    //   - In-progress exempted
    //   - Summary accurate
}

#[tokio::test]
async fn test_50mb_limit_warning() {
    // Create evidence >45MB
    // Run cleanup
    // Verify warning logged
}
```

### Phase 4: Documentation & Validation (1-2h)

**Step 4.1: Update documentation**
- Add cleanup policy to `docs/spec-kit/evidence-policy.md`
- Document `/spec-evidence-stats` integration
- Update COMPONENT-3-COMPLETE.md with final 100% status

**Step 4.2: Final validation**
```bash
# Compile
cargo check -p codex-tui

# Run tests
cargo test -p codex-tui evidence_cleanup

# Full workspace test
cargo test --workspace --lib

# Dry run on real data
SPEC_KIT_CLEANUP_DRY_RUN=1 cargo run
```

## Acceptance Criteria

| # | Criterion | Validation | Evidence |
|---|-----------|------------|----------|
| AC6 | Daily execution | Scheduler logs | Startup logs |
| AC7 | Retention policy | Dry-run validation | Cleanup summary |
| AC8 | 50MB limit | Evidence stats | /spec-evidence-stats |
| AC9 | In Progress exempt | Test validation | test_in_progress_exemption |
| AC10 | Cleanup <5min | Performance measurement | Execution timer |

## Risks & Mitigations

**Risk 1: Accidental deletion of critical data**
- Mitigation: Archive before purge, dry-run testing, In Progress exemption
- Contingency: Archive recovery (.archived/ directory)

**Risk 2: Cleanup interferes with active work**
- Mitigation: Run at startup only (not during active use), In Progress check
- Contingency: Make cleanup optional via config

**Risk 3: Performance impact**
- Mitigation: Async execution, limit to startup, <5min target
- Contingency: Skip cleanup if TUI used immediately after startup

## Configuration

```toml
# Example: ~/.code/config.toml (if integrated with spec-kit config)

[evidence.cleanup]
enabled = true
archive_after_days = 30
purge_after_days = 90      # Or 180 for extra safety
dry_run = false             # Set true for testing
run_on_startup = true
```

## Testing Checklist

- [ ] Unit tests pass (4+ tests)
- [ ] Integration tests pass (2+ tests)
- [ ] Dry-run on real evidence directory succeeds
- [ ] In-progress SPEC exemption works
- [ ] Archive directory created correctly
- [ ] Cleanup summary accurate
- [ ] 50MB warning triggers correctly
- [ ] Full test suite passes (604+)
- [ ] No regressions in existing functionality

## Next Session Prompt

```
SPEC-933 Component 4: Daily Cleanup Cron Implementation

Context:
- Components 1-3: ✅ COMPLETE (ACID, auto-vacuum, parallel spawning)
- Component 4: Final deliverable (8-12h remaining)
- Progress: 87.5% → 100%

Goal:
Implement automated evidence cleanup (archive >30d, purge >90d, 50MB limit monitoring)

Tasks:
1. Create evidence_cleanup.rs module with CleanupConfig + CleanupSummary
2. Implement find_old_artifacts(), is_in_progress(), archive/purge operations
3. Integrate with TUI startup (app.rs)
4. Write 4 unit tests + 2 integration tests
5. Validate with dry-run on real evidence
6. Update documentation and mark SPEC-933 100% complete

Reference:
- PRD: docs/SPEC-KIT-933-database-integrity-hygiene/PRD.md
- Handoff: docs/SPEC-KIT-933-database-integrity-hygiene/COMPONENT-4-HANDOFF.md
- Component 3: docs/SPEC-KIT-933-database-integrity-hygiene/COMPONENT-3-COMPLETE.md

Effort: 8-12 hours
Priority: P0-CRITICAL
```

## Files to Create/Modify

**Create**:
1. `tui/src/chatwidget/spec_kit/evidence_cleanup.rs` (~400-500 lines)
2. Tests in evidence_cleanup.rs (~200 lines)

**Modify**:
1. `tui/src/chatwidget/spec_kit/mod.rs` (+1 line)
2. `tui/src/app.rs` or startup location (+20-30 lines)
3. Documentation updates

**Total Effort**: ~620-730 lines

## Success Metrics

- Evidence cleanup runs automatically on TUI startup
- Old artifacts archived after 30 days
- Archived artifacts purged after 90 days
- In-progress SPECs exempted from cleanup
- 50MB limit warning triggers at 45MB
- Cleanup execution <5 minutes
- Zero data loss (archive before purge)
- All tests passing

---

**Component 4 Status**: READY TO START
**Overall SPEC-933 Progress**: 87.5% → 100% (after completion)
**Estimated Time**: 8-12 hours
**Priority**: P0-CRITICAL
