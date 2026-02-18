//! PM-006: Packet Persistence Schema
//!
//! Defines the `packet.json` schema with atomic read/write and sacred
//! anchor immutability. The packet is the execution contract that binds
//! intent to milestones.
//!
//! ## Design (ADR-005, ADR-006, ADR-009, D122)
//!
//! - Packet is the single source of truth for execution state
//! - Sacred anchors (intent, success criteria) are immutable without
//!   explicit amendment workflow
//! - Atomic writes via temp-file + fsync + rename
//! - Identical read semantics for TUI/CLI/headless (D113 parity)

pub mod anchor_guard;
pub mod io;
pub mod schema;

pub use anchor_guard::AnchorGuardError;
pub use io::{PacketIoError, read_packet, write_packet};
pub use schema::{
    AmendmentRecord, ExecutionState, MilestoneContract, MilestoneState, Packet, PacketHeader,
    Phase, SacredAnchors,
};
