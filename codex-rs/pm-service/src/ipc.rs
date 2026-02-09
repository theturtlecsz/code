//! Unix domain socket IPC listener (PM-D7, PM-D8, PM-D9).
//!
//! Listens on the socket path and dispatches JSON-RPC-lite messages
//! to the BotRunManager.

use std::path::Path;
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

/// Start the IPC listener on the given socket path.
///
/// Accepts connections, reads newline-delimited JSON-RPC messages,
/// dispatches to the manager, and writes responses.
pub async fn serve(manager: Arc<BotRunManager>, socket_path: Option<&Path>) -> std::io::Result<()> {
    let path = socket_path
        .map(Path::to_path_buf)
        .unwrap_or_else(default_socket_path);

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

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let mgr = Arc::clone(&manager);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(mgr, stream).await {
                        tracing::warn!("Connection error: {e}");
                    }
                });
            }
            Err(e) => {
                tracing::error!("Accept error: {e}");
            }
        }
    }
}

/// Handle a single client connection.
///
/// Reads newline-delimited JSON-RPC messages and sends responses.
async fn handle_connection(manager: Arc<BotRunManager>, stream: UnixStream) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = dispatch_message(&manager, trimmed).await;
        let mut response_bytes = serde_json::to_vec(&response).unwrap_or_else(|_| b"{}".to_vec());
        response_bytes.push(b'\n');
        writer.write_all(&response_bytes).await?;
        writer.flush().await?;
    }

    Ok(())
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
        "bot.cancel" => handle_bot_cancel(manager, params).await,
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
        ManagerError::InvalidRequest { .. } => (ERR_INVALID_PARAMS, err.to_string()),
        ManagerError::Infra(_) => (ERR_INFRA, err.to_string()),
    }
}

/// Send a notification (no id, no response expected) to a client.
///
/// Used for `bot.progress` and `bot.terminal` push notifications.
#[allow(dead_code)]
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

    #[tokio::test]
    async fn dispatch_hello() {
        let manager = BotRunManager::new();
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
        let manager = BotRunManager::new();
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
        let manager = BotRunManager::new();
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
        let manager = BotRunManager::new();
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
}
