//! SPEC-KIT-971: Memvid Capsule Foundation
//! SPEC-KIT-977: PolicySnapshot Integration
//!
//! This module implements the Memvid adapter for Stage0 memory traits.
//! All Memvid concepts are isolated here; Stage0 core has no Memvid dependency.
//!
//! ## Decision IDs Implemented
//! - D1: Workspace capsule + per-run exports
//! - D2: Canonical capsule path conventions
//! - D7: Single-writer capsule model
//! - D18: Stage boundary checkpoints
//! - D70: Stable mv2:// URI scheme
//! - D100-D102: PolicySnapshot capture and storage
//!
//! ## Key Invariants (from SPEC.md Docs Contract)
//! - Logical URIs are immutable once returned
//! - Single-writer: global lock + writer queue
//! - Stage boundary commits create checkpoints
//! - All cross-object references use logical URIs (never raw frame IDs)

use std::path::{Path, PathBuf};

// =============================================================================
// Canonical Capsule Configuration (SPEC-KIT-971/977 alignment)
// =============================================================================

/// Canonical relative path for the workspace capsule.
///
/// All pipeline, TUI, and CLI operations MUST use this path to ensure
/// a single capsule location.
pub const DEFAULT_CAPSULE_RELATIVE_PATH: &str = ".speckit/memvid/workspace.mv2";

/// Canonical workspace ID for minting mv2:// URIs.
///
/// All write operations that mint URIs (policy snapshots, events, artifacts)
/// MUST use this workspace ID to ensure URI consistency.
pub const DEFAULT_WORKSPACE_ID: &str = "default";

mod adapter;
mod capsule;
pub mod eval;
pub mod lock;
pub mod policy_capture;
mod types;

pub use adapter::{
    MemvidMemoryAdapter, MemoryMeta, UnifiedMemoryClient,
    create_memory_client, create_unified_memory_client,
};
pub use eval::{
    ABHarness, ABReport, EvalRunResult, GoldenQuery,
    golden_query_suite, golden_test_memories,
    run_ab_harness_and_save, run_ab_harness_synthetic,
};
pub use capsule::{
    CapsuleHandle, CapsuleConfig, CapsuleError, CapsuleOpenOptions,
    CapsuleStats, DiagnosticResult, IndexStatus, CurrentPolicyInfo,
};
pub use lock::{LockMetadata, is_locked, lock_path_for};
pub use types::{
    LogicalUri, CheckpointId, CheckpointMetadata, BranchId,
    RunEventEnvelope, EventType, UriIndex, UriIndexSnapshot,
    // SPEC-KIT-978: Routing decision types
    RoutingMode, RoutingFallbackReason, RoutingDecisionPayload,
    // SPEC-KIT-971: Branch merge types
    MergeMode, BranchMergedPayload,
};
pub use policy_capture::{
    capture_and_store_policy, load_policy, list_policies, latest_policy,
};

// =============================================================================
// Capsule Configuration Helpers
// =============================================================================

/// Get the canonical capsule path for a given working directory.
pub fn default_capsule_path(cwd: &Path) -> PathBuf {
    cwd.join(DEFAULT_CAPSULE_RELATIVE_PATH)
}

/// Get the canonical capsule configuration for a given working directory.
///
/// Use this for all capsule operations to ensure consistent paths and
/// workspace IDs across TUI, CLI, and pipeline.
pub fn default_capsule_config(cwd: &Path) -> CapsuleConfig {
    CapsuleConfig {
        capsule_path: default_capsule_path(cwd),
        workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
