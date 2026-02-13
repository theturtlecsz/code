//! Disk persistence for bot run artifacts (Phase-1.5 + Phase-2).
//!
//! Two persistence layers:
//! - **`PersistenceStore`** (local cache): fast local files for crash-resume
//! - **`CapsulePersistence`** (SoR): capsule-backed `mv2://` URIs (D114)
//!
//! ## Local Cache Layout
//!
//! ```text
//! ~/.local/share/codex-pm/runs/{run_id}/
//!   request.json          BotRunRequest
//!   meta.json             { workspace_path }
//!   checkpoint-{seq}.json BotRunCheckpoint
//!   log.json              BotRunLog (terminal record)
//!   report.json           BotRunResult (serialized report)
//! ```
//!
//! ## URI Schemes
//!
//! ```text
//! # Local cache (Phase-1.5, used for resume):
//! pm://runs/{run_id}/request
//! pm://runs/{run_id}/checkpoint/{seq}
//! pm://runs/{run_id}/log
//! pm://runs/{run_id}/report
//!
//! # Capsule-authoritative (Phase-2, SoR):
//! mv2://default/pm/{run_id}/artifact/request.json
//! mv2://default/pm/{run_id}/artifact/log.json
//! mv2://default/pm/{run_id}/artifact/report.json
//! mv2://default/pm/{run_id}/artifact/checkpoint/{seq}.json
//! ```

use std::path::{Path, PathBuf};

use codex_core::pm::artifacts::{BotRunCheckpoint, BotRunLog};
use codex_core::pm::bot::BotRunRequest;
use serde::{Deserialize, Serialize};

/// Errors from the persistence layer.
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("run not found: {run_id}")]
    NotFound { run_id: String },
}

/// An incomplete run discovered during `scan_incomplete`.
#[derive(Debug, Clone)]
pub struct IncompleteRun {
    pub run_id: String,
    pub request: BotRunRequest,
    pub workspace_path: String,
    pub last_checkpoint: Option<BotRunCheckpoint>,
}

/// Metadata stored alongside the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunMeta {
    workspace_path: String,
}

/// Persistent store for bot run artifacts.
pub struct PersistenceStore {
    base_dir: PathBuf,
}

impl PersistenceStore {
    /// Create a store at the XDG data directory (`~/.local/share/codex-pm/runs/`).
    pub fn new() -> Result<Self, PersistenceError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| {
                PersistenceError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine XDG_DATA_HOME",
                ))
            })?
            .join("codex-pm")
            .join("runs");
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self { base_dir: data_dir })
    }

    /// Create a store with a custom base directory (for testing).
    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self, PersistenceError> {
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    /// Base directory accessor.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Directory for a specific run.
    fn run_dir(&self, run_id: &str) -> PathBuf {
        self.base_dir.join(run_id)
    }

    /// Atomically write `data` to `path` via a `.tmp` sibling.
    fn atomic_write(&self, path: &Path, data: &[u8]) -> Result<(), PersistenceError> {
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    // ── Write methods ────────────────────────────────────────────────────

    /// Persist a `BotRunRequest` and workspace metadata. Returns the request URI.
    pub fn write_request(
        &self,
        request: &BotRunRequest,
        workspace_path: &str,
    ) -> Result<String, PersistenceError> {
        let dir = self.run_dir(&request.run_id);
        std::fs::create_dir_all(&dir)?;

        let request_json = serde_json::to_string_pretty(request)?;
        self.atomic_write(&dir.join("request.json"), request_json.as_bytes())?;

        let meta = RunMeta {
            workspace_path: workspace_path.to_string(),
        };
        let meta_json = serde_json::to_string_pretty(&meta)?;
        self.atomic_write(&dir.join("meta.json"), meta_json.as_bytes())?;

        Ok(format!("pm://runs/{}/request", request.run_id))
    }

    /// Persist a checkpoint. Returns the checkpoint URI.
    pub fn write_checkpoint(
        &self,
        checkpoint: &BotRunCheckpoint,
    ) -> Result<String, PersistenceError> {
        let dir = self.run_dir(&checkpoint.run_id);
        std::fs::create_dir_all(&dir)?;

        let filename = format!("checkpoint-{}.json", checkpoint.seq);
        let json = serde_json::to_string_pretty(checkpoint)?;
        self.atomic_write(&dir.join(&filename), json.as_bytes())?;

        Ok(format!(
            "pm://runs/{}/checkpoint/{}",
            checkpoint.run_id, checkpoint.seq
        ))
    }

    /// Persist a terminal log. Returns the log URI.
    pub fn write_log(&self, log: &BotRunLog) -> Result<String, PersistenceError> {
        let dir = self.run_dir(&log.run_id);
        std::fs::create_dir_all(&dir)?;

        let json = serde_json::to_string_pretty(log)?;
        self.atomic_write(&dir.join("log.json"), json.as_bytes())?;

        Ok(format!("pm://runs/{}/log", log.run_id))
    }

    /// Persist a report artifact. Returns the report URI.
    pub fn write_report(
        &self,
        run_id: &str,
        report_json: &str,
    ) -> Result<String, PersistenceError> {
        let dir = self.run_dir(run_id);
        std::fs::create_dir_all(&dir)?;

        self.atomic_write(&dir.join("report.json"), report_json.as_bytes())?;

        Ok(format!("pm://runs/{run_id}/report"))
    }

    /// Persist a patch bundle artifact. Returns the patch_bundle URI.
    pub fn write_patch_bundle(&self, run_id: &str, json: &str) -> Result<String, PersistenceError> {
        let dir = self.run_dir(run_id);
        std::fs::create_dir_all(&dir)?;

        self.atomic_write(&dir.join("patch_bundle.json"), json.as_bytes())?;

        Ok(format!("pm://runs/{run_id}/patch_bundle"))
    }

    /// Persist a conflict summary artifact. Returns the conflict_summary URI.
    pub fn write_conflict_summary(
        &self,
        run_id: &str,
        json: &str,
    ) -> Result<String, PersistenceError> {
        let dir = self.run_dir(run_id);
        std::fs::create_dir_all(&dir)?;

        self.atomic_write(&dir.join("conflict_summary.json"), json.as_bytes())?;

        Ok(format!("pm://runs/{run_id}/conflict_summary"))
    }

    // ── Read methods ─────────────────────────────────────────────────────

    /// Read back a persisted request and its workspace path.
    pub fn read_request(&self, run_id: &str) -> Result<(BotRunRequest, String), PersistenceError> {
        let dir = self.run_dir(run_id);
        let req_path = dir.join("request.json");
        if !req_path.exists() {
            return Err(PersistenceError::NotFound {
                run_id: run_id.to_string(),
            });
        }

        let req_data = std::fs::read_to_string(&req_path)?;
        let request: BotRunRequest = serde_json::from_str(&req_data)?;

        let meta_data = std::fs::read_to_string(dir.join("meta.json"))?;
        let meta: RunMeta = serde_json::from_str(&meta_data)?;

        Ok((request, meta.workspace_path))
    }

    /// Read back a persisted log, if present.
    pub fn read_log(&self, run_id: &str) -> Result<Option<BotRunLog>, PersistenceError> {
        let path = self.run_dir(run_id).join("log.json");
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path)?;
        let log: BotRunLog = serde_json::from_str(&data)?;
        Ok(Some(log))
    }

    /// Read back a persisted report, if present.
    pub fn read_report(&self, run_id: &str) -> Result<Option<String>, PersistenceError> {
        let path = self.run_dir(run_id).join("report.json");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read_to_string(&path)?))
    }

    // ── Scan methods ─────────────────────────────────────────────────────

    /// Scan for incomplete runs: directories with `request.json` but no
    /// terminal `log.json` (absent or with a non-terminal state).
    pub fn scan_incomplete(&self) -> Result<Vec<IncompleteRun>, PersistenceError> {
        let mut incomplete = Vec::new();

        let entries = match std::fs::read_dir(&self.base_dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(incomplete),
            Err(e) => return Err(e.into()),
        };

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let run_id = entry.file_name().to_string_lossy().to_string();
            let dir = entry.path();

            // Must have request.json
            let req_path = dir.join("request.json");
            if !req_path.exists() {
                continue;
            }

            // Check log.json
            let log_path = dir.join("log.json");
            if log_path.exists() {
                let log_data = std::fs::read_to_string(&log_path)?;
                let log: BotRunLog = serde_json::from_str(&log_data)?;
                if log.state.is_terminal() {
                    // Already finished, skip
                    continue;
                }
            }

            // Read the request + meta
            let req_data = std::fs::read_to_string(&req_path)?;
            let request: BotRunRequest = serde_json::from_str(&req_data)?;

            let meta_path = dir.join("meta.json");
            let workspace_path = if meta_path.exists() {
                let meta_data = std::fs::read_to_string(&meta_path)?;
                let meta: RunMeta = serde_json::from_str(&meta_data)?;
                meta.workspace_path
            } else {
                String::new()
            };

            // Find the latest checkpoint
            let last_checkpoint = self.read_last_checkpoint(&run_id, &dir)?;

            incomplete.push(IncompleteRun {
                run_id,
                request,
                workspace_path,
                last_checkpoint,
            });
        }

        Ok(incomplete)
    }

    /// Read the highest-numbered checkpoint in a run directory.
    fn read_last_checkpoint(
        &self,
        _run_id: &str,
        dir: &Path,
    ) -> Result<Option<BotRunCheckpoint>, PersistenceError> {
        let mut max_seq: Option<u32> = None;

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(rest) = name.strip_prefix("checkpoint-")
                && let Some(seq_str) = rest.strip_suffix(".json")
                && let Ok(seq) = seq_str.parse::<u32>()
            {
                max_seq = Some(max_seq.map_or(seq, |m: u32| m.max(seq)));
            }
        }

        if let Some(seq) = max_seq {
            let path = dir.join(format!("checkpoint-{seq}.json"));
            let data = std::fs::read_to_string(&path)?;
            let cp: BotRunCheckpoint = serde_json::from_str(&data)?;
            Ok(Some(cp))
        } else {
            Ok(None)
        }
    }

    /// List all artifact URIs present for a run.
    pub fn artifact_uris(&self, run_id: &str) -> Vec<String> {
        let dir = self.run_dir(run_id);
        let mut uris = Vec::new();

        if dir.join("request.json").exists() {
            uris.push(format!("pm://runs/{run_id}/request"));
        }

        // Collect checkpoint URIs in order
        if let Ok(entries) = std::fs::read_dir(&dir) {
            let mut seqs: Vec<u32> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(rest) = name.strip_prefix("checkpoint-")
                    && let Some(seq_str) = rest.strip_suffix(".json")
                    && let Ok(seq) = seq_str.parse::<u32>()
                {
                    seqs.push(seq);
                }
            }
            seqs.sort_unstable();
            for seq in seqs {
                uris.push(format!("pm://runs/{run_id}/checkpoint/{seq}"));
            }
        }

        if dir.join("log.json").exists() {
            uris.push(format!("pm://runs/{run_id}/log"));
        }

        if dir.join("report.json").exists() {
            uris.push(format!("pm://runs/{run_id}/report"));
        }

        if dir.join("patch_bundle.json").exists() {
            uris.push(format!("pm://runs/{run_id}/patch_bundle"));
        }

        if dir.join("conflict_summary.json").exists() {
            uris.push(format!("pm://runs/{run_id}/conflict_summary"));
        }

        uris
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Capsule-Authoritative Persistence (Phase-2, D114)
// ─────────────────────────────────────────────────────────────────────────────

use codex_tui::memvid_adapter::{CapsuleConfig, CapsuleHandle, ObjectType, default_capsule_config};

/// Capsule-backed persistence layer — source of record for bot run artifacts.
///
/// Writes artifacts to the workspace capsule using `mv2://` URIs.
/// Falls back gracefully if the capsule is unavailable (e.g. missing workspace).
pub struct CapsulePersistence {
    handle: CapsuleHandle,
}

/// Spec ID prefix used for PM artifacts in the capsule.
const PM_SPEC_PREFIX: &str = "pm";

impl CapsulePersistence {
    /// Open the workspace capsule at the canonical path for `workspace_path`.
    pub fn open(workspace_path: &Path) -> Result<Self, PersistenceError> {
        let config = default_capsule_config(workspace_path);
        let handle = CapsuleHandle::open(config).map_err(|e| {
            PersistenceError::Io(std::io::Error::other(format!("capsule open: {e}")))
        })?;
        Ok(Self { handle })
    }

    /// Open with an explicit config (for testing).
    pub fn open_with_config(config: CapsuleConfig) -> Result<Self, PersistenceError> {
        let handle = CapsuleHandle::open(config).map_err(|e| {
            PersistenceError::Io(std::io::Error::other(format!("capsule open: {e}")))
        })?;
        Ok(Self { handle })
    }

    /// Write a `BotRunRequest` to the capsule. Returns `mv2://` URI.
    pub fn write_request(
        &self,
        request: &BotRunRequest,
        run_id: &str,
    ) -> Result<String, PersistenceError> {
        let json = serde_json::to_string_pretty(request)?;
        let uri = self
            .handle
            .put(
                PM_SPEC_PREFIX,
                run_id,
                ObjectType::Artifact,
                "request.json",
                json.into_bytes(),
                serde_json::json!({"schema_version": BotRunRequest::SCHEMA_VERSION}),
            )
            .map_err(|e| {
                PersistenceError::Io(std::io::Error::other(format!("capsule put request: {e}")))
            })?;
        Ok(uri.to_string())
    }

    /// Write a `BotRunLog` to the capsule. Returns `mv2://` URI.
    pub fn write_log(&self, log: &BotRunLog, run_id: &str) -> Result<String, PersistenceError> {
        let json = serde_json::to_string_pretty(log)?;
        let uri = self
            .handle
            .put(
                PM_SPEC_PREFIX,
                run_id,
                ObjectType::Artifact,
                "log.json",
                json.into_bytes(),
                serde_json::json!({"schema_version": BotRunLog::SCHEMA_VERSION, "state": log.state}),
            )
            .map_err(|e| {
                PersistenceError::Io(std::io::Error::other(format!("capsule put log: {e}")))
            })?;
        Ok(uri.to_string())
    }

    /// Write a report JSON blob to the capsule. Returns `mv2://` URI.
    pub fn write_report(
        &self,
        run_id: &str,
        report_json: &str,
    ) -> Result<String, PersistenceError> {
        let uri = self
            .handle
            .put(
                PM_SPEC_PREFIX,
                run_id,
                ObjectType::Artifact,
                "report.json",
                report_json.as_bytes().to_vec(),
                serde_json::json!({"type": "report"}),
            )
            .map_err(|e| {
                PersistenceError::Io(std::io::Error::other(format!("capsule put report: {e}")))
            })?;
        Ok(uri.to_string())
    }

    /// Write a patch bundle JSON blob to the capsule. Returns `mv2://` URI.
    pub fn write_patch_bundle(&self, run_id: &str, json: &str) -> Result<String, PersistenceError> {
        let uri = self
            .handle
            .put(
                PM_SPEC_PREFIX,
                run_id,
                ObjectType::Artifact,
                "patch_bundle.json",
                json.as_bytes().to_vec(),
                serde_json::json!({"type": "patch_bundle"}),
            )
            .map_err(|e| {
                PersistenceError::Io(std::io::Error::other(format!(
                    "capsule put patch_bundle: {e}"
                )))
            })?;
        Ok(uri.to_string())
    }

    /// Write a conflict summary JSON blob to the capsule. Returns `mv2://` URI.
    pub fn write_conflict_summary(
        &self,
        run_id: &str,
        json: &str,
    ) -> Result<String, PersistenceError> {
        let uri = self
            .handle
            .put(
                PM_SPEC_PREFIX,
                run_id,
                ObjectType::Artifact,
                "conflict_summary.json",
                json.as_bytes().to_vec(),
                serde_json::json!({"type": "conflict_summary"}),
            )
            .map_err(|e| {
                PersistenceError::Io(std::io::Error::other(format!(
                    "capsule put conflict_summary: {e}"
                )))
            })?;
        Ok(uri.to_string())
    }

    /// Write a checkpoint to the capsule. Returns `mv2://` URI.
    pub fn write_checkpoint(
        &self,
        checkpoint: &BotRunCheckpoint,
    ) -> Result<String, PersistenceError> {
        let json = serde_json::to_string_pretty(checkpoint)?;
        let path = format!("checkpoint/{}.json", checkpoint.seq);
        let uri = self
            .handle
            .put(
                PM_SPEC_PREFIX,
                &checkpoint.run_id,
                ObjectType::Artifact,
                &path,
                json.into_bytes(),
                serde_json::json!({"seq": checkpoint.seq, "state": checkpoint.state}),
            )
            .map_err(|e| {
                PersistenceError::Io(std::io::Error::other(format!(
                    "capsule put checkpoint: {e}"
                )))
            })?;
        Ok(uri.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::pm::artifacts::BotRunState;
    use codex_core::pm::bot::{BotCaptureMode, BotKind, BotWriteMode};

    fn make_request(run_id: &str, work_item_id: &str) -> BotRunRequest {
        BotRunRequest {
            schema_version: BotRunRequest::SCHEMA_VERSION.to_string(),
            run_id: run_id.to_string(),
            work_item_id: work_item_id.to_string(),
            kind: BotKind::Research,
            capture_mode: BotCaptureMode::PromptsOnly,
            write_mode: BotWriteMode::None,
            requested_at: "2026-02-09T12:00:00Z".to_string(),
            trigger: None,
        }
    }

    fn make_log(run_id: &str, work_item_id: &str, state: BotRunState) -> BotRunLog {
        BotRunLog {
            schema_version: BotRunLog::SCHEMA_VERSION.to_string(),
            run_id: run_id.to_string(),
            work_item_id: work_item_id.to_string(),
            state,
            started_at: "2026-02-09T12:00:00Z".to_string(),
            finished_at: "2026-02-09T12:00:01Z".to_string(),
            duration_s: 1,
            exit_code: 0,
            summary: "Test log".to_string(),
            partial: false,
            checkpoint_count: 0,
            error: None,
        }
    }

    fn make_checkpoint(run_id: &str, work_item_id: &str, seq: u32) -> BotRunCheckpoint {
        BotRunCheckpoint {
            schema_version: BotRunCheckpoint::SCHEMA_VERSION.to_string(),
            run_id: run_id.to_string(),
            work_item_id: work_item_id.to_string(),
            seq,
            state: BotRunState::Running,
            timestamp: "2026-02-09T12:00:00Z".to_string(),
            summary: format!("Checkpoint {seq}"),
            percent: Some(50),
            phase: None,
        }
    }

    #[test]
    fn write_read_request_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = PersistenceStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        let request = make_request("run-001", "SPEC-TEST-001");
        let uri = store.write_request(&request, "/tmp/workspace").unwrap();
        assert_eq!(uri, "pm://runs/run-001/request");

        let (read_req, ws) = store.read_request("run-001").unwrap();
        assert_eq!(read_req, request);
        assert_eq!(ws, "/tmp/workspace");
    }

    #[test]
    fn write_checkpoint_creates_numbered_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = PersistenceStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        // Need a run directory first
        let request = make_request("run-002", "SPEC-TEST-001");
        store.write_request(&request, "/tmp/ws").unwrap();

        let cp0 = make_checkpoint("run-002", "SPEC-TEST-001", 0);
        let uri0 = store.write_checkpoint(&cp0).unwrap();
        assert_eq!(uri0, "pm://runs/run-002/checkpoint/0");

        let cp1 = make_checkpoint("run-002", "SPEC-TEST-001", 1);
        let uri1 = store.write_checkpoint(&cp1).unwrap();
        assert_eq!(uri1, "pm://runs/run-002/checkpoint/1");

        // Verify files exist
        assert!(tmp.path().join("run-002/checkpoint-0.json").exists());
        assert!(tmp.path().join("run-002/checkpoint-1.json").exists());
    }

    #[test]
    fn write_read_log_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = PersistenceStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        let request = make_request("run-003", "SPEC-TEST-001");
        store.write_request(&request, "/tmp/ws").unwrap();

        let log = make_log("run-003", "SPEC-TEST-001", BotRunState::Succeeded);
        let uri = store.write_log(&log).unwrap();
        assert_eq!(uri, "pm://runs/run-003/log");

        let read_log = store.read_log("run-003").unwrap();
        assert_eq!(read_log, Some(log));
    }

    #[test]
    fn scan_incomplete_finds_runs_without_log() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = PersistenceStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        // Run with no log.json → incomplete
        let req = make_request("run-inc", "SPEC-TEST-001");
        store.write_request(&req, "/tmp/ws").unwrap();

        let incomplete = store.scan_incomplete().unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].run_id, "run-inc");
        assert_eq!(incomplete[0].workspace_path, "/tmp/ws");
    }

    #[test]
    fn scan_incomplete_skips_terminated_runs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = PersistenceStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        // Run with terminal log → not incomplete
        let req = make_request("run-done", "SPEC-TEST-001");
        store.write_request(&req, "/tmp/ws").unwrap();
        let log = make_log("run-done", "SPEC-TEST-001", BotRunState::Succeeded);
        store.write_log(&log).unwrap();

        let incomplete = store.scan_incomplete().unwrap();
        assert!(incomplete.is_empty());
    }

    #[test]
    fn scan_incomplete_returns_runs_with_non_terminal_log() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = PersistenceStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        // Run with non-terminal log (Running) → incomplete
        let req = make_request("run-partial", "SPEC-TEST-001");
        store.write_request(&req, "/tmp/ws").unwrap();
        let log = make_log("run-partial", "SPEC-TEST-001", BotRunState::Running);
        store.write_log(&log).unwrap();

        let incomplete = store.scan_incomplete().unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].run_id, "run-partial");
    }

    #[test]
    fn artifact_uris_returns_correct_uris() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = PersistenceStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        let req = make_request("run-uri", "SPEC-TEST-001");
        store.write_request(&req, "/tmp/ws").unwrap();

        let cp = make_checkpoint("run-uri", "SPEC-TEST-001", 0);
        store.write_checkpoint(&cp).unwrap();

        let log = make_log("run-uri", "SPEC-TEST-001", BotRunState::Succeeded);
        store.write_log(&log).unwrap();

        store.write_report("run-uri", r#"{"stub":true}"#).unwrap();

        let uris = store.artifact_uris("run-uri");
        assert_eq!(
            uris,
            vec![
                "pm://runs/run-uri/request",
                "pm://runs/run-uri/checkpoint/0",
                "pm://runs/run-uri/log",
                "pm://runs/run-uri/report",
            ]
        );
    }

    #[test]
    fn read_request_not_found() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = PersistenceStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        let result = store.read_request("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
