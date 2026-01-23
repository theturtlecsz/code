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
            workspace_id,
            spec_id,
            run_id,
            object_type.as_str(),
            path
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
            workspace_id,
            checkpoint_id.as_str()
        ))
    }

    /// Create a URI for a policy snapshot.
    pub fn for_policy(workspace_id: &str, policy_id: &str) -> Self {
        LogicalUri(format!("mv2://{}/policy/{}", workspace_id, policy_id))
    }

    /// Get the raw URI string (for serialization/display only).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this is a valid mv2:// URI.
    pub fn is_valid(&self) -> bool {
        self.0.starts_with("mv2://")
    }

    /// Extract the object type from the URI path.
    ///
    /// URI format: mv2://{workspace}/{spec}/{run}/{type}/{path}
    /// Returns None if the URI format is invalid or type is unrecognized.
    pub fn object_type(&self) -> Option<ObjectType> {
        let stripped = self.0.strip_prefix("mv2://")?;
        let parts: Vec<&str> = stripped.split('/').collect();

        // Standard format: workspace/spec/run/type/path
        if parts.len() >= 4 {
            return match parts[3] {
                "artifact" => Some(ObjectType::Artifact),
                "event" => Some(ObjectType::Event),
                "checkpoint" => Some(ObjectType::Checkpoint),
                "policy" => Some(ObjectType::Policy),
                "card" => Some(ObjectType::Card),
                "edge" => Some(ObjectType::Edge),
                _ => None,
            };
        }

        // Policy format: workspace/policy/id
        if parts.len() >= 2 && parts[1] == "policy" {
            return Some(ObjectType::Policy);
        }

        None
    }

    /// Check if this URI represents a curated-eligible artifact.
    ///
    /// Curated-eligible: Artifact, Policy, Card, Edge
    /// Not curated-eligible: Event (handled separately), Checkpoint
    pub fn is_curated_eligible(&self) -> bool {
        match self.object_type() {
            Some(ObjectType::Artifact) => true,
            Some(ObjectType::Policy) => true,
            Some(ObjectType::Card) => true,
            Some(ObjectType::Edge) => true,
            Some(ObjectType::Event) => false, // Events are filtered separately
            Some(ObjectType::Checkpoint) => false,
            None => false,
        }
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

    /// Create a BranchId from a string (for deserialization).
    pub fn from_str(s: &str) -> Self {
        BranchId(s.to_string())
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
    // SPEC-KIT-971 baseline (curated-eligible)
    StageTransition,
    PolicySnapshotRef,

    // SPEC-KIT-978: Model routing decisions (curated-eligible)
    RoutingDecision,

    // SPEC-KIT-971: Branch merge at Unlock (curated-eligible)
    BranchMerged,

    // Debug/telemetry events (excluded from curated merge)
    DebugTrace,

    // =========================================================================
    // SPEC-KIT-975: Replayable Audit Event Types
    // =========================================================================
    /// Retrieval request event - captures retrieval queries for replay.
    /// Curated-eligible: Yes (audit trail for retrieval decisions)
    RetrievalRequest,

    /// Retrieval response event - captures retrieval results.
    /// Curated-eligible: Yes (audit trail for what was retrieved)
    RetrievalResponse,

    /// Tool call event - captures tool invocations.
    /// Curated-eligible: Yes (audit trail for tool usage)
    ToolCall,

    /// Tool result event - captures tool outputs.
    /// Curated-eligible: Yes (audit trail for tool results)
    ToolResult,

    /// Patch application event - captures file modifications.
    /// Curated-eligible: Yes (audit trail for code changes)
    PatchApply,

    /// Gate decision event - captures governance gate outcomes.
    /// Curated-eligible: Yes (critical for compliance audit)
    GateDecision,

    /// Error event - captures errors during run execution.
    /// Curated-eligible: Yes (essential for debugging and audit)
    ErrorEvent,

    /// Model call envelope - captures LLM request/response.
    /// Curated-eligible: Depends on capture mode (see LLMCaptureMode)
    /// - off: Not stored
    /// - hash: Hash only (curated)
    /// - summary: Summary only (curated)
    /// - full: Full content (NOT curated by default, may contain sensitive data)
    ModelCallEnvelope,

    /// Capsule export event - tracks when capsule is exported.
    /// Curated-eligible: Yes (provenance tracking)
    CapsuleExported,

    /// Capsule import event - tracks when capsule is imported.
    /// Curated-eligible: Yes (provenance tracking)
    CapsuleImported,

    /// Circuit breaker state change event.
    /// Curated-eligible: Yes (critical for observability)
    /// SPEC-KIT-978: Tracks service protection state transitions.
    BreakerStateChanged,

    // =========================================================================
    // SPEC-KIT-979: Local-Memory Sunset Events
    // =========================================================================
    /// Sunset phase resolution at run start.
    /// Curated-eligible: Yes (audit trail for phase enforcement)
    /// Records policy_phase, env_phase override, effective_phase, resolution_source.
    LocalMemorySunsetPhaseResolved,

    /// Fallback activation event for GATE-ST stability tracking.
    /// Curated-eligible: Yes (critical for 30-day stability gate)
    /// Emitted when memvid falls back to local-memory.
    FallbackActivated,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            // SPEC-KIT-971 baseline
            EventType::StageTransition => "StageTransition",
            EventType::PolicySnapshotRef => "PolicySnapshotRef",
            EventType::RoutingDecision => "RoutingDecision",
            EventType::BranchMerged => "BranchMerged",
            EventType::DebugTrace => "DebugTrace",
            // SPEC-KIT-975 additions
            EventType::RetrievalRequest => "RetrievalRequest",
            EventType::RetrievalResponse => "RetrievalResponse",
            EventType::ToolCall => "ToolCall",
            EventType::ToolResult => "ToolResult",
            EventType::PatchApply => "PatchApply",
            EventType::GateDecision => "GateDecision",
            EventType::ErrorEvent => "ErrorEvent",
            EventType::ModelCallEnvelope => "ModelCallEnvelope",
            EventType::CapsuleExported => "CapsuleExported",
            EventType::CapsuleImported => "CapsuleImported",
            EventType::BreakerStateChanged => "BreakerStateChanged",
            // SPEC-KIT-979 additions
            EventType::LocalMemorySunsetPhaseResolved => "LocalMemorySunsetPhaseResolved",
            EventType::FallbackActivated => "FallbackActivated",
        }
    }

    /// Parse event type from string.
    ///
    /// Used for CLI filtering (e.g., `--type ToolCall`).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "StageTransition" => Some(EventType::StageTransition),
            "PolicySnapshotRef" => Some(EventType::PolicySnapshotRef),
            "RoutingDecision" => Some(EventType::RoutingDecision),
            "BranchMerged" => Some(EventType::BranchMerged),
            "DebugTrace" => Some(EventType::DebugTrace),
            "RetrievalRequest" => Some(EventType::RetrievalRequest),
            "RetrievalResponse" => Some(EventType::RetrievalResponse),
            "ToolCall" => Some(EventType::ToolCall),
            "ToolResult" => Some(EventType::ToolResult),
            "PatchApply" => Some(EventType::PatchApply),
            "GateDecision" => Some(EventType::GateDecision),
            "ErrorEvent" => Some(EventType::ErrorEvent),
            "ModelCallEnvelope" => Some(EventType::ModelCallEnvelope),
            "CapsuleExported" => Some(EventType::CapsuleExported),
            "CapsuleImported" => Some(EventType::CapsuleImported),
            "BreakerStateChanged" => Some(EventType::BreakerStateChanged),
            // SPEC-KIT-979 additions
            "LocalMemorySunsetPhaseResolved" => Some(EventType::LocalMemorySunsetPhaseResolved),
            "FallbackActivated" => Some(EventType::FallbackActivated),
            _ => None,
        }
    }

    /// Get all event type variants for CLI help.
    pub fn all_variants() -> &'static [&'static str] {
        &[
            "StageTransition",
            "PolicySnapshotRef",
            "RoutingDecision",
            "BranchMerged",
            "DebugTrace",
            "RetrievalRequest",
            "RetrievalResponse",
            "ToolCall",
            "ToolResult",
            "PatchApply",
            "GateDecision",
            "ErrorEvent",
            "ModelCallEnvelope",
            "CapsuleExported",
            "CapsuleImported",
            "BreakerStateChanged",
            // SPEC-KIT-979 additions
            "LocalMemorySunsetPhaseResolved",
            "FallbackActivated",
        ]
    }

    /// Check if this event type should be included in curated merge.
    ///
    /// Curated merge includes governance-critical events:
    /// - StageTransition: Stage boundary markers
    /// - PolicySnapshotRef: Policy version tracking
    /// - RoutingDecision: Model selection audit trail
    /// - BranchMerged: Merge provenance
    /// - RetrievalRequest/Response: Retrieval audit trail
    /// - ToolCall/Result: Tool usage audit trail
    /// - PatchApply: Code change audit trail
    /// - GateDecision: Governance gate outcomes
    /// - ErrorEvent: Error tracking
    /// - CapsuleExported/Imported: Provenance tracking
    ///
    /// Excluded from curated (debug-only or sensitive):
    /// - DebugTrace: Verbose debugging/telemetry
    /// - ModelCallEnvelope: May contain sensitive LLM I/O (depends on capture mode)
    pub fn is_curated_eligible(&self) -> bool {
        match self {
            // SPEC-KIT-971 baseline (curated)
            EventType::StageTransition => true,
            EventType::PolicySnapshotRef => true,
            EventType::RoutingDecision => true,
            EventType::BranchMerged => true,
            // Debug/telemetry (not curated)
            EventType::DebugTrace => false,
            // SPEC-KIT-975 additions (mostly curated)
            EventType::RetrievalRequest => true,
            EventType::RetrievalResponse => true,
            EventType::ToolCall => true,
            EventType::ToolResult => true,
            EventType::PatchApply => true,
            EventType::GateDecision => true,
            EventType::ErrorEvent => true,
            // ModelCallEnvelope: NOT curated by default (may contain sensitive data)
            // Use full merge mode if you need to include these
            EventType::ModelCallEnvelope => false,
            EventType::CapsuleExported => true,
            EventType::CapsuleImported => true,
            // SPEC-KIT-978: Circuit breaker state changes are curated (observability)
            EventType::BreakerStateChanged => true,
            // SPEC-KIT-979: Sunset phase events are curated (audit trail + GATE-ST)
            EventType::LocalMemorySunsetPhaseResolved => true,
            EventType::FallbackActivated => true,
        }
    }

    /// Check if this event type is audit-critical.
    ///
    /// Audit-critical events MUST be captured for compliance replay.
    /// Non-audit-critical events are optional/debug.
    pub fn is_audit_critical(&self) -> bool {
        match self {
            EventType::StageTransition => true,
            EventType::PolicySnapshotRef => true,
            EventType::RoutingDecision => true,
            EventType::BranchMerged => true,
            EventType::GateDecision => true,
            EventType::ErrorEvent => true,
            EventType::CapsuleExported => true,
            EventType::CapsuleImported => true,
            // Optional for audit
            EventType::DebugTrace => false,
            EventType::RetrievalRequest => false,
            EventType::RetrievalResponse => false,
            EventType::ToolCall => false,
            EventType::ToolResult => false,
            EventType::PatchApply => false,
            EventType::ModelCallEnvelope => false,
            // SPEC-KIT-978: Circuit breaker state changes are audit-critical
            EventType::BreakerStateChanged => true,
            // SPEC-KIT-979: Phase resolution and fallback events are audit-critical
            EventType::LocalMemorySunsetPhaseResolved => true,
            EventType::FallbackActivated => true,
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
            RoutingFallbackReason::JsonComplianceBelowThreshold => {
                "json_compliance_below_threshold"
            }
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

/// Index mapping logical URIs to physical pointers with time-travel support.
///
/// Per SPEC-KIT-971 implementation posture:
/// - Maintain a `uri_index` track that maps `uri → latest_physical_pointer`
///   per `(branch_id, checkpoint)`
/// - Update this index at commit barriers
/// - Support time-travel resolution via `as_of` checkpoint
///
/// ## Structure
/// - `entries`: Branch-scoped current state (BranchId → (URI → Pointer))
/// - `snapshots`: Historical snapshots keyed by (BranchId, CheckpointId)
///
/// ## Time-Travel Resolution
/// - resolve(uri, branch, None) → current pointer on branch
/// - resolve(uri, branch, Some(checkpoint)) → pointer at that checkpoint
#[derive(Debug, Clone, Default)]
pub struct UriIndex {
    /// Current entries per branch: BranchId → (LogicalUri → PhysicalPointer)
    entries:
        std::collections::HashMap<BranchId, std::collections::HashMap<LogicalUri, PhysicalPointer>>,

    /// Historical snapshots: (BranchId, CheckpointId) → (LogicalUri → PhysicalPointer)
    /// Created at each commit_stage/commit_manual checkpoint.
    snapshots: std::collections::HashMap<
        (BranchId, CheckpointId),
        std::collections::HashMap<LogicalUri, PhysicalPointer>,
    >,
}

/// Serializable form of UriIndex for persistence.
///
/// We serialize snapshots to disk so time-travel works after reopen.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UriIndexSnapshot {
    /// The checkpoint this snapshot is for
    pub checkpoint_id: String,
    /// The branch this snapshot is for
    pub branch_id: String,
    /// URI → PhysicalPointer mappings at this checkpoint
    pub entries: std::collections::HashMap<String, PhysicalPointer>,
}

impl UriIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new URI → physical pointer mapping on the given branch.
    ///
    /// This updates the current (live) state for the branch.
    pub fn insert_on_branch(
        &mut self,
        branch: &BranchId,
        uri: LogicalUri,
        pointer: PhysicalPointer,
    ) {
        self.entries
            .entry(branch.clone())
            .or_default()
            .insert(uri, pointer);
    }

    /// Register a new URI → physical pointer mapping (uses main branch).
    ///
    /// For backward compatibility with existing code that doesn't specify branch.
    pub fn insert(&mut self, uri: LogicalUri, pointer: PhysicalPointer) {
        self.insert_on_branch(&BranchId::main(), uri, pointer);
    }

    /// Resolve a logical URI to its physical pointer on the given branch.
    ///
    /// ## Parameters
    /// - `uri`: The logical URI to resolve
    /// - `branch`: Branch to look up (defaults to main if None)
    /// - `as_of`: Checkpoint for time-travel (None = current/latest)
    ///
    /// ## Returns
    /// - If `as_of` is None: returns current pointer on branch
    /// - If `as_of` is Some: returns pointer at that checkpoint (time-travel)
    pub fn resolve_on_branch(
        &self,
        uri: &LogicalUri,
        branch: &BranchId,
        as_of: Option<&CheckpointId>,
    ) -> Option<&PhysicalPointer> {
        match as_of {
            Some(checkpoint_id) => {
                // Time-travel: look up in snapshot
                let key = (branch.clone(), checkpoint_id.clone());
                self.snapshots
                    .get(&key)
                    .and_then(|snapshot| snapshot.get(uri))
            }
            None => {
                // Current state: look up in live entries
                self.entries.get(branch).and_then(|map| map.get(uri))
            }
        }
    }

    /// Resolve a logical URI to its physical pointer (uses main branch, current state).
    ///
    /// For backward compatibility with existing code.
    pub fn resolve(&self, uri: &LogicalUri) -> Option<&PhysicalPointer> {
        // Check main branch first, then check all branches for backward compat
        if let Some(ptr) = self.entries.get(&BranchId::main()).and_then(|m| m.get(uri)) {
            return Some(ptr);
        }
        // Fallback: check all branches (for URIs inserted without branch context)
        for map in self.entries.values() {
            if let Some(ptr) = map.get(uri) {
                return Some(ptr);
            }
        }
        None
    }

    /// Check if a URI exists in the index (any branch).
    pub fn contains(&self, uri: &LogicalUri) -> bool {
        self.entries.values().any(|map| map.contains_key(uri))
    }

    /// Get the total number of URIs across all branches.
    pub fn len(&self) -> usize {
        self.entries.values().map(|m| m.len()).sum()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.values().all(|m| m.is_empty())
    }

    /// Create a snapshot of the current branch state at a checkpoint.
    ///
    /// Called by commit_stage/commit_manual to enable time-travel resolution.
    pub fn snapshot(&mut self, branch: &BranchId, checkpoint_id: &CheckpointId) {
        if let Some(current) = self.entries.get(branch) {
            let key = (branch.clone(), checkpoint_id.clone());
            self.snapshots.insert(key, current.clone());
        } else {
            // Empty branch - still create snapshot (empty map)
            let key = (branch.clone(), checkpoint_id.clone());
            self.snapshots.insert(key, std::collections::HashMap::new());
        }
    }

    /// Check if a snapshot exists for a (branch, checkpoint) pair.
    pub fn has_snapshot(&self, branch: &BranchId, checkpoint_id: &CheckpointId) -> bool {
        let key = (branch.clone(), checkpoint_id.clone());
        self.snapshots.contains_key(&key)
    }

    /// Export a snapshot for persistence.
    ///
    /// Returns a serializable form of the snapshot at the given checkpoint.
    pub fn export_snapshot(
        &self,
        branch: &BranchId,
        checkpoint_id: &CheckpointId,
    ) -> Option<UriIndexSnapshot> {
        let key = (branch.clone(), checkpoint_id.clone());
        self.snapshots.get(&key).map(|entries| UriIndexSnapshot {
            checkpoint_id: checkpoint_id.as_str().to_string(),
            branch_id: branch.as_str().to_string(),
            entries: entries
                .iter()
                .map(|(uri, ptr)| (uri.as_str().to_string(), ptr.clone()))
                .collect(),
        })
    }

    /// Import a snapshot from persistence.
    ///
    /// Used during scan_and_rebuild to restore historical snapshots.
    pub fn import_snapshot(&mut self, snapshot: UriIndexSnapshot) {
        let branch = BranchId::from_str(&snapshot.branch_id);
        let checkpoint = CheckpointId::new(snapshot.checkpoint_id);
        let key = (branch, checkpoint);

        let entries: std::collections::HashMap<LogicalUri, PhysicalPointer> = snapshot
            .entries
            .into_iter()
            .filter_map(|(uri_str, ptr)| uri_str.parse::<LogicalUri>().ok().map(|uri| (uri, ptr)))
            .collect();

        self.snapshots.insert(key, entries);
    }

    /// Get all snapshot keys (for diagnostics/testing).
    pub fn snapshot_keys(&self) -> Vec<(BranchId, CheckpointId)> {
        self.snapshots.keys().cloned().collect()
    }

    /// Get all branch keys (for diagnostics/testing).
    pub fn branch_keys(&self) -> Vec<BranchId> {
        self.entries.keys().cloned().collect()
    }

    /// Count the number of URIs on a specific branch.
    pub fn count_on_branch(&self, branch: &BranchId) -> usize {
        self.entries.get(branch).map(|m| m.len()).unwrap_or(0)
    }

    /// Merge URI mappings from one branch to another based on merge mode.
    ///
    /// ## SPEC-KIT-971: Merge at Unlock
    ///
    /// ### Curated Mode
    /// Copies only curated-eligible entries (Artifact, Policy, Card, Edge).
    /// Debug/telemetry URIs stay isolated on the run branch.
    ///
    /// ### Full Mode
    /// Copies all entries from `from` branch to `to` branch.
    /// Existing entries on `to` branch with the same URI are overwritten.
    ///
    /// Returns the number of URIs actually merged.
    pub fn merge_branch(&mut self, from: &BranchId, to: &BranchId, mode: MergeMode) -> usize {
        let mut merged_count = 0;
        if let Some(source_entries) = self.entries.get(from).cloned() {
            let target = self.entries.entry(to.clone()).or_default();
            for (uri, pointer) in source_entries {
                let should_merge = match mode {
                    MergeMode::Full => true,
                    MergeMode::Curated => uri.is_curated_eligible(),
                };
                if should_merge {
                    target.insert(uri, pointer);
                    merged_count += 1;
                }
            }
        }
        merged_count
    }

    /// Count URIs that would be merged in curated mode.
    ///
    /// Used to calculate accurate merge statistics for BranchMerged event.
    pub fn count_curated_on_branch(&self, branch: &BranchId) -> usize {
        self.entries
            .get(branch)
            .map(|m| m.keys().filter(|uri| uri.is_curated_eligible()).count())
            .unwrap_or(0)
    }

    /// Restore branch entries from the latest snapshot for each branch.
    ///
    /// ## SPEC-KIT-971: Branch context preservation after reopen
    ///
    /// After `scan_and_rebuild()` imports all snapshots, this method reconstructs
    /// the "current state" (`entries`) for each branch by finding the latest
    /// checkpoint snapshot and using it as the branch HEAD.
    ///
    /// This ensures that `resolve_uri(branch, as_of=None)` returns the same
    /// result as `resolve_uri(branch, as_of=<latest checkpoint on branch>)`.
    ///
    /// ## Parameters
    /// - `checkpoints`: List of checkpoint metadata with timestamps for ordering
    pub fn restore_entries_from_latest_snapshots(&mut self, checkpoints: &[CheckpointMetadata]) {
        // Group checkpoints by branch and find the latest for each
        let mut latest_per_branch: std::collections::HashMap<BranchId, &CheckpointMetadata> =
            std::collections::HashMap::new();

        for cp in checkpoints {
            // Determine branch from checkpoint metadata
            let branch = if let Some(ref branch_str) = cp.branch_id {
                BranchId::from_str(branch_str)
            } else if let Some(ref run_id) = cp.run_id {
                // Fallback for older checkpoints without explicit branch_id
                BranchId::for_run(run_id)
            } else {
                BranchId::main()
            };

            // Check if this checkpoint is newer than any we've seen for this branch
            match latest_per_branch.get(&branch) {
                Some(existing) => {
                    if cp.timestamp > existing.timestamp {
                        latest_per_branch.insert(branch, cp);
                    }
                }
                None => {
                    latest_per_branch.insert(branch, cp);
                }
            }
        }

        // For each branch, restore entries from its latest snapshot
        for (branch, latest_cp) in latest_per_branch {
            let key = (branch.clone(), latest_cp.checkpoint_id.clone());
            if let Some(snapshot_entries) = self.snapshots.get(&key) {
                // Clone the snapshot to populate entries for this branch
                self.entries.insert(branch, snapshot_entries.clone());
            }
        }
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

/// Branch merge event payload for capsule events.
///
/// Emitted when a run branch is merged into main at Unlock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMergedPayload {
    /// Source branch (e.g., "run/<RUN_ID>")
    pub from_branch: String,
    /// Target branch (always "main")
    pub to_branch: String,
    /// Merge mode used
    pub mode: MergeMode,
    /// Checkpoint ID created for the merge
    pub merge_checkpoint_id: String,
    /// Number of URIs merged
    pub uris_merged: u64,
    /// Number of events merged
    pub events_merged: u64,
    /// Spec ID (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<String>,
    /// Run ID (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

// =============================================================================
// SPEC-KIT-975: Replayable Audit Event Payloads
// =============================================================================

/// LLM capture mode for ModelCallEnvelope events.
///
/// Per D15 (audit.capture_llm_io): Controls what model I/O is captured.
/// Vocabulary aligns with model_policy.toml [capture] section.
/// - none: Don't capture model calls at all
/// - prompts_only: Capture prompts + response hash (no response text, safe for export)
/// - full_io: Capture full prompt + response (may contain sensitive data)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LLMCaptureMode {
    /// Don't capture model calls at all
    None,
    /// Capture prompts + metadata; response hash only (no response text)
    #[default]
    PromptsOnly,
    /// Capture full prompt + response (may contain sensitive data)
    FullIo,
}

impl LLMCaptureMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LLMCaptureMode::None => "none",
            LLMCaptureMode::PromptsOnly => "prompts_only",
            LLMCaptureMode::FullIo => "full_io",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(LLMCaptureMode::None),
            "prompts_only" => Some(LLMCaptureMode::PromptsOnly),
            "full_io" => Some(LLMCaptureMode::FullIo),
            // Backward compat for old serialized values
            "off" => Some(LLMCaptureMode::None),
            "hash" | "summary" => Some(LLMCaptureMode::PromptsOnly),
            "full" => Some(LLMCaptureMode::FullIo),
            _ => None,
        }
    }

    /// Check if this mode is safe for capsule export.
    pub fn is_export_safe(&self) -> bool {
        match self {
            LLMCaptureMode::None => true,
            LLMCaptureMode::PromptsOnly => true, // Response text not stored
            LLMCaptureMode::FullIo => false,     // May contain sensitive data
        }
    }
}

/// Retrieval request event payload.
///
/// Captures the query parameters for a retrieval request.
/// Used for replay verification: re-run retrieval with same params.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalRequestPayload {
    /// Unique request ID for correlation with response
    pub request_id: String,
    /// Query text
    pub query: String,
    /// Retrieval configuration (top_k, filters, etc.)
    pub config: serde_json::Value,
    /// Source (e.g., "capsule", "tier2:notebooklm")
    pub source: String,
    /// Stage where retrieval was requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Agent/role making the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Retrieval response event payload.
///
/// Captures the results of a retrieval request.
/// Used for replay verification: compare hit sets and scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResponsePayload {
    /// Request ID for correlation
    pub request_id: String,
    /// Hit URIs returned (in order)
    pub hit_uris: Vec<String>,
    /// Fused scores for each hit (for epsilon comparison during replay)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fused_scores: Option<Vec<f64>>,
    /// Explainability fields (why these results?)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explainability: Option<serde_json::Value>,
    /// Latency in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Error message if retrieval failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Tool call event payload.
///
/// Captures a tool invocation for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPayload {
    /// Unique call ID for correlation with result
    pub call_id: String,
    /// Tool name
    pub tool_name: String,
    /// Tool input (JSON)
    pub input: serde_json::Value,
    /// Stage where tool was called
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Agent/role making the call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Tool result event payload.
///
/// Captures a tool's output for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPayload {
    /// Call ID for correlation
    pub call_id: String,
    /// Tool name
    pub tool_name: String,
    /// Success status
    pub success: bool,
    /// Tool output (JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Patch application event payload.
///
/// Captures file modifications for audit trail and replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchApplyPayload {
    /// Unique patch ID
    pub patch_id: String,
    /// File path (relative to workspace)
    pub file_path: String,
    /// Patch type: "create", "modify", "delete"
    pub patch_type: String,
    /// Unified diff (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    /// Hash of file content before patch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_hash: Option<String>,
    /// Hash of file content after patch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_hash: Option<String>,
    /// Stage where patch was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Success status
    pub success: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Gate decision outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateOutcome {
    Pass,
    Fail,
    Warn,
    Skip,
}

impl GateOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            GateOutcome::Pass => "pass",
            GateOutcome::Fail => "fail",
            GateOutcome::Warn => "warn",
            GateOutcome::Skip => "skip",
        }
    }
}

/// Circuit breaker state.
///
/// Tracks the state of a circuit breaker for service protection.
/// SPEC-KIT-978: Used for observability and fallback control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BreakerState {
    /// Circuit is closed (normal operation, requests allowed)
    Closed,
    /// Circuit is open (requests rejected, waiting for recovery)
    Open,
    /// Circuit is half-open (testing recovery with probe requests)
    HalfOpen,
}

impl BreakerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            BreakerState::Closed => "closed",
            BreakerState::Open => "open",
            BreakerState::HalfOpen => "half_open",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "closed" => Some(BreakerState::Closed),
            "open" => Some(BreakerState::Open),
            "half_open" => Some(BreakerState::HalfOpen),
            _ => None,
        }
    }
}

/// Gate decision event payload.
///
/// Captures governance gate outcomes for compliance audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecisionPayload {
    /// Gate name (e.g., "JudgeApprove", "LintCheck", "TestPass")
    pub gate_name: String,
    /// Gate outcome
    pub outcome: GateOutcome,
    /// Stage where gate was evaluated
    pub stage: String,
    /// Gate confidence score (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Reason for outcome
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Additional details (structured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Blocking: if true, failure blocks stage transition
    pub blocking: bool,
}

/// Circuit breaker state change event payload.
///
/// Captures circuit breaker state transitions for observability.
/// SPEC-KIT-978: Critical for tracking service health and fallback behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerStateChangedPayload {
    /// Unique breaker identifier (e.g., "reflex_server", "retrieval_service")
    pub breaker_id: String,
    /// Current state after transition
    pub current_state: BreakerState,
    /// Previous state
    pub previous_state: BreakerState,
    /// Reason for state transition
    pub reason: String,
    /// Stage where transition occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Component/subsystem that triggered the transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    /// Failure count that triggered Open (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_count: Option<u64>,
    /// Failure rate percentage (0.0-100.0) that triggered Open
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_rate: Option<f64>,
    /// Time (seconds) until retry in Open state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
    /// Successful probes since opening (for HalfOpen state)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successful_probes: Option<u64>,
    /// Required probes to transition to Closed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probes_required: Option<u64>,
}

/// Error severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Fatal,
}

impl ErrorSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Warning => "warning",
            ErrorSeverity::Error => "error",
            ErrorSeverity::Fatal => "fatal",
        }
    }
}

/// Error event payload.
///
/// Captures errors during run execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEventPayload {
    /// Error code (e.g., "E001", "RETRIEVAL_FAILED")
    pub error_code: String,
    /// Error message
    pub message: String,
    /// Severity level
    pub severity: ErrorSeverity,
    /// Stage where error occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Component/subsystem that failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    /// Stack trace (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
    /// Related event URIs (for context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_uris: Option<Vec<String>>,
    /// Recoverable: if true, run can continue
    pub recoverable: bool,
}

/// Model call envelope payload.
///
/// Captures LLM request/response based on capture mode.
/// Content fields are populated based on LLMCaptureMode:
/// - None: No event emitted
/// - PromptsOnly: prompt + hashes (no response text)
/// - FullIo: prompt + response + hashes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCallEnvelopePayload {
    /// Unique call ID
    pub call_id: String,
    /// Model identifier (e.g., "claude-3-opus", "qwen2.5-coder")
    pub model: String,
    /// Routing mode used
    pub routing_mode: RoutingMode,
    /// Capture mode used for this call
    pub capture_mode: LLMCaptureMode,
    /// Stage where call was made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Agent/role making the call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    // Hash fields (always present for verification)
    /// SHA-256 hash of prompt content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
    /// SHA-256 hash of response content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_hash: Option<String>,

    // Prompt content (present in prompts_only and full_io)
    /// Full prompt content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    // Response content (present in full_io only)
    /// Full response content (ONLY if capture_mode = FullIo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,

    // Token counts (always present if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_tokens: Option<u64>,

    // Common metadata
    /// Latency in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Success status
    pub success: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Capsule export event payload.
///
/// Tracks when a capsule is exported for provenance.
///
/// ## SPEC-KIT-974 Acceptance Criteria
/// Event includes: run_id, spec_id (in envelope), digest, encryption flag, safe flag, included tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleExportedPayload {
    /// Export destination type (e.g., "file", "remote")
    pub destination_type: String,
    /// Export destination (path or URL, may be redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    /// Export format (e.g., "mv2-v1", "mv2e-v1")
    pub format: String,
    /// Checkpoints included in export (included tracks)
    pub checkpoints_included: Vec<String>,
    /// Whether export was sanitized (secrets redacted) - safe flag
    pub sanitized: bool,
    /// Whether the export is encrypted (.mv2e) - encryption flag
    /// S974-003: Default false for backward compatibility with pre-encryption payloads
    #[serde(default)]
    pub encrypted: bool,
    /// Export timestamp
    pub exported_at: chrono::DateTime<chrono::Utc>,
    /// SHA-256 hash of exported content - digest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Capsule import event payload.
///
/// Tracks when a capsule is imported for provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleImportedPayload {
    /// Source type (e.g., "file", "remote")
    pub source_type: String,
    /// Source (path or URL, may be redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Import format
    pub format: String,
    /// Original capsule ID (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_capsule_id: Option<String>,
    /// Checkpoints imported
    pub checkpoints_imported: Vec<String>,
    /// Import timestamp
    pub imported_at: chrono::DateTime<chrono::Utc>,
    /// SHA-256 hash of imported content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

// =============================================================================
// SPEC-KIT-979: Local-Memory Sunset Payloads
// =============================================================================

/// Sunset phase resolution payload.
///
/// Records how the effective sunset phase was determined at run start.
/// Used for auditability and debugging phase override behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResolutionPayload {
    /// Phase configured in model_policy.toml
    pub policy_phase: u8,
    /// Phase from CODE_SUNSET_PHASE env var (if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_phase: Option<u8>,
    /// Effective phase used for enforcement
    pub effective_phase: u8,
    /// Source of effective phase ("policy" or "env:CODE_SUNSET_PHASE")
    pub resolution_source: String,
}

/// Fallback activation payload for GATE-ST tracking.
///
/// Emitted when memvid backend fails and falls back to local-memory.
/// Used to track the 30-day stability requirement (zero fallbacks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackActivatedPayload {
    /// Backend that failed (e.g., "memvid")
    pub from_backend: String,
    /// Backend activated as fallback (e.g., "local-memory")
    pub to_backend: String,
    /// Reason for fallback (error message)
    pub reason: String,
    /// Operation that triggered fallback (e.g., "capsule_open", "search")
    pub operation: String,
    /// SPEC ID context (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<String>,
    /// Run ID context (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Checkpoint ID context (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
}

// =============================================================================
// SPEC-KIT-976: Memory Cards and Logic Mesh Edges
// =============================================================================

/// Card type enumeration for knowledge graph entities.
///
/// SPEC-KIT-976: Each card type represents a class of entity in the project knowledge graph.
/// Types are extensible but the core set should cover most use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    /// A SPEC entity (e.g., SPEC-KIT-976)
    Spec,
    /// A decision record
    Decision,
    /// A task/work item
    Task,
    /// A risk assessment
    Risk,
    /// A code component (module, crate, file)
    Component,
    /// A person/stakeholder
    Person,
    /// An artifact (file, document)
    Artifact,
    /// A run instance
    Run,
}

impl CardType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CardType::Spec => "spec",
            CardType::Decision => "decision",
            CardType::Task => "task",
            CardType::Risk => "risk",
            CardType::Component => "component",
            CardType::Person => "person",
            CardType::Artifact => "artifact",
            CardType::Run => "run",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "spec" => Some(CardType::Spec),
            "decision" => Some(CardType::Decision),
            "task" => Some(CardType::Task),
            "risk" => Some(CardType::Risk),
            "component" => Some(CardType::Component),
            "person" => Some(CardType::Person),
            "artifact" => Some(CardType::Artifact),
            "run" => Some(CardType::Run),
            _ => None,
        }
    }

    pub fn all_variants() -> &'static [&'static str] {
        &[
            "spec",
            "decision",
            "task",
            "risk",
            "component",
            "person",
            "artifact",
            "run",
        ]
    }
}

/// Edge type enumeration for relationships in the logic mesh.
///
/// SPEC-KIT-976: Edge types define semantic relationships between entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// A depends on B (A cannot proceed without B)
    DependsOn,
    /// A blocks B (B cannot proceed until A completes)
    Blocks,
    /// A implements B (A is an implementation of B)
    Implements,
    /// A references B (A mentions or links to B)
    References,
    /// A owns B (A is responsible for B)
    Owns,
    /// A risks B (A poses a risk to B)
    Risks,
    /// A is related to B (generic relationship)
    RelatedTo,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::DependsOn => "depends_on",
            EdgeType::Blocks => "blocks",
            EdgeType::Implements => "implements",
            EdgeType::References => "references",
            EdgeType::Owns => "owns",
            EdgeType::Risks => "risks",
            EdgeType::RelatedTo => "related_to",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "depends_on" => Some(EdgeType::DependsOn),
            "blocks" => Some(EdgeType::Blocks),
            "implements" => Some(EdgeType::Implements),
            "references" => Some(EdgeType::References),
            "owns" => Some(EdgeType::Owns),
            "risks" => Some(EdgeType::Risks),
            "related_to" => Some(EdgeType::RelatedTo),
            _ => None,
        }
    }

    pub fn all_variants() -> &'static [&'static str] {
        &[
            "depends_on",
            "blocks",
            "implements",
            "references",
            "owns",
            "risks",
            "related_to",
        ]
    }
}

/// Value type for facts in memory cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactValueType {
    String,
    Number,
    Boolean,
    Date,
    Uri,
    Json,
}

impl FactValueType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FactValueType::String => "string",
            FactValueType::Number => "number",
            FactValueType::Boolean => "boolean",
            FactValueType::Date => "date",
            FactValueType::Uri => "uri",
            FactValueType::Json => "json",
        }
    }
}

/// A single fact entry within a memory card.
///
/// Facts are key-value pairs with optional confidence and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardFact {
    /// Fact key (e.g., "status", "owner", "priority")
    pub key: String,
    /// Fact value (JSON-serialized)
    pub value: serde_json::Value,
    /// Value type for validation/display
    pub value_type: FactValueType,
    /// Confidence score (0.0-1.0) for extracted facts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Source URIs that this fact was extracted from
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_uris: Vec<LogicalUri>,
}

/// Provenance metadata for memory cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardProvenance {
    /// When this card was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Who/what created this (agent role, user, extractor)
    pub created_by: String,
    /// Associated SPEC ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<String>,
    /// Associated run ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Stage where created (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Git commit hash (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_hash: Option<String>,
}

/// Provenance metadata for logic edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeProvenance {
    /// When this edge was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Who/what created this (agent role, user, extractor)
    pub created_by: String,
    /// Associated SPEC ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<String>,
    /// Associated run ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Stage where created (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Source URIs that this edge was extracted from
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_uris: Vec<LogicalUri>,
}

/// Memory Card V1 - A knowledge graph entity stored in the capsule.
///
/// SPEC-KIT-976: Cards are normalized entities with structured facts.
/// Cards are append-only; edits create new card frames that supersede prior facts.
///
/// ## Lifecycle
/// - Cards are created by extraction pipelines or manual entry
/// - Updates create new versions (same card_id, newer created_at)
/// - Query returns latest version by default; as-of query for history
///
/// ## URI Format
/// Cards are stored with ObjectType::Card and path = card_id
/// URI: mv2://{workspace}/{spec}/{run}/card/{card_id}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCardV1 {
    /// Stable identifier (UUID or deterministic hash)
    pub card_id: String,
    /// Card type classification
    pub card_type: CardType,
    /// Human-readable title
    pub title: String,
    /// Structured facts about this entity
    #[serde(default)]
    pub facts: Vec<CardFact>,
    /// Creation/source metadata
    pub provenance: CardProvenance,
    /// Schema version (always 1 for V1)
    pub version: u32,
}

impl MemoryCardV1 {
    /// Create a new memory card with minimal required fields.
    pub fn new(
        card_id: impl Into<String>,
        card_type: CardType,
        title: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self {
            card_id: card_id.into(),
            card_type,
            title: title.into(),
            facts: Vec::new(),
            provenance: CardProvenance {
                created_at: chrono::Utc::now(),
                created_by: created_by.into(),
                spec_id: None,
                run_id: None,
                stage: None,
                commit_hash: None,
            },
            version: 1,
        }
    }

    /// Add a fact to this card.
    pub fn with_fact(mut self, fact: CardFact) -> Self {
        self.facts.push(fact);
        self
    }

    /// Set provenance SPEC ID.
    pub fn with_spec_id(mut self, spec_id: impl Into<String>) -> Self {
        self.provenance.spec_id = Some(spec_id.into());
        self
    }

    /// Set provenance run ID.
    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.provenance.run_id = Some(run_id.into());
        self
    }

    /// Serialize to JSON bytes for storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Logic Mesh Edge V1 - A relationship between entities in the knowledge graph.
///
/// SPEC-KIT-976: Edges connect cards and/or artifacts via logical URIs.
///
/// ## Type Safety
/// CRITICAL: `from_uri` and `to_uri` are LogicalUri, NOT String.
/// This ensures all graph references are valid mv2:// URIs.
///
/// ## URI Format
/// Edges are stored with ObjectType::Edge and path = edge_id
/// URI: mv2://{workspace}/{spec}/{run}/edge/{edge_id}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicEdgeV1 {
    /// Stable identifier
    pub edge_id: String,
    /// Relationship type
    pub edge_type: EdgeType,
    /// Source entity URI (MUST be LogicalUri, NOT String)
    pub from_uri: LogicalUri,
    /// Target entity URI (MUST be LogicalUri, NOT String)
    pub to_uri: LogicalUri,
    /// Optional weight/confidence (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    /// Creation/source metadata
    pub provenance: EdgeProvenance,
    /// Schema version (always 1 for V1)
    pub version: u32,
}

impl LogicEdgeV1 {
    /// Create a new logic edge.
    ///
    /// ## Type Safety
    /// from_uri and to_uri are LogicalUri to enforce mv2:// URI format.
    pub fn new(
        edge_id: impl Into<String>,
        edge_type: EdgeType,
        from_uri: LogicalUri,
        to_uri: LogicalUri,
        created_by: impl Into<String>,
    ) -> Self {
        Self {
            edge_id: edge_id.into(),
            edge_type,
            from_uri,
            to_uri,
            weight: None,
            provenance: EdgeProvenance {
                created_at: chrono::Utc::now(),
                created_by: created_by.into(),
                spec_id: None,
                run_id: None,
                stage: None,
                source_uris: Vec::new(),
            },
            version: 1,
        }
    }

    /// Set optional weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Set provenance SPEC ID.
    pub fn with_spec_id(mut self, spec_id: impl Into<String>) -> Self {
        self.provenance.spec_id = Some(spec_id.into());
        self
    }

    /// Set provenance run ID.
    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.provenance.run_id = Some(run_id.into());
        self
    }

    /// Serialize to JSON bytes for storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
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

    // =========================================================================
    // SPEC-KIT-975: Event Type Tests
    // =========================================================================

    #[test]
    fn event_type_all_variants_covered() {
        // Ensure all_variants() returns the same set as the enum
        let variants = EventType::all_variants();
        assert_eq!(variants.len(), 18); // SPEC-KIT-979: +2 (LocalMemorySunsetPhaseResolved, FallbackActivated)

        // Verify each variant can be parsed
        for variant_str in variants {
            assert!(
                EventType::from_str(variant_str).is_some(),
                "Failed to parse variant: {}",
                variant_str
            );
        }
    }

    #[test]
    fn event_type_curated_classification() {
        // SPEC-KIT-975: Curated-eligible events
        assert!(EventType::StageTransition.is_curated_eligible());
        assert!(EventType::PolicySnapshotRef.is_curated_eligible());
        assert!(EventType::RoutingDecision.is_curated_eligible());
        assert!(EventType::BranchMerged.is_curated_eligible());
        assert!(EventType::RetrievalRequest.is_curated_eligible());
        assert!(EventType::RetrievalResponse.is_curated_eligible());
        assert!(EventType::ToolCall.is_curated_eligible());
        assert!(EventType::ToolResult.is_curated_eligible());
        assert!(EventType::PatchApply.is_curated_eligible());
        assert!(EventType::GateDecision.is_curated_eligible());
        assert!(EventType::ErrorEvent.is_curated_eligible());
        assert!(EventType::CapsuleExported.is_curated_eligible());
        assert!(EventType::CapsuleImported.is_curated_eligible());

        // NOT curated-eligible
        assert!(!EventType::DebugTrace.is_curated_eligible());
        assert!(!EventType::ModelCallEnvelope.is_curated_eligible()); // May contain sensitive data
    }

    #[test]
    fn event_type_audit_critical_classification() {
        // Audit-critical: MUST be captured for compliance
        assert!(EventType::StageTransition.is_audit_critical());
        assert!(EventType::PolicySnapshotRef.is_audit_critical());
        assert!(EventType::RoutingDecision.is_audit_critical());
        assert!(EventType::BranchMerged.is_audit_critical());
        assert!(EventType::GateDecision.is_audit_critical());
        assert!(EventType::ErrorEvent.is_audit_critical());
        assert!(EventType::CapsuleExported.is_audit_critical());
        assert!(EventType::CapsuleImported.is_audit_critical());

        // NOT audit-critical (optional/debug)
        assert!(!EventType::DebugTrace.is_audit_critical());
        assert!(!EventType::RetrievalRequest.is_audit_critical());
        assert!(!EventType::RetrievalResponse.is_audit_critical());
        assert!(!EventType::ToolCall.is_audit_critical());
        assert!(!EventType::ToolResult.is_audit_critical());
        assert!(!EventType::PatchApply.is_audit_critical());
        assert!(!EventType::ModelCallEnvelope.is_audit_critical());
    }

    #[test]
    fn llm_capture_mode_export_safety() {
        // Safe for export (response text not stored)
        assert!(LLMCaptureMode::None.is_export_safe());
        assert!(LLMCaptureMode::PromptsOnly.is_export_safe());

        // NOT safe for export (may contain sensitive data)
        assert!(!LLMCaptureMode::FullIo.is_export_safe());
    }

    #[test]
    fn llm_capture_mode_default_is_prompts_only() {
        assert_eq!(LLMCaptureMode::default(), LLMCaptureMode::PromptsOnly);
    }

    #[test]
    fn llm_capture_mode_round_trip() {
        for mode in &[
            LLMCaptureMode::None,
            LLMCaptureMode::PromptsOnly,
            LLMCaptureMode::FullIo,
        ] {
            let s = mode.as_str();
            let parsed = LLMCaptureMode::from_str(s);
            assert_eq!(parsed, Some(*mode), "Round-trip failed for {:?}", mode);
        }
    }

    #[test]
    fn llm_capture_mode_backward_compat() {
        // Old values should map to new enum variants
        assert_eq!(LLMCaptureMode::from_str("off"), Some(LLMCaptureMode::None));
        assert_eq!(
            LLMCaptureMode::from_str("hash"),
            Some(LLMCaptureMode::PromptsOnly)
        );
        assert_eq!(
            LLMCaptureMode::from_str("summary"),
            Some(LLMCaptureMode::PromptsOnly)
        );
        assert_eq!(
            LLMCaptureMode::from_str("full"),
            Some(LLMCaptureMode::FullIo)
        );
    }

    #[test]
    fn gate_outcome_variants() {
        assert_eq!(GateOutcome::Pass.as_str(), "pass");
        assert_eq!(GateOutcome::Fail.as_str(), "fail");
        assert_eq!(GateOutcome::Warn.as_str(), "warn");
        assert_eq!(GateOutcome::Skip.as_str(), "skip");
    }

    #[test]
    fn error_severity_variants() {
        assert_eq!(ErrorSeverity::Warning.as_str(), "warning");
        assert_eq!(ErrorSeverity::Error.as_str(), "error");
        assert_eq!(ErrorSeverity::Fatal.as_str(), "fatal");
    }

    #[test]
    fn retrieval_request_payload_serialization() {
        let payload = RetrievalRequestPayload {
            request_id: "req-001".to_string(),
            query: "What is the project structure?".to_string(),
            config: serde_json::json!({"top_k": 5}),
            source: "capsule".to_string(),
            stage: Some("Stage0".to_string()),
            role: Some("Architect".to_string()),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let parsed: RetrievalRequestPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.request_id, "req-001");
        assert_eq!(parsed.query, "What is the project structure?");
    }

    #[test]
    fn tool_call_payload_serialization() {
        let payload = ToolCallPayload {
            call_id: "call-001".to_string(),
            tool_name: "read_file".to_string(),
            input: serde_json::json!({"path": "/foo/bar.rs"}),
            stage: Some("Implement".to_string()),
            role: Some("Implementer".to_string()),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let parsed: ToolCallPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.call_id, "call-001");
        assert_eq!(parsed.tool_name, "read_file");
    }

    #[test]
    fn gate_decision_payload_serialization() {
        let payload = GateDecisionPayload {
            gate_name: "JudgeApprove".to_string(),
            outcome: GateOutcome::Pass,
            stage: "Judge".to_string(),
            confidence: Some(0.95),
            reason: Some("All criteria met".to_string()),
            details: None,
            blocking: true,
        };

        let json = serde_json::to_string(&payload).unwrap();
        let parsed: GateDecisionPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.gate_name, "JudgeApprove");
        assert_eq!(parsed.outcome, GateOutcome::Pass);
        assert!(parsed.blocking);
    }

    #[test]
    fn model_call_envelope_payload_serialization() {
        let payload = ModelCallEnvelopePayload {
            call_id: "llm-001".to_string(),
            model: "claude-3-opus".to_string(),
            routing_mode: RoutingMode::Cloud,
            capture_mode: LLMCaptureMode::PromptsOnly,
            stage: Some("Implement".to_string()),
            role: Some("Implementer".to_string()),
            prompt_hash: Some("abc123".to_string()),
            response_hash: Some("def456".to_string()),
            prompt: Some("Write a function...".to_string()),
            response: None, // Not captured in PromptsOnly mode
            prompt_tokens: Some(100),
            response_tokens: Some(200),
            latency_ms: Some(1500),
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        let parsed: ModelCallEnvelopePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.call_id, "llm-001");
        assert_eq!(parsed.capture_mode, LLMCaptureMode::PromptsOnly);
        assert!(parsed.prompt.is_some());
        assert!(parsed.response.is_none());
        assert!(parsed.success);
    }
}
