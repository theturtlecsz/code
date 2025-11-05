# PRD: SPEC-KIT-906 - Legacy Config Migration Warning

**Priority**: P1 (Medium Priority)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

The codex-rs config system supports two locations with ambiguous behavior:

1. **Current location**: `~/.code/config.toml` (preferred)
2. **Legacy location**: `~/.codex/config.toml` (fallback, read-only)

This duplication creates user confusion:

- Users may have both configs → unclear which takes precedence
- Legacy location still works but discouraged → no migration guidance
- No deprecation timeline → indefinite maintenance burden
- No warning when using legacy location → users unaware of better practice

The current code prefers `~/.code/` and falls back to `~/.codex/` read-only, but there's no active migration or deprecation strategy.

---

## Goals

### Primary Goal
Add automatic detection and migration from legacy `~/.codex/` to new `~/.code/` location, with clear warnings and timeline for legacy support removal.

### Secondary Goals
- Reduce user confusion by consolidating to single config location
- Establish deprecation timeline (6 months warning → removal)
- Auto-migrate configs with user confirmation
- Clear path to removing legacy support code

---

## Requirements

### Functional Requirements

1. **Legacy Detection**
   - On startup, check if `~/.codex/config.toml` exists
   - Check if `~/.code/config.toml` exists
   - Determine migration necessity

2. **User Warning**
   - Display clear warning message if legacy config detected
   - Explain: legacy location deprecated, new location preferred
   - State deprecation timeline: "Legacy support ends in 6 months (April 2026)"

3. **Auto-Migration with Confirmation**
   - Prompt user: "Migrate config to ~/.code/ now? [Y/n]"
   - If yes: Copy entire `~/.codex/` directory to `~/.code/`
   - Preserve legacy directory (don't delete, mark as backup)
   - If no: Continue with fallback, show warning on every startup

4. **Migration Actions**
   - Copy `~/.codex/config.toml` → `~/.code/config.toml`
   - Copy entire `~/.codex/` directory structure (sessions, logs, etc.)
   - Create `.migrated` marker file in `~/.codex/`
   - Display success message with new location

5. **Fallback Behavior** (unchanged)
   - If `~/.code/` doesn't exist, continue using `~/.codex/` (with warning)
   - Maintain read-only fallback behavior

### Non-Functional Requirements

1. **User Experience**
   - Non-intrusive: Single prompt per migration opportunity
   - Clear messaging: No technical jargon
   - Safe: Original config preserved (copy, not move)

2. **Backward Compatibility**
   - Migration optional: Users can decline and continue using legacy
   - No forced breakage: Legacy location works for 6 months

3. **Data Safety**
   - Never delete legacy directory automatically
   - Verify copy success before marking migration complete

---

## Technical Approach

### Detection Logic

```rust
// core/src/config.rs
pub fn load() -> Result<Config> {
    let new_path = home_dir()?.join(".code/config.toml");
    let legacy_path = home_dir()?.join(".codex/config.toml");

    // Check migration status
    if legacy_path.exists() && !new_path.exists() {
        handle_legacy_config_migration(&legacy_path, &new_path)?;
    }

    // Existing load logic (with fallback)
    load_with_fallback(&new_path, &legacy_path)
}
```

### Migration Flow

```rust
fn handle_legacy_config_migration(
    legacy_path: &Path,
    new_path: &Path,
) -> Result<()> {
    // Check if already migrated
    let migrated_marker = legacy_path.parent().unwrap().join(".migrated");
    if migrated_marker.exists() {
        return Ok(()); // Already migrated, no warning needed
    }

    // Display warning
    eprintln!("⚠️  Config found at legacy location: {}", legacy_path.display());
    eprintln!("   New location: {}", new_path.display());
    eprintln!();
    eprintln!("   Legacy location will be deprecated in 6 months (April 2026).");
    eprintln!("   Please migrate to new location for continued support.");
    eprintln!();

    // Prompt for migration
    eprint!("   Migrate config to new location now? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() || input == "y" || input == "yes" {
        migrate_config_directory(legacy_path.parent().unwrap(), new_path.parent().unwrap())?;
        eprintln!("✅ Config migrated to {}", new_path.display());
        eprintln!("   Legacy directory preserved as backup.");
    } else {
        eprintln!("⚠️  Migration skipped. Warning will appear on every startup.");
    }

    Ok(())
}
```

### Migration Implementation

```rust
fn migrate_config_directory(
    legacy_dir: &Path,
    new_dir: &Path,
) -> Result<()> {
    // Create new directory
    fs::create_dir_all(new_dir)?;

    // Copy entire directory structure
    copy_dir_all(legacy_dir, new_dir)?;

    // Create migration marker in legacy directory
    let marker = legacy_dir.join(".migrated");
    fs::write(&marker, format!(
        "Migrated to {} on {}\n",
        new_dir.display(),
        Utc::now().to_rfc3339()
    ))?;

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_all(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }
    Ok(())
}
```

### Warning Message Format

```
⚠️  Config found at legacy location: /home/user/.codex/config.toml
   New location: /home/user/.code/config.toml

   Legacy location will be deprecated in 6 months (April 2026).
   Please migrate to new location for continued support.

   Migrate config to new location now? [Y/n]
```

### Success Message Format

```
✅ Config migrated to /home/user/.code/config.toml
   Legacy directory preserved as backup at /home/user/.codex/

   Future startups will use new location automatically.
```

---

## Acceptance Criteria

- [ ] Legacy detection logic implemented in `config.rs::load()`
- [ ] Warning message displays on legacy config detection
- [ ] Migration prompt asks for user confirmation
- [ ] Auto-migration copies entire `~/.codex/` directory to `~/.code/`
- [ ] Legacy directory preserved (not deleted)
- [ ] `.migrated` marker file created after successful migration
- [ ] Migration skipped if user declines (fallback continues)
- [ ] Warning appears on every startup if migration declined
- [ ] Deprecation timeline documented (6 months)
- [ ] Unit tests verify migration logic
- [ ] Integration tests verify warning appears correctly
- [ ] Documentation updated (`CLAUDE.md`, README, config guide)
- [ ] Release notes include migration notice

---

## Out of Scope

- **Forced migration**: Migration always optional, never automatic without consent
- **Legacy deletion**: Never delete `~/.codex/` automatically
- **Backward sync**: No automatic sync of changes back to legacy location
- **Immediate removal**: Legacy support remains for 6 months minimum

---

## Success Metrics

1. **Adoption**: 80% of users migrate within 3 months
2. **Clarity**: <5% of users confused by migration prompt
3. **Safety**: Zero data loss incidents during migration
4. **Timeline**: Legacy support removed after 6-month grace period (April 2026)

---

## Dependencies

### Prerequisites
- None (standalone config system enhancement)

### Downstream Dependencies
- Future config enhancements can assume single location after deprecation period

---

## Estimated Effort

**2-3 hours** (as per architecture review)

**Breakdown**:
- Detection logic: 30 min
- Migration implementation: 1 hour
- Warning/prompt UI: 30 min
- Unit tests: 30 min
- Documentation: 30 min

---

## Priority

**P1 (Medium Priority)** - User experience improvement, fits within 30-day action window. Low risk, clear benefit for reducing config confusion.

---

## Related Documents

- Architecture Review: Section "30-Day Actions, Task 3"
- `codex-rs/core/src/config.rs` - Config loading logic
- `codex-rs/core/src/config_types.rs` - Config structure
- Configuration documentation (user-facing guides)
