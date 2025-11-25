//! ACE (Agentic Context Engine) MCP client wrapper
//!
//! ACE is a data-only strategy memory (SQLite via MCP).
//! It does NOT call LLMs - the CODE orchestrator calls LLMs using client's API keys.
//!
//! Thin client for calling ACE MCP tools:
//! - ace.playbook.slice: Retrieve relevant playbook heuristics (data retrieval)
//! - ace.learn: Store outcomes for future learning (data storage)
//! - ace.playbook.pin: Pin constitution bullets globally (data storage)
//!
//! Note: MCP client infrastructure ready, full error handling features pending.

#![allow(dead_code)] // Full error handling features pending

use anyhow::{Context, Result, anyhow};
use codex_mcp_client::McpClient;
use mcp_types::{CallToolRequest, CallToolRequestParams, CallToolResult, ContentBlock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OnceCell};
use tracing::{debug, error, info};

/// Timeout for ACE tool calls (30 seconds default)
const ACE_TOOL_TIMEOUT: Duration = Duration::from_secs(30);

/// Result types for ACE operations
#[derive(Debug, Clone)]
pub enum AceResult<T> {
    /// ACE returned successfully
    Ok(T),
    /// ACE is disabled or unavailable
    Disabled,
    /// ACE returned an error
    Error(String),
}

impl<T> AceResult<T> {
    pub fn ok(self) -> Option<T> {
        match self {
            AceResult::Ok(value) => Some(value),
            _ => None,
        }
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, AceResult::Disabled)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, AceResult::Error(_))
    }
}

/// A single playbook bullet/heuristic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookBullet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>, // ACE uses integer IDs
    pub text: String,
    #[serde(default)]
    pub helpful: bool,
    #[serde(default)]
    pub harmful: bool,
    #[serde(default)]
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Response from ace.playbook.slice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookSliceResponse {
    pub bullets: Vec<PlaybookBullet>,
    #[serde(default)]
    pub truncated: bool,
}

/// Response from ace.learn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnResponse {
    pub status: String,
    #[serde(default)]
    pub updated_bullets: UpdatedBullets,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdatedBullets {
    #[serde(default)]
    pub added: usize,
    #[serde(default)]
    pub demoted: usize,
    #[serde(default)]
    pub promoted: usize,
}

/// Response from ace.playbook.pin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinResponse {
    pub status: String,
    #[serde(default)]
    pub pinned_count: usize,
}

/// Global ACE client singleton
static ACE_CLIENT: OnceCell<Arc<Mutex<Option<McpClient>>>> = OnceCell::const_new();
static ACE_DISABLED_LOGGED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// Initialize the ACE MCP client
///
/// This should be called once at startup. Subsequent calls return the existing client.
pub async fn init_ace_client(
    command: String,
    args: Vec<String>,
    env: Option<HashMap<String, String>>,
) -> Result<()> {
    ACE_CLIENT
        .get_or_try_init(|| async {
            match McpClient::new_stdio_client(
                OsString::from(&command),
                args.iter().map(|s| OsString::from(s)).collect(),
                env,
            )
            .await
            {
                Ok(client) => {
                    // Initialize the client with standard MCP handshake
                    match client
                        .initialize(
                            mcp_types::InitializeRequestParams {
                                protocol_version: mcp_types::MCP_SCHEMA_VERSION.to_string(),
                                capabilities: mcp_types::ClientCapabilities {
                                    elicitation: None,
                                    experimental: None,
                                    roots: None,
                                    sampling: None,
                                },
                                client_info: mcp_types::Implementation {
                                    name: "code-cli".to_string(),
                                    title: Some("Code CLI".to_string()),
                                    version: env!("CARGO_PKG_VERSION").to_string(),
                                    user_agent: Some(format!(
                                        "code-cli/{}",
                                        env!("CARGO_PKG_VERSION")
                                    )),
                                },
                            },
                            None,
                            Some(Duration::from_secs(10)),
                        )
                        .await
                    {
                        Ok(_) => {
                            info!("ACE MCP client initialized successfully");
                            Ok::<_, anyhow::Error>(Arc::new(Mutex::new(Some(client))))
                        }
                        Err(e) => {
                            error!("Failed to initialize ACE MCP client: {}", e);
                            Ok::<_, anyhow::Error>(Arc::new(Mutex::new(None)))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to spawn ACE MCP server: {}", e);
                    Ok::<_, anyhow::Error>(Arc::new(Mutex::new(None)))
                }
            }
        })
        .await?;

    Ok(())
}

/// Get the ACE client, logging "ACE: disabled" once if unavailable
async fn get_client() -> Option<Arc<Mutex<Option<McpClient>>>> {
    match ACE_CLIENT.get() {
        Some(client) => Some(client.clone()),
        None => {
            // Log once that ACE is disabled
            ACE_DISABLED_LOGGED.get_or_init(|| {
                info!("ACE: disabled (client not initialized)");
            });
            None
        }
    }
}

/// Call an ACE tool with proper error handling
async fn call_ace_tool(
    tool_name: &str,
    arguments: HashMap<String, Value>,
) -> AceResult<CallToolResult> {
    let Some(client_arc) = get_client().await else {
        return AceResult::Disabled;
    };

    let guard = client_arc.lock().await;
    let Some(client) = guard.as_ref() else {
        return AceResult::Disabled;
    };

    // ACE FastMCP expects arguments wrapped in "input" field
    let wrapped_args = serde_json::json!({
        "input": Value::Object(
            arguments
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect(),
        )
    });

    let params = CallToolRequestParams {
        name: tool_name.to_string(),
        arguments: Some(wrapped_args),
    };

    match client
        .send_request::<CallToolRequest>(params, Some(ACE_TOOL_TIMEOUT))
        .await
    {
        Ok(result) => AceResult::Ok(result),
        Err(e) => {
            error!("ACE tool call failed ({}): {}", tool_name, e);
            AceResult::Error(e.to_string())
        }
    }
}

/// Retrieve relevant playbook heuristics
///
/// # Arguments
/// * `repo_root` - Repository root path
/// * `branch` - Git branch name
/// * `scope` - Scope for filtering (global, specify, tasks, implement, test)
/// * `k` - Number of bullets to retrieve (max 8 recommended)
/// * `include_neutral` - Whether to include neutral bullets
pub async fn playbook_slice(
    repo_root: String,
    branch: String,
    scope: String,
    k: usize,
    include_neutral: bool,
) -> AceResult<PlaybookSliceResponse> {
    debug!(
        "ACE playbook_slice: repo={}, branch={}, scope={}, k={}",
        repo_root, branch, scope, k
    );

    let mut args = HashMap::new();
    args.insert("repo_root".to_string(), Value::String(repo_root));
    args.insert("branch".to_string(), Value::String(branch));
    args.insert("scope".to_string(), Value::String(scope));
    args.insert("k".to_string(), Value::Number(k.into()));
    args.insert("include_neutral".to_string(), Value::Bool(include_neutral));

    match call_ace_tool("playbook_slice", args).await {
        AceResult::Ok(result) => {
            // Parse the result content
            match parse_tool_result::<PlaybookSliceResponse>(&result) {
                Ok(response) => AceResult::Ok(response),
                Err(e) => AceResult::Error(format!("Failed to parse playbook slice: {}", e)),
            }
        }
        AceResult::Disabled => AceResult::Disabled,
        AceResult::Error(e) => AceResult::Error(e),
    }
}

/// Store execution outcome for learning
///
/// # Arguments
/// * `repo_root` - Repository root path
/// * `branch` - Git branch name
/// * `scope` - Task scope
/// * `question` - The original question/task
/// * `attempt` - Summary of what was attempted
/// * `feedback` - Outcome feedback (JSON string with compile/test/lint results)
/// * `bullet_ids_used` - Integer IDs of bullets that were injected into the prompt
pub async fn learn(
    repo_root: String,
    branch: String,
    scope: String,
    question: String,
    attempt: String,
    feedback: String,
    bullet_ids_used: Vec<i32>,
) -> AceResult<LearnResponse> {
    debug!("ACE learn: scope={}, question={}", scope, question);

    let mut args = HashMap::new();
    args.insert("repo_root".to_string(), Value::String(repo_root));
    args.insert("branch".to_string(), Value::String(branch));
    args.insert("scope".to_string(), Value::String(scope));
    args.insert("question".to_string(), Value::String(question));
    args.insert("attempt".to_string(), Value::String(attempt));
    args.insert("feedback".to_string(), Value::String(feedback));
    args.insert(
        "bullet_ids_used".to_string(),
        Value::Array(
            bullet_ids_used
                .into_iter()
                .map(|id| Value::Number(id.into()))
                .collect(),
        ),
    );

    match call_ace_tool("learn", args).await {
        AceResult::Ok(result) => match parse_tool_result::<LearnResponse>(&result) {
            Ok(response) => {
                info!(
                    "ACE learned: +{} bullets, ^{} promoted, v{} demoted",
                    response.updated_bullets.added,
                    response.updated_bullets.promoted,
                    response.updated_bullets.demoted
                );
                AceResult::Ok(response)
            }
            Err(e) => AceResult::Error(format!("Failed to parse learn response: {}", e)),
        },
        AceResult::Disabled => AceResult::Disabled,
        AceResult::Error(e) => AceResult::Error(e),
    }
}

/// Pin constitution bullets globally
///
/// # Arguments
/// * `repo_root` - Repository root path
/// * `branch` - Git branch name
/// * `scope` - Scope to pin to (global, specify, tasks, implement, test)
/// * `bullets` - Bullets to pin (text and kind)
pub async fn pin(
    repo_root: String,
    branch: String,
    scope: String,
    bullets: Vec<(String, String)>, // (text, kind)
) -> AceResult<PinResponse> {
    debug!("ACE pin: {} bullets to scope {}", bullets.len(), scope);

    let mut args = HashMap::new();
    args.insert("repo_root".to_string(), Value::String(repo_root));
    args.insert("branch".to_string(), Value::String(branch));
    args.insert("scope".to_string(), Value::String(scope));
    args.insert(
        "bullets".to_string(),
        Value::Array(
            bullets
                .into_iter()
                .map(|(text, kind)| {
                    serde_json::json!({
                        "text": text,
                        "kind": kind
                    })
                })
                .collect(),
        ),
    );

    match call_ace_tool("playbook_pin", args).await {
        AceResult::Ok(result) => match parse_tool_result::<PinResponse>(&result) {
            Ok(response) => {
                info!("ACE pinned {} bullets", response.pinned_count);
                AceResult::Ok(response)
            }
            Err(e) => AceResult::Error(format!("Failed to parse pin response: {}", e)),
        },
        AceResult::Disabled => AceResult::Disabled,
        AceResult::Error(e) => AceResult::Error(e),
    }
}

/// Parse MCP tool result into typed response
fn parse_tool_result<T: for<'de> Deserialize<'de>>(result: &CallToolResult) -> Result<T> {
    // MCP tools return content as an array of content items
    let content = result
        .content
        .first()
        .ok_or_else(|| anyhow!("ACE tool returned empty content"))?;

    // Extract text from content item
    let text = match content {
        ContentBlock::TextContent(text_content) => &text_content.text,
        ContentBlock::ImageContent(_) => {
            return Err(anyhow!("ACE tool returned image content, expected text"));
        }
        ContentBlock::AudioContent(_) => {
            return Err(anyhow!("ACE tool returned audio content, expected text"));
        }
        ContentBlock::ResourceLink(_) | ContentBlock::EmbeddedResource(_) => {
            return Err(anyhow!("ACE tool returned resource content, expected text"));
        }
    };

    // Log the actual response for debugging
    debug!("ACE MCP response text: {}", text);

    // Parse JSON response
    serde_json::from_str(text).with_context(|| {
        format!(
            "Failed to parse ACE tool response as JSON. Response was: {}",
            text
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ace_result() {
        let ok_result: AceResult<i32> = AceResult::Ok(42);
        assert!(ok_result.clone().ok().is_some());
        assert!(!ok_result.is_disabled());
        assert!(!ok_result.is_error());

        let disabled: AceResult<i32> = AceResult::Disabled;
        assert!(disabled.clone().ok().is_none());
        assert!(disabled.is_disabled());
        assert!(!disabled.is_error());

        let error: AceResult<i32> = AceResult::Error("test".to_string());
        assert!(error.clone().ok().is_none());
        assert!(!error.is_disabled());
        assert!(error.is_error());
    }

    #[test]
    fn test_parse_playbook_slice() {
        let json = r#"{
            "bullets": [
                {"text": "Use X pattern", "helpful": true, "confidence": 0.9},
                {"text": "Avoid Y anti-pattern", "harmful": true, "confidence": 0.8}
            ],
            "truncated": false
        }"#;

        let parsed: PlaybookSliceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.bullets.len(), 2);
        assert!(parsed.bullets[0].helpful);
        assert!(parsed.bullets[1].harmful);
        assert!(!parsed.truncated);
    }

    #[test]
    fn test_parse_learn_response() {
        let json = r#"{
            "status": "learned",
            "updated_bullets": {
                "added": 3,
                "demoted": 1,
                "promoted": 2
            }
        }"#;

        let parsed: LearnResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.status, "learned");
        assert_eq!(parsed.updated_bullets.added, 3);
        assert_eq!(parsed.updated_bullets.demoted, 1);
        assert_eq!(parsed.updated_bullets.promoted, 2);
    }
}
