//! Spec-Kit Multi-Agent Automation Framework
//!
//! FORK-SPECIFIC (just-every/code): Extracted from TUI (MAINT-10)
//!
//! Single-owner, stage-based pipeline with deterministic quality gates.
//! Each stage has one owner (a Role implemented by a Worker). Optional
//! sidecars emit signals but never produce competing answers.
//!
//! **Design Principle**: No voting, no committee synthesis, no debate loops.
//! See `docs/spec-kit/GATE_POLICY.md` for full specification.
//!
//! ## Core Concepts
//!
//! - **Stage**: Pipeline step (Specify→Plan→Tasks→Implement→Validate→Audit→Unlock)
//! - **Role**: Responsibility in workflow (Architect, Implementer, Validator, Judge)
//! - **Worker**: Runtime implementation of a Role (model/provider + permissions)
//! - **Gate**: Deterministic decision point at stage boundaries
//! - **Signal**: Typed input to a gate (owner confidence, tool truth, counter-signals)
//!
//! ## Module Organization
//!
//! - [`gate_policy`]: Domain vocabulary and gate evaluation contracts
//! - [`router`]: Role → Worker implementation mapping
//! - [`config`]: Configuration management (layered config, hot-reload)
//! - [`error`]: Error types and result aliases
//! - [`retry`]: Retry logic (backoff, error classification)
//! - [`timing`]: Performance timing infrastructure
//! - [`types`]: Legacy types (SpecStage, SpecAgent) for migration compatibility

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod config; // SPEC-945D: Configuration management (layered config, hot-reload)
pub mod error;
#[cfg(feature = "dev-faults")]
pub mod faults; // P6-SYNC Phase 3: Fault injection for testing error handling
pub mod gate_policy; // PR1: Gate Policy domain types and contracts
pub mod retry; // SPEC-945C: Retry logic (backoff, error classification)
pub mod router; // PR1: Router trait and WorkerSpec
pub mod timing; // SPEC-940: Performance timing infrastructure
pub mod types;

// Phase 1: Core types and error handling
pub use error::{Result, SpecKitError};
pub use types::{SpecAgent, SpecStage};

// PR1: Gate Policy canonical vocabulary
pub use gate_policy::{
    Checkpoint, ConfidenceLevel, CounterSignal, CounterSignalKind, DecisionRule, EscalationTarget,
    GateContext, GateVerdict, PolicyToggles, Role, RoleAssignment, RiskLevel, Signal,
    SignalSeverity, Stage, StageContext, ToolTruth, ToolTruthKind, Verdict,
};

// PR1: Router interface
pub use router::{Budget, DefaultRouter, Router, RoutingContext, ToolPermissions, WorkerKind, WorkerSpec};

// SPEC-940: Re-export timing macros for convenience
pub use timing::Timer;

/// Spec-Kit version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Evidence base directory (centralized)
pub const DEFAULT_EVIDENCE_BASE: &str = "docs/SPEC-OPS-004-integrated-coder-hooks/evidence";

/// Build consensus directory path
pub fn consensus_dir(cwd: &std::path::Path) -> std::path::PathBuf {
    cwd.join(DEFAULT_EVIDENCE_BASE).join("consensus")
}

/// Build commands directory path
pub fn commands_dir(cwd: &std::path::Path) -> std::path::PathBuf {
    cwd.join(DEFAULT_EVIDENCE_BASE).join("commands")
}
