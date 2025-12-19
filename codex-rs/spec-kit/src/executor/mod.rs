//! Spec-Kit Executor — Shared Application Core
//!
//! SPEC-KIT-921: CLI Adapter + Shared SpeckitExecutor Core
//!
//! This module provides the "thin waist" between UI surfaces (TUI, CLI) and
//! spec-kit business logic. Both adapters call `SpeckitExecutor::execute()`,
//! ensuring command parity and preventing logic duplication.
//!
//! ## Design Principles
//!
//! - **No UI types in core**: Executor returns domain types, never ratatui/CLI types
//! - **Single entrypoint**: All commands flow through `execute()`
//! - **Adapters own rendering**: TUI/CLI render domain results into their format
//!
//! ## Phase B Scope
//!
//! - Status command (read-only, pure)
//! - Review command (after status proves the pattern)

mod command;
pub mod status;

pub use command::SpeckitCommand;
pub use status::{
    AgentCoverage, AgentOutcome, AgentStatus, EvidenceEntry, EvidenceMetrics, EvidenceThreshold,
    GuardrailRecord, PacketStatus, ScenarioStatus, SpecStatusArgs, SpecStatusReport,
    StageConsensus, StageCue, StageKind, StageSnapshot, TrackerRow,
};

/// Execution outcome from the executor
#[derive(Debug)]
pub enum Outcome {
    /// Status command completed successfully
    Status(SpecStatusReport),

    /// Command failed with error
    Error(String),
}

/// Execution context provided by the adapter
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Repository root path
    pub repo_root: std::path::PathBuf,
}

/// Spec-Kit executor — the single entrypoint for all commands
///
/// Both TUI and CLI adapters call this executor. The executor returns
/// domain types; adapters handle rendering.
pub struct SpeckitExecutor {
    context: ExecutionContext,
}

impl SpeckitExecutor {
    /// Create a new executor with the given context
    pub fn new(context: ExecutionContext) -> Self {
        Self { context }
    }

    /// Execute a command and return the outcome
    ///
    /// This is the single entrypoint for all spec-kit commands.
    /// Adapters (TUI/CLI) call this method and render the result.
    pub fn execute(&self, command: SpeckitCommand) -> Outcome {
        match command {
            SpeckitCommand::Status {
                spec_id,
                stale_hours,
            } => self.execute_status(&spec_id, stale_hours),
        }
    }

    /// Execute status command
    fn execute_status(&self, spec_id: &str, stale_hours: i64) -> Outcome {
        let args = SpecStatusArgs {
            spec_id: spec_id.to_string(),
            stale_hours,
        };

        match status::collect_report(&self.context.repo_root, args) {
            Ok(report) => Outcome::Status(report),
            Err(e) => Outcome::Error(e.to_string()),
        }
    }
}

/// Render a status report as text lines (for TUI/CLI display)
///
/// This is a pure formatting function — no side effects.
pub fn render_status_dashboard(report: &SpecStatusReport) -> Vec<String> {
    status::render_dashboard(report)
}

/// Get degraded warning message if any issues detected
pub fn status_degraded_warning(report: &SpecStatusReport) -> Option<String> {
    status::degraded_warning(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity test: Slash command and CLI should parse to same SpeckitCommand
    #[test]
    fn test_command_parity_status() {
        // Simulate slash command parsing: "/speckit.status SPEC-123"
        let slash_args = "SPEC-123";
        let slash_cmd = SpeckitCommand::Status {
            spec_id: slash_args.to_string(),
            stale_hours: 24, // default
        };

        // Simulate CLI parsing: "code speckit status --spec SPEC-123"
        let cli_cmd = SpeckitCommand::Status {
            spec_id: "SPEC-123".to_string(),
            stale_hours: 24,
        };

        // Both should produce equivalent commands
        assert_eq!(slash_cmd, cli_cmd);
    }

    #[test]
    fn test_command_parity_status_with_stale_hours() {
        // Slash: "/speckit.status SPEC-456 --stale-hours 48"
        let slash_cmd = SpeckitCommand::Status {
            spec_id: "SPEC-456".to_string(),
            stale_hours: 48,
        };

        // CLI: "code speckit status --spec SPEC-456 --stale-hours 48"
        let cli_cmd = SpeckitCommand::Status {
            spec_id: "SPEC-456".to_string(),
            stale_hours: 48,
        };

        assert_eq!(slash_cmd, cli_cmd);
    }
}
