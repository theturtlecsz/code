//! BotRunManager — orchestrates bot run lifecycle (PM-D22).
//!
//! Purpose-built manager (not reusing AgentManager) with different
//! lifecycle characteristics: days-long runs, checkpoint/resume,
//! capsule-backed state.
//!
//! ## Decision References
//! - PM-D11: Capsule-backed run queue
//! - PM-D14: Reject duplicate (work_item_id, kind)
//! - PM-D16: Reject capture=none

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use codex_core::pm::artifacts::BotRunState;
use codex_core::pm::bot::{BotCaptureMode, BotKind, BotRunRequest};
use tokio::sync::{Mutex, broadcast};

use crate::engine;
use crate::persistence::{CapsulePersistence, PersistenceStore};
use crate::protocol::{
    BotRunParams, BotRunResult, BotRunsResult, BotShowResult, BotStatusResult,
    BotTerminalNotification, RunSummary,
};

/// Key for the active run index: (workspace_path, work_item_id, bot_kind).
type ActiveRunKey = (String, String, BotKind);

/// Error type for manager operations.
#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    #[error("capture_mode=none not allowed for bot runs (PM-D16)")]
    CaptureNoneRejected,

    #[error("duplicate run: {work_item_id}/{kind:?} already active")]
    DuplicateRun { work_item_id: String, kind: BotKind },

    #[error("run not found: {run_id}")]
    RunNotFound { run_id: String },

    #[error("run already terminal: {run_id}")]
    AlreadyTerminal { run_id: String },

    #[error("invalid request: {reason}")]
    InvalidRequest { reason: String },

    #[error("infrastructure error: {0}")]
    Infra(String),
}

/// Tracks an active or completed run.
#[derive(Debug, Clone)]
struct RunRecord {
    request: BotRunRequest,
    workspace_path: String,
    state: BotRunState,
    started_at: Option<String>,
    finished_at: Option<String>,
    summary: Option<String>,
    report_json: Option<String>,
    /// Cached artifact URIs (capsule `mv2://` or local `pm://`).
    cached_artifact_uris: Vec<String>,
}

/// Manages bot runs for all workspaces served by this service instance.
pub struct BotRunManager {
    /// Active and recent runs, keyed by run_id.
    runs: Arc<Mutex<HashMap<String, RunRecord>>>,
    /// Active runs index: (workspace, work_item_id, kind) -> run_id.
    /// Used for duplicate detection (PM-D14).
    active_index: Arc<Mutex<HashMap<ActiveRunKey, String>>>,
    /// Broadcast channel for terminal notifications (PM-D24).
    terminal_tx: broadcast::Sender<BotTerminalNotification>,
    /// Service start time.
    started_at: Instant,
    /// Last activity timestamp for idle timeout (D135).
    last_activity: Arc<Mutex<Instant>>,
    /// Active connection count for idle timeout tracking.
    connection_count: Arc<AtomicU32>,
    /// Persistence store for durable run artifacts (local cache).
    store: Arc<PersistenceStore>,
    /// Capsule persistence (SoR, Phase-2 D114). Optional — absent when
    /// no workspace capsule is available.
    capsule: Arc<Mutex<Option<CapsulePersistence>>>,
}

impl BotRunManager {
    pub fn new(store: Arc<PersistenceStore>) -> Self {
        let (terminal_tx, _) = broadcast::channel(64);
        let now = Instant::now();
        Self {
            runs: Arc::new(Mutex::new(HashMap::new())),
            active_index: Arc::new(Mutex::new(HashMap::new())),
            terminal_tx,
            started_at: now,
            last_activity: Arc::new(Mutex::new(now)),
            connection_count: Arc::new(AtomicU32::new(0)),
            store,
            capsule: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with an existing capsule persistence (for testing or pre-opened capsule).
    pub fn with_capsule(store: Arc<PersistenceStore>, capsule: CapsulePersistence) -> Self {
        let (terminal_tx, _) = broadcast::channel(64);
        let now = Instant::now();
        Self {
            runs: Arc::new(Mutex::new(HashMap::new())),
            active_index: Arc::new(Mutex::new(HashMap::new())),
            terminal_tx,
            started_at: now,
            last_activity: Arc::new(Mutex::new(now)),
            connection_count: Arc::new(AtomicU32::new(0)),
            store,
            capsule: Arc::new(Mutex::new(Some(capsule))),
        }
    }

    /// Subscribe to terminal notifications for --wait support.
    pub fn subscribe_terminal(&self) -> broadcast::Receiver<BotTerminalNotification> {
        self.terminal_tx.subscribe()
    }

    /// Service uptime in seconds.
    pub fn uptime_s(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Number of currently active (non-terminal) runs.
    pub async fn active_run_count(&self) -> usize {
        self.active_index.lock().await.len()
    }

    /// List workspace paths that have active runs.
    pub async fn active_workspaces(&self) -> Vec<String> {
        let index = self.active_index.lock().await;
        let mut workspaces: Vec<String> = index.keys().map(|(ws, _, _)| ws.clone()).collect();
        workspaces.sort();
        workspaces.dedup();
        workspaces
    }

    /// Update last activity timestamp to now.
    pub async fn touch_activity(&self) {
        let mut ts = self.last_activity.lock().await;
        *ts = Instant::now();
    }

    /// Time elapsed since last activity.
    pub async fn last_activity_elapsed(&self) -> std::time::Duration {
        let ts = self.last_activity.lock().await;
        ts.elapsed()
    }

    /// Increment active connection count.
    pub fn inc_connections(&self) -> u32 {
        self.connection_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Decrement active connection count.
    pub fn dec_connections(&self) -> u32 {
        self.connection_count.fetch_sub(1, Ordering::Relaxed) - 1
    }

    /// Current connection count.
    pub fn connection_count(&self) -> u32 {
        self.connection_count.load(Ordering::Relaxed)
    }

    /// Try to open a capsule for a workspace and cache it.
    ///
    /// Called lazily on first write to a workspace. If the capsule can't be
    /// opened (e.g. no `.speckit/` directory), logs a warning and returns None.
    async fn ensure_capsule(&self, workspace_path: &str) -> bool {
        let mut cap = self.capsule.lock().await;
        if cap.is_some() {
            return true;
        }
        match CapsulePersistence::open(std::path::Path::new(workspace_path)) {
            Ok(c) => {
                *cap = Some(c);
                true
            }
            Err(e) => {
                tracing::debug!("Capsule not available for {workspace_path}: {e}");
                false
            }
        }
    }

    /// Write terminal artifacts to capsule (best-effort). Returns `mv2://` URIs on success.
    async fn capsule_write_terminal(
        &self,
        request: &BotRunRequest,
        run_id: &str,
        log: &codex_core::pm::artifacts::BotRunLog,
        report_json: &str,
    ) -> Vec<String> {
        let cap = self.capsule.lock().await;
        let Some(capsule) = cap.as_ref() else {
            return vec![];
        };

        let mut uris = Vec::new();

        match capsule.write_request(request, run_id) {
            Ok(uri) => uris.push(uri),
            Err(e) => tracing::warn!("Capsule write request failed: {e}"),
        }
        match capsule.write_log(log, run_id) {
            Ok(uri) => uris.push(uri),
            Err(e) => tracing::warn!("Capsule write log failed: {e}"),
        }
        match capsule.write_report(run_id, report_json) {
            Ok(uri) => uris.push(uri),
            Err(e) => tracing::warn!("Capsule write report failed: {e}"),
        }

        uris
    }

    /// Get artifact URIs for a run, preferring capsule `mv2://` URIs over local `pm://`.
    async fn artifact_uris(&self, run_id: &str, capsule_uris: &[String]) -> Vec<String> {
        if capsule_uris.is_empty() {
            // Fallback to local store URIs
            self.store.artifact_uris(run_id)
        } else {
            capsule_uris.to_vec()
        }
    }

    /// Submit a new bot run (PM-D11).
    ///
    /// Validates the request, checks for duplicates (PM-D14), rejects
    /// capture=none (PM-D16), persists to disk, then executes the stub engine.
    pub async fn submit(&self, params: BotRunParams) -> Result<BotRunResult, ManagerError> {
        // PM-D16: reject capture=none
        if params.capture_mode == BotCaptureMode::None {
            return Err(ManagerError::CaptureNoneRejected);
        }

        // Create the BotRunRequest (validates write_mode constraints)
        let request = BotRunRequest::new(
            &params.work_item_id,
            params.kind,
            params.capture_mode,
            params.write_mode,
            None, // trigger metadata handled by CLI layer
        )
        .map_err(|e| ManagerError::InvalidRequest {
            reason: e.to_string(),
        })?;

        let run_id = request.run_id.clone();
        let workspace = params.workspace_path.clone();

        // PM-D14: check for duplicate (work_item_id, kind)
        {
            let index = self.active_index.lock().await;
            let key = (workspace.clone(), params.work_item_id.clone(), params.kind);
            if index.contains_key(&key) {
                return Err(ManagerError::DuplicateRun {
                    work_item_id: params.work_item_id,
                    kind: params.kind,
                });
            }
        }

        // Persist request BEFORE engine execution
        self.store
            .write_request(&request, &workspace)
            .map_err(|e| ManagerError::Infra(format!("persist request: {e}")))?;

        // Record the run as queued
        let record = RunRecord {
            request: request.clone(),
            workspace_path: workspace.clone(),
            state: BotRunState::Queued,
            started_at: None,
            finished_at: None,
            summary: None,
            report_json: None,
            cached_artifact_uris: vec![],
        };

        {
            let mut runs = self.runs.lock().await;
            runs.insert(run_id.clone(), record);
        }
        {
            let mut index = self.active_index.lock().await;
            let key = (workspace.clone(), params.work_item_id.clone(), params.kind);
            index.insert(key, run_id.clone());
        }

        // Execute the real engine
        let engine_result = engine::run_engine(engine::EngineParams {
            kind: params.kind,
            run_id: run_id.clone(),
            work_item_id: params.work_item_id.clone(),
            workspace_path: workspace.clone(),
            write_mode: params.write_mode,
            rebase_target: params.rebase_target.clone(),
            allow_degraded: params.allow_degraded,
            notebooklm_health_url: params.notebooklm_health_url.clone(),
        })
        .await;

        // Persist terminal artifacts (local cache)
        let _ = self.store.write_log(&engine_result.log);
        let _ = self.store.write_report(&run_id, &engine_result.report_json);
        if let Some(ref pb) = engine_result.patch_bundle_json {
            let _ = self.store.write_patch_bundle(&run_id, pb);
        }
        if let Some(ref cs) = engine_result.conflict_summary_json {
            let _ = self.store.write_conflict_summary(&run_id, cs);
        }

        // Persist checkpoints (local cache)
        for cp in &engine_result.checkpoints {
            let _ = self.store.write_checkpoint(cp);
        }

        // Dual-write to capsule (SoR, D114) — best effort
        self.ensure_capsule(&workspace).await;
        let mut capsule_uris = self
            .capsule_write_terminal(
                &request,
                &run_id,
                &engine_result.log,
                &engine_result.report_json,
            )
            .await;

        // Persist patch/conflict artifacts to capsule
        {
            let cap = self.capsule.lock().await;
            if let Some(capsule) = cap.as_ref() {
                if let Some(ref pb) = engine_result.patch_bundle_json {
                    match capsule.write_patch_bundle(&run_id, pb) {
                        Ok(uri) => capsule_uris.push(uri),
                        Err(e) => tracing::warn!("Capsule write patch_bundle failed: {e}"),
                    }
                }
                if let Some(ref cs) = engine_result.conflict_summary_json {
                    match capsule.write_conflict_summary(&run_id, cs) {
                        Ok(uri) => capsule_uris.push(uri),
                        Err(e) => tracing::warn!("Capsule write conflict_summary failed: {e}"),
                    }
                }
            }
        }

        // Persist checkpoints to capsule
        {
            let cap = self.capsule.lock().await;
            if let Some(capsule) = cap.as_ref() {
                for cp in &engine_result.checkpoints {
                    match capsule.write_checkpoint(cp) {
                        Ok(uri) => capsule_uris.push(uri),
                        Err(e) => tracing::warn!("Capsule write checkpoint failed: {e}"),
                    }
                }
            }
        }

        let artifact_uris = self.artifact_uris(&run_id, &capsule_uris).await;

        // Update run record with results
        {
            let mut runs = self.runs.lock().await;
            if let Some(record) = runs.get_mut(&run_id) {
                record.state = engine_result.state;
                record.started_at = Some(engine_result.log.started_at.clone());
                record.finished_at = Some(engine_result.log.finished_at.clone());
                record.summary = Some(engine_result.summary.clone());
                record.report_json = Some(engine_result.report_json.clone());
                record.cached_artifact_uris = artifact_uris.clone();
            }
        }

        // Remove from active index (run is now terminal)
        {
            let mut index = self.active_index.lock().await;
            let key = (workspace, params.work_item_id.clone(), params.kind);
            index.remove(&key);
        }

        // Broadcast terminal notification
        let notification = BotTerminalNotification {
            run_id: run_id.clone(),
            status: engine_result.state,
            exit_code: engine_result.exit_code,
            summary: engine_result.summary.clone(),
            artifact_uris: artifact_uris.clone(),
        };
        // Ignore error if no subscribers
        let _ = self.terminal_tx.send(notification);

        Ok(BotRunResult {
            run_id,
            status: engine_result.state,
            work_item_id: params.work_item_id,
            kind: params.kind,
            exit_code: engine_result.exit_code,
            summary: Some(engine_result.summary),
            artifact_uris,
        })
    }

    /// Query run status (PM-D14).
    pub async fn status(
        &self,
        workspace: &str,
        work_item_id: &str,
        kind: Option<BotKind>,
    ) -> BotStatusResult {
        let runs = self.runs.lock().await;
        let summaries: Vec<RunSummary> = runs
            .values()
            .filter(|r| {
                r.workspace_path == workspace
                    && r.request.work_item_id == work_item_id
                    && kind.is_none_or(|k| r.request.kind == k)
            })
            .map(|r| RunSummary {
                run_id: r.request.run_id.clone(),
                status: r.state,
                kind: r.request.kind,
                started_at: r.started_at.clone(),
                last_checkpoint: None,
                summary: r.summary.clone(),
            })
            .collect();

        BotStatusResult { runs: summaries }
    }

    /// Get detailed information for a specific run.
    ///
    /// Returns `mv2://` URIs when capsule is available, otherwise `pm://` URIs.
    pub async fn show(&self, run_id: &str) -> Result<BotShowResult, ManagerError> {
        let runs = self.runs.lock().await;
        let record = runs.get(run_id).ok_or_else(|| ManagerError::RunNotFound {
            run_id: run_id.to_string(),
        })?;

        // Prefer cached artifact URIs (which may be mv2:// from capsule)
        let artifact_uris = if record.cached_artifact_uris.is_empty() {
            self.store.artifact_uris(run_id)
        } else {
            record.cached_artifact_uris.clone()
        };

        Ok(BotShowResult {
            run_id: record.request.run_id.clone(),
            work_item_id: record.request.work_item_id.clone(),
            kind: record.request.kind,
            status: record.state,
            capture_mode: record.request.capture_mode,
            write_mode: record.request.write_mode,
            started_at: record.started_at.clone(),
            finished_at: record.finished_at.clone(),
            summary: record.summary.clone(),
            report_json: record.report_json.clone(),
            artifact_uris,
        })
    }

    /// List runs for a workspace + work item, sorted by started_at desc, paginated.
    pub async fn list_runs(
        &self,
        workspace: &str,
        work_item_id: &str,
        limit: u32,
        offset: u32,
    ) -> BotRunsResult {
        let runs = self.runs.lock().await;
        let mut matching: Vec<&RunRecord> = runs
            .values()
            .filter(|r| r.workspace_path == workspace && r.request.work_item_id == work_item_id)
            .collect();

        // Sort by started_at descending (None sorts last)
        matching.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        let total = matching.len();
        let page: Vec<RunSummary> = matching
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|r| RunSummary {
                run_id: r.request.run_id.clone(),
                status: r.state,
                kind: r.request.kind,
                started_at: r.started_at.clone(),
                last_checkpoint: None,
                summary: r.summary.clone(),
            })
            .collect();

        BotRunsResult { runs: page, total }
    }

    /// Cancel an active run (PM-D13).
    pub async fn cancel(
        &self,
        _workspace: &str,
        _work_item_id: &str,
        run_id: &str,
    ) -> Result<BotRunState, ManagerError> {
        let mut runs = self.runs.lock().await;
        let record = runs
            .get_mut(run_id)
            .ok_or_else(|| ManagerError::RunNotFound {
                run_id: run_id.to_string(),
            })?;

        if record.state.is_terminal() {
            // Already done, return current state
            return Ok(record.state);
        }

        let now = chrono::Utc::now().to_rfc3339();
        record.state = BotRunState::Cancelled;
        record.finished_at = Some(now.clone());
        record.summary = Some("Cancelled by user".to_string());

        // Persist cancellation log
        let log = codex_core::pm::artifacts::BotRunLog {
            schema_version: codex_core::pm::artifacts::BotRunLog::SCHEMA_VERSION.to_string(),
            run_id: run_id.to_string(),
            work_item_id: record.request.work_item_id.clone(),
            state: BotRunState::Cancelled,
            started_at: record.started_at.clone().unwrap_or_else(|| now.clone()),
            finished_at: now,
            duration_s: 0,
            exit_code: 2,
            summary: "Cancelled by user".to_string(),
            partial: true,
            checkpoint_count: 0,
            error: None,
        };
        let _ = self.store.write_log(&log);

        Ok(BotRunState::Cancelled)
    }

    /// Resume an incomplete run from persistence.
    ///
    /// Loads the persisted request, validates that it is incomplete,
    /// re-executes the engine, and persists terminal artifacts.
    pub async fn resume(
        &self,
        run_id: &str,
        workspace_path: &str,
    ) -> Result<BotRunResult, ManagerError> {
        // Load from persistence
        let (request, _persisted_ws) = self.store.read_request(run_id).map_err(|e| match e {
            crate::persistence::PersistenceError::NotFound { .. } => ManagerError::RunNotFound {
                run_id: run_id.to_string(),
            },
            other => ManagerError::Infra(format!("read request: {other}")),
        })?;

        // Check if already terminal
        if let Ok(Some(log)) = self.store.read_log(run_id)
            && log.state.is_terminal()
        {
            return Err(ManagerError::AlreadyTerminal {
                run_id: run_id.to_string(),
            });
        }

        let workspace = workspace_path.to_string();

        // Record in-memory as queued
        let record = RunRecord {
            request: request.clone(),
            workspace_path: workspace.clone(),
            state: BotRunState::Queued,
            started_at: None,
            finished_at: None,
            summary: None,
            report_json: None,
            cached_artifact_uris: vec![],
        };

        {
            let mut runs = self.runs.lock().await;
            runs.insert(run_id.to_string(), record);
        }
        {
            let mut index = self.active_index.lock().await;
            let key = (
                workspace.clone(),
                request.work_item_id.clone(),
                request.kind,
            );
            index.insert(key, run_id.to_string());
        }

        // Re-execute the real engine
        let engine_result = engine::run_engine(engine::EngineParams {
            kind: request.kind,
            run_id: run_id.to_string(),
            work_item_id: request.work_item_id.clone(),
            workspace_path: workspace.clone(),
            write_mode: request.write_mode,
            rebase_target: None,         // resume does not specify rebase target
            allow_degraded: None,        // resume uses default allow_degraded
            notebooklm_health_url: None, // resume uses default health URL
        })
        .await;

        // Persist terminal artifacts (local cache)
        let _ = self.store.write_log(&engine_result.log);
        let _ = self.store.write_report(run_id, &engine_result.report_json);
        if let Some(ref pb) = engine_result.patch_bundle_json {
            let _ = self.store.write_patch_bundle(run_id, pb);
        }
        if let Some(ref cs) = engine_result.conflict_summary_json {
            let _ = self.store.write_conflict_summary(run_id, cs);
        }

        // Persist checkpoints (local cache)
        for cp in &engine_result.checkpoints {
            let _ = self.store.write_checkpoint(cp);
        }

        // Dual-write to capsule (SoR, D114) — best effort
        self.ensure_capsule(workspace_path).await;
        let mut capsule_uris = self
            .capsule_write_terminal(
                &request,
                run_id,
                &engine_result.log,
                &engine_result.report_json,
            )
            .await;

        // Persist patch/conflict artifacts to capsule
        {
            let cap = self.capsule.lock().await;
            if let Some(capsule) = cap.as_ref() {
                if let Some(ref pb) = engine_result.patch_bundle_json {
                    match capsule.write_patch_bundle(run_id, pb) {
                        Ok(uri) => capsule_uris.push(uri),
                        Err(e) => tracing::warn!("Capsule write patch_bundle failed: {e}"),
                    }
                }
                if let Some(ref cs) = engine_result.conflict_summary_json {
                    match capsule.write_conflict_summary(run_id, cs) {
                        Ok(uri) => capsule_uris.push(uri),
                        Err(e) => tracing::warn!("Capsule write conflict_summary failed: {e}"),
                    }
                }
            }
        }

        // Persist checkpoints to capsule
        {
            let cap = self.capsule.lock().await;
            if let Some(capsule) = cap.as_ref() {
                for cp in &engine_result.checkpoints {
                    match capsule.write_checkpoint(cp) {
                        Ok(uri) => capsule_uris.push(uri),
                        Err(e) => tracing::warn!("Capsule write checkpoint failed: {e}"),
                    }
                }
            }
        }

        let artifact_uris = self.artifact_uris(run_id, &capsule_uris).await;

        // Update in-memory record
        {
            let mut runs = self.runs.lock().await;
            if let Some(record) = runs.get_mut(run_id) {
                record.state = engine_result.state;
                record.started_at = Some(engine_result.log.started_at.clone());
                record.finished_at = Some(engine_result.log.finished_at.clone());
                record.summary = Some(engine_result.summary.clone());
                record.report_json = Some(engine_result.report_json.clone());
                record.cached_artifact_uris = artifact_uris.clone();
            }
        }

        // Remove from active index
        {
            let mut index = self.active_index.lock().await;
            let key = (workspace, request.work_item_id.clone(), request.kind);
            index.remove(&key);
        }

        // Broadcast terminal notification
        let notification = BotTerminalNotification {
            run_id: run_id.to_string(),
            status: engine_result.state,
            exit_code: engine_result.exit_code,
            summary: engine_result.summary.clone(),
            artifact_uris: artifact_uris.clone(),
        };
        let _ = self.terminal_tx.send(notification);

        Ok(BotRunResult {
            run_id: run_id.to_string(),
            status: engine_result.state,
            work_item_id: request.work_item_id,
            kind: request.kind,
            exit_code: engine_result.exit_code,
            summary: Some(engine_result.summary),
            artifact_uris,
        })
    }

    /// Resume all incomplete runs found on disk.
    ///
    /// Called at startup before accepting connections.
    pub async fn resume_incomplete(&self) {
        let incomplete = match self.store.scan_incomplete() {
            Ok(runs) => runs,
            Err(e) => {
                tracing::warn!("Failed to scan incomplete runs: {e}");
                return;
            }
        };

        if incomplete.is_empty() {
            tracing::info!("No incomplete runs to resume");
            return;
        }

        tracing::info!("Resuming {} incomplete run(s)", incomplete.len());

        for run in incomplete {
            tracing::info!("Resuming run {} ({})", run.run_id, run.request.work_item_id);
            match self.resume(&run.run_id, &run.workspace_path).await {
                Ok(result) => {
                    tracing::info!("Resumed run {}: {:?}", run.run_id, result.status);
                }
                Err(e) => {
                    tracing::warn!("Failed to resume run {}: {e}", run.run_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Arc<PersistenceStore> {
        let tmp = tempfile::TempDir::new().unwrap();
        // Leak the TempDir so it isn't cleaned up before the test ends
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        Arc::new(PersistenceStore::with_base_dir(path).unwrap())
    }

    fn test_params(work_item_id: &str, kind: BotKind) -> BotRunParams {
        BotRunParams {
            workspace_path: "/tmp/test-workspace".to_string(),
            work_item_id: work_item_id.to_string(),
            kind,
            capture_mode: BotCaptureMode::PromptsOnly,
            write_mode: codex_core::pm::bot::BotWriteMode::None,
            intensity: None,
            rebase_target: None,
            subscribe: false,
            allow_degraded: None,
            notebooklm_health_url: None,
        }
    }

    #[tokio::test]
    async fn submit_and_status() {
        let mgr = BotRunManager::new(test_store());
        let result = mgr
            .submit(test_params("SPEC-TEST-001", BotKind::Research))
            .await;
        let result = match result {
            Ok(r) => r,
            Err(e) => panic!("submit failed: {e}"),
        };
        assert_eq!(result.status, BotRunState::Succeeded);

        let status = mgr
            .status("/tmp/test-workspace", "SPEC-TEST-001", None)
            .await;
        assert_eq!(status.runs.len(), 1);
        assert_eq!(status.runs[0].status, BotRunState::Succeeded);
    }

    #[tokio::test]
    async fn reject_capture_none() {
        let mgr = BotRunManager::new(test_store());
        let mut params = test_params("SPEC-TEST-001", BotKind::Research);
        params.capture_mode = BotCaptureMode::None;

        let result = mgr.submit(params).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("capture_mode=none")
        );
    }

    #[tokio::test]
    async fn cross_kind_allowed() {
        let mgr = BotRunManager::new(test_store());

        let r1 = mgr
            .submit(test_params("SPEC-TEST-001", BotKind::Research))
            .await;
        assert!(r1.is_ok());

        let r2 = mgr
            .submit(test_params("SPEC-TEST-001", BotKind::Review))
            .await;
        assert!(r2.is_ok());
    }

    #[tokio::test]
    async fn uptime_and_active_count() {
        let mgr = BotRunManager::new(test_store());
        assert!(mgr.uptime_s() < 2);
        assert_eq!(mgr.active_run_count().await, 0);
    }

    #[tokio::test]
    async fn test_show_returns_record() {
        let mgr = BotRunManager::new(test_store());
        let result = mgr
            .submit(test_params("SPEC-TEST-001", BotKind::Research))
            .await
            .expect("submit should succeed");

        let show = mgr.show(&result.run_id).await.expect("show should succeed");
        assert_eq!(show.run_id, result.run_id);
        assert_eq!(show.work_item_id, "SPEC-TEST-001");
        assert_eq!(show.kind, BotKind::Research);
        assert_eq!(show.status, BotRunState::Succeeded);
        assert!(show.started_at.is_some());
        assert!(show.report_json.is_some());
    }

    #[tokio::test]
    async fn test_show_not_found() {
        let mgr = BotRunManager::new(test_store());
        let result = mgr.show("nonexistent-run-id").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_list_runs_filters_workspace() {
        let mgr = BotRunManager::new(test_store());

        // Submit to workspace A
        let mut params_a = test_params("SPEC-TEST-001", BotKind::Research);
        params_a.workspace_path = "/tmp/workspace-a".to_string();
        mgr.submit(params_a).await.expect("submit a");

        // Submit to workspace B
        let mut params_b = test_params("SPEC-TEST-001", BotKind::Review);
        params_b.workspace_path = "/tmp/workspace-b".to_string();
        mgr.submit(params_b).await.expect("submit b");

        let runs_a = mgr
            .list_runs("/tmp/workspace-a", "SPEC-TEST-001", 10, 0)
            .await;
        assert_eq!(runs_a.runs.len(), 1);
        assert_eq!(runs_a.total, 1);
        assert_eq!(runs_a.runs[0].kind, BotKind::Research);

        let runs_b = mgr
            .list_runs("/tmp/workspace-b", "SPEC-TEST-001", 10, 0)
            .await;
        assert_eq!(runs_b.runs.len(), 1);
        assert_eq!(runs_b.total, 1);
        assert_eq!(runs_b.runs[0].kind, BotKind::Review);
    }

    #[tokio::test]
    async fn test_list_runs_pagination() {
        let mgr = BotRunManager::new(test_store());

        mgr.submit(test_params("SPEC-TEST-001", BotKind::Research))
            .await
            .expect("submit 1");
        mgr.submit(test_params("SPEC-TEST-001", BotKind::Review))
            .await
            .expect("submit 2");

        let page1 = mgr
            .list_runs("/tmp/test-workspace", "SPEC-TEST-001", 1, 0)
            .await;
        assert_eq!(page1.runs.len(), 1);
        assert_eq!(page1.total, 2);

        let page2 = mgr
            .list_runs("/tmp/test-workspace", "SPEC-TEST-001", 1, 1)
            .await;
        assert_eq!(page2.runs.len(), 1);
        assert_eq!(page2.total, 2);

        assert_ne!(page1.runs[0].run_id, page2.runs[0].run_id);
    }

    // ── Persistence-specific tests ───────────────────────────────────────

    #[tokio::test]
    async fn submit_persists_request_on_disk() {
        let store = test_store();
        let mgr = BotRunManager::new(Arc::clone(&store));

        let result = mgr
            .submit(test_params("SPEC-TEST-P01", BotKind::Research))
            .await
            .expect("submit should succeed");

        // Verify request.json exists on disk
        let (req, ws) = store.read_request(&result.run_id).unwrap();
        assert_eq!(req.work_item_id, "SPEC-TEST-P01");
        assert_eq!(ws, "/tmp/test-workspace");
    }

    #[tokio::test]
    async fn submit_persists_terminal_artifacts() {
        let store = test_store();
        let mgr = BotRunManager::new(Arc::clone(&store));

        let result = mgr
            .submit(test_params("SPEC-TEST-P02", BotKind::Research))
            .await
            .expect("submit should succeed");

        // Verify log.json and report.json exist
        let log = store.read_log(&result.run_id).unwrap();
        assert!(log.is_some());
        assert_eq!(log.unwrap().state, BotRunState::Succeeded);

        let report = store.read_report(&result.run_id).unwrap();
        assert!(report.is_some());
    }

    #[tokio::test]
    async fn show_returns_artifact_uris() {
        let store = test_store();
        let mgr = BotRunManager::new(Arc::clone(&store));

        let result = mgr
            .submit(test_params("SPEC-TEST-P03", BotKind::Research))
            .await
            .expect("submit should succeed");

        let show = mgr.show(&result.run_id).await.unwrap();
        assert!(!show.artifact_uris.is_empty());
        assert!(show.artifact_uris.iter().any(|u| u.contains("/request")));
        assert!(show.artifact_uris.iter().any(|u| u.contains("/log")));
        assert!(show.artifact_uris.iter().any(|u| u.contains("/report")));
    }

    #[tokio::test]
    async fn resume_re_executes_and_creates_artifacts() {
        let store = test_store();

        // Manually write an incomplete request (simulating crash before engine)
        let request = BotRunRequest {
            schema_version: BotRunRequest::SCHEMA_VERSION.to_string(),
            run_id: "resume-test-001".to_string(),
            work_item_id: "SPEC-RESUME-001".to_string(),
            kind: BotKind::Research,
            capture_mode: BotCaptureMode::PromptsOnly,
            write_mode: codex_core::pm::bot::BotWriteMode::None,
            requested_at: "2026-02-09T12:00:00Z".to_string(),
            trigger: None,
        };
        store.write_request(&request, "/tmp/ws").unwrap();

        // No log.json → incomplete
        let mgr = BotRunManager::new(Arc::clone(&store));
        let result = mgr.resume("resume-test-001", "/tmp/ws").await.unwrap();

        assert_eq!(result.status, BotRunState::Succeeded);

        // Terminal artifacts should now exist
        let log = store.read_log("resume-test-001").unwrap();
        assert!(log.is_some());
        assert_eq!(log.unwrap().state, BotRunState::Succeeded);

        let report = store.read_report("resume-test-001").unwrap();
        assert!(report.is_some());
    }

    #[tokio::test]
    async fn resume_fails_for_terminal_run() {
        let store = test_store();
        let mgr = BotRunManager::new(Arc::clone(&store));

        // Submit a run (which completes immediately)
        let result = mgr
            .submit(test_params("SPEC-TEST-P05", BotKind::Research))
            .await
            .expect("submit should succeed");

        // Try to resume a terminal run
        let resume_result = mgr.resume(&result.run_id, "/tmp/test-workspace").await;
        assert!(resume_result.is_err());
        assert!(
            resume_result
                .unwrap_err()
                .to_string()
                .contains("already terminal")
        );
    }
}
