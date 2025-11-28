//! SPEC-958 Session 9: Tests for fork_conversation using rollout_path from SessionConfiguredEvent
//!
//! The rollout_path API extension (added in Session 9) allows tests to access the rollout path
//! via SessionConfiguredEvent.rollout_path instead of the removed Op::GetPath operation.

#![allow(clippy::expect_used)]

use codex_core::CodexAuth;
use codex_core::ConversationManager;
use codex_core::NewConversation;
use core_test_support::load_default_config_for_test;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test that SessionConfiguredEvent contains the rollout_path field.
/// This validates the API extension for SPEC-958.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_configured_event_contains_rollout_path() {
    let home = TempDir::new().expect("create temp dir");
    let config = load_default_config_for_test(&home);

    let manager = ConversationManager::with_auth(CodexAuth::from_api_key("dummy"));
    let NewConversation {
        conversation: _,
        conversation_id: _,
        session_configured,
    } = manager
        .new_conversation(config)
        .await
        .expect("create conversation");

    // The SessionConfiguredEvent should have a rollout_path
    assert!(
        session_configured.rollout_path.is_some(),
        "SessionConfiguredEvent should include rollout_path"
    );

    // Verify the path is under the expected sessions directory
    let rollout_path = session_configured.rollout_path.unwrap();
    assert!(
        rollout_path.to_string_lossy().contains("sessions"),
        "rollout_path should be in the sessions directory: {rollout_path:?}"
    );
}

/// Test that fork_conversation can use the rollout_path from SessionConfiguredEvent.
/// This validates that the rollout_path is a valid path that can be passed to fork_conversation.
/// Note: fork_conversation on an empty session will fail with "empty session file" -
/// this is expected behavior as there's nothing to fork from.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fork_conversation_with_rollout_path_from_event() {
    let home = TempDir::new().expect("create temp dir");
    let config = load_default_config_for_test(&home);

    let manager = ConversationManager::with_auth(CodexAuth::from_api_key("dummy"));

    // Create initial conversation and capture its rollout_path
    let NewConversation {
        conversation: _,
        conversation_id: _,
        session_configured,
    } = manager
        .new_conversation(config.clone())
        .await
        .expect("create conversation");

    let rollout_path = session_configured
        .rollout_path
        .expect("rollout_path should be present");

    assert!(
        rollout_path.exists(),
        "rollout_path should exist on disk: {rollout_path:?}"
    );

    // Fork the conversation - drop 0 messages (keep all)
    // Note: This will fail with "empty session file" because no messages have been
    // sent yet. This is correct behavior - the key validation is that rollout_path
    // is a valid path that fork_conversation accepts.
    let fork_result = manager
        .fork_conversation(0, config.clone(), rollout_path.clone())
        .await;

    // The result should be an error about empty session (since no messages were sent)
    // OR success if the implementation handles empty sessions gracefully
    match fork_result {
        Ok(forked) => {
            // If it succeeds, the forked conversation should have a rollout_path
            assert!(
                forked.session_configured.rollout_path.is_some(),
                "forked conversation should have a rollout_path"
            );
        }
        Err(e) => {
            // Empty session file error is expected - validates path is correct format
            let error_msg = format!("{e:?}");
            assert!(
                error_msg.contains("empty session") || error_msg.contains("empty rollout"),
                "error should be about empty session, got: {error_msg}"
            );
        }
    }
}

/// Test that rollout_path is a valid PathBuf that can be used for file operations.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rollout_path_is_valid_for_file_operations() {
    let home = TempDir::new().expect("create temp dir");
    let config = load_default_config_for_test(&home);

    let manager = ConversationManager::with_auth(CodexAuth::from_api_key("dummy"));
    let NewConversation {
        conversation: _,
        conversation_id: _,
        session_configured,
    } = manager
        .new_conversation(config)
        .await
        .expect("create conversation");

    let rollout_path: PathBuf = session_configured
        .rollout_path
        .expect("rollout_path should be present");

    // Should be able to get file metadata
    let metadata = std::fs::metadata(&rollout_path);
    assert!(
        metadata.is_ok(),
        "should be able to get metadata for rollout_path: {rollout_path:?}"
    );

    // Should be a file (not a directory)
    assert!(
        metadata.unwrap().is_file(),
        "rollout_path should point to a file"
    );

    // File should have .jsonl extension
    assert!(
        rollout_path.extension().is_some_and(|ext| ext == "jsonl"),
        "rollout_path should have .jsonl extension: {rollout_path:?}"
    );
}
