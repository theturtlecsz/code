use async_trait::async_trait;
use tokio::sync::mpsc;

pub mod types;
pub mod context;
pub mod stream;
pub mod claude;
pub mod gemini;
pub mod prompt_detector;
pub mod gemini_pty;
pub mod gemini_pipes;
pub mod claude_pipes;

pub use types::*;
pub use claude::{ClaudeCliExecutor, ClaudeCliConfig};
pub use gemini::{GeminiCliExecutor, GeminiCliConfig};
pub use context::CliContextManager;
pub use prompt_detector::PromptDetector;
pub use gemini_pty::{GeminiPtySession, GeminiPtyConfig, GeminiPtyProvider, SessionStats, ConversationId};
pub use gemini_pipes::{GeminiPipesSession, GeminiPipesConfig, GeminiPipesProvider};
pub use claude_pipes::{ClaudePipesSession, ClaudePipesConfig, ClaudePipesProvider, SessionInfo};

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
