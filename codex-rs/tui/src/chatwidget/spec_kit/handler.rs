//! Spec-Kit command handlers as free functions
//!
//! These functions are extracted from chatwidget.rs to isolate spec-kit code.
//! Using free functions instead of methods to avoid Rust borrow checker issues.

// Re-export command handlers for backward compatibility
pub use super::command_handlers::{
    halt_spec_auto_with_error, handle_guardrail, handle_spec_consensus, handle_spec_status,
};

// Re-export consensus coordination for use by command_handlers
pub(crate) use super::consensus_coordinator::handle_spec_consensus_impl;

// Re-export agent orchestration functions
pub(crate) use super::agent_orchestrator::schedule_degraded_follow_up;
pub use super::agent_orchestrator::{
    auto_submit_spec_stage_prompt, on_spec_auto_agents_complete,
    on_spec_auto_agents_complete_with_ids, record_agent_costs,
};

// Re-export pipeline coordination functions (MAINT-3 Phase 5: Extracted to pipeline_coordinator.rs)
pub(crate) use super::pipeline_coordinator::check_consensus_and_advance_spec_auto;
pub use super::pipeline_coordinator::{
    advance_spec_auto, handle_spec_auto, on_spec_auto_task_complete, on_spec_auto_task_started,
};

// === Quality Gate Handlers ===
// MAINT-2: Extracted to quality_gate_handler.rs (925 LOC)
// Re-exported from mod.rs for backward compatibility

pub use super::quality_gate_handler::{
    on_quality_gate_agents_complete, on_quality_gate_answers, on_quality_gate_broker_result,
    on_quality_gate_cancelled, on_quality_gate_validation_result,
};
