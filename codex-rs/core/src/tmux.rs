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
            .args([
                "select-pane",
                "-t",
                &pane_id,
                "-T",
                pane_title,
            ])
            .status()
            .await;

        tracing::debug!("Created pane {} with title '{}'", pane_id, pane_title);
        Ok(pane_id)
    } else {
        // Use the first pane (already exists from session creation)
        let pane_id = format!("{}:0.0", session_name);

        // Set pane title
        let _ = Command::new("tmux")
            .args([
                "select-pane",
                "-t",
                &pane_id,
                "-T",
                pane_title,
            ])
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

    // Add the actual command
    full_command.push_str(command);
    for arg in args {
        // Escape single quotes in args
        let escaped_arg = arg.replace('\'', "'\\''");
        full_command.push_str(&format!(" '{}'", escaped_arg));
    }

    // Append marker for completion detection
    full_command.push_str("; echo '___AGENT_COMPLETE___'");

    tracing::debug!("Executing in pane {}: {}", pane_id, full_command);

    // Clear the pane first
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, "-X", "clear-history"])
        .status()
        .await;

    // Send the command to the pane
    let send = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, &full_command, "Enter"])
        .status()
        .await
        .map_err(|e| format!("Failed to send command to pane: {}", e))?;

    if !send.success() {
        return Err(format!("Failed to execute command in pane {}", pane_id));
    }

    // Wait for completion (poll for marker)
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

            return Err(format!(
                "Timeout waiting for agent completion after {}s",
                timeout_secs
            ));
        }

        // Capture pane content
        let capture = Command::new("tmux")
            .args([
                "capture-pane",
                "-t",
                pane_id,
                "-p",
                "-S",
                "-",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to capture pane: {}", e))?;

        if capture.status.success() {
            let output = String::from_utf8_lossy(&capture.stdout).to_string();
            if output.contains("___AGENT_COMPLETE___") {
                // Remove the marker from output
                let output = output.replace("___AGENT_COMPLETE___", "").trim().to_string();
                tracing::info!("Agent completed in pane {}", pane_id);
                return Ok(output);
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// Capture the final output from a pane
pub async fn capture_pane_output(pane_id: &str) -> Result<String, String> {
    let capture = Command::new("tmux")
        .args([
            "capture-pane",
            "-t",
            pane_id,
            "-p",
            "-S",
            "-",
        ])
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
pub async fn save_pane_evidence(
    pane_output: &str,
    evidence_path: &Path,
) -> Result<(), String> {
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
}
