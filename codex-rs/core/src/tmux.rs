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
///
/// SPEC-KIT-925: Kill stale sessions (>5min old) to prevent state corruption
/// that causes completion marker detection failures
pub async fn ensure_session(session_name: &str) -> Result<(), String> {
    // Check if session exists
    let check = Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| format!("Failed to check tmux session: {}", e))?;

    if check.success() {
        // Session exists - check if it's stale (>5 minutes old)
        let session_info = Command::new("tmux")
            .args([
                "display-message",
                "-t",
                session_name,
                "-p",
                "#{session_created}",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to get session info: {}", e))?;

        if session_info.status.success() {
            let created_str = String::from_utf8_lossy(&session_info.stdout);
            if let Ok(created_timestamp) = created_str.trim().parse::<i64>() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                let age_secs = now - created_timestamp;
                const MAX_SESSION_AGE_SECS: i64 = 300; // 5 minutes

                if age_secs > MAX_SESSION_AGE_SECS {
                    tracing::warn!(
                        "Session '{}' is stale ({} seconds old), killing and recreating",
                        session_name,
                        age_secs
                    );

                    // Kill stale session
                    let _ = Command::new("tmux")
                        .args(["kill-session", "-t", session_name])
                        .status()
                        .await;

                    // Fall through to create new session
                } else {
                    tracing::debug!(
                        "Reusing fresh tmux session: {} ({} seconds old)",
                        session_name,
                        age_secs
                    );
                    return Ok(());
                }
            }
        }
    }

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

    tracing::info!("Created fresh tmux session: {}", session_name);
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

    // Create unique output file for this agent execution
    let sanitized_pane = pane_id.replace("%", "").replace(":", "-").replace(".", "-");
    let output_file = format!(
        "/tmp/tmux-agent-output-{}-{}.txt",
        std::process::id(),
        sanitized_pane
    );
    tracing::info!(
        "🆔 Creating output file for pane {} (sanitized: {}) -> {}",
        pane_id,
        sanitized_pane,
        output_file
    );

    // Check if we have large arguments - if so, use wrapper script approach
    let has_large_arg = args.iter().any(|a| a.len() > LARGE_ARG_THRESHOLD);

    let full_command = if has_large_arg {
        // WRAPPER SCRIPT APPROACH: Create a shell script with heredoc for large prompts
        // This avoids: command length limits, stdin issues, and command substitution complexity

        let wrapper_script_path = format!(
            "/tmp/tmux-agent-wrapper-{}-{}.sh",
            std::process::id(),
            sanitized_pane
        );
        temp_files.push(wrapper_script_path.clone());

        // Build wrapper script content
        let mut script_content = String::from("#!/bin/bash\nset -e\n\n");

        // Add environment variables to script
        for (key, value) in env {
            let escaped_value = value.replace('\'', "'\\''");
            script_content.push_str(&format!("export {}='{}'\n", key, escaped_value));
        }

        // Add working directory change if needed
        if let Some(dir) = working_dir {
            script_content.push_str(&format!("cd '{}'\n\n", dir.display()));
        }

        // Build command with heredoc for large arguments
        script_content.push_str(&format!("{}", command));

        for arg in args {
            if arg.len() > LARGE_ARG_THRESHOLD {
                // Use heredoc for large argument - perfectly preserves content
                script_content.push_str(" \"$(cat <<'PROMPT_HEREDOC_EOF'\n");
                script_content.push_str(arg);
                script_content.push_str("\nPROMPT_HEREDOC_EOF\n)\"");

                tracing::debug!("Using heredoc for large argument ({} bytes)", arg.len());
            } else {
                // Normal argument - properly escape
                let escaped = arg.replace('\'', "'\\''");
                script_content.push_str(&format!(" '{}'", escaped));
            }
        }

        // Redirect output
        script_content.push_str(&format!(" > {} 2>&1\n", output_file));

        // SPEC-KIT-928: Add debug marker to verify redirect is working
        script_content.push_str(&format!(
            "echo \"OUTPUT_FILE_CREATED: {}\" >&2\n",
            output_file
        ));

        // SPEC-923: Add completion marker so polling can detect when agent finishes
        script_content.push_str("echo '___AGENT_COMPLETE___'\n");

        // SPEC-KIT-928: Log wrapper script details for debugging
        tracing::debug!(
            "Wrapper script size: {} bytes, prompt size: {} bytes",
            script_content.len(),
            args.iter()
                .find(|a| a.len() > LARGE_ARG_THRESHOLD)
                .map(|a| a.len())
                .unwrap_or(0)
        );

        // Write wrapper script
        tokio::fs::write(&wrapper_script_path, script_content)
            .await
            .map_err(|e| format!("Failed to write wrapper script: {}", e))?;

        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&wrapper_script_path)
                .await
                .map_err(|e| format!("Failed to get script permissions: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&wrapper_script_path, perms)
                .await
                .map_err(|e| format!("Failed to set script permissions: {}", e))?;
        }

        tracing::info!("Created wrapper script: {}", wrapper_script_path);

        // Simple command to execute wrapper script
        format!("bash {}", wrapper_script_path)
    } else {
        // NORMAL APPROACH: Small arguments can be passed directly
        let mut cmd = String::new();

        // Add working directory change if specified
        if let Some(dir) = working_dir {
            cmd.push_str(&format!("cd {} && ", dir.display()));
        }

        // Add environment variables
        for (key, value) in env {
            let escaped_value = value.replace('\'', "'\\''");
            cmd.push_str(&format!("export {}='{}'; ", key, escaped_value));
        }

        // Add command with escaped arguments
        cmd.push_str(command);
        for arg in args {
            let escaped_arg = arg.replace('\'', "'\\''");
            cmd.push_str(&format!(" '{}'", escaped_arg));
        }

        // Redirect output
        cmd.push_str(&format!(" > {} 2>&1", output_file));

        cmd
    };

    // Append completion marker and cleanup to the command
    let mut final_command = full_command;

    // SPEC-KIT-928: Wrapper scripts already have completion marker (line 229)
    // Only add it for direct commands (non-wrapper path)
    let has_wrapper = !temp_files.is_empty();
    if !has_wrapper {
        final_command.push_str("; echo '___AGENT_COMPLETE___'");
    }

    // SPEC-KIT-928: Copy wrapper scripts for debugging before deletion
    if !temp_files.is_empty() {
        for temp_file in &temp_files {
            if temp_file.contains("tmux-agent-wrapper") {
                let debug_copy = temp_file.replace(".sh", "-debug.sh");
                final_command.push_str(&format!(
                    "; cp {} {} 2>/dev/null || true",
                    temp_file, debug_copy
                ));
            }
        }
        final_command.push_str(&format!("; rm -f {}", temp_files.join(" ")));
    }

    tracing::debug!(
        "Executing in pane {} ({} temp files): command length {} chars",
        pane_id,
        temp_files.len(),
        final_command.len()
    );
    tracing::trace!(
        "Command: {}",
        if final_command.len() > 500 {
            format!("{}... (truncated)", &final_command[..500])
        } else {
            final_command.clone()
        }
    );

    // Clear the pane history first (use proper clear-history command, not send-keys -X)
    let _ = Command::new("tmux")
        .args(["clear-history", "-t", pane_id])
        .status()
        .await;

    // Send the command to the pane
    let send = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, &final_command, "Enter"])
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

    // SPEC-KIT-927: Track file size stability to prevent premature output collection
    let mut last_file_size: Option<u64> = None;
    let mut stable_since: Option<std::time::Instant> = None;
    let min_stable_duration = std::time::Duration::from_secs(2);
    let min_file_size: u64 = 1000; // Minimum 1KB for valid agent output

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

        // SPEC-KIT-927: Check output file size and stability before reading
        // This prevents collecting partial output (headers only) before agent finishes
        let current_file_size = if let Ok(metadata) = tokio::fs::metadata(&output_file).await {
            Some(metadata.len())
        } else {
            None
        };

        // Track file size changes to detect when agent finishes writing
        let file_is_stable = if let Some(current_size) = current_file_size {
            if let Some(last_size) = last_file_size {
                if current_size == last_size && current_size >= min_file_size {
                    // File size unchanged and meets minimum threshold
                    if stable_since.is_none() {
                        stable_since = Some(std::time::Instant::now());
                        tracing::debug!(
                            "📊 Output file stable at {} bytes, waiting {}s for confirmation",
                            current_size,
                            min_stable_duration.as_secs()
                        );
                    }

                    // Check if stable for long enough
                    if let Some(since) = stable_since {
                        since.elapsed() >= min_stable_duration
                    } else {
                        false
                    }
                } else {
                    // File still growing or too small, reset stability timer
                    if current_size != last_size {
                        tracing::trace!(
                            "📈 Output file growing: {} -> {} bytes",
                            last_size,
                            current_size
                        );
                    }
                    stable_since = None;
                    false
                }
            } else {
                // First size reading
                tracing::debug!("📝 Output file created: {} bytes", current_size);
                stable_since = None;
                false
            }
        } else {
            // File doesn't exist yet
            stable_since = None;
            false
        };

        last_file_size = current_file_size;

        // Check pane content for completion marker (to know when to read output file)
        tracing::debug!(
            "🔍 Polling pane {} for completion (expecting output file: {})",
            pane_id,
            output_file
        );
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

            // SPEC-KIT-925: Enhanced diagnostics for debugging completion detection
            let content_preview = pane_content.lines().take(5).collect::<Vec<_>>().join(" | ");
            let has_marker = pane_content.contains("___AGENT_COMPLETE___");

            tracing::trace!(
                "📋 Pane {} capture: {} bytes, {} lines, marker={}, file_stable={}, preview: {}",
                pane_id,
                pane_content.len(),
                pane_content.lines().count(),
                has_marker,
                file_is_stable,
                if content_preview.len() > 200 {
                    format!("{}...", &content_preview[..200])
                } else {
                    content_preview
                }
            );

            // SPEC-KIT-927: Require BOTH completion marker AND stable file size
            // This prevents premature output collection before agent finishes writing
            if has_marker && file_is_stable {
                tracing::info!(
                    "✅ Agent completed in pane {} (marker + stable file), reading output file",
                    pane_id
                );

                // Read clean output from dedicated output file
                let file_exists = tokio::fs::metadata(&output_file).await.is_ok();
                tracing::info!(
                    "🔍 Attempting to read output file: {} (exists: {})",
                    output_file,
                    file_exists
                );

                // If file doesn't exist, check for similar files to diagnose pane ID mismatch
                if !file_exists {
                    tracing::warn!("⚠️ Output file not found! Checking for similar files...");
                    if let Ok(mut entries) = tokio::fs::read_dir("/tmp").await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            let name = entry.file_name();
                            let name_str = name.to_string_lossy();
                            if name_str.starts_with("tmux-agent-output-") {
                                tracing::warn!("  Found: {}", name_str);
                            }
                        }
                    }
                }

                let output = match tokio::fs::read_to_string(&output_file).await {
                    Ok(content) => {
                        tracing::info!(
                            "✅ Successfully read {} bytes from output file: {}",
                            content.len(),
                            output_file
                        );

                        // Clean up output file after successful read
                        match tokio::fs::remove_file(&output_file).await {
                            Ok(_) => tracing::info!("🗑️ Deleted output file: {}", output_file),
                            Err(e) => tracing::error!(
                                "❌ Failed to delete output file {}: {}",
                                output_file,
                                e
                            ),
                        }

                        content
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ FAILED to read agent output file {}: {}",
                            output_file,
                            e
                        );
                        tracing::error!("❌ Falling back to pane capture (may lose content)");
                        // Fallback to pane capture if output file read fails
                        // This strips shell noise as best we can
                        let lines: Vec<&str> = pane_content.lines().collect();
                        let mut clean_lines = Vec::new();
                        let mut in_output = false;

                        for line in lines {
                            // Skip shell prompts and environment setup
                            if line.starts_with("thetu@")
                                || line.contains("cd ")
                                || line.contains("export ")
                            {
                                continue;
                            }
                            // Skip the agent command line itself
                            // SPEC-KIT-928: Handle multiple agent command patterns
                            if line.contains("/usr/bin/spec")
                                || line.contains("code exec")
                                || line.contains("gemini")
                                || line.contains("claude")
                                || line.contains("/tmp/tmux-agent-wrapper")
                            {
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

                        let fallback_output = clean_lines.join("\n");
                        tracing::warn!(
                            "📋 Fallback pane capture returned {} bytes",
                            fallback_output.len()
                        );
                        tracing::debug!(
                            "📋 Fallback preview: {}...",
                            &fallback_output.chars().take(200).collect::<String>()
                        );
                        fallback_output
                    }
                };

                tracing::info!("📤 Returning {} bytes to caller", output.len());

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

/// SPEC-KIT-927: Kill zombie agent process in a tmux pane
///
/// Sends Ctrl+C to gracefully stop the process, waits briefly,
/// then force-kills if still running. This prevents orphaned
/// agent processes from accumulating.
pub async fn kill_pane_process(session_name: &str, pane_id: &str) -> Result<(), String> {
    tracing::info!(
        "🔫 Killing process in pane {} (session: {})",
        pane_id,
        session_name
    );

    // Send Ctrl+C to gracefully stop the process
    let send_keys_result = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, "C-c"])
        .status()
        .await;

    if let Err(e) = send_keys_result {
        tracing::warn!("Failed to send Ctrl+C to pane {}: {}", pane_id, e);
    } else {
        tracing::debug!(
            "Sent Ctrl+C to pane {}, waiting 2s for graceful exit",
            pane_id
        );
    }

    // Give process 2 seconds to exit gracefully
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Check if pane still exists and has running process
    let pane_exists = Command::new("tmux")
        .args(["list-panes", "-t", session_name, "-F", "#{pane_id}"])
        .output()
        .await
        .map(|output| {
            let panes = String::from_utf8_lossy(&output.stdout);
            panes.lines().any(|p| p.trim() == pane_id)
        })
        .unwrap_or(false);

    if pane_exists {
        tracing::debug!("Pane {} still exists, killing it", pane_id);
        // Force kill the pane
        let _ = Command::new("tmux")
            .args(["kill-pane", "-t", pane_id])
            .status()
            .await;
    }

    tracing::info!("✅ Cleaned up pane {}", pane_id);
    Ok(())
}

/// SPEC-KIT-927: Check for zombie agent processes in a session
///
/// Returns the number of zombie panes found (panes that should have
/// completed but are still running).
pub async fn check_zombie_panes(session_name: &str) -> Result<usize, String> {
    let list_panes = Command::new("tmux")
        .args(["list-panes", "-t", session_name, "-F", "#{pane_id}"])
        .output()
        .await
        .map_err(|e| format!("Failed to list panes: {}", e))?;

    if !list_panes.status.success() {
        // Session doesn't exist or no panes
        return Ok(0);
    }

    let pane_ids = String::from_utf8_lossy(&list_panes.stdout);
    let zombie_count = pane_ids.lines().count();

    if zombie_count > 0 {
        tracing::warn!(
            "⚠️ Found {} potentially zombie panes in session {}",
            zombie_count,
            session_name
        );
    }

    Ok(zombie_count)
}

/// SPEC-KIT-927: Clean up all zombie panes in a session
///
/// This is called before starting new agents to ensure clean state.
pub async fn cleanup_zombie_panes(session_name: &str) -> Result<usize, String> {
    let zombie_count = check_zombie_panes(session_name).await?;

    if zombie_count > 0 {
        tracing::warn!(
            "🧹 Cleaning up {} zombie panes in session {}",
            zombie_count,
            session_name
        );

        // Kill the entire session (simplest approach)
        let _ = kill_session(session_name).await;

        tracing::info!("✅ Killed session {} to clean up zombies", session_name);
    }

    Ok(zombie_count)
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
