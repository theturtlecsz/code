//! SPEC-KIT-971: Type-safe primitives for Memvid capsules
//!
//! ## Design Principle (from Architect feedback)
//! "Make it hard to do the wrong thing" - use type-safe wrappers, not String.
//! Graph edges, audit events, and export manifests can ONLY reference LogicalUri.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// =============================================================================
// LogicalUri - The stable addressing primitive (D70)
// =============================================================================

/// A stable, immutable logical URI for capsule objects.
///
/// ## URI Format
/// `mv2://<workspace>/<spec>/<run>/<type>/<path>`
///
/// ## Invariants (SPEC-KIT-971, URI invariants section)
/// 1. Logical URIs are immutable once returned
/// 2. Logical URIs are stable keys, not "frame IDs"
/// 3. All cross-object references use logical URIs
/// 4. Promotion/merge writes preserve the same logical URI
///
/// ## Type Safety
/// This is NOT a String. Graph edges must use `Edge { from: LogicalUri, to: LogicalUri }`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogicalUri(String);

impl LogicalUri {
    /// Create a new LogicalUri from components.
    ///
    /// Returns None if any component is invalid.
    pub fn new(
        workspace_id: &str,
        spec_id: &str,
        run_id: &str,
        object_type: ObjectType,
        path: &str,
    ) -> Option<Self> {
        // Validate components (no empty strings, no special chars that break URIs)
        if workspace_id.is_empty() || spec_id.is_empty() || run_id.is_empty() || path.is_empty() {
            return None;
        }

        let uri = format!(
            "mv2://{}/{}/{}/{}/{}",
            workspace_id, spec_id, run_id, object_type.as_str(), path
        );
        Some(LogicalUri(uri))
    }

    /// Create a URI for an event.
    pub fn for_event(workspace_id: &str, spec_id: &str, run_id: &str, seq: u64) -> Self {
        LogicalUri(format!(
            "mv2://{}/{}/{}/event/{}",
            workspace_id, spec_id, run_id, seq
        ))
    }

    /// Create a URI for a checkpoint.
    pub fn for_checkpoint(workspace_id: &str, checkpoint_id: &CheckpointId) -> Self {
        LogicalUri(format!(
            "mv2://{}/checkpoint/{}",
            workspace_id, checkpoint_id.as_str()
        ))
    }

    /// Create a URI for a policy snapshot.
    pub fn for_policy(workspace_id: &str, policy_id: &str) -> Self {
        LogicalUri(format!(
            "mv2://{}/policy/{}",
            workspace_id, policy_id
        ))
    }

    /// Get the raw URI string (for serialization/display only).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this is a valid mv2:// URI.
    pub fn is_valid(&self) -> bool {
        self.0.starts_with("mv2://")
    }
}

impl fmt::Display for LogicalUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for LogicalUri {
    type Err = UriParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("mv2://") {
            return Err(UriParseError::InvalidScheme);
        }
        Ok(LogicalUri(s.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UriParseError {
    InvalidScheme,
    InvalidFormat,
}

impl std::error::Error for UriParseError {}

impl fmt::Display for UriParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UriParseError::InvalidScheme => write!(f, "URI must start with mv2://"),
            UriParseError::InvalidFormat => write!(f, "Invalid URI format"),
        }
    }
}

/// Object types that can be stored in a capsule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    Artifact,
    Event,
    Checkpoint,
    Policy,
    Card,
    Edge,
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Artifact => "artifact",
            ObjectType::Event => "event",
            ObjectType::Checkpoint => "checkpoint",
            ObjectType::Policy => "policy",
            ObjectType::Card => "card",
            ObjectType::Edge => "edge",
        }
    }
}

// =============================================================================
// CheckpointId - Stage boundary + manual commits (D18)
// =============================================================================

/// A checkpoint identifier.
///
/// Checkpoints are created at:
/// - Stage boundary commits (automatic)
/// - Manual commits via `speckit capsule commit --label <LABEL>`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(String);

impl CheckpointId {
    pub fn new(id: impl Into<String>) -> Self {
        CheckpointId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Metadata for a checkpoint.
///
/// ## SPEC-KIT-971: Run Isolation
/// The `branch_id` field enables filtering checkpoints by branch without
/// guessing based on run_id. Format: "main" or "run/<RUN_ID>".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub checkpoint_id: CheckpointId,
    pub label: Option<String>,
    pub stage: Option<String>,
    pub spec_id: Option<String>,
    pub run_id: Option<String>,
    pub commit_hash: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub is_manual: bool,
    /// SPEC-KIT-971: Branch this checkpoint was created on (e.g., "main", "run/abc123")
    #[serde(default)]
    pub branch_id: Option<String>,
}

// =============================================================================
// BranchId - Run isolation (D73, D74)
// =============================================================================

/// A branch identifier for run isolation.
///
/// Every run creates a branch `run/<RUN_ID>` from `main`.
/// Merge to `main` only on Unlock PASS.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(String);

impl BranchId {
    /// The main branch.
    pub fn main() -> Self {
        BranchId("main".to_string())
    }

    /// Create a run branch.
    pub fn for_run(run_id: &str) -> Self {
        BranchId(format!("run/{}", run_id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_main(&self) -> bool {
        self.0 == "main"
    }

    pub fn is_run_branch(&self) -> bool {
        self.0.starts_with("run/")
    }
}

impl fmt::Display for BranchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =============================================================================
// RunEventEnvelope - Event track plumbing (SPEC-KIT-971 baseline)
// =============================================================================

/// Minimal event envelope for the events track.
///
/// SPEC-KIT-971 requires at least:
/// - StageTransition
/// - PolicySnapshotRef
///
/// More event types are added in SPEC-KIT-975.
///
/// ## SPEC-KIT-971: Run Isolation
/// The `branch_id` field enables filtering events by branch without
/// guessing based on run_id. Format: "main" or "run/<RUN_ID>".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEventEnvelope {
    pub uri: LogicalUri,
    pub event_type: EventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub spec_id: String,
    pub run_id: String,
    pub stage: Option<String>,
    pub payload: serde_json::Value,
    /// SPEC-KIT-971: Branch this event was emitted on (e.g., "main", "run/abc123")
    #[serde(default)]
    pub branch_id: Option<String>,
}

/// Event types for the baseline event track.
///
/// SPEC-KIT-971 baseline: StageTransition, PolicySnapshotRef
/// SPEC-KIT-978 adds: RoutingDecision (Implementer mode selection)
/// SPEC-KIT-975 expands: ToolCall, RetrievalRequest, GateDecision, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    // SPEC-KIT-971 baseline
    StageTransition,
    PolicySnapshotRef,

    // SPEC-KIT-978: Model routing decisions (reflex vs cloud)
    RoutingDecision,

    // SPEC-KIT-975 will add:
    // RetrievalRequest,
    // RetrievalResponse,
    // ToolCall,
    // ToolResult,
    // PatchApply,
    // GateDecision,
    // ErrorEvent,
    // ModelCallEnvelope,
    // BranchMerged,
    // CapsuleExported,
    // CapsuleImported,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::StageTransition => "StageTransition",
            EventType::PolicySnapshotRef => "PolicySnapshotRef",
            EventType::RoutingDecision => "RoutingDecision",
        }
    }
}

// =============================================================================
// SPEC-KIT-978: Routing Decision Types
// =============================================================================

/// Routing mode for Implementer role.
///
/// SPEC-KIT-978: Implementer can run in two modes:
/// - Cloud: Standard cloud inference (Claude/GPT)
/// - Reflex: Local inference via SGLang/vLLM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingMode {
    /// Standard cloud inference (default)
    Cloud,
    /// Local reflex inference (SPEC-KIT-978)
    Reflex,
}

impl RoutingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoutingMode::Cloud => "cloud",
            RoutingMode::Reflex => "reflex",
        }
    }
}

/// Reason for routing decision fallback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingFallbackReason {
    /// Reflex mode is disabled in config
    ReflexDisabled,
    /// Reflex server is not healthy
    ServerUnhealthy,
    /// Configured model not available
    ModelNotAvailable,
    /// Latency threshold exceeded
    LatencyThresholdExceeded,
    /// Success rate below threshold
    SuccessRateBelowThreshold,
    /// JSON schema compliance below threshold
    JsonComplianceBelowThreshold,
    /// Not in Implement stage (reflex only valid for Implement)
    NotImplementStage,
}

impl RoutingFallbackReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoutingFallbackReason::ReflexDisabled => "reflex_disabled",
            RoutingFallbackReason::ServerUnhealthy => "server_unhealthy",
            RoutingFallbackReason::ModelNotAvailable => "model_not_available",
            RoutingFallbackReason::LatencyThresholdExceeded => "latency_threshold_exceeded",
            RoutingFallbackReason::SuccessRateBelowThreshold => "success_rate_below_threshold",
            RoutingFallbackReason::JsonComplianceBelowThreshold => "json_compliance_below_threshold",
            RoutingFallbackReason::NotImplementStage => "not_implement_stage",
        }
    }
}

/// Routing decision outcome for capsule event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecisionPayload {
    /// Selected routing mode
    pub mode: RoutingMode,
    /// Stage where decision was made
    pub stage: String,
    /// Agent/role making the request
    pub role: String,
    /// Whether this was a fallback from reflex
    pub is_fallback: bool,
    /// Reason for fallback (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<RoutingFallbackReason>,
    /// Reflex endpoint (if reflex mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflex_endpoint: Option<String>,
    /// Reflex model (if reflex mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflex_model: Option<String>,
    /// Cloud model (if cloud mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_model: Option<String>,
    /// Health check latency in ms (if reflex attempted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_latency_ms: Option<u64>,
}

// =============================================================================
// UriIndex - URI resolution (D70 implementation posture)
// =============================================================================

/// Index mapping logical URIs to physical pointers.
///
/// Per SPEC-KIT-971 implementation posture:
/// - Maintain a `uri_index` track that maps `uri → latest_physical_pointer`
///   per `(branch_id, checkpoint)`
/// - Update this index at commit barriers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UriIndex {
    /// Map from logical URI to physical frame pointer
    entries: std::collections::HashMap<LogicalUri, PhysicalPointer>,
}

impl UriIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new URI → physical pointer mapping.
    pub fn insert(&mut self, uri: LogicalUri, pointer: PhysicalPointer) {
        self.entries.insert(uri, pointer);
    }

    /// Resolve a logical URI to its physical pointer.
    pub fn resolve(&self, uri: &LogicalUri) -> Option<&PhysicalPointer> {
        self.entries.get(uri)
    }

    /// Check if a URI exists in the index.
    pub fn contains(&self, uri: &LogicalUri) -> bool {
        self.entries.contains_key(uri)
    }

    /// Get the number of URIs in the index.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A physical pointer to data in the capsule (internal, not exposed externally).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhysicalPointer {
    pub frame_id: u64,
    pub offset: u64,
    pub length: u64,
}

// =============================================================================
// MergeMode - curated|full (not squash|ff!) per architect guidance
// =============================================================================

/// Merge mode for run branches.
///
/// ## CRITICAL: Use curated|full, NOT squash|ff
/// Per architect feedback: "define a shared enum/type used by CLI/TUI + adapter
/// so nobody reintroduces squash|ff in code."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeMode {
    /// Default: promote curated artifacts + graph deltas + summary events.
    /// Debug/telemetry stays run-isolated.
    Curated,

    /// Escape hatch: promote everything (deep audit / incident review).
    Full,
}

impl MergeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            MergeMode::Curated => "curated",
            MergeMode::Full => "full",
        }
    }
}

impl Default for MergeMode {
    fn default() -> Self {
        MergeMode::Curated
    }
}

#[cfg(test)]
mod type_tests {
    use super::*;

    #[test]
    fn logical_uri_is_not_string() {
        // This test exists to remind implementors: LogicalUri is NOT String.
        // If you find yourself doing `.to_string()` on a LogicalUri to pass
        // it somewhere, you're probably doing it wrong.
        let uri = LogicalUri::new("ws1", "SPEC-971", "run1", ObjectType::Artifact, "file.md");
        assert!(uri.is_some());
        assert!(uri.unwrap().is_valid());
    }

    #[test]
    fn merge_mode_is_curated_or_full_not_squash_ff() {
        // Per architect: "Add a unit test that asserts the public CLI help text
        // contains curated and not squash."
        // This test ensures the enum uses correct terminology.
        assert_eq!(MergeMode::Curated.as_str(), "curated");
        assert_eq!(MergeMode::Full.as_str(), "full");

        // These should NEVER exist:
        // MergeMode::Squash
        // MergeMode::FastForward
    }

    #[test]
    fn branch_id_run_isolation() {
        let main = BranchId::main();
        assert!(main.is_main());
        assert!(!main.is_run_branch());

        let run = BranchId::for_run("abc123");
        assert!(!run.is_main());
        assert!(run.is_run_branch());
        assert_eq!(run.as_str(), "run/abc123");
    }
}
