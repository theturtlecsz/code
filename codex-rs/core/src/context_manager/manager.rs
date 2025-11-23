//! Context Manager - central orchestrator for conversation context

use super::{
    ConversationHistory, Message, ProviderId, TokenBudget, TruncationStrategy,
    persistence::SessionManager, serializer, tokenizer,
};
use serde_json::Value;
use std::path::PathBuf;

/// Context Manager - central orchestrator for conversation context
///
/// Provides a high-level API for managing conversation history across
/// multiple providers with token tracking, truncation, and persistence.
pub struct ContextManager {
    /// Current conversation history
    history: ConversationHistory,

    /// Session persistence manager
    session_manager: SessionManager,

    /// Current session ID (if persisted)
    session_id: Option<String>,
}

impl ContextManager {
    /// Create new context manager for a provider
    pub fn new(provider: ProviderId, model: &str, codex_home: impl Into<PathBuf>) -> Self {
        let codex_home = codex_home.into();
        let context_window = tokenizer::get_context_window(provider, model);

        let budget = TokenBudget {
            max_context_tokens: context_window,
            system_prompt_reserved: 2_000,
            response_reserved: 4_000,
        };

        Self {
            history: ConversationHistory::new(provider, budget),
            session_manager: SessionManager::new(codex_home),
            session_id: None,
        }
    }

    /// Create context manager with custom token budget
    pub fn with_budget(
        provider: ProviderId,
        budget: TokenBudget,
        codex_home: impl Into<PathBuf>,
    ) -> Self {
        Self {
            history: ConversationHistory::new(provider, budget),
            session_manager: SessionManager::new(codex_home.into()),
            session_id: None,
        }
    }

    /// Set system prompt
    pub fn set_system_prompt(&mut self, text: impl Into<String>) {
        self.history.set_system_prompt(Message::system(text));
    }

    /// Get system prompt
    pub fn system_prompt(&self) -> Option<&Message> {
        self.history.system_prompt()
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

    /// Get available tokens before truncation
    pub fn tokens_available(&self) -> usize {
        self.history.tokens_available()
    }

    /// Get token usage as percentage of budget
    pub fn token_usage_percent(&self) -> f32 {
        let budget = self.history.budget().available_for_history();
        if budget == 0 {
            return 100.0;
        }
        (self.history.token_count() as f32 / budget as f32) * 100.0
    }

    /// Clear conversation (keep system prompt)
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Set truncation strategy
    pub fn set_truncation_strategy(&mut self, strategy: TruncationStrategy) {
        self.history.set_truncation_strategy(strategy);
    }

    /// Save current session
    pub fn save_session(&self, session_id: &str) -> std::io::Result<()> {
        self.session_manager.save_session(&self.history, session_id)
    }

    /// Save current session (auto-generate ID if not set)
    pub fn save(&mut self) -> std::io::Result<String> {
        let id = self
            .session_id
            .clone()
            .unwrap_or_else(|| generate_session_id());
        self.save_session(&id)?;
        self.session_id = Some(id.clone());
        Ok(id)
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

    /// Check if session exists
    pub fn session_exists(&self, session_id: &str) -> bool {
        self.session_manager.session_exists(session_id)
    }

    /// Get current session ID
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Set session ID
    pub fn set_session_id(&mut self, id: impl Into<String>) {
        self.session_id = Some(id.into());
    }

    /// Get message count (excluding system prompt)
    pub fn message_count(&self) -> usize {
        self.history.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Get provider
    pub fn provider(&self) -> ProviderId {
        self.history.provider()
    }

    /// Get all messages (including system prompt)
    pub fn all_messages(&self) -> Vec<&Message> {
        self.history.all_messages()
    }

    /// Get conversation messages only (no system prompt)
    pub fn messages(&self) -> &[Message] {
        self.history.messages()
    }

    /// Get the underlying history (for advanced use)
    pub fn history(&self) -> &ConversationHistory {
        &self.history
    }

    /// Get mutable reference to history (for advanced use)
    pub fn history_mut(&mut self) -> &mut ConversationHistory {
        &mut self.history
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let random: u32 = rand::random();
    format!("{}-{:08x}", timestamp, random)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_context_manager(provider: ProviderId) -> (ContextManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = ContextManager::new(provider, "gpt-4", temp_dir.path());
        (manager, temp_dir)
    }

    #[test]
    fn test_basic_usage() {
        let (mut manager, _dir) = temp_context_manager(ProviderId::OpenAI);

        manager.set_system_prompt("You are helpful");
        manager.add_user_message("Hello");
        manager.add_assistant_message("Hi!");

        assert_eq!(manager.message_count(), 2);
        assert!(manager.system_prompt().is_some());
    }

    #[test]
    fn test_serialize_for_request() {
        let (mut manager, _dir) = temp_context_manager(ProviderId::OpenAI);

        manager.set_system_prompt("System");
        manager.add_user_message("User");

        let serialized = manager.serialize_for_request();
        let arr = serialized.as_array().unwrap();

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["role"], "system");
        assert_eq!(arr[1]["role"], "user");
    }

    #[test]
    fn test_token_tracking() {
        let (mut manager, _dir) = temp_context_manager(ProviderId::OpenAI);

        assert_eq!(manager.token_count(), 0);

        manager.add_user_message("Hello, world!");
        assert!(manager.token_count() > 0);
    }

    #[test]
    fn test_clear_preserves_system() {
        let (mut manager, _dir) = temp_context_manager(ProviderId::OpenAI);

        manager.set_system_prompt("System");
        manager.add_user_message("User");
        manager.clear();

        assert!(manager.is_empty());
        assert!(manager.system_prompt().is_some());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();

        // Create and save
        {
            let mut manager =
                ContextManager::new(ProviderId::Anthropic, "claude-3", temp_dir.path());
            manager.set_system_prompt("Be helpful");
            manager.add_user_message("Test");
            manager.save_session("test-save").unwrap();
        }

        // Load into new manager
        {
            let mut manager =
                ContextManager::new(ProviderId::Anthropic, "claude-3", temp_dir.path());
            manager.load_session("test-save").unwrap();

            assert_eq!(manager.message_count(), 1);
            assert!(manager.system_prompt().is_some());
            assert_eq!(manager.session_id(), Some("test-save"));
        }
    }

    #[test]
    fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = ContextManager::new(ProviderId::OpenAI, "gpt-4", temp_dir.path());

        manager.add_user_message("One");
        manager.save_session("session-1").unwrap();

        manager.clear();
        manager.add_user_message("Two");
        manager.save_session("session-2").unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_auto_save() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = ContextManager::new(ProviderId::OpenAI, "gpt-4", temp_dir.path());

        manager.add_user_message("Test");
        let id = manager.save().unwrap();

        assert!(manager.session_exists(&id));
        assert_eq!(manager.session_id(), Some(id.as_str()));
    }

    #[test]
    fn test_token_usage_percent() {
        let budget = TokenBudget::new(1000, 0, 0);
        let temp_dir = TempDir::new().unwrap();
        let mut manager = ContextManager::with_budget(ProviderId::OpenAI, budget, temp_dir.path());

        assert_eq!(manager.token_usage_percent(), 0.0);

        // Add some messages
        manager.add_user_message("Hello world");
        assert!(manager.token_usage_percent() > 0.0);
        assert!(manager.token_usage_percent() < 100.0);
    }

    #[test]
    fn test_provider_specific_serialization() {
        // OpenAI
        let (mut openai_manager, _d1) = temp_context_manager(ProviderId::OpenAI);
        openai_manager.add_user_message("Test");
        let openai_json = openai_manager.serialize_for_request();
        assert!(openai_json.is_array());

        // Anthropic
        let temp_dir = TempDir::new().unwrap();
        let mut anthropic_manager =
            ContextManager::new(ProviderId::Anthropic, "claude-3", temp_dir.path());
        anthropic_manager.add_user_message("Test");
        let anthropic_json = anthropic_manager.serialize_for_request();
        assert!(anthropic_json.is_object());
        assert!(anthropic_json.get("messages").is_some());

        // Google
        let temp_dir2 = TempDir::new().unwrap();
        let mut google_manager =
            ContextManager::new(ProviderId::Google, "gemini-pro", temp_dir2.path());
        google_manager.add_user_message("Test");
        let google_json = google_manager.serialize_for_request();
        assert!(google_json.is_object());
        assert!(google_json.get("contents").is_some());
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert!(!id1.is_empty());
        assert_ne!(id1, id2); // Should be unique
        assert!(id1.contains('-')); // Format: timestamp-random
    }
}
