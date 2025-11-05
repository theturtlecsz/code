# PRD: SPEC-KIT-903 - Add Template Version Tracking

**Priority**: P1 (Medium Priority)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

Spec-kit templates (spec.md, PRD.md, tasks.md, etc.) are embedded in Rust code but lack version tracking. This creates maintenance challenges:

1. **Unknown Template Origin**: Existing SPECs don't indicate which template version generated them
2. **Evolution Blindness**: Template changes over time but no way to identify old vs new format
3. **Migration Path Unclear**: When templates evolve, how to identify SPECs needing updates?
4. **Debugging Difficulty**: Template-related issues hard to diagnose without knowing generation version
5. **Backward Compatibility Risk**: Template changes may break tools expecting old format

The `/speckit.new` command generates SPECs from templates, but there's no metadata indicating template version used. This makes template evolution risky and harder to maintain.

---

## Goals

### Primary Goal
Add version tracking to all spec-kit templates, embedding version metadata in generated artifacts to enable safe template evolution and clear migration paths.

### Secondary Goals
- Track template changes in CHANGELOG for audit trail
- Enable detection of SPECs generated from outdated templates
- Simplify debugging by knowing which template version generated each artifact
- Establish foundation for future template migration tooling

---

## Requirements

### Functional Requirements

1. **Template Version Constants**
   - Add `TEMPLATE_VERSION` constant to each template in `spec_prompts.rs`
   - Use semantic versioning: `v1.0`, `v1.1`, etc.
   - Separate versions per template type (SPEC, PRD, Tasks, Plan, etc.)

2. **Frontmatter Embedding**
   - Inject version metadata into YAML frontmatter of generated files
   - Include: `template_version`, `created`, `spec_id`
   - Format compatible with existing Markdown parsers

3. **Version Replacement in Templates**
   - Update template constants to include `{version}` placeholder
   - Replace at generation time with current version
   - Preserve all existing template content

4. **CHANGELOG Integration**
   - Document template changes in project CHANGELOG
   - Section: "Template Versions" with chronological entries
   - Include: version number, date, changes summary

5. **Detection Tooling** (future-ready)
   - Prepare for future `/speckit.template-audit` command
   - Would scan all SPECs and report template versions used
   - Foundation for migration guidance

### Non-Functional Requirements

1. **Backward Compatibility**
   - Existing SPECs without version metadata continue to work
   - Version detection treats missing version as "pre-versioning" (v0.9 or earlier)

2. **Minimal Overhead**
   - Version embedding adds <50 bytes per file
   - No performance impact on generation time

3. **Standards Compliance**
   - YAML frontmatter follows Jekyll/Hugo conventions
   - Markdown content unchanged (only frontmatter added)

---

## Technical Approach

### Template Version Constants

```rust
// spec_prompts.rs
const SPEC_TEMPLATE_VERSION: &str = "1.0";
const PRD_TEMPLATE_VERSION: &str = "1.0";
const TASKS_TEMPLATE_VERSION: &str = "1.0";
const PLAN_TEMPLATE_VERSION: &str = "1.0";

const SPEC_TEMPLATE: &str = r#"
---
template_version: {version}
created: {date}
spec_id: {spec_id}
---
# SPEC-{spec_id}: {title}

**Status**: {status}
**Priority**: {priority}
**Created**: {date}

## Problem Statement
{problem}

## Goals
{goals}

## Requirements
{requirements}

## Acceptance Criteria
{acceptance}
"#;
```

### Generation with Version Embedding

```rust
// spec_kit/commands/new.rs
pub fn generate_spec_md(spec_id: &str, title: &str) -> String {
    SPEC_TEMPLATE
        .replace("{version}", SPEC_TEMPLATE_VERSION)
        .replace("{date}", &Utc::now().to_rfc3339())
        .replace("{spec_id}", spec_id)
        .replace("{title}", title)
        .replace("{status}", "Draft")
        .replace("{priority}", "TBD")
        // ... other replacements
}
```

### Example Generated Output

```markdown
---
template_version: 1.0
created: 2025-10-30T14:23:45Z
spec_id: SPEC-KIT-905
---
# SPEC-KIT-905: Integration Test Freeze

**Status**: Draft
**Priority**: P1
**Created**: 2025-10-30T14:23:45Z

## Problem Statement
...
```

### CHANGELOG Format

```markdown
## Template Versions

### SPEC Template
- **v1.1** (2025-11-15): Added "Out of Scope" section, enhanced acceptance criteria format
- **v1.0** (2025-10-01): Initial GitHub-inspired template format

### PRD Template
- **v1.2** (2025-12-01): Added technical approach code examples section
- **v1.1** (2025-11-01): Added success metrics section
- **v1.0** (2025-10-01): Initial PRD template

### Tasks Template
- **v1.0** (2025-10-01): Initial task breakdown template

### Plan Template
- **v1.0** (2025-10-01): Initial plan template with work breakdown structure
```

### Version Detection (Future)

```rust
// spec_kit/template_audit.rs (future SPEC)
pub fn detect_template_version(spec_path: &Path) -> Option<String> {
    let content = fs::read_to_string(spec_path).ok()?;

    // Parse YAML frontmatter
    if content.starts_with("---\n") {
        let end = content[4..].find("---\n")?;
        let frontmatter = &content[4..4+end];

        // Extract template_version
        for line in frontmatter.lines() {
            if line.starts_with("template_version:") {
                return Some(line.split(':').nth(1)?.trim().to_string());
            }
        }
    }

    // No version = pre-versioning
    Some("0.9".to_string())
}

pub fn audit_all_specs(cwd: &Path) -> Vec<TemplateVersionReport> {
    // Scan docs/SPEC-*/spec.md, PRD.md, etc.
    // Report: spec_id, file_type, template_version, outdated flag
}
```

---

## Acceptance Criteria

- [ ] Template version constants defined for all templates (SPEC, PRD, Tasks, Plan)
- [ ] YAML frontmatter with `template_version` embedded in all generated files
- [ ] `spec_prompts.rs` updated with version placeholders
- [ ] Generation functions updated to replace `{version}` placeholder
- [ ] CHANGELOG.md section created for template version tracking
- [ ] Initial template versions documented (all start at v1.0)
- [ ] Existing SPECs unaffected (backward compatible)
- [ ] Documentation updated (`CLAUDE.md`, template guide)
- [ ] Unit tests verify version embedding works correctly
- [ ] Example generated files include proper frontmatter

---

## Out of Scope

- **Template migration tooling**: This SPEC only adds versioning, not migration automation
- **Retroactive versioning**: Existing SPECs not modified, only new ones get versions
- **Template content changes**: Focus is versioning system, not template improvements
- **Automated upgrade**: No automatic template upgrade system (manual for now)

---

## Success Metrics

1. **Adoption**: 100% of new SPECs include template version metadata
2. **Clarity**: Template version visible in first 10 lines of every spec.md
3. **Auditability**: CHANGELOG reflects all template changes chronologically
4. **Foundation**: Version detection logic ready for future migration tooling

---

## Dependencies

### Prerequisites
- None (standalone versioning system)

### Downstream Dependencies
- Future template migration tooling will rely on version metadata
- Template evolution tracking depends on this foundation

---

## Estimated Effort

**3-4 hours** (as per architecture review)

**Breakdown**:
- Template constant updates: 1 hour
- Generation function updates: 1 hour
- CHANGELOG documentation: 30 min
- Unit tests: 1 hour
- Documentation updates: 30 min

---

## Priority

**P1 (Medium Priority)** - Important for maintainability and template evolution, fits within 60-day action window. Low risk, high clarity benefit.

---

## Related Documents

- Architecture Review: Section "60-Day Actions, Task 6"
- `codex-rs/tui/src/spec_prompts.rs` - Template definitions
- `spec_kit/commands/new.rs` - SPEC generation entry point
- CHANGELOG.md - Will include template version history
