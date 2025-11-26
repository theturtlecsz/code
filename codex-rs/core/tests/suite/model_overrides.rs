// SPEC-957: These tests relied on Op::OverrideTurnContext which was removed.
// The tests are marked #[ignore] and stubbed to allow compilation.
#![allow(unused_imports)]

use codex_core::CodexAuth;
use codex_core::ConversationManager;
use codex_core::protocol::EventMsg;
use codex_core::protocol::Op;
use codex_core::protocol_config_types::ReasoningEffort;
use core_test_support::load_default_config_for_test;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

const CONFIG_TOML: &str = "config.toml";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::OverrideTurnContext was removed"]
async fn override_turn_context_does_not_persist_when_config_exists() {
    unimplemented!("SPEC-957: Op::OverrideTurnContext was removed from the protocol");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::OverrideTurnContext was removed"]
async fn override_turn_context_does_not_create_config_file() {
    unimplemented!("SPEC-957: Op::OverrideTurnContext was removed from the protocol");
}
