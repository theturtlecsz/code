//! Common test utilities for spec-kit
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit test infrastructure (MAINT-3, Phase 3)

pub mod integration_harness;
pub mod mock_mcp;

pub use integration_harness::{EvidenceVerifier, IntegrationTestContext, StateBuilder};
pub use mock_mcp::MockMcpManager;
