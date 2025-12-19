//! Spec Status Dashboard — TUI Re-exports
//!
//! **SPEC-KIT-921**: The actual logic has moved to `codex_spec_kit::executor::status`.
//! This module re-exports the types and functions for backward compatibility with
//! existing TUI code and tests.
//!
//! ## Migration Note
//!
//! New code should import directly from `codex_spec_kit::executor`:
//! ```ignore
//! use codex_spec_kit::executor::{
//!     SpeckitCommand, SpeckitExecutor, ExecutionContext, Outcome,
//!     render_status_dashboard, status_degraded_warning,
//! };
//! ```

// Re-export all public types from the executor's status module
pub use codex_spec_kit::executor::{
    // Report types
    AgentCoverage,
    AgentOutcome,
    AgentStatus,
    EvidenceEntry,
    EvidenceMetrics,
    EvidenceThreshold,
    GuardrailRecord,
    PacketStatus,
    ScenarioStatus,
    SpecStatusArgs,
    SpecStatusReport,
    StageConsensus,
    StageCue,
    StageKind,
    StageSnapshot,
    TrackerRow,
};

// Re-export the core functions with their original names for backward compatibility
pub use codex_spec_kit::executor::status::{collect_report, degraded_warning, render_dashboard};
