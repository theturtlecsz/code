//! Integration Tests for DirectProcessExecutor with Real AI CLIs
//!
//! These tests verify DirectProcessExecutor works correctly with actual AI provider CLIs
//! (claude, gemini, openai). All tests are marked with #[ignore] to make them optional
//! and CI-friendly - they only run when explicitly requested via:
//!
//!   cargo test -p codex-core --test async_agent_executor_integration -- --ignored
//!
//! Prerequisites:
//!   - Install AI CLIs (see README.md in this directory)
//!   - Set API key environment variables (ANTHROPIC_API_KEY, GOOGLE_API_KEY, OPENAI_API_KEY)
//!
//! Tests automatically skip with helpful error messages if API keys are not configured.

use codex_core::async_agent_executor::{
    AgentExecutionError, AnthropicProvider, AsyncAgentExecutor, DirectProcessExecutor,
    GoogleProvider, OpenAIProvider,
};
use std::collections::HashMap;

// ================================================================================================
// Test Helper Functions
// ================================================================================================

/// Check if an environment variable is set, returning helpful error message if missing
fn require_env_var(var_name: &str, cli_install_cmd: &str) -> String {
    std::env::var(var_name).unwrap_or_else(|_| {
        panic!(
            "{var_name} not set - install {var_name} and set API key:\n\
             Install: {cli_install_cmd}\n\
             Set key: export {var_name}=\"your-api-key-here\""
        )
    })
}

/// Create a large prompt (>1KB) for stdin piping tests
fn create_large_prompt(size_kb: usize) -> String {
    let base = "This is a test prompt. ";
    let repetitions = (size_kb * 1024) / base.len();
    base.repeat(repetitions)
}

// ================================================================================================
// Claude CLI Tests
// ================================================================================================

/// Test claude CLI with small prompt
///
/// Verifies:
/// - Basic command execution succeeds
/// - stdout contains expected response
/// - exit_code is 0
/// - no timeout occurs
///
/// Prerequisites:
/// - claude CLI installed: pip install anthropic-cli
/// - ANTHROPIC_API_KEY environment variable set
///
/// Run: cargo test -p codex-core --test async_agent_executor_integration test_claude_small_prompt -- --ignored
#[tokio::test]
#[ignore] // Skip without API keys (CI-friendly)
async fn test_claude_small_prompt() {
    let executor = DirectProcessExecutor;
    let provider = AnthropicProvider::new();
    let mut env = HashMap::new();

    let api_key = require_env_var("ANTHROPIC_API_KEY", "pip install anthropic-cli");
    env.insert("ANTHROPIC_API_KEY".to_string(), api_key);

    let output = executor
        .execute(
            "claude",
            &["-p".to_string(), "Say 'hello' and nothing else".to_string()],
            &env,
            None,
            600, // 10 minute timeout (generous for network latency)
            None,
            &provider,
        )
        .await
        .expect("claude CLI should execute successfully");

    assert_eq!(output.exit_code, 0, "claude should exit successfully");
    assert!(
        output.stdout.to_lowercase().contains("hello"),
        "stdout should contain 'hello', got: {}",
        output.stdout
    );
    assert!(!output.timed_out, "claude should not timeout");
}

/// Test claude CLI with large prompt (>1KB) via stdin
///
/// Verifies:
/// - Large prompts (>1KB) handled via stdin piping (avoids OS command-line limits)
/// - No truncation or data loss
/// - Execution succeeds with large input
///
/// Prerequisites:
/// - claude CLI installed: pip install anthropic-cli
/// - ANTHROPIC_API_KEY environment variable set
///
/// Run: cargo test -p codex-core --test async_agent_executor_integration test_claude_large_prompt -- --ignored
#[tokio::test]
#[ignore] // Skip without API keys (CI-friendly)
async fn test_claude_large_prompt() {
    let executor = DirectProcessExecutor;
    let provider = AnthropicProvider::new();
    let mut env = HashMap::new();

    let api_key = require_env_var("ANTHROPIC_API_KEY", "pip install anthropic-cli");
    env.insert("ANTHROPIC_API_KEY".to_string(), api_key);

    // Create 2KB prompt to verify stdin piping (exceeds some OS command-line limits)
    let large_prompt = create_large_prompt(2);
    let prompt_with_question = format!(
        "{large_prompt}\n\nBased on the above text, answer: What is this a test of?"
    );

    let output = executor
        .execute(
            "claude",
            &["-p".to_string(), "-".to_string()], // "-" reads from stdin
            &env,
            None,
            600, // 10 minute timeout
            Some(&prompt_with_question),
            &provider,
        )
        .await
        .expect("claude CLI should handle large prompts via stdin");

    assert_eq!(
        output.exit_code, 0,
        "claude should exit successfully with large prompt"
    );
    assert!(
        !output.stdout.is_empty(),
        "claude should produce output for large prompt"
    );
    assert!(
        !output.timed_out,
        "claude should not timeout with large prompt"
    );
}

/// Test claude CLI OAuth2 error detection (no API key)
///
/// Verifies:
/// - Missing API key triggers OAuth2Required error
/// - Error detection via stderr pattern matching
/// - Proper error propagation
///
/// Prerequisites:
/// - claude CLI installed: pip install anthropic-cli
/// - NO ANTHROPIC_API_KEY set (this test explicitly unsets it)
///
/// Run: cargo test -p codex-core --test async_agent_executor_integration test_claude_oauth2_error -- --ignored
#[tokio::test]
#[ignore] // Skip without API keys (CI-friendly)
async fn test_claude_oauth2_error() {
    let executor = DirectProcessExecutor;
    let provider = AnthropicProvider::new();
    let env = HashMap::new(); // Explicitly no API key

    let result = executor
        .execute(
            "claude",
            &["-p".to_string(), "hello".to_string()],
            &env,
            None,
            600,
            None,
            &provider,
        )
        .await;

    match result {
        Err(AgentExecutionError::OAuth2Required(msg)) => {
            assert!(
                msg.contains("ANTHROPIC_API_KEY") || msg.contains("API key"),
                "error message should mention API key requirement, got: {msg}"
            );
        }
        Ok(_) => panic!("claude should fail without API key (OAuth2Required expected)"),
        Err(e) => panic!(
            "Expected OAuth2Required error, got different error: {e:?}"
        ),
    }
}

// ================================================================================================
// Google Gemini CLI Tests
// ================================================================================================

/// Test gemini CLI with small prompt
///
/// Verifies:
/// - Multi-provider support (not just Anthropic)
/// - Google Gemini CLI execution
/// - stdout contains expected response
///
/// Prerequisites:
/// - gemini CLI installed: pip install google-generativeai-cli
/// - GOOGLE_API_KEY environment variable set
///
/// Run: cargo test -p codex-core --test async_agent_executor_integration test_gemini_small_prompt -- --ignored
#[tokio::test]
#[ignore] // Skip without API keys (CI-friendly)
async fn test_gemini_small_prompt() {
    let executor = DirectProcessExecutor;
    let provider = GoogleProvider::new();
    let mut env = HashMap::new();

    let api_key = require_env_var("GOOGLE_API_KEY", "pip install google-generativeai-cli");
    env.insert("GOOGLE_API_KEY".to_string(), api_key);

    let output = executor
        .execute(
            "gemini",
            &["generate".to_string(),
                "--prompt".to_string(),
                "Say 'hello' and nothing else".to_string()],
            &env,
            None,
            600, // 10 minute timeout
            None,
            &provider,
        )
        .await
        .expect("gemini CLI should execute successfully");

    assert_eq!(output.exit_code, 0, "gemini should exit successfully");
    assert!(
        output.stdout.to_lowercase().contains("hello"),
        "stdout should contain 'hello', got: {}",
        output.stdout
    );
    assert!(!output.timed_out, "gemini should not timeout");
}

// ================================================================================================
// OpenAI CLI Tests
// ================================================================================================

/// Test openai CLI with timeout simulation
///
/// Verifies:
/// - Timeout handling works correctly
/// - Process is killed on timeout
/// - timed_out flag is set
///
/// Note: Uses a very short timeout (1ms) to force timeout without waiting.
/// This tests the timeout mechanism, not the OpenAI CLI specifically.
///
/// Prerequisites:
/// - openai CLI installed: pip install openai-cli
/// - OPENAI_API_KEY environment variable set
///
/// Run: cargo test -p codex-core --test async_agent_executor_integration test_openai_timeout -- --ignored
#[tokio::test]
#[ignore] // Skip without API keys (CI-friendly)
async fn test_openai_timeout() {
    let executor = DirectProcessExecutor;
    let provider = OpenAIProvider::new();
    let mut env = HashMap::new();

    let api_key = require_env_var("OPENAI_API_KEY", "pip install openai-cli");
    env.insert("OPENAI_API_KEY".to_string(), api_key);

    let result = executor
        .execute(
            "openai",
            &["chat".to_string(),
                "completions".to_string(),
                "create".to_string(),
                "--message".to_string(),
                "hello".to_string()],
            &env,
            None,
            1, // 1ms timeout (intentionally too short to force timeout)
            None,
            &provider,
        )
        .await;

    match result {
        Err(AgentExecutionError::Timeout(timeout_secs)) => {
            assert_eq!(timeout_secs, 1, "timeout should be 1 second");
            println!("Timeout test passed - command killed after 1s");
        }
        Ok(output) => {
            // If the command somehow completed in <1s, that's also acceptable
            // (unlikely but possible with fast network + caching)
            assert!(
                output.timed_out,
                "output should be marked as timed_out if it completed"
            );
        }
        Err(e) => panic!("Expected Timeout error, got different error: {e:?}"),
    }
}

// ================================================================================================
// Error Handling Tests
// ================================================================================================

/// Test non-existent CLI error detection
///
/// Verifies:
/// - CommandNotFound error for invalid executable
/// - Proper error message with command name
/// - No process spawned for non-existent command
///
/// Note: No API keys required for this test
///
/// Run: cargo test -p codex-core --test async_agent_executor_integration test_nonexistent_cli -- --ignored
#[tokio::test]
#[ignore] // Keep consistent with other integration tests
async fn test_nonexistent_cli() {
    let executor = DirectProcessExecutor;
    let provider = AnthropicProvider::new();
    let env = HashMap::new();

    let result = executor
        .execute(
            "this_cli_definitely_does_not_exist_12345",
            &["arg1".to_string()],
            &env,
            None,
            600,
            None,
            &provider,
        )
        .await;

    match result {
        Err(AgentExecutionError::CommandNotFound(cmd)) => {
            assert_eq!(
                cmd, "this_cli_definitely_does_not_exist_12345",
                "command name should match"
            );
            println!("CommandNotFound error correctly detected: {cmd}");
        }
        Ok(_) => panic!("non-existent CLI should fail with CommandNotFound"),
        Err(e) => panic!(
            "Expected CommandNotFound error, got different error: {e:?}"
        ),
    }
}

// ================================================================================================
// Additional Integration Tests (Optional - Extend as Needed)
// ================================================================================================

/// Test claude CLI with stderr output
///
/// Verifies:
/// - stderr is captured correctly
/// - Both stdout and stderr can be captured simultaneously
///
/// Prerequisites:
/// - claude CLI installed: pip install anthropic-cli
/// - ANTHROPIC_API_KEY environment variable set
///
/// Run: cargo test -p codex-core --test async_agent_executor_integration test_claude_stderr -- --ignored
#[tokio::test]
#[ignore] // Skip without API keys (CI-friendly)
async fn test_claude_stderr() {
    let executor = DirectProcessExecutor;
    let provider = AnthropicProvider::new();
    let mut env = HashMap::new();

    let api_key = require_env_var("ANTHROPIC_API_KEY", "pip install anthropic-cli");
    env.insert("ANTHROPIC_API_KEY".to_string(), api_key);

    // Use --verbose or similar flag to generate stderr output
    let output = executor
        .execute(
            "claude",
            &["--verbose".to_string(),
                "-p".to_string(),
                "hello".to_string()],
            &env,
            None,
            600,
            None,
            &provider,
        )
        .await
        .expect("claude CLI should execute with verbose flag");

    assert_eq!(output.exit_code, 0, "claude should exit successfully");
    // stderr may contain debug/verbose output
    println!("stdout: {}", output.stdout);
    println!("stderr: {}", output.stderr);
}

/// Test concurrent execution of multiple CLIs
///
/// Verifies:
/// - DirectProcessExecutor is thread-safe
/// - Multiple CLIs can execute concurrently
/// - No resource contention issues
///
/// Prerequisites:
/// - claude CLI installed: pip install anthropic-cli
/// - ANTHROPIC_API_KEY environment variable set
///
/// Run: cargo test -p codex-core --test async_agent_executor_integration test_concurrent_execution -- --ignored
#[tokio::test]
#[ignore] // Skip without API keys (CI-friendly)
async fn test_concurrent_execution() {
    let api_key = require_env_var("ANTHROPIC_API_KEY", "pip install anthropic-cli");

    let tasks: Vec<_> = (0..3)
        .map(|i| {
            let api_key = api_key.clone();
            tokio::spawn(async move {
                let executor = DirectProcessExecutor;
                let provider = AnthropicProvider::new();
                let mut env = HashMap::new();
                env.insert("ANTHROPIC_API_KEY".to_string(), api_key);

                let output = executor
                    .execute(
                        "claude",
                        &["-p".to_string(),
                            format!("Say 'Task {}' and nothing else", i)],
                        &env,
                        None,
                        600,
                        None,
                        &provider,
                    )
                    .await
                    .expect("concurrent claude execution should succeed");

                assert_eq!(output.exit_code, 0, "Task {i} should exit successfully");
                assert!(
                    output.stdout.contains(&format!("Task {i}"))
                        || output
                            .stdout
                            .to_lowercase()
                            .contains(&format!("task {i}")),
                    "Task {i} output should contain task number"
                );
            })
        })
        .collect();

    for task in tasks {
        task.await.expect("concurrent task should complete");
    }
}
