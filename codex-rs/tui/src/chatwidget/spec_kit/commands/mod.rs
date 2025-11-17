//! Spec-Kit command implementations
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! Each command implements the SpecKitCommand trait and delegates to
//! existing handlers in ../handler.rs

mod configure; // SPEC-947 Phase 4: Pipeline configurator command
mod guardrail;
mod plan;
mod quality;
mod special;
mod status;
pub mod verify;

// Re-export all commands
pub use configure::*;
pub use guardrail::*;
pub use plan::*;
pub use quality::*;
pub use special::*;
pub use status::*;
pub use verify::*;
