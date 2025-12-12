// SPEC-957: Allow expect/unwrap in test code
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

use codex_core::config_types::McpServerConfig;
use codex_core::mcp_connection_manager::McpConnectionManager;

static BUILD_TEST_SERVER: Once = Once::new();

fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let mut root = PathBuf::from(manifest_dir);
    // codex-rs/core -> codex-rs
    root.pop();
    root
}

fn server_bin_path() -> PathBuf {
    // prefer debug profile location for tests
    let mut p = workspace_root();
    p.push("target");
    p.push("debug");
    #[cfg(windows)]
    p.push("codex-mcp-test-server.exe");
    #[cfg(not(windows))]
    p.push("codex-mcp-test-server");
    p
}

fn ensure_test_server() -> PathBuf {
    let root = workspace_root();
    let server = server_bin_path();

    BUILD_TEST_SERVER.call_once(|| {
        if server.exists() {
            return;
        }

        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let status = Command::new(cargo)
            .current_dir(root)
            .args(["build", "-p", "codex-mcp-test-server"])
            .status()
            .expect("build codex-mcp-test-server");
        assert!(status.success(), "build codex-mcp-test-server failed");
    });

    assert!(
        server.exists(),
        "expected test server at {}",
        server.display()
    );
    server
}

#[tokio::test]
async fn mcp_manager_skips_slow_server_on_timeout() {
    let server = ensure_test_server();

    // Slow server exceeds timeout (init/list 200ms vs 100ms timeout)
    let slow_cfg = McpServerConfig {
        command: "bash".to_string(),
        args: vec![
            "-lc".to_string(),
            format!("SLOW_INIT_MS=200 SLOW_LIST_MS=200 {}", server.display()),
        ],
        env: None,
        startup_timeout_ms: Some(100),
    };
    // Fast server responds immediately
    let fast_cfg = McpServerConfig {
        command: server.to_string_lossy().to_string(),
        args: vec![],
        env: None,
        startup_timeout_ms: Some(500),
    };

    let mut servers = HashMap::new();
    servers.insert("slow".to_string(), slow_cfg);
    servers.insert("fast".to_string(), fast_cfg);

    let (mgr, errs) = McpConnectionManager::new(servers, std::collections::HashSet::new())
        .await
        .expect("manager creation should not fail entirely");

    // Slow should be reported as error; fast should be available.
    assert!(errs.contains_key("slow"));
    assert!(!errs.contains_key("fast"));

    let tools = mgr.list_all_tools();
    // Expect tool echo from fast server only: qualified name fast__echo
    assert!(tools.keys().any(|k| k.starts_with("fast__")));
    assert!(!tools.keys().any(|k| k.starts_with("slow__")));
}

#[tokio::test]
async fn mcp_manager_respects_extended_startup_timeout() {
    let server = ensure_test_server();

    // Slow server within extended timeout (init/list 200ms vs 500ms)
    let slow_ok = McpServerConfig {
        command: "bash".to_string(),
        args: vec![
            "-lc".to_string(),
            format!("SLOW_INIT_MS=200 SLOW_LIST_MS=200 {}", server.display()),
        ],
        env: None,
        startup_timeout_ms: Some(500),
    };
    let mut servers = HashMap::new();
    servers.insert("slow_ok".to_string(), slow_ok);

    let (mgr, errs) = McpConnectionManager::new(servers, std::collections::HashSet::new())
        .await
        .expect("manager creation should not fail");

    assert!(errs.is_empty(), "no errors expected, got: {errs:?}");
    let tools = mgr.list_all_tools();
    assert!(tools.keys().any(|k| k.starts_with("slow_ok__")));
}
