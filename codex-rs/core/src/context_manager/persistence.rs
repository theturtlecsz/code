//! Session persistence for conversation history

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
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Serializable token budget configuration
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
    pub fn new(codex_home: impl AsRef<Path>) -> Self {
        let sessions_dir = codex_home.as_ref().join("sessions");
        Self { sessions_dir }
    }

    /// Save session to disk
    pub fn save_session(
        &self,
        history: &ConversationHistory,
        session_id: &str,
    ) -> std::io::Result<()> {
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
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Atomic write: write to temp file then rename
        let temp_path = path.with_extension("json.tmp");
        std::fs::write(&temp_path, json)?;
        std::fs::rename(temp_path, path)?;

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
        history.recalculate_tokens();

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
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false)
                && let Some(name) = path.file_stem() {
                    sessions.push(name.to_string_lossy().to_string());
                }
        }

        // Sort by name (which often includes timestamp)
        sessions.sort();

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

    /// Check if session exists
    pub fn session_exists(&self, session_id: &str) -> bool {
        self.session_path(session_id).exists()
    }

    /// Get path for a session file
    fn session_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{session_id}.json"))
    }

    /// Get sessions directory path
    pub fn sessions_dir(&self) -> &Path {
        &self.sessions_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_history() -> ConversationHistory {
        let budget = TokenBudget::new(10_000, 500, 1000);
        let mut history = ConversationHistory::new(ProviderId::OpenAI, budget);
        history.set_system_prompt(Message::system("You are helpful"));
        history.add_message(Message::user("Hello"));
        history.add_message(Message::assistant("Hi there!"));
        history
    }

    #[test]
    fn test_save_and_load_session() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path());

        let history = create_test_history();
        manager.save_session(&history, "test-session").unwrap();

        let loaded = manager.load_session("test-session").unwrap();
        assert_eq!(loaded.provider(), ProviderId::OpenAI);
        assert_eq!(loaded.len(), 2);
        assert!(loaded.system_prompt().is_some());
    }

    #[test]
    fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path());

        // Empty initially
        assert!(manager.list_sessions().unwrap().is_empty());

        // Add sessions
        let history = create_test_history();
        manager.save_session(&history, "session-1").unwrap();
        manager.save_session(&history, "session-2").unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&"session-1".to_string()));
        assert!(sessions.contains(&"session-2".to_string()));
    }

    #[test]
    fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path());

        let history = create_test_history();
        manager.save_session(&history, "to-delete").unwrap();
        assert!(manager.session_exists("to-delete"));

        manager.delete_session("to-delete").unwrap();
        assert!(!manager.session_exists("to-delete"));
    }

    #[test]
    fn test_delete_nonexistent_session() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path());

        // Should not error
        manager.delete_session("nonexistent").unwrap();
    }

    #[test]
    fn test_session_state_serialization() {
        let history = create_test_history();
        let state = SessionState {
            session_id: "test".to_string(),
            provider: history.provider,
            system_prompt: history.system_prompt.clone(),
            messages: history.messages.clone(),
            budget: history.budget.clone().into(),
            truncation_strategy: "SlidingWindow".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: SessionState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.session_id, "test");
        assert_eq!(parsed.provider, ProviderId::OpenAI);
        assert_eq!(parsed.messages.len(), 2);
    }

    #[test]
    fn test_token_budget_config_conversion() {
        let budget = TokenBudget::new(100_000, 2_000, 4_000);
        let config: TokenBudgetConfig = budget.clone().into();
        let restored: TokenBudget = config.into();

        assert_eq!(budget.max_context_tokens, restored.max_context_tokens);
        assert_eq!(
            budget.system_prompt_reserved,
            restored.system_prompt_reserved
        );
        assert_eq!(budget.response_reserved, restored.response_reserved);
    }

    #[test]
    fn test_truncation_strategy_preservation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path());

        let budget = TokenBudget::default();
        let mut history = ConversationHistory::new(ProviderId::Anthropic, budget);
        history.set_truncation_strategy(TruncationStrategy::PriorityBased);
        history.add_message(Message::user("Test"));

        manager.save_session(&history, "strategy-test").unwrap();
        let loaded = manager.load_session("strategy-test").unwrap();

        assert_eq!(
            loaded.truncation_strategy,
            TruncationStrategy::PriorityBased
        );
    }
}
