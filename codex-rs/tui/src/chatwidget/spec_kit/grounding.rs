//! SPEC-KIT Phase 3B: Deep Grounding Capture
//!
//! Captures grounding artifacts (Architect Harvest + Stage0 Project Intel) during
//! deep intake flows and persists them to capsule for replay/audit linking.
//!
//! ## Design Decisions
//! - Correctness-first: Run grounding BEFORE persisting DesignBrief/ProjectBrief
//!   so grounding_uris[] is populated in the SoR object
//! - Failure blocks: Grounding failure in deep mode blocks completion (SoR integrity)
//! - Progress feedback: TUI receives progress updates to avoid frozen UX
//!
//! ## Artifact URIs
//! - Spec: mv2://default/<spec_id>/<intake_id>/artifact/intake/grounding/...
//! - Project: mv2://default/project/<project_id>/artifact/intake/grounding/...

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use serde::{Deserialize, Serialize};

use crate::memvid_adapter::{
    CapsuleConfig, CapsuleHandle, DEFAULT_CAPSULE_RELATIVE_PATH, DEFAULT_WORKSPACE_ID, ObjectType,
};
use codex_core::architect::{HarvestResults, HarvesterConfig, run_harvest};
use codex_stage0::project_intel::{ProjectSnapshot, ProjectSnapshotBuilder, SnapshotConfig};

use super::intake::sha256_hex;

// =============================================================================
// Constants
// =============================================================================

/// Schema version for grounding artifacts
pub const GROUNDING_ARTIFACT_SCHEMA_VERSION: &str = "grounding_artifact@1.0";

// =============================================================================
// Progress Types
// =============================================================================

/// Progress updates for TUI feedback during grounding capture.
#[derive(Debug, Clone)]
pub enum GroundingProgress {
    /// Grounding capture starting
    Starting,
    /// Running Architect Harvest (churn, complexity, skeleton)
    RunningArchitectHarvest,
    /// Architect Harvest complete
    ArchitectHarvestComplete { files: usize },
    /// Running Project Intel snapshot
    RunningProjectIntel,
    /// Project Intel complete
    ProjectIntelComplete { feeds: usize },
    /// Persisting artifacts to capsule
    PersistingToCapsule,
    /// Grounding capture complete
    Complete { artifact_count: usize },
    /// Grounding capture failed
    Failed { reason: String },
}

// =============================================================================
// Result Types
// =============================================================================

/// Summary of Architect Harvest artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestSummary {
    /// URI of churn_matrix.md
    pub churn_matrix_uri: String,
    /// SHA256 of churn_matrix.md
    pub churn_matrix_sha256: String,
    /// Number of files analyzed for churn
    pub churn_files: usize,
    /// URI of complexity_map.json
    pub complexity_map_uri: String,
    /// SHA256 of complexity_map.json
    pub complexity_map_sha256: String,
    /// Number of files analyzed for complexity
    pub complexity_files: usize,
    /// URI of repo_skeleton.xml
    pub repo_skeleton_uri: String,
    /// SHA256 of repo_skeleton.xml
    pub repo_skeleton_sha256: String,
    /// Number of files in skeleton
    pub skeleton_files: usize,
}

/// Summary of Project Intel artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIntelSummary {
    /// URI of project_snapshot.json
    pub snapshot_uri: String,
    /// SHA256 of project_snapshot.json
    pub snapshot_sha256: String,
    /// URIs of markdown feeds (6 feeds)
    pub feed_uris: Vec<String>,
    /// SHA256s of feeds (parallel to feed_uris)
    pub feed_sha256s: Vec<String>,
    /// Names of feeds (parallel to feed_uris)
    pub feed_names: Vec<String>,
}

/// Result from grounding capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingCaptureResult {
    /// URIs of all grounding artifacts stored in capsule (for linking in briefs)
    pub grounding_uris: Vec<String>,
    /// SHA256 hashes of each artifact (parallel to grounding_uris)
    pub artifact_hashes: Vec<String>,
    /// Harvest results (if architect harvest succeeded)
    pub harvest: Option<HarvestSummary>,
    /// Project intel results (if project intel succeeded)
    pub project_intel: Option<ProjectIntelSummary>,
}

// =============================================================================
// Internal Types
// =============================================================================

/// Raw artifacts from Architect Harvest (before capsule persistence).
struct HarvestArtifacts {
    churn_matrix: Vec<u8>,
    complexity_map: Vec<u8>,
    repo_skeleton: Vec<u8>,
    results: HarvestResults,
}

/// Raw artifacts from Project Intel (before capsule persistence).
struct ProjectIntelArtifacts {
    snapshot_json: Vec<u8>,
    feeds: Vec<(String, Vec<u8>)>, // (name, content)
}

// =============================================================================
// Feed Names
// =============================================================================

/// Canonical feed names for Project Intel.
const PROJECT_INTEL_FEEDS: &[&str] = &[
    "code_topology.md",
    "speckit_workflows.md",
    "specs_and_phases.md",
    "governance_and_drift.md",
    "memory_and_librarian.md",
    "session_lineage.md",
];

// =============================================================================
// Main Capture Function
// =============================================================================

/// Capture grounding artifacts for deep intake.
///
/// This is the main entry point called from intake handlers.
/// Runs both Architect Harvest and Project Intel Snapshot, persists to capsule,
/// and returns URIs for linking in the brief.
///
/// ## Arguments
/// * `cwd` - Working directory (repo root)
/// * `spec_id` - SPEC-ID (for spec intake) or "project" (for project intake)
/// * `run_id` - Intake ID (UUID for spec) or project_id (for project)
/// * `progress_tx` - Optional channel for progress updates
///
/// ## Returns
/// * `Ok(GroundingCaptureResult)` with URIs and summaries
/// * `Err(String)` on any failure (grounding failure blocks deep intake)
pub async fn capture_grounding_artifacts(
    cwd: &Path,
    spec_id: &str,
    run_id: &str,
    progress_tx: Option<mpsc::Sender<GroundingProgress>>,
) -> Result<GroundingCaptureResult, String> {
    let send_progress = |p: GroundingProgress| {
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(p);
        }
    };

    send_progress(GroundingProgress::Starting);

    // Create temp dir for harvest outputs
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp dir for grounding: {}", e))?;

    // ---------------------------------------------------------------------
    // Step 1: Run Architect Harvest
    // ---------------------------------------------------------------------
    send_progress(GroundingProgress::RunningArchitectHarvest);

    let harvest_artifacts = run_architect_harvest_internal(cwd, temp_dir.path()).await?;
    let harvest_file_count = harvest_artifacts.results.churn_files
        + harvest_artifacts.results.complexity_files
        + harvest_artifacts.results.skeleton_files;

    send_progress(GroundingProgress::ArchitectHarvestComplete {
        files: harvest_file_count,
    });

    // ---------------------------------------------------------------------
    // Step 2: Run Project Intel Snapshot
    // ---------------------------------------------------------------------
    send_progress(GroundingProgress::RunningProjectIntel);

    let intel_artifacts = run_project_intel_internal(cwd)?;
    let feed_count = intel_artifacts.feeds.len();

    send_progress(GroundingProgress::ProjectIntelComplete { feeds: feed_count });

    // ---------------------------------------------------------------------
    // Step 3: Persist to Capsule
    // ---------------------------------------------------------------------
    send_progress(GroundingProgress::PersistingToCapsule);

    let result =
        persist_grounding_to_capsule(cwd, spec_id, run_id, &harvest_artifacts, &intel_artifacts)?;

    let artifact_count = result.grounding_uris.len();
    send_progress(GroundingProgress::Complete { artifact_count });

    Ok(result)
}

/// Capture grounding artifacts for spec intake (synchronous wrapper).
///
/// This is the convenience wrapper for spec_intake_handler.rs.
/// Blocks on async execution.
pub fn capture_grounding_for_spec_intake(
    cwd: &Path,
    spec_id: &str,
    intake_id: &str,
) -> Result<GroundingCaptureResult, String> {
    // Build a mini runtime for the async call
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

    rt.block_on(capture_grounding_artifacts(cwd, spec_id, intake_id, None))
}

/// Capture grounding artifacts for project intake (synchronous wrapper).
///
/// This is the convenience wrapper for project_intake_handler.rs.
/// Blocks on async execution.
pub fn capture_grounding_for_project_intake(
    cwd: &Path,
    project_id: &str,
) -> Result<GroundingCaptureResult, String> {
    // For project intake, spec_id is "project" and run_id is project_id
    capture_grounding_for_spec_intake(cwd, "project", project_id)
}

// =============================================================================
// Async Spawn (for TUI progress)
// =============================================================================

/// Pending operation handle for async grounding capture.
pub struct GroundingPendingOperation {
    /// Channel to receive progress updates
    pub progress_rx: mpsc::Receiver<GroundingProgress>,
    /// Channel to receive final result
    pub result_rx: mpsc::Receiver<Result<GroundingCaptureResult, String>>,
}

/// Spawn grounding capture in a background thread.
///
/// Returns immediately with channels for progress and result.
/// Use this when you need non-blocking grounding with progress feedback.
pub fn spawn_grounding_async(
    cwd: PathBuf,
    spec_id: String,
    run_id: String,
) -> GroundingPendingOperation {
    let (progress_tx, progress_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime for grounding");

        let result = rt.block_on(capture_grounding_artifacts(
            &cwd,
            &spec_id,
            &run_id,
            Some(progress_tx),
        ));

        let _ = result_tx.send(result);
    });

    GroundingPendingOperation {
        progress_rx,
        result_rx,
    }
}

// =============================================================================
// Internal: Architect Harvest
// =============================================================================

async fn run_architect_harvest_internal(
    cwd: &Path,
    output_dir: &Path,
) -> Result<HarvestArtifacts, String> {
    let config = HarvesterConfig::new();

    let results = run_harvest(cwd, output_dir, &config)
        .await
        .map_err(|e| format!("Architect harvest failed: {}", e))?;

    // Read the generated files
    let churn_matrix = if let Some(ref path) = results.churn_path {
        tokio::fs::read(path)
            .await
            .map_err(|e| format!("Failed to read churn_matrix: {}", e))?
    } else {
        return Err("Churn analysis produced no output".to_string());
    };

    let complexity_map = if let Some(ref path) = results.complexity_path {
        tokio::fs::read(path)
            .await
            .map_err(|e| format!("Failed to read complexity_map: {}", e))?
    } else {
        return Err("Complexity analysis produced no output".to_string());
    };

    let repo_skeleton = if let Some(ref path) = results.skeleton_path {
        tokio::fs::read(path)
            .await
            .map_err(|e| format!("Failed to read repo_skeleton: {}", e))?
    } else {
        return Err("Skeleton extraction produced no output".to_string());
    };

    Ok(HarvestArtifacts {
        churn_matrix,
        complexity_map,
        repo_skeleton,
        results,
    })
}

// =============================================================================
// Internal: Project Intel
// =============================================================================

fn run_project_intel_internal(cwd: &Path) -> Result<ProjectIntelArtifacts, String> {
    let config = SnapshotConfig::default();
    let builder = ProjectSnapshotBuilder::new(config, "codex-rs");

    let snapshot = builder
        .build()
        .map_err(|e| format!("Project intel snapshot failed: {}", e))?;

    // Serialize snapshot to JSON
    let snapshot_json = serde_json::to_vec_pretty(&snapshot)
        .map_err(|e| format!("Failed to serialize snapshot: {}", e))?;

    // Generate feeds
    let feeds = generate_project_intel_feeds(&snapshot);

    Ok(ProjectIntelArtifacts {
        snapshot_json,
        feeds,
    })
}

fn generate_project_intel_feeds(snapshot: &ProjectSnapshot) -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "code_topology.md".to_string(),
            snapshot.code_topology_md().into_bytes(),
        ),
        (
            "speckit_workflows.md".to_string(),
            snapshot.workflows_md().into_bytes(),
        ),
        (
            "specs_and_phases.md".to_string(),
            snapshot.specs_md().into_bytes(),
        ),
        (
            "governance_and_drift.md".to_string(),
            snapshot.governance_md().into_bytes(),
        ),
        (
            "memory_and_librarian.md".to_string(),
            snapshot.memory_md().into_bytes(),
        ),
        (
            "session_lineage.md".to_string(),
            snapshot.sessions_md().into_bytes(),
        ),
    ]
}

// =============================================================================
// Internal: Capsule Persistence
// =============================================================================

fn persist_grounding_to_capsule(
    cwd: &Path,
    spec_id: &str,
    run_id: &str,
    harvest: &HarvestArtifacts,
    intel: &ProjectIntelArtifacts,
) -> Result<GroundingCaptureResult, String> {
    // Open capsule
    let capsule_path = cwd.join(DEFAULT_CAPSULE_RELATIVE_PATH);
    let config = CapsuleConfig {
        capsule_path,
        workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
        ..Default::default()
    };

    let capsule = CapsuleHandle::open(config).map_err(|e| format!("Capsule open failed: {}", e))?;

    let mut grounding_uris = Vec::new();
    let mut artifact_hashes = Vec::new();

    // -------------------------------------------------------------------------
    // Persist Architect Harvest artifacts
    // -------------------------------------------------------------------------

    // Churn matrix
    let churn_sha256 = sha256_hex(&harvest.churn_matrix);
    let churn_meta = serde_json::json!({
        "schema_version": GROUNDING_ARTIFACT_SCHEMA_VERSION,
        "sha256": &churn_sha256,
        "artifact_type": "churn_matrix",
    });
    let churn_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/grounding/harvest/churn_matrix.md",
            harvest.churn_matrix.clone(),
            churn_meta,
        )
        .map_err(|e| format!("Capsule put churn_matrix failed: {}", e))?
        .to_string();
    grounding_uris.push(churn_uri.clone());
    artifact_hashes.push(churn_sha256.clone());

    // Complexity map
    let complexity_sha256 = sha256_hex(&harvest.complexity_map);
    let complexity_meta = serde_json::json!({
        "schema_version": GROUNDING_ARTIFACT_SCHEMA_VERSION,
        "sha256": &complexity_sha256,
        "artifact_type": "complexity_map",
    });
    let complexity_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/grounding/harvest/complexity_map.json",
            harvest.complexity_map.clone(),
            complexity_meta,
        )
        .map_err(|e| format!("Capsule put complexity_map failed: {}", e))?
        .to_string();
    grounding_uris.push(complexity_uri.clone());
    artifact_hashes.push(complexity_sha256.clone());

    // Repo skeleton
    let skeleton_sha256 = sha256_hex(&harvest.repo_skeleton);
    let skeleton_meta = serde_json::json!({
        "schema_version": GROUNDING_ARTIFACT_SCHEMA_VERSION,
        "sha256": &skeleton_sha256,
        "artifact_type": "repo_skeleton",
    });
    let skeleton_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/grounding/harvest/repo_skeleton.xml",
            harvest.repo_skeleton.clone(),
            skeleton_meta,
        )
        .map_err(|e| format!("Capsule put repo_skeleton failed: {}", e))?
        .to_string();
    grounding_uris.push(skeleton_uri.clone());
    artifact_hashes.push(skeleton_sha256.clone());

    let harvest_summary = HarvestSummary {
        churn_matrix_uri: churn_uri,
        churn_matrix_sha256: churn_sha256,
        churn_files: harvest.results.churn_files,
        complexity_map_uri: complexity_uri,
        complexity_map_sha256: complexity_sha256,
        complexity_files: harvest.results.complexity_files,
        repo_skeleton_uri: skeleton_uri,
        repo_skeleton_sha256: skeleton_sha256,
        skeleton_files: harvest.results.skeleton_files,
    };

    // -------------------------------------------------------------------------
    // Persist Project Intel artifacts
    // -------------------------------------------------------------------------

    // Snapshot JSON
    let snapshot_sha256 = sha256_hex(&intel.snapshot_json);
    let snapshot_meta = serde_json::json!({
        "schema_version": GROUNDING_ARTIFACT_SCHEMA_VERSION,
        "sha256": &snapshot_sha256,
        "artifact_type": "project_snapshot",
    });
    let snapshot_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/grounding/project_intel/project_snapshot.json",
            intel.snapshot_json.clone(),
            snapshot_meta,
        )
        .map_err(|e| format!("Capsule put project_snapshot failed: {}", e))?
        .to_string();
    grounding_uris.push(snapshot_uri.clone());
    artifact_hashes.push(snapshot_sha256.clone());

    // Feeds
    let mut feed_uris = Vec::new();
    let mut feed_sha256s = Vec::new();
    let mut feed_names = Vec::new();

    for (name, content) in &intel.feeds {
        let feed_sha256 = sha256_hex(content);
        let feed_meta = serde_json::json!({
            "schema_version": GROUNDING_ARTIFACT_SCHEMA_VERSION,
            "sha256": &feed_sha256,
            "artifact_type": "project_intel_feed",
            "feed_name": name,
        });
        let feed_path = format!("intake/grounding/project_intel/{}", name);
        let feed_uri = capsule
            .put(
                spec_id,
                run_id,
                ObjectType::Artifact,
                &feed_path,
                content.clone(),
                feed_meta,
            )
            .map_err(|e| format!("Capsule put {} failed: {}", name, e))?
            .to_string();

        grounding_uris.push(feed_uri.clone());
        artifact_hashes.push(feed_sha256.clone());

        feed_uris.push(feed_uri);
        feed_sha256s.push(feed_sha256);
        feed_names.push(name.clone());
    }

    let intel_summary = ProjectIntelSummary {
        snapshot_uri,
        snapshot_sha256,
        feed_uris,
        feed_sha256s,
        feed_names,
    };

    Ok(GroundingCaptureResult {
        grounding_uris,
        artifact_hashes,
        harvest: Some(harvest_summary),
        project_intel: Some(intel_summary),
    })
}

// =============================================================================
// Utility: Extract artifact name from URI
// =============================================================================

/// Extract a human-readable artifact name from a grounding URI.
///
/// Example: "mv2://default/SPEC-123/abc/artifact/intake/grounding/harvest/churn_matrix.md"
///          → "harvest/churn_matrix.md"
pub fn extract_artifact_name_from_uri(uri: &str) -> String {
    // Look for "grounding/" in the URI and return everything after it
    if let Some(idx) = uri.find("/grounding/") {
        uri[idx + "/grounding/".len()..].to_string()
    } else if let Some(idx) = uri.rfind('/') {
        // Fallback: just the filename
        uri[idx + 1..].to_string()
    } else {
        uri.to_string()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_artifact_name_from_uri() {
        let uri = "mv2://default/SPEC-123/abc/artifact/intake/grounding/harvest/churn_matrix.md";
        assert_eq!(
            extract_artifact_name_from_uri(uri),
            "harvest/churn_matrix.md"
        );

        let uri2 =
            "mv2://default/project/myproj/artifact/intake/grounding/project_intel/code_topology.md";
        assert_eq!(
            extract_artifact_name_from_uri(uri2),
            "project_intel/code_topology.md"
        );

        let simple = "some/path/file.md";
        assert_eq!(extract_artifact_name_from_uri(simple), "file.md");
    }
}
