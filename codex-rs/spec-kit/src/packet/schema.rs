//! PM-006 T001: Packet schema types.
//!
//! The packet is the execution contract. It records what was agreed
//! (sacred anchors), what must be achieved (milestones), and where
//! execution currently stands (execution state).

use serde::{Deserialize, Serialize};

/// Schema version constant for forward compatibility.
pub const SCHEMA_VERSION: &str = "packet@1.0";

/// The top-level packet contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Packet {
    /// Packet metadata (version, identity, timestamps).
    pub header: PacketHeader,
    /// Immutable intent and success criteria.
    pub sacred_anchors: SacredAnchors,
    /// Milestone contracts defining class-2 gate boundaries.
    pub milestones: Vec<MilestoneContract>,
    /// Current execution state.
    pub execution_state: ExecutionState,
}

/// Packet header with versioning and identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PacketHeader {
    /// Schema version (e.g., "packet@1.0").
    pub schema_version: String,
    /// Unique packet identifier.
    pub packet_id: String,
    /// Monotonic epoch counter (incremented on each write).
    pub epoch: u32,
    /// RFC 3339 timestamp of initial creation.
    pub created_at: String,
    /// RFC 3339 timestamp of last modification.
    pub last_modified_at: String,
}

/// Sacred anchors: the immutable core of the execution contract.
///
/// These fields define WHAT was agreed to achieve and HOW success is
/// measured. They cannot be modified without an explicit amendment
/// workflow that records the reason and timestamp.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SacredAnchors {
    /// High-level intent summary (what this execution achieves).
    pub intent_summary: String,
    /// Measurable success criteria.
    pub success_criteria: Vec<String>,
    /// Amendment history (empty if never amended).
    #[serde(default)]
    pub amend_history: Vec<AmendmentRecord>,
}

/// Record of an amendment to sacred anchors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmendmentRecord {
    /// RFC 3339 timestamp of the amendment.
    pub amended_at: String,
    /// Reason for the amendment.
    pub reason: String,
    /// Which field was amended ("intent_summary" or "success_criteria").
    pub field: String,
    /// Previous value (serialized as JSON string for flexibility).
    pub previous_value: String,
}

/// A milestone contract within the packet.
///
/// Milestones define the deliverables that must be achieved. Class 2
/// gate enforcement checks milestone state before allowing progression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MilestoneContract {
    /// Human-readable milestone name.
    pub name: String,
    /// Whether this milestone is required for class-2 gate passage.
    pub required_for_class2: bool,
    /// Current milestone state.
    pub state: MilestoneState,
    /// Optional description of acceptance criteria for this milestone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_criteria: Option<String>,
}

/// Milestone completion state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneState {
    /// Not yet started.
    Pending,
    /// Work in progress.
    InProgress,
    /// Completed and verified.
    Done,
    /// Blocked by external dependency.
    Blocked,
    /// Explicitly skipped (with justification in milestone description).
    Skipped,
}

/// Current execution state within the packet lifecycle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionState {
    /// Current pipeline phase.
    pub phase: Phase,
    /// Milestones that have been completed (by name).
    #[serde(default)]
    pub completed_milestones: Vec<String>,
    /// Currently active milestone (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_milestone: Option<String>,
}

/// Pipeline phase for execution tracking.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Initial setup, anchors being defined.
    Initializing,
    /// Active execution.
    Executing,
    /// All milestones completed, awaiting final gate.
    Completing,
    /// Packet is sealed (no further modifications).
    Sealed,
}

impl Packet {
    /// Create a new packet with the given intent and success criteria.
    pub fn new(packet_id: String, intent_summary: String, success_criteria: Vec<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            header: PacketHeader {
                schema_version: SCHEMA_VERSION.to_string(),
                packet_id,
                epoch: 1,
                created_at: now.clone(),
                last_modified_at: now,
            },
            sacred_anchors: SacredAnchors {
                intent_summary,
                success_criteria,
                amend_history: Vec::new(),
            },
            milestones: Vec::new(),
            execution_state: ExecutionState {
                phase: Phase::Initializing,
                completed_milestones: Vec::new(),
                current_milestone: None,
            },
        }
    }

    /// Add a milestone to the packet.
    pub fn add_milestone(&mut self, name: String, required_for_class2: bool) {
        self.milestones.push(MilestoneContract {
            name,
            required_for_class2,
            state: MilestoneState::Pending,
            acceptance_criteria: None,
        });
    }
}
