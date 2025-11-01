//! Spec-Kit multi-agent automation framework
//!
//! This module isolates all spec-kit functionality from upstream TUI code
//! to minimize rebase conflict surface area.
//!
//! Uses free functions instead of methods to avoid Rust borrow checker issues
//! when accessing ChatWidget fields.

pub mod ace_client; // ACE (Agentic Context Engine) MCP client
pub mod ace_constitution; // ACE constitution pinning
pub mod ace_curator; // ACE Curator - Strategic playbook management
pub mod ace_learning; // ACE learning from execution outcomes
pub mod ace_orchestrator; // ACE Orchestrator - Full reflection-curation cycle
pub mod ace_prompt_injector; // ACE prompt injection logic
pub mod ace_reflector; // ACE Reflector - Deep outcome analysis
pub mod ace_route_selector; // ACE route selection for complex tasks
pub mod agent_orchestrator; // Agent orchestration functions (extracted from handler.rs)
pub mod analyze_native; // Native consistency checking (zero agents, zero cost)
pub mod checklist_native; // Native quality scoring (zero agents, zero cost)
pub mod clarify_native; // Native ambiguity detection (zero agents, zero cost)
pub mod command_handlers; // Command entry points (status, consensus, guardrail)
pub mod command_registry;
pub mod commands;
pub mod config_validator;
pub mod new_native; // SPEC-KIT-072: Native SPEC creation (eliminates 2 agents, $0.15 → $0)
pub mod consensus;
pub mod consensus_coordinator; // Consensus checking with MCP retry logic
pub mod context;
pub mod cost_tracker; // SPEC-KIT-070: Cost tracking and budget management
pub mod error;
pub mod evidence;
pub mod file_modifier;
pub mod guardrail;
pub mod handler;
pub mod native_consensus_executor; // Native consensus orchestration (eliminates orchestrator agent)
pub mod pipeline_coordinator; // MAINT-3 Phase 5: Pipeline state machine (extracted from handler.rs)
// FORK-SPECIFIC (just-every/code): local_memory_client.rs deleted 2025-10-18
// Replaced by native MCP integration in consensus.rs
pub mod quality;
pub mod quality_gate_broker;
pub mod quality_gate_handler; // MAINT-2: Extracted from handler.rs (925 LOC)
pub mod routing;
pub mod schemas;
pub mod spec_id_generator; // SPEC-KIT-070: Native SPEC-ID generation (cost optimization)
pub mod state;
pub mod subagent_defaults;
pub mod validation_lifecycle; // Validation lifecycle tracking and telemetry

// Re-export context types
pub use context::SpecKitContext;

// MAINT-3 Phase 2: Re-export test utilities
#[cfg(any(test, feature = "test-utils"))]
pub use context::test_mock::MockSpecKitContext;

// Re-export error types
pub(crate) use error::Result;

// Re-export evidence types

// Re-export key consensus functions (pub(crate) since types are private)

// Re-export guardrail functions
pub use guardrail::{evaluate_guardrail_value, validate_guardrail_schema};

// Re-export routing functions
pub use routing::try_dispatch_spec_kit_command;

// Re-export state types and helpers
pub use state::{
    // Quality gate types (T85)
    Confidence,
    EscalatedQuestion,
    GuardrailOutcome,
    Magnitude,
    QualityCheckpoint,
    QualityGateType,
    QualityIssue,
    Resolution,
    Resolvability,
    SpecAutoState,
    spec_ops_stage_prefix,
    validate_guardrail_evidence,
};

// Re-export handler functions
pub use handler::{
    advance_spec_auto, auto_submit_spec_stage_prompt, halt_spec_auto_with_error, handle_guardrail,
    handle_spec_auto, handle_spec_consensus, handle_spec_status, on_quality_gate_answers,
    on_quality_gate_cancelled, on_spec_auto_agents_complete, on_spec_auto_task_complete,
    on_spec_auto_task_started,
};

// Re-export validation lifecycle functions
pub use validation_lifecycle::{
    cleanup_spec_auto_with_cancel, compute_validate_payload_hash,
    record_validate_lifecycle_event, ValidateLifecycleEvent, ValidateMode,
};

// Re-export quality gate functions
pub use quality::{
    classify_issue_agreement, merge_agent_issues, parse_quality_issue_from_agent,
    resolve_quality_issue, should_auto_resolve,
};

// Re-export ACE functions for integration testing and widget usage
#[cfg(any(test, feature = "test-utils"))]
pub use ace_prompt_injector::should_use_ace;
#[cfg(any(test, feature = "test-utils"))]
pub use ace_route_selector::{select_route, DiffStat, RouteDecision};

// Re-export broker handle for UI integration
pub(crate) use quality_gate_broker::{
    QualityGateBroker, QualityGateBrokerResult, QualityGateValidationResult,
};

// Re-export file modification functions
