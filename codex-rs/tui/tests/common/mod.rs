//! Common test utilities for spec-kit
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit test infrastructure (MAINT-3, Phase 3)

// Test utilities - allow dead code for unused test helpers
#![allow(dead_code)]
// Test utilities use expect/unwrap for simplicity
#![allow(clippy::expect_used, clippy::unwrap_used)]

pub mod integration_harness;
pub mod mock_mcp;

#[allow(unused_imports)]
pub use integration_harness::{EvidenceVerifier, IntegrationTestContext, StateBuilder};
#[allow(unused_imports)]
pub use mock_mcp::MockMcpManager;
