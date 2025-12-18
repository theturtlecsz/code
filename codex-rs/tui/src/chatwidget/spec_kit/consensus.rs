//! Deprecated shim module for backward compatibility.
//!
//! This module re-exports all items from `gate_evaluation` to maintain
//! backward compatibility during the vocabulary migration.
//!
//! **DEPRECATED**: Use `gate_evaluation` module directly.
//!
//! This shim will be removed after PR6 completes the legacy voting deletion.

// Allow unused since these are re-exports for external callers who haven't migrated yet
#[allow(unused_imports, deprecated)]
#[deprecated(
    since = "0.1.0",
    note = "Module renamed to `gate_evaluation`. Update imports to use `gate_evaluation` instead."
)]
pub use super::gate_evaluation::*;
