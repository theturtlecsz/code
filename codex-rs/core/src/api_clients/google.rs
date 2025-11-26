//! Native Google Generative AI client (SPEC-KIT-953-G)
//!
//! Direct integration with Google Generative AI API supporting:
//! - Streaming newline-delimited JSON responses
//! - Conversation history via ContextManager
//! - OAuth token retrieval via ProviderAuthManager

use std::path::PathBuf;

use futures::{Stream, StreamExt};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::context_manager::{Message, ProviderId, serialize_for_provider};
use crate::provider_auth::ProviderAuthManager;

use super::{ApiError, ApiResult, StreamEvent, TokenUsage};

/// Google Generative AI API base URL.
const GOOGLE_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Configuration for the Gemini client.
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    /// Model to use (e.g., "gemini-2.5-flash", "gemini-2.5-pro").
    pub model: String,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Temperature for sampling (0.0-2.0).
    pub temperature: Option<f32>,
    /// Top-p sampling parameter.
    pub top_p: Option<f32>,
    /// System prompt (optional).
    pub system: Option<String>,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            model: "gemini-2.5-flash".to_string(),
            max_tokens: 8192,
            temperature: None,
            top_p: None,
            system: None,
        }
    }
}

/// Stream of events from Google Generative AI API.
pub struct GeminiStream {
    rx: mpsc::Receiver<ApiResult<StreamEvent>>,
}

impl Stream for GeminiStream {
    type Item = ApiResult<StreamEvent>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Google error response format.
#[derive(Debug, Deserialize)]
struct GoogleError {
    code: Option<u16>,
    message: String,
    status: Option<String>,
}

/// Google error wrapper.
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: GoogleError,
}

/// Streaming response chunk from Google API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamChunk {
    candidates: Option<Vec<Candidate>>,
    usage_metadata: Option<UsageMetadata>,
    #[serde(default)]
    prompt_feedback: Option<PromptFeedback>,
}

/// Candidate response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Option<CandidateContent>,
    finish_reason: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    safety_ratings: Vec<SafetyRating>,
}

/// Candidate content.
#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<Part>,
    #[allow(dead_code)]
    role: Option<String>,
}

/// Content part.
#[derive(Debug, Deserialize)]
struct Part {
    text: Option<String>,
}

/// Usage metadata from response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    #[allow(dead_code)]
    total_token_count: Option<u32>,
}

/// Safety rating.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SafetyRating {
    category: String,
    probability: String,
}

/// Prompt feedback (for blocked prompts).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PromptFeedback {
    block_reason: Option<String>,
}

/// Native Google Generative AI client.
///
/// Provides direct integration with the Google Generative AI API,
/// supporting streaming responses and conversation history.
pub struct GeminiClient {
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// Path to codex home for auth.
    codex_home: PathBuf,
}

impl GeminiClient {
    /// Creates a new Gemini client.
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
        config: &GeminiConfig,
    ) -> ApiResult<GeminiStream> {
        // Get access token
        let auth_manager = ProviderAuthManager::new(self.codex_home.clone());
        let token = auth_manager
            .get_token(crate::provider_auth::ProviderId::Google)
            .await
            .map_err(|_| ApiError::NotAuthenticated)?;

        // Build request body
        let body = self.build_request_body(messages, config)?;

        // Build headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| ApiError::InvalidConfig("Invalid token".to_string()))?,
        );

        // Build URL with streaming endpoint
        let url = format!(
            "{}/{}:streamGenerateContent?alt=sse",
            GOOGLE_API_BASE, config.model
        );

        // Make streaming request
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        // Check for error response
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse as Google error
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&error_text) {
                return Err(ApiError::ApiResponse {
                    status: error_response.error.code.unwrap_or(status.as_u16()),
                    message: error_response.error.message,
                    error_type: error_response.error.status,
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

        // Spawn task to process stream
        let stream = response.bytes_stream();
        let model = config.model.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::process_stream(stream, tx.clone(), model).await {
                let _ = tx.send(Err(e)).await;
            }
        });

        Ok(GeminiStream { rx })
    }

    /// Builds the request body for the Generative AI API.
    fn build_request_body(&self, messages: &[Message], config: &GeminiConfig) -> ApiResult<Value> {
        // Convert messages to references
        let message_refs: Vec<&Message> = messages.iter().collect();

        // Serialize using context_manager serializer
        let serialized = serialize_for_provider(ProviderId::Google, &message_refs);

        // Build final request
        let mut body = json!({});

        // Add contents from serialization
        if let Some(contents) = serialized.get("contents") {
            body["contents"] = contents.clone();
        }

        // Add system instruction (from config or serialization)
        if let Some(ref system) = config.system {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system }]
            });
        } else if let Some(sys) = serialized.get("system_instruction")
            && !sys.is_null() {
                body["systemInstruction"] = sys.clone();
            }

        // Add generation config
        let mut gen_config = json!({
            "maxOutputTokens": config.max_tokens
        });

        if let Some(temp) = config.temperature {
            gen_config["temperature"] = json!(temp);
        }

        if let Some(top_p) = config.top_p {
            gen_config["topP"] = json!(top_p);
        }

        body["generationConfig"] = gen_config;

        Ok(body)
    }

    /// Processes the streaming response and sends events to the channel.
    async fn process_stream(
        mut stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
        tx: mpsc::Sender<ApiResult<StreamEvent>>,
        model: String,
    ) -> ApiResult<()> {
        let mut buffer = String::new();
        let mut started = false;
        let content_index: u32 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(ApiError::Network)?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines (Google uses SSE format with data: prefix)
            while let Some((line, rest)) = Self::extract_line(&buffer) {
                buffer = rest;

                // Skip empty lines
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Handle SSE format - extract data
                let json_str = if let Some(data) = line.strip_prefix("data: ") {
                    data.trim()
                } else {
                    // Not SSE format, try as raw JSON
                    line
                };

                // Skip empty data or opening brackets
                if json_str.is_empty() || json_str == "[" || json_str == "]" || json_str == "," {
                    continue;
                }

                // Parse JSON chunk
                let parsed: StreamChunk = match serde_json::from_str(json_str) {
                    Ok(p) => p,
                    Err(e) => {
                        // Log parse error but continue - might be partial data
                        tracing::debug!("Failed to parse chunk: {} - data: {}", e, json_str);
                        continue;
                    }
                };

                // Handle prompt blocked
                if let Some(feedback) = &parsed.prompt_feedback
                    && let Some(reason) = &feedback.block_reason {
                        return Err(ApiError::ApiResponse {
                            status: 400,
                            message: format!("Prompt blocked: {reason}"),
                            error_type: Some("PROMPT_BLOCKED".to_string()),
                        });
                    }

                // Send message start on first chunk
                if !started {
                    started = true;
                    let _ = tx
                        .send(Ok(StreamEvent::MessageStart {
                            id: format!("gemini-{}", uuid::Uuid::new_v4()),
                            model: model.clone(),
                        }))
                        .await;

                    let _ = tx
                        .send(Ok(StreamEvent::ContentBlockStart {
                            index: content_index,
                            block_type: "text".to_string(),
                        }))
                        .await;
                }

                // Extract text from candidates
                if let Some(candidates) = &parsed.candidates {
                    for candidate in candidates {
                        // Check for safety block
                        if let Some(reason) = &candidate.finish_reason
                            && reason == "SAFETY" {
                                return Err(ApiError::ApiResponse {
                                    status: 400,
                                    message: "Response blocked due to safety concerns".to_string(),
                                    error_type: Some("SAFETY_BLOCK".to_string()),
                                });
                            }

                        if let Some(content) = &candidate.content {
                            for part in &content.parts {
                                if let Some(text) = &part.text
                                    && !text.is_empty()
                                        && tx
                                            .send(Ok(StreamEvent::TextDelta {
                                                index: content_index,
                                                text: text.clone(),
                                            }))
                                            .await
                                            .is_err()
                                        {
                                            return Ok(());
                                        }
                            }
                        }

                        // Check for completion
                        if let Some(reason) = &candidate.finish_reason
                            && (reason == "STOP" || reason == "MAX_TOKENS") {
                                // Send content block stop
                                let _ = tx
                                    .send(Ok(StreamEvent::ContentBlockStop {
                                        index: content_index,
                                    }))
                                    .await;

                                // Send message delta with stop reason and usage
                                let usage = parsed.usage_metadata.as_ref().map(|u| TokenUsage {
                                    input_tokens: u.prompt_token_count.unwrap_or(0),
                                    output_tokens: u.candidates_token_count.unwrap_or(0),
                                    cache_creation_input_tokens: 0,
                                    cache_read_input_tokens: 0,
                                });

                                let _ = tx
                                    .send(Ok(StreamEvent::MessageDelta {
                                        stop_reason: Some(reason.to_lowercase()),
                                        usage,
                                    }))
                                    .await;

                                // Send message stop
                                let _ = tx.send(Ok(StreamEvent::MessageStop)).await;
                            }
                    }
                }
            }
        }

        // If we never started, send minimal events
        if !started {
            let _ = tx
                .send(Ok(StreamEvent::MessageStart {
                    id: format!("gemini-{}", uuid::Uuid::new_v4()),
                    model,
                }))
                .await;
        }

        Ok(())
    }

    /// Extracts a complete line from the buffer.
    fn extract_line(buffer: &str) -> Option<(String, String)> {
        if let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].to_string();
            let rest = buffer[pos + 1..].to_string();
            Some((line, rest))
        } else {
            None
        }
    }

    /// Checks if the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        let auth_manager = ProviderAuthManager::new(self.codex_home.clone());
        auth_manager
            .is_authenticated(crate::provider_auth::ProviderId::Google)
            .unwrap_or(false)
    }
}

/// Convenience function to collect all text from a stream.
///
/// Useful for non-streaming use cases or testing.
#[allow(dead_code)]
pub async fn collect_text(mut stream: GeminiStream) -> ApiResult<String> {
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

/// Map model preset names to actual Google model IDs.
pub fn map_model_name(preset: &str) -> &str {
    let preset_lower = preset.to_ascii_lowercase();

    if preset_lower.contains("3-pro") || preset_lower.contains("3.0-pro") {
        "gemini-3.0-pro"
    } else if preset_lower.contains("2.5-pro") {
        "gemini-2.5-pro"
    } else if preset_lower.contains("2.5-flash") {
        "gemini-2.5-flash"
    } else if preset_lower.contains("2.0-flash") || preset_lower == "flash" {
        "gemini-2.0-flash"
    } else {
        preset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GeminiConfig::default();
        assert_eq!(config.model, "gemini-2.5-flash");
        assert_eq!(config.max_tokens, 8192);
        assert!(config.temperature.is_none());
        assert!(config.top_p.is_none());
    }

    #[test]
    fn test_extract_line() {
        let buffer = "data: {\"test\": 1}\nremaining";
        let result = GeminiClient::extract_line(buffer);
        assert!(result.is_some());
        let (line, rest) = result.unwrap();
        assert_eq!(line, "data: {\"test\": 1}");
        assert_eq!(rest, "remaining");
    }

    #[test]
    fn test_extract_line_incomplete() {
        let buffer = "data: {\"test\": 1}";
        let result = GeminiClient::extract_line(buffer);
        assert!(result.is_none());
    }

    #[test]
    fn test_map_model_name() {
        assert_eq!(map_model_name("gemini-3-pro"), "gemini-3.0-pro");
        assert_eq!(map_model_name("gemini-2.5-pro"), "gemini-2.5-pro");
        assert_eq!(map_model_name("gemini-2.5-flash"), "gemini-2.5-flash");
        assert_eq!(map_model_name("gemini-2.0-flash"), "gemini-2.0-flash");
        assert_eq!(map_model_name("flash"), "gemini-2.0-flash");
        assert_eq!(map_model_name("custom-model"), "custom-model");
    }

    #[test]
    fn test_parse_stream_chunk() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":5}}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();

        assert!(chunk.candidates.is_some());
        let candidates = chunk.candidates.unwrap();
        assert_eq!(candidates.len(), 1);

        let candidate = &candidates[0];
        assert_eq!(candidate.finish_reason, Some("STOP".to_string()));

        let content = candidate.content.as_ref().unwrap();
        assert_eq!(content.parts[0].text, Some("Hello".to_string()));

        let usage = chunk.usage_metadata.unwrap();
        assert_eq!(usage.prompt_token_count, Some(10));
        assert_eq!(usage.candidates_token_count, Some(5));
    }

    #[test]
    fn test_parse_stream_chunk_partial() {
        // Partial response during streaming
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"Partial"}],"role":"model"}}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();

        assert!(chunk.candidates.is_some());
        let candidates = chunk.candidates.unwrap();
        let content = candidates[0].content.as_ref().unwrap();
        assert_eq!(content.parts[0].text, Some("Partial".to_string()));
        assert!(candidates[0].finish_reason.is_none());
    }

    #[test]
    fn test_parse_error_response() {
        let json =
            r#"{"error":{"code":400,"message":"Invalid request","status":"INVALID_ARGUMENT"}}"#;
        let error: ErrorResponse = serde_json::from_str(json).unwrap();

        assert_eq!(error.error.code, Some(400));
        assert_eq!(error.error.message, "Invalid request");
        assert_eq!(error.error.status, Some("INVALID_ARGUMENT".to_string()));
    }

    #[test]
    fn test_build_request_body() {
        let client = GeminiClient::new(PathBuf::from("/tmp"));
        let messages = vec![Message::system("You are helpful"), Message::user("Hello")];
        let config = GeminiConfig::default();

        let body = client.build_request_body(&messages, &config).unwrap();

        assert!(body["contents"].is_array());
        assert!(body["generationConfig"]["maxOutputTokens"].is_number());
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 8192);
    }

    #[test]
    fn test_build_request_body_with_temperature() {
        let client = GeminiClient::new(PathBuf::from("/tmp"));
        let messages = vec![Message::user("Test")];
        let config = GeminiConfig {
            temperature: Some(0.7),
            ..Default::default()
        };

        let body = client.build_request_body(&messages, &config).unwrap();
        let temp = body["generationConfig"]["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_build_request_body_with_custom_system() {
        let client = GeminiClient::new(PathBuf::from("/tmp"));
        let messages = vec![Message::user("Test")];
        let config = GeminiConfig {
            system: Some("Custom system".to_string()),
            ..Default::default()
        };

        let body = client.build_request_body(&messages, &config).unwrap();
        let sys = &body["systemInstruction"]["parts"][0]["text"];
        assert_eq!(sys, "Custom system");
    }

    #[test]
    fn test_build_request_body_with_top_p() {
        let client = GeminiClient::new(PathBuf::from("/tmp"));
        let messages = vec![Message::user("Test")];
        let config = GeminiConfig {
            top_p: Some(0.95),
            ..Default::default()
        };

        let body = client.build_request_body(&messages, &config).unwrap();
        let top_p = body["generationConfig"]["topP"].as_f64().unwrap();
        assert!((top_p - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_api_error_display() {
        let err = ApiError::ApiResponse {
            status: 400,
            message: "Invalid request".to_string(),
            error_type: Some("INVALID_ARGUMENT".to_string()),
        };
        let display = format!("{}", err);
        assert!(display.contains("400"));
        assert!(display.contains("Invalid request"));
    }

    #[test]
    fn test_usage_metadata_parsing() {
        let json = r#"{"promptTokenCount":100,"candidatesTokenCount":50,"totalTokenCount":150}"#;
        let usage: UsageMetadata = serde_json::from_str(json).unwrap();

        assert_eq!(usage.prompt_token_count, Some(100));
        assert_eq!(usage.candidates_token_count, Some(50));
        assert_eq!(usage.total_token_count, Some(150));
    }

    #[test]
    fn test_safety_rating_parsing() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"test"}],"role":"model"},"safetyRatings":[{"category":"HARM_CATEGORY_HATE_SPEECH","probability":"NEGLIGIBLE"}]}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();

        let candidates = chunk.candidates.unwrap();
        assert!(!candidates[0].safety_ratings.is_empty());
        assert_eq!(
            candidates[0].safety_ratings[0].category,
            "HARM_CATEGORY_HATE_SPEECH"
        );
    }
}
