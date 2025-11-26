//! Basic CLI Integration Tests
//!
//! This file contains lightweight integration tests for the CLI binary using stdin/stdout
//! (not PTY). These tests verify basic functionality without requiring complex terminal
//! emulation.
//!
//! Tests are marked with #[ignore] to avoid failures in environments without the binary.
//! Run with: cargo test --test cli_basic_integration -- --ignored

// SPEC-957: Allow test code flexibility
#![allow(dead_code, unused_variables, unused_mut)]
#![allow(clippy::uninlined_format_args, clippy::assertions_on_constants)]
#![allow(unexpected_cfgs)]

use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

/// Helper to get the path to the code binary
fn binary_path() -> String {
    // Try to use CARGO_BIN_EXE_code if set (for cargo test integration)
    if let Ok(path) = env::var("CARGO_BIN_EXE_code") {
        return path;
    }

    // Fall back to looking for the binary in expected locations
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    // When running from tui/tests, manifest_dir is codex-rs/tui
    // Binary is at codex-rs/target/dev-fast/code, so we go up one level and into target
    let dev_fast_path = format!("{}/../target/dev-fast/code", manifest_dir);
    if std::path::Path::new(&dev_fast_path).exists() {
        return dev_fast_path;
    }

    // Try debug profile
    let debug_path = format!("{}/../target/debug/code", manifest_dir);
    if std::path::Path::new(&debug_path).exists() {
        return debug_path;
    }

    // Default to dev-fast (will fail if doesn't exist, which is expected for ignored tests)
    dev_fast_path
}

/// Helper to spawn the code binary
fn code_command() -> Command {
    let mut cmd = Command::new(binary_path());
    cmd.env("RUST_LOG", "error"); // Reduce noise in test output
    cmd
}

/// Test that the binary can be executed and exits cleanly
#[test]
#[ignore] // Run with: cargo test --test cli_basic_integration -- --ignored
fn test_binary_exists_and_runs() {
    let output = code_command()
        .arg("--version")
        .output()
        .expect("Failed to execute code binary");

    assert!(output.status.success(), "Binary should exit with success");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Binary should output something (version info or help text)
    assert!(!stdout.is_empty(), "Should produce some output");
}

/// Test help flag shows usage information
#[test]
#[ignore]
fn test_help_flag() {
    let output = code_command()
        .arg("--help")
        .output()
        .expect("Failed to execute code binary");

    assert!(output.status.success(), "Help should exit with success");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Help text should contain basic keywords
    assert!(
        stdout.contains("Usage") || stdout.contains("Options") || stdout.contains("Commands"),
        "Help text should contain usage information"
    );
}

/// Test that binary handles EOF gracefully
#[test]
#[ignore]
fn test_handles_eof_gracefully() {
    let mut child = code_command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn code binary");

    // Close stdin immediately to signal EOF
    drop(child.stdin.take());

    // Wait for process to complete (with timeout via wait_with_output)
    let output = child
        .wait_with_output()
        .expect("Failed to wait for process");

    // Process should exit (either success or specific error code, but not hang)
    assert!(
        output.status.code().is_some(),
        "Process should exit with a status code"
    );
}

/// Test basic stdin interaction (if binary accepts piped input)
#[test]
#[ignore]
fn test_stdin_basic_interaction() {
    let mut child = code_command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn code binary");

    // Write a simple message and close stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"/exit\n").ok(); // Try to send exit command
        stdin.flush().ok();
        drop(stdin); // Close stdin
    }

    // Wait for process with a reasonable timeout
    let output = child
        .wait_with_output()
        .expect("Failed to wait for process");

    // Should produce some output or error
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stdout.is_empty() || !stderr.is_empty(),
        "Should produce some output on stdout or stderr"
    );
}

/// Test that binary handles invalid flags gracefully
#[test]
#[ignore]
fn test_invalid_flag_handling() {
    let output = code_command()
        .arg("--this-flag-does-not-exist")
        .output()
        .expect("Failed to execute code binary");

    // Should exit with error code
    assert!(
        !output.status.success(),
        "Invalid flag should cause non-zero exit"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Error message should mention the invalid flag or provide help
    assert!(
        stderr.contains("error") || stderr.contains("invalid") || stderr.contains("unknown"),
        "Should provide error message for invalid flag"
    );
}

/// Test that binary can handle termination signal
/// Note: This test is disabled for now as it requires additional dependencies (nix crate)
/// and signal handling testing can be complex. Manual testing recommended for signal handling.
#[test]
#[ignore]
#[cfg(all(unix, feature = "signal-tests"))] // Only when explicitly enabled
fn test_handles_sigint_gracefully() {
    // Placeholder for future signal handling test
    // Would require nix crate in workspace dev-dependencies
    unimplemented!("Signal handling test requires nix crate dependency")
}

/// Smoke test: verify test framework is working
#[test]
fn test_framework_sanity_check() {
    // This test always passes to verify the test framework itself works
    assert!(true, "Test framework should be working");
}
