# SPEC-KIT-101: Git Branch Enforcement & Auto-Detection

**Status**: Deferred (documented for future implementation)
**Created**: 2025-11-30
**Depends On**: SPEC-KIT-971 (clarify command)

---

## Problem Statement

Currently, `/speckit.clarify` requires explicit SPEC ID argument. To enable auto-detection from current branch, we need:

1. **Branch naming convention** enforced at SPEC creation
2. **Detection code** in Rust that reads current branch on startup/change
3. **Workflow changes** to stop direct commits to main for SPEC work

## Proposed Solution

### Branch Naming Convention

```
spec/SPEC-KIT-###    # Active SPEC work
main                  # Integration only
```

### Enforcement Points

| Event | Action |
|-------|--------|
| `/speckit.new` | Auto-create + checkout `spec/SPEC-KIT-###` |
| TUI startup | Detect branch → set active SPEC context |
| Branch change | Update active SPEC context |
| Commit to main | Warn if SPEC-related files modified |

### Required Rust Components

```rust
// Extend git_integration.rs or new git_utils.rs

/// Get current git branch name
pub fn get_current_branch(cwd: &Path) -> Option<String>;

/// Parse SPEC ID from branch name (e.g., "spec/SPEC-KIT-123" → "SPEC-KIT-123")
pub fn parse_spec_from_branch(branch: &str) -> Option<String>;

/// Create and checkout branch for new SPEC
pub fn create_spec_branch(spec_id: &str, cwd: &Path) -> Result<()>;

/// Check if on protected branch (main, master)
pub fn is_protected_branch(cwd: &Path) -> bool;
```

### App State Changes

```rust
// In App or ChatWidget state
pub struct SpecContext {
    /// Auto-detected from branch, or manually set
    pub active_spec: Option<String>,
    /// Source of active_spec
    pub detection_source: DetectionSource,
}

pub enum DetectionSource {
    Branch,      // Auto-detected from branch name
    Manual,      // User specified via command
    None,        // No active spec
}
```

## Integration with Existing Commands

### `/speckit.new` Enhancement

```
BEFORE: Creates SPEC in docs/, stays on main
AFTER:  Creates SPEC in docs/, creates + checks out spec/SPEC-KIT-### branch
```

### `/speckit.clarify` Enhancement

```
BEFORE: /speckit.clarify SPEC-KIT-123  (required arg)
AFTER:  /speckit.clarify               (uses active spec from branch)
        /speckit.clarify SPEC-KIT-123  (explicit override)
```

### Status Display

Show active SPEC in TUI status bar or header when detected.

## Migration Considerations

- Existing SPECs without branches continue to work (manual ID required)
- Optional `/speckit.branch SPEC-ID` command to create branch for existing SPEC
- Gradual adoption - enforcement can be warning-only initially

## Open Questions

1. **Strictness**: Hard block commits to main, or warnings only?
2. **Polling**: How often to check branch (startup only, or periodic)?
3. **Multi-SPEC**: Allow `spec/SPEC-KIT-123+124` for related work?

## Implementation Estimate

| Phase | Task | Effort |
|-------|------|--------|
| 1 | Git utilities (get_current_branch, parse) | ~30 min |
| 2 | App state + startup detection | ~45 min |
| 3 | `/speckit.new` branch creation | ~30 min |
| 4 | Command auto-detection integration | ~30 min |
| 5 | Warning system for main commits | ~30 min |

**Total**: ~3 hours

---

*Deferred: Focus on completing SPEC-KIT-971 (clarify) first. Revisit after clarify is working.*
