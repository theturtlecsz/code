//! PM-005: Change Classifier (Gatekeeper)
//!
//! Deterministic Class 0/1/2/E change classification based on file-type
//! heuristics and metadata signals. Pure classifier with no dependency
//! on packet state.
//!
//! ## Design (ADR-007, ADR-009, ADR-012)
//!
//! - Shared classifier used by TUI/CLI/headless per D113 (tiered parity)
//! - No voting or committee synthesis; classification is deterministic
//! - Emergency class requires explicit evidence + snapshot

pub mod classifier;

pub use classifier::{
    ChangeClass, ChangeMetadata, ClassificationResult, ClassifierError, classify_change,
};
