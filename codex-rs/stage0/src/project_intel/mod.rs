//! Project Intel Module for SPEC-KIT
//!
//! SPEC-KIT-2XX: Gathers intense project details for NotebookLM synthesis.
//! Produces structured snapshots and markdown feeds for NL_* doc generation.
//!
//! Pipeline:
//! 1. snapshot - Gather project details → JSON + markdown feeds
//! 2. curate-nl - Compress feeds into canonical NL_* docs
//! 3. sync-nl - Push NL_* docs to NotebookLM
//! 4. overview - Query NotebookLM for global mental model

pub mod snapshot;
pub mod types;

pub use snapshot::{load_governance_from_db, ProjectSnapshotBuilder, SnapshotConfig};
pub use types::*;
