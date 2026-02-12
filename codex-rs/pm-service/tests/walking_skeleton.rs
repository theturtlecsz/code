#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Walking-skeleton end-to-end test (Phase-0 Step 8, Phase-1.5).
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
//!   9. Verify persistence artifacts on disk
//!  10. Verify auto-resume of incomplete runs

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;

use codex_pm_service::PROTOCOL_VERSION;
use codex_pm_service::manager::BotRunManager;
use codex_pm_service::persistence::PersistenceStore;

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
    data_dir: &std::path::Path,
) -> (Arc<BotRunManager>, tokio::task::JoinHandle<()>) {
    let store = Arc::new(PersistenceStore::with_base_dir(data_dir.to_path_buf()).unwrap());
    let manager = Arc::new(BotRunManager::new(store));
    let mgr_clone = Arc::clone(&manager);
    let sock = socket_path.to_path_buf();

    let handle = tokio::spawn(async move {
        codex_pm_service::ipc::serve_path(mgr_clone, &sock)
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
    let data_dir = temp_dir.path().join("data");

    let (_manager, server_handle) = start_service(&socket_path, &data_dir).await;

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

/// Test that a read-only review run succeeds with findings.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn walking_skeleton_review() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-review.sock");
    let data_dir = temp_dir.path().join("data");
    let workspace_path = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    // Create a source file for the review engine to analyze
    std::fs::write(
        workspace_path.join("main.rs"),
        "fn main() {  \n    println!(\"hello\");\n}\n",
    )
    .unwrap();

    let (_manager, server_handle) = start_service(&socket_path, &data_dir).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // Submit read-only review run (write_mode=none)
    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-TEST-001",
            "kind": "review",
            "capture_mode": "full_io"
        }
    }));
    assert!(
        run_resp.get("result").is_some(),
        "Review run should succeed: {run_resp}"
    );
    assert_eq!(run_resp["result"]["status"].as_str().unwrap(), "succeeded");
    assert_eq!(run_resp["result"]["kind"].as_str().unwrap(), "review");

    // Verify report has findings
    let run_id = run_resp["result"]["run_id"].as_str().unwrap();
    let show_resp = client.rpc(serde_json::json!({
        "id": 2,
        "method": "bot.show",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-TEST-001",
            "run_id": run_id
        }
    }));
    assert!(show_resp.get("result").is_some());
    let report_json_str = show_resp["result"]["report_json"].as_str().unwrap();
    let report: serde_json::Value = serde_json::from_str(report_json_str).unwrap();
    assert!(
        !report["findings"].as_array().unwrap().is_empty(),
        "Review should produce findings"
    );
    assert!(!report["has_patches"].as_bool().unwrap_or(true));

    server_handle.abort();
}

/// Test that unknown methods return proper errors.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn walking_skeleton_unknown_method() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-unknown.sock");
    let data_dir = temp_dir.path().join("data");

    let (_manager, server_handle) = start_service(&socket_path, &data_dir).await;

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

/// Phase-1.5: Verify that submit persists artifacts to disk and bot.show returns URIs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persistence_roundtrip_e2e() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-persist.sock");
    let data_dir = temp_dir.path().join("data");

    let (_manager, server_handle) = start_service(&socket_path, &data_dir).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // Submit a run
    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-PERSIST-001",
            "kind": "research",
            "capture_mode": "prompts_only"
        }
    }));
    assert!(run_resp.get("result").is_some(), "bot.run should succeed");
    let run_id = run_resp["result"]["run_id"].as_str().unwrap();

    // Verify files on disk
    let run_dir = data_dir.join(run_id);
    assert!(
        run_dir.join("request.json").exists(),
        "request.json should exist"
    );
    assert!(run_dir.join("meta.json").exists(), "meta.json should exist");
    assert!(run_dir.join("log.json").exists(), "log.json should exist");
    assert!(
        run_dir.join("report.json").exists(),
        "report.json should exist"
    );

    // Verify request.json contents
    let req_data = std::fs::read_to_string(run_dir.join("request.json")).unwrap();
    let req: serde_json::Value = serde_json::from_str(&req_data).unwrap();
    assert_eq!(req["work_item_id"].as_str().unwrap(), "SPEC-PERSIST-001");
    assert_eq!(req["kind"].as_str().unwrap(), "research");

    // bot.show returns artifact_uris
    let show_resp = client.rpc(serde_json::json!({
        "id": 2,
        "method": "bot.show",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-PERSIST-001",
            "run_id": run_id
        }
    }));
    assert!(show_resp.get("result").is_some(), "bot.show should succeed");
    let show_result = &show_resp["result"];
    let uris = show_result["artifact_uris"].as_array().unwrap();
    assert!(!uris.is_empty(), "artifact_uris should not be empty");
    assert!(
        uris.iter()
            .any(|u| u.as_str().unwrap().contains("/request")),
        "Should contain request URI"
    );
    assert!(
        uris.iter().any(|u| u.as_str().unwrap().contains("/log")),
        "Should contain log URI"
    );
    assert!(
        uris.iter().any(|u| u.as_str().unwrap().contains("/report")),
        "Should contain report URI"
    );

    server_handle.abort();
}

/// Phase-1.5: Verify auto-resume after simulated restart.
///
/// 1. Write an incomplete request.json to disk (no log.json)
/// 2. Start a new manager with the same data dir
/// 3. Manager calls resume_incomplete()
/// 4. Verify log.json + report.json now exist
/// 5. bot.show succeeds with status=succeeded
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_resume_after_restart() {
    use codex_core::pm::bot::{BotCaptureMode, BotKind, BotRunRequest, BotWriteMode};

    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-resume.sock");
    let data_dir = temp_dir.path().join("data");

    // Step 1: Write incomplete request to disk (simulating crash before engine)
    let store = PersistenceStore::with_base_dir(data_dir.clone()).unwrap();
    let request = BotRunRequest {
        schema_version: BotRunRequest::SCHEMA_VERSION.to_string(),
        run_id: "resume-e2e-001".to_string(),
        work_item_id: "SPEC-RESUME-E2E".to_string(),
        kind: BotKind::Research,
        capture_mode: BotCaptureMode::PromptsOnly,
        write_mode: BotWriteMode::None,
        requested_at: "2026-02-09T12:00:00Z".to_string(),
        trigger: None,
    };
    store
        .write_request(&request, &temp_dir.path().to_string_lossy())
        .unwrap();

    // Verify: no log.json yet
    assert!(
        !data_dir.join("resume-e2e-001/log.json").exists(),
        "log.json should not exist before resume"
    );

    // Step 2: Start a new manager (simulating service restart)
    let store = Arc::new(PersistenceStore::with_base_dir(data_dir.clone()).unwrap());
    let manager = Arc::new(BotRunManager::new(Arc::clone(&store)));

    // Step 3: resume_incomplete
    manager.resume_incomplete().await;

    // Step 4: Verify terminal artifacts
    assert!(
        data_dir.join("resume-e2e-001/log.json").exists(),
        "log.json should exist after resume"
    );
    assert!(
        data_dir.join("resume-e2e-001/report.json").exists(),
        "report.json should exist after resume"
    );

    // Step 5: Start IPC and verify bot.show
    let mgr_clone = Arc::clone(&manager);
    let sock = socket_path.clone();
    let server_handle = tokio::spawn(async move {
        codex_pm_service::ipc::serve_path(mgr_clone, &sock)
            .await
            .unwrap();
    });

    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    let show_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.show",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-RESUME-E2E",
            "run_id": "resume-e2e-001"
        }
    }));
    assert!(
        show_resp.get("result").is_some(),
        "bot.show should succeed after resume: {show_resp}"
    );
    assert_eq!(
        show_resp["result"]["status"].as_str().unwrap(),
        "succeeded",
        "Resumed run should be succeeded"
    );
    assert_eq!(
        show_resp["result"]["run_id"].as_str().unwrap(),
        "resume-e2e-001"
    );

    // Verify artifact URIs are populated
    let uris = show_resp["result"]["artifact_uris"].as_array().unwrap();
    assert!(!uris.is_empty(), "artifact_uris should not be empty");

    server_handle.abort();
}

/// Start the service with a capsule persistence layer enabled.
async fn start_service_with_capsule(
    socket_path: &std::path::Path,
    data_dir: &std::path::Path,
    workspace_path: &std::path::Path,
) -> (Arc<BotRunManager>, tokio::task::JoinHandle<()>) {
    use codex_pm_service::persistence::CapsulePersistence;
    use codex_tui::memvid_adapter::CapsuleConfig;

    let store = Arc::new(PersistenceStore::with_base_dir(data_dir.to_path_buf()).unwrap());

    // Create capsule in the workspace dir
    let capsule_path = workspace_path.join(".speckit/memvid/workspace.mv2");
    if let Some(parent) = capsule_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "default".to_string(),
        ..Default::default()
    };
    let capsule = CapsulePersistence::open_with_config(config).unwrap();

    let manager = Arc::new(BotRunManager::with_capsule(store, capsule));
    let mgr_clone = Arc::clone(&manager);
    let sock = socket_path.to_path_buf();

    let handle = tokio::spawn(async move {
        codex_pm_service::ipc::serve_path(mgr_clone, &sock)
            .await
            .unwrap();
    });

    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    (manager, handle)
}

/// Phase-2: Verify capsule-authoritative submit produces mv2:// URIs.
///
/// 1. Start service with capsule enabled
/// 2. Submit a run via IPC (bot.run)
/// 3. Verify bot.show returns mv2:// URIs
/// 4. Verify local cache files also exist (dual-write)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capsule_submit_produces_mv2_uris() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-capsule.sock");
    let data_dir = temp_dir.path().join("data");
    let workspace_path = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    let (_manager, server_handle) =
        start_service_with_capsule(&socket_path, &data_dir, &workspace_path).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // Submit a run
    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-CAPSULE-001",
            "kind": "research",
            "capture_mode": "prompts_only"
        }
    }));
    assert!(
        run_resp.get("result").is_some(),
        "bot.run should succeed: {run_resp}"
    );
    let run_id = run_resp["result"]["run_id"].as_str().unwrap();
    assert_eq!(run_resp["result"]["status"].as_str().unwrap(), "succeeded");

    // Verify bot.show returns mv2:// URIs
    let show_resp = client.rpc(serde_json::json!({
        "id": 2,
        "method": "bot.show",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-CAPSULE-001",
            "run_id": run_id
        }
    }));
    assert!(
        show_resp.get("result").is_some(),
        "bot.show should succeed: {show_resp}"
    );
    let uris = show_resp["result"]["artifact_uris"].as_array().unwrap();
    assert!(!uris.is_empty(), "artifact_uris should not be empty");

    // All URIs should be mv2:// (capsule-authoritative)
    for uri in uris {
        let uri_str = uri.as_str().unwrap();
        assert!(
            uri_str.starts_with("mv2://"),
            "Expected mv2:// URI, got: {uri_str}"
        );
    }

    // Verify specific artifact URIs are present
    let uri_strs: Vec<&str> = uris.iter().filter_map(|u| u.as_str()).collect();
    assert!(
        uri_strs.iter().any(|u| u.contains("request.json")),
        "Should contain request URI in {uri_strs:?}"
    );
    assert!(
        uri_strs.iter().any(|u| u.contains("log.json")),
        "Should contain log URI in {uri_strs:?}"
    );
    assert!(
        uri_strs.iter().any(|u| u.contains("report.json")),
        "Should contain report URI in {uri_strs:?}"
    );

    // Verify local cache files also exist (dual-write)
    let local_run_dir = data_dir.join(run_id);
    assert!(
        local_run_dir.join("request.json").exists(),
        "Local request.json should exist (dual-write)"
    );
    assert!(
        local_run_dir.join("log.json").exists(),
        "Local log.json should exist (dual-write)"
    );

    server_handle.abort();
}

/// Phase-2: Verify capsule-authoritative resume produces mv2:// URIs.
///
/// 1. Write an incomplete request to disk
/// 2. Start service with capsule
/// 3. resume_incomplete()
/// 4. Verify bot.show returns mv2:// URIs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capsule_resume_produces_mv2_uris() {
    use codex_core::pm::bot::{BotCaptureMode, BotKind, BotRunRequest, BotWriteMode};

    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-capsule-resume.sock");
    let data_dir = temp_dir.path().join("data");
    let workspace_path = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    // Write an incomplete request (simulating crash before engine)
    let store = PersistenceStore::with_base_dir(data_dir.clone()).unwrap();
    let incomplete_request = BotRunRequest {
        schema_version: BotRunRequest::SCHEMA_VERSION.to_string(),
        run_id: "resume-capsule-001".to_string(),
        work_item_id: "SPEC-CAPSULE-RESUME".to_string(),
        kind: BotKind::Research,
        capture_mode: BotCaptureMode::PromptsOnly,
        write_mode: BotWriteMode::None,
        requested_at: "2026-02-10T00:00:00Z".to_string(),
        trigger: None,
    };
    store
        .write_request(&incomplete_request, &workspace_path.to_string_lossy())
        .unwrap();

    // Start service with capsule
    let (_manager, server_handle) =
        start_service_with_capsule(&socket_path, &data_dir, &workspace_path).await;

    // Resume incomplete runs
    _manager.resume_incomplete().await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // Verify resumed run has mv2:// URIs via bot.show
    let show_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.show",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-CAPSULE-RESUME",
            "run_id": "resume-capsule-001"
        }
    }));
    assert!(
        show_resp.get("result").is_some(),
        "bot.show after resume should succeed: {show_resp}"
    );
    assert_eq!(show_resp["result"]["status"].as_str().unwrap(), "succeeded");

    let uris = show_resp["result"]["artifact_uris"].as_array().unwrap();
    assert!(!uris.is_empty(), "Resumed run should have artifact_uris");

    // Resumed run should have mv2:// URIs
    for uri in uris {
        let uri_str = uri.as_str().unwrap();
        assert!(
            uri_str.starts_with("mv2://"),
            "Resumed run expected mv2:// URI, got: {uri_str}"
        );
    }

    server_handle.abort();
}

// ── Auto-resume: reboot simulation ──────────────────────────────────────

/// Start a service that resumes incomplete runs during startup, then
/// accept client connections. This mirrors the main service startup path
/// when started on login/restart (D141).
async fn start_service_with_resume(
    socket_path: &std::path::Path,
    data_dir: &std::path::Path,
) -> (Arc<BotRunManager>, tokio::task::JoinHandle<()>) {
    let store = Arc::new(PersistenceStore::with_base_dir(data_dir.to_path_buf()).unwrap());
    let manager = Arc::new(BotRunManager::new(Arc::clone(&store)));

    // Resume incomplete runs (mirrors main.rs startup path)
    manager.resume_incomplete().await;

    let mgr_clone = Arc::clone(&manager);
    let sock = socket_path.to_path_buf();

    let handle = tokio::spawn(async move {
        codex_pm_service::ipc::serve_path(mgr_clone, &sock)
            .await
            .unwrap();
    });

    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    (manager, handle)
}

/// Simulate restart/login: incomplete run on disk → service restart
/// (with resume_incomplete at startup) → client connect → verify run
/// was resumed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reboot_simulation_resumes_incomplete_runs() {
    use codex_core::pm::bot::{BotCaptureMode, BotKind, BotRunRequest, BotWriteMode};

    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-reboot.sock");
    let data_dir = temp_dir.path().join("data");

    // ── Phase 1: "Before reboot" — write an incomplete run ──────────
    let store = PersistenceStore::with_base_dir(data_dir.clone()).unwrap();
    let request = BotRunRequest {
        schema_version: BotRunRequest::SCHEMA_VERSION.to_string(),
        run_id: "reboot-sim-001".to_string(),
        work_item_id: "SPEC-REBOOT-SIM".to_string(),
        kind: BotKind::Research,
        capture_mode: BotCaptureMode::PromptsOnly,
        write_mode: BotWriteMode::None,
        requested_at: "2026-02-10T00:00:00Z".to_string(),
        trigger: None,
    };
    store
        .write_request(&request, &temp_dir.path().to_string_lossy())
        .unwrap();

    // Sanity: no terminal artifacts yet
    assert!(!data_dir.join("reboot-sim-001/log.json").exists());
    assert!(!data_dir.join("reboot-sim-001/report.json").exists());

    // ── Phase 2: "After reboot" — start service (triggers resume) ───
    let (_manager, server_handle) = start_service_with_resume(&socket_path, &data_dir).await;

    // ── Phase 3: Ping-style connect (what --ping does) ──────────────
    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // ── Phase 4: Verify the run was auto-resumed ────────────────────

    // 4a. Disk artifacts exist
    assert!(
        data_dir.join("reboot-sim-001/log.json").exists(),
        "log.json should exist after auto-resume"
    );
    assert!(
        data_dir.join("reboot-sim-001/report.json").exists(),
        "report.json should exist after auto-resume"
    );

    // 4b. bot.show returns succeeded with artifact URIs
    let show_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.show",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-REBOOT-SIM",
            "run_id": "reboot-sim-001"
        }
    }));
    assert!(
        show_resp.get("result").is_some(),
        "bot.show should succeed after reboot-resume: {show_resp}"
    );
    assert_eq!(
        show_resp["result"]["status"].as_str().unwrap(),
        "succeeded",
        "Auto-resumed run should be succeeded"
    );

    let uris = show_resp["result"]["artifact_uris"].as_array().unwrap();
    assert!(!uris.is_empty(), "artifact_uris should not be empty");
    assert!(
        uris.iter().any(|u| u.as_str().unwrap().contains("/log")),
        "Should contain log URI"
    );
    assert!(
        uris.iter().any(|u| u.as_str().unwrap().contains("/report")),
        "Should contain report URI"
    );

    // 4c. Verify log.json on disk shows succeeded state
    let log_data = std::fs::read_to_string(data_dir.join("reboot-sim-001/log.json")).unwrap();
    let log: serde_json::Value = serde_json::from_str(&log_data).unwrap();
    assert_eq!(log["state"].as_str().unwrap(), "succeeded");

    server_handle.abort();
}

/// Verify that after reboot-resume, the service returns to a quiescent state:
/// 0 connections + 0 active runs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reboot_resume_preserves_quiescent_state() {
    use codex_core::pm::bot::{BotCaptureMode, BotKind, BotRunRequest, BotWriteMode};

    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-idle.sock");
    let data_dir = temp_dir.path().join("data");

    // Write incomplete run
    let store = PersistenceStore::with_base_dir(data_dir.clone()).unwrap();
    let request = BotRunRequest {
        schema_version: BotRunRequest::SCHEMA_VERSION.to_string(),
        run_id: "idle-test-001".to_string(),
        work_item_id: "SPEC-IDLE-TEST".to_string(),
        kind: BotKind::Research,
        capture_mode: BotCaptureMode::PromptsOnly,
        write_mode: BotWriteMode::None,
        requested_at: "2026-02-10T00:00:00Z".to_string(),
        trigger: None,
    };
    store
        .write_request(&request, &temp_dir.path().to_string_lossy())
        .unwrap();

    // Start service with resume
    let (manager, server_handle) = start_service_with_resume(&socket_path, &data_dir).await;

    // After resume_incomplete, all runs should be terminal
    assert_eq!(
        manager.active_run_count().await,
        0,
        "No active runs after resume completes"
    );

    // Simulate --ping: connect and immediately disconnect
    {
        let mut client = TestClient::connect(&socket_path);
        client.handshake();
        // client drops here — connection closed
    }

    // Give the server a moment to process the disconnect
    tokio::time::sleep(Duration::from_millis(50)).await;

    // D135 conditions: 0 connections, 0 active runs
    assert_eq!(
        manager.connection_count(),
        0,
        "No connections after ping disconnect"
    );
    assert_eq!(manager.active_run_count().await, 0, "No active runs");

    server_handle.abort();
}

// ── Real engine integration tests ────────────────────────────────────────

/// Integration: NotebookLM unavailable + allow_degraded=false → blocked, exit 2.
///
/// Proves that when the service cannot reach NotebookLM and degraded
/// mode is explicitly disallowed, the run reaches terminal `blocked`
/// state and `--wait` exit code maps to 2.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocked_when_notebooklm_unavailable_e2e() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-blocked.sock");
    let data_dir = temp_dir.path().join("data");

    let (_manager, server_handle) = start_service(&socket_path, &data_dir).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // Submit with allow_degraded=false, subscribe=true (--wait mode),
    // and a health URL guaranteed to be unreachable.
    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": temp_dir.path().to_string_lossy(),
            "work_item_id": "SPEC-BLOCKED-E2E",
            "kind": "research",
            "capture_mode": "prompts_only",
            "allow_degraded": false,
            "subscribe": true,
            "notebooklm_health_url": "http://127.0.0.1:1/nonexistent"
        }
    }));

    assert!(
        run_resp.get("result").is_some(),
        "bot.run should return a result (even when blocked): {run_resp}"
    );
    let run_result = &run_resp["result"];

    // Run should be terminal blocked
    assert_eq!(
        run_result["status"].as_str().unwrap(),
        "blocked",
        "Run should be blocked when NotebookLM unavailable and degraded disallowed"
    );
    assert_eq!(
        run_result["exit_code"].as_i64().unwrap(),
        2,
        "Blocked exit code should be 2"
    );

    // Since it's synchronous and subscribe=true, we should get a bot.terminal
    // notification on the same connection. Read it.
    let mut notif_line = String::new();
    use std::io::BufRead;
    client
        .reader
        .read_line(&mut notif_line)
        .expect("read terminal notification");
    let notif: serde_json::Value =
        serde_json::from_str(notif_line.trim()).expect("parse notification");

    assert_eq!(
        notif["method"].as_str().unwrap(),
        "bot.terminal",
        "Should receive bot.terminal notification"
    );
    let notif_params = &notif["params"];
    assert_eq!(
        notif_params["status"].as_str().unwrap(),
        "blocked",
        "Terminal notification should show blocked"
    );
    assert_eq!(
        notif_params["exit_code"].as_i64().unwrap(),
        2,
        "Terminal notification exit_code should be 2"
    );
    assert!(
        notif_params["summary"]
            .as_str()
            .unwrap()
            .contains("Blocked"),
        "Summary should mention blocked"
    );

    // Verify on-disk artifacts
    let run_id = run_result["run_id"].as_str().unwrap();
    let run_dir = data_dir.join(run_id);
    assert!(run_dir.join("log.json").exists(), "log.json should exist");
    assert!(
        run_dir.join("report.json").exists(),
        "report.json should exist"
    );

    // Verify log state on disk
    let log_data = std::fs::read_to_string(run_dir.join("log.json")).unwrap();
    let log: serde_json::Value = serde_json::from_str(&log_data).unwrap();
    assert_eq!(log["state"].as_str().unwrap(), "blocked");
    assert_eq!(log["exit_code"].as_i64().unwrap(), 2);

    server_handle.abort();
}

/// Integration: degraded path produces report + log + mv2:// URIs.
///
/// When NotebookLM is unavailable but degraded mode is allowed (default),
/// the engine should produce a complete but degraded research report,
/// with findings from workspace analysis, and all artifacts written
/// to capsule (mv2:// URIs).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn degraded_path_produces_report_with_capsule_uris() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-degraded.sock");
    let data_dir = temp_dir.path().join("data");
    let workspace_path = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    // Create spec documents in workspace for the engine to find
    let spec_dir = workspace_path.join("docs").join("SPEC-DEGRADED-001");
    std::fs::create_dir_all(&spec_dir).unwrap();
    std::fs::write(
        spec_dir.join("spec.md"),
        "# Research Spec\n\n## Requirements\n\nIntegration test spec.\n",
    )
    .unwrap();

    let (_manager, server_handle) =
        start_service_with_capsule(&socket_path, &data_dir, &workspace_path).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // Submit with default allow_degraded (true) and unreachable health URL
    // to force degraded mode regardless of local NotebookLM status.
    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-DEGRADED-001",
            "kind": "research",
            "capture_mode": "prompts_only",
            "notebooklm_health_url": "http://127.0.0.1:1/nonexistent"
        }
    }));

    assert!(
        run_resp.get("result").is_some(),
        "bot.run should succeed in degraded mode: {run_resp}"
    );
    let run_result = &run_resp["result"];
    let run_id = run_result["run_id"].as_str().unwrap();

    assert_eq!(
        run_result["status"].as_str().unwrap(),
        "succeeded",
        "Degraded run should succeed"
    );
    assert_eq!(run_result["exit_code"].as_i64().unwrap(), 0);

    // bot.show to get full details
    let show_resp = client.rpc(serde_json::json!({
        "id": 2,
        "method": "bot.show",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-DEGRADED-001",
            "run_id": run_id
        }
    }));
    assert!(
        show_resp.get("result").is_some(),
        "bot.show should succeed: {show_resp}"
    );
    let show_result = &show_resp["result"];

    // Verify artifact URIs are mv2:// (capsule-authoritative)
    let uris = show_result["artifact_uris"].as_array().unwrap();
    assert!(!uris.is_empty(), "artifact_uris should not be empty");
    for uri in uris {
        let uri_str = uri.as_str().unwrap();
        assert!(
            uri_str.starts_with("mv2://"),
            "Expected mv2:// URI, got: {uri_str}"
        );
    }

    // Verify report is marked degraded
    let report_json_str = show_result["report_json"].as_str().unwrap();
    let report: serde_json::Value = serde_json::from_str(report_json_str).unwrap();
    assert!(
        report["degraded"].as_bool().unwrap_or(false),
        "Report should be marked degraded"
    );
    assert!(
        !report["findings"].as_array().unwrap().is_empty(),
        "Report should have findings from workspace analysis"
    );
    assert!(
        report["sources_used"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s.as_str().unwrap() == "workspace-local-only"),
        "sources_used should include workspace-local-only marker"
    );

    // Verify determinism boundary fields
    assert!(
        report.get("input_uris").is_some(),
        "Report should have input_uris"
    );

    // Verify local cache files exist (dual-write)
    let local_run_dir = data_dir.join(run_id);
    assert!(
        local_run_dir.join("request.json").exists(),
        "Local request.json should exist"
    );
    assert!(
        local_run_dir.join("log.json").exists(),
        "Local log.json should exist"
    );
    assert!(
        local_run_dir.join("report.json").exists(),
        "Local report.json should exist"
    );

    // Verify checkpoints were persisted
    let has_checkpoint = std::fs::read_dir(&local_run_dir)
        .unwrap()
        .filter_map(std::result::Result::ok)
        .any(|e| e.file_name().to_string_lossy().starts_with("checkpoint-"));
    assert!(
        has_checkpoint,
        "At least one checkpoint should be persisted"
    );

    server_handle.abort();
}

// ── Review engine integration tests ─────────────────────────────────────

/// Helper: create a git repo with a source file in the workspace.
fn init_git_workspace(workspace_path: &std::path::Path) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(workspace_path)
        .output()
        .expect("git init");
    std::process::Command::new("git")
        .args(["config", "user.email", "bot@test.local"])
        .current_dir(workspace_path)
        .output()
        .expect("git config email");
    std::process::Command::new("git")
        .args(["config", "user.name", "Test Bot"])
        .current_dir(workspace_path)
        .output()
        .expect("git config name");
}

/// Integration: write-mode review produces PatchBundle with mv2:// URIs.
///
/// 1. Create workspace git repo with a file that has trailing whitespace
/// 2. Start service with capsule
/// 3. Submit review with write_mode=worktree
/// 4. Verify status=succeeded, has_patches=true
/// 5. Verify mv2:// URIs include patch_bundle.json
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_write_mode_produces_patch_bundle_mv2_uris() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-review-wm.sock");
    let data_dir = temp_dir.path().join("data");
    let workspace_path = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    // Initialize git repo with a file that has trailing whitespace
    init_git_workspace(&workspace_path);
    std::fs::write(
        workspace_path.join("main.rs"),
        "fn main() {  \n    println!(\"hello\");  \n}\n",
    )
    .unwrap();
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(&workspace_path)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(&workspace_path)
        .output()
        .expect("git commit");

    let (_manager, server_handle) =
        start_service_with_capsule(&socket_path, &data_dir, &workspace_path).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-REVIEW-WM",
            "kind": "review",
            "capture_mode": "full_io",
            "write_mode": "worktree"
        }
    }));
    assert!(
        run_resp.get("result").is_some(),
        "Write-mode review should succeed: {run_resp}"
    );
    assert_eq!(run_resp["result"]["status"].as_str().unwrap(), "succeeded");
    let run_id = run_resp["result"]["run_id"].as_str().unwrap();

    // bot.show to verify artifacts
    let show_resp = client.rpc(serde_json::json!({
        "id": 2,
        "method": "bot.show",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-REVIEW-WM",
            "run_id": run_id
        }
    }));
    assert!(show_resp.get("result").is_some());
    let show_result = &show_resp["result"];

    // Verify report has_patches=true
    let report_json_str = show_result["report_json"].as_str().unwrap();
    let report: serde_json::Value = serde_json::from_str(report_json_str).unwrap();
    assert!(
        report["has_patches"].as_bool().unwrap_or(false),
        "Report should have has_patches=true"
    );

    // Verify mv2:// URIs include patch_bundle.json
    let uris = show_result["artifact_uris"].as_array().unwrap();
    assert!(!uris.is_empty(), "artifact_uris should not be empty");

    let uri_strs: Vec<&str> = uris.iter().filter_map(|u| u.as_str()).collect();
    assert!(
        uri_strs.iter().any(|u| u.starts_with("mv2://")),
        "Should have mv2:// URIs: {uri_strs:?}"
    );
    assert!(
        uri_strs.iter().any(|u| u.contains("patch_bundle.json")),
        "Should contain patch_bundle.json URI: {uri_strs:?}"
    );

    // Verify local cache also has patch_bundle.json (dual-write)
    let local_run_dir = data_dir.join(run_id);
    assert!(
        local_run_dir.join("patch_bundle.json").exists(),
        "Local patch_bundle.json should exist"
    );

    server_handle.abort();
}

/// Integration: forced rebase conflict yields needs_attention and --wait exits 10.
///
/// 1. Create workspace git repo
/// 2. Commit a file, create a branch, make conflicting changes on both sides
/// 3. Submit review with write_mode=worktree, rebase_target=main, subscribe=true
/// 4. Verify terminal state = needs_attention, exit_code = 10
/// 5. Verify conflict_summary.json and patch_bundle.json artifacts
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_rebase_conflict_yields_needs_attention() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-conflict.sock");
    let data_dir = temp_dir.path().join("data");
    let workspace_path = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    // Initialize git repo with a file that has trailing whitespace
    init_git_workspace(&workspace_path);
    std::fs::write(
        workspace_path.join("main.rs"),
        "fn main() {  \n    println!(\"hello\");  \n}\n",
    )
    .unwrap();
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(&workspace_path)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(&workspace_path)
        .output()
        .expect("git commit");

    // Now advance main with a conflicting edit to the same lines
    // (the review engine will fix trailing whitespace on those same lines)
    std::fs::write(
        workspace_path.join("main.rs"),
        "fn main() {\n    println!(\"CONFLICTING CHANGE ON MAIN\");\n}\n",
    )
    .unwrap();
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(&workspace_path)
        .output()
        .expect("git add conflict");
    std::process::Command::new("git")
        .args(["commit", "-m", "advance main with conflict"])
        .current_dir(&workspace_path)
        .output()
        .expect("git commit conflict");

    // Get the current main HEAD (the rebase target)
    let main_head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&workspace_path)
        .output()
        .expect("git rev-parse");
    let main_sha = String::from_utf8_lossy(&main_head.stdout)
        .trim()
        .to_string();

    // Reset back to the first commit so the bot works on the old state
    std::process::Command::new("git")
        .args(["reset", "--hard", "HEAD~1"])
        .current_dir(&workspace_path)
        .output()
        .expect("git reset");

    let (_manager, server_handle) = start_service(&socket_path, &data_dir).await;

    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // Submit review with rebase_target pointing to main's advanced commit
    let run_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.run",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-CONFLICT",
            "kind": "review",
            "capture_mode": "full_io",
            "write_mode": "worktree",
            "rebase_target": main_sha,
            "subscribe": true
        }
    }));

    assert!(
        run_resp.get("result").is_some(),
        "bot.run should return a result: {run_resp}"
    );
    let run_result = &run_resp["result"];

    // Run should be needs_attention due to rebase conflict
    assert_eq!(
        run_result["status"].as_str().unwrap(),
        "needs_attention",
        "Run should be needs_attention: {run_result}"
    );
    assert_eq!(
        run_result["exit_code"].as_i64().unwrap(),
        10,
        "Exit code should be 10 for needs_attention"
    );

    // Read the terminal notification (--wait mode)
    let mut notif_line = String::new();
    use std::io::BufRead;
    client
        .reader
        .read_line(&mut notif_line)
        .expect("read terminal notification");
    let notif: serde_json::Value =
        serde_json::from_str(notif_line.trim()).expect("parse notification");

    assert_eq!(notif["method"].as_str().unwrap(), "bot.terminal");
    let notif_params = &notif["params"];
    assert_eq!(notif_params["status"].as_str().unwrap(), "needs_attention");
    assert_eq!(notif_params["exit_code"].as_i64().unwrap(), 10);

    // Verify disk artifacts
    let run_id = run_result["run_id"].as_str().unwrap();
    let local_run_dir = data_dir.join(run_id);
    assert!(
        local_run_dir.join("patch_bundle.json").exists(),
        "patch_bundle.json should exist"
    );
    assert!(
        local_run_dir.join("conflict_summary.json").exists(),
        "conflict_summary.json should exist"
    );

    // Verify conflict_summary contents
    let cs_data = std::fs::read_to_string(local_run_dir.join("conflict_summary.json")).unwrap();
    let cs: serde_json::Value = serde_json::from_str(&cs_data).unwrap();
    assert!(
        !cs["resolution_instructions"].as_array().unwrap().is_empty(),
        "Should have resolution instructions"
    );

    server_handle.abort();
}

// ── Auto-resume: full reboot→capsule→mv2:// acceptance test ─────────

/// **Acceptance test for auto-resume after restart/login.**
///
/// Simulates the systemd behavior:
///   1. Service instance #1 accepts a run and crashes mid-flight
///      (incomplete request on disk, no terminal log)
///   2. Service instance #1 is gone (simulating reboot)
///   3. Service instance #2 starts and calls `resume_incomplete()` at startup
///   4. A client connects and verifies status/artifacts
///   5. Verify: run is now terminal (succeeded), mv2:// capsule
///      artifacts exist, local cache has all files
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reboot_resume_full_flow_with_capsule_mv2_artifacts() {
    use codex_core::pm::bot::{BotCaptureMode, BotKind, BotRunRequest, BotWriteMode};
    use codex_pm_service::persistence::CapsulePersistence;
    use codex_tui::memvid_adapter::CapsuleConfig;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test-pm-reboot-mv2.sock");
    let data_dir = temp_dir.path().join("data");
    let workspace_path = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_path).unwrap();

    // ── Phase 1: "Before crash" — write an incomplete run ────────────
    let store = PersistenceStore::with_base_dir(data_dir.clone()).unwrap();
    let request = BotRunRequest {
        schema_version: BotRunRequest::SCHEMA_VERSION.to_string(),
        run_id: "reboot-mv2-001".to_string(),
        work_item_id: "SPEC-REBOOT-MV2".to_string(),
        kind: BotKind::Research,
        capture_mode: BotCaptureMode::PromptsOnly,
        write_mode: BotWriteMode::None,
        requested_at: "2026-02-10T12:00:00Z".to_string(),
        trigger: None,
    };
    store
        .write_request(&request, &workspace_path.to_string_lossy())
        .unwrap();

    // Sanity: no terminal artifacts yet
    assert!(!data_dir.join("reboot-mv2-001/log.json").exists());

    // ── Phase 2: "After reboot" — start service with capsule ─────────
    //
    // This mirrors the main service startup path:
    //   codex-pm-service starts → resume_incomplete() before IPC
    let capsule_path = workspace_path.join(".speckit/memvid/workspace.mv2");
    if let Some(parent) = capsule_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "default".to_string(),
        ..Default::default()
    };
    let capsule = CapsulePersistence::open_with_config(config).unwrap();

    let store = Arc::new(PersistenceStore::with_base_dir(data_dir.clone()).unwrap());
    let manager = Arc::new(BotRunManager::with_capsule(store, capsule));

    // Resume incomplete runs (mirrors main.rs startup path)
    manager.resume_incomplete().await;

    let mgr_clone = Arc::clone(&manager);
    let sock = socket_path.clone();
    let server_handle = tokio::spawn(async move {
        codex_pm_service::ipc::serve_path(mgr_clone, &sock)
            .await
            .unwrap();
    });
    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // ── Phase 3: client connect + handshake ─────────────────────────
    let mut client = TestClient::connect(&socket_path);
    client.handshake();

    // ── Phase 4: Verify run reached terminal with mv2:// artifacts ───

    // 4a. Local disk artifacts exist
    assert!(
        data_dir.join("reboot-mv2-001/log.json").exists(),
        "log.json should exist after resume"
    );
    assert!(
        data_dir.join("reboot-mv2-001/report.json").exists(),
        "report.json should exist after resume"
    );

    // 4b. Log on disk confirms terminal succeeded state
    let log_data = std::fs::read_to_string(data_dir.join("reboot-mv2-001/log.json")).unwrap();
    let log: serde_json::Value = serde_json::from_str(&log_data).unwrap();
    assert_eq!(
        log["state"].as_str().unwrap(),
        "succeeded",
        "Resumed run should be succeeded"
    );

    // 4c. bot.show returns mv2:// capsule-authoritative URIs
    let show_resp = client.rpc(serde_json::json!({
        "id": 1,
        "method": "bot.show",
        "params": {
            "workspace_path": workspace_path.to_string_lossy(),
            "work_item_id": "SPEC-REBOOT-MV2",
            "run_id": "reboot-mv2-001"
        }
    }));
    assert!(
        show_resp.get("result").is_some(),
        "bot.show should succeed: {show_resp}"
    );
    assert_eq!(show_resp["result"]["status"].as_str().unwrap(), "succeeded");

    let uris = show_resp["result"]["artifact_uris"].as_array().unwrap();
    assert!(!uris.is_empty(), "artifact_uris should not be empty");

    // All URIs must be mv2:// (capsule SoR, D114)
    for uri in uris {
        let uri_str = uri.as_str().unwrap();
        assert!(
            uri_str.starts_with("mv2://"),
            "Expected mv2:// URI, got: {uri_str}"
        );
    }

    // Must include the three core artifacts
    let uri_strs: Vec<&str> = uris.iter().filter_map(|u| u.as_str()).collect();
    assert!(
        uri_strs.iter().any(|u| u.contains("request.json")),
        "Should contain request URI in {uri_strs:?}"
    );
    assert!(
        uri_strs.iter().any(|u| u.contains("log.json")),
        "Should contain log URI in {uri_strs:?}"
    );
    assert!(
        uri_strs.iter().any(|u| u.contains("report.json")),
        "Should contain report URI in {uri_strs:?}"
    );

    // 4d. Checkpoint URIs also present in capsule
    assert!(
        uri_strs.iter().any(|u| u.contains("checkpoint/")),
        "Should contain checkpoint URIs in {uri_strs:?}"
    );

    // ── Phase 5: quiescent state ─────────────────────────────────────
    // After resume + client disconnect, the service should be quiescent
    drop(client); // disconnect
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(
        manager.active_run_count().await,
        0,
        "No active runs after resume"
    );
    assert_eq!(
        manager.connection_count(),
        0,
        "No connections after ping disconnect"
    );

    server_handle.abort();
}
