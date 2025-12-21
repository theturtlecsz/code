//! Spec-Kit CLI Test Helpers
//!
//! SPEC-KIT-921 P6-D: Shared test helpers to reduce boilerplate.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use speckit_helpers::*;
//!
//! #[test]
//! fn my_test() -> Result<()> {
//!     let ctx = TestContext::new()?;
//!     ctx.setup_spec_dir("SPEC-TEST-001", &[("plan.md", "# Plan")])?;
//!
//!     let result = ctx.run_cli(&["speckit", "tasks", "--spec", "SPEC-TEST-001", "--json"])?;
//!
//!     result.assert_success();
//!     result.assert_schema_version(1);
//!     result.assert_stage("Tasks");
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use tempfile::TempDir;

/// Test context with isolated CODEX_HOME and repo root
pub struct TestContext {
    pub codex_home: TempDir,
    pub repo_root: TempDir,
}

impl TestContext {
    /// Create a new test context with fresh temp directories
    pub fn new() -> Result<Self> {
        Ok(Self {
            codex_home: TempDir::new()?,
            repo_root: TempDir::new()?,
        })
    }

    /// Get the repo root path
    pub fn repo_path(&self) -> &Path {
        self.repo_root.path()
    }

    /// Create a SPEC directory with optional files
    ///
    /// # Arguments
    /// * `spec_id` - The SPEC identifier (e.g., "SPEC-TEST-001")
    /// * `files` - Slice of (filename, content) tuples to create in the SPEC dir
    ///
    /// # Returns
    /// Path to the created SPEC directory
    ///
    /// # Example
    /// ```rust,ignore
    /// ctx.setup_spec_dir("SPEC-001", &[
    ///     ("plan.md", "# Plan\nTest plan."),
    ///     ("tasks.md", "# Tasks\n- Task 1"),
    /// ])?;
    /// ```
    pub fn setup_spec_dir(&self, spec_id: &str, files: &[(&str, &str)]) -> Result<PathBuf> {
        let spec_dir = self.repo_root.path().join("docs").join(spec_id);
        fs::create_dir_all(&spec_dir)?;

        for (filename, content) in files {
            fs::write(spec_dir.join(filename), content)?;
        }

        Ok(spec_dir)
    }

    /// Create evidence directory structure for a SPEC
    ///
    /// Uses the hardcoded path: docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/{spec_id}
    pub fn setup_evidence_dir(&self, spec_id: &str) -> Result<PathBuf> {
        let evidence_dir = self
            .repo_root
            .path()
            .join("docs")
            .join("SPEC-OPS-004-integrated-coder-hooks")
            .join("evidence")
            .join("consensus")
            .join(spec_id);
        fs::create_dir_all(&evidence_dir)?;
        Ok(evidence_dir)
    }

    /// Create a consensus JSON file with specified conflicts
    pub fn create_consensus_file(
        &self,
        dir: &Path,
        stage: &str,
        agent: &str,
        timestamp: &str,
        conflicts: &[&str],
        synthesis_status: &str,
    ) -> Result<()> {
        let filename = format!("spec-{stage}_{agent}_{timestamp}.json");
        let conflicts_json: Vec<&str> = conflicts.to_vec();

        let content = serde_json::json!({
            "agent": agent,
            "model": "test-model",
            "consensus": {
                "conflicts": conflicts_json,
                "synthesis_status": synthesis_status
            }
        });

        fs::write(dir.join(&filename), serde_json::to_string_pretty(&content)?)?;
        Ok(())
    }

    /// Run a CLI command and return the result wrapper
    pub fn run_cli(&self, args: &[&str]) -> Result<CliResult> {
        let mut cmd = assert_cmd::Command::cargo_bin("code")?;
        cmd.env("CODEX_HOME", self.codex_home.path());
        cmd.current_dir(self.repo_root.path());
        cmd.args(args);

        let output = cmd.output()?;
        Ok(CliResult { output })
    }
}

/// Result wrapper for CLI command output with assertion helpers
pub struct CliResult {
    pub output: Output,
}

impl CliResult {
    /// Assert the command succeeded (exit 0)
    pub fn assert_success(&self) {
        assert!(
            self.output.status.success(),
            "Expected success, got {:?}\nstderr: {}",
            self.output.status.code(),
            String::from_utf8_lossy(&self.output.stderr)
        );
    }

    /// Assert the command exited with a specific code
    pub fn assert_exit_code(&self, expected: i32) {
        assert_eq!(
            self.output.status.code(),
            Some(expected),
            "Expected exit {expected}, got {:?}\nstderr: {}",
            self.output.status.code(),
            String::from_utf8_lossy(&self.output.stderr)
        );
    }

    /// Parse stdout as JSON
    pub fn json(&self) -> Result<JsonValue> {
        Ok(serde_json::from_slice(&self.output.stdout)?)
    }

    /// Get stdout as string
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    /// Get stderr as string
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }

    /// Assert JSON output has expected schema_version
    pub fn assert_schema_version(&self, expected: u64) -> &Self {
        let json = self.json().expect("Failed to parse JSON");
        let version = json.get("schema_version").and_then(|v| v.as_u64());
        assert_eq!(
            version,
            Some(expected),
            "Expected schema_version {expected}, got {:?}",
            version
        );
        self
    }

    /// Assert JSON output has expected stage containing the string
    pub fn assert_stage(&self, expected: &str) -> &Self {
        let json = self.json().expect("Failed to parse JSON");
        let stage = json.get("stage").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            stage.contains(expected),
            "Expected stage containing '{}', got '{}'",
            expected,
            stage
        );
        self
    }

    /// Assert JSON output has expected status
    pub fn assert_status(&self, expected: &str) -> &Self {
        let json = self.json().expect("Failed to parse JSON");
        let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(
            status, expected,
            "Expected status '{}', got '{}'",
            expected, status
        );
        self
    }

    /// Assert JSON has tool_version field
    pub fn assert_has_tool_version(&self) -> &Self {
        let json = self.json().expect("Failed to parse JSON");
        assert!(
            json.get("tool_version").is_some(),
            "Missing tool_version in JSON"
        );
        self
    }

    /// Assert JSON has warnings array with at least one entry
    pub fn assert_has_warnings(&self) -> &Self {
        let json = self.json().expect("Failed to parse JSON");
        let warnings = json.get("warnings").and_then(|v| v.as_array());
        assert!(
            warnings.map(|w| !w.is_empty()).unwrap_or(false),
            "Expected warnings array with entries"
        );
        self
    }

    /// Assert JSON has errors array with at least one entry
    pub fn assert_has_errors(&self) -> &Self {
        let json = self.json().expect("Failed to parse JSON");
        let errors = json.get("errors").and_then(|v| v.as_array());
        assert!(
            errors.map(|e| !e.is_empty()).unwrap_or(false),
            "Expected errors array with entries"
        );
        self
    }

    /// Assert JSON errors contain a specific string
    pub fn assert_error_contains(&self, needle: &str) -> &Self {
        let json = self.json().expect("Failed to parse JSON");
        let errors = json.get("errors").and_then(|v| v.as_array());
        let has_match = errors
            .map(|e| {
                e.iter().any(|err| {
                    err.as_str()
                        .map(|s| s.contains(needle))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        assert!(has_match, "Expected error containing '{}' in {:?}", needle, errors);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() -> Result<()> {
        let ctx = TestContext::new()?;
        assert!(ctx.repo_path().exists());
        Ok(())
    }

    #[test]
    fn test_setup_spec_dir() -> Result<()> {
        let ctx = TestContext::new()?;
        let spec_dir = ctx.setup_spec_dir("SPEC-TEST", &[("plan.md", "# Plan")])?;
        assert!(spec_dir.exists());
        assert!(spec_dir.join("plan.md").exists());
        Ok(())
    }

    #[test]
    fn test_run_cli_help() -> Result<()> {
        let ctx = TestContext::new()?;
        let result = ctx.run_cli(&["--help"])?;
        // --help should succeed
        result.assert_success();
        Ok(())
    }
}
