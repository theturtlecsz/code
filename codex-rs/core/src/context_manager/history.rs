//! Conversation history with token tracking and truncation

use super::{Message, MessageRole, ProviderId, tokenizer};

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
    /// Create a new token budget
    pub fn new(max_context: usize, system_reserved: usize, response_reserved: usize) -> Self {
        Self {
            max_context_tokens: max_context,
            system_prompt_reserved: system_reserved,
            response_reserved,
        }
    }

    /// Available tokens for conversation history
    pub fn available_for_history(&self) -> usize {
        self.max_context_tokens
            .saturating_sub(self.system_prompt_reserved)
            .saturating_sub(self.response_reserved)
    }
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_context_tokens: 128_000,
            system_prompt_reserved: 2_000,
            response_reserved: 4_000,
        }
    }
}

/// Truncation strategy for managing context window
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TruncationStrategy {
    /// Remove oldest messages first (sliding window)
    #[default]
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
    pub(crate) system_prompt: Option<Message>,

    /// Conversation messages (user/assistant turns)
    pub(crate) messages: Vec<Message>,

    /// Total tokens in history (excluding system prompt)
    pub(crate) total_tokens: usize,

    /// Token counter for the target provider
    pub(crate) provider: ProviderId,

    /// Token budget configuration
    pub(crate) budget: TokenBudget,

    /// Truncation strategy
    pub(crate) truncation_strategy: TruncationStrategy,
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
            truncation_strategy: TruncationStrategy::default(),
        }
    }

    /// Set system prompt
    pub fn set_system_prompt(&mut self, prompt: Message) {
        self.system_prompt = Some(prompt);
    }

    /// Get system prompt
    pub fn system_prompt(&self) -> Option<&Message> {
        self.system_prompt.as_ref()
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

    /// Get only conversation messages (no system prompt)
    pub fn messages(&self) -> &[Message] {
        &self.messages
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
        self.budget
            .available_for_history()
            .saturating_sub(self.total_tokens)
    }

    /// Message count (excluding system prompt)
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get provider
    pub fn provider(&self) -> ProviderId {
        self.provider
    }

    /// Get token budget
    pub fn budget(&self) -> &TokenBudget {
        &self.budget
    }

    /// Set truncation strategy
    pub fn set_truncation_strategy(&mut self, strategy: TruncationStrategy) {
        self.truncation_strategy = strategy;
    }

    /// Count tokens for a message using provider-specific tokenizer
    fn count_tokens(&self, message: &Message) -> usize {
        tokenizer::count_tokens(self.provider, message)
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
                    // TODO: Implement summarization (requires LLM call)
                    // For now, fall back to sliding window
                    if let Some(removed) = self.messages.first() {
                        let token_count = self.count_tokens(removed);
                        self.total_tokens = self.total_tokens.saturating_sub(token_count);
                    }
                    self.messages.remove(0);
                }
                TruncationStrategy::PriorityBased => {
                    // Remove oldest non-essential message
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
        // Simple heuristic: oldest user message without images or tool results
        self.messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.role == MessageRole::User)
            .filter(|(_, m)| !m.has_images())
            .filter(|(_, m)| {
                !m.content
                    .iter()
                    .any(|c| matches!(c, super::ContentBlock::ToolResult { .. }))
            })
            .map(|(i, _)| i)
            .next()
    }

    /// Recalculate total tokens (call after loading from disk)
    pub(crate) fn recalculate_tokens(&mut self) {
        self.total_tokens = self.messages.iter().map(|m| self.count_tokens(m)).sum();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_budget() {
        let budget = TokenBudget::new(128_000, 2_000, 4_000);
        assert_eq!(budget.available_for_history(), 122_000);
    }

    #[test]
    fn test_history_add_message() {
        let budget = TokenBudget::new(1000, 100, 100);
        let mut history = ConversationHistory::new(ProviderId::OpenAI, budget);

        history.add_message(Message::user("Hello"));
        assert_eq!(history.len(), 1);
        assert!(history.token_count() > 0);
    }

    #[test]
    fn test_history_system_prompt() {
        let mut history = ConversationHistory::new(ProviderId::OpenAI, TokenBudget::default());

        history.set_system_prompt(Message::system("You are helpful"));
        assert!(history.system_prompt().is_some());

        let all = history.all_messages();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].role, MessageRole::System);
    }

    #[test]
    fn test_history_clear() {
        let mut history = ConversationHistory::new(ProviderId::OpenAI, TokenBudget::default());

        history.set_system_prompt(Message::system("System"));
        history.add_message(Message::user("User"));
        history.add_message(Message::assistant("Assistant"));

        history.clear();
        assert!(history.is_empty());
        assert!(history.system_prompt().is_some()); // System prompt preserved
    }

    #[test]
    fn test_sliding_window_truncation() {
        // Very small budget to trigger truncation
        let budget = TokenBudget::new(100, 0, 0);
        let mut history = ConversationHistory::new(ProviderId::OpenAI, budget);

        // Add many messages to trigger truncation
        for i in 0..20 {
            history.add_message(Message::user(format!("Message {i}")));
        }

        // Should have truncated oldest messages
        assert!(history.len() < 20);
        assert!(history.token_count() <= 100);
    }

    #[test]
    fn test_all_messages_order() {
        let mut history = ConversationHistory::new(ProviderId::OpenAI, TokenBudget::default());

        history.set_system_prompt(Message::system("System"));
        history.add_message(Message::user("User"));
        history.add_message(Message::assistant("Assistant"));

        let all = history.all_messages();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].role, MessageRole::System);
        assert_eq!(all[1].role, MessageRole::User);
        assert_eq!(all[2].role, MessageRole::Assistant);
    }
}
