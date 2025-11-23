use async_trait::async_trait;
use tokio::sync::mpsc;

pub mod claude;
pub mod claude_pipes;
pub mod context;
pub mod gemini;
pub mod gemini_pipes;
pub mod gemini_pty;
pub mod prompt_detector;
pub mod stream;
pub mod types;

pub use claude::{ClaudeCliConfig, ClaudeCliExecutor};
pub use claude_pipes::{ClaudePipesConfig, ClaudePipesProvider, ClaudePipesSession, SessionInfo};
pub use context::CliContextManager;
pub use gemini::{GeminiCliConfig, GeminiCliExecutor};
pub use gemini_pipes::{GeminiPipesConfig, GeminiPipesProvider, GeminiPipesSession};
pub use gemini_pty::{
    ConversationId, GeminiPtyConfig, GeminiPtyProvider, GeminiPtySession, SessionStats,
};
pub use prompt_detector::PromptDetector;
pub use types::*;

/// Core trait for CLI-based model executors
///
/// Implementations spawn external CLI processes (claude, gemini) and manage
/// request/response lifecycle through stdin/stdout.
#[async_trait]
pub trait CliExecutor: Send + Sync {
    /// Execute a request with conversation history
    ///
    /// Returns a channel that streams response events (deltas, metadata, completion).
    /// The executor formats history, spawns the CLI, writes the prompt, and parses output.
    async fn execute(
        &self,
        conversation: &Conversation,
        user_message: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError>;

    /// Check if CLI is available and authenticated
    ///
    /// Runs a lightweight command (e.g., `--version`) to verify:
    /// - Binary exists and is executable
    /// - User is authenticated (if required by provider)
    async fn health_check(&self) -> Result<(), CliError>;

    /// Estimate token count for validation
    ///
    /// Uses heuristic (char_count / 4 for prose, / 3 for code) to avoid
    /// hitting context limits. Not precise but sufficient for MVP.
    fn estimate_tokens(&self, conversation: &Conversation) -> usize;
}
