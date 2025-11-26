#![cfg(unix)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use async_channel::Receiver;
use codex_core::error::CodexErr;
use codex_core::error::SandboxErr;
use codex_core::exec::ExecParams;
use codex_core::exec::SandboxType;
use codex_core::exec::StdoutStream;
use codex_core::exec::process_exec_tool_call;
use codex_core::protocol::Event;
use codex_core::protocol::EventMsg;
use codex_core::protocol::ExecCommandOutputDeltaEvent;
use codex_core::protocol::ExecOutputStream;
use codex_core::protocol::SandboxPolicy;

fn collect_stdout_events(rx: Receiver<Event>) -> Vec<u8> {
    let mut out = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let EventMsg::ExecCommandOutputDelta(ExecCommandOutputDeltaEvent {
            stream: ExecOutputStream::Stdout,
            chunk,
            ..
        }) = ev.msg
        {
            out.extend_from_slice(&chunk);
        }
    }
    out
}

/// SPEC-957: StdoutStream has private fields - test stubbed
#[tokio::test]
#[ignore = "SPEC-957: StdoutStream has private fields that prevent struct literal construction"]
async fn test_exec_stdout_stream_events_echo() {
    unimplemented!("SPEC-957: StdoutStream has private fields");
}

/// SPEC-957: StdoutStream has private fields - test stubbed
#[tokio::test]
#[ignore = "SPEC-957: StdoutStream has private fields that prevent struct literal construction"]
async fn test_exec_stderr_stream_events_echo() {
    unimplemented!("SPEC-957: StdoutStream has private fields");
}

#[tokio::test]
async fn test_aggregated_output_interleaves_in_order() {
    // Spawn a shell that alternates stdout and stderr with sleeps to enforce order.
    let cmd = vec![
        "/bin/sh".to_string(),
        "-c".to_string(),
        "printf 'O1\\n'; sleep 0.01; printf 'E1\\n' 1>&2; sleep 0.01; printf 'O2\\n'; sleep 0.01; printf 'E2\\n' 1>&2".to_string(),
    ];

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let params = ExecParams {
        command: cmd,
        cwd: cwd.clone(),
        timeout_ms: Some(5_000),
        env: HashMap::new(),
        with_escalated_permissions: None,
        justification: None,
    };

    let policy = SandboxPolicy::new_read_only_policy();

    let result = process_exec_tool_call(
        params,
        SandboxType::None,
        &policy,
        cwd.as_path(),
        &None,
        None,
    )
    .await
    .expect("process_exec_tool_call");

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.text, "O1\nO2\n");
    assert_eq!(result.stderr.text, "E1\nE2\n");
    assert_eq!(result.aggregated_output.text, "O1\nE1\nO2\nE2\n");
    assert_eq!(result.aggregated_output.truncated_after_lines, None);
}

#[tokio::test]
async fn test_exec_timeout_returns_partial_output() {
    let cmd = vec![
        "/bin/sh".to_string(),
        "-c".to_string(),
        "printf 'before\\n'; sleep 2; printf 'after\\n'".to_string(),
    ];

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let params = ExecParams {
        command: cmd,
        cwd: cwd.clone(),
        timeout_ms: Some(200),
        env: HashMap::new(),
        with_escalated_permissions: None,
        justification: None,
    };

    let policy = SandboxPolicy::new_read_only_policy();

    let result = process_exec_tool_call(
        params,
        SandboxType::None,
        &policy,
        cwd.as_path(),
        &None,
        None,
    )
    .await;

    let Err(CodexErr::Sandbox(SandboxErr::Timeout { output })) = result else {
        panic!("expected timeout error");
    };

    assert_eq!(output.exit_code, 124);
    assert_eq!(output.stdout.text, "before\n");
    assert!(output.stderr.text.is_empty());
    assert_eq!(output.aggregated_output.text, "before\n");
    assert!(output.duration >= Duration::from_millis(200));
    assert!(output.timed_out);
}
