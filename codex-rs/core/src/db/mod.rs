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
//! - Async wrappers for Tokio runtime integration

pub mod async_wrapper;
pub mod connection;
pub mod integration_examples;
pub mod migrations;
pub mod transactions;
pub mod vacuum;

// Sync API
pub use connection::initialize_pool;
pub use transactions::{batch_insert, execute_in_transaction, upsert_consensus_run};
pub use vacuum::{
    VacuumStats, estimate_vacuum_savings, get_freelist_size, run_vacuum_cycle, spawn_vacuum_daemon,
};

// Async API (Week 2 Day 3-4)
pub use async_wrapper::{
    query_consensus_runs, store_agent_output, store_consensus_run, with_connection,
};

// Integration Examples (Week 2 Day 3-4)
// See integration_examples module for TUI integration patterns

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
