# PRD: /speckit.project Command (SPEC-KIT-960)

**Version**: v20251129-project-b
**Status**: Complete ✅
**Author**: Claude (P58)
**Created**: 2025-11-29
**Completed**: 2025-11-29

## 1. Overview

### Problem Statement
Starting a new project with spec-kit workflow support requires manual creation of:
- CLAUDE.md with project instructions
- SPEC.md task tracker
- docs/ directory for SPEC artifacts
- memory/constitution.md project charter

This is repetitive, error-prone, and creates friction for adopting spec-kit workflows.

### Solution
Create `/speckit.project` command that scaffolds new projects with full spec-kit infrastructure in under 1 second.

### Success Criteria
- [x] `/speckit.project <type> <name>` creates complete project structure
- [x] All 4 template types work (rust, python, typescript, generic)
- [x] Created projects are immediately ready for `/speckit.new`
- [x] Execution time <1s (Tier 0 native)
- [x] Zero agent cost ($0)

## 2. Functional Requirements

### FR-1: Command Syntax
```
/speckit.project <type> <name>
/speckit.project                   # Interactive mode
```

**Arguments:**
- `type`: One of `rust`, `python`, `typescript`, `generic`
- `name`: Project directory name (lowercase, no spaces)

### FR-2: Core Structure (All Templates)
Every template creates this base structure:
```
<name>/
├── CLAUDE.md              # Project-specific Claude instructions
├── SPEC.md                # Task tracker table (empty template)
├── docs/                  # SPEC directories created here
│   └── .gitkeep
└── memory/
    └── constitution.md    # Project charter template
```

### FR-3: Type-Specific Additions

#### Rust Template (`rust`)
```
├── Cargo.toml             # Workspace manifest
├── src/
│   └── lib.rs             # Placeholder with module docs
├── tests/
│   └── .gitkeep
└── .gitignore             # Rust-specific ignores
```

#### Python Template (`python`)
```
├── pyproject.toml         # uv/poetry compatible
├── src/
│   └── <name>/
│       └── __init__.py    # Package init
├── tests/
│   └── __init__.py
└── .gitignore             # Python-specific ignores
```

#### TypeScript Template (`typescript`)
```
├── package.json           # npm package manifest
├── tsconfig.json          # TypeScript config
├── src/
│   └── index.ts           # Entry point
├── tests/
│   └── .gitkeep
└── .gitignore             # Node-specific ignores
```

#### Generic Template (`generic`)
```
# Only core spec-kit files, no language-specific structure
└── .gitignore             # Minimal ignores
```

### FR-4: Interactive Mode
When invoked without arguments:
1. Prompt for project type (select from list)
2. Prompt for project name (text input)
3. Validate inputs
4. Create project

### FR-5: Validation
- Project name: lowercase letters, numbers, hyphens only
- Directory must not exist (fail with helpful message)
- Parent directory must exist and be writable

## 3. Non-Functional Requirements

### NFR-1: Performance
- Execution time: <1 second
- No network calls
- No agent invocations

### NFR-2: Implementation
- Pure Rust native implementation
- Templates embedded in binary (no external files)
- Tier 0 classification (zero agents, $0)

### NFR-3: Idempotency
- If directory exists, fail with clear error
- No partial creates (atomic: all or nothing)

### NFR-4: Cross-Platform
- Works on Linux, macOS, Windows
- Uses platform-appropriate path separators
- UTF-8 encoding for all files

## 4. Template Content

### 4.1 CLAUDE.md Template
```markdown
# CLAUDE.md — [PROJECT_NAME] Instructions

## Repository Context
**Project**: [PROJECT_NAME]
**Created**: [DATE]
**Type**: [PROJECT_TYPE]

## Spec-Kit Workflow
This project uses spec-kit for structured development:
- `/speckit.new <description>` - Create new SPEC
- `/speckit.auto SPEC-ID` - Full automation pipeline
- `/speckit.status SPEC-ID` - Check progress

## Getting Started
1. Define your first feature with `/speckit.new`
2. Review generated PRD in `docs/SPEC-*/PRD.md`
3. Run `/speckit.auto` to implement

## Project Structure
- `docs/` - SPEC directories and documentation
- `memory/` - Project charter and context
- `SPEC.md` - Task tracking table

## Build Commands
[PROJECT_TYPE specific commands]

## Testing
[PROJECT_TYPE specific test commands]
```

### 4.2 SPEC.md Template
```markdown
# SPEC Tracker

| # | ID | Title | Status | Owner | PRD | Branch | PR | Created | Summary | Notes |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | SPEC-001 | Initial setup | Done | - | - | main | - | [DATE] | Project scaffolded | Created via /speckit.project |
```

### 4.3 constitution.md Template
```markdown
# Project Constitution — [PROJECT_NAME]

## Mission
[Define the project's core purpose]

## Principles
1. [Core principle 1]
2. [Core principle 2]
3. [Core principle 3]

## Constraints
- [Technical constraint]
- [Business constraint]

## Quality Standards
- All code must pass linting
- Tests required for new features
- Documentation for public APIs

## Decision Log
| Date | Decision | Rationale |
|------|----------|-----------|
| [DATE] | Project created | Scaffolded via /speckit.project |
```

### 4.4 Type-Specific Files

#### Rust: Cargo.toml
```toml
[package]
name = "[PROJECT_NAME]"
version = "0.1.0"
edition = "2024"

[dependencies]

[dev-dependencies]
```

#### Rust: src/lib.rs
```rust
//! [PROJECT_NAME] - [DESCRIPTION]

pub fn hello() -> &'static str {
    "Hello from [PROJECT_NAME]!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        assert_eq!(hello(), "Hello from [PROJECT_NAME]!");
    }
}
```

#### Python: pyproject.toml
```toml
[project]
name = "[PROJECT_NAME]"
version = "0.1.0"
description = ""
requires-python = ">=3.11"
dependencies = []

[project.optional-dependencies]
dev = ["pytest", "ruff"]

[tool.ruff]
line-length = 88
```

#### Python: __init__.py
```python
"""[PROJECT_NAME] package."""

__version__ = "0.1.0"
```

#### TypeScript: package.json
```json
{
  "name": "[PROJECT_NAME]",
  "version": "0.1.0",
  "type": "module",
  "main": "dist/index.js",
  "scripts": {
    "build": "tsc",
    "test": "vitest"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "vitest": "^1.0.0"
  }
}
```

#### TypeScript: tsconfig.json
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "node",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

#### TypeScript: src/index.ts
```typescript
/**
 * [PROJECT_NAME] entry point
 */

export function hello(): string {
  return "Hello from [PROJECT_NAME]!";
}
```

## 5. Implementation Phases

### Phase 1: Command Registration (20 min)
- Create `commands/project.rs`
- Define `SpecKitProjectCommand` struct
- Register in `command_registry.rs`
- Add to `mod.rs`

### Phase 2: Template System (45 min)
- Create `ProjectTemplate` enum with variants
- Define file generation methods for each template
- Use `const` strings for embedded templates

### Phase 3: Project Creation Logic (45 min)
- Parse arguments (type, name)
- Validate inputs
- Create directory structure atomically
- Write template files
- Handle errors gracefully

### Phase 4: Interactive Mode (30 min)
- Detect when no args provided
- Build TUI prompts or simple input
- Validate and proceed to creation

### Phase 5: Testing (30 min)
- Unit tests for template generation
- Integration test for full project creation
- Verify file contents

## 6. Testing Plan

### Unit Tests
- `test_project_name_validation()` - Valid/invalid names
- `test_template_content_generation()` - Each template type
- `test_substitution_variables()` - Placeholder replacement

### Integration Tests
- `test_create_rust_project()` - Full Rust scaffold
- `test_create_python_project()` - Full Python scaffold
- `test_create_typescript_project()` - Full TypeScript scaffold
- `test_create_generic_project()` - Full Generic scaffold
- `test_existing_directory_fails()` - Error handling
- `test_interactive_mode()` - Prompt flow

### Manual Validation
```bash
# After implementation
/speckit.project rust my-rust-lib
ls -la my-rust-lib/
cat my-rust-lib/CLAUDE.md
cat my-rust-lib/Cargo.toml

/speckit.project python my-py-app
/speckit.project typescript my-ts-lib
/speckit.project generic minimal-spec
```

## 7. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Template drift from best practices | Medium | Version templates, review quarterly |
| Cross-platform path issues | Low | Use Rust std::path, test on all platforms |
| Large binary size from embedded templates | Low | Templates are text, <10KB total |

## 8. Future Considerations

- **Template customization**: User-defined templates in `~/.code/templates/`
- **Git initialization**: Optional `--git` flag to run `git init`
- **More languages**: Go, Java, C++ templates on demand
- **Template updates**: `/speckit.project --upgrade` to update existing CLAUDE.md

## 9. References

- SPEC-KIT-902: Native routing architecture
- `/speckit.new`: Similar native command pattern
- `new_native.rs`: Reference implementation for file creation

---

## 10. Completion Notes (P58)

**Implemented**: 2025-11-29
**Commits**: `0316aa037`, `55ca08937`

### Final Implementation

**Primary file**: `codex-rs/tui/src/chatwidget/spec_kit/project_native.rs`

### Files Created Per Template

**All templates include:**
- `CLAUDE.md` - Project instructions with type-specific build commands
- `SPEC.md` - Task tracker (initialized)
- `product-requirements.md` - PRD structure
- `PLANNING.md` - Architecture and build commands
- `templates/PRD-template.md` - PRD template
- `templates/spec-template.md` - SPEC template
- `docs/.gitkeep` - Documentation directory
- `memory/constitution.md` - Project charter

**Rust-specific:**
- `Cargo.toml`, `src/lib.rs`, `tests/.gitkeep`, `.gitignore`

**Python-specific:**
- `pyproject.toml`, `src/<name>/__init__.py`, `tests/__init__.py`, `.gitignore`

**TypeScript-specific:**
- `package.json`, `tsconfig.json`, `src/index.ts`, `tests/.gitkeep`, `.gitignore`

**Generic:**
- `.gitignore` (minimal)

### Key Decisions

1. **Embedded templates**: Templates are const strings in binary, not external files
2. **CLAUDE.md Section 1 compliance**: Added `product-requirements.md` and `PLANNING.md` per mandatory references
3. **Tier 0 classification**: Zero agents, $0 cost, <1s execution
4. **Variable substitution**: `[PROJECT_NAME]`, `[PROJECT_TYPE]`, `[DATE]`, `[DESCRIPTION]` patterns

### Future Enhancements (Not Implemented)

- Go template (SPEC-KIT-961 candidate)
- Full 11-template directory (currently 2)
- Interactive mode
- `--git` flag for git init
