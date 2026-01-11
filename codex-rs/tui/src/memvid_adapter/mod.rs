//! SPEC-KIT-971: Memvid Capsule Foundation
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
//!
//! ## Key Invariants (from SPEC.md Docs Contract)
//! - Logical URIs are immutable once returned
//! - Single-writer: global lock + writer queue
//! - Stage boundary commits create checkpoints
//! - All cross-object references use logical URIs (never raw frame IDs)

mod adapter;
mod capsule;
mod types;

pub use adapter::MemvidMemoryAdapter;
pub use capsule::{
    CapsuleHandle, CapsuleConfig, CapsuleError,
    CapsuleStats, DiagnosticResult, IndexStatus,
};
pub use types::{
    LogicalUri, CheckpointId, CheckpointMetadata, BranchId,
    RunEventEnvelope, EventType, UriIndex,
};

#[cfg(test)]
mod tests;
