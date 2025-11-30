//! Native project scaffolding - creates spec-kit ready project structure
//!
//! FORK-SPECIFIC (just-every/code): SPEC-KIT-960 - /speckit.project command
//!
//! Scaffolds new projects with spec-kit workflow infrastructure.
//! Pure Rust implementation - zero agents, $0 cost, <1s execution.
//!
//! SPEC-KIT-962: Uses resolve_template() for layered template resolution.

use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

use super::error::SpecKitError;
use crate::templates::resolve_template;

/// Project template types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Python,
    TypeScript,
    Generic,
}

impl ProjectType {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Self::Rust),
            "python" | "py" => Some(Self::Python),
            "typescript" | "ts" => Some(Self::TypeScript),
            "generic" | "gen" => Some(Self::Generic),
            _ => None,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::TypeScript => "TypeScript",
            Self::Generic => "Generic",
        }
    }

    /// Get valid type strings for help
    pub fn valid_types() -> &'static str {
        "rust, python, typescript, generic"
    }
}

/// Result of successful project creation
#[derive(Debug)]
pub struct ProjectCreationResult {
    pub project_type: ProjectType,
    pub project_name: String,
    pub directory: PathBuf,
    pub files_created: Vec<String>,
}

/// Create a new project with spec-kit infrastructure
///
/// # Arguments
/// * `project_type` - Type of project (rust, python, typescript, generic)
/// * `project_name` - Name for the project directory
/// * `parent_dir` - Directory to create project in
///
/// # Returns
/// * `Ok(ProjectCreationResult)` - Project created successfully
/// * `Err(SpecKitError)` - Creation failed
pub fn create_project(
    project_type: ProjectType,
    project_name: &str,
    parent_dir: &Path,
) -> Result<ProjectCreationResult, SpecKitError> {
    // Validate project name
    validate_project_name(project_name)?;

    // Create project directory
    let project_dir = parent_dir.join(project_name);
    if project_dir.exists() {
        return Err(SpecKitError::Other(format!(
            "Directory '{}' already exists. Choose a different name or remove existing directory.",
            project_name
        )));
    }

    fs::create_dir(&project_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: project_dir.clone(),
        source: e,
    })?;

    let mut files_created = Vec::new();
    let date = Local::now().format("%Y-%m-%d").to_string();

    // Create core spec-kit files (CLAUDE.md Section 1 requirements)
    create_claude_md(&project_dir, project_name, project_type, &date, &mut files_created)?;
    create_spec_md(&project_dir, &date, &mut files_created)?;
    create_product_requirements(&project_dir, project_name, &date, &mut files_created)?;
    create_planning_md(&project_dir, project_name, project_type, &date, &mut files_created)?;
    create_docs_dir(&project_dir, &mut files_created)?;
    create_constitution(&project_dir, project_name, &date, &mut files_created)?;
    create_templates_dir(&project_dir, &mut files_created)?;

    // Create type-specific files
    match project_type {
        ProjectType::Rust => create_rust_files(&project_dir, project_name, &mut files_created)?,
        ProjectType::Python => {
            create_python_files(&project_dir, project_name, &mut files_created)?
        }
        ProjectType::TypeScript => {
            create_typescript_files(&project_dir, project_name, &mut files_created)?
        }
        ProjectType::Generic => create_generic_files(&project_dir, &mut files_created)?,
    }

    Ok(ProjectCreationResult {
        project_type,
        project_name: project_name.to_string(),
        directory: project_dir,
        files_created,
    })
}

/// Validate project name
fn validate_project_name(name: &str) -> Result<(), SpecKitError> {
    if name.is_empty() {
        return Err(SpecKitError::Other(
            "Project name cannot be empty".to_string(),
        ));
    }

    if name.len() > 100 {
        return Err(SpecKitError::Other(
            "Project name too long (max 100 characters)".to_string(),
        ));
    }

    // Allow lowercase, numbers, hyphens, underscores
    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' && c != '_' {
            return Err(SpecKitError::Other(format!(
                "Invalid character '{}' in project name. Use lowercase letters, numbers, hyphens, or underscores.",
                c
            )));
        }
    }

    // Must start with letter
    if !name.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
        return Err(SpecKitError::Other(
            "Project name must start with a lowercase letter".to_string(),
        ));
    }

    Ok(())
}

/// Create CLAUDE.md
fn create_claude_md(
    project_dir: &Path,
    project_name: &str,
    project_type: ProjectType,
    date: &str,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    let (build_cmd, test_cmd) = match project_type {
        ProjectType::Rust => ("cargo build", "cargo test"),
        ProjectType::Python => ("uv sync", "pytest"),
        ProjectType::TypeScript => ("npm run build", "npm test"),
        ProjectType::Generic => ("# Add your build command", "# Add your test command"),
    };

    let content = format!(
        r#"# CLAUDE.md - {name} Instructions

## Repository Context
**Project**: {name}
**Created**: {date}
**Type**: {type_name}

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
```bash
{build_cmd}
```

## Testing
```bash
{test_cmd}
```
"#,
        name = project_name,
        date = date,
        type_name = project_type.display_name(),
        build_cmd = build_cmd,
        test_cmd = test_cmd,
    );

    let path = project_dir.join("CLAUDE.md");
    fs::write(&path, content).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("CLAUDE.md".to_string());
    Ok(())
}

/// Create SPEC.md tracker
fn create_spec_md(
    project_dir: &Path,
    date: &str,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    let content = format!(
        r#"# SPEC Tracker

| # | ID | Title | Status | Owner | PRD | Branch | PR | Created | Summary | Notes |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | SPEC-001 | Initial setup | Done | - | - | main | - | {date} | Project scaffolded | Created via /speckit.project |
"#,
        date = date
    );

    let path = project_dir.join("SPEC.md");
    fs::write(&path, content).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("SPEC.md".to_string());
    Ok(())
}

/// Create product-requirements.md (CLAUDE.md Section 1 requirement)
fn create_product_requirements(
    project_dir: &Path,
    project_name: &str,
    date: &str,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    let content = format!(
        r#"# {name} - Product Requirements

> Status: v1.0 ({date}) - Initial draft

## 1. Product Summary
- **Product name:** {name}
- **Domain:** [Define the domain/category]
- **Mission:** [Define the core mission in one sentence]

## 2. Primary Users & Goals
- **[User Type 1]** - [What they want to accomplish]
- **[User Type 2]** - [What they want to accomplish]

## 3. Core Features

### Must Have (P0)
- [ ] [Critical feature 1]
- [ ] [Critical feature 2]

### Should Have (P1)
- [ ] [Important feature 1]
- [ ] [Important feature 2]

### Nice to Have (P2)
- [ ] [Enhancement 1]
- [ ] [Enhancement 2]

## 4. Non-Goals (Out of Scope)
- [Explicitly excluded feature 1]
- [Explicitly excluded feature 2]

## 5. Success Metrics
| Metric | Target | Measurement |
|--------|--------|-------------|
| [KPI 1] | [Target] | [How to measure] |
| [KPI 2] | [Target] | [How to measure] |

## 6. Constraints
- **Technical:** [Technical constraints]
- **Timeline:** [Timeline constraints]
- **Resources:** [Resource constraints]

## 7. Dependencies
- [External dependency 1]
- [External dependency 2]

---
*Use `/speckit.specify SPEC-ID` to refine requirements for specific features.*
"#,
        name = project_name,
        date = date
    );

    let path = project_dir.join("product-requirements.md");
    fs::write(&path, content).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("product-requirements.md".to_string());
    Ok(())
}

/// Create PLANNING.md (CLAUDE.md Section 1 requirement)
fn create_planning_md(
    project_dir: &Path,
    project_name: &str,
    project_type: ProjectType,
    date: &str,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    let (lang_section, build_info) = match project_type {
        ProjectType::Rust => (
            "- **Primary language:** Rust",
            "- `cargo build` - Build the project\n- `cargo test` - Run tests\n- `cargo clippy` - Lint checks",
        ),
        ProjectType::Python => (
            "- **Primary language:** Python",
            "- `uv sync` - Install dependencies\n- `pytest` - Run tests\n- `ruff check` - Lint checks",
        ),
        ProjectType::TypeScript => (
            "- **Primary language:** TypeScript",
            "- `npm install` - Install dependencies\n- `npm run build` - Build\n- `npm test` - Run tests",
        ),
        ProjectType::Generic => (
            "- **Primary language:** [Specify]",
            "- [Add build commands]\n- [Add test commands]",
        ),
    };

    let content = format!(
        r#"# {name} - Architecture & Planning

> Status: v1.0 ({date}) - Initial structure

## 1. Repository Overview
- **Repository:** [Add repository URL]
{lang_section}
- **Key directories:**
  - `docs/` - SPEC directories and documentation
  - `memory/` - Constitution and project context
  - `templates/` - PRD and spec templates for /speckit.new

## 2. Architecture

### 2.1 Component Structure
```
[Describe your component architecture]
```

### 2.2 Data Flow
```
[Input] -> [Processing] -> [Output]
```

## 3. Development Workflow

### Spec-Kit Integration
This project uses spec-kit for structured development:
1. `/speckit.new <description>` - Create new SPEC
2. `/speckit.auto SPEC-ID` - Run full automation pipeline
3. Review generated artifacts in `docs/SPEC-*/`

### Build Commands
{build_info}

## 4. Key Decisions

| Date | Decision | Rationale |
|------|----------|-----------|
| {date} | Project structure established | Spec-kit ready scaffold |

## 5. Constraints & Risks

### Technical Constraints
- [Constraint 1]

### Known Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| [Risk 1] | [Impact] | [Mitigation] |

## 6. Roadmap

### Phase 1: Foundation
- [ ] Core implementation
- [ ] Basic testing

### Phase 2: Enhancement
- [ ] Additional features
- [ ] Documentation

---
*Complements `product-requirements.md` - see that file for feature requirements.*
"#,
        name = project_name,
        date = date,
        lang_section = lang_section,
        build_info = build_info
    );

    let path = project_dir.join("PLANNING.md");
    fs::write(&path, content).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("PLANNING.md".to_string());
    Ok(())
}

/// Create templates/ directory with PRD and spec templates
///
/// SPEC-KIT-962: Uses resolve_template() to get template content from the
/// layered resolution system (project-local -> user config -> embedded).
fn create_templates_dir(
    project_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    let templates_dir = project_dir.join("templates");
    fs::create_dir(&templates_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: templates_dir.clone(),
        source: e,
    })?;

    // PRD template (from embedded defaults or user config)
    let prd_content = resolve_template("prd");
    let path = templates_dir.join("PRD-template.md");
    fs::write(&path, &prd_content).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("templates/PRD-template.md".to_string());

    // Spec template (from embedded defaults or user config)
    let spec_content = resolve_template("spec");
    let path = templates_dir.join("spec-template.md");
    fs::write(&path, &spec_content).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("templates/spec-template.md".to_string());

    Ok(())
}

/// Create docs/ directory
fn create_docs_dir(project_dir: &Path, files: &mut Vec<String>) -> Result<(), SpecKitError> {
    let docs_dir = project_dir.join("docs");
    fs::create_dir(&docs_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: docs_dir.clone(),
        source: e,
    })?;

    let gitkeep = docs_dir.join(".gitkeep");
    fs::write(&gitkeep, "").map_err(|e| SpecKitError::FileWrite {
        path: gitkeep.clone(),
        source: e,
    })?;
    files.push("docs/.gitkeep".to_string());
    Ok(())
}

/// Create memory/constitution.md
fn create_constitution(
    project_dir: &Path,
    project_name: &str,
    date: &str,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    let memory_dir = project_dir.join("memory");
    fs::create_dir(&memory_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: memory_dir.clone(),
        source: e,
    })?;

    let content = format!(
        r#"# Project Constitution - {name}

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
| {date} | Project created | Scaffolded via /speckit.project |
"#,
        name = project_name,
        date = date
    );

    let path = memory_dir.join("constitution.md");
    fs::write(&path, content).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("memory/constitution.md".to_string());
    Ok(())
}

/// Create Rust-specific files
fn create_rust_files(
    project_dir: &Path,
    project_name: &str,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]

[dev-dependencies]
"#,
        name = project_name
    );

    let path = project_dir.join("Cargo.toml");
    fs::write(&path, cargo_toml).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("Cargo.toml".to_string());

    // src/lib.rs
    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: src_dir.clone(),
        source: e,
    })?;

    let lib_rs = format!(
        r#"//! {name} - TODO: Add description

pub fn hello() -> &'static str {{
    "Hello from {name}!"
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_hello() {{
        assert_eq!(hello(), "Hello from {name}!");
    }}
}}
"#,
        name = project_name
    );

    let path = src_dir.join("lib.rs");
    fs::write(&path, lib_rs).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("src/lib.rs".to_string());

    // tests/
    let tests_dir = project_dir.join("tests");
    fs::create_dir(&tests_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: tests_dir.clone(),
        source: e,
    })?;

    let gitkeep = tests_dir.join(".gitkeep");
    fs::write(&gitkeep, "").map_err(|e| SpecKitError::FileWrite {
        path: gitkeep.clone(),
        source: e,
    })?;
    files.push("tests/.gitkeep".to_string());

    // .gitignore
    let gitignore = r#"/target/
Cargo.lock
**/*.rs.bk
*.pdb
"#;

    let path = project_dir.join(".gitignore");
    fs::write(&path, gitignore).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push(".gitignore".to_string());

    Ok(())
}

/// Create Python-specific files
fn create_python_files(
    project_dir: &Path,
    project_name: &str,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    // Convert project name to valid Python package name
    let package_name = project_name.replace('-', "_");

    // pyproject.toml
    let pyproject = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"
description = ""
requires-python = ">=3.11"
dependencies = []

[project.optional-dependencies]
dev = ["pytest", "ruff"]

[tool.ruff]
line-length = 88
"#,
        name = project_name
    );

    let path = project_dir.join("pyproject.toml");
    fs::write(&path, pyproject).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("pyproject.toml".to_string());

    // src/<package>/
    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: src_dir.clone(),
        source: e,
    })?;

    let pkg_dir = src_dir.join(&package_name);
    fs::create_dir(&pkg_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: pkg_dir.clone(),
        source: e,
    })?;

    let init_py = format!(
        r#""""{name} package."""

__version__ = "0.1.0"
"#,
        name = project_name
    );

    let path = pkg_dir.join("__init__.py");
    fs::write(&path, init_py).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push(format!("src/{}/__init__.py", package_name));

    // tests/
    let tests_dir = project_dir.join("tests");
    fs::create_dir(&tests_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: tests_dir.clone(),
        source: e,
    })?;

    let path = tests_dir.join("__init__.py");
    fs::write(&path, "").map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("tests/__init__.py".to_string());

    // .gitignore
    let gitignore = r#"__pycache__/
*.py[cod]
*$py.class
.Python
build/
dist/
*.egg-info/
.venv/
.env
.ruff_cache/
"#;

    let path = project_dir.join(".gitignore");
    fs::write(&path, gitignore).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push(".gitignore".to_string());

    Ok(())
}

/// Create TypeScript-specific files
fn create_typescript_files(
    project_dir: &Path,
    project_name: &str,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    // package.json
    let package_json = format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "main": "dist/index.js",
  "scripts": {{
    "build": "tsc",
    "test": "vitest"
  }},
  "devDependencies": {{
    "typescript": "^5.0.0",
    "vitest": "^1.0.0"
  }}
}}
"#,
        name = project_name
    );

    let path = project_dir.join("package.json");
    fs::write(&path, package_json).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("package.json".to_string());

    // tsconfig.json
    let tsconfig = r#"{
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
"#;

    let path = project_dir.join("tsconfig.json");
    fs::write(&path, tsconfig).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("tsconfig.json".to_string());

    // src/
    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: src_dir.clone(),
        source: e,
    })?;

    let index_ts = format!(
        r#"/**
 * {name} entry point
 */

export function hello(): string {{
  return "Hello from {name}!";
}}
"#,
        name = project_name
    );

    let path = src_dir.join("index.ts");
    fs::write(&path, index_ts).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push("src/index.ts".to_string());

    // tests/
    let tests_dir = project_dir.join("tests");
    fs::create_dir(&tests_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: tests_dir.clone(),
        source: e,
    })?;

    let gitkeep = tests_dir.join(".gitkeep");
    fs::write(&gitkeep, "").map_err(|e| SpecKitError::FileWrite {
        path: gitkeep.clone(),
        source: e,
    })?;
    files.push("tests/.gitkeep".to_string());

    // .gitignore
    let gitignore = r#"node_modules/
dist/
*.log
.env
.env.local
"#;

    let path = project_dir.join(".gitignore");
    fs::write(&path, gitignore).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push(".gitignore".to_string());

    Ok(())
}

/// Create Generic-specific files (minimal)
fn create_generic_files(
    project_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), SpecKitError> {
    // Only .gitignore for generic projects
    let gitignore = r#"# Add project-specific ignores here
*.log
.env
"#;

    let path = project_dir.join(".gitignore");
    fs::write(&path, gitignore).map_err(|e| SpecKitError::FileWrite {
        path: path.clone(),
        source: e,
    })?;
    files.push(".gitignore".to_string());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_project_type_from_str() {
        assert_eq!(ProjectType::from_str("rust"), Some(ProjectType::Rust));
        assert_eq!(ProjectType::from_str("rs"), Some(ProjectType::Rust));
        assert_eq!(ProjectType::from_str("RUST"), Some(ProjectType::Rust));
        assert_eq!(ProjectType::from_str("python"), Some(ProjectType::Python));
        assert_eq!(ProjectType::from_str("py"), Some(ProjectType::Python));
        assert_eq!(
            ProjectType::from_str("typescript"),
            Some(ProjectType::TypeScript)
        );
        assert_eq!(ProjectType::from_str("ts"), Some(ProjectType::TypeScript));
        assert_eq!(ProjectType::from_str("generic"), Some(ProjectType::Generic));
        assert_eq!(ProjectType::from_str("gen"), Some(ProjectType::Generic));
        assert_eq!(ProjectType::from_str("invalid"), None);
    }

    #[test]
    fn test_validate_project_name_valid() {
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("myproject").is_ok());
        assert!(validate_project_name("my_project").is_ok());
        assert!(validate_project_name("project123").is_ok());
        assert!(validate_project_name("a").is_ok());
    }

    #[test]
    fn test_validate_project_name_invalid() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("MyProject").is_err()); // uppercase
        assert!(validate_project_name("123project").is_err()); // starts with number
        assert!(validate_project_name("my project").is_err()); // space
        assert!(validate_project_name("my.project").is_err()); // dot
    }

    #[test]
    fn test_create_rust_project() {
        let temp = TempDir::new().unwrap();
        let result = create_project(ProjectType::Rust, "my-rust-lib", temp.path()).unwrap();

        assert_eq!(result.project_type, ProjectType::Rust);
        assert_eq!(result.project_name, "my-rust-lib");
        assert!(result.directory.exists());
        // Core spec-kit files (CLAUDE.md Section 1 requirements)
        assert!(result.directory.join("CLAUDE.md").exists());
        assert!(result.directory.join("SPEC.md").exists());
        assert!(result.directory.join("product-requirements.md").exists());
        assert!(result.directory.join("PLANNING.md").exists());
        assert!(result.directory.join("docs").is_dir());
        assert!(result.directory.join("memory/constitution.md").exists());
        assert!(result.directory.join("templates/PRD-template.md").exists());
        assert!(result.directory.join("templates/spec-template.md").exists());
        // Type-specific files
        assert!(result.directory.join("Cargo.toml").exists());
        assert!(result.directory.join("src/lib.rs").exists());
        assert!(result.directory.join(".gitignore").exists());
    }

    #[test]
    fn test_create_python_project() {
        let temp = TempDir::new().unwrap();
        let result = create_project(ProjectType::Python, "my-py-app", temp.path()).unwrap();

        assert_eq!(result.project_type, ProjectType::Python);
        assert!(result.directory.join("pyproject.toml").exists());
        assert!(result.directory.join("src/my_py_app/__init__.py").exists());
        assert!(result.directory.join("tests/__init__.py").exists());
    }

    #[test]
    fn test_create_typescript_project() {
        let temp = TempDir::new().unwrap();
        let result = create_project(ProjectType::TypeScript, "my-ts-lib", temp.path()).unwrap();

        assert_eq!(result.project_type, ProjectType::TypeScript);
        assert!(result.directory.join("package.json").exists());
        assert!(result.directory.join("tsconfig.json").exists());
        assert!(result.directory.join("src/index.ts").exists());
    }

    #[test]
    fn test_create_generic_project() {
        let temp = TempDir::new().unwrap();
        let result = create_project(ProjectType::Generic, "minimal-spec", temp.path()).unwrap();

        assert_eq!(result.project_type, ProjectType::Generic);
        // Core spec-kit files (CLAUDE.md Section 1 requirements)
        assert!(result.directory.join("CLAUDE.md").exists());
        assert!(result.directory.join("SPEC.md").exists());
        assert!(result.directory.join("product-requirements.md").exists());
        assert!(result.directory.join("PLANNING.md").exists());
        assert!(result.directory.join("docs").is_dir());
        assert!(result.directory.join("memory/constitution.md").exists());
        assert!(result.directory.join("templates/PRD-template.md").exists());
        assert!(result.directory.join("templates/spec-template.md").exists());
        // Only .gitignore for type-specific
        assert!(result.directory.join(".gitignore").exists());
        // No src, Cargo.toml, etc
        assert!(!result.directory.join("src").exists());
        assert!(!result.directory.join("Cargo.toml").exists());
    }

    #[test]
    fn test_existing_directory_fails() {
        let temp = TempDir::new().unwrap();
        let existing = temp.path().join("existing");
        fs::create_dir(&existing).unwrap();

        let result = create_project(ProjectType::Generic, "existing", temp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_claude_md_content() {
        let temp = TempDir::new().unwrap();
        create_project(ProjectType::Rust, "test-proj", temp.path()).unwrap();

        let content = fs::read_to_string(temp.path().join("test-proj/CLAUDE.md")).unwrap();
        assert!(content.contains("test-proj"));
        assert!(content.contains("Rust"));
        assert!(content.contains("/speckit.new"));
        assert!(content.contains("cargo build"));
    }

    #[test]
    fn test_spec_md_content() {
        let temp = TempDir::new().unwrap();
        create_project(ProjectType::Generic, "test-proj", temp.path()).unwrap();

        let content = fs::read_to_string(temp.path().join("test-proj/SPEC.md")).unwrap();
        assert!(content.contains("SPEC Tracker"));
        assert!(content.contains("SPEC-001"));
        assert!(content.contains("/speckit.project"));
    }

    #[test]
    fn test_product_requirements_content() {
        let temp = TempDir::new().unwrap();
        create_project(ProjectType::Rust, "test-proj", temp.path()).unwrap();

        let content =
            fs::read_to_string(temp.path().join("test-proj/product-requirements.md")).unwrap();
        assert!(content.contains("test-proj"));
        assert!(content.contains("Product Summary"));
        assert!(content.contains("Primary Users"));
        assert!(content.contains("Core Features"));
        assert!(content.contains("/speckit.specify"));
    }

    #[test]
    fn test_planning_md_content() {
        let temp = TempDir::new().unwrap();
        create_project(ProjectType::Rust, "test-proj", temp.path()).unwrap();

        let content = fs::read_to_string(temp.path().join("test-proj/PLANNING.md")).unwrap();
        assert!(content.contains("test-proj"));
        assert!(content.contains("Architecture"));
        assert!(content.contains("Rust"));
        assert!(content.contains("cargo build"));
        assert!(content.contains("/speckit.new"));
    }

    #[test]
    fn test_templates_content() {
        let temp = TempDir::new().unwrap();
        create_project(ProjectType::Generic, "test-proj", temp.path()).unwrap();

        // PRD template
        let prd = fs::read_to_string(temp.path().join("test-proj/templates/PRD-template.md")).unwrap();
        assert!(prd.contains("[FEATURE_NAME]"));
        assert!(prd.contains("[SPEC_ID]"));
        assert!(prd.contains("Problem Statement"));

        // Spec template
        let spec = fs::read_to_string(temp.path().join("test-proj/templates/spec-template.md")).unwrap();
        assert!(spec.contains("[SPEC_ID]"));
        assert!(spec.contains("Requirements"));
    }
}
