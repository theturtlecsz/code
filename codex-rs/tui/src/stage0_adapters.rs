//! Stage0 adapter implementations for codex-rs
//!
//! This module provides concrete implementations of the Stage0 trait abstractions:
//! - `LocalMemoryMcpAdapter`: Wraps MCP `local-memory` server
//! - `LlmStubAdapter`: Stub LLM client (uses heuristics, no actual LLM calls in V1)
//! - `Tier2McpAdapter`: Wraps MCP `notebooklm` server
//!
//! These adapters bridge Stage0's trait-based design with codex-rs's MCP infrastructure.

use async_trait::async_trait;
use codex_core::mcp_connection_manager::McpConnectionManager;
use serde_json::json;
use codex_stage0::dcc::{EnvCtx, Iqo, LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary};
use codex_stage0::errors::{Result, Stage0Error};
use codex_stage0::guardians::{LlmClient, MemoryKind};
use codex_stage0::tier2::{CausalLinkSuggestion, Tier2Client, Tier2Response};
use std::sync::Arc;
use std::time::Duration;

// ─────────────────────────────────────────────────────────────────────────────
// MCP Server Names
// ─────────────────────────────────────────────────────────────────────────────

/// MCP server name for local-memory
const LOCAL_MEMORY_SERVER: &str = "local-memory";

/// MCP server name for NotebookLM
const NOTEBOOKLM_SERVER: &str = "notebooklm";

/// Default timeout for MCP tool calls
const DEFAULT_MCP_TIMEOUT: Duration = Duration::from_secs(30);

// ─────────────────────────────────────────────────────────────────────────────
// LocalMemoryMcpAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter that implements `LocalMemoryClient` using MCP's local-memory server
pub struct LocalMemoryMcpAdapter {
    mcp_manager: Arc<McpConnectionManager>,
}

impl LocalMemoryMcpAdapter {
    /// Create a new adapter wrapping the MCP connection manager
    pub fn new(mcp_manager: Arc<McpConnectionManager>) -> Self {
        Self { mcp_manager }
    }
}

#[async_trait]
impl LocalMemoryClient for LocalMemoryMcpAdapter {
    async fn search_memories(
        &self,
        params: LocalMemorySearchParams,
    ) -> Result<Vec<LocalMemorySummary>> {
        // Build search arguments for mcp__local-memory__search
        let mut query_parts: Vec<String> = Vec::new();

        // Add keywords to query
        if !params.iqo.keywords.is_empty() {
            query_parts.push(params.iqo.keywords.join(" "));
        }

        // Add domains as part of query
        if !params.iqo.domains.is_empty() {
            query_parts.push(params.iqo.domains.join(" "));
        }

        let query = if query_parts.is_empty() {
            "*".to_string() // Fallback to wildcard search
        } else {
            query_parts.join(" ")
        };

        // Build tags array for filtering
        let mut tags: Vec<String> = params.iqo.required_tags.clone();
        tags.extend(params.iqo.optional_tags.iter().cloned());

        // Build search request
        let search_args = json!({
            "query": query,
            "search_type": "hybrid",  // Use hybrid for best results
            "use_ai": true,           // Enable semantic search
            "limit": params.max_results.min(100),
            "response_format": "detailed",
            "tags": if tags.is_empty() { None } else { Some(tags) },
            "domain": params.iqo.domains.first().cloned(),
        });

        // Call MCP tool
        let result = self
            .mcp_manager
            .call_tool(
                LOCAL_MEMORY_SERVER,
                "search",
                Some(search_args),
                Some(DEFAULT_MCP_TIMEOUT),
            )
            .await
            .map_err(|e| Stage0Error::local_memory(format!("MCP call failed: {e}")))?;

        // Parse response
        parse_local_memory_search_response(&result)
    }
}

/// Parse MCP search response into LocalMemorySummary vec
fn parse_local_memory_search_response(
    result: &mcp_types::CallToolResult,
) -> Result<Vec<LocalMemorySummary>> {
    // Extract text content from MCP result
    let text = result
        .content
        .iter()
        .filter_map(|c| match c {
            mcp_types::ContentBlock::TextContent(tc) => Some(tc.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    if text.is_empty() {
        return Ok(Vec::new());
    }

    // Parse JSON response
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| Stage0Error::local_memory(format!("Failed to parse response: {e}")))?;

    // Extract memories array
    let memories = json
        .get("memories")
        .or_else(|| json.get("results"))
        .and_then(|v| v.as_array())
        .unwrap_or(&Vec::new())
        .clone();

    let summaries: Vec<LocalMemorySummary> = memories
        .iter()
        .filter_map(|m| {
            let id = m.get("id")?.as_str()?.to_string();
            let content = m
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Truncate snippet to ~200 chars
            let snippet = if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content
            };

            let domain = m
                .get("domain")
                .and_then(|v| v.as_str())
                .map(String::from);

            let tags: Vec<String> = m
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let created_at = m
                .get("created_at")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));

            // Extract similarity score if available
            let similarity_score = m
                .get("similarity")
                .or_else(|| m.get("score"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5); // Default to middle score

            Some(LocalMemorySummary {
                id,
                domain,
                tags,
                created_at,
                snippet,
                similarity_score,
            })
        })
        .collect();

    Ok(summaries)
}

// ─────────────────────────────────────────────────────────────────────────────
// LlmStubAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Stub LLM client that uses heuristics instead of actual LLM calls
///
/// For V1, we rely on Stage0's built-in heuristic fallbacks. This adapter
/// returns errors that trigger those fallbacks, avoiding the complexity
/// of integrating with the full LLM infrastructure.
///
/// Future versions can implement proper LLM calls for:
/// - IQO generation (better query understanding)
/// - Memory classification (accurate kind detection)
/// - Template restructuring (consistent formatting)
pub struct LlmStubAdapter;

impl LlmStubAdapter {
    /// Create a new stub adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for LlmStubAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmClient for LlmStubAdapter {
    async fn classify_kind(&self, _input: &str) -> Result<MemoryKind> {
        // Return error to trigger heuristic fallback in Stage0
        // Stage0's apply_template_guardian_passthrough handles this gracefully
        Err(Stage0Error::prompt("LLM stub: using heuristic classification"))
    }

    async fn restructure_template(&self, input: &str, _kind: MemoryKind) -> Result<String> {
        // Pass through without restructuring
        // Stage0's apply_template_guardian_passthrough preserves original content
        Ok(input.to_string())
    }

    async fn generate_iqo(&self, _spec_content: &str, _env: &EnvCtx) -> Result<Iqo> {
        // Return error to trigger heuristic IQO generation in Stage0
        // Stage0's build_iqo falls back to heuristic_iqo on error
        Err(Stage0Error::prompt("LLM stub: using heuristic IQO"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tier2McpAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter that implements `Tier2Client` using MCP's NotebookLM server
pub struct Tier2McpAdapter {
    mcp_manager: Arc<McpConnectionManager>,
    /// Optional notebook ID to use (if None, uses active notebook)
    notebook_id: Option<String>,
}

impl Tier2McpAdapter {
    /// Create a new adapter wrapping the MCP connection manager
    pub fn new(mcp_manager: Arc<McpConnectionManager>) -> Self {
        Self {
            mcp_manager,
            notebook_id: None,
        }
    }

    /// Create adapter with a specific notebook ID
    pub fn with_notebook(mcp_manager: Arc<McpConnectionManager>, notebook_id: String) -> Self {
        Self {
            mcp_manager,
            notebook_id: Some(notebook_id),
        }
    }
}

#[async_trait]
impl Tier2Client for Tier2McpAdapter {
    async fn generate_divine_truth(
        &self,
        spec_id: &str,
        spec_content: &str,
        task_brief_md: &str,
    ) -> Result<Tier2Response> {
        // Build the Tier2 prompt using Stage0's helper
        let prompt = codex_stage0::build_tier2_prompt(spec_id, spec_content, task_brief_md);

        // Build MCP tool arguments
        let mut args = json!({
            "question": prompt,
        });

        // Add notebook_id if specified
        if let Some(ref nb_id) = self.notebook_id {
            args["notebook_id"] = json!(nb_id);
        }

        // Call NotebookLM via MCP
        let result = self
            .mcp_manager
            .call_tool(
                NOTEBOOKLM_SERVER,
                "ask_question",
                Some(args),
                Some(Duration::from_secs(120)), // Longer timeout for NotebookLM
            )
            .await
            .map_err(|e| Stage0Error::tier2(format!("NotebookLM MCP call failed: {e}")))?;

        // Parse response
        parse_tier2_response(&result, spec_id)
    }
}

/// Parse NotebookLM MCP response into Tier2Response
fn parse_tier2_response(
    result: &mcp_types::CallToolResult,
    spec_id: &str,
) -> Result<Tier2Response> {
    // Extract text content from MCP result
    let text = result
        .content
        .iter()
        .filter_map(|c| match c {
            mcp_types::ContentBlock::TextContent(tc) => Some(tc.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        return Err(Stage0Error::tier2("Empty response from NotebookLM"));
    }

    // Try to parse as JSON first (structured response)
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
        // Extract answer/response field
        let divine_truth_md = json
            .get("answer")
            .or_else(|| json.get("response"))
            .or_else(|| json.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or(&text)
            .to_string();

        // Try to parse causal links from response
        let suggested_links = parse_causal_links_from_markdown(&divine_truth_md);

        return Ok(Tier2Response {
            divine_truth_md,
            suggested_links,
        });
    }

    // Plain text response - wrap in Divine Truth format
    let divine_truth_md = if text.contains("# Divine Truth") || text.contains("## 1.") {
        // Already formatted as Divine Truth
        text
    } else {
        // Wrap raw response in minimal Divine Truth structure
        format!(
            "# Divine Truth Brief: {spec_id}\n\n## 1. Executive Summary\n\n{text}\n\n## 2. Architectural Guardrails\n\n_See task brief for context._\n\n## 3. Historical Context & Lessons\n\n_Derived from NotebookLM sources._\n\n## 4. Risks & Open Questions\n\n_Review task brief for implementation details._\n"
        )
    };

    // Parse causal links from the markdown
    let suggested_links = parse_causal_links_from_markdown(&divine_truth_md);

    Ok(Tier2Response {
        divine_truth_md,
        suggested_links,
    })
}

/// Parse causal link suggestions from Divine Truth markdown
///
/// Looks for Section 5 JSON block or inline JSON arrays.
fn parse_causal_links_from_markdown(markdown: &str) -> Vec<CausalLinkSuggestion> {
    // Look for JSON block in Section 5 or anywhere in the document
    let json_pattern = regex_lite::Regex::new(r"```json\s*([\s\S]*?)\s*```").ok();

    if let Some(re) = json_pattern {
        for cap in re.captures_iter(markdown) {
            if let Some(json_str) = cap.get(1) {
                if let Ok(links) =
                    serde_json::from_str::<Vec<CausalLinkSuggestion>>(json_str.as_str())
                {
                    return links;
                }
                // Try parsing as array of generic objects
                if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str.as_str()) {
                    let parsed: Vec<CausalLinkSuggestion> = arr
                        .iter()
                        .filter_map(|v| {
                            Some(CausalLinkSuggestion {
                                from_id: v.get("from_id")?.as_str()?.to_string(),
                                to_id: v.get("to_id")?.as_str()?.to_string(),
                                rel_type: v
                                    .get("type")
                                    .or_else(|| v.get("rel_type"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("causes")
                                    .to_string(),
                                confidence: v
                                    .get("confidence")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.5),
                                reasoning: v
                                    .get("reasoning")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            })
                        })
                        .collect();
                    if !parsed.is_empty() {
                        return parsed;
                    }
                }
            }
        }
    }

    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// NoopTier2Client (for when NotebookLM is unavailable)
// ─────────────────────────────────────────────────────────────────────────────

/// Noop Tier2 client that returns errors to trigger fallback
///
/// Used when NotebookLM MCP server is not available.
/// Stage0 will gracefully degrade to Tier1-only mode.
pub struct NoopTier2Client;

impl NoopTier2Client {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopTier2Client {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tier2Client for NoopTier2Client {
    async fn generate_divine_truth(
        &self,
        _spec_id: &str,
        _spec_content: &str,
        _task_brief_md: &str,
    ) -> Result<Tier2Response> {
        Err(Stage0Error::tier2("NotebookLM not available (noop client)"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Check if local-memory MCP server is available
pub fn has_local_memory_server(mcp_manager: &McpConnectionManager) -> bool {
    mcp_manager
        .list_all_tools()
        .keys()
        .any(|k| k.starts_with(&format!("{LOCAL_MEMORY_SERVER}__")))
}

/// Check if NotebookLM MCP server is available
pub fn has_notebooklm_server(mcp_manager: &McpConnectionManager) -> bool {
    mcp_manager
        .list_all_tools()
        .keys()
        .any(|k| k.starts_with(&format!("{NOTEBOOKLM_SERVER}__")))
}

/// Create all Stage0 adapters from an MCP connection manager
///
/// Returns `(local_memory, llm, tier2)` adapters.
/// If a required MCP server is unavailable, returns None for that adapter.
pub fn create_stage0_adapters(
    mcp_manager: Arc<McpConnectionManager>,
) -> (
    Option<LocalMemoryMcpAdapter>,
    LlmStubAdapter,
    Option<Tier2McpAdapter>,
) {
    let local_memory = if has_local_memory_server(&mcp_manager) {
        Some(LocalMemoryMcpAdapter::new(mcp_manager.clone()))
    } else {
        tracing::warn!("local-memory MCP server not available, Stage0 will use fallback");
        None
    };

    let llm = LlmStubAdapter::new();

    let tier2 = if has_notebooklm_server(&mcp_manager) {
        Some(Tier2McpAdapter::new(mcp_manager))
    } else {
        tracing::info!("notebooklm MCP server not available, Tier2 synthesis disabled");
        None
    };

    (local_memory, llm, tier2)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_stub_default() {
        let stub = LlmStubAdapter::default();
        // Just verify it creates without panic
        drop(stub);
    }

    #[test]
    fn test_parse_causal_links_from_markdown() {
        let md = r#"
# Divine Truth

## 5. Suggested Causal Links

```json
[
  {"from_id": "mem-001", "to_id": "mem-002", "type": "causes", "confidence": 0.8, "reasoning": "Test"}
]
```
"#;

        let links = parse_causal_links_from_markdown(md);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].from_id, "mem-001");
        assert_eq!(links[0].to_id, "mem-002");
        assert_eq!(links[0].rel_type, "causes");
    }

    #[test]
    fn test_parse_causal_links_empty() {
        let md = "# No JSON here";
        let links = parse_causal_links_from_markdown(md);
        assert!(links.is_empty());
    }

    #[test]
    fn test_parse_causal_links_invalid_json() {
        let md = "```json\n{invalid}\n```";
        let links = parse_causal_links_from_markdown(md);
        assert!(links.is_empty());
    }
}
