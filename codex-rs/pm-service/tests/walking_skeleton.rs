#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Walking-skeleton end-to-end test (Phase-0 Step 8).
//!
//! Proves the full pipeline:
//!   1. Start PM service on a temp socket
//!   2. Connect as client
//!   3. Send hello handshake
//!   4. Submit a bot run (research)
//!   5. Verify run succeeds
//!   6. Query bot status → shows completed
//!   7. Query service status → healthy
//!   8. Query service doctor → all checks pass

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;

use codex_pm_service::PROTOCOL_VERSION;
use codex_pm_service::manager::BotRunManager;

/// Stateful client that keeps a single connection + buffered reader.
struct TestClient {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl TestClient {
    fn connect(socket_path: &std::path::Path) -> Self {
        let stream = UnixStream::connect(socket_path).expect("Failed to connect to PM service");
        let writer = stream.try_clone().expect("clone stream");
        let reader = BufReader::new(stream);
        Self { writer, reader }
    }

    fn rpc(&mut self, msg: serde_json::Value) -> serde_json::Value {
        let mut bytes = serde_json::to_vec(&msg).expect("serialize");
        bytes.push(b'\n');
        self.writer.write_all(&bytes).expect("write");
        self.writer.flush().expect("flush");

        let mut line = String::new();
        self.reader.read_line(&mut line).expect("read response");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("parse response: {e}\nraw: {line}"))
    }

    fn handshake(&mut self) {
        let resp = self.rpc(serde_json::json!({
            "id": 0,
            "method": "hello",
            "params": {
                "protocol_version": PROTOCOL_VERSION,
                "client_version": "test-0.1.0"
            }
        }));
        assert!(resp.get("result").is_some(), "Hello should succeed: {resp}");
    }
}

/// Start the service in the background and return a handle.
async fn start_service(
    socket_path: &std::path::Path,
) -> (Arc<BotRunManager>, tokio::task::JoinHandle<()>) {
    let manager = Arc::new(BotRunManager::new());
    let mgr_clone = Arc::clone(&manager);
    let sock = socket_path.to_path_buf();

    let handle = tokio::spawn(async move {
        codex_pm_service::ipc::serve(mgr_clone, Some(&sock))
            .await
            .unwrap();
    });

    // Wait for socket to be ready
    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    (manager, handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn walking_skeleton_e2e() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm.sock");

    let (_manager, server_handle) = start_service(&socket_path).await;

    let mut client = TestClient::connect(&socket_path);

    // 1. Handshake
    client.handshake();

    // 2. Submit a bot run (research)
    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-TEST-001",
            "kind": "research",
            "capture_mode": "prompts_only"
        }
    }));
    assert!(
        run_resp.get("result").is_some(),
        "bot.run should succeed: {run_resp}"
    );
    let run_result = &run_resp["result"];
    assert_eq!(run_result["status"].as_str().unwrap(), "succeeded");
    assert_eq!(run_result["kind"].as_str().unwrap(), "research");
    let run_id = run_result["run_id"].as_str().unwrap();
    assert!(!run_id.is_empty(), "run_id should not be empty");

    // 3. Query bot status → shows completed
    let status_resp = client.rpc(serde_json::json!({
        "id": 2,
        "method": "bot.status",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-TEST-001"
        }
    }));
    assert!(
        status_resp.get("result").is_some(),
        "bot.status should succeed: {status_resp}"
    );
    let status_result = &status_resp["result"];
    let runs = status_result["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["status"].as_str().unwrap(), "succeeded");
    assert_eq!(runs[0]["run_id"].as_str().unwrap(), run_id);

    // 4. Query service status → healthy
    let svc_resp = client.rpc(serde_json::json!({
        "id": 3,
        "method": "service.status"
    }));
    assert!(
        svc_resp.get("result").is_some(),
        "service.status should succeed: {svc_resp}"
    );
    let svc_result = &svc_resp["result"];
    assert!(svc_result["uptime_s"].as_u64().is_some());
    assert_eq!(svc_result["active_runs"].as_u64().unwrap(), 0);

    // 5. Query service doctor → all checks pass
    let doc_resp = client.rpc(serde_json::json!({
        "id": 4,
        "method": "service.doctor"
    }));
    assert!(
        doc_resp.get("result").is_some(),
        "service.doctor should succeed: {doc_resp}"
    );
    let doc_result = &doc_resp["result"];
    let checks = doc_result["checks"].as_array().unwrap();
    assert!(!checks.is_empty());
    for check in checks {
        assert_eq!(
            check["status"].as_str().unwrap(),
            "ok",
            "Check {} should be ok",
            check["name"]
        );
    }

    // 6. Verify capture=none is rejected (PM-D16)
    let none_resp = client.rpc(serde_json::json!({
        "id": 5,
        "method": "bot.run",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-TEST-002",
            "kind": "research",
            "capture_mode": "none"
        }
    }));
    assert!(
        none_resp.get("error").is_some(),
        "capture=none should be rejected: {none_resp}"
    );
    assert_eq!(none_resp["error"]["code"].as_i64().unwrap(), 10); // ERR_NEEDS_INPUT

    // 7. Verify incompatible protocol version is rejected (PM-D9)
    let bad_hello = client.rpc(serde_json::json!({
        "id": 6,
        "method": "hello",
        "params": {
            "protocol_version": "99.0",
            "client_version": "test-0.1.0"
        }
    }));
    assert!(
        bad_hello.get("error").is_some(),
        "Incompatible version should be rejected: {bad_hello}"
    );

    // Clean up
    server_handle.abort();
}

/// Test that a review run also succeeds with the stub engine.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn walking_skeleton_review() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-review.sock");

    let (_manager, server_handle) = start_service(&socket_path).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // Submit review run
    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-TEST-001",
            "kind": "review",
            "capture_mode": "full_io",
            "write_mode": "worktree"
        }
    }));
    assert!(
        run_resp.get("result").is_some(),
        "Review run should succeed: {run_resp}"
    );
    assert_eq!(run_resp["result"]["status"].as_str().unwrap(), "succeeded");
    assert_eq!(run_resp["result"]["kind"].as_str().unwrap(), "review");

    server_handle.abort();
}

/// Test that unknown methods return proper errors.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn walking_skeleton_unknown_method() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-unknown.sock");

    let (_manager, server_handle) = start_service(&socket_path).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    let resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "nonexistent.method"
    }));
    assert!(
        resp.get("error").is_some(),
        "Unknown method should error: {resp}"
    );
    assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32601);

    server_handle.abort();
}
