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

use codex_core::pm::artifacts::BotRunState;
use codex_core::pm::bot::{BotCaptureMode, BotKind, BotRunRequest};
use tokio::sync::{Mutex, broadcast};

use crate::engine;
use crate::protocol::{
    BotRunParams, BotRunResult, BotStatusResult, BotTerminalNotification, RunSummary,
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

    #[error("invalid request: {reason}")]
    InvalidRequest { reason: String },

    #[error("infrastructure error: {0}")]
    Infra(String),
}

/// Tracks an active or completed run.
#[derive(Debug, Clone)]
struct RunRecord {
    request: BotRunRequest,
    state: BotRunState,
    started_at: Option<String>,
    finished_at: Option<String>,
    summary: Option<String>,
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
    started_at: std::time::Instant,
}

impl Default for BotRunManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BotRunManager {
    pub fn new() -> Self {
        let (terminal_tx, _) = broadcast::channel(64);
        Self {
            runs: Arc::new(Mutex::new(HashMap::new())),
            active_index: Arc::new(Mutex::new(HashMap::new())),
            terminal_tx,
            started_at: std::time::Instant::now(),
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

    /// Submit a new bot run (PM-D11).
    ///
    /// Validates the request, checks for duplicates (PM-D14), rejects
    /// capture=none (PM-D16), then executes the stub engine.
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

        // Record the run as queued
        let record = RunRecord {
            request: request.clone(),
            state: BotRunState::Queued,
            started_at: None,
            finished_at: None,
            summary: None,
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

        // Execute the stub engine (Phase-0: synchronous, immediate)
        let engine_result = engine::run_stub(params.kind, &run_id, &params.work_item_id);

        // Update run record with results
        {
            let mut runs = self.runs.lock().await;
            if let Some(record) = runs.get_mut(&run_id) {
                record.state = engine_result.state;
                record.started_at = Some(engine_result.log.started_at.clone());
                record.finished_at = Some(engine_result.log.finished_at.clone());
                record.summary = Some(engine_result.summary.clone());
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
            summary: engine_result.summary,
            artifact_uris: vec![],
        };
        // Ignore error if no subscribers
        let _ = self.terminal_tx.send(notification);

        Ok(BotRunResult {
            run_id,
            status: engine_result.state,
            work_item_id: params.work_item_id,
            kind: params.kind,
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
                r.request.work_item_id == work_item_id && kind.is_none_or(|k| r.request.kind == k)
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

        // Note: workspace filtering would be done here when we track workspace per run
        let _ = workspace;

        BotStatusResult { runs: summaries }
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

        record.state = BotRunState::Cancelled;
        record.finished_at = Some(chrono::Utc::now().to_rfc3339());
        record.summary = Some("Cancelled by user".to_string());

        Ok(BotRunState::Cancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        }
    }

    #[tokio::test]
    async fn submit_and_status() {
        let mgr = BotRunManager::new();
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
        let mgr = BotRunManager::new();
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
        let mgr = BotRunManager::new();

        // Research completes immediately (stub), so we can't test true concurrency.
        // But we verify that submitting different kinds for the same item works.
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
        let mgr = BotRunManager::new();
        assert!(mgr.uptime_s() < 2);
        assert_eq!(mgr.active_run_count().await, 0);
    }
}
