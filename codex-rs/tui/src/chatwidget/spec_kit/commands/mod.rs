//! Spec-Kit command implementations
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! Each command implements the SpecKitCommand trait and delegates to
//! existing handlers in ../handler.rs

mod configure; // SPEC-947 Phase 4: Pipeline configurator command
mod guardrail;
mod librarian; // SPEC-KIT-103: Librarian memory quality engine
mod plan;
mod project; // SPEC-KIT-960: Project scaffolding command
mod quality;
pub mod search;
mod special;
mod status;
mod templates; // SPEC-KIT-962: Template management commands
pub mod verify;

// Re-export all commands
pub use configure::*;
pub use guardrail::*;
pub use librarian::*;
pub use plan::*;
pub use project::*;
pub use quality::*;
pub use search::*;
pub use special::*;
pub use status::*;
pub use templates::*;
pub use verify::*;
