use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::mpsc;

use super::types::{CliError, ResponseMetadata, StreamEvent};

/// Parse Claude CLI stream-json format
///
/// Claude outputs newline-delimited JSON:
/// - First line: {"type": "system", "subtype": "init", ...}
/// - Subsequent lines: {"type": "assistant", "message": {...}}
pub async fn parse_claude_stream(
    stdout: ChildStdout,
    tx: mpsc::Sender<StreamEvent>,
) -> Result<(), CliError> {
    let mut reader = BufReader::new(stdout).lines();

    while let Some(line) = reader.next_line().await.map_err(|e| CliError::Internal {
        message: format!("Failed to read stdout: {e}"),
    })? {
        let json: Value = serde_json::from_str(&line).map_err(|e| CliError::ParseError {
            details: format!("Invalid JSON: {e}"),
        })?;

        match json["type"].as_str() {
            Some("system") => {
                // Log init metadata but don't send to user
                tracing::debug!(
                    "Claude init: session_id={}, model={}",
                    json["session_id"].as_str().unwrap_or("unknown"),
                    json["model"].as_str().unwrap_or("unknown")
                );
            }
            Some("assistant") => {
                // Extract response text
                if let Some(content) = json["message"]["content"].as_array() {
                    for item in content {
                        if item["type"] == "text"
                            && let Some(text) = item["text"].as_str() {
                                tx.send(StreamEvent::Delta(text.to_string()))
                                    .await
                                    .map_err(|e| CliError::Internal {
                                        message: format!("Channel send failed: {e}"),
                                    })?;
                            }
                    }
                }

                // Extract token usage
                if let Some(usage) = json["message"]["usage"].as_object() {
                    let metadata = ResponseMetadata {
                        model: json["message"]["model"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string(),
                        input_tokens: usage["input_tokens"].as_u64().map(|n| n as usize),
                        output_tokens: usage["output_tokens"].as_u64().map(|n| n as usize),
                    };
                    tx.send(StreamEvent::Metadata(metadata))
                        .await
                        .map_err(|e| CliError::Internal {
                            message: format!("Channel send failed: {e}"),
                        })?;
                }
            }
            _ => {
                tracing::warn!("Unknown stream type: {}", json["type"]);
            }
        }
    }

    tx.send(StreamEvent::Done)
        .await
        .map_err(|e| CliError::Internal {
            message: format!("Channel send failed: {e}"),
        })?;

    Ok(())
}

/// Parse Gemini CLI stream-json format
///
/// Gemini format may differ from Claude - adjust based on actual output
pub async fn parse_gemini_stream(
    stdout: ChildStdout,
    tx: mpsc::Sender<StreamEvent>,
) -> Result<(), CliError> {
    let mut reader = BufReader::new(stdout).lines();
    let mut accumulated_text = String::new();

    while let Some(line) = reader.next_line().await.map_err(|e| CliError::Internal {
        message: format!("Failed to read stdout: {e}"),
    })? {
        // Skip credential loading messages (not JSON)
        if line.starts_with("Loaded cached credentials") || line.starts_with("Attempt ") {
            tracing::debug!("Gemini status: {}", line);
            continue;
        }

        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str::<Value>(&line) {
            // Handle error responses
            if let Some(error) = json.get("error") {
                let code = error["code"].as_i64().unwrap_or(0);
                let message = error["message"]
                    .as_str()
                    .unwrap_or("Unknown error")
                    .to_string();

                return Err(CliError::ProcessFailed {
                    code: code as i32,
                    stderr: message,
                });
            }

            // Handle Gemini message format: {"type":"message","role":"assistant","content":"...","delta":true}
            if json["type"] == "message" && json["role"] == "assistant"
                && let Some(content) = json["content"].as_str() {
                    accumulated_text.push_str(content);
                    tx.send(StreamEvent::Delta(content.to_string()))
                        .await
                        .map_err(|e| CliError::Internal {
                            message: format!("Channel send failed: {e}"),
                        })?;
                }

            // Handle result/stats
            if json["type"] == "result"
                && let Some(stats) = json["stats"].as_object() {
                    tx.send(StreamEvent::Metadata(ResponseMetadata {
                        model: "gemini".to_string(),
                        input_tokens: stats["input_tokens"].as_u64().map(|n| n as usize),
                        output_tokens: stats["output_tokens"].as_u64().map(|n| n as usize),
                    }))
                    .await
                    .map_err(|e| CliError::Internal {
                        message: format!("Channel send failed: {e}"),
                    })?;
                }
        } else {
            // Fallback: treat as plain text response
            // Gemini may output plain text after retries succeed
            if !line.is_empty() {
                accumulated_text.push_str(&line);
                tx.send(StreamEvent::Delta(line))
                    .await
                    .map_err(|e| CliError::Internal {
                        message: format!("Channel send failed: {e}"),
                    })?;
            }
        }
    }

    // Send metadata if we got any response
    if !accumulated_text.is_empty() {
        tx.send(StreamEvent::Metadata(ResponseMetadata {
            model: "gemini".to_string(),
            input_tokens: None, // Not available in text mode
            output_tokens: None,
        }))
        .await
        .map_err(|e| CliError::Internal {
            message: format!("Channel send failed: {e}"),
        })?;
    }

    tx.send(StreamEvent::Done)
        .await
        .map_err(|e| CliError::Internal {
            message: format!("Channel send failed: {e}"),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_claude_stream() {
        // Test will be implemented when we have actual CLI output samples
        // For now, this is a placeholder
    }

    #[tokio::test]
    async fn test_parse_gemini_stream() {
        // Test will be implemented when we have actual CLI output samples
    }
}
