//! Spec-Kit command implementations
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! Each command implements the SpecKitCommand trait and delegates to
//! existing handlers in ../handler.rs

mod cancel; // SPEC-DOGFOOD-001: Cancel stale pipeline state
mod capsule; // SPEC-KIT-971: Memvid capsule commands
mod configure; // SPEC-947 Phase 4: Pipeline configurator command
mod guardrail;
mod intel; // SPEC-KIT-2XX: Project Intel for NotebookLM
mod librarian; // SPEC-KIT-103: Librarian memory quality engine
mod msearch; // SPEC-KIT-972: Memory search with --explain
mod plan;
mod policy; // SPEC-KIT-977: Policy management commands
mod project; // SPEC-KIT-960: Project scaffolding command
mod projections; // WP-A: Filesystem projections rebuild
pub mod projectnew; // /speckit.projectnew - full project setup with vision + intake
mod quality;
mod reflex; // SPEC-KIT-978: Reflex local inference
pub mod search;
mod special;
mod stage_agents; // SPEC-KIT-983: Stage→agent defaults UI
mod status;
mod templates; // SPEC-KIT-962: Template management commands
mod timeline; // SPEC-KIT-973: Time-travel UI commands
pub mod verify;

// Re-export all commands
pub use cancel::*;
pub use capsule::*;
pub use configure::*;
pub use guardrail::*;
pub use intel::*;
pub use librarian::*;
pub use msearch::*;
pub use plan::*;
pub use policy::*;
pub use project::*;
pub use projections::*;
pub use projectnew::*;
pub use quality::*;
pub use reflex::*;
pub use search::*;
pub use special::*;
pub use stage_agents::*;
pub use status::*;
pub use templates::*;
pub use timeline::*;
pub use verify::*;
