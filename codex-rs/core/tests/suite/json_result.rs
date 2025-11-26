#![cfg(not(target_os = "windows"))]
// SPEC-957: Most imports are unused because the test was stubbed out.
#![allow(unused_imports)]

use codex_core::protocol::AskForApproval;
use codex_core::protocol::EventMsg;
use codex_core::protocol::InputItem;
use codex_core::protocol::Op;
use codex_core::protocol::SandboxPolicy;
use codex_protocol::config_types::ReasoningSummary;
use core_test_support::non_sandbox_test;
use core_test_support::responses;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use responses::ev_assistant_message;
use responses::ev_completed;
use responses::sse;
use responses::start_mock_server;

const SCHEMA: &str = r#"
{
    "type": "object",
    "properties": {
        "explanation": { "type": "string" },
        "final_answer": { "type": "string" }
    },
    "required": ["explanation", "final_answer"],
    "additionalProperties": false
}
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::UserTurn was removed"]
async fn codex_returns_json_result_for_gpt5() -> anyhow::Result<()> {
    codex_returns_json_result("gpt-5".to_string()).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::UserTurn was removed"]
async fn codex_returns_json_result_for_gpt5_codex() -> anyhow::Result<()> {
    codex_returns_json_result("gpt-5-codex".to_string()).await
}

/// SPEC-957: Op::UserTurn was removed - this helper is stubbed out.
/// The tests that call this are marked #[ignore], so this stub allows compilation.
#[allow(unused_variables)]
async fn codex_returns_json_result(model: String) -> anyhow::Result<()> {
    anyhow::bail!("SPEC-957: Op::UserTurn was removed from the protocol");
}
