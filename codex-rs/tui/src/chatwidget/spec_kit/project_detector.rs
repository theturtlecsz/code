//! Project type detection for context-aware question customization (SPEC-KIT-971)
//!
//! Detects project type from filesystem markers to customize PRD builder questions.
//! Zero cost, instant detection based on config file presence.

use std::path::Path;

/// Detected project type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Generic,
}

impl ProjectType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProjectType::Rust => "Rust",
            ProjectType::Python => "Python",
            ProjectType::TypeScript => "TypeScript",
            ProjectType::JavaScript => "JavaScript",
            ProjectType::Go => "Go",
            ProjectType::Generic => "Generic",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ProjectType::Rust => "🦀",
            ProjectType::Python => "🐍",
            ProjectType::TypeScript => "📘",
            ProjectType::JavaScript => "📒",
            ProjectType::Go => "🐹",
            ProjectType::Generic => "📦",
        }
    }
}

/// Detect project type from current working directory
///
/// Checks for standard config files in priority order:
/// 1. Cargo.toml → Rust
/// 2. go.mod → Go
/// 3. pyproject.toml / setup.py / requirements.txt → Python
/// 4. package.json with typescript dep → TypeScript
/// 5. package.json without typescript → JavaScript
/// 6. fallback → Generic
pub fn detect_project_type(cwd: &Path) -> ProjectType {
    // Rust: Cargo.toml
    if cwd.join("Cargo.toml").exists() {
        return ProjectType::Rust;
    }

    // Go: go.mod
    if cwd.join("go.mod").exists() {
        return ProjectType::Go;
    }

    // Python: pyproject.toml, setup.py, or requirements.txt
    if cwd.join("pyproject.toml").exists()
        || cwd.join("setup.py").exists()
        || cwd.join("requirements.txt").exists()
    {
        return ProjectType::Python;
    }

    // JavaScript/TypeScript: package.json
    let package_json = cwd.join("package.json");
    if package_json.exists() {
        // Check for TypeScript indicators
        if has_typescript_dependency(&package_json)
            || cwd.join("tsconfig.json").exists()
            || cwd.join("src").join("index.ts").exists()
        {
            return ProjectType::TypeScript;
        }
        return ProjectType::JavaScript;
    }

    ProjectType::Generic
}

/// Check if package.json has typescript as a dependency
fn has_typescript_dependency(package_json: &Path) -> bool {
    let content = match std::fs::read_to_string(package_json) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Simple check: look for "typescript" in the file
    // More robust would be to parse JSON, but this is fast and good enough
    content.contains("\"typescript\"")
}

/// Get project-specific questions for the PRD builder
pub fn get_project_questions(project_type: ProjectType) -> Vec<ProjectQuestion> {
    match project_type {
        ProjectType::Rust => rust_questions(),
        ProjectType::Python => python_questions(),
        ProjectType::TypeScript => typescript_questions(),
        ProjectType::JavaScript => javascript_questions(),
        ProjectType::Go => go_questions(),
        ProjectType::Generic => generic_questions(),
    }
}

/// A question with predefined options for the PRD builder
#[derive(Clone)]
pub struct ProjectQuestion {
    pub category: &'static str,
    pub question: &'static str,
    pub options: Vec<ProjectOption>,
}

/// An option for a question
#[derive(Clone)]
pub struct ProjectOption {
    pub label: char,
    pub text: &'static str,
    pub is_custom: bool,
}

// =============================================================================
// Project-Specific Question Sets
// =============================================================================

fn rust_questions() -> Vec<ProjectQuestion> {
    vec![
        ProjectQuestion {
            category: "Artifact",
            question: "What type of Rust artifact is this?",
            options: vec![
                ProjectOption { label: 'A', text: "Library crate (lib.rs)", is_custom: false },
                ProjectOption { label: 'B', text: "Binary crate (main.rs)", is_custom: false },
                ProjectOption { label: 'C', text: "Workspace member", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Target",
            question: "Who is the primary user?",
            options: vec![
                ProjectOption { label: 'A', text: "Rust developers (API consumers)", is_custom: false },
                ProjectOption { label: 'B', text: "CLI users", is_custom: false },
                ProjectOption { label: 'C', text: "Internal codebase only", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Success",
            question: "How will you know it's complete?",
            options: vec![
                ProjectOption { label: 'A', text: "cargo test passes", is_custom: false },
                ProjectOption { label: 'B', text: "cargo clippy clean", is_custom: false },
                ProjectOption { label: 'C', text: "Documented with rustdoc", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
    ]
}

fn python_questions() -> Vec<ProjectQuestion> {
    vec![
        ProjectQuestion {
            category: "Artifact",
            question: "What type of Python artifact is this?",
            options: vec![
                ProjectOption { label: 'A', text: "Library/Package (pip installable)", is_custom: false },
                ProjectOption { label: 'B', text: "CLI tool", is_custom: false },
                ProjectOption { label: 'C', text: "Web application (FastAPI/Django)", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Async",
            question: "Does this require async/await?",
            options: vec![
                ProjectOption { label: 'A', text: "Yes, async throughout", is_custom: false },
                ProjectOption { label: 'B', text: "No, synchronous", is_custom: false },
                ProjectOption { label: 'C', text: "Mixed (async-optional)", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Success",
            question: "How will you know it's complete?",
            options: vec![
                ProjectOption { label: 'A', text: "pytest passes", is_custom: false },
                ProjectOption { label: 'B', text: "Type hints validated (mypy)", is_custom: false },
                ProjectOption { label: 'C', text: "Feature works end-to-end", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
    ]
}

fn typescript_questions() -> Vec<ProjectQuestion> {
    vec![
        ProjectQuestion {
            category: "Platform",
            question: "What platform is this for?",
            options: vec![
                ProjectOption { label: 'A', text: "Frontend (React/Vue/Svelte)", is_custom: false },
                ProjectOption { label: 'B', text: "Backend (Node.js/Deno)", is_custom: false },
                ProjectOption { label: 'C', text: "Full-stack (Next.js/Nuxt)", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Target",
            question: "Who is the primary user?",
            options: vec![
                ProjectOption { label: 'A', text: "End-users (UI consumers)", is_custom: false },
                ProjectOption { label: 'B', text: "Developers (API/SDK)", is_custom: false },
                ProjectOption { label: 'C', text: "Internal team only", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Success",
            question: "How will you know it's complete?",
            options: vec![
                ProjectOption { label: 'A', text: "Tests pass (Jest/Vitest)", is_custom: false },
                ProjectOption { label: 'B', text: "Type-safe (tsc --noEmit)", is_custom: false },
                ProjectOption { label: 'C', text: "E2E tests pass (Playwright)", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
    ]
}

fn javascript_questions() -> Vec<ProjectQuestion> {
    vec![
        ProjectQuestion {
            category: "Platform",
            question: "What platform is this for?",
            options: vec![
                ProjectOption { label: 'A', text: "Frontend (Browser)", is_custom: false },
                ProjectOption { label: 'B', text: "Backend (Node.js)", is_custom: false },
                ProjectOption { label: 'C', text: "Universal (Isomorphic)", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Target",
            question: "Who is the primary user?",
            options: vec![
                ProjectOption { label: 'A', text: "End-users", is_custom: false },
                ProjectOption { label: 'B', text: "Developers", is_custom: false },
                ProjectOption { label: 'C', text: "Internal team", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Success",
            question: "How will you know it's complete?",
            options: vec![
                ProjectOption { label: 'A', text: "Tests pass", is_custom: false },
                ProjectOption { label: 'B', text: "Feature works end-to-end", is_custom: false },
                ProjectOption { label: 'C', text: "No console errors", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
    ]
}

fn go_questions() -> Vec<ProjectQuestion> {
    vec![
        ProjectQuestion {
            category: "Artifact",
            question: "What type of Go artifact is this?",
            options: vec![
                ProjectOption { label: 'A', text: "Library (importable package)", is_custom: false },
                ProjectOption { label: 'B', text: "CLI tool (main package)", is_custom: false },
                ProjectOption { label: 'C', text: "Web service (HTTP/gRPC)", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Target",
            question: "Who is the primary user?",
            options: vec![
                ProjectOption { label: 'A', text: "Go developers", is_custom: false },
                ProjectOption { label: 'B', text: "CLI users", is_custom: false },
                ProjectOption { label: 'C', text: "Service consumers (API)", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Success",
            question: "How will you know it's complete?",
            options: vec![
                ProjectOption { label: 'A', text: "go test passes", is_custom: false },
                ProjectOption { label: 'B', text: "golangci-lint clean", is_custom: false },
                ProjectOption { label: 'C', text: "Benchmarks meet target", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
    ]
}

fn generic_questions() -> Vec<ProjectQuestion> {
    vec![
        ProjectQuestion {
            category: "Problem",
            question: "What problem does this solve?",
            options: vec![
                ProjectOption { label: 'A', text: "Performance issue", is_custom: false },
                ProjectOption { label: 'B', text: "Missing functionality", is_custom: false },
                ProjectOption { label: 'C', text: "Developer experience", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Target",
            question: "Who is the primary user?",
            options: vec![
                ProjectOption { label: 'A', text: "Developer", is_custom: false },
                ProjectOption { label: 'B', text: "End-user", is_custom: false },
                ProjectOption { label: 'C', text: "Admin/Operator", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
        ProjectQuestion {
            category: "Success",
            question: "How will you know it's complete?",
            options: vec![
                ProjectOption { label: 'A', text: "Tests pass", is_custom: false },
                ProjectOption { label: 'B', text: "Feature works end-to-end", is_custom: false },
                ProjectOption { label: 'C', text: "Performance target met", is_custom: false },
                ProjectOption { label: 'D', text: "Custom...", is_custom: true },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Rust);
    }

    #[test]
    fn test_detect_python_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "[project]").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Python);
    }

    #[test]
    fn test_detect_go_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("go.mod"), "module example").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Go);
    }

    #[test]
    fn test_detect_typescript_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), r#"{"devDependencies":{"typescript":"5.0"}}"#).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::TypeScript);
    }

    #[test]
    fn test_detect_typescript_by_tsconfig() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::TypeScript);
    }

    #[test]
    fn test_detect_javascript_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::JavaScript);
    }

    #[test]
    fn test_detect_generic_project() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Generic);
    }

    #[test]
    fn test_project_questions_count() {
        // All project types should have exactly 3 questions
        assert_eq!(get_project_questions(ProjectType::Rust).len(), 3);
        assert_eq!(get_project_questions(ProjectType::Python).len(), 3);
        assert_eq!(get_project_questions(ProjectType::TypeScript).len(), 3);
        assert_eq!(get_project_questions(ProjectType::JavaScript).len(), 3);
        assert_eq!(get_project_questions(ProjectType::Go).len(), 3);
        assert_eq!(get_project_questions(ProjectType::Generic).len(), 3);
    }

    #[test]
    fn test_rust_priority_over_python() {
        // A project with both Cargo.toml and requirements.txt should be detected as Rust
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(dir.path().join("requirements.txt"), "requests").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Rust);
    }
}
