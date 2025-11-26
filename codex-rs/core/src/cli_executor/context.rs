use super::types::{Conversation, Message, Role};

/// Maximum context size as percentage of model limit (safety margin)
const CONTEXT_SAFETY_MARGIN: f32 = 0.9;

/// Claude model context limits (tokens)
const CLAUDE_OPUS_LIMIT: usize = 200_000;
const CLAUDE_SONNET_LIMIT: usize = 200_000;
const CLAUDE_HAIKU_LIMIT: usize = 200_000;

/// Gemini model context limits (tokens)
const GEMINI_PRO_LIMIT: usize = 1_000_000;
const GEMINI_FLASH_LIMIT: usize = 1_000_000;

/// Context manager for formatting conversation history
pub struct CliContextManager;

impl CliContextManager {
    /// Format conversation history into CLI-friendly prompt
    ///
    /// Embeds prior messages with clear delimiters and timestamps:
    /// ```
    /// SYSTEM: You are a helpful coding assistant.
    ///
    /// --- Previous Conversation ---
    /// USER (2025-11-20 19:30):
    /// What's the best error handling in Rust?
    ///
    /// ASSISTANT (2025-11-20 19:30):
    /// Rust uses Result<T, E>...
    /// --- End Previous Conversation ---
    ///
    /// USER (current):
    /// Show me an example.
    /// ```
    pub fn format_history(conversation: &Conversation, current_message: &str) -> String {
        let mut prompt = String::new();

        // Add system prompt if present
        if let Some(system) = &conversation.system_prompt {
            prompt.push_str("SYSTEM: ");
            prompt.push_str(system);
            prompt.push_str("\n\n");
        }

        // Add conversation history if present
        if !conversation.messages.is_empty() {
            prompt.push_str("--- Previous Conversation ---\n");

            for msg in &conversation.messages {
                let role = match msg.role {
                    Role::System => "SYSTEM",
                    Role::User => "USER",
                    Role::Assistant => "ASSISTANT",
                };

                let timestamp = msg
                    .timestamp
                    .as_ref()
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                prompt.push_str(&format!("{} ({}):\n{}\n\n", role, timestamp, msg.content));
            }

            prompt.push_str("--- End Previous Conversation ---\n\n");
        }

        // Add current message
        prompt.push_str("USER (current):\n");
        prompt.push_str(current_message);

        prompt
    }

    /// Estimate token count using heuristic
    ///
    /// Without tiktoken, use character-based estimation:
    /// - Code: ~3 chars per token (denser)
    /// - Prose: ~4 chars per token (conservative)
    pub fn estimate_tokens(text: &str) -> usize {
        let char_count = text.chars().count();

        // Detect code by looking for common programming keywords
        let is_code = text.contains("fn ")
            || text.contains("def ")
            || text.contains("class ")
            || text.contains("impl ")
            || text.contains("struct ")
            || text.contains("const ")
            || text.contains("let ")
            || text.contains("var ");

        if is_code {
            char_count / 3 // Code is denser
        } else {
            char_count / 4 // Prose (conservative)
        }
    }

    /// Get context limit for model
    pub fn get_context_limit(model: &str) -> usize {
        let limit = if model.contains("opus") {
            CLAUDE_OPUS_LIMIT
        } else if model.contains("sonnet") {
            CLAUDE_SONNET_LIMIT
        } else if model.contains("haiku") {
            CLAUDE_HAIKU_LIMIT
        } else if model.contains("gemini") && model.contains("pro") {
            GEMINI_PRO_LIMIT
        } else if model.contains("gemini") && model.contains("flash") {
            GEMINI_FLASH_LIMIT
        } else {
            // Default to smallest known limit
            CLAUDE_HAIKU_LIMIT
        };

        (limit as f32 * CONTEXT_SAFETY_MARGIN) as usize
    }

    /// Compress conversation if needed (Level 1: Lossless)
    ///
    /// Strategy:
    /// - Keep first message (context setter)
    /// - Keep last N messages (immediate context)
    /// - Remove middle messages if over limit
    pub fn compress_if_needed(conversation: &Conversation, current_message: &str) -> Conversation {
        let full_prompt = Self::format_history(conversation, current_message);
        let estimated_tokens = Self::estimate_tokens(&full_prompt);
        let limit = Self::get_context_limit(&conversation.model);

        if estimated_tokens <= limit {
            return conversation.clone();
        }

        tracing::warn!(
            "Context exceeds limit ({} > {}), compressing...",
            estimated_tokens,
            limit
        );

        let mut compressed = conversation.clone();

        // Keep first message + last 3 messages
        if compressed.messages.len() > 4 {
            let first = compressed.messages[0].clone();
            let last_three: Vec<Message> = compressed
                .messages
                .iter()
                .rev()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();

            compressed.messages = vec![first];
            compressed.messages.extend(last_three);

            tracing::info!(
                "Compressed conversation: {} -> {} messages",
                conversation.messages.len(),
                compressed.messages.len()
            );
        }

        compressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_history_empty() {
        let conversation = Conversation {
            messages: vec![],
            system_prompt: None,
            model: "claude-sonnet-4.5".to_string(),
        };

        let formatted = CliContextManager::format_history(&conversation, "Hello");
        assert_eq!(formatted, "USER (current):\nHello");
    }

    #[test]
    fn test_format_history_with_system() {
        let conversation = Conversation {
            messages: vec![],
            system_prompt: Some("You are helpful.".to_string()),
            model: "claude-sonnet-4.5".to_string(),
        };

        let formatted = CliContextManager::format_history(&conversation, "Hello");
        assert!(formatted.contains("SYSTEM: You are helpful."));
        assert!(formatted.contains("USER (current):\nHello"));
    }

    #[test]
    fn test_estimate_tokens_prose() {
        let text = "This is a simple sentence with no code.";
        let tokens = CliContextManager::estimate_tokens(text);
        assert_eq!(tokens, text.chars().count() / 4);
    }

    #[test]
    fn test_estimate_tokens_code() {
        let text = "fn main() { let x = 42; }";
        let tokens = CliContextManager::estimate_tokens(text);
        assert_eq!(tokens, text.chars().count() / 3);
    }

    #[test]
    fn test_get_context_limit() {
        assert_eq!(
            CliContextManager::get_context_limit("claude-opus-4.5"),
            (CLAUDE_OPUS_LIMIT as f32 * CONTEXT_SAFETY_MARGIN) as usize
        );
        assert_eq!(
            CliContextManager::get_context_limit("gemini-2.5-pro"),
            (GEMINI_PRO_LIMIT as f32 * CONTEXT_SAFETY_MARGIN) as usize
        );
    }

    #[test]
    fn test_compress_if_needed_no_compression() {
        let conversation = Conversation {
            messages: vec![Message {
                role: Role::User,
                content: "Short message".to_string(),
                timestamp: Some(Utc::now()),
            }],
            system_prompt: None,
            model: "claude-sonnet-4.5".to_string(),
        };

        let compressed =
            CliContextManager::compress_if_needed(&conversation, "Another short message");
        assert_eq!(compressed.messages.len(), conversation.messages.len());
    }
}
