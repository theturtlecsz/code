//! Error types for spec-kit operations
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! Migrated from tui/src/chatwidget/spec_kit/error.rs (MAINT-10)

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

use crate::retry::classifier::{
    DegradedError, ErrorClass, PermanentError, RetryClassifiable, RetryableError,
};

/// Spec-kit result type alias
pub type Result<T> = std::result::Result<T, SpecKitError>;

/// Spec-kit error taxonomy
#[derive(Debug, Error)]
pub enum SpecKitError {
    #[error("Failed to write file {path}: {source}")]
    FileWrite {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to read file {path}: {source}")]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to read directory {path}: {source}")]
    DirectoryRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to create directory {path}: {source}")]
    DirectoryCreate {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to serialize JSON: {source}")]
    JsonSerialize { source: serde_json::Error },

    #[error("Failed to deserialize JSON: {source}")]
    JsonDeserialize { source: serde_json::Error },

    #[error("No consensus found for {spec_id} stage {stage} in {directory:?}")]
    NoConsensusFound {
        spec_id: String,
        stage: String,
        directory: PathBuf,
    },

    #[error("Agent execution failed: expected {expected:?}, completed {completed:?}")]
    AgentExecutionFailed {
        expected: Vec<String>,
        completed: Vec<String>,
    },

    #[error("MCP call failed: {0}")]
    McpCallFailed(String),

    #[error("Invalid SPEC ID format: {0}")]
    InvalidSpecId(String),

    #[error("Stage {stage} not valid for operation")]
    InvalidStage { stage: String },

    #[error("Configuration validation failed: {0}")]
    ConfigValidation(String),

    #[error("Evidence repository error: {0}")]
    EvidenceRepository(String),

    #[error("{0}")]
    Other(String),
}

impl From<String> for SpecKitError {
    fn from(s: String) -> Self {
        SpecKitError::Other(s)
    }
}

impl From<&str> for SpecKitError {
    fn from(s: &str) -> Self {
        SpecKitError::Other(s.to_string())
    }
}

// ============================================================================
// SPEC-945C: Error Classification for Retry Logic
// ============================================================================

impl RetryClassifiable for SpecKitError {
    fn classify(&self) -> ErrorClass {
        match self {
            // File I/O errors: classify based on underlying error kind
            SpecKitError::FileWrite { source, .. }
            | SpecKitError::FileRead { source, .. }
            | SpecKitError::DirectoryRead { source, .. }
            | SpecKitError::DirectoryCreate { source, .. } => classify_io_error(source),

            // MCP calls: classify based on error message
            SpecKitError::McpCallFailed(msg) => classify_mcp_error(msg),

            // Agent execution: degraded if partial success
            SpecKitError::AgentExecutionFailed {
                expected,
                completed,
            } => {
                if !completed.is_empty() && completed.len() < expected.len() {
                    ErrorClass::Degraded(DegradedError::DegradedConsensus {
                        success: completed.len(),
                        total: expected.len(),
                    })
                } else {
                    // Total failure - could be retryable network issue
                    ErrorClass::Retryable(RetryableError::ServiceUnavailable)
                }
            }

            // Consensus not found: degraded state (partial consensus possible)
            SpecKitError::NoConsensusFound { .. } => {
                ErrorClass::Degraded(DegradedError::DegradedConsensus {
                    success: 0,
                    total: 1,
                })
            }

            // Validation errors: permanent (won't fix themselves)
            SpecKitError::JsonSerialize { .. }
            | SpecKitError::JsonDeserialize { .. }
            | SpecKitError::InvalidSpecId(_)
            | SpecKitError::InvalidStage { .. }
            | SpecKitError::ConfigValidation(_) => {
                ErrorClass::Permanent(PermanentError::InvalidInput {
                    field: "input".to_string(),
                    reason: self.to_string(),
                })
            }

            // Evidence repository: could be transient (disk I/O)
            SpecKitError::EvidenceRepository(msg) => {
                if msg.to_lowercase().contains("lock") || msg.to_lowercase().contains("busy") {
                    ErrorClass::Retryable(RetryableError::DatabaseLocked)
                } else {
                    ErrorClass::Permanent(PermanentError::InvalidInput {
                        field: "evidence".to_string(),
                        reason: msg.clone(),
                    })
                }
            }

            // Generic errors: parse message for clues
            SpecKitError::Other(msg) => classify_generic_error(msg),
        }
    }

    fn suggested_backoff(&self) -> Option<Duration> {
        match self.classify() {
            ErrorClass::Retryable(ref err) => match err {
                RetryableError::RateLimitExceeded { retry_after } => {
                    Some(Duration::from_secs(*retry_after))
                }
                RetryableError::DatabaseLocked => {
                    // SQLite lock contention: short backoff (100-500ms)
                    Some(Duration::from_millis(200))
                }
                RetryableError::NetworkTimeout(_) | RetryableError::ConnectionRefused => {
                    // Network issues: use default exponential backoff
                    None
                }
                RetryableError::ServiceUnavailable => {
                    // Service unavailable: moderate backoff
                    Some(Duration::from_secs(5))
                }
            },
            ErrorClass::Permanent(_) => None, // Don't retry
            ErrorClass::Degraded(_) => {
                // Degraded consensus: short backoff, might resolve
                Some(Duration::from_millis(500))
            }
        }
    }
}

// Helper: Classify std::io::Error based on ErrorKind
fn classify_io_error(err: &std::io::Error) -> ErrorClass {
    use std::io::ErrorKind;

    match err.kind() {
        // Transient errors (retry recommended)
        ErrorKind::TimedOut | ErrorKind::Interrupted | ErrorKind::WouldBlock => {
            ErrorClass::Retryable(RetryableError::NetworkTimeout(30))
        }
        ErrorKind::ConnectionRefused
        | ErrorKind::ConnectionReset
        | ErrorKind::ConnectionAborted => ErrorClass::Retryable(RetryableError::ConnectionRefused),

        // Permanent errors (won't fix themselves)
        ErrorKind::NotFound => {
            ErrorClass::Permanent(PermanentError::ResourceNotFound(err.to_string()))
        }
        ErrorKind::PermissionDenied => {
            ErrorClass::Permanent(PermanentError::AuthenticationFailed(err.to_string()))
        }
        ErrorKind::InvalidInput | ErrorKind::InvalidData => {
            ErrorClass::Permanent(PermanentError::InvalidInput {
                field: "io".to_string(),
                reason: err.to_string(),
            })
        }

        // Other I/O errors: retry with caution
        _ => ErrorClass::Retryable(RetryableError::ServiceUnavailable),
    }
}

// Helper: Classify MCP errors based on message content
fn classify_mcp_error(msg: &str) -> ErrorClass {
    let msg_lower = msg.to_lowercase();

    // Rate limit detection
    if msg_lower.contains("rate limit")
        || msg_lower.contains("429")
        || msg_lower.contains("too many requests")
    {
        // Try to parse Retry-After from message (e.g., "Retry-After: 60")
        let retry_after = parse_retry_after(msg).unwrap_or(60);
        return ErrorClass::Retryable(RetryableError::RateLimitExceeded { retry_after });
    }

    // Database lock detection
    if msg_lower.contains("sqlite_busy")
        || msg_lower.contains("sqlite_locked")
        || msg_lower.contains("database is locked")
    {
        return ErrorClass::Retryable(RetryableError::DatabaseLocked);
    }

    // Network timeout detection
    if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
        return ErrorClass::Retryable(RetryableError::NetworkTimeout(30));
    }

    // Service unavailable
    if msg_lower.contains("503") || msg_lower.contains("unavailable") {
        return ErrorClass::Retryable(RetryableError::ServiceUnavailable);
    }

    // Authentication failures
    if msg_lower.contains("auth")
        || msg_lower.contains("unauthorized")
        || msg_lower.contains("401")
        || msg_lower.contains("403")
    {
        return ErrorClass::Permanent(PermanentError::AuthenticationFailed(msg.to_string()));
    }

    // Not found
    if msg_lower.contains("not found") || msg_lower.contains("404") {
        return ErrorClass::Permanent(PermanentError::ResourceNotFound(msg.to_string()));
    }

    // Default: assume retryable (conservative)
    ErrorClass::Retryable(RetryableError::ServiceUnavailable)
}

// Helper: Classify generic errors based on message
fn classify_generic_error(msg: &str) -> ErrorClass {
    // Reuse MCP classification logic
    classify_mcp_error(msg)
}

// Helper: Parse Retry-After value from error message
fn parse_retry_after(msg: &str) -> Option<u64> {
    // Look for patterns like "Retry-After: 60" or "retry after 60s"
    let msg_lower = msg.to_lowercase();

    // Try to find "retry-after" or "retry after" (with hyphen or space)
    let search_patterns = ["retry-after", "retry after"];

    for pattern in search_patterns {
        if let Some(pos) = msg_lower.find(pattern) {
            let after_pos = pos + pattern.len();
            let remainder = &msg[after_pos..];

            // Extract first number found
            if let Ok(num) = remainder
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u64>()
            {
                return Some(num.clamp(30, 120)); // Clamp to 30-120s
            }
        }
    }

    None
}

// === SPEC-945C Day 4-5: RetryClassifiable implementation for rusqlite::Error ===
//
// Enables retry logic for direct SQLite operations in consensus_db.rs.
// Classifies SQLITE_BUSY and SQLITE_LOCKED as retryable, all others as permanent.
impl RetryClassifiable for rusqlite::Error {
    fn classify(&self) -> ErrorClass {
        match self {
            rusqlite::Error::SqliteFailure(err, msg) => match err.code {
                rusqlite::ErrorCode::DatabaseBusy => {
                    ErrorClass::Retryable(RetryableError::DatabaseLocked)
                }
                rusqlite::ErrorCode::DatabaseLocked => {
                    ErrorClass::Retryable(RetryableError::DatabaseLocked)
                }
                _ => ErrorClass::Permanent(PermanentError::DatabaseError(
                    msg.clone()
                        .unwrap_or_else(|| format!("SQLite error: {:?}", err.code)),
                )),
            },
            _ => ErrorClass::Permanent(PermanentError::DatabaseError(self.to_string())),
        }
    }

    fn suggested_backoff(&self) -> Option<Duration> {
        match self {
            rusqlite::Error::SqliteFailure(err, _) => match err.code {
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => {
                    Some(Duration::from_millis(200))
                }
                _ => None,
            },
            _ => None,
        }
    }
}

// === SPEC-945C Day 4-5: RetryClassifiable implementation for codex_core::db::DbError ===
//
// Enables retry logic for async database operations that return DbError.
// Delegates to rusqlite::Error for Sqlite variant, treats others as permanent.
impl RetryClassifiable for codex_core::db::DbError {
    fn classify(&self) -> ErrorClass {
        match self {
            codex_core::db::DbError::Sqlite(err) => err.classify(),
            _ => ErrorClass::Permanent(PermanentError::DatabaseError(self.to_string())),
        }
    }

    fn suggested_backoff(&self) -> Option<Duration> {
        match self {
            codex_core::db::DbError::Sqlite(err) => err.suggested_backoff(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_busy_retryable() {
        let err = SpecKitError::McpCallFailed("SQLITE_BUSY error".to_string());
        assert!(err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Retryable(RetryableError::DatabaseLocked)
        ));
        assert_eq!(err.suggested_backoff(), Some(Duration::from_millis(200)));
    }

    #[test]
    fn test_sqlite_locked_retryable() {
        let err = SpecKitError::McpCallFailed("database is locked".to_string());
        assert!(err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Retryable(RetryableError::DatabaseLocked)
        ));
        assert_eq!(err.suggested_backoff(), Some(Duration::from_millis(200)));
    }

    #[test]
    fn test_rate_limit_retryable() {
        let err = SpecKitError::McpCallFailed("Rate limit exceeded, Retry-After: 45".to_string());
        assert!(err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Retryable(RetryableError::RateLimitExceeded { retry_after: 45 })
        ));
        assert_eq!(err.suggested_backoff(), Some(Duration::from_secs(45)));
    }

    #[test]
    fn test_rate_limit_http_429() {
        let err = SpecKitError::McpCallFailed("HTTP 429 Too Many Requests".to_string());
        assert!(err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Retryable(RetryableError::RateLimitExceeded { retry_after: 60 })
        ));
        assert_eq!(err.suggested_backoff(), Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_network_timeout_retryable() {
        let err = SpecKitError::McpCallFailed("Request timed out after 30s".to_string());
        assert!(err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Retryable(RetryableError::NetworkTimeout(30))
        ));
        assert_eq!(err.suggested_backoff(), None); // Use default backoff
    }

    #[test]
    fn test_auth_failure_permanent() {
        let err = SpecKitError::McpCallFailed("Authentication failed: invalid token".to_string());
        assert!(!err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Permanent(PermanentError::AuthenticationFailed(_))
        ));
        assert_eq!(err.suggested_backoff(), None);
    }

    #[test]
    fn test_invalid_input_permanent() {
        let err = SpecKitError::InvalidSpecId("not-a-valid-id".to_string());
        assert!(!err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Permanent(PermanentError::InvalidInput { .. })
        ));
        assert_eq!(err.suggested_backoff(), None);
    }

    #[test]
    fn test_resource_not_found_permanent() {
        let err = SpecKitError::McpCallFailed("Resource not found: /api/missing".to_string());
        assert!(!err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Permanent(PermanentError::ResourceNotFound(_))
        ));
        assert_eq!(err.suggested_backoff(), None);
    }

    #[test]
    fn test_degraded_consensus_handling() {
        let err = SpecKitError::AgentExecutionFailed {
            expected: vec![
                "agent1".to_string(),
                "agent2".to_string(),
                "agent3".to_string(),
            ],
            completed: vec!["agent1".to_string(), "agent2".to_string()],
        };
        assert!(!err.is_retryable()); // Degraded, not retryable
        assert!(matches!(
            err.classify(),
            ErrorClass::Degraded(DegradedError::DegradedConsensus {
                success: 2,
                total: 3
            })
        ));
        assert_eq!(err.suggested_backoff(), Some(Duration::from_millis(500)));
    }

    #[test]
    fn test_parse_retry_after_variants() {
        // Test different Retry-After formats
        assert_eq!(parse_retry_after("Retry-After: 45"), Some(45));
        assert_eq!(parse_retry_after("retry after 75s"), Some(75));
        assert_eq!(
            parse_retry_after("please retry after 150 seconds"),
            Some(120)
        ); // Clamped
        assert_eq!(parse_retry_after("retry after 15 seconds"), Some(30)); // Clamped
        assert_eq!(parse_retry_after("no number here"), None);
    }

    #[test]
    fn test_io_error_classification() {
        use std::io::{Error, ErrorKind};

        // Timeout -> Retryable
        let err = SpecKitError::FileRead {
            path: "/tmp/test".into(),
            source: Error::new(ErrorKind::TimedOut, "timeout"),
        };
        assert!(err.is_retryable());

        // NotFound -> Permanent
        let err = SpecKitError::FileRead {
            path: "/tmp/test".into(),
            source: Error::new(ErrorKind::NotFound, "not found"),
        };
        assert!(!err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Permanent(PermanentError::ResourceNotFound(_))
        ));

        // PermissionDenied -> Permanent
        let err = SpecKitError::FileWrite {
            path: "/tmp/test".into(),
            source: Error::new(ErrorKind::PermissionDenied, "denied"),
        };
        assert!(!err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Permanent(PermanentError::AuthenticationFailed(_))
        ));
    }

    #[test]
    fn test_evidence_repository_lock() {
        let err = SpecKitError::EvidenceRepository("Database locked".to_string());
        assert!(err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Retryable(RetryableError::DatabaseLocked)
        ));
    }

    #[test]
    fn test_agent_total_failure_retryable() {
        // All agents failed -> retryable (could be network issue)
        let err = SpecKitError::AgentExecutionFailed {
            expected: vec!["agent1".to_string(), "agent2".to_string()],
            completed: vec![],
        };
        assert!(err.is_retryable());
        assert!(matches!(
            err.classify(),
            ErrorClass::Retryable(RetryableError::ServiceUnavailable)
        ));
    }
}
