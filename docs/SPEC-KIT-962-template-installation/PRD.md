# PRD: Template Installation & Distribution Architecture (SPEC-KIT-962)

**Version**: v20251129-install-a
**Status**: Draft
**Author**: Claude (P60)
**Created**: 2025-11-29
**Depends On**: SPEC-KIT-961 (Template Ecosystem Parity)

---

## 1. Executive Summary

### Problem Statement

The spec-kit template system is **broken by design**:

```

---

Back to [Key Docs](../KEY_DOCS.md)
┌─────────────────────────────────────────────────────────────┐
│ CURRENT STATE: Distribution Failure                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  prompts.json references:  ~/.code/templates/*.md           │
│  Repo contains:            ~/code/templates/*.md            │
│  Installation provides:    NOTHING                          │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ User Scenario          │ Templates │ Result         │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ cargo install          │ Missing   │ BROKEN         │   │
│  │ git clone + build      │ Wrong path│ BROKEN         │   │
│  │ Download release binary│ Missing   │ BROKEN         │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

No matter how the user installs the TUI, templates are either missing or at the wrong path.

### Solution

Implement **layered template resolution** with embedded defaults:

```
┌─────────────────────────────────────────────────────────────┐
│ TEMPLATE RESOLUTION ORDER                                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ 1. PROJECT-LOCAL     ./templates/{name}-template.md         │
│    (per-project)     Created by /speckit.project            │
│                      ↓ not found                            │
│                                                             │
│ 2. USER CONFIG       ~/.config/code/templates/              │
│    (XDG-compliant)   User customizations                    │
│                      ↓ not found                            │
│                                                             │
│ 3. EMBEDDED          include_str!() compiled in binary      │
│    (always works)    Zero external dependencies             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Key principle**: The binary works out of the box. No installation step required. Customization is opt-in.

### Success Criteria

- [ ] Binary works immediately after any installation method
- [ ] `cargo install` users get working templates
- [ ] Git clone developers get working templates
- [ ] Power users can customize via `~/.config/code/templates/`
- [ ] Per-project templates override global defaults
- [ ] No external file dependencies for core functionality

---

## 2. Background & Analysis

### Industry Patterns

| Tool | Strategy | Tradeoffs |
|------|----------|-----------|
| **rustfmt** | Embedded in binary | Always works, no customization |
| **prettier** | Embedded + config override | Works, customizable |
| **neovim** | First-run wizard | Complex, good UX |
| **brew formulas** | Package installs files | Distribution-specific |
| **LSP servers** | Runtime download | Network dependency |

### Why Embedded + Override

1. **Zero-config for 90% of users**: Just works
2. **Customization for power users**: Override with local files
3. **No network dependency**: Works offline
4. **No installer complexity**: No scripts, no wizards
5. **Update-friendly**: New embedded defaults with each release

### Current Code Locations

| Component | Path | Purpose |
|-----------|------|---------|
| Templates | `templates/*.md` | Source templates (11 files) |
| prompts.json | `docs/spec-kit/prompts.json` | Agent prompt definitions |
| new_native.rs | `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs` | /speckit.new implementation |
| project_native.rs | `codex-rs/tui/src/chatwidget/spec_kit/project_native.rs` | /speckit.project implementation |

---

## 3. Functional Requirements

### FR-1: Embed Templates in Binary

Create `codex-rs/tui/src/templates/mod.rs`:

```rust
//! Embedded template defaults.
//!
//! Templates are compiled into the binary using include_str!().
//! This ensures the TUI works without any external file dependencies.

pub mod embedded {
    pub const PLAN: &str = include_str!("../../../../templates/plan-template.md");
    pub const TASKS: &str = include_str!("../../../../templates/tasks-template.md");
    pub const IMPLEMENT: &str = include_str!("../../../../templates/implement-template.md");
    pub const VALIDATE: &str = include_str!("../../../../templates/validate-template.md");
    pub const AUDIT: &str = include_str!("../../../../templates/audit-template.md");
    pub const UNLOCK: &str = include_str!("../../../../templates/unlock-template.md");
    pub const CLARIFY: &str = include_str!("../../../../templates/clarify-template.md");
    pub const ANALYZE: &str = include_str!("../../../../templates/analyze-template.md");
    pub const CHECKLIST: &str = include_str!("../../../../templates/checklist-template.md");
    pub const PRD: &str = include_str!("../../../../templates/PRD-template.md");
    pub const SPEC: &str = include_str!("../../../../templates/spec-template.md");
}

/// Get embedded template by name.
///
/// Returns None if template name is not recognized.
pub fn get_embedded(name: &str) -> Option<&'static str> {
    match name {
        "plan" => Some(embedded::PLAN),
        "tasks" => Some(embedded::TASKS),
        "implement" => Some(embedded::IMPLEMENT),
        "validate" => Some(embedded::VALIDATE),
        "audit" => Some(embedded::AUDIT),
        "unlock" => Some(embedded::UNLOCK),
        "clarify" => Some(embedded::CLARIFY),
        "analyze" => Some(embedded::ANALYZE),
        "checklist" => Some(embedded::CHECKLIST),
        "PRD" | "prd" => Some(embedded::PRD),
        "spec" => Some(embedded::SPEC),
        _ => None,
    }
}

/// List all available template names.
pub fn template_names() -> &'static [&'static str] {
    &[
        "plan", "tasks", "implement", "validate", "audit", "unlock",
        "clarify", "analyze", "checklist", "PRD", "spec"
    ]
}
```

### FR-2: Template Resolution Function

Add resolution logic that checks locations in priority order:

```rust
use std::path::{Path, PathBuf};
use std::fs;

/// Resolution priority:
/// 1. Project-local: ./templates/{name}-template.md
/// 2. User config: ~/.config/code/templates/{name}-template.md
/// 3. Embedded fallback (always succeeds)
pub fn resolve_template(name: &str) -> String {
    // Normalize template name
    let normalized = name.to_lowercase();
    let filename = format!("{}-template.md", normalized);

    // 1. Project-local
    let local_path = PathBuf::from("templates").join(&filename);
    if local_path.exists() {
        if let Ok(content) = fs::read_to_string(&local_path) {
            tracing::debug!("Template '{}' resolved from project-local: {}", name, local_path.display());
            return content;
        }
    }

    // 2. User config (XDG Base Directory)
    if let Some(config_dir) = dirs::config_dir() {
        let user_path = config_dir.join("code/templates").join(&filename);
        if user_path.exists() {
            if let Ok(content) = fs::read_to_string(&user_path) {
                tracing::debug!("Template '{}' resolved from user config: {}", name, user_path.display());
                return content;
            }
        }
    }

    // 3. Embedded fallback
    tracing::debug!("Template '{}' resolved from embedded defaults", name);
    get_embedded(&normalized)
        .unwrap_or_else(|| {
            tracing::warn!("Unknown template '{}', returning empty", name);
            ""
        })
        .to_string()
}

/// Get the path where a template would be resolved from.
/// Useful for debugging and diagnostics.
pub fn resolve_template_source(name: &str) -> TemplateSource {
    let normalized = name.to_lowercase();
    let filename = format!("{}-template.md", normalized);

    let local_path = PathBuf::from("templates").join(&filename);
    if local_path.exists() {
        return TemplateSource::ProjectLocal(local_path);
    }

    if let Some(config_dir) = dirs::config_dir() {
        let user_path = config_dir.join("code/templates").join(&filename);
        if user_path.exists() {
            return TemplateSource::UserConfig(user_path);
        }
    }

    TemplateSource::Embedded
}

#[derive(Debug, Clone)]
pub enum TemplateSource {
    ProjectLocal(PathBuf),
    UserConfig(PathBuf),
    Embedded,
}
```

### FR-3: Update prompts.json Reference Syntax

Change hardcoded paths to symbolic references:

**Before**:
```json
"prompt": "Template: ~/.code/templates/plan-template.md\n\nTask:..."
```

**After**:
```json
"prompt": "Template: ${TEMPLATE:plan}\n\nTask:..."
```

The orchestrator expands `${TEMPLATE:plan}` using `resolve_template("plan")` before sending to the model.

### FR-4: Template Variable Expansion

Add to orchestrator prompt expansion:

```rust
fn expand_template_refs(prompt: &str) -> String {
    let re = Regex::new(r"\$\{TEMPLATE:(\w+)\}").unwrap();
    re.replace_all(prompt, |caps: &Captures| {
        let name = &caps[1];
        // Return the path where template was resolved from
        match resolve_template_source(name) {
            TemplateSource::ProjectLocal(p) => p.display().to_string(),
            TemplateSource::UserConfig(p) => p.display().to_string(),
            TemplateSource::Embedded => format!("[embedded:{}]", name),
        }
    }).to_string()
}
```

### FR-5: Install Templates Command

Add `/speckit.install-templates` command:

```rust
/// Copies embedded templates to user config directory for customization.
///
/// Usage: /speckit.install-templates [--force]
///
/// Creates: ~/.config/code/templates/
///
/// Options:
///   --force  Overwrite existing templates
pub fn install_templates(force: bool) -> Result<InstallResult> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Could not determine config directory"))?;

    let templates_dir = config_dir.join("code/templates");
    fs::create_dir_all(&templates_dir)?;

    let mut installed = Vec::new();
    let mut skipped = Vec::new();

    for name in template_names() {
        let filename = format!("{}-template.md", name);
        let dest = templates_dir.join(&filename);

        if dest.exists() && !force {
            skipped.push(filename);
            continue;
        }

        if let Some(content) = get_embedded(name) {
            fs::write(&dest, content)?;
            installed.push(filename);
        }
    }

    Ok(InstallResult { installed, skipped, path: templates_dir })
}
```

### FR-6: Template Status Command

Add `/speckit.template-status` diagnostic:

```rust
/// Shows template resolution status for all templates.
///
/// Usage: /speckit.template-status
///
/// Output:
///   plan      : ./templates/plan-template.md (project-local)
///   tasks     : ~/.config/code/templates/tasks-template.md (user)
///   implement : [embedded] (default)
///   ...
pub fn template_status() -> Vec<TemplateStatus> {
    template_names()
        .iter()
        .map(|name| {
            let source = resolve_template_source(name);
            TemplateStatus {
                name: name.to_string(),
                source,
            }
        })
        .collect()
}
```

---

## 4. Non-Functional Requirements

### NFR-1: Zero External Dependencies

The binary MUST work without any external files. Embedded templates ensure this.

### NFR-2: XDG Compliance

User config location MUST follow XDG Base Directory Specification:
- Linux: `~/.config/code/templates/`
- macOS: `~/Library/Application Support/code/templates/`
- Windows: `%APPDATA%\code\templates\`

Use the `dirs` crate for cross-platform paths.

### NFR-3: Binary Size Impact

Embedding 11 templates (~35KB total) has minimal impact:
- Current binary: ~15MB
- Added size: ~35KB (0.2% increase)
- Acceptable tradeoff for zero-config experience

### NFR-4: Backward Compatibility

- Existing prompts.json `~/.code/templates/` paths continue to work during transition
- Migration path: update prompts.json to `${TEMPLATE:name}` syntax
- Old paths fall back to embedded if file not found

---

## 5. Implementation Phases

### Phase 1: Embed Templates (2 hours)

1. Create `codex-rs/tui/src/templates/mod.rs`
2. Add `include_str!()` for all 11 templates
3. Add `get_embedded()` function
4. Add to `mod.rs` exports
5. Build and verify binary includes templates

**Verification**:
```bash
# Check binary size increase
ls -la target/release/codex-tui

# Verify templates accessible
cargo test -p codex-tui templates::
```

### Phase 2: Resolution Function (1 hour)

1. Add `resolve_template()` function
2. Add `resolve_template_source()` for diagnostics
3. Add `dirs` crate dependency
4. Add unit tests for resolution order

**Verification**:
```bash
cargo test -p codex-tui templates::resolve
```

### Phase 3: Update Orchestrator (2 hours)

1. Add `expand_template_refs()` to prompt expansion
2. Update agent spawning to expand `${TEMPLATE:name}`
3. Test with all spec-kit commands

**Verification**:
```bash
# Test in TUI
/speckit.plan SPEC-TEST-001
# Verify template resolved without errors
```

### Phase 4: Update prompts.json (1 hour)

1. Replace `~/.code/templates/*.md` with `${TEMPLATE:name}`
2. Update all 38 template references
3. Bump version strings
4. Validate JSON

**Verification**:
```bash
jq empty docs/spec-kit/prompts.json
grep -c '\${TEMPLATE:' docs/spec-kit/prompts.json  # Should be 38
```

### Phase 5: Add Commands (1 hour)

1. Implement `/speckit.install-templates`
2. Implement `/speckit.template-status`
3. Add help text and documentation

**Verification**:
```bash
# In TUI
/speckit.template-status
/speckit.install-templates
ls ~/.config/code/templates/
```

### Phase 6: Update /speckit.project (30 min)

1. Use `resolve_template()` instead of hardcoded content
2. Ensure project-local templates are created from embedded source

**Verification**:
```bash
/speckit.project rust test-project
ls test-project/templates/  # Should have all 11 templates
```

---

## 6. Testing Plan

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_embedded() {
        for name in template_names() {
            assert!(get_embedded(name).is_some(), "Missing embedded template: {}", name);
        }
    }

    #[test]
    fn test_template_content_not_empty() {
        for name in template_names() {
            let content = get_embedded(name).unwrap();
            assert!(!content.is_empty(), "Empty template: {}", name);
            assert!(content.contains('#'), "Template missing header: {}", name);
        }
    }

    #[test]
    fn test_resolve_falls_back_to_embedded() {
        // In clean environment, should always get embedded
        let content = resolve_template("plan");
        assert!(!content.is_empty());
        assert!(content.contains("Plan"));
    }

    #[test]
    fn test_unknown_template_returns_empty() {
        let content = resolve_template("nonexistent");
        assert!(content.is_empty());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_project_creates_all_templates() {
    let temp = tempdir().unwrap();
    std::env::set_current_dir(&temp).unwrap();

    create_project("rust", "test-proj").unwrap();

    for name in template_names() {
        let path = temp.path().join("test-proj/templates").join(format!("{}-template.md", name));
        assert!(path.exists(), "Missing template: {}", name);
    }
}

#[test]
fn test_local_template_overrides_embedded() {
    let temp = tempdir().unwrap();
    std::env::set_current_dir(&temp).unwrap();

    // Create custom local template
    fs::create_dir_all("templates").unwrap();
    fs::write("templates/plan-template.md", "# Custom Plan").unwrap();

    let content = resolve_template("plan");
    assert!(content.contains("Custom Plan"));
}
```

---

## 7. Migration Guide

### For Developers

1. Pull latest changes
2. Rebuild: `~/code/build-fast.sh`
3. Templates now embedded - no action needed

### For Users with Custom Templates

1. Run `/speckit.install-templates` to create `~/.config/code/templates/`
2. Copy your customizations from `~/.code/templates/` to `~/.config/code/templates/`
3. Remove old `~/.code/templates/` directory

### For CI/CD Pipelines

No changes needed - embedded templates work automatically.

---

## 8. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Binary size increase | Low | 35KB is negligible |
| Template changes require rebuild | Medium | Acceptable - templates rarely change |
| XDG paths differ across platforms | Low | Use `dirs` crate for cross-platform |
| Breaking existing `~/.code/` users | Medium | Support old paths during transition |

---

## 9. Success Metrics

- [ ] `cargo install codex-tui` works without additional setup
- [ ] `/speckit.plan` works on fresh install
- [ ] `/speckit.template-status` shows all templates as "embedded" on fresh install
- [ ] User config overrides work correctly
- [ ] Project-local templates take precedence
- [ ] All 38 prompts.json references updated

---

## 10. Future Considerations

### Template Versioning

Consider adding version metadata to templates:
```markdown
<!-- template-version: 20251129-a -->
# Plan: [FEATURE_NAME]
```

This would enable:
- Warning when local template is outdated
- Migration assistance for breaking changes

### Template Marketplace

Long-term: community-contributed templates
- `/speckit.template-search "security audit"`
- Download specialized templates for different domains

### Hot Reload

Development mode: watch `templates/` directory and reload without restart

---

## 11. References

- **SPEC-KIT-961**: Template ecosystem parity (dependency)
- **XDG Base Directory Spec**: https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
- **dirs crate**: https://docs.rs/dirs/latest/dirs/
- **include_str! macro**: https://doc.rust-lang.org/std/macro.include_str.html

---

## Appendix A: File Changes Summary

| File | Change |
|------|--------|
| `codex-rs/tui/src/templates/mod.rs` | NEW - Embedded templates module |
| `codex-rs/tui/src/chatwidget/spec_kit/mod.rs` | Add templates module import |
| `codex-rs/tui/src/chatwidget/spec_kit/project_native.rs` | Use resolve_template() |
| `codex-rs/tui/Cargo.toml` | Add dirs dependency |
| `docs/spec-kit/prompts.json` | Update 38 template refs |
| `docs/spec-kit/TEMPLATES.md` | Update resolution docs |

---

## Appendix B: prompts.json Migration Script

```bash
#!/bin/bash
# migrate-template-refs.sh
# Converts ~/.code/templates/*.md to ${TEMPLATE:name}

sed -i 's|~/.code/templates/plan-template.md|${TEMPLATE:plan}|g' docs/spec-kit/prompts.json
sed -i 's|~/.code/templates/tasks-template.md|${TEMPLATE:tasks}|g' docs/spec-kit/prompts.json
sed -i 's|~/.code/templates/implement-template.md|${TEMPLATE:implement}|g' docs/spec-kit/prompts.json
sed -i 's|~/.code/templates/validate-template.md|${TEMPLATE:validate}|g' docs/spec-kit/prompts.json
sed -i 's|~/.code/templates/audit-template.md|${TEMPLATE:audit}|g' docs/spec-kit/prompts.json
sed -i 's|~/.code/templates/unlock-template.md|${TEMPLATE:unlock}|g' docs/spec-kit/prompts.json
sed -i 's|~/.code/templates/clarify-template.md|${TEMPLATE:clarify}|g' docs/spec-kit/prompts.json
sed -i 's|~/.code/templates/analyze-template.md|${TEMPLATE:analyze}|g' docs/spec-kit/prompts.json
sed -i 's|~/.code/templates/checklist-template.md|${TEMPLATE:checklist}|g' docs/spec-kit/prompts.json

echo "Migration complete. Verify with: jq empty docs/spec-kit/prompts.json"
```
