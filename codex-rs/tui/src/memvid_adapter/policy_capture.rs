//! SPEC-KIT-977: PolicySnapshot integration with Memvid capsule
//!
//! Integrates PolicySnapshot capture with capsule storage for traceability.
//!
//! ## Decision IDs
//! - D100: JSON format compiled from human-readable source
//! - D101: Dual storage (filesystem + capsule)
//! - D102: Events tagged with policy_id for traceability
//!
//! ## Drift Detection (SPEC-KIT-977)
//!
//! Policy drift detection relies on `CapsuleHandle.current_policy`, but this
//! is not persisted across `CapsuleHandle::open()` calls. To fix this,
//! `latest_policy_ref_for_run()` derives the current policy from capsule
//! events (PolicySnapshotRef) so drift detection works across reopens.

use super::capsule::{CapsuleHandle, CurrentPolicyInfo, Result as CapsuleResult};
use super::types::{EventType, LogicalUri};
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

/// SPEC-KIT-977: Derive current policy from capsule events.
///
/// This function scans the capsule's event track for PolicySnapshotRef events
/// matching the given spec_id/run_id and returns the latest one. This enables
/// drift detection to work correctly across `CapsuleHandle::open()` calls.
///
/// ## Parameters
/// - `handle`: Open capsule handle
/// - `spec_id`: SPEC identifier to filter events
/// - `run_id`: Run identifier to filter events
///
/// ## Returns
/// - `Some(CurrentPolicyInfo)` if a PolicySnapshotRef event is found
/// - `None` if no matching events exist
///
/// ## Usage
/// ```ignore
/// // After opening capsule, restore policy state
/// if let Some(policy_info) = latest_policy_ref_for_run(&handle, spec_id, run_id) {
///     handle.set_current_policy(&policy_info.policy_id, &policy_info.hash, &policy_info.uri);
/// }
/// ```
pub fn latest_policy_ref_for_run(
    handle: &CapsuleHandle,
    spec_id: &str,
    run_id: &str,
) -> Option<CurrentPolicyInfo> {
    let events = handle.list_events();

    // Filter to PolicySnapshotRef events for this spec/run
    let policy_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.event_type == EventType::PolicySnapshotRef
                && e.spec_id == spec_id
                && e.run_id == run_id
        })
        .collect();

    if policy_events.is_empty() {
        return None;
    }

    // Find the latest event (by timestamp)
    let latest = policy_events.iter().max_by_key(|e| e.timestamp)?;

    // Extract policy info from payload
    let policy_uri_str = latest.payload.get("policy_uri")?.as_str()?;
    let policy_id = latest.payload.get("policy_id")?.as_str()?;
    let policy_hash = latest.payload.get("policy_hash")?.as_str()?;

    // Parse the URI
    let uri: LogicalUri = policy_uri_str.parse().ok()?;

    Some(CurrentPolicyInfo {
        policy_id: policy_id.to_string(),
        hash: policy_hash.to_string(),
        uri,
    })
}

/// Restore policy state from capsule events if not already set.
///
/// SPEC-KIT-977: This should be called after opening a capsule handle
/// to ensure drift detection works correctly across reopens.
///
/// ## Returns
/// - `true` if policy was restored from events
/// - `false` if no policy was found or policy was already set
pub fn restore_policy_from_events(handle: &CapsuleHandle, spec_id: &str, run_id: &str) -> bool {
    // Skip if policy is already set
    if handle.current_policy().is_some() {
        return false;
    }

    // Try to restore from events
    if let Some(policy_info) = latest_policy_ref_for_run(handle, spec_id, run_id) {
        handle.set_current_policy(&policy_info.policy_id, &policy_info.hash, &policy_info.uri);
        tracing::debug!(
            policy_id = %policy_info.policy_id,
            hash = %policy_info.hash,
            "Restored policy state from capsule events"
        );
        true
    } else {
        false
    }
}

/// Check if policy has drifted since last capture, recapture if needed.
///
/// SPEC-KIT-977: Stage Boundary Policy Drift Detection
///
/// At stage boundary/manual commit: if policy hash differs from last captured
/// hash, capture new snapshot before checkpoint.
///
/// ## Returns
/// - `Ok(None)` if no drift detected (current policy is still valid)
/// - `Ok(Some(snapshot))` if drift detected and new policy was captured
/// - `Err(_)` on capsule/storage errors
///
/// ## Usage
/// ```ignore
/// // Before commit_stage
/// if let Some(new_policy) = check_and_recapture_if_changed(&handle, &config, spec_id, run_id)? {
///     tracing::info!(policy_id = %new_policy.policy_id, "Policy drift detected, recaptured");
/// }
/// ```
pub fn check_and_recapture_if_changed(
    handle: &CapsuleHandle,
    config: &Stage0Config,
    spec_id: &str,
    run_id: &str,
) -> CapsuleResult<Option<PolicySnapshot>> {
    // SPEC-KIT-977: First try to restore policy state from capsule events.
    // This is needed because CapsuleHandle.current_policy is not persisted
    // across open() calls, so we derive it from PolicySnapshotRef events.
    let _ = restore_policy_from_events(handle, spec_id, run_id);

    // Get current policy info from handle (may have just been restored)
    let current_policy = match handle.current_policy() {
        Some(p) => p,
        None => {
            // No policy captured yet - do initial capture
            let snapshot = capture_and_store_policy(handle, config, spec_id, run_id)?;
            return Ok(Some(snapshot));
        }
    };

    // Capture a fresh snapshot to compare (don't store yet)
    let fresh_snapshot = capture_policy_snapshot(config);

    // Check if content has changed using deterministic hash comparison
    if fresh_snapshot.hash == current_policy.hash {
        // No drift - policy unchanged
        tracing::debug!(
            current_hash = %current_policy.hash,
            "Policy unchanged at stage boundary"
        );
        return Ok(None);
    }

    // Drift detected! Capture and store the new policy
    tracing::info!(
        old_hash = %current_policy.hash,
        new_hash = %fresh_snapshot.hash,
        "Policy drift detected at stage boundary, recapturing"
    );

    // Store to filesystem (D101: dual storage)
    let store = PolicyStore::new();
    if let Err(e) = store.store(&fresh_snapshot) {
        tracing::warn!(
            error = %e,
            policy_id = %fresh_snapshot.policy_id,
            "Failed to store drifted policy snapshot to filesystem"
        );
    }

    // Store to capsule using global policy URI
    let policy_json = fresh_snapshot.to_json().unwrap_or_default();
    let policy_uri = handle.put_policy(
        &fresh_snapshot.policy_id,
        &fresh_snapshot.hash,
        policy_json.into_bytes(),
        serde_json::json!({
            "schema_version": fresh_snapshot.schema_version,
            "hash": fresh_snapshot.hash,
            "created_at": fresh_snapshot.created_at.to_rfc3339(),
            "drift_from": current_policy.policy_id,
        }),
    )?;

    // Emit PolicySnapshotRef event for the new policy
    handle.emit_policy_snapshot_ref_with_info(
        spec_id,
        run_id,
        None, // Stage will be set by commit_stage
        &policy_uri,
        &fresh_snapshot.policy_id,
        &fresh_snapshot.hash,
    )?;

    // Update current policy in handle
    handle.set_current_policy(&fresh_snapshot.policy_id, &fresh_snapshot.hash, &policy_uri);

    Ok(Some(fresh_snapshot))
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

    /// SPEC-KIT-977: Test that latest_policy_ref_for_run finds policy from events
    #[test]
    fn test_latest_policy_ref_for_run() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let capsule_path = temp_dir.path().join("test_policy_ref.mv2");

        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "test".to_string(),
            ..Default::default()
        };

        // Create capsule and capture policy
        let handle = CapsuleHandle::open(config.clone()).expect("open capsule");
        let stage0_config = Stage0Config::default();

        let snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-977", "run-001")
            .expect("capture policy");

        // Verify policy is in handle
        let current = handle.current_policy().expect("should have current policy");
        assert_eq!(current.policy_id, snapshot.policy_id);

        // Drop handle to simulate reopen
        drop(handle);

        // Reopen capsule - current_policy will be None
        let handle2 = CapsuleHandle::open(config).expect("reopen capsule");
        assert!(
            handle2.current_policy().is_none(),
            "Fresh handle should have no current_policy"
        );

        // Use latest_policy_ref_for_run to find the policy
        let found = latest_policy_ref_for_run(&handle2, "SPEC-977", "run-001");
        assert!(found.is_some(), "Should find policy from events");

        let found_policy = found.unwrap();
        assert_eq!(found_policy.policy_id, snapshot.policy_id);
        assert_eq!(found_policy.hash, snapshot.hash);
    }

    /// SPEC-KIT-977: Test drift detection works across capsule reopens
    #[test]
    fn test_drift_detection_across_reopens() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let capsule_path = temp_dir.path().join("test_drift.mv2");

        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "test".to_string(),
            ..Default::default()
        };

        // Create capsule and capture policy
        let handle = CapsuleHandle::open(config.clone()).expect("open capsule");
        let stage0_config = Stage0Config::default();

        let snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-977", "run-drift")
            .expect("capture policy");

        // Drop handle
        drop(handle);

        // Reopen capsule (simulates stage boundary)
        let handle2 = CapsuleHandle::open(config).expect("reopen capsule");

        // Current policy should be None on fresh open
        assert!(
            handle2.current_policy().is_none(),
            "Fresh handle should have no current_policy"
        );

        // Call check_and_recapture_if_changed - it should restore from events
        // and return None (no drift) since config hasn't changed
        let result =
            check_and_recapture_if_changed(&handle2, &stage0_config, "SPEC-977", "run-drift");
        assert!(result.is_ok());

        // Should return None (no recapture needed) because policy was restored
        // and hash matches
        let recaptured = result.unwrap();
        assert!(
            recaptured.is_none(),
            "Should NOT recapture when config unchanged - policy_id: {}, hash: {}",
            snapshot.policy_id,
            snapshot.hash
        );

        // Verify current_policy was restored
        let restored = handle2.current_policy().expect("policy should be restored");
        assert_eq!(restored.policy_id, snapshot.policy_id);
        assert_eq!(restored.hash, snapshot.hash);
    }

    /// SPEC-KIT-977: Test restore_policy_from_events helper
    #[test]
    fn test_restore_policy_from_events() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let capsule_path = temp_dir.path().join("test_restore.mv2");

        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "test".to_string(),
            ..Default::default()
        };

        // Create capsule and capture policy
        let handle = CapsuleHandle::open(config.clone()).expect("open capsule");
        let stage0_config = Stage0Config::default();

        let snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-977", "run-restore")
            .expect("capture policy");

        drop(handle);

        // Reopen and test restore
        let handle2 = CapsuleHandle::open(config).expect("reopen capsule");
        assert!(handle2.current_policy().is_none());

        // Restore should succeed
        let restored = restore_policy_from_events(&handle2, "SPEC-977", "run-restore");
        assert!(restored, "Should restore policy from events");

        // Verify restored policy
        let current = handle2.current_policy().expect("should have policy");
        assert_eq!(current.policy_id, snapshot.policy_id);

        // Calling restore again should return false (already set)
        let restored_again = restore_policy_from_events(&handle2, "SPEC-977", "run-restore");
        assert!(!restored_again, "Should not restore when already set");
    }

    /// SPEC-KIT-977: Test that wrong spec/run doesn't restore wrong policy
    #[test]
    fn test_restore_policy_filtering() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let capsule_path = temp_dir.path().join("test_filter.mv2");

        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config.clone()).expect("open capsule");
        let stage0_config = Stage0Config::default();

        // Capture policy for run-A
        capture_and_store_policy(&handle, &stage0_config, "SPEC-977", "run-A")
            .expect("capture policy A");

        drop(handle);

        // Reopen and try to restore for run-B (different run)
        let handle2 = CapsuleHandle::open(config).expect("reopen capsule");

        let found = latest_policy_ref_for_run(&handle2, "SPEC-977", "run-B");
        assert!(found.is_none(), "Should not find policy for different run");

        let found_wrong_spec = latest_policy_ref_for_run(&handle2, "SPEC-OTHER", "run-A");
        assert!(
            found_wrong_spec.is_none(),
            "Should not find policy for different spec"
        );
    }
}
