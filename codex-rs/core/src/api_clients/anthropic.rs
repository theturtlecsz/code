//! Native Anthropic API client (SPEC-KIT-953-F)
//!
//! Direct integration with Anthropic Messages API supporting:
//! - Streaming Server-Sent Events (SSE)
//! - Conversation history via ContextManager
//! - OAuth token retrieval via ProviderAuthManager

use std::path::PathBuf;

use futures::{Stream, StreamExt};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::context_manager::{Message, ProviderId, serialize_for_provider};
use crate::provider_auth::{ProviderAuthManager, TokenSource};

use super::{ApiError, ApiResult, StreamEvent, TokenUsage};

/// User-Agent for Claude Code compatibility.
/// Used when making requests with CLI credentials to match Claude Code's behavior.
const CLAUDE_CODE_USER_AGENT: &str = "Claude Code/2.0.47";

/// Anthropic API endpoint.
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Anthropic API version header.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Beta header required for OAuth authentication.
const ANTHROPIC_BETA: &str = "oauth-2025-04-20";

/// Configuration for the Anthropic client.
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// Model to use (e.g., "claude-sonnet-4-5-20250514").
    pub model: String,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Temperature for sampling (0.0-1.0).
    pub temperature: Option<f32>,
    /// System prompt (optional, can also be in messages).
    pub system: Option<String>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-5-20250514".to_string(),
            max_tokens: 8192,
            temperature: None,
            system: None,
        }
    }
}

/// Stream of events from Anthropic API.
pub struct AnthropicStream {
    rx: mpsc::Receiver<ApiResult<StreamEvent>>,
}

impl Stream for AnthropicStream {
    type Item = ApiResult<StreamEvent>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Anthropic error response format.
#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

/// Anthropic error wrapper.
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: AnthropicError,
}

/// Anthropic streaming event types.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SseEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStartData },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlockData,
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: DeltaData },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },

    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaData,
        usage: Option<UsageData>,
    },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "error")]
    Error { error: AnthropicError },
}

#[derive(Debug, Deserialize)]
struct MessageStartData {
    id: String,
    model: String,
    #[allow(dead_code)]
    usage: Option<UsageData>,
}

#[derive(Debug, Deserialize)]
struct ContentBlockData {
    #[serde(rename = "type")]
    block_type: String,
    #[allow(dead_code)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum DeltaData {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
struct MessageDeltaData {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageData {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    #[serde(default)]
    cache_creation_input_tokens: u32,
    #[serde(default)]
    cache_read_input_tokens: u32,
}

/// Native Anthropic API client.
///
/// Provides direct integration with the Anthropic Messages API,
/// supporting streaming responses and conversation history.
pub struct AnthropicClient {
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// Path to codex home for auth.
    codex_home: PathBuf,
}

impl AnthropicClient {
    /// Creates a new Anthropic client.
    ///
    /// # Arguments
    ///
    /// * `codex_home` - Path to codex home directory for credential storage
    pub fn new(codex_home: PathBuf) -> Self {
        let client = crate::default_client::create_client("codex_cli_rs");
        Self { client, codex_home }
    }

    /// Creates a client with a custom HTTP client.
    ///
    /// Useful for testing or custom configurations.
    pub fn with_client(client: reqwest::Client, codex_home: PathBuf) -> Self {
        Self { client, codex_home }
    }

    /// Sends a message and returns a streaming response.
    ///
    /// # Arguments
    ///
    /// * `messages` - Conversation history
    /// * `config` - Request configuration
    ///
    /// # Returns
    ///
    /// A stream of events from the API.
    pub async fn send_message(
        &self,
        messages: &[Message],
        config: &AnthropicConfig,
    ) -> ApiResult<AnthropicStream> {
        // Get access token with source information
        let auth_manager = ProviderAuthManager::new(self.codex_home.clone());
        let token_info = auth_manager
            .get_token_with_source(crate::provider_auth::ProviderId::Anthropic)
            .await
            .map_err(|_| ApiError::NotAuthenticated)?;

        tracing::error!(
            "Token source: {:?}, Model: {}",
            token_info.source,
            config.model
        );

        // Build request body
        let body = self.build_request_body(messages, config)?;

        // Build headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Use Bearer token authentication for OAuth
        let auth_value = format!("Bearer {}", token_info.token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .map_err(|_| ApiError::InvalidConfig("Invalid token".to_string()))?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        // Beta header required for OAuth authentication
        headers.insert("anthropic-beta", HeaderValue::from_static(ANTHROPIC_BETA));

        // If using CLI credentials, use Claude Code's User-Agent
        if token_info.source == TokenSource::ClaudeCli {
            tracing::error!("Setting Claude Code User-Agent for model: {}", config.model);
            headers.insert(
                reqwest::header::USER_AGENT,
                HeaderValue::from_static(CLAUDE_CODE_USER_AGENT),
            );
        } else {
            tracing::error!(
                "NOT using Claude Code User-Agent, source: {:?}",
                token_info.source
            );
        }

        // Make streaming request
        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        // Check for error response
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse as Anthropic error
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&error_text) {
                return Err(ApiError::ApiResponse {
                    status: status.as_u16(),
                    message: error_response.error.message,
                    error_type: Some(error_response.error.error_type),
                });
            }

            return Err(ApiError::ApiResponse {
                status: status.as_u16(),
                message: error_text,
                error_type: None,
            });
        }

        // Create channel for streaming events
        let (tx, rx) = mpsc::channel(100);

        // Spawn task to process SSE stream
        let stream = response.bytes_stream();
        tokio::spawn(async move {
            if let Err(e) = Self::process_stream(stream, tx.clone()).await {
                let _ = tx.send(Err(e)).await;
            }
        });

        Ok(AnthropicStream { rx })
    }

    /// Builds the request body for the Messages API.
    fn build_request_body(
        &self,
        messages: &[Message],
        config: &AnthropicConfig,
    ) -> ApiResult<Value> {
        // Convert messages to references
        let message_refs: Vec<&Message> = messages.iter().collect();

        // Serialize using context_manager serializer
        let serialized = serialize_for_provider(ProviderId::Anthropic, &message_refs);

        // Build final request
        let mut body = json!({
            "model": config.model,
            "max_tokens": config.max_tokens,
            "stream": true
        });

        // Add messages from serialization
        if let Some(msgs) = serialized.get("messages") {
            body["messages"] = msgs.clone();
        }

        // Add system prompt (from config or serialization)
        if let Some(ref system) = config.system {
            body["system"] = json!(system);
        } else if let Some(system) = serialized.get("system")
            && !system.is_null()
        {
            body["system"] = system.clone();
        }

        // Add temperature if specified
        if let Some(temp) = config.temperature {
            body["temperature"] = json!(temp);
        }

        Ok(body)
    }

    /// Processes the SSE stream and sends events to the channel.
    async fn process_stream(
        mut stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
        tx: mpsc::Sender<ApiResult<StreamEvent>>,
    ) -> ApiResult<()> {
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(ApiError::Network)?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE events
            while let Some(event) = Self::extract_sse_event(&mut buffer) {
                if let Some(stream_event) = Self::parse_sse_event(&event)?
                    && tx.send(Ok(stream_event)).await.is_err()
                {
                    // Receiver dropped, stop processing
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Extracts a complete SSE event from the buffer.
    fn extract_sse_event(buffer: &mut String) -> Option<String> {
        // SSE events are terminated by double newline
        if let Some(pos) = buffer.find("\n\n") {
            let event = buffer[..pos].to_string();
            *buffer = buffer[pos + 2..].to_string();
            Some(event)
        } else {
            None
        }
    }

    /// Parses an SSE event string into a StreamEvent.
    fn parse_sse_event(event_str: &str) -> ApiResult<Option<StreamEvent>> {
        let mut event_type = String::new();
        let mut data = String::new();

        for line in event_str.lines() {
            if let Some(value) = line.strip_prefix("event: ") {
                event_type = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("data: ") {
                data = value.to_string();
            }
        }

        // Skip empty events
        if event_type.is_empty() || data.is_empty() {
            return Ok(None);
        }

        // Parse JSON data
        let parsed: SseEvent = serde_json::from_str(&data)
            .map_err(|e| ApiError::Parse(format!("Failed to parse SSE event: {e}")))?;

        let stream_event = match parsed {
            SseEvent::MessageStart { message } => StreamEvent::MessageStart {
                id: message.id,
                model: message.model,
            },
            SseEvent::ContentBlockStart {
                index,
                content_block,
            } => StreamEvent::ContentBlockStart {
                index,
                block_type: content_block.block_type,
            },
            SseEvent::ContentBlockDelta { index, delta } => {
                let text = match delta {
                    DeltaData::TextDelta { text } => text,
                    DeltaData::InputJsonDelta { partial_json } => partial_json,
                };
                StreamEvent::TextDelta { index, text }
            }
            SseEvent::ContentBlockStop { index } => StreamEvent::ContentBlockStop { index },
            SseEvent::MessageDelta { delta, usage } => StreamEvent::MessageDelta {
                stop_reason: delta.stop_reason,
                usage: usage.map(|u| TokenUsage {
                    input_tokens: u.input_tokens.unwrap_or(0),
                    output_tokens: u.output_tokens.unwrap_or(0),
                    cache_creation_input_tokens: u.cache_creation_input_tokens,
                    cache_read_input_tokens: u.cache_read_input_tokens,
                }),
            },
            SseEvent::MessageStop => StreamEvent::MessageStop,
            SseEvent::Ping => StreamEvent::Ping,
            SseEvent::Error { error } => {
                return Err(ApiError::ApiResponse {
                    status: 500,
                    message: error.message,
                    error_type: Some(error.error_type),
                });
            }
        };

        Ok(Some(stream_event))
    }

    /// Checks if the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        let auth_manager = ProviderAuthManager::new(self.codex_home.clone());
        auth_manager
            .is_authenticated(crate::provider_auth::ProviderId::Anthropic)
            .unwrap_or(false)
    }
}

/// Convenience function to collect all text from a stream.
///
/// Useful for non-streaming use cases or testing.
#[allow(dead_code)]
pub async fn collect_text(mut stream: AnthropicStream) -> ApiResult<String> {
    let mut text = String::new();

    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::TextDelta { text: delta, .. } => {
                text.push_str(&delta);
            }
            StreamEvent::MessageStop => break,
            _ => {}
        }
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AnthropicConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-5-20250514");
        assert_eq!(config.max_tokens, 8192);
        assert!(config.temperature.is_none());
    }

    #[test]
    fn test_extract_sse_event() {
        let mut buffer =
            "event: message_start\ndata: {\"type\":\"message_start\"}\n\nremaining".to_string();
        let event = AnthropicClient::extract_sse_event(&mut buffer);
        assert!(event.is_some());
        assert_eq!(
            event.unwrap(),
            "event: message_start\ndata: {\"type\":\"message_start\"}"
        );
        assert_eq!(buffer, "remaining");
    }

    #[test]
    fn test_extract_sse_event_incomplete() {
        let mut buffer = "event: message_start\ndata: {\"type\":\"message_start\"}".to_string();
        let event = AnthropicClient::extract_sse_event(&mut buffer);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_message_start() {
        let event = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_123\",\"model\":\"claude-3-sonnet\"}}";
        let result = AnthropicClient::parse_sse_event(event).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            StreamEvent::MessageStart { id, model } => {
                assert_eq!(id, "msg_123");
                assert_eq!(model, "claude-3-sonnet");
            }
            _ => panic!("Expected MessageStart event"),
        }
    }

    #[test]
    fn test_parse_text_delta() {
        let event = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}";
        let result = AnthropicClient::parse_sse_event(event).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            StreamEvent::TextDelta { index, text } => {
                assert_eq!(index, 0);
                assert_eq!(text, "Hello");
            }
            _ => panic!("Expected TextDelta event"),
        }
    }

    #[test]
    fn test_parse_content_block_start() {
        let event = "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\"}}";
        let result = AnthropicClient::parse_sse_event(event).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            StreamEvent::ContentBlockStart { index, block_type } => {
                assert_eq!(index, 0);
                assert_eq!(block_type, "text");
            }
            _ => panic!("Expected ContentBlockStart event"),
        }
    }

    #[test]
    fn test_parse_message_delta() {
        let event = "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":50}}";
        let result = AnthropicClient::parse_sse_event(event).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            StreamEvent::MessageDelta { stop_reason, usage } => {
                assert_eq!(stop_reason, Some("end_turn".to_string()));
                assert!(usage.is_some());
                assert_eq!(usage.unwrap().output_tokens, 50);
            }
            _ => panic!("Expected MessageDelta event"),
        }
    }

    #[test]
    fn test_parse_message_stop() {
        let event = "event: message_stop\ndata: {\"type\":\"message_stop\"}";
        let result = AnthropicClient::parse_sse_event(event).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            StreamEvent::MessageStop => {}
            _ => panic!("Expected MessageStop event"),
        }
    }

    #[test]
    fn test_parse_ping() {
        let event = "event: ping\ndata: {\"type\":\"ping\"}";
        let result = AnthropicClient::parse_sse_event(event).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            StreamEvent::Ping => {}
            _ => panic!("Expected Ping event"),
        }
    }

    #[test]
    fn test_build_request_body() {
        let client = AnthropicClient::new(PathBuf::from("/tmp"));
        let messages = vec![Message::system("You are helpful"), Message::user("Hello")];
        let config = AnthropicConfig::default();

        let body = client.build_request_body(&messages, &config).unwrap();

        assert_eq!(body["model"], "claude-sonnet-4-5-20250514");
        assert_eq!(body["max_tokens"], 8192);
        assert_eq!(body["stream"], true);
        assert!(body["messages"].is_array());
    }

    #[test]
    fn test_build_request_body_with_temperature() {
        let client = AnthropicClient::new(PathBuf::from("/tmp"));
        let messages = vec![Message::user("Test")];
        let config = AnthropicConfig {
            temperature: Some(0.7),
            ..Default::default()
        };

        let body = client.build_request_body(&messages, &config).unwrap();
        // Use approximate comparison for f32
        let temp = body["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_build_request_body_with_custom_system() {
        let client = AnthropicClient::new(PathBuf::from("/tmp"));
        let messages = vec![Message::user("Test")];
        let config = AnthropicConfig {
            system: Some("Custom system".to_string()),
            ..Default::default()
        };

        let body = client.build_request_body(&messages, &config).unwrap();
        assert_eq!(body["system"], "Custom system");
    }

    #[test]
    fn test_api_error_display() {
        let err = ApiError::ApiResponse {
            status: 400,
            message: "Invalid request".to_string(),
            error_type: Some("invalid_request_error".to_string()),
        };
        let display = format!("{err}");
        assert!(display.contains("400"));
        assert!(display.contains("Invalid request"));
    }
}
