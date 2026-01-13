//! SPEC-KIT-977: PolicySnapshot integration with Memvid capsule
//!
//! Integrates PolicySnapshot capture with capsule storage for traceability.
//!
//! ## Decision IDs
//! - D100: JSON format compiled from human-readable source
//! - D101: Dual storage (filesystem + capsule)
//! - D102: Events tagged with policy_id for traceability

use super::capsule::{CapsuleHandle, Result as CapsuleResult};
use super::types::LogicalUri;
use codex_stage0::{PolicySnapshot, PolicyStore, Stage0Config, capture_policy_snapshot};

/// Capture and store a policy snapshot for a run.
///
/// This is the main entry point called at run start. It:
/// 1. Captures the active policy configuration
/// 2. Stores to filesystem (.speckit/policies/)
/// 3. Stores to capsule (mv2://<workspace>/policy/<ID>) - global URI
/// 4. Emits PolicySnapshotRef event
///
/// ## SPEC-KIT-977: Dual Storage + Event Binding
/// - Filesystem: `.speckit/policies/snapshot-<POLICY_ID>.json`
/// - Capsule: `mv2://<workspace>/policy/<POLICY_ID>` (global, not spec/run scoped)
/// - Event: `PolicySnapshotRef` with policy_uri, policy_id, policy_hash
///
/// Returns the captured PolicySnapshot for event tagging.
pub fn capture_and_store_policy(
    handle: &CapsuleHandle,
    config: &Stage0Config,
    spec_id: &str,
    run_id: &str,
) -> CapsuleResult<PolicySnapshot> {
    // 1. Capture the policy snapshot
    let snapshot = capture_policy_snapshot(config);

    // 2. Store to filesystem (D101: dual storage)
    let store = PolicyStore::new();
    if let Err(e) = store.store(&snapshot) {
        tracing::warn!(
            error = %e,
            policy_id = %snapshot.policy_id,
            "Failed to store policy snapshot to filesystem"
        );
        // Continue - capsule storage is the primary
    }

    // 3. Store to capsule using global policy URI (SPEC-KIT-977)
    // URI: mv2://<workspace>/policy/<policy_id> (not spec/run scoped)
    let policy_json = snapshot.to_json().unwrap_or_default();
    let policy_uri = handle.put_policy(
        &snapshot.policy_id,
        &snapshot.hash,
        policy_json.into_bytes(),
        serde_json::json!({
            "schema_version": snapshot.schema_version,
            "hash": snapshot.hash,
            "created_at": snapshot.created_at.to_rfc3339(),
        }),
    )?;

    // 4. Emit PolicySnapshotRef event with full policy info (D102: events tagged)
    handle.emit_policy_snapshot_ref_with_info(
        spec_id,
        run_id,
        None,
        &policy_uri,
        &snapshot.policy_id,
        &snapshot.hash,
    )?;

    tracing::info!(
        policy_id = %snapshot.policy_id,
        hash = %snapshot.hash,
        uri = %policy_uri,
        "Captured and stored policy snapshot"
    );

    Ok(snapshot)
}

/// Get the policy URI for a snapshot.
pub fn policy_uri(workspace_id: &str, policy_id: &str) -> LogicalUri {
    LogicalUri::for_policy(workspace_id, policy_id)
}

/// Load policy snapshot from capsule by URI.
///
/// Returns None if the policy is not found or cannot be parsed.
pub fn load_policy_from_capsule(
    _handle: &CapsuleHandle,
    _uri: &LogicalUri,
) -> Option<PolicySnapshot> {
    // TODO: Implement when capsule read is available
    // For now, fall back to filesystem
    None
}

/// Load policy snapshot by ID, trying capsule first then filesystem.
pub fn load_policy(policy_id: &str) -> Option<PolicySnapshot> {
    // Try filesystem
    let store = PolicyStore::new();
    store.load(policy_id).ok()
}

/// List all available policy snapshots.
pub fn list_policies() -> Vec<codex_stage0::PolicySnapshotInfo> {
    let store = PolicyStore::new();
    store.list().unwrap_or_default()
}

/// Get the latest policy snapshot.
pub fn latest_policy() -> Option<PolicySnapshot> {
    let store = PolicyStore::new();
    store.latest().ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memvid_adapter::CapsuleConfig;

    #[test]
    fn test_capture_and_store_policy() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("open capsule");
        let stage0_config = Stage0Config::default();

        let result = capture_and_store_policy(&handle, &stage0_config, "SPEC-TEST", "run-001");

        assert!(result.is_ok());
        let snapshot = result.unwrap();
        assert!(!snapshot.policy_id.is_empty());
        assert_eq!(snapshot.schema_version, "1.0");
    }

    #[test]
    fn test_policy_uri_generation() {
        let uri = policy_uri("workspace1", "policy-abc123");
        assert!(uri.as_str().contains("policy"));
        assert!(uri.as_str().contains("policy-abc123"));
    }
}
