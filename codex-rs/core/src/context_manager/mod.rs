//! Context Manager for Multi-Provider Conversation Management (SPEC-KIT-953-E)
//!
//! Provides provider-agnostic conversation history with:
//! - Abstract message types that work across providers
//! - Token counting per provider
//! - Truncation strategies
//! - Session persistence
//! - Provider-specific serialization

mod history;
mod manager;
mod persistence;
mod serializer;
mod tokenizer;

pub use history::{ConversationHistory, TokenBudget, TruncationStrategy};
pub use manager::ContextManager;
pub use persistence::{SessionManager, SessionState};
pub use serializer::serialize_for_provider;
pub use tokenizer::{count_tokens, get_context_window};

use serde::{Deserialize, Serialize};

/// Provider identifier for routing and storage
///
/// This enum matches `ProviderId` in SPEC-953-D but is defined here
/// to avoid circular dependencies. The types should be kept in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    /// OpenAI (ChatGPT, GPT-4, etc.)
    OpenAI,
    /// Anthropic (Claude)
    Anthropic,
    /// Google (Gemini)
    Google,
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderId::OpenAI => write!(f, "OpenAI"),
            ProviderId::Anthropic => write!(f, "Anthropic"),
            ProviderId::Google => write!(f, "Google"),
        }
    }
}

/// Message role - canonical representation across providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System prompt (instructions to the model)
    System,
    /// User input
    User,
    /// Model response
    Assistant,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

/// Content block - canonical representation across providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content
    Text { text: String },

    /// Image content (base64 data URL or URL)
    Image {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },

    /// Tool use request (from assistant)
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// Tool result (from user, in response to tool use)
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

impl ContentBlock {
    /// Create a text content block
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an image content block from URL
    pub fn image(url: impl Into<String>) -> Self {
        Self::Image {
            url: url.into(),
            media_type: None,
        }
    }

    /// Create an image content block with media type
    pub fn image_with_type(url: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self::Image {
            url: url.into(),
            media_type: Some(media_type.into()),
        }
    }

    /// Create a tool use content block
    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Create a tool result content block
    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: false,
        }
    }

    /// Create a tool error result content block
    pub fn tool_error(tool_use_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: error.into(),
            is_error: true,
        }
    }

    /// Get text content if this is a text block
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// Message role
    pub role: MessageRole,

    /// Content blocks
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a new message
    pub fn new(role: MessageRole, content: Vec<ContentBlock>) -> Self {
        Self { role, content }
    }

    /// Create a text message
    pub fn text(role: MessageRole, text: impl Into<String>) -> Self {
        Self::new(role, vec![ContentBlock::text(text)])
    }

    /// Create a system message
    pub fn system(text: impl Into<String>) -> Self {
        Self::text(MessageRole::System, text)
    }

    /// Create a user message
    pub fn user(text: impl Into<String>) -> Self {
        Self::text(MessageRole::User, text)
    }

    /// Create an assistant message
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::text(MessageRole::Assistant, text)
    }

    /// Get the first text content from this message
    pub fn first_text(&self) -> Option<&str> {
        self.content.iter().find_map(|c| c.as_text())
    }

    /// Get all text content concatenated
    pub fn all_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if this message contains any images
    pub fn has_images(&self) -> bool {
        self.content
            .iter()
            .any(|c| matches!(c, ContentBlock::Image { .. }))
    }

    /// Check if this message contains any tool use
    pub fn has_tool_use(&self) -> bool {
        self.content
            .iter()
            .any(|c| matches!(c, ContentBlock::ToolUse { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.first_text(), Some("Hello"));
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::text("test");
        assert_eq!(block.as_text(), Some("test"));
    }

    #[test]
    fn test_content_block_image() {
        let block = ContentBlock::image("https://example.com/image.png");
        assert!(
            matches!(block, ContentBlock::Image { url, media_type: None } if url == "https://example.com/image.png")
        );
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn test_provider_id_serialization() {
        let provider = ProviderId::Anthropic;
        let json = serde_json::to_string(&provider).unwrap();
        assert_eq!(json, "\"anthropic\"");

        let parsed: ProviderId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ProviderId::Anthropic);
    }

    #[test]
    fn test_message_all_text() {
        let msg = Message::new(
            MessageRole::User,
            vec![
                ContentBlock::text("Hello"),
                ContentBlock::image("img.png"),
                ContentBlock::text("World"),
            ],
        );
        assert_eq!(msg.all_text(), "Hello\nWorld");
    }

    #[test]
    fn test_message_has_images() {
        let msg_with_image = Message::new(
            MessageRole::User,
            vec![
                ContentBlock::text("Look at this"),
                ContentBlock::image("img.png"),
            ],
        );
        assert!(msg_with_image.has_images());

        let msg_without_image = Message::user("Just text");
        assert!(!msg_without_image.has_images());
    }
}
