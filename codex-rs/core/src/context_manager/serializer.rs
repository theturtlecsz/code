//! Provider-specific message serialization

use super::{ContentBlock, Message, MessageRole, ProviderId};
use serde_json::{Value, json};

/// Serialize messages to provider-specific format
pub fn serialize_for_provider(provider: ProviderId, messages: &[&Message]) -> Value {
    match provider {
        ProviderId::OpenAI => serialize_openai(messages),
        ProviderId::Anthropic => serialize_anthropic(messages),
        ProviderId::Google => serialize_google(messages),
    }
}

/// OpenAI Chat Completions format
///
/// Format:
/// ```json
/// [
///   {"role": "system", "content": "..."},
///   {"role": "user", "content": "..."},
///   {"role": "assistant", "content": "..."}
/// ]
/// ```
fn serialize_openai(messages: &[&Message]) -> Value {
    let msgs: Vec<Value> = messages
        .iter()
        .map(|m| {
            let role = match m.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };

            // OpenAI can use string content for simple text
            if m.content.len() == 1 {
                if let ContentBlock::Text { text } = &m.content[0] {
                    return json!({
                        "role": role,
                        "content": text
                    });
                }
            }

            // Multi-part content
            let content: Vec<Value> = m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => json!({
                        "type": "text",
                        "text": text
                    }),
                    ContentBlock::Image { url, .. } => json!({
                        "type": "image_url",
                        "image_url": { "url": url }
                    }),
                    ContentBlock::ToolUse { id, name, input } => json!({
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": input.to_string()
                        },
                        "id": id
                    }),
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => json!({
                        "type": "tool_result",
                        "tool_call_id": tool_use_id,
                        "content": content
                    }),
                })
                .collect();

            json!({
                "role": role,
                "content": content
            })
        })
        .collect();

    json!(msgs)
}

/// Anthropic Messages API format
///
/// Format:
/// ```json
/// {
///   "system": "system prompt",
///   "messages": [
///     {"role": "user", "content": [{"type": "text", "text": "..."}]},
///     {"role": "assistant", "content": [{"type": "text", "text": "..."}]}
///   ]
/// }
/// ```
fn serialize_anthropic(messages: &[&Message]) -> Value {
    let mut system_prompt: Option<String> = None;
    let mut conversation: Vec<Value> = Vec::new();

    for m in messages {
        match m.role {
            MessageRole::System => {
                // Anthropic uses separate system field
                if let Some(ContentBlock::Text { text }) = m.content.first() {
                    system_prompt = Some(text.clone());
                }
            }
            MessageRole::User | MessageRole::Assistant => {
                let role = if m.role == MessageRole::User {
                    "user"
                } else {
                    "assistant"
                };

                let content: Vec<Value> = m
                    .content
                    .iter()
                    .map(|c| match c {
                        ContentBlock::Text { text } => json!({
                            "type": "text",
                            "text": text
                        }),
                        ContentBlock::Image { url, media_type } => {
                            // Anthropic expects base64 data in specific format
                            if url.starts_with("data:") {
                                // Parse data URL
                                let parts: Vec<&str> = url.splitn(2, ',').collect();
                                if parts.len() == 2 {
                                    let media = media_type.as_deref().unwrap_or("image/png");
                                    json!({
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": media,
                                            "data": parts[1]
                                        }
                                    })
                                } else {
                                    json!({
                                        "type": "text",
                                        "text": "[Image]"
                                    })
                                }
                            } else {
                                // URL-based image
                                json!({
                                    "type": "image",
                                    "source": {
                                        "type": "url",
                                        "url": url
                                    }
                                })
                            }
                        }
                        ContentBlock::ToolUse { id, name, input } => json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input
                        }),
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } => json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content,
                            "is_error": is_error
                        }),
                    })
                    .collect();

                conversation.push(json!({
                    "role": role,
                    "content": content
                }));
            }
        }
    }

    json!({
        "system": system_prompt,
        "messages": conversation
    })
}

/// Google Generative AI format
///
/// Format:
/// ```json
/// {
///   "system_instruction": {"parts": [{"text": "..."}]},
///   "contents": [
///     {"role": "user", "parts": [{"text": "..."}]},
///     {"role": "model", "parts": [{"text": "..."}]}
///   ]
/// }
/// ```
fn serialize_google(messages: &[&Message]) -> Value {
    let mut system_instruction: Option<Value> = None;
    let mut contents: Vec<Value> = Vec::new();

    for m in messages {
        match m.role {
            MessageRole::System => {
                // Google uses system_instruction field
                if let Some(ContentBlock::Text { text }) = m.content.first() {
                    system_instruction = Some(json!({
                        "parts": [{ "text": text }]
                    }));
                }
            }
            MessageRole::User | MessageRole::Assistant => {
                // Google uses "model" instead of "assistant"
                let role = if m.role == MessageRole::User {
                    "user"
                } else {
                    "model"
                };

                let parts: Vec<Value> = m
                    .content
                    .iter()
                    .map(|c| match c {
                        ContentBlock::Text { text } => json!({
                            "text": text
                        }),
                        ContentBlock::Image { url, media_type } => {
                            // Google expects inline_data for base64
                            if url.starts_with("data:") {
                                let parts: Vec<&str> = url.splitn(2, ',').collect();
                                if parts.len() == 2 {
                                    let mime = media_type.as_deref().unwrap_or("image/png");
                                    json!({
                                        "inline_data": {
                                            "mime_type": mime,
                                            "data": parts[1]
                                        }
                                    })
                                } else {
                                    json!({ "text": "[Image]" })
                                }
                            } else {
                                json!({
                                    "file_data": {
                                        "file_uri": url
                                    }
                                })
                            }
                        }
                        ContentBlock::ToolUse { name, input, .. } => json!({
                            "function_call": {
                                "name": name,
                                "args": input
                            }
                        }),
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            ..
                        } => json!({
                            "function_response": {
                                "name": tool_use_id,
                                "response": {
                                    "content": content
                                }
                            }
                        }),
                    })
                    .collect();

                contents.push(json!({
                    "role": role,
                    "parts": parts
                }));
            }
        }
    }

    json!({
        "system_instruction": system_instruction,
        "contents": contents
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_openai_simple() {
        let msg = Message::user("Hello");
        let result = serialize_for_provider(ProviderId::OpenAI, &[&msg]);

        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);

        let first = &arr[0];
        assert_eq!(first["role"], "user");
        assert_eq!(first["content"], "Hello");
    }

    #[test]
    fn test_serialize_openai_with_system() {
        let system = Message::system("You are helpful");
        let user = Message::user("Hi");
        let result = serialize_for_provider(ProviderId::OpenAI, &[&system, &user]);

        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["role"], "system");
        assert_eq!(arr[1]["role"], "user");
    }

    #[test]
    fn test_serialize_anthropic_separates_system() {
        let system = Message::system("You are helpful");
        let user = Message::user("Hi");
        let result = serialize_for_provider(ProviderId::Anthropic, &[&system, &user]);

        // System should be in separate field
        assert_eq!(result["system"], "You are helpful");

        // Messages should only contain user
        let messages = result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
    }

    #[test]
    fn test_serialize_anthropic_content_format() {
        let msg = Message::user("Hello");
        let result = serialize_for_provider(ProviderId::Anthropic, &[&msg]);

        let messages = result["messages"].as_array().unwrap();
        let content = messages[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Hello");
    }

    #[test]
    fn test_serialize_google_uses_model_role() {
        let msg = Message::assistant("Hi there");
        let result = serialize_for_provider(ProviderId::Google, &[&msg]);

        let contents = result["contents"].as_array().unwrap();
        assert_eq!(contents[0]["role"], "model"); // Not "assistant"
    }

    #[test]
    fn test_serialize_google_system_instruction() {
        let system = Message::system("Be helpful");
        let user = Message::user("Hi");
        let result = serialize_for_provider(ProviderId::Google, &[&system, &user]);

        // System should be in system_instruction
        let sys = &result["system_instruction"];
        assert!(sys.is_object());
        let parts = sys["parts"].as_array().unwrap();
        assert_eq!(parts[0]["text"], "Be helpful");
    }

    #[test]
    fn test_serialize_google_parts_format() {
        let msg = Message::user("Hello");
        let result = serialize_for_provider(ProviderId::Google, &[&msg]);

        let contents = result["contents"].as_array().unwrap();
        let parts = contents[0]["parts"].as_array().unwrap();
        assert_eq!(parts[0]["text"], "Hello");
    }

    #[test]
    fn test_serialize_anthropic_image_base64() {
        let msg = Message::new(
            MessageRole::User,
            vec![ContentBlock::Image {
                url: "data:image/png;base64,abc123".to_string(),
                media_type: Some("image/png".to_string()),
            }],
        );
        let result = serialize_for_provider(ProviderId::Anthropic, &[&msg]);

        let messages = result["messages"].as_array().unwrap();
        let content = messages[0]["content"].as_array().unwrap();
        let image = &content[0];

        assert_eq!(image["type"], "image");
        assert_eq!(image["source"]["type"], "base64");
        assert_eq!(image["source"]["data"], "abc123");
    }

    #[test]
    fn test_serialize_tool_use_openai() {
        let msg = Message::new(
            MessageRole::Assistant,
            vec![ContentBlock::ToolUse {
                id: "call_123".to_string(),
                name: "get_weather".to_string(),
                input: json!({"city": "London"}),
            }],
        );
        let result = serialize_for_provider(ProviderId::OpenAI, &[&msg]);

        let arr = result.as_array().unwrap();
        let content = arr[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "function");
        assert_eq!(content[0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_serialize_tool_result_anthropic() {
        let msg = Message::new(
            MessageRole::User,
            vec![ContentBlock::ToolResult {
                tool_use_id: "call_123".to_string(),
                content: "Sunny, 22C".to_string(),
                is_error: false,
            }],
        );
        let result = serialize_for_provider(ProviderId::Anthropic, &[&msg]);

        let messages = result["messages"].as_array().unwrap();
        let content = messages[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_result");
        assert_eq!(content[0]["tool_use_id"], "call_123");
        assert_eq!(content[0]["is_error"], false);
    }
}
