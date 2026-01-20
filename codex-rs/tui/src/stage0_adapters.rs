//! Stage0 adapter implementations for codex-rs
//!
//! This module provides concrete implementations of the Stage0 trait abstractions:
//! - `LocalMemoryCliAdapter`: Wraps `local-memory` CLI search (REST health + CLI commands)
//! - `Tier2HttpAdapter`: Uses HTTP to call notebooklm-mcp service (no MCP dependency)
//! - `LibrarianMemoryRestAdapter`: Uses CLI + REST for Librarian sweep/edit
//! - `RelationshipsRestAdapter`: Uses REST for relationships
//! - `LlmStubAdapter`: Stub LLM client (uses heuristics, no actual LLM calls in V1)
//!
//! **CONVERGENCE (MAINT-12)**: Stage0 uses HTTP-only for NotebookLM and CLI/REST for
//! local-memory. No MCP dependency required.

use crate::local_memory_cli;
use async_trait::async_trait;
use codex_stage0::dcc::{
    EnvCtx, Iqo, LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary,
};
use codex_stage0::errors::{Result, Stage0Error};
use codex_stage0::guardians::{LlmClient, MemoryKind};
use codex_stage0::tier2::{CausalLinkSuggestion, Tier2Client, Tier2Response};
use reqwest::blocking::Client as BlockingHttpClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

// ─────────────────────────────────────────────────────────────────────────────
// Integration Defaults
// ─────────────────────────────────────────────────────────────────────────────

const LOCAL_MEMORY_API_BASE_DEFAULT: &str = "http://localhost:3002/api/v1";
const LOCAL_MEMORY_HTTP_TIMEOUT: Duration = Duration::from_secs(30);

fn local_memory_api_base() -> String {
    std::env::var("LOCAL_MEMORY_API_BASE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| LOCAL_MEMORY_API_BASE_DEFAULT.to_string())
}

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

        // Build exclusion set for client-side filtering
        // CONVERGENCE: local-memory CLI doesn't support --exclude-tags, so filter here
        let exclude_set: std::collections::HashSet<&str> =
            params.iqo.exclude_tags.iter().map(|s| s.as_str()).collect();

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let id = r.memory.id?;

                // Client-side exclusion: skip memories with any excluded tag
                if let Some(ref tags) = r.memory.tags {
                    if tags.iter().any(|t| exclude_set.contains(t.as_str())) {
                        return None;
                    }
                }

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

// ─────────────────────────────────────────────────────────────────────────────
// NotebookLM API Response Types
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

#[derive(Debug, Deserialize)]
struct NotebooklmUpsertResponse {
    pub success: bool,
    pub data: Option<NotebooklmUpsertData>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NotebooklmUpsertData {
    pub name: String,
    pub action: String, // "created" | "updated"
}

/// Upsert a source document to NotebookLM
///
/// SPEC-TIER2-SOURCES: Source-based architecture. Upload SPEC and TASK_BRIEF as sources
/// before sending a minimal query, avoiding the 2k chat query limit.
fn upsert_source_blocking(
    client: &BlockingHttpClient,
    base_url: &str,
    notebook: &str,
    name: &str,
    content: &str,
) -> std::result::Result<String, String> {
    use std::io::Write;

    let url = format!("{}/api/sources/upsert", base_url);

    let trace_msg = format!(
        "[{}] Tier2 UPSERT: name={}, notebook={}, content_len={}\n",
        chrono::Utc::now().format("%H:%M:%S%.3f"),
        name,
        notebook,
        content.len()
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/speckit-trace.log")
    {
        let _ = f.write_all(trace_msg.as_bytes());
    }

    let body = json!({
        "notebook": notebook,
        "name": name,
        "content": content,
    });

    let resp = client
        .post(&url)
        .timeout(Duration::from_secs(120))
        .json(&body)
        .send()
        .map_err(|e| format!("Upsert HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Upsert HTTP error: {}", resp.status()));
    }

    let parsed: NotebooklmUpsertResponse = resp
        .json()
        .map_err(|e| format!("Failed to parse upsert response: {e}"))?;

    if !parsed.success {
        return Err(parsed
            .error
            .unwrap_or_else(|| "Unknown upsert error".to_string()));
    }

    let action = parsed
        .data
        .map(|d| d.action)
        .unwrap_or_else(|| "unknown".to_string());

    let trace_msg = format!(
        "[{}] Tier2 UPSERT SUCCESS: name={}, action={}\n",
        chrono::Utc::now().format("%H:%M:%S%.3f"),
        name,
        action
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/speckit-trace.log")
    {
        let _ = f.write_all(trace_msg.as_bytes());
    }

    Ok(action)
}

/// Adapter that implements `Tier2Client` using notebooklm-mcp HTTP service.
///
/// IMPORTANT: Due to tokio runtime conflicts, HTTP calls must be made OUTSIDE
/// the tokio runtime. Use `fetch_tier2_response_blocking()` before entering
/// Stage0, then pass a `PrecomputedTier2Client` with the result.
pub struct Tier2HttpAdapter {
    base_url: String,
    notebook: String,
}

impl Tier2HttpAdapter {
    pub fn new(base_url: String, notebook: String) -> Self {
        Self { base_url, notebook }
    }

    /// Fetch Tier2 response using blocking HTTP - call OUTSIDE tokio runtime!
    ///
    /// SPEC-DOGFOOD-001 S30: reqwest::blocking creates its own tokio runtime,
    /// so this must be called before entering any tokio block_on context.
    pub fn fetch_tier2_response_blocking(
        &self,
        spec_id: &str,
        spec_content: &str,
        task_brief_md: &str,
    ) -> Result<Tier2Response> {
        // FILE-BASED TRACE
        {
            use std::io::Write;
            let trace_msg = format!(
                "[{}] Tier2 FETCH BLOCKING: spec_id={}, url={}/api/ask, notebook={}\n",
                chrono::Utc::now().format("%H:%M:%S%.3f"),
                spec_id,
                self.base_url,
                self.notebook
            );
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/speckit-trace.log")
            {
                let _ = f.write_all(trace_msg.as_bytes());
            }
        }

        let prompt = codex_stage0::build_tier2_prompt(spec_id, spec_content, task_brief_md);
        let url = format!("{}/api/ask", self.base_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300)) // 5 min to match NotebookLM service timeout
            .build()
            .map_err(|e| Stage0Error::tier2(format!("Failed to create HTTP client: {e}")))?;

        let body = json!({
            "question": prompt,
            "notebook": &self.notebook,
        });

        let resp = client.post(&url).json(&body).send().map_err(|e| {
            // FILE-BASED TRACE
            {
                use std::io::Write;
                let trace_msg = format!(
                    "[{}] Tier2 HTTP ERROR: {}\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    e
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
            }
            Stage0Error::tier2(format!("NotebookLM HTTP request failed: {e}"))
        })?;

        if !resp.status().is_success() {
            // FILE-BASED TRACE
            {
                use std::io::Write;
                let trace_msg = format!(
                    "[{}] Tier2 HTTP STATUS ERROR: {}\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    resp.status()
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
            }
            return Err(Stage0Error::tier2(format!(
                "NotebookLM HTTP error: {}",
                resp.status()
            )));
        }

        let parsed: NotebooklmAskResponse = resp.json().map_err(|e| {
            // FILE-BASED TRACE
            {
                use std::io::Write;
                let trace_msg = format!(
                    "[{}] Tier2 JSON PARSE ERROR: {}\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    e
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
            }
            Stage0Error::tier2(format!("Failed to parse NotebookLM response: {e}"))
        })?;

        if !parsed.success {
            let error_msg = parsed
                .error
                .unwrap_or_else(|| "Unknown NotebookLM error".to_string());
            return Err(Stage0Error::tier2(format!(
                "NotebookLM error: {}",
                error_msg
            )));
        }

        let answer = parsed
            .data
            .map(|d| d.answer)
            .unwrap_or_else(|| "No answer received".to_string());

        // FILE-BASED TRACE: Success
        {
            use std::io::Write;
            let trace_msg = format!(
                "[{}] Tier2 FETCH SUCCESS: answer_len={}\n",
                chrono::Utc::now().format("%H:%M:%S%.3f"),
                answer.len()
            );
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/speckit-trace.log")
            {
                let _ = f.write_all(trace_msg.as_bytes());
            }
        }

        Ok(Tier2Response {
            divine_truth_md: answer,
            suggested_links: vec![],
        })
    }
}

/// Pre-computed Tier2 client that holds an already-fetched response.
///
/// SPEC-DOGFOOD-001 S30: Use this to avoid tokio runtime conflicts.
/// Fetch the response using `Tier2HttpAdapter::fetch_tier2_response_blocking()`
/// BEFORE entering the tokio runtime, then pass this client to Stage0Engine.
pub struct PrecomputedTier2Client {
    response: Option<Tier2Response>,
    error: Option<String>,
}

impl PrecomputedTier2Client {
    /// Create with a successful response
    pub fn with_response(response: Tier2Response) -> Self {
        Self {
            response: Some(response),
            error: None,
        }
    }

    /// Create with an error
    pub fn with_error(error: String) -> Self {
        Self {
            response: None,
            error: Some(error),
        }
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
        // SPEC-TIER2-SOURCES (S32): Source-based architecture.
        // 1. Upsert CURRENT_SPEC.md source
        // 2. Upsert CURRENT_TASK_BRIEF.md source
        // 3. Send minimal query (~100 chars) referencing sources
        //
        // SPEC-DOGFOOD-001 S30: Spawn a completely separate thread for the HTTP call.
        // reqwest::blocking creates its own tokio runtime, which conflicts with our
        // existing runtime. By spawning a new std::thread, we isolate the runtimes.
        let base_url = self.base_url.clone();
        let notebook = self.notebook.clone();
        let spec_id = spec_id.to_string();
        let spec_content = spec_content.to_string();
        let task_brief_md = task_brief_md.to_string();

        let handle = std::thread::spawn(move || {
            use std::io::Write;

            // FILE-BASED TRACE
            {
                let trace_msg = format!(
                    "[{}] Tier2 THREAD START (source-based): spec_id={}, notebook={}\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    spec_id,
                    notebook
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
            }

            // Create HTTP client with longer timeout for source operations
            let client = match reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
            {
                Ok(c) => c,
                Err(e) => return Err(format!("Failed to create HTTP client: {e}")),
            };

            // SPEC-TIER2-SOURCES: Step 1 - Upsert CURRENT_SPEC.md source
            // Prepend spec_id as heading for context
            let spec_source_content = format!("# SPEC: {}\n\n{}", spec_id, spec_content);
            if let Err(e) = upsert_source_blocking(
                &client,
                &base_url,
                &notebook,
                "CURRENT_SPEC",
                &spec_source_content,
            ) {
                let trace_msg = format!(
                    "[{}] Tier2 UPSERT SPEC FAILED: {}\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    e
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
                // Continue anyway - we'll try with the query
            }

            // SPEC-TIER2-SOURCES: Step 2 - Upsert CURRENT_TASK_BRIEF.md source
            let brief_source_content = format!("# Task Brief: {}\n\n{}", spec_id, task_brief_md);
            if let Err(e) = upsert_source_blocking(
                &client,
                &base_url,
                &notebook,
                "CURRENT_TASK_BRIEF",
                &brief_source_content,
            ) {
                let trace_msg = format!(
                    "[{}] Tier2 UPSERT BRIEF FAILED: {}\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    e
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
                // Continue anyway - we'll try with the query
            }

            // SPEC-TIER2-SOURCES S33: Close session after source operations
            // The upsert operations leave the browser on the Sources tab.
            // Close all sessions so the ask gets a fresh browser in chat view.
            {
                let sessions_url = format!("{}/api/sessions", base_url);
                if let Ok(sessions_resp) = client.get(&sessions_url).send() {
                    if let Ok(sessions_json) = sessions_resp.json::<serde_json::Value>() {
                        if let Some(sessions) = sessions_json["data"]["sessions"].as_array() {
                            for session in sessions {
                                if let Some(id) = session["id"].as_str() {
                                    let close_url = format!("{}/api/sessions/{}", base_url, id);
                                    let _ = client.delete(&close_url).send();
                                }
                            }
                        }
                    }
                }
                // Brief delay for session cleanup
                std::thread::sleep(Duration::from_millis(500));
            }

            // SPEC-TIER2-SOURCES: Step 3 - Send minimal query
            // Now that sources are uploaded, we send a short query
            let prompt = codex_stage0::build_tier2_prompt(&spec_id, &spec_content, &task_brief_md);
            let url = format!("{}/api/ask", base_url);

            // FILE-BASED TRACE: Log prompt size for debugging
            {
                let preview: String = prompt.chars().take(500).collect();
                let trace_msg = format!(
                    "[{}] Tier2 PROMPT: len={} chars, preview=\"{}...\"\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    prompt.len(),
                    preview.replace('\n', "\\n")
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
                // Also write full prompt to separate file for inspection
                let _ = std::fs::write("/tmp/tier2-prompt.txt", &prompt);
            }

            let body = serde_json::json!({
                "question": prompt,
                "notebook": notebook,
            });

            let resp = match client.post(&url).json(&body).send() {
                Ok(r) => r,
                Err(e) => {
                    let trace_msg = format!(
                        "[{}] Tier2 HTTP ERROR: {}\n",
                        chrono::Utc::now().format("%H:%M:%S%.3f"),
                        e
                    );
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/speckit-trace.log")
                    {
                        let _ = f.write_all(trace_msg.as_bytes());
                    }
                    return Err(format!("NotebookLM HTTP request failed: {e}"));
                }
            };

            if !resp.status().is_success() {
                let trace_msg = format!(
                    "[{}] Tier2 HTTP STATUS ERROR: {}\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    resp.status()
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
                return Err(format!("NotebookLM HTTP error: {}", resp.status()));
            }

            let parsed: NotebooklmAskResponse = match resp.json() {
                Ok(p) => p,
                Err(e) => {
                    let trace_msg = format!(
                        "[{}] Tier2 JSON PARSE ERROR: {}\n",
                        chrono::Utc::now().format("%H:%M:%S%.3f"),
                        e
                    );
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/speckit-trace.log")
                    {
                        let _ = f.write_all(trace_msg.as_bytes());
                    }
                    return Err(format!("Failed to parse NotebookLM response: {e}"));
                }
            };

            if !parsed.success {
                let error_msg = parsed
                    .error
                    .unwrap_or_else(|| "Unknown NotebookLM error".to_string());
                return Err(format!("NotebookLM error: {}", error_msg));
            }

            let answer = parsed
                .data
                .map(|d| d.answer)
                .unwrap_or_else(|| "No answer received".to_string());

            // FILE-BASED TRACE: Success
            {
                let trace_msg = format!(
                    "[{}] Tier2 THREAD SUCCESS: answer_len={}\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    answer.len()
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
            }

            Ok(Tier2Response {
                divine_truth_md: answer,
                suggested_links: vec![],
            })
        });

        // Wait for the thread to complete
        match handle.join() {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(Stage0Error::tier2(e)),
            Err(_) => Err(Stage0Error::tier2("Tier2 thread panicked")),
        }
    }
}

#[async_trait]
impl Tier2Client for PrecomputedTier2Client {
    async fn generate_divine_truth(
        &self,
        _spec_id: &str,
        _spec_content: &str,
        _task_brief_md: &str,
    ) -> Result<Tier2Response> {
        // Simply return the pre-computed response
        if let Some(ref response) = self.response {
            Ok(response.clone())
        } else if let Some(ref error) = self.error {
            Err(Stage0Error::tier2(error.clone()))
        } else {
            Err(Stage0Error::tier2(
                "No pre-computed Tier2 response available",
            ))
        }
    }
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
// LibrarianMemoryRestAdapter (for SPEC-KIT-103 Librarian)
// ─────────────────────────────────────────────────────────────────────────────

use codex_stage0::librarian::{
    ListParams as LibrarianListParams, LocalMemoryClient as LibrarianLocalMemoryClient,
    Memory as LibrarianMemory, MemoryChange as LibrarianMemoryChange,
    MemoryMeta as LibrarianMemoryMeta,
};

#[derive(Debug, Deserialize)]
struct LocalMemoryRestEnvelope<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalMemoryRestMemory {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub importance: Option<i32>,
    pub domain: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalMemoryRestUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<i32>,
}

/// Adapter that implements Librarian's `LocalMemoryClient` using local-memory CLI + REST (no MCP).
pub struct LibrarianMemoryRestAdapter {
    client: BlockingHttpClient,
    api_base: String,
    max_content_length: usize,
}

impl LibrarianMemoryRestAdapter {
    pub fn new(api_base: String) -> Self {
        Self {
            client: BlockingHttpClient::new(),
            api_base,
            max_content_length: 50_000,
        }
    }

    fn memory_url(&self, id: &str) -> String {
        format!("{}/memories/{}", self.api_base.trim_end_matches('/'), id)
    }
}

impl LibrarianLocalMemoryClient for LibrarianMemoryRestAdapter {
    fn list_memories(&self, params: &LibrarianListParams) -> Result<Vec<LibrarianMemoryMeta>> {
        let limit = params.limit.clamp(1, 100);
        let tags: Vec<String> = Vec::new();

        let domains: Vec<Option<&str>> = if params.domains.is_empty() {
            vec![None]
        } else {
            params.domains.iter().map(|d| Some(d.as_str())).collect()
        };

        let per_domain_limit = (limit + domains.len().saturating_sub(1)) / domains.len();

        let mut metas: Vec<LibrarianMemoryMeta> = Vec::new();
        for domain in domains {
            let results = local_memory_cli::search_blocking(
                "*",
                per_domain_limit.min(100),
                &tags,
                domain,
                self.max_content_length,
            )
            .map_err(|e| Stage0Error::local_memory(format!("local-memory search failed: {e}")))?;

            for r in results {
                let Some(id) = r.memory.id else {
                    continue;
                };

                let importance = r.memory.importance.map(|i| i as i32);
                if let Some(min) = params.min_importance {
                    if importance.unwrap_or(0) < min {
                        continue;
                    }
                }

                metas.push(LibrarianMemoryMeta {
                    id,
                    content: r.memory.content,
                    tags: r.memory.tags.unwrap_or_default(),
                    importance,
                    domain: r.memory.domain,
                });

                if metas.len() >= limit {
                    break;
                }
            }

            if metas.len() >= limit {
                break;
            }
        }

        Ok(metas)
    }

    fn get_memory(&self, id: &str) -> Result<LibrarianMemory> {
        let url = self.memory_url(id);
        let resp = self
            .client
            .get(&url)
            .timeout(LOCAL_MEMORY_HTTP_TIMEOUT)
            .send()
            .map_err(|e| Stage0Error::local_memory(format!("GET {url} failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(Stage0Error::local_memory(format!(
                "GET {url} failed: {}",
                resp.status()
            )));
        }

        let parsed: LocalMemoryRestEnvelope<LocalMemoryRestMemory> = resp
            .json()
            .map_err(|e| Stage0Error::local_memory(format!("Failed to parse GET response: {e}")))?;

        if !parsed.success {
            return Err(Stage0Error::local_memory(
                parsed
                    .error
                    .or(parsed.message)
                    .unwrap_or_else(|| "local-memory GET failed".to_string()),
            ));
        }

        let mem = parsed
            .data
            .ok_or_else(|| Stage0Error::local_memory("local-memory GET missing data"))?;

        Ok(LibrarianMemory {
            id: mem.id,
            content: mem.content,
            tags: mem.tags,
            importance: mem.importance,
            domain: mem.domain,
            created_at: mem.created_at,
        })
    }

    fn update_memory(&self, id: &str, change: &LibrarianMemoryChange) -> Result<()> {
        let url = self.memory_url(id);
        let body = LocalMemoryRestUpdateRequest {
            content: change.content.clone(),
            tags: change.tags.clone(),
            importance: change.importance,
        };

        let resp = self
            .client
            .put(&url)
            .timeout(LOCAL_MEMORY_HTTP_TIMEOUT)
            .json(&body)
            .send()
            .map_err(|e| Stage0Error::local_memory(format!("PUT {url} failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(Stage0Error::local_memory(format!(
                "PUT {url} failed: {}",
                resp.status()
            )));
        }

        let parsed: LocalMemoryRestEnvelope<serde_json::Value> = resp
            .json()
            .map_err(|e| Stage0Error::local_memory(format!("Failed to parse PUT response: {e}")))?;

        if !parsed.success {
            return Err(Stage0Error::local_memory(
                parsed
                    .error
                    .or(parsed.message)
                    .unwrap_or_else(|| "local-memory update failed".to_string()),
            ));
        }

        Ok(())
    }
}

/// Create a Librarian memory client using CLI + REST (no MCP).
pub fn create_librarian_memory_client() -> Option<LibrarianMemoryRestAdapter> {
    if local_memory_cli::local_memory_daemon_healthy_blocking(Duration::from_secs(2)) {
        Some(LibrarianMemoryRestAdapter::new(local_memory_api_base()))
    } else {
        tracing::warn!("local-memory daemon not healthy for Librarian (using sample data)");
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RelationshipsRestAdapter (SPEC-KIT-103 P98 Task 6)
// ─────────────────────────────────────────────────────────────────────────────

use codex_stage0::librarian::{RelationshipInput, RelationshipsClient};

#[derive(Debug, Serialize)]
struct LocalMemoryRestRelationshipRequest<'a> {
    pub source_memory_id: &'a str,
    pub target_memory_id: &'a str,
    pub relationship_type: &'a str,
    pub strength: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<&'a str>,
}

pub struct RelationshipsRestAdapter {
    client: BlockingHttpClient,
    api_base: String,
}

impl RelationshipsRestAdapter {
    pub fn new(api_base: String) -> Self {
        Self {
            client: BlockingHttpClient::new(),
            api_base,
        }
    }

    fn relationships_url(&self) -> String {
        format!("{}/relationships", self.api_base.trim_end_matches('/'))
    }
}

impl RelationshipsClient for RelationshipsRestAdapter {
    fn create_relationship(&self, input: &RelationshipInput) -> Result<()> {
        let url = self.relationships_url();
        let body = LocalMemoryRestRelationshipRequest {
            source_memory_id: &input.source_id,
            target_memory_id: &input.target_id,
            relationship_type: &input.relationship_type,
            strength: input.strength,
            context: input.context.as_deref(),
        };

        let resp = self
            .client
            .post(&url)
            .timeout(LOCAL_MEMORY_HTTP_TIMEOUT)
            .json(&body)
            .send()
            .map_err(|e| Stage0Error::local_memory(format!("POST {url} failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(Stage0Error::local_memory(format!(
                "POST {url} failed: {}",
                resp.status()
            )));
        }

        let parsed: LocalMemoryRestEnvelope<serde_json::Value> = resp.json().map_err(|e| {
            Stage0Error::local_memory(format!("Failed to parse relationships response: {e}"))
        })?;

        if !parsed.success {
            return Err(Stage0Error::local_memory(
                parsed
                    .error
                    .or(parsed.message)
                    .unwrap_or_else(|| "local-memory relationship creation failed".to_string()),
            ));
        }

        Ok(())
    }
}

/// Create a Librarian relationships client using REST (no MCP).
pub fn create_relationships_client() -> Option<RelationshipsRestAdapter> {
    if local_memory_cli::local_memory_daemon_healthy_blocking(Duration::from_secs(2)) {
        Some(RelationshipsRestAdapter::new(local_memory_api_base()))
    } else {
        tracing::warn!("local-memory daemon not healthy for relationships (skipping edges)");
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_stub_default() {
        // Just verify it creates without panic
        let _ = LlmStubAdapter;
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
