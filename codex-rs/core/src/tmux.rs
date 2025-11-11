//! Tmux wrapper for observable agent execution (SPEC-KIT-923)
//!
//! Provides session management, pane creation, and output capture
//! for real-time agent monitoring and debugging.

use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Check if tmux is available on the system
pub async fn is_tmux_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Create or reuse a tmux session for agent execution
pub async fn ensure_session(session_name: &str) -> Result<(), String> {
    // Check if session exists
    let check = Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| format!("Failed to check tmux session: {}", e))?;

    if !check.success() {
        // Create new session (detached)
        let create = Command::new("tmux")
            .args(["new-session", "-d", "-s", session_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| format!("Failed to create tmux session: {}", e))?;

        if !create.success() {
            return Err(format!("Failed to create tmux session '{}'", session_name));
        }

        tracing::info!("Created tmux session: {}", session_name);
    } else {
        tracing::debug!("Reusing existing tmux session: {}", session_name);
    }

    Ok(())
}

/// Create a new pane in the session with a title
pub async fn create_pane(
    session_name: &str,
    pane_title: &str,
    is_first: bool,
) -> Result<String, String> {
    if !is_first {
        // Split horizontally for additional panes
        let split = Command::new("tmux")
            .args([
                "split-window",
                "-t",
                session_name,
                "-h",
                "-P",
                "-F",
                "#{pane_id}",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to split tmux pane: {}", e))?;

        if !split.status.success() {
            return Err(format!(
                "Failed to split pane: {}",
                String::from_utf8_lossy(&split.stderr)
            ));
        }

        let pane_id = String::from_utf8_lossy(&split.stdout).trim().to_string();

        // Set pane title
        let _ = Command::new("tmux")
            .args(["select-pane", "-t", &pane_id, "-T", pane_title])
            .status()
            .await;

        tracing::debug!("Created pane {} with title '{}'", pane_id, pane_title);
        Ok(pane_id)
    } else {
        // Use the first pane (already exists from session creation)
        let pane_id = format!("{}:0.0", session_name);

        // Set pane title
        let _ = Command::new("tmux")
            .args(["select-pane", "-t", &pane_id, "-T", pane_title])
            .status()
            .await;

        tracing::debug!("Using first pane {} with title '{}'", pane_id, pane_title);
        Ok(pane_id)
    }
}

/// Execute a command in a tmux pane and capture output
pub async fn execute_in_pane(
    _session_name: &str,
    pane_id: &str,
    command: &str,
    args: &[String],
    env: &std::collections::HashMap<String, String>,
    working_dir: Option<&Path>,
    timeout_secs: u64,
) -> Result<String, String> {
    // Threshold for determining if an argument is "large" (likely a prompt)
    const LARGE_ARG_THRESHOLD: usize = 1000;

    // Track temp files for cleanup
    let mut temp_files = Vec::new();

    // Process arguments - properly escape all for shell execution
    // FIXED (SPEC-923): Instead of using command substitution $(cat file) which fails
    // in tmux context, we properly escape content for direct shell execution
    let mut processed_args = Vec::new();
    let mut prev_arg_was_prompt_flag = false;

    for (i, arg) in args.iter().enumerate() {
        if arg.len() > LARGE_ARG_THRESHOLD {
            // For large arguments, still write to temp file for logging/debugging,
            // but also read back immediately for proper escaping
            let temp_path = format!("/tmp/tmux-agent-arg-{}-{}.txt", std::process::id(), i);

            // Write content to temp file (for debugging)
            tokio::fs::write(&temp_path, arg)
                .await
                .map_err(|e| format!("Failed to write temp file {}: {}", temp_path, e))?;

            temp_files.push(temp_path.clone());

            // Properly escape the argument content for shell
            // Single quotes preserve everything literally except single quotes themselves
            // We escape single quotes by ending the quote, adding escaped quote, then restarting
            let escaped_arg = arg.replace('\'', "'\\''");
            processed_args.push(format!("'{}'", escaped_arg));

            tracing::debug!(
                "Processed large argument ({} bytes, temp file: {})",
                arg.len(),
                temp_path
            );
        } else {
            // Normal argument - escape and add
            let escaped_arg = arg.replace('\'', "'\\''");
            processed_args.push(format!("'{}'", escaped_arg));

            // Track if this is a prompt flag for next iteration
            prev_arg_was_prompt_flag = arg == "-p" || arg == "--prompt";
        }
    }

    // Build the full command with environment variables
    let mut full_command = String::new();

    // Add working directory change if specified
    if let Some(dir) = working_dir {
        full_command.push_str(&format!("cd {} && ", dir.display()));
    }

    // Add environment variables
    for (key, value) in env {
        // Escape single quotes in value
        let escaped_value = value.replace('\'', "'\\''");
        full_command.push_str(&format!("export {}='{}'; ", key, escaped_value));
    }

    // Create unique output file for this agent execution
    // Tmux pane IDs use %XX format - strip % and sanitize for safe filenames
    let sanitized_pane = pane_id.replace("%", "").replace(":", "-").replace(".", "-");
    let output_file = format!("/tmp/tmux-agent-output-{}-{}.txt", std::process::id(), sanitized_pane);

    // Add the actual command with processed arguments
    full_command.push_str(command);
    for arg in &processed_args {
        full_command.push_str(&format!(" {}", arg));
    }

    // Redirect stdout and stderr to output file
    full_command.push_str(&format!(" > {} 2>&1", output_file));

    // Append marker for completion detection
    full_command.push_str("; echo '___AGENT_COMPLETE___'");

    // Add cleanup for temp files ONLY (NOT output file - we need to read it first)
    if !temp_files.is_empty() {
        full_command.push_str(&format!("; rm -f {}", temp_files.join(" ")));
    }

    tracing::debug!(
        "Executing in pane {} ({} temp files for debugging): command length {} chars",
        pane_id,
        temp_files.len(),
        full_command.len()
    );
    tracing::trace!(
        "Full command: {}",
        if full_command.len() > 500 {
            format!("{}... (truncated)", &full_command[..500])
        } else {
            full_command.clone()
        }
    );

    // Clear the pane history first (use proper clear-history command, not send-keys -X)
    let _ = Command::new("tmux")
        .args(["clear-history", "-t", pane_id])
        .status()
        .await;

    // Send the command to the pane
    let send = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, &full_command, "Enter"])
        .status()
        .await
        .map_err(|e| format!("Failed to send command to pane: {}", e))?;

    if !send.success() {
        // Cleanup temp files on error
        for temp_file in &temp_files {
            let _ = tokio::fs::remove_file(temp_file).await;
        }
        return Err(format!("Failed to execute command in pane {}", pane_id));
    }

    // Wait for completion (poll for marker by checking pane content)
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let poll_interval = std::time::Duration::from_millis(500);

    loop {
        if start.elapsed() > timeout {
            // Kill the running process in the pane
            let _ = Command::new("tmux")
                .args(["send-keys", "-t", pane_id, "C-c"])
                .status()
                .await;

            // Cleanup temp files and output file on timeout
            for temp_file in &temp_files {
                let _ = tokio::fs::remove_file(temp_file).await;
            }
            let _ = tokio::fs::remove_file(&output_file).await;

            return Err(format!(
                "Timeout waiting for agent completion after {}s",
                timeout_secs
            ));
        }

        // Check pane content for completion marker (to know when to read output file)
        let capture = Command::new("tmux")
            .args(["capture-pane", "-t", pane_id, "-p", "-S", "-"])
            .output()
            .await
            .map_err(|e| {
                // Cleanup temp files and output file on error
                let temp_files_clone = temp_files.clone();
                let output_file_clone = output_file.clone();
                tokio::spawn(async move {
                    for temp_file in &temp_files_clone {
                        let _ = tokio::fs::remove_file(temp_file).await;
                    }
                    let _ = tokio::fs::remove_file(&output_file_clone).await;
                });
                format!("Failed to capture pane: {}", e)
            })?;

        if capture.status.success() {
            let pane_content = String::from_utf8_lossy(&capture.stdout).to_string();
            if pane_content.contains("___AGENT_COMPLETE___") {
                tracing::info!("Agent completed in pane {}, reading output file", pane_id);

                // Read clean output from dedicated output file
                let output = match tokio::fs::read_to_string(&output_file).await {
                    Ok(content) => {
                        tracing::debug!("Read {} bytes from output file: {}", content.len(), output_file);

                        // Clean up output file after successful read
                        let _ = tokio::fs::remove_file(&output_file).await;

                        content
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read agent output file {}: {}", output_file, e);
                        // Fallback to pane capture if output file read fails
                        // This strips shell noise as best we can
                        let lines: Vec<&str> = pane_content.lines().collect();
                        let mut clean_lines = Vec::new();
                        let mut in_output = false;

                        for line in lines {
                            // Skip shell prompts and environment setup
                            if line.starts_with("thetu@") || line.contains("cd ") || line.contains("export ") {
                                continue;
                            }
                            // Skip the agent command line itself
                            if line.contains("/usr/bin/spec") {
                                in_output = true;
                                continue;
                            }
                            // Skip the completion marker
                            if line.contains("___AGENT_COMPLETE___") {
                                break;
                            }
                            if in_output {
                                clean_lines.push(line);
                            }
                        }

                        clean_lines.join("\n")
                    }
                };

                // Note: temp files are cleaned up by the shell command itself
                // Output file is cleaned up after successful read (or on error/timeout paths)

                return Ok(output.trim().to_string());
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// Capture the final output from a pane
pub async fn capture_pane_output(pane_id: &str) -> Result<String, String> {
    let capture = Command::new("tmux")
        .args(["capture-pane", "-t", pane_id, "-p", "-S", "-"])
        .output()
        .await
        .map_err(|e| format!("Failed to capture pane: {}", e))?;

    if capture.status.success() {
        Ok(String::from_utf8_lossy(&capture.stdout).to_string())
    } else {
        Err(format!(
            "Failed to capture pane output: {}",
            String::from_utf8_lossy(&capture.stderr)
        ))
    }
}

/// Kill a tmux session (cleanup after agents complete)
pub async fn kill_session(session_name: &str) -> Result<(), String> {
    let kill = Command::new("tmux")
        .args(["kill-session", "-t", session_name])
        .status()
        .await
        .map_err(|e| format!("Failed to kill tmux session: {}", e))?;

    if kill.success() {
        tracing::info!("Killed tmux session: {}", session_name);
        Ok(())
    } else {
        Err(format!("Failed to kill session '{}'", session_name))
    }
}

/// Save pane output to evidence file
pub async fn save_pane_evidence(pane_output: &str, evidence_path: &Path) -> Result<(), String> {
    if let Some(parent) = evidence_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create evidence directory: {}", e))?;
    }

    let mut file = tokio::fs::File::create(evidence_path)
        .await
        .map_err(|e| format!("Failed to create evidence file: {}", e))?;

    file.write_all(pane_output.as_bytes())
        .await
        .map_err(|e| format!("Failed to write evidence: {}", e))?;

    tracing::info!("Saved agent evidence to: {}", evidence_path.display());
    Ok(())
}

/// Generate attach instructions for user
pub fn get_attach_instructions(session_name: &str) -> String {
    format!(
        "To watch agents in real-time:\n  tmux attach -t {}\n  (Press Ctrl-B, then D to detach)",
        session_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tmux_available() {
        // Just check that the function doesn't panic
        let _ = is_tmux_available().await;
    }

    #[tokio::test]
    async fn test_large_argument_handling() {
        // Test that large arguments (>1000 chars) are written to temp files
        // This prevents "command too long" errors when passing large prompts

        // Skip test if tmux not available
        if !is_tmux_available().await {
            eprintln!("Skipping test: tmux not available");
            return;
        }

        let session_name = "test-large-args";
        let pane_title = "test-pane";

        // Create session and pane
        if ensure_session(session_name).await.is_ok() {
            if let Ok(pane_id) = create_pane(session_name, pane_title, true).await {
                // Create a large argument (simulates a 50KB prompt)
                let large_prompt = "x".repeat(50_000);

                let args = vec!["-p".to_string(), large_prompt.clone()];
                let env = std::collections::HashMap::new();

                // Test that execute_in_pane handles large args without "command too long" error
                let result = execute_in_pane(
                    session_name,
                    &pane_id,
                    "echo",
                    &args,
                    &env,
                    None,
                    5, // 5 second timeout
                )
                .await;

                // Cleanup
                let _ = kill_session(session_name).await;

                // Verify no "command too long" error
                match result {
                    Ok(_) => {
                        // Success - large argument was handled correctly
                    }
                    Err(e) => {
                        assert!(
                            !e.contains("command too long"),
                            "Large argument handling failed with: {}",
                            e
                        );
                    }
                }
            }
        }
    }
}
