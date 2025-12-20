//! Local-memory utility types for MCP response parsing
//!
//! STATUS (2025-10-18): ARCH-004 migration complete
//! - ✅ Types: Used by MCP native calls (spec_prompts.rs, consensus.rs)
//! - ✅ Subprocess functions deleted (migrated to native MCP)
//!
//! Local-memory access in this repo may use CLI/REST or MCP, depending on the
//! integration point.

#![allow(dead_code)] // Response types used by MCP parsing

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LocalMemorySearchResponse {
    pub success: bool,
    #[serde(default)]
    pub data: Option<LocalMemorySearchData>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LocalMemorySearchData {
    #[serde(default)]
    pub results: Vec<LocalMemorySearchResult>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LocalMemorySearchResult {
    pub memory: LocalMemoryRecord,
    #[serde(default)]
    pub relevance_score: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LocalMemoryRecord {
    #[serde(default)]
    pub id: Option<String>,
    pub content: String,
    #[serde(default)]
    pub importance: Option<u8>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}
