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
    /// Results array. Uses custom deserializer to handle `null` as empty vec.
    /// The local-memory CLI returns `"results": null` when no matches found.
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub results: Vec<LocalMemorySearchResult>,
}

/// Deserialize null or missing array as empty vec
fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt: Option<Vec<T>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC-DOGFOOD-001: Verify null results array is handled correctly
    /// The local-memory CLI returns `"results": null` when no matches found
    #[test]
    fn test_null_results_array_handled() {
        let json = r#"{"success":true,"data":{"query":"*","result_count":0,"results":null}}"#;
        let parsed: LocalMemorySearchResponse =
            serde_json::from_str(json).expect("should parse JSON with null results");

        assert!(parsed.success);
        assert!(parsed.data.is_some());
        assert!(parsed.data.unwrap().results.is_empty());
    }

    /// Verify empty array still works
    #[test]
    fn test_empty_results_array() {
        let json = r#"{"success":true,"data":{"results":[]}}"#;
        let parsed: LocalMemorySearchResponse =
            serde_json::from_str(json).expect("should parse JSON with empty results");

        assert!(parsed.data.unwrap().results.is_empty());
    }

    /// Verify actual results still work
    #[test]
    fn test_populated_results_array() {
        let json = r#"{"success":true,"data":{"results":[{"memory":{"content":"test"},"relevance_score":0.95}]}}"#;
        let parsed: LocalMemorySearchResponse =
            serde_json::from_str(json).expect("should parse JSON with results");

        let results = parsed.data.unwrap().results;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.content, "test");
    }
}
