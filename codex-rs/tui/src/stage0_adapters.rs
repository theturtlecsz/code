//! Stage0 adapter implementations for codex-rs
//!
//! This module provides concrete implementations of the Stage0 trait abstractions:
//! - `LocalMemoryMcpAdapter`: Wraps MCP `local-memory` server
//! - `LlmStubAdapter`: Stub LLM client (uses heuristics, no actual LLM calls in V1)
//! - `Tier2McpAdapter`: Wraps MCP `notebooklm` server
//!
//! These adapters bridge Stage0's trait-based design with codex-rs's MCP infrastructure.

use async_trait::async_trait;
use crate::local_memory_cli;
use codex_core::mcp_connection_manager::McpConnectionManager;
use codex_stage0::dcc::{
    EnvCtx, Iqo, LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary,
};
use codex_stage0::errors::{Result, Stage0Error};
use codex_stage0::guardians::{LlmClient, MemoryKind};
use codex_stage0::tier2::{CausalLinkSuggestion, Tier2Client, Tier2Response};
use serde::Deserialize;
use serde_json::json;
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
// LocalMemoryCliAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter that implements `LocalMemoryClient` using local-memory CLI (and REST health upstream).
///
/// This is the default path for Planner; Stage0 should not require MCP.
pub struct LocalMemoryCliAdapter {
    max_content_length: usize,
}

impl LocalMemoryCliAdapter {
    pub fn new() -> Self {
        Self {
            max_content_length: 50_000,
        }
    }
}

impl Default for LocalMemoryCliAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LocalMemoryClient for LocalMemoryCliAdapter {
    async fn search_memories(
        &self,
        params: LocalMemorySearchParams,
    ) -> Result<Vec<LocalMemorySummary>> {
        let mut query_parts: Vec<String> = Vec::new();

        if !params.iqo.keywords.is_empty() {
            query_parts.push(params.iqo.keywords.join(" "));
        }
        if !params.iqo.domains.is_empty() {
            query_parts.push(params.iqo.domains.join(" "));
        }
        let query = if query_parts.is_empty() {
            "*".to_string()
        } else {
            query_parts.join(" ")
        };

        let mut tags: Vec<String> = params.iqo.required_tags.clone();
        tags.extend(params.iqo.optional_tags.iter().cloned());

        let domain = params.iqo.domains.first().map(|s| s.as_str());

        let results = local_memory_cli::search(
            &query,
            params.max_results.min(100),
            &tags,
            domain,
            self.max_content_length,
        )
        .await
        .map_err(|e| Stage0Error::local_memory(format!("local-memory search failed: {e}")))?;

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let id = r.memory.id?;
                let snippet = if r.memory.content.len() > 200 {
                    format!("{}...", &r.memory.content[..200])
                } else {
                    r.memory.content.clone()
                };
                Some(LocalMemorySummary {
                    id,
                    domain: r.memory.domain,
                    tags: r.memory.tags.unwrap_or_default(),
                    created_at: r.memory.created_at,
                    snippet,
                    similarity_score: r.relevance_score.unwrap_or(0.0),
                })
            })
            .collect())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tier2HttpAdapter
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct NotebooklmAskResponse {
    pub success: bool,
    pub data: Option<NotebooklmAskData>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotebooklmAskData {
    pub answer: String,
}

/// Adapter that implements `Tier2Client` using notebooklm-mcp HTTP service.
pub struct Tier2HttpAdapter {
    client: reqwest::Client,
    base_url: String,
    notebook: String,
}

impl Tier2HttpAdapter {
    pub fn new(base_url: String, notebook: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            notebook,
        }
    }

    pub async fn is_healthy(&self, timeout: Duration) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .timeout(timeout)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[async_trait]
impl Tier2Client for Tier2HttpAdapter {
    async fn generate_divine_truth(
        &self,
        spec_id: &str,
        spec_content: &str,
        task_brief_md: &str,
    ) -> Result<Tier2Response> {
        let prompt = codex_stage0::build_tier2_prompt(spec_id, spec_content, task_brief_md);

        let url = format!("{}/api/ask", self.base_url);
        let body = json!({
            "question": prompt,
            "notebook": self.notebook,
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| Stage0Error::tier2(format!("NotebookLM HTTP request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(Stage0Error::tier2(format!(
                "NotebookLM HTTP error: {}",
                resp.status()
            )));
        }

        let parsed: NotebooklmAskResponse = resp
            .json()
            .await
            .map_err(|e| Stage0Error::tier2(format!("Failed to parse NotebookLM response: {e}")))?;

        if !parsed.success {
            return Err(Stage0Error::tier2(format!(
                "NotebookLM error: {}",
                parsed.error.unwrap_or_else(|| "Unknown error".to_string())
            )));
        }

        let data = parsed
            .data
            .ok_or_else(|| Stage0Error::tier2("NotebookLM response missing data"))?;
        parse_tier2_answer_text(&data.answer, spec_id)
    }
}

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

            let domain = m.get("domain").and_then(|v| v.as_str()).map(String::from);

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
        Err(Stage0Error::prompt(
            "LLM stub: using heuristic classification",
        ))
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

fn parse_tier2_answer_text(text: &str, spec_id: &str) -> Result<Tier2Response> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Stage0Error::tier2("Empty response from NotebookLM"));
    }

    // Try to parse as JSON first (structured response)
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        // Extract answer/response field
        let divine_truth_md = json
            .get("answer")
            .or_else(|| json.get("response"))
            .or_else(|| json.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or(text)
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
        text.to_string()
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

/// Parse NotebookLM MCP response into Tier2Response
fn parse_tier2_response(result: &mcp_types::CallToolResult, spec_id: &str) -> Result<Tier2Response> {
    let text = result
        .content
        .iter()
        .filter_map(|c| match c {
            mcp_types::ContentBlock::TextContent(tc) => Some(tc.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    parse_tier2_answer_text(&text, spec_id)
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
// LibrarianMemoryMcpAdapter (for SPEC-KIT-103 Librarian)
// ─────────────────────────────────────────────────────────────────────────────

use codex_stage0::librarian::{
    ListParams as LibrarianListParams, LocalMemoryClient as LibrarianLocalMemoryClient,
    Memory as LibrarianMemory, MemoryChange as LibrarianMemoryChange,
    MemoryMeta as LibrarianMemoryMeta,
};

/// Adapter that implements Librarian's `LocalMemoryClient` using MCP
///
/// SPEC-KIT-103 P98: This adapter bridges the Librarian's synchronous trait
/// with the async MCP infrastructure using block_on_sync.
///
/// Operations:
/// - `list_memories` → `mcp__local-memory__search`
/// - `get_memory` → `mcp__local-memory__get_memory_by_id`
/// - `update_memory` → `mcp__local-memory__update_memory`
pub struct LibrarianMemoryMcpAdapter {
    mcp_manager: Arc<McpConnectionManager>,
}

impl LibrarianMemoryMcpAdapter {
    /// Create a new adapter wrapping the MCP connection manager
    pub fn new(mcp_manager: Arc<McpConnectionManager>) -> Self {
        Self { mcp_manager }
    }

    /// Async implementation of list_memories
    async fn list_memories_async(
        &self,
        params: &LibrarianListParams,
    ) -> Result<Vec<LibrarianMemoryMeta>> {
        // Build search arguments for mcp__local-memory__search
        let mut args = json!({
            "search_type": "semantic",
            "limit": params.limit.min(100),
            "response_format": "detailed",
        });

        // Add domain filter if specified
        if let Some(domain) = params.domains.first() {
            args["domain"] = json!(domain);
        }

        // Add minimum importance filter
        // Note: local-memory search doesn't directly support min_importance,
        // so we fetch more and filter client-side if needed

        // Call MCP tool
        let result = self
            .mcp_manager
            .call_tool(
                LOCAL_MEMORY_SERVER,
                "search",
                Some(args),
                Some(DEFAULT_MCP_TIMEOUT),
            )
            .await
            .map_err(|e| Stage0Error::local_memory(format!("MCP list_memories failed: {e}")))?;

        // Parse response
        parse_librarian_list_response(&result, params.min_importance)
    }

    /// Async implementation of get_memory
    async fn get_memory_async(&self, id: &str) -> Result<LibrarianMemory> {
        let args = json!({
            "id": id,
        });

        let result = self
            .mcp_manager
            .call_tool(
                LOCAL_MEMORY_SERVER,
                "get_memory_by_id",
                Some(args),
                Some(DEFAULT_MCP_TIMEOUT),
            )
            .await
            .map_err(|e| Stage0Error::local_memory(format!("MCP get_memory failed: {e}")))?;

        parse_librarian_get_response(&result, id)
    }

    /// Async implementation of update_memory
    async fn update_memory_async(&self, id: &str, change: &LibrarianMemoryChange) -> Result<()> {
        let mut args = json!({
            "id": id,
        });

        if let Some(ref content) = change.content {
            args["content"] = json!(content);
        }
        if let Some(ref tags) = change.tags {
            args["tags"] = json!(tags);
        }
        if let Some(importance) = change.importance {
            args["importance"] = json!(importance);
        }

        self.mcp_manager
            .call_tool(
                LOCAL_MEMORY_SERVER,
                "update_memory",
                Some(args),
                Some(DEFAULT_MCP_TIMEOUT),
            )
            .await
            .map_err(|e| Stage0Error::local_memory(format!("MCP update_memory failed: {e}")))?;

        Ok(())
    }
}

/// Block on async operation from sync context
/// Uses the same pattern as stage0_integration.rs
fn block_on_sync<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T> + Send,
    T: Send,
{
    // Try to use existing runtime, or create a new one
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // We're in an async context, use spawn_blocking to avoid blocking
            std::thread::scope(|s| {
                s.spawn(|| handle.block_on(f))
                    .join()
                    .expect("thread panicked")
            })
        }
        Err(_) => {
            // No runtime, create a new one
            tokio::runtime::Runtime::new()
                .expect("failed to create runtime")
                .block_on(f)
        }
    }
}

impl LibrarianLocalMemoryClient for LibrarianMemoryMcpAdapter {
    fn list_memories(&self, params: &LibrarianListParams) -> Result<Vec<LibrarianMemoryMeta>> {
        block_on_sync(self.list_memories_async(params))
    }

    fn get_memory(&self, id: &str) -> Result<LibrarianMemory> {
        block_on_sync(self.get_memory_async(id))
    }

    fn update_memory(&self, id: &str, change: &LibrarianMemoryChange) -> Result<()> {
        block_on_sync(self.update_memory_async(id, change))
    }
}

/// Parse MCP search response into LibrarianMemoryMeta vec
fn parse_librarian_list_response(
    result: &mcp_types::CallToolResult,
    min_importance: Option<i32>,
) -> Result<Vec<LibrarianMemoryMeta>> {
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
        .map_err(|e| Stage0Error::local_memory(format!("Failed to parse list response: {e}")))?;

    // Extract memories array
    let memories = json
        .get("memories")
        .or_else(|| json.get("results"))
        .and_then(|v| v.as_array())
        .unwrap_or(&Vec::new())
        .clone();

    let metas: Vec<LibrarianMemoryMeta> = memories
        .iter()
        .filter_map(|m| {
            let id = m.get("id")?.as_str()?.to_string();
            let content = m
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let importance = m
                .get("importance")
                .and_then(|v| v.as_i64())
                .map(|i| i as i32);

            // Apply min_importance filter
            if let Some(min) = min_importance {
                if importance.unwrap_or(0) < min {
                    return None;
                }
            }

            let domain = m.get("domain").and_then(|v| v.as_str()).map(String::from);

            let tags: Vec<String> = m
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Some(LibrarianMemoryMeta {
                id,
                content,
                tags,
                importance,
                domain,
            })
        })
        .collect();

    Ok(metas)
}

/// Parse MCP get_memory_by_id response into LibrarianMemory
fn parse_librarian_get_response(
    result: &mcp_types::CallToolResult,
    id: &str,
) -> Result<LibrarianMemory> {
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
        return Err(Stage0Error::local_memory(format!(
            "Empty response for memory: {}",
            id
        )));
    }

    // Parse JSON response
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| Stage0Error::local_memory(format!("Failed to parse get response: {e}")))?;

    // Extract memory fields (might be wrapped in "memory" field or at root)
    let mem = json.get("memory").unwrap_or(&json);

    let memory_id = mem
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(id)
        .to_string();

    let content = mem
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tags: Vec<String> = mem
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let importance = mem
        .get("importance")
        .and_then(|v| v.as_i64())
        .map(|i| i as i32);

    let domain = mem.get("domain").and_then(|v| v.as_str()).map(String::from);

    let created_at = mem
        .get("created_at")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(LibrarianMemory {
        id: memory_id,
        content,
        tags,
        importance,
        domain,
        created_at,
    })
}

/// Create a Librarian memory client from an MCP connection manager
///
/// SPEC-KIT-103 P98: Factory function for creating the Librarian adapter.
/// Returns None if the local-memory MCP server is unavailable.
pub fn create_librarian_memory_client(
    mcp_manager: Arc<McpConnectionManager>,
) -> Option<LibrarianMemoryMcpAdapter> {
    if has_local_memory_server(&mcp_manager) {
        Some(LibrarianMemoryMcpAdapter::new(mcp_manager))
    } else {
        tracing::warn!("local-memory MCP server not available for Librarian");
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RelationshipsMcpAdapter (SPEC-KIT-103 P98 Task 6)
// ─────────────────────────────────────────────────────────────────────────────

use codex_stage0::librarian::{RelationshipInput, RelationshipsClient};

/// Adapter that implements RelationshipsClient using MCP
///
/// SPEC-KIT-103 P98 Task 6: Creates causal relationship edges in local-memory
/// via the `mcp__local-memory__relationships` tool.
pub struct RelationshipsMcpAdapter {
    mcp_manager: Arc<McpConnectionManager>,
}

impl RelationshipsMcpAdapter {
    /// Create a new adapter wrapping the MCP connection manager
    pub fn new(mcp_manager: Arc<McpConnectionManager>) -> Self {
        Self { mcp_manager }
    }

    /// Async implementation of create_relationship
    async fn create_relationship_async(&self, input: &RelationshipInput) -> Result<()> {
        let args = json!({
            "relationship_type": "create",
            "source_memory_id": input.source_id,
            "target_memory_id": input.target_id,
            "relationship_type_enum": input.relationship_type,
            "strength": input.strength,
            "context": input.context,
        });

        self.mcp_manager
            .call_tool(
                LOCAL_MEMORY_SERVER,
                "relationships",
                Some(args),
                Some(DEFAULT_MCP_TIMEOUT),
            )
            .await
            .map_err(|e| {
                Stage0Error::local_memory(format!("MCP create_relationship failed: {e}"))
            })?;

        Ok(())
    }
}

impl RelationshipsClient for RelationshipsMcpAdapter {
    fn create_relationship(&self, input: &RelationshipInput) -> Result<()> {
        block_on_sync(self.create_relationship_async(input))
    }
}

/// Create a Librarian relationships client from an MCP connection manager
///
/// SPEC-KIT-103 P98 Task 6: Factory function for creating the relationships adapter.
/// Returns None if the local-memory MCP server is unavailable.
pub fn create_relationships_client(
    mcp_manager: Arc<McpConnectionManager>,
) -> Option<RelationshipsMcpAdapter> {
    if has_local_memory_server(&mcp_manager) {
        Some(RelationshipsMcpAdapter::new(mcp_manager))
    } else {
        tracing::warn!("local-memory MCP server not available for relationships");
        None
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
