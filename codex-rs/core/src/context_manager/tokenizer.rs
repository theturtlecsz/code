//! Token counting for different providers

use super::{ContentBlock, Message, ProviderId};

/// Count tokens for a message using provider-specific tokenizer
///
/// Currently uses approximations. Future: integrate tiktoken-rs for OpenAI,
/// and provider-specific tokenizers for Anthropic/Google.
pub fn count_tokens(provider: ProviderId, message: &Message) -> usize {
    match provider {
        ProviderId::OpenAI => count_tokens_openai(message),
        ProviderId::Anthropic => count_tokens_anthropic(message),
        ProviderId::Google => count_tokens_google(message),
    }
}

/// OpenAI token counting using approximation
///
/// OpenAI uses tiktoken (BPE). Approximation: ~4 chars per token.
fn count_tokens_openai(message: &Message) -> usize {
    let (text_chars, extra_tokens): (usize, usize) =
        message
            .content
            .iter()
            .fold((0, 0), |(chars, tokens), c| match c {
                ContentBlock::Text { text } => (chars + text.len(), tokens),
                ContentBlock::Image { .. } => (chars, tokens + 85), // Images are ~85 tokens base
                ContentBlock::ToolUse { name, input, .. } => {
                    (chars + name.len() + input.to_string().len(), tokens)
                }
                ContentBlock::ToolResult { content, .. } => (chars + content.len(), tokens),
            });

    // Role overhead (~4 tokens) + text tokens + image tokens
    4 + (text_chars / 4) + extra_tokens
}

/// Anthropic token counting using approximation
///
/// Anthropic uses similar BPE tokenization.
/// Approximation: ~3.5 chars per token (slightly more efficient).
fn count_tokens_anthropic(message: &Message) -> usize {
    let (text_chars, extra_tokens): (usize, usize) =
        message
            .content
            .iter()
            .fold((0, 0), |(chars, tokens), c| match c {
                ContentBlock::Text { text } => (chars + text.len(), tokens),
                ContentBlock::Image { .. } => (chars, tokens + 1000), // Images cost more in Anthropic (~1000 tokens)
                ContentBlock::ToolUse { name, input, .. } => {
                    (chars + name.len() + input.to_string().len(), tokens)
                }
                ContentBlock::ToolResult { content, .. } => (chars + content.len(), tokens),
            });

    // Role overhead (~3 tokens) + text tokens + image tokens
    3 + (text_chars * 10 / 35) + extra_tokens
}

/// Google token counting using approximation
///
/// Google uses SentencePiece tokenization.
/// Approximation: ~4 chars per token.
fn count_tokens_google(message: &Message) -> usize {
    let (text_chars, extra_tokens): (usize, usize) =
        message
            .content
            .iter()
            .fold((0, 0), |(chars, tokens), c| match c {
                ContentBlock::Text { text } => (chars + text.len(), tokens),
                ContentBlock::Image { .. } => (chars, tokens + 258), // Google charges per image (~258 tokens)
                ContentBlock::ToolUse { name, input, .. } => {
                    (chars + name.len() + input.to_string().len(), tokens)
                }
                ContentBlock::ToolResult { content, .. } => (chars + content.len(), tokens),
            });

    // Role overhead (~4 tokens) + text tokens + image tokens
    4 + (text_chars / 4) + extra_tokens
}

/// Get context window size for a model
pub fn get_context_window(provider: ProviderId, model: &str) -> usize {
    match provider {
        ProviderId::OpenAI => get_openai_context_window(model),
        ProviderId::Anthropic => get_anthropic_context_window(model),
        ProviderId::Google => get_google_context_window(model),
    }
}

fn get_openai_context_window(model: &str) -> usize {
    let model_lower = model.to_lowercase();

    // GPT-5 and O-series models
    if model_lower.contains("gpt-5") || model_lower.contains("o1") || model_lower.contains("o3") {
        200_000
    }
    // GPT-4 Turbo and newer
    else if model_lower.contains("gpt-4-turbo")
        || model_lower.contains("gpt-4o")
        || model_lower.contains("gpt-4-1106")
        || model_lower.contains("gpt-4-0125")
    {
        128_000
    }
    // GPT-4 base
    else if model_lower.contains("gpt-4-32k") {
        32_000
    } else if model_lower.contains("gpt-4") {
        8_192
    }
    // GPT-3.5
    else if model_lower.contains("gpt-3.5-turbo-16k") {
        16_385
    } else if model_lower.contains("gpt-3.5") {
        4_096
    }
    // Default
    else {
        128_000
    }
}

fn get_anthropic_context_window(model: &str) -> usize {
    let model_lower = model.to_lowercase();

    // All Claude 3.x models have 200k context
    if model_lower.contains("claude-3")
        || model_lower.contains("claude-sonnet")
        || model_lower.contains("claude-opus")
        || model_lower.contains("claude-haiku")
    {
        200_000
    }
    // Claude 2.x
    else if model_lower.contains("claude-2") {
        100_000
    }
    // Default
    else {
        200_000
    }
}

fn get_google_context_window(model: &str) -> usize {
    let model_lower = model.to_lowercase();

    // Gemini 1.5/2.0 Pro
    if model_lower.contains("gemini-1.5-pro")
        || model_lower.contains("gemini-2.0-pro")
        || model_lower.contains("gemini-pro")
        || model_lower.contains("gemini-3-pro")
        || model_lower.contains("gemini-ultra")
    {
        2_000_000
    }
    // Gemini Flash
    else if model_lower.contains("flash") {
        1_000_000
    }
    // Gemini 1.0
    else if model_lower.contains("gemini-1.0") {
        32_000
    }
    // Default
    else {
        1_000_000
    }
}

/// Estimate tokens for a string (quick approximation)
#[allow(dead_code)]
pub fn estimate_tokens(text: &str) -> usize {
    // Simple approximation: ~4 chars per token
    text.len() / 4 + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_manager::MessageRole;

    #[test]
    fn test_count_tokens_openai() {
        let msg = Message::user("Hello, world!");
        let tokens = count_tokens(ProviderId::OpenAI, &msg);
        // "Hello, world!" is ~3 tokens + overhead
        assert!(tokens > 0 && tokens < 20);
    }

    #[test]
    fn test_count_tokens_anthropic() {
        let msg = Message::user("Hello, world!");
        let tokens = count_tokens(ProviderId::Anthropic, &msg);
        assert!(tokens > 0 && tokens < 20);
    }

    #[test]
    fn test_count_tokens_google() {
        let msg = Message::user("Hello, world!");
        let tokens = count_tokens(ProviderId::Google, &msg);
        assert!(tokens > 0 && tokens < 20);
    }

    #[test]
    fn test_count_tokens_with_image() {
        let msg = Message::new(
            MessageRole::User,
            vec![
                ContentBlock::text("Look at this:"),
                ContentBlock::image("data:image/png;base64,abc123"),
            ],
        );

        let openai_tokens = count_tokens(ProviderId::OpenAI, &msg);
        let anthropic_tokens = count_tokens(ProviderId::Anthropic, &msg);

        // Images should add significant tokens
        assert!(openai_tokens > 80);
        assert!(anthropic_tokens > 900); // Anthropic charges more for images
    }

    #[test]
    fn test_get_context_window_openai() {
        assert_eq!(get_context_window(ProviderId::OpenAI, "gpt-5"), 200_000);
        assert_eq!(
            get_context_window(ProviderId::OpenAI, "gpt-4-turbo"),
            128_000
        );
        assert_eq!(get_context_window(ProviderId::OpenAI, "gpt-4-32k"), 32_000);
        assert_eq!(get_context_window(ProviderId::OpenAI, "gpt-4"), 8_192);
    }

    #[test]
    fn test_get_context_window_anthropic() {
        assert_eq!(
            get_context_window(ProviderId::Anthropic, "claude-sonnet-4.5"),
            200_000
        );
        assert_eq!(
            get_context_window(ProviderId::Anthropic, "claude-opus-4.5"),
            200_000
        );
        assert_eq!(
            get_context_window(ProviderId::Anthropic, "claude-haiku-4.5"),
            200_000
        );
    }

    #[test]
    fn test_get_context_window_google() {
        assert_eq!(
            get_context_window(ProviderId::Google, "gemini-3-pro"),
            2_000_000
        );
        assert_eq!(
            get_context_window(ProviderId::Google, "gemini-2.5-flash"),
            1_000_000
        );
    }

    #[test]
    fn test_estimate_tokens() {
        // 100 chars should be ~25 tokens
        let text = "a".repeat(100);
        let estimate = estimate_tokens(&text);
        assert!(estimate >= 20 && estimate <= 30);
    }
}
