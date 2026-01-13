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
    RunEventEnvelope, EventType, UriIndex,
};
pub use policy_capture::{
    capture_and_store_policy, load_policy, list_policies, latest_policy,
};

#[cfg(test)]
mod tests;
