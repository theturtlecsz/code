# SPEC-KIT-953-E: Context Manager

**Status**: Draft
**Created**: 2025-11-20
**Type**: Implementation SPEC
**Priority**: High
**Estimated Effort**: 25-35 hours
**Dependencies**: SPEC-953-A, SPEC-953-B findings (message formats); SPEC-953-D (ProviderAuth types)

---

## Executive Summary

Implement a Context Manager that abstracts conversation management across providers with different message formats, token limits, and serialization requirements. This enables stateful multi-turn conversations with Claude and Gemini providers, replacing the current stateless CLI routing.

---

## Problem Statement

Current architecture is stateless:
- `model_router.rs:90` - `execute_prompt()` takes single prompt, no history
- `providers/mod.rs:23` - `ProviderResponse` has content only, no context tracking
- `message_history.rs:5-10` - `HistoryEntry` is minimal (session_id, timestamp, text)

Each provider requires different message formats:
- **OpenAI**: `{"role": "user/assistant/system", "content": "..."}`
- **Anthropic**: `{"role": "user/assistant", "content": [{"type": "text", "text": "..."}]}`
- **Google**: `{"parts": [{"text": "..."}], "role": "user/model"}`

Token counting also varies by provider tokenizer (tiktoken vs claude vs google).

---

## Solution: Provider-Agnostic Context Manager

### Core Abstractions

```rust
// codex-rs/core/src/context_manager/mod.rs

use serde::{Deserialize, Serialize};

/// Provider identifier (matches SPEC-953-D)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderId {
    OpenAI,
    Anthropic,
    Google,
}

/// Message role - canonical representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Content block - canonical representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentBlock {
    /// Plain text content
    Text { text: String },

    /// Image content (base64 data URL or URL)
    Image {
        url: String,
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
        is_error: bool,
    },
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role
    pub role: MessageRole,

    /// Content blocks
    pub content: Vec<ContentBlock>,

    /// Token count (computed lazily, cached)
    #[serde(skip)]
    token_count: Option<usize>,
}

impl Message {
    pub fn new(role: MessageRole, content: Vec<ContentBlock>) -> Self {
        Self {
            role,
            content,
            token_count: None,
        }
    }

    pub fn text(role: MessageRole, text: impl Into<String>) -> Self {
        Self::new(role, vec![ContentBlock::Text { text: text.into() }])
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self::text(MessageRole::System, text)
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::text(MessageRole::User, text)
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::text(MessageRole::Assistant, text)
    }
}
```

### Conversation History

```rust
// codex-rs/core/src/context_manager/history.rs

use super::{Message, MessageRole, ProviderId};
use std::path::PathBuf;

/// Token budget configuration
#[derive(Debug, Clone)]
pub struct TokenBudget {
    /// Maximum tokens for the entire context window
    pub max_context_tokens: usize,

    /// Reserved tokens for system prompt
    pub system_prompt_reserved: usize,

    /// Reserved tokens for response
    pub response_reserved: usize,
}

impl TokenBudget {
    /// Available tokens for conversation history
    pub fn available_for_history(&self) -> usize {
        self.max_context_tokens
            .saturating_sub(self.system_prompt_reserved)
            .saturating_sub(self.response_reserved)
    }
}

/// Truncation strategy for managing context window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationStrategy {
    /// Remove oldest messages first (sliding window)
    SlidingWindow,

    /// Summarize older messages into a single message
    Summarize,

    /// Remove messages by priority (preserve system, recent user/assistant)
    PriorityBased,
}

/// Conversation history with token tracking
#[derive(Debug, Clone)]
pub struct ConversationHistory {
    /// System prompt (always preserved)
    system_prompt: Option<Message>,

    /// Conversation messages (user/assistant turns)
    messages: Vec<Message>,

    /// Total tokens in history (excluding system prompt)
    total_tokens: usize,

    /// Token counter for the target provider
    provider: ProviderId,

    /// Token budget configuration
    budget: TokenBudget,

    /// Truncation strategy
    truncation_strategy: TruncationStrategy,
}

impl ConversationHistory {
    /// Create new history for a provider
    pub fn new(provider: ProviderId, budget: TokenBudget) -> Self {
        Self {
            system_prompt: None,
            messages: Vec::new(),
            total_tokens: 0,
            provider,
            budget,
            truncation_strategy: TruncationStrategy::SlidingWindow,
        }
    }

    /// Set system prompt
    pub fn set_system_prompt(&mut self, prompt: Message) {
        self.system_prompt = Some(prompt);
    }

    /// Add a message to history
    pub fn add_message(&mut self, message: Message) {
        let token_count = self.count_tokens(&message);
        self.messages.push(message);
        self.total_tokens += token_count;

        // Truncate if needed
        self.truncate_if_needed();
    }

    /// Get all messages including system prompt
    pub fn all_messages(&self) -> Vec<&Message> {
        let mut result = Vec::with_capacity(1 + self.messages.len());
        if let Some(ref system) = self.system_prompt {
            result.push(system);
        }
        result.extend(self.messages.iter());
        result
    }

    /// Clear conversation history (keep system prompt)
    pub fn clear(&mut self) {
        self.messages.clear();
        self.total_tokens = 0;
    }

    /// Current token usage
    pub fn token_count(&self) -> usize {
        self.total_tokens
    }

    /// Available tokens before truncation
    pub fn tokens_available(&self) -> usize {
        self.budget.available_for_history().saturating_sub(self.total_tokens)
    }

    /// Count tokens for a message using provider-specific tokenizer
    fn count_tokens(&self, message: &Message) -> usize {
        // Delegate to token counter
        crate::context_manager::tokenizer::count_tokens(self.provider, message)
    }

    /// Apply truncation strategy if over budget
    fn truncate_if_needed(&mut self) {
        let available = self.budget.available_for_history();

        while self.total_tokens > available && !self.messages.is_empty() {
            match self.truncation_strategy {
                TruncationStrategy::SlidingWindow => {
                    // Remove oldest message
                    if let Some(removed) = self.messages.first() {
                        let token_count = self.count_tokens(removed);
                        self.total_tokens = self.total_tokens.saturating_sub(token_count);
                    }
                    self.messages.remove(0);
                }
                TruncationStrategy::Summarize => {
                    // TODO: Implement summarization
                    // For now, fall back to sliding window
                    if let Some(removed) = self.messages.first() {
                        let token_count = self.count_tokens(removed);
                        self.total_tokens = self.total_tokens.saturating_sub(token_count);
                    }
                    self.messages.remove(0);
                }
                TruncationStrategy::PriorityBased => {
                    // Remove oldest non-essential message
                    // Keep recent user messages and all assistant messages with tool calls
                    let remove_idx = self.find_lowest_priority_message();
                    if let Some(idx) = remove_idx {
                        let removed = &self.messages[idx];
                        let token_count = self.count_tokens(removed);
                        self.total_tokens = self.total_tokens.saturating_sub(token_count);
                        self.messages.remove(idx);
                    } else {
                        // Fall back to removing oldest
                        if let Some(removed) = self.messages.first() {
                            let token_count = self.count_tokens(removed);
                            self.total_tokens = self.total_tokens.saturating_sub(token_count);
                        }
                        self.messages.remove(0);
                    }
                }
            }
        }
    }

    /// Find index of lowest priority message for removal
    fn find_lowest_priority_message(&self) -> Option<usize> {
        // Simple heuristic: oldest user message without images
        self.messages.iter().enumerate()
            .filter(|(_, m)| m.role == MessageRole::User)
            .filter(|(_, m)| !m.content.iter().any(|c| matches!(c, ContentBlock::Image { .. })))
            .map(|(i, _)| i)
            .next()
    }
}
```

### Provider-Specific Serializers

```rust
// codex-rs/core/src/context_manager/serializer.rs

use super::{Message, MessageRole, ContentBlock, ProviderId};
use serde_json::{json, Value};

/// Serialize messages to provider-specific format
pub fn serialize_for_provider(provider: ProviderId, messages: &[&Message]) -> Value {
    match provider {
        ProviderId::OpenAI => serialize_openai(messages),
        ProviderId::Anthropic => serialize_anthropic(messages),
        ProviderId::Google => serialize_google(messages),
    }
}

/// OpenAI Chat Completions format
fn serialize_openai(messages: &[&Message]) -> Value {
    let msgs: Vec<Value> = messages.iter().map(|m| {
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
        let content: Vec<Value> = m.content.iter().map(|c| {
            match c {
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
                ContentBlock::ToolResult { tool_use_id, content, .. } => json!({
                    "type": "tool_result",
                    "tool_call_id": tool_use_id,
                    "content": content
                }),
            }
        }).collect();

        json!({
            "role": role,
            "content": content
        })
    }).collect();

    json!(msgs)
}

/// Anthropic Messages API format
fn serialize_anthropic(messages: &[&Message]) -> Value {
    let mut system_prompt = None;
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
                let role = if m.role == MessageRole::User { "user" } else { "assistant" };

                let content: Vec<Value> = m.content.iter().map(|c| {
                    match c {
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
                                    let media = media_type.as_deref()
                                        .unwrap_or("image/png");
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
                        ContentBlock::ToolResult { tool_use_id, content, is_error } => json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content,
                            "is_error": is_error
                        }),
                    }
                }).collect();

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
fn serialize_google(messages: &[&Message]) -> Value {
    let mut system_instruction = None;
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
                let role = if m.role == MessageRole::User { "user" } else { "model" };

                let parts: Vec<Value> = m.content.iter().map(|c| {
                    match c {
                        ContentBlock::Text { text } => json!({
                            "text": text
                        }),
                        ContentBlock::Image { url, media_type } => {
                            // Google expects inline_data for base64
                            if url.starts_with("data:") {
                                let parts: Vec<&str> = url.splitn(2, ',').collect();
                                if parts.len() == 2 {
                                    let mime = media_type.as_deref()
                                        .unwrap_or("image/png");
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
                        ContentBlock::ToolUse { id, name, input } => json!({
                            "function_call": {
                                "name": name,
                                "args": input
                            }
                        }),
                        ContentBlock::ToolResult { tool_use_id, content, .. } => json!({
                            "function_response": {
                                "name": tool_use_id,
                                "response": {
                                    "content": content
                                }
                            }
                        }),
                    }
                }).collect();

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
```

### Token Counting

```rust
// codex-rs/core/src/context_manager/tokenizer.rs

use super::{Message, ProviderId, ContentBlock};

/// Count tokens for a message using provider-specific tokenizer
pub fn count_tokens(provider: ProviderId, message: &Message) -> usize {
    match provider {
        ProviderId::OpenAI => count_tokens_openai(message),
        ProviderId::Anthropic => count_tokens_anthropic(message),
        ProviderId::Google => count_tokens_google(message),
    }
}

/// OpenAI token counting using tiktoken
fn count_tokens_openai(message: &Message) -> usize {
    // Use tiktoken-rs for accurate counting
    // For now, use approximation: ~4 chars per token
    let text_len: usize = message.content.iter().map(|c| {
        match c {
            ContentBlock::Text { text } => text.len(),
            ContentBlock::Image { .. } => 85, // Images are ~85 tokens base
            ContentBlock::ToolUse { name, input, .. } => {
                name.len() + input.to_string().len()
            }
            ContentBlock::ToolResult { content, .. } => content.len(),
        }
    }).sum();

    // Role overhead + content
    4 + (text_len / 4)
}

/// Anthropic token counting
fn count_tokens_anthropic(message: &Message) -> usize {
    // Anthropic uses similar BPE tokenization
    // Approximation: ~3.5 chars per token (slightly more efficient)
    let text_len: usize = message.content.iter().map(|c| {
        match c {
            ContentBlock::Text { text } => text.len(),
            ContentBlock::Image { .. } => 1000, // Images cost more in Anthropic
            ContentBlock::ToolUse { name, input, .. } => {
                name.len() + input.to_string().len()
            }
            ContentBlock::ToolResult { content, .. } => content.len(),
        }
    }).sum();

    // Role overhead + content
    3 + (text_len * 10 / 35)
}

/// Google token counting
fn count_tokens_google(message: &Message) -> usize {
    // Google uses SentencePiece tokenization
    // Approximation: ~4 chars per token
    let text_len: usize = message.content.iter().map(|c| {
        match c {
            ContentBlock::Text { text } => text.len(),
            ContentBlock::Image { .. } => 258, // Google charges per image
            ContentBlock::ToolUse { name, input, .. } => {
                name.len() + input.to_string().len()
            }
            ContentBlock::ToolResult { content, .. } => content.len(),
        }
    }).sum();

    // Role overhead + content
    4 + (text_len / 4)
}

/// Get context window size for a model
pub fn get_context_window(provider: ProviderId, model: &str) -> usize {
    match provider {
        ProviderId::OpenAI => {
            // GPT-4 variants
            if model.contains("gpt-5") || model.contains("o1") || model.contains("o3") {
                200_000
            } else if model.contains("gpt-4") {
                128_000
            } else {
                16_000
            }
        }
        ProviderId::Anthropic => {
            // Claude models have large context
            if model.contains("opus") || model.contains("sonnet") {
                200_000
            } else {
                // Haiku
                200_000
            }
        }
        ProviderId::Google => {
            // Gemini models
            if model.contains("pro") || model.contains("ultra") {
                2_000_000 // Gemini 1.5/2.0 Pro
            } else if model.contains("flash") {
                1_000_000 // Gemini Flash
            } else {
                32_000
            }
        }
    }
}
```

### Session Persistence

```rust
// codex-rs/core/src/context_manager/persistence.rs

use super::{ConversationHistory, Message, ProviderId, TokenBudget, TruncationStrategy};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Serializable session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session identifier
    pub session_id: String,

    /// Provider for this session
    pub provider: ProviderId,

    /// System prompt
    pub system_prompt: Option<Message>,

    /// Conversation messages
    pub messages: Vec<Message>,

    /// Token budget configuration
    pub budget: TokenBudgetConfig,

    /// Truncation strategy
    pub truncation_strategy: String,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudgetConfig {
    pub max_context_tokens: usize,
    pub system_prompt_reserved: usize,
    pub response_reserved: usize,
}

impl From<TokenBudget> for TokenBudgetConfig {
    fn from(budget: TokenBudget) -> Self {
        Self {
            max_context_tokens: budget.max_context_tokens,
            system_prompt_reserved: budget.system_prompt_reserved,
            response_reserved: budget.response_reserved,
        }
    }
}

impl From<TokenBudgetConfig> for TokenBudget {
    fn from(config: TokenBudgetConfig) -> Self {
        Self {
            max_context_tokens: config.max_context_tokens,
            system_prompt_reserved: config.system_prompt_reserved,
            response_reserved: config.response_reserved,
        }
    }
}

/// Session manager for persistence
pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    /// Create session manager
    pub fn new(codex_home: PathBuf) -> Self {
        let sessions_dir = codex_home.join("sessions");
        Self { sessions_dir }
    }

    /// Save session to disk
    pub fn save_session(&self, history: &ConversationHistory, session_id: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.sessions_dir)?;

        let state = SessionState {
            session_id: session_id.to_string(),
            provider: history.provider,
            system_prompt: history.system_prompt.clone(),
            messages: history.messages.clone(),
            budget: history.budget.clone().into(),
            truncation_strategy: format!("{:?}", history.truncation_strategy),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let path = self.session_path(session_id);
        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(path, json)?;

        Ok(())
    }

    /// Load session from disk
    pub fn load_session(&self, session_id: &str) -> std::io::Result<ConversationHistory> {
        let path = self.session_path(session_id);
        let json = std::fs::read_to_string(path)?;
        let state: SessionState = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut history = ConversationHistory::new(state.provider, state.budget.into());
        history.system_prompt = state.system_prompt;
        history.messages = state.messages;
        history.truncation_strategy = match state.truncation_strategy.as_str() {
            "SlidingWindow" => TruncationStrategy::SlidingWindow,
            "Summarize" => TruncationStrategy::Summarize,
            "PriorityBased" => TruncationStrategy::PriorityBased,
            _ => TruncationStrategy::SlidingWindow,
        };

        // Recalculate total tokens
        history.total_tokens = history.messages.iter()
            .map(|m| crate::context_manager::tokenizer::count_tokens(history.provider, m))
            .sum();

        Ok(history)
    }

    /// List all sessions
    pub fn list_sessions(&self) -> std::io::Result<Vec<String>> {
        if !self.sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(name) = entry.path().file_stem() {
                    sessions.push(name.to_string_lossy().to_string());
                }
            }
        }

        Ok(sessions)
    }

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> std::io::Result<()> {
        let path = self.session_path(session_id);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", session_id))
    }
}
```

### Public API (Context Manager)

```rust
// codex-rs/core/src/context_manager/manager.rs

use super::{
    ConversationHistory, Message, ProviderId, TokenBudget, TruncationStrategy,
    persistence::SessionManager, serializer, tokenizer,
};
use std::path::PathBuf;
use serde_json::Value;

/// Context Manager - central orchestrator for conversation context
pub struct ContextManager {
    /// Current conversation history
    history: ConversationHistory,

    /// Session persistence
    session_manager: SessionManager,

    /// Current session ID (if persisted)
    session_id: Option<String>,
}

impl ContextManager {
    /// Create new context manager for a provider
    pub fn new(provider: ProviderId, model: &str, codex_home: PathBuf) -> Self {
        let context_window = tokenizer::get_context_window(provider, model);

        let budget = TokenBudget {
            max_context_tokens: context_window,
            system_prompt_reserved: 2000,
            response_reserved: 4000,
        };

        Self {
            history: ConversationHistory::new(provider, budget),
            session_manager: SessionManager::new(codex_home),
            session_id: None,
        }
    }

    /// Set system prompt
    pub fn set_system_prompt(&mut self, text: impl Into<String>) {
        self.history.set_system_prompt(Message::system(text));
    }

    /// Add user message
    pub fn add_user_message(&mut self, text: impl Into<String>) {
        self.history.add_message(Message::user(text));
    }

    /// Add assistant message
    pub fn add_assistant_message(&mut self, text: impl Into<String>) {
        self.history.add_message(Message::assistant(text));
    }

    /// Add a complete message
    pub fn add_message(&mut self, message: Message) {
        self.history.add_message(message);
    }

    /// Get serialized messages for API request
    pub fn serialize_for_request(&self) -> Value {
        let messages: Vec<&Message> = self.history.all_messages();
        serializer::serialize_for_provider(self.history.provider, &messages)
    }

    /// Get current token usage
    pub fn token_count(&self) -> usize {
        self.history.token_count()
    }

    /// Get available tokens
    pub fn tokens_available(&self) -> usize {
        self.history.tokens_available()
    }

    /// Clear conversation (keep system prompt)
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Set truncation strategy
    pub fn set_truncation_strategy(&mut self, strategy: TruncationStrategy) {
        self.history.truncation_strategy = strategy;
    }

    /// Save current session
    pub fn save_session(&self, session_id: &str) -> std::io::Result<()> {
        self.session_manager.save_session(&self.history, session_id)
    }

    /// Load a session
    pub fn load_session(&mut self, session_id: &str) -> std::io::Result<()> {
        self.history = self.session_manager.load_session(session_id)?;
        self.session_id = Some(session_id.to_string());
        Ok(())
    }

    /// List available sessions
    pub fn list_sessions(&self) -> std::io::Result<Vec<String>> {
        self.session_manager.list_sessions()
    }

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> std::io::Result<()> {
        self.session_manager.delete_session(session_id)
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.history.messages.len()
    }

    /// Get provider
    pub fn provider(&self) -> ProviderId {
        self.history.provider
    }
}
```

---

## File Structure

```
codex-rs/core/src/
├── context_manager/
│   ├── mod.rs              # Module exports, core types (Message, ContentBlock, etc.)
│   ├── history.rs          # ConversationHistory with truncation
│   ├── serializer.rs       # Provider-specific serialization
│   ├── tokenizer.rs        # Token counting per provider
│   ├── persistence.rs      # Session save/load
│   └── manager.rs          # ContextManager (public API)
└── lib.rs                  # Add context_manager module
```

---

## Task Breakdown

| # | Task | Hours | Description |
|---|------|-------|-------------|
| 1 | Core types | 4 | `Message`, `ContentBlock`, `MessageRole`, `ProviderId` |
| 2 | ConversationHistory | 5 | Token tracking, truncation strategies |
| 3 | OpenAI serializer | 3 | Chat Completions format |
| 4 | Anthropic serializer | 3 | Messages API format |
| 5 | Google serializer | 3 | Generative AI format |
| 6 | Token counting | 4 | Per-provider tokenizer approximations |
| 7 | Session persistence | 4 | Save/load sessions to disk |
| 8 | ContextManager API | 3 | Public orchestration interface |
| 9 | Unit tests | 4 | Serialization, truncation, persistence |
| 10 | Integration tests | 3 | Cross-provider conversion roundtrip |
| **Total** | | **36** | |

---

## Acceptance Criteria

### Must Pass

1. **Core Types**
   - [ ] `Message` struct compiles with role, content, token_count
   - [ ] `ContentBlock` enum handles Text, Image, ToolUse, ToolResult
   - [ ] All types derive Serialize/Deserialize

2. **Serialization**
   - [ ] OpenAI format matches Chat Completions API schema
   - [ ] Anthropic format matches Messages API schema
   - [ ] Google format matches Generative AI schema
   - [ ] Round-trip conversion preserves semantics

3. **Token Management**
   - [ ] Token counting returns reasonable estimates per provider
   - [ ] Context window limits respected
   - [ ] Truncation removes oldest messages first (sliding window)
   - [ ] System prompt never truncated

4. **Session Persistence**
   - [ ] Save session to `~/.codex/sessions/<id>.json`
   - [ ] Load session restores full history
   - [ ] List sessions returns all saved sessions
   - [ ] Delete session removes file

5. **Quality Gates**
   - [ ] `cargo build --workspace` passes
   - [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
   - [ ] `cargo test -p codex-core context_manager` passes

### Tests Required

1. **Unit Tests**
   - [ ] Serialization to each provider format
   - [ ] Token counting accuracy (within 20% of actual)
   - [ ] Truncation behavior
   - [ ] Session save/load roundtrip

2. **Integration Tests**
   - [ ] Multi-turn conversation simulation
   - [ ] Provider switching mid-session (migration)
   - [ ] Large context truncation

---

## Dependencies

### Crate Dependencies

```toml
# codex-rs/core/Cargo.toml additions
[dependencies]
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
# tiktoken-rs = "0.5"  # Future: accurate OpenAI token counting
```

---

## Integration Points

### With SPEC-953-D (Provider Auth)
- Uses same `ProviderId` enum
- Context manager works alongside auth manager

### With SPEC-953-F/G (Native Providers)
- Claude/Gemini providers call `context_manager.serialize_for_request()`
- Response parsing adds messages via `context_manager.add_assistant_message()`

### With TUI (SPEC-953-H)
- Display token usage in chat view
- Session save/restore from UI

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Token counting inaccuracy | Start with approximations, add tiktoken-rs later |
| Serialization format drift | Pin to API versions, add validation tests |
| Large history performance | Lazy token counting, efficient truncation |
| Session file corruption | Atomic writes with temp file rename |

---

## Future Enhancements

1. **Accurate tokenizers**: Integrate tiktoken-rs, Anthropic tokenizer, Google tokenizer
2. **Summarization**: Implement LLM-based summarization for truncation
3. **Streaming**: Support incremental message building during streaming
4. **Compression**: Compress old sessions to save disk space
5. **Export**: Export conversations to markdown/JSON for sharing

---

## References

- SPEC-KIT-953: Master SPEC
- SPEC-KIT-953-D: Provider Authentication Framework
- OpenAI Chat Completions: https://platform.openai.com/docs/api-reference/chat
- Anthropic Messages: https://docs.anthropic.com/en/api/messages
- Google Generative AI: https://ai.google.dev/api/generate-content

---

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2025-11-20 | Claude | Initial SPEC with full design |
