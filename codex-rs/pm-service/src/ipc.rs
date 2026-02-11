//! Unix domain socket IPC listener (PM-D7, PM-D8, PM-D9).
//!
//! Listens on the socket path and dispatches JSON-RPC-lite messages
//! to the BotRunManager. Supports push notifications for --wait (PM-D24).

use std::sync::Arc;

use codex_app_server_protocol::{
    JSONRPCError, JSONRPCErrorError, JSONRPCNotification, JSONRPCRequest, JSONRPCResponse,
    RequestId,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::manager::BotRunManager;
use crate::protocol::*;
use crate::{PROTOCOL_VERSION, default_socket_path};

/// Start the IPC listener on a pre-bound listener with shutdown support.
///
/// Accepts connections, dispatches to the manager, tracks connections,
/// and exits when `shutdown_rx` signals.
pub async fn serve(
    manager: Arc<BotRunManager>,
    listener: UnixListener,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> std::io::Result<()> {
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _addr)) => {
                        let mgr = Arc::clone(&manager);
                        mgr.inc_connections();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(Arc::clone(&mgr), stream).await {
                                tracing::warn!("Connection error: {e}");
                            }
                            mgr.dec_connections();
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {e}");
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("IPC server shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Bind and serve on a socket path (convenience for tests).
///
/// Creates the listener, then delegates to `serve()`. The shutdown
/// receiver never fires, so this runs until the task is aborted.
pub async fn serve_path(
    manager: Arc<BotRunManager>,
    socket_path: &std::path::Path,
) -> std::io::Result<()> {
    let path = socket_path.to_path_buf();

    // Clean up stale socket file
    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(&path)?;
    tracing::info!("PM service listening on {}", path.display());

    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    serve(manager, listener, shutdown_rx).await
}

/// Handle a single client connection.
///
/// Reads newline-delimited JSON-RPC messages, sends responses,
/// and supports push notifications for subscribed bot.run requests.
async fn handle_connection(manager: Arc<BotRunManager>, stream: UnixStream) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Track active subscription for --wait support
    let mut pending_subscription: Option<(
        String,
        tokio::sync::broadcast::Receiver<BotTerminalNotification>,
    )> = None;

    loop {
        // If we have a pending subscription, select between client input and notification
        if let Some((ref sub_run_id, ref mut rx)) = pending_subscription {
            let run_id = sub_run_id.clone();
            tokio::select! {
                // Client sends another message (or disconnects)
                read_result = async {
                    line.clear();
                    reader.read_line(&mut line).await
                } => {
                    let n = read_result?;
                    if n == 0 {
                        break; // EOF
                    }
                    // Process the message normally (fall through below)
                    pending_subscription = None;
                }
                // Terminal notification arrives
                notif_result = rx.recv() => {
                    match notif_result {
                        Ok(notif) if notif.run_id == run_id => {
                            let bytes = encode_notification(
                                "bot.terminal",
                                serde_json::to_value(&notif).unwrap_or_default(),
                            );
                            writer.write_all(&bytes).await?;
                            writer.flush().await?;
                            pending_subscription = None;
                            continue;
                        }
                        Ok(_) => continue, // Not our run, keep waiting
                        Err(_) => {
                            // Channel closed — no more notifications possible
                            pending_subscription = None;
                            continue;
                        }
                    }
                }
            }
        } else {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        manager.touch_activity().await;

        // Check if this is a bot.run with subscribe: true
        let wants_subscribe = check_subscribe(trimmed);

        let response = dispatch_message(&manager, trimmed).await;
        let mut response_bytes = serde_json::to_vec(&response).unwrap_or_else(|_| b"{}".to_vec());
        response_bytes.push(b'\n');
        writer.write_all(&response_bytes).await?;
        writer.flush().await?;

        // If this was a subscribed bot.run, set up push notification
        if wants_subscribe
            && let Some(result) = response.get("result")
            && let Some(run_id) = result.get("run_id").and_then(|v| v.as_str())
        {
            let status_str = result.get("status").and_then(|v| v.as_str()).unwrap_or("");
            // Check if already terminal (sync stub case)
            let already_terminal = matches!(
                status_str,
                "succeeded" | "needs_attention" | "blocked" | "failed" | "cancelled"
            );

            if already_terminal {
                // Synthesize terminal notification from the run result
                let exit_code = result
                    .get("exit_code")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0) as i32;
                let summary = result
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Run completed synchronously")
                    .to_string();
                let artifact_uris = result
                    .get("artifact_uris")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let notif = BotTerminalNotification {
                    run_id: run_id.to_string(),
                    status: serde_json::from_value(
                        result.get("status").cloned().unwrap_or_default(),
                    )
                    .unwrap_or(codex_core::pm::artifacts::BotRunState::Succeeded),
                    exit_code,
                    summary,
                    artifact_uris,
                };
                let bytes = encode_notification(
                    "bot.terminal",
                    serde_json::to_value(&notif).unwrap_or_default(),
                );
                writer.write_all(&bytes).await?;
                writer.flush().await?;
            } else {
                // Subscribe and wait for async completion
                let rx = manager.subscribe_terminal();
                pending_subscription = Some((run_id.to_string(), rx));
            }
        }
    }

    Ok(())
}

/// Check if a raw JSON-RPC message is a bot.run with subscribe: true.
fn check_subscribe(raw: &str) -> bool {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        v.get("method").and_then(|m| m.as_str()) == Some("bot.run")
            && v.get("params")
                .and_then(|p| p.get("subscribe"))
                .and_then(serde_json::Value::as_bool)
                == Some(true)
    } else {
        false
    }
}

/// Parse and dispatch a single JSON-RPC message.
async fn dispatch_message(manager: &BotRunManager, raw: &str) -> serde_json::Value {
    // Parse the request
    let request: JSONRPCRequest = match serde_json::from_str(raw) {
        Ok(req) => req,
        Err(e) => {
            return serde_json::to_value(JSONRPCError {
                id: RequestId::Integer(0),
                error: JSONRPCErrorError {
                    code: ERR_INVALID_REQUEST,
                    message: format!("Invalid JSON-RPC: {e}"),
                    data: None,
                },
            })
            .unwrap_or_default();
        }
    };

    let id = request.id.clone();
    let result = dispatch_method(manager, &request.method, request.params).await;

    match result {
        Ok(value) => {
            serde_json::to_value(JSONRPCResponse { id, result: value }).unwrap_or_default()
        }
        Err((code, message)) => serde_json::to_value(JSONRPCError {
            id,
            error: JSONRPCErrorError {
                code,
                message,
                data: None,
            },
        })
        .unwrap_or_default(),
    }
}

/// Dispatch to the appropriate handler based on method name.
async fn dispatch_method(
    manager: &BotRunManager,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, (i64, String)> {
    match method {
        "hello" => handle_hello(params),
        "bot.run" => handle_bot_run(manager, params).await,
        "bot.status" => handle_bot_status(manager, params).await,
        "bot.show" => handle_bot_show(manager, params).await,
        "bot.runs" => handle_bot_runs(manager, params).await,
        "bot.cancel" => handle_bot_cancel(manager, params).await,
        "bot.resume" => handle_bot_resume(manager, params).await,
        "service.status" => handle_service_status(manager).await,
        "service.doctor" => handle_service_doctor(params),
        _ => Err((ERR_METHOD_NOT_FOUND, format!("Unknown method: {method}"))),
    }
}

/// Handle the `hello` handshake (PM-D9).
fn handle_hello(params: Option<serde_json::Value>) -> Result<serde_json::Value, (i64, String)> {
    let hello: HelloParams = params
        .ok_or_else(|| (ERR_INVALID_PARAMS, "Missing params".to_string()))
        .and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| (ERR_INVALID_PARAMS, format!("Invalid hello params: {e}")))
        })?;

    // Version compatibility check
    if hello.protocol_version != PROTOCOL_VERSION {
        return Err((
            ERR_INVALID_PARAMS,
            format!(
                "Incompatible protocol version: client={}, service={}",
                hello.protocol_version, PROTOCOL_VERSION
            ),
        ));
    }

    let result = HelloResult {
        protocol_version: PROTOCOL_VERSION.to_string(),
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
            "bot.run".to_string(),
            "bot.status".to_string(),
            "bot.show".to_string(),
            "bot.runs".to_string(),
            "bot.cancel".to_string(),
            "bot.resume".to_string(),
            "service.status".to_string(),
            "service.doctor".to_string(),
        ],
    };

    serde_json::to_value(result).map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Handle `bot.run`.
async fn handle_bot_run(
    manager: &BotRunManager,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, (i64, String)> {
    let params: BotRunParams = params
        .ok_or_else(|| (ERR_INVALID_PARAMS, "Missing params".to_string()))
        .and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| (ERR_INVALID_PARAMS, format!("Invalid bot.run params: {e}")))
        })?;

    let result = manager
        .submit(params)
        .await
        .map_err(|e| manager_error_to_rpc(&e))?;

    serde_json::to_value(result).map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Handle `bot.status`.
async fn handle_bot_status(
    manager: &BotRunManager,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, (i64, String)> {
    let params: BotStatusParams = params
        .ok_or_else(|| (ERR_INVALID_PARAMS, "Missing params".to_string()))
        .and_then(|v| {
            serde_json::from_value(v).map_err(|e| {
                (
                    ERR_INVALID_PARAMS,
                    format!("Invalid bot.status params: {e}"),
                )
            })
        })?;

    let result = manager
        .status(&params.workspace_path, &params.work_item_id, params.kind)
        .await;

    serde_json::to_value(result).map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Handle `bot.show`.
async fn handle_bot_show(
    manager: &BotRunManager,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, (i64, String)> {
    let params: BotShowParams = params
        .ok_or_else(|| (ERR_INVALID_PARAMS, "Missing params".to_string()))
        .and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| (ERR_INVALID_PARAMS, format!("Invalid bot.show params: {e}")))
        })?;

    let result = manager
        .show(&params.run_id)
        .await
        .map_err(|e| manager_error_to_rpc(&e))?;

    serde_json::to_value(result).map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Handle `bot.runs` (list).
async fn handle_bot_runs(
    manager: &BotRunManager,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, (i64, String)> {
    let params: BotRunsParams = params
        .ok_or_else(|| (ERR_INVALID_PARAMS, "Missing params".to_string()))
        .and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| (ERR_INVALID_PARAMS, format!("Invalid bot.runs params: {e}")))
        })?;

    let limit = params.limit.unwrap_or(10);
    let offset = params.offset.unwrap_or(0);
    let result = manager
        .list_runs(&params.workspace_path, &params.work_item_id, limit, offset)
        .await;

    serde_json::to_value(result).map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Handle `bot.cancel`.
async fn handle_bot_cancel(
    manager: &BotRunManager,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, (i64, String)> {
    let params: BotCancelParams = params
        .ok_or_else(|| (ERR_INVALID_PARAMS, "Missing params".to_string()))
        .and_then(|v| {
            serde_json::from_value(v).map_err(|e| {
                (
                    ERR_INVALID_PARAMS,
                    format!("Invalid bot.cancel params: {e}"),
                )
            })
        })?;

    let state = manager
        .cancel(&params.workspace_path, &params.work_item_id, &params.run_id)
        .await
        .map_err(|e| manager_error_to_rpc(&e))?;

    serde_json::to_value(serde_json::json!({
        "run_id": params.run_id,
        "status": state,
    }))
    .map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Handle `bot.resume`.
async fn handle_bot_resume(
    manager: &BotRunManager,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, (i64, String)> {
    let params: BotResumeParams = params
        .ok_or_else(|| (ERR_INVALID_PARAMS, "Missing params".to_string()))
        .and_then(|v| {
            serde_json::from_value(v).map_err(|e| {
                (
                    ERR_INVALID_PARAMS,
                    format!("Invalid bot.resume params: {e}"),
                )
            })
        })?;

    let result = manager
        .resume(&params.run_id, &params.workspace_path)
        .await
        .map_err(|e| manager_error_to_rpc(&e))?;

    serde_json::to_value(result).map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Handle `service.status`.
async fn handle_service_status(
    manager: &BotRunManager,
) -> Result<serde_json::Value, (i64, String)> {
    let result = ServiceStatusResult {
        uptime_s: manager.uptime_s(),
        active_runs: manager.active_run_count().await,
        workspaces: manager.active_workspaces().await,
    };

    serde_json::to_value(result).map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Handle `service.doctor`.
fn handle_service_doctor(
    _params: Option<serde_json::Value>,
) -> Result<serde_json::Value, (i64, String)> {
    let checks = vec![
        DoctorCheck {
            name: "service".to_string(),
            status: "ok".to_string(),
            detail: Some("Service is running".to_string()),
        },
        DoctorCheck {
            name: "socket".to_string(),
            status: "ok".to_string(),
            detail: Some(format!("Listening on {}", default_socket_path().display())),
        },
    ];

    let result = ServiceDoctorResult { checks };
    serde_json::to_value(result).map_err(|e| (ERR_INFRA, format!("Serialize error: {e}")))
}

/// Map ManagerError to JSON-RPC error (code, message).
fn manager_error_to_rpc(err: &crate::manager::ManagerError) -> (i64, String) {
    use crate::manager::ManagerError;
    match err {
        ManagerError::CaptureNoneRejected => (ERR_NEEDS_INPUT, err.to_string()),
        ManagerError::DuplicateRun { .. } => (ERR_DUPLICATE_RUN, err.to_string()),
        ManagerError::RunNotFound { .. } => (ERR_INVALID_PARAMS, err.to_string()),
        ManagerError::AlreadyTerminal { .. } => (ERR_INVARIANT, err.to_string()),
        ManagerError::InvalidRequest { .. } => (ERR_INVALID_PARAMS, err.to_string()),
        ManagerError::Infra(_) => (ERR_INFRA, err.to_string()),
    }
}

/// Encode a notification (no id, no response expected) for a client.
///
/// Used for `bot.progress` and `bot.terminal` push notifications.
pub fn encode_notification(method: &str, params: serde_json::Value) -> Vec<u8> {
    let notif = JSONRPCNotification {
        method: method.to_string(),
        params: Some(params),
    };
    let mut bytes = serde_json::to_vec(&notif).unwrap_or_else(|_| b"{}".to_vec());
    bytes.push(b'\n');
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::PersistenceStore;

    fn test_manager() -> BotRunManager {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let store = Arc::new(PersistenceStore::with_base_dir(path).unwrap());
        BotRunManager::new(store)
    }

    #[tokio::test]
    async fn dispatch_hello() {
        let manager = test_manager();
        let hello_json = serde_json::json!({
            "id": 0,
            "method": "hello",
            "params": {
                "protocol_version": "1.0",
                "client_version": "0.1.0"
            }
        });

        let result = dispatch_message(&manager, &hello_json.to_string()).await;
        let result_obj = result
            .as_object()
            .unwrap_or_else(|| panic!("Expected object"));
        assert!(
            result_obj.contains_key("result"),
            "Expected result field, got: {result}"
        );
    }

    #[tokio::test]
    async fn dispatch_unknown_method() {
        let manager = test_manager();
        let msg = serde_json::json!({
            "id": 1,
            "method": "unknown.method",
        });

        let result = dispatch_message(&manager, &msg.to_string()).await;
        let result_obj = result
            .as_object()
            .unwrap_or_else(|| panic!("Expected object"));
        assert!(result_obj.contains_key("error"), "Expected error field");
    }

    #[tokio::test]
    async fn dispatch_service_status() {
        let manager = test_manager();
        let msg = serde_json::json!({
            "id": 2,
            "method": "service.status",
        });

        let result = dispatch_message(&manager, &msg.to_string()).await;
        let result_obj = result
            .as_object()
            .unwrap_or_else(|| panic!("Expected object"));
        assert!(
            result_obj.contains_key("result"),
            "Expected result field, got: {result}"
        );
    }

    #[tokio::test]
    async fn dispatch_bot_run() {
        let manager = test_manager();
        let msg = serde_json::json!({
            "id": 3,
            "method": "bot.run",
            "params": {
                "workspace_path": "/tmp/test",
                "work_item_id": "SPEC-TEST-001",
                "kind": "research",
                "capture_mode": "prompts_only"
            }
        });

        let result = dispatch_message(&manager, &msg.to_string()).await;
        let result_obj = result
            .as_object()
            .unwrap_or_else(|| panic!("Expected object"));
        assert!(
            result_obj.contains_key("result"),
            "Expected result, got: {result}"
        );
    }

    #[tokio::test]
    async fn dispatch_bot_show() {
        let manager = test_manager();

        // First submit a run
        let run_msg = serde_json::json!({
            "id": 1,
            "method": "bot.run",
            "params": {
                "workspace_path": "/tmp/test",
                "work_item_id": "SPEC-TEST-001",
                "kind": "research",
                "capture_mode": "prompts_only"
            }
        });
        let run_resp = dispatch_message(&manager, &run_msg.to_string()).await;
        let run_id = run_resp["result"]["run_id"].as_str().unwrap();

        // Now show it
        let show_msg = serde_json::json!({
            "id": 2,
            "method": "bot.show",
            "params": {
                "workspace_path": "/tmp/test",
                "work_item_id": "SPEC-TEST-001",
                "run_id": run_id
            }
        });
        let result = dispatch_message(&manager, &show_msg.to_string()).await;
        assert!(
            result.get("result").is_some(),
            "Expected result, got: {result}"
        );
        assert_eq!(result["result"]["run_id"].as_str().unwrap(), run_id);
        assert_eq!(result["result"]["status"].as_str().unwrap(), "succeeded");
    }

    #[tokio::test]
    async fn dispatch_bot_runs() {
        let manager = test_manager();

        // Submit a run
        let run_msg = serde_json::json!({
            "id": 1,
            "method": "bot.run",
            "params": {
                "workspace_path": "/tmp/test",
                "work_item_id": "SPEC-TEST-001",
                "kind": "research",
                "capture_mode": "prompts_only"
            }
        });
        dispatch_message(&manager, &run_msg.to_string()).await;

        // List runs
        let list_msg = serde_json::json!({
            "id": 2,
            "method": "bot.runs",
            "params": {
                "workspace_path": "/tmp/test",
                "work_item_id": "SPEC-TEST-001"
            }
        });
        let result = dispatch_message(&manager, &list_msg.to_string()).await;
        assert!(
            result.get("result").is_some(),
            "Expected result, got: {result}"
        );
        assert_eq!(result["result"]["runs"].as_array().unwrap().len(), 1);
        assert_eq!(result["result"]["total"].as_u64().unwrap(), 1);
    }

    #[tokio::test]
    async fn dispatch_bot_resume_not_found() {
        let manager = test_manager();
        let msg = serde_json::json!({
            "id": 1,
            "method": "bot.resume",
            "params": {
                "workspace_path": "/tmp/test",
                "work_item_id": "SPEC-TEST-001",
                "run_id": "nonexistent-run-id"
            }
        });

        let result = dispatch_message(&manager, &msg.to_string()).await;
        assert!(
            result.get("error").is_some(),
            "Expected error, got: {result}"
        );
        // ERR_INVALID_PARAMS for RunNotFound
        assert_eq!(
            result["error"]["code"].as_i64().unwrap(),
            ERR_INVALID_PARAMS
        );
        assert!(
            result["error"]["message"]
                .as_str()
                .unwrap()
                .contains("not found")
        );
    }
}
