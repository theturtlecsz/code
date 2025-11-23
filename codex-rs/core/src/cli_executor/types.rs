use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Delta(String),              // Incremental text
    Metadata(ResponseMetadata), // Token usage, model info
    Done,                       // Response complete
    Error(CliError),            // Error occurred
}

#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    pub model: String,
    pub input_tokens: Option<usize>,
    pub output_tokens: Option<usize>,
}

#[derive(Error, Debug, Clone)]
pub enum CliError {
    #[error("CLI binary not found: {binary}. Install via: {install_hint}")]
    BinaryNotFound {
        binary: String,
        install_hint: String,
    },

    #[error("CLI not authenticated. Run: {auth_command}")]
    NotAuthenticated { auth_command: String },

    #[error("CLI process failed with code {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    #[error("Timeout after {elapsed:?}")]
    Timeout { elapsed: std::time::Duration },

    #[error("Parse error: {details}")]
    ParseError { details: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}
