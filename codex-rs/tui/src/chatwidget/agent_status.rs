//! Agent status types and display helpers.
//!
//! This module contains enums and functions for representing and displaying
//! agent status information in the TUI.
//!
//! Extracted from mod.rs as part of MAINT-11 to reduce cognitive load
//! and improve code organization.

use chrono::{DateTime, Local};
use ratatui::style::Color;

use crate::colors;

/// Lifecycle status of an agent task.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AgentStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Kind of log entry for agent activity tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentLogKind {
    Status,
    Progress,
    Result,
    Error,
}

/// A single log entry for agent activity.
#[derive(Debug, Clone)]
pub(crate) struct AgentLogEntry {
    pub timestamp: DateTime<Local>,
    pub kind: AgentLogKind,
    pub message: String,
}

/// Parse a status string into an `AgentStatus` enum value.
///
/// Unknown strings default to `Pending`.
pub(crate) fn agent_status_from_str(status: &str) -> AgentStatus {
    match status {
        "pending" => AgentStatus::Pending,
        "running" => AgentStatus::Running,
        "completed" => AgentStatus::Completed,
        "failed" => AgentStatus::Failed,
        _ => AgentStatus::Pending,
    }
}

/// Get the display label for an agent status.
pub(crate) fn agent_status_label(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Pending => "Pending",
        AgentStatus::Running => "Running",
        AgentStatus::Completed => "Completed",
        AgentStatus::Failed => "Failed",
    }
}

/// Get the display color for an agent status.
pub(crate) fn agent_status_color(status: AgentStatus) -> Color {
    match status {
        AgentStatus::Pending => colors::warning(),
        AgentStatus::Running => colors::info(),
        AgentStatus::Completed => colors::success(),
        AgentStatus::Failed => colors::error(),
    }
}

/// Get the display label for an agent log kind.
pub(crate) fn agent_log_label(kind: AgentLogKind) -> &'static str {
    match kind {
        AgentLogKind::Status => "status",
        AgentLogKind::Progress => "progress",
        AgentLogKind::Result => "result",
        AgentLogKind::Error => "error",
    }
}

/// Get the display color for an agent log kind.
pub(crate) fn agent_log_color(kind: AgentLogKind) -> Color {
    match kind {
        AgentLogKind::Status => colors::info(),
        AgentLogKind::Progress => colors::primary(),
        AgentLogKind::Result => colors::success(),
        AgentLogKind::Error => colors::error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_from_str() {
        assert_eq!(agent_status_from_str("pending"), AgentStatus::Pending);
        assert_eq!(agent_status_from_str("running"), AgentStatus::Running);
        assert_eq!(agent_status_from_str("completed"), AgentStatus::Completed);
        assert_eq!(agent_status_from_str("failed"), AgentStatus::Failed);
        // Unknown defaults to Pending
        assert_eq!(agent_status_from_str("unknown"), AgentStatus::Pending);
        assert_eq!(agent_status_from_str(""), AgentStatus::Pending);
    }

    #[test]
    fn test_agent_status_label() {
        assert_eq!(agent_status_label(AgentStatus::Pending), "Pending");
        assert_eq!(agent_status_label(AgentStatus::Running), "Running");
        assert_eq!(agent_status_label(AgentStatus::Completed), "Completed");
        assert_eq!(agent_status_label(AgentStatus::Failed), "Failed");
    }

    #[test]
    fn test_agent_log_label() {
        assert_eq!(agent_log_label(AgentLogKind::Status), "status");
        assert_eq!(agent_log_label(AgentLogKind::Progress), "progress");
        assert_eq!(agent_log_label(AgentLogKind::Result), "result");
        assert_eq!(agent_log_label(AgentLogKind::Error), "error");
    }
}
