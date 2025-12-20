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
pub mod agent_retry; // SPEC-938: Agent spawn retry logic with exponential backoff
pub mod analyze_native; // Native consistency checking (zero agents, zero cost)
pub mod checklist_native; // Native quality scoring (zero agents, zero cost)
pub mod clarify_handler; // SPEC-KIT-971: Clarify modal event handlers
pub mod clarify_native; // Native ambiguity detection (zero agents, zero cost)
pub mod code_index; // P85: Code unit extraction for Shadow Code Brain
pub mod command_handlers; // Command entry points (status, consensus, guardrail)
pub mod command_registry;
pub mod commands;
pub mod config_reload; // SPEC-945D Phase 2.2: Config hot-reload handler
pub mod config_validator;
pub mod consensus_coordinator; // Consensus checking with MCP retry logic
pub mod consensus_db; // SPEC-KIT-072: SQLite storage for consensus artifacts (replaces local-memory)
pub mod context;
pub mod cost_tracker; // SPEC-KIT-070: Cost tracking and budget management
pub mod error;
pub mod evidence;
pub mod evidence_cleanup; // SPEC-933 Component 4: Automated evidence cleanup (archive/purge)
pub mod execution_logger; // SPEC-KIT-070: End-to-end execution visibility
pub mod file_modifier;
pub mod gate_evaluation; // Gate evaluation (renamed from consensus - PR4)
pub mod git_integration; // SPEC-KIT-922: Auto-commit stage artifacts
pub mod guardrail;
pub mod handler;
pub mod isolation_validator; // SPEC-KIT-964 Phase 6: Hermetic isolation validation for multi-agent spawning
pub mod json_extractor; // SPEC-KIT-927: Industrial-strength JSON extraction from LLM outputs
pub mod native_consensus_executor; // Native consensus orchestration (eliminates orchestrator agent)
pub mod native_guardrail; // SPEC-KIT-066, SPEC-KIT-902: Native guardrail validation (replaces bash scripts)
pub mod native_quality_gate_orchestrator; // SPEC-KIT-900, I-003: Native quality gate orchestration (eliminates LLM plumbing)
pub mod new_native; // SPEC-KIT-072: Native SPEC creation (eliminates 2 agents, $0.15 → $0)
pub mod pipeline_config; // SPEC-948: Modular pipeline logic - stage filtering and configuration
pub mod pipeline_configurator; // SPEC-947: Pipeline UI configurator - interactive stage selection
pub mod pipeline_coordinator;
pub mod prd_builder_handler; // SPEC-KIT-970: PRD builder modal event handlers
pub mod project_detector; // SPEC-KIT-971: Project type detection for context-aware questions
pub mod project_native; // SPEC-KIT-960: Native project scaffolding
pub mod spec_directory;
pub mod stage0_integration; // SPEC-KIT-102: Stage 0 context injection for /speckit.auto
pub mod stage0_seeding;
pub mod stage_details; // SPEC-947 Phase 3: Stage details widget (right pane)
pub mod stage_selector;
pub mod vision_builder_handler; // P93/SPEC-KIT-105: Vision builder modal event handlers // SPEC-947 Phase 3: Stage selector widget (checkbox list) // SPEC-KIT-900 Session 3: ACID-compliant SPEC directory resolution // MAINT-3 Phase 5: Pipeline state machine (extracted from handler.rs) // SPEC-KIT-102: Shadow Notebook Seeder for NotebookLM
// FORK-SPECIFIC (just-every/code): local_memory_client.rs deleted 2025-10-18
// Replaced by native MCP integration in consensus.rs
pub mod quality;
pub mod quality_gate_broker;
pub mod quality_gate_handler; // MAINT-2: Extracted from handler.rs (925 LOC)
pub mod routing;
pub mod schemas;
pub mod session_metrics; // P6-SYNC Phase 2: Token usage tracking with sliding window estimation
pub mod spawn_metrics; // SPEC-933 Component 3: Agent spawn performance tracking
pub mod spec_id_generator; // SPEC-KIT-070: Native SPEC-ID generation (cost optimization)
pub mod state;
pub mod subagent_defaults;
pub mod validation_lifecycle; // Validation lifecycle tracking and telemetry

// Re-export context types
pub(crate) use context::SpecKitContext;

// MAINT-3 Phase 2: Re-export test utilities

// Re-export error types
pub(crate) use error::Result;

// Re-export evidence types

// Re-export key consensus functions (pub(crate) since types are private)

// Re-export guardrail functions
pub use guardrail::{
    display_guardrail_result_and_advance, evaluate_guardrail_value, validate_guardrail_schema,
};

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
pub(crate) use handler::{
    advance_spec_auto, auto_submit_spec_stage_prompt, halt_spec_auto_with_error, handle_guardrail,
    handle_spec_auto, handle_spec_consensus, handle_spec_status, on_quality_gate_agents_complete,
    on_quality_gate_answers, on_quality_gate_cancelled, on_spec_auto_agents_complete,
    on_spec_auto_task_complete, on_spec_auto_task_started,
};

// Re-export PRD builder handler functions (SPEC-KIT-970)
pub(crate) use prd_builder_handler::{on_prd_builder_cancelled, on_prd_builder_submitted};

// Re-export clarify handler functions (SPEC-KIT-971)
pub(crate) use clarify_handler::{on_clarify_cancelled, on_clarify_submitted};

// Re-export vision builder handler functions (P93/SPEC-KIT-105)
pub(crate) use vision_builder_handler::{on_vision_builder_cancelled, on_vision_builder_submitted};

// Re-export pipeline configuration types (SPEC-948)
pub use pipeline_config::PipelineOverrides;

// Re-export agent orchestrator functions
pub use agent_orchestrator::{
    on_spec_auto_agents_complete_with_ids, on_spec_auto_agents_complete_with_results,
};
pub use quality_gate_handler::set_native_agent_ids;

// Re-export validation lifecycle functions
pub use validation_lifecycle::{compute_validate_payload_hash, record_validate_lifecycle_event};

// Re-export quality gate functions
pub use quality::{
    classify_issue_agreement, merge_agent_issues, parse_quality_issue_from_agent,
    resolve_quality_issue, should_auto_resolve,
};

// Re-export ACE functions for integration testing and widget usage
#[cfg(any(test, feature = "test-utils"))]
pub use ace_prompt_injector::should_use_ace;
#[cfg(any(test, feature = "test-utils"))]
pub use ace_route_selector::{DiffStat, RouteDecision, select_route};

// Re-export broker handle for UI integration
pub(crate) use quality_gate_broker::{
    QualityGateBroker, QualityGateBrokerResult, QualityGateValidationResult,
};

// Re-export file modification functions
