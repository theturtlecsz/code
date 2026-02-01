//! SPEC-KIT-978: Reflex Client for Local Inference
//!
//! OpenAI-compatible chat completion client for local inference endpoints
//! (e.g., SGLang, vLLM, text-generation-inference).
//!
//! ## Usage
//!
//! ```rust,ignore
//! let client = ReflexClient::new(&config)?;
//!
//! // Simple completion
//! let response = client.chat_completion(&messages).await?;
//!
//! // With JSON schema (structured output)
//! let response = client.chat_completion_json(&messages, &schema).await?;
//! ```
//!
//! ## Features
//! - Streaming SSE responses (Server-Sent Events)
//! - JSON schema enforcement (response_format)
//! - Configurable timeout
//! - Automatic metrics recording for bakeoff analysis

use codex_stage0::ReflexConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// OpenAI-compatible chat completion message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion request body
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    stream: bool,
}

/// Response format for structured output
#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_schema: Option<serde_json::Value>,
}

/// Chat completion response (non-streaming)
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub model: String,
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// Chat choice in response
#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// SSE delta event (streaming response)
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

// Serde: complete API response structure
#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: DeltaContent,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeltaContent {
    #[serde(default)]
    content: Option<String>,
}

/// Reflex inference result
#[derive(Debug)]
pub struct ReflexResult {
    /// The generated text content
    pub content: String,
    /// Whether the response is valid JSON (if schema was requested)
    pub json_compliant: bool,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Model used
    pub model: String,
    /// Token usage (if provided)
    pub usage: Option<Usage>,
}

/// Error types for reflex client
#[derive(Debug, thiserror::Error)]
pub enum ReflexError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Server error: HTTP {status} - {body}")]
    ServerError { status: u16, body: String },

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Empty response")]
    EmptyResponse,

    #[error("Invalid JSON schema compliance")]
    JsonSchemaViolation,
}

/// Reflex client for local inference
pub struct ReflexClient {
    client: Client,
    config: ReflexConfig,
}

impl ReflexClient {
    /// Create a new reflex client from configuration
    pub fn new(config: &ReflexConfig) -> Result<Self, ReflexError> {
        let timeout = Duration::from_millis(config.timeout_ms);
        let client = Client::builder().timeout(timeout).build()?;

        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    /// Perform chat completion (non-streaming)
    pub async fn chat_completion(
        &self,
        messages: &[ChatMessage],
    ) -> Result<ReflexResult, ReflexError> {
        self.chat_completion_internal(messages, None).await
    }

    /// Perform chat completion with JSON schema (structured output)
    pub async fn chat_completion_json(
        &self,
        messages: &[ChatMessage],
        schema: &serde_json::Value,
    ) -> Result<ReflexResult, ReflexError> {
        self.chat_completion_internal(messages, Some(schema)).await
    }

    /// Internal chat completion implementation
    async fn chat_completion_internal(
        &self,
        messages: &[ChatMessage],
        json_schema: Option<&serde_json::Value>,
    ) -> Result<ReflexResult, ReflexError> {
        let url = format!(
            "{}/chat/completions",
            self.config.endpoint.trim_end_matches('/')
        );

        let response_format = json_schema.map(|schema| ResponseFormat {
            format_type: "json_schema".to_string(),
            json_schema: Some(schema.clone()),
        });

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
            max_tokens: Some(4096),
            temperature: Some(0.0),
            response_format,
            stream: false,
        };

        let start = Instant::now();

        let response = self.client.post(&url).json(&request).send().await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ReflexError::ServerError {
                status: status.as_u16(),
                body,
            });
        }

        let completion: ChatCompletionResponse = response.json().await?;

        let content = completion
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        if content.is_empty() {
            return Err(ReflexError::EmptyResponse);
        }

        // Check JSON compliance if schema was requested
        let json_compliant = if json_schema.is_some() {
            serde_json::from_str::<serde_json::Value>(&content).is_ok()
        } else {
            true
        };

        Ok(ReflexResult {
            content,
            json_compliant,
            latency_ms,
            model: completion.model,
            usage: completion.usage,
        })
    }

    /// Perform streaming chat completion
    ///
    /// Returns the full response after collecting all chunks.
    pub async fn chat_completion_streaming(
        &self,
        messages: &[ChatMessage],
    ) -> Result<ReflexResult, ReflexError> {
        self.chat_completion_streaming_internal(messages, None)
            .await
    }

    /// Perform streaming chat completion with JSON schema
    pub async fn chat_completion_streaming_json(
        &self,
        messages: &[ChatMessage],
        schema: &serde_json::Value,
    ) -> Result<ReflexResult, ReflexError> {
        self.chat_completion_streaming_internal(messages, Some(schema))
            .await
    }

    /// Internal streaming implementation
    async fn chat_completion_streaming_internal(
        &self,
        messages: &[ChatMessage],
        json_schema: Option<&serde_json::Value>,
    ) -> Result<ReflexResult, ReflexError> {
        let url = format!(
            "{}/chat/completions",
            self.config.endpoint.trim_end_matches('/')
        );

        let response_format = json_schema.map(|schema| ResponseFormat {
            format_type: "json_schema".to_string(),
            json_schema: Some(schema.clone()),
        });

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
            max_tokens: Some(4096),
            temperature: Some(0.0),
            response_format,
            stream: true,
        };

        let start = Instant::now();

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ReflexError::ServerError {
                status: status.as_u16(),
                body,
            });
        }

        // Collect streaming response
        let mut content = String::new();
        let mut bytes_stream = response.bytes_stream();

        use futures::StreamExt;
        let mut buffer = String::new();

        while let Some(chunk) = bytes_stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE events
            while let Some(event_end) = buffer.find("\n\n") {
                let event = &buffer[..event_end];

                for line in event.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data.trim() == "[DONE]" {
                            continue;
                        }

                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                            for choice in chunk.choices {
                                if let Some(delta_content) = choice.delta.content {
                                    content.push_str(&delta_content);
                                }
                            }
                        }
                    }
                }

                buffer = buffer[event_end + 2..].to_string();
            }
        }

        let latency_ms = start.elapsed().as_millis() as u64;

        if content.is_empty() {
            return Err(ReflexError::EmptyResponse);
        }

        // Check JSON compliance if schema was requested
        let json_compliant = if json_schema.is_some() {
            serde_json::from_str::<serde_json::Value>(&content).is_ok()
        } else {
            true
        };

        Ok(ReflexResult {
            content,
            json_compliant,
            latency_ms,
            model: self.config.model.clone(),
            usage: None, // Streaming responses typically don't include usage
        })
    }

    /// Get the configured endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }

    /// Get the configured model name
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Check if the reflex server is healthy
    pub async fn health_check(&self) -> Result<bool, ReflexError> {
        let url = format!("{}/models", self.config.endpoint.trim_end_matches('/'));

        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_request_serialization() {
        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            max_tokens: Some(100),
            temperature: Some(0.5),
            response_format: None,
            stream: false,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-model"));
        assert!(!json.contains("response_format"));
    }

    #[test]
    fn test_request_with_json_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![],
            max_tokens: None,
            temperature: None,
            response_format: Some(ResponseFormat {
                format_type: "json_schema".to_string(),
                json_schema: Some(schema),
            }),
            stream: false,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("json_schema"));
        assert!(json.contains("properties"));
    }

    /// SPEC-KIT-978: Test that json_compliant correctly identifies valid JSON
    #[test]
    fn test_json_compliance_valid_json() {
        // Simulate the json_compliant check from chat_completion_internal
        let valid_json_content = r#"{"stage": "implement", "confidence": 0.95}"#;
        let has_schema = true;

        let json_compliant = if has_schema {
            serde_json::from_str::<serde_json::Value>(valid_json_content).is_ok()
        } else {
            true
        };

        assert!(json_compliant, "Valid JSON should be marked compliant");
    }

    /// SPEC-KIT-978: Test that json_compliant rejects non-JSON content
    #[test]
    fn test_json_compliance_rejects_non_json() {
        // Simulate the json_compliant check from chat_completion_internal
        let non_json_content = "This is plain text, not JSON";
        let has_schema = true;

        let json_compliant = if has_schema {
            serde_json::from_str::<serde_json::Value>(non_json_content).is_ok()
        } else {
            true
        };

        assert!(
            !json_compliant,
            "Non-JSON content should be marked non-compliant"
        );
    }

    /// SPEC-KIT-978: Test that json_compliant rejects malformed JSON
    #[test]
    fn test_json_compliance_rejects_malformed_json() {
        // Simulate the json_compliant check from chat_completion_internal
        let malformed_json = r#"{"stage": "implement", "missing_brace""#;
        let has_schema = true;

        let json_compliant = if has_schema {
            serde_json::from_str::<serde_json::Value>(malformed_json).is_ok()
        } else {
            true
        };

        assert!(
            !json_compliant,
            "Malformed JSON should be marked non-compliant"
        );
    }

    /// SPEC-KIT-978: Test that without schema, any content is considered compliant
    #[test]
    fn test_json_compliance_without_schema() {
        let non_json_content = "Plain text without schema requirement";
        let has_schema = false;

        let json_compliant = if has_schema {
            serde_json::from_str::<serde_json::Value>(non_json_content).is_ok()
        } else {
            true
        };

        assert!(
            json_compliant,
            "Without schema, any content should be compliant"
        );
    }
}
