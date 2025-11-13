//! Database layer for codex-rs
//!
//! SPEC-945B: SQLite optimization, ACID transactions, connection pooling
//!
//! This module provides:
//! - Connection pooling (r2d2-sqlite)
//! - ACID transaction helpers
//! - Schema migrations
//! - Auto-vacuum scheduling
//! - WAL mode + performance pragmas

// TODO: Implementation in Phase 1, Week 1

pub mod connection;
pub mod migrations;
pub mod transactions;
pub mod vacuum;

pub use connection::initialize_pool;
pub use transactions::{batch_insert, execute_in_transaction};
pub use vacuum::{VacuumStats, spawn_vacuum_daemon};

/// Database module result type
pub type Result<T> = std::result::Result<T, DbError>;

/// Database error types
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Connection pool error: {0}")]
    Pool(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Migration error: {0}")]
    Migration(String),
}
