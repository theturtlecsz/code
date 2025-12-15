//! NotebookLM HTTP Service client.
//!
//! Connects to notebooklm-mcp service (default: http://127.0.0.1:3456)
//! with lazy service spawning and budget tracking.
//!
//! # Architecture
//! This module is the SOLE GATEKEEPER for NotebookLM quota.
//! Other parts of codex-rs should not access NotebookLM directly.

use super::budget::BudgetTracker;
use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const DEFAULT_PORT: u16 = 3456;
const DEFAULT_HOST: &str = "127.0.0.1";
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Artifact with [ARCH] prefix for namespace isolation.
#[derive(Debug, Clone)]
pub struct Artifact {
    /// Title with [ARCH] prefix (e.g., "[ARCH] Churn Metrics").
    pub title: String,
    /// Content to upload.
    pub content: String,
}

impl Artifact {
    /// Create a new artifact with automatic [ARCH] prefix.
    pub fn new(name: &str, content: String) -> Self {
        Self {
            title: format!("[ARCH] {name}"),
            content,
        }
    }
}

/// Service health status response.
#[derive(Debug, Clone, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: Option<String>,
    pub uptime: Option<u64>,
    pub queue: Option<QueueStatus>,
    pub sessions: Option<SessionStatus>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueStatus {
    pub pending: u32,
    pub processing: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionStatus {
    pub active: u32,
    pub max: u32,
}

/// Response from ask endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct AskResponse {
    pub success: bool,
    pub data: Option<AskData>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskData {
    pub answer: String,
    pub session_id: Option<String>,
}

/// Source entry in a notebook.
#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub index: usize,
    pub title: String,
    pub status: Option<String>,
}

/// Response from list sources endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesResponse {
    pub success: bool,
    pub data: Option<SourcesData>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesData {
    pub sources: Vec<Source>,
    pub source_count: usize,
}

/// Generic API response.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Request body for ask endpoint.
#[derive(Debug, Serialize)]
struct AskRequest {
    question: String,
    notebook: String,
}

/// Request body for add source endpoint.
#[derive(Debug, Serialize)]
struct AddSourceRequest {
    source_type: String,
    content: String,
    notebook: String,
    title: Option<String>,
}

/// Request body for delete source endpoint.
#[derive(Debug, Serialize)]
struct DeleteSourceRequest {
    notebook: String,
    index: usize,
}

/// NotebookLM HTTP Service client.
pub struct NlmService {
    base_url: String,
    client: Client,
    pub budget: BudgetTracker,
    notebook: String,
}

impl NlmService {
    /// Create a new service client.
    pub fn new(vault_path: &Path, notebook: &str) -> Result<Self> {
        let budget = BudgetTracker::load(vault_path)?;
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            base_url: format!("http://{DEFAULT_HOST}:{DEFAULT_PORT}"),
            client,
            budget,
            notebook: notebook.to_string(),
        })
    }

    /// Create with custom port.
    pub fn with_port(vault_path: &Path, notebook: &str, port: u16) -> Result<Self> {
        let mut service = Self::new(vault_path, notebook)?;
        service.base_url = format!("http://{DEFAULT_HOST}:{port}");
        Ok(service)
    }

    /// Check if service is running.
    pub async fn is_running(&self) -> bool {
        let health_url = format!("{}/health", self.base_url);
        let result = self
            .client
            .get(&health_url)
            .timeout(HEALTH_TIMEOUT)
            .send()
            .await;

        match result {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get service health status.
    pub async fn health(&self) -> Result<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .client
            .get(&url)
            .timeout(HEALTH_TIMEOUT)
            .send()
            .await
            .context("Failed to connect to NotebookLM service")?;

        if !resp.status().is_success() {
            bail!("Health check failed: {}", resp.status());
        }

        resp.json().await.context("Failed to parse health response")
    }

    /// Start the service daemon.
    pub fn start_service(port: u16, foreground: bool) -> Result<()> {
        let nlm_cli = find_nlm_cli()?;

        if foreground {
            // Run in foreground (blocking)
            let status = Command::new("node")
                .arg(&nlm_cli)
                .args([
                    "service",
                    "start",
                    "--foreground",
                    "--port",
                    &port.to_string(),
                ])
                .status()
                .context("Failed to start NotebookLM service")?;

            if !status.success() {
                bail!("Service exited with error");
            }
        } else {
            // Spawn daemon (non-blocking)
            Command::new("node")
                .arg(&nlm_cli)
                .args(["service", "start", "--port", &port.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("Failed to spawn NotebookLM service")?;

            // Give it time to start
            std::thread::sleep(Duration::from_millis(500));
        }

        Ok(())
    }

    /// Stop the service daemon.
    pub fn stop_service() -> Result<()> {
        let nlm_cli = find_nlm_cli()?;

        let output = Command::new("node")
            .arg(&nlm_cli)
            .args(["service", "stop"])
            .output()
            .context("Failed to stop NotebookLM service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Not an error if service wasn't running
            if !stderr.contains("not running") {
                bail!("Failed to stop service: {}", stderr);
            }
        }

        Ok(())
    }

    /// Get service status via CLI.
    pub fn service_status() -> Result<String> {
        let nlm_cli = find_nlm_cli()?;

        let output = Command::new("node")
            .arg(&nlm_cli)
            .args(["service", "status"])
            .output()
            .context("Failed to check NotebookLM service status")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Ask a question (budget-tracked).
    pub async fn ask(&mut self, question: &str) -> Result<String> {
        // Check budget
        if self.budget.is_exhausted() {
            bail!(
                "Daily limit ({}) reached. Resets in {}.",
                self.budget.limit(),
                self.budget.time_until_reset()
            );
        }

        let url = format!("{}/api/ask", self.base_url);
        let req = AskRequest {
            question: question.to_string(),
            notebook: self.notebook.clone(),
        };

        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to send ask request")?;

        if !resp.status().is_success() {
            bail!("Ask request failed: {}", resp.status());
        }

        let result: AskResponse = resp.json().await.context("Failed to parse ask response")?;

        if !result.success {
            bail!(
                "NotebookLM error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        // Record successful query
        self.budget.record_query()?;

        let data = result.data.context("No data in response")?;
        Ok(data.answer)
    }

    /// List sources in the notebook.
    pub async fn list_sources(&self) -> Result<Vec<Source>> {
        let url = format!(
            "{}/api/sources?notebook={}",
            self.base_url,
            urlencoding::encode(&self.notebook)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list sources")?;

        if !resp.status().is_success() {
            bail!("List sources failed: {}", resp.status());
        }

        let result: SourcesResponse = resp
            .json()
            .await
            .context("Failed to parse sources response")?;

        if !result.success {
            bail!(
                "NotebookLM error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        let data = result.data.context("No data in response")?;
        Ok(data.sources)
    }

    /// Delete a source by index.
    pub async fn delete_source(&self, index: usize) -> Result<()> {
        let url = format!("{}/api/sources/delete", self.base_url);
        let req = DeleteSourceRequest {
            notebook: self.notebook.clone(),
            index,
        };

        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to delete source")?;

        if !resp.status().is_success() {
            bail!("Delete source failed: {}", resp.status());
        }

        let result: ApiResponse = resp
            .json()
            .await
            .context("Failed to parse delete response")?;

        if !result.success {
            bail!(
                "Delete failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        Ok(())
    }

    /// Maximum source size for NotebookLM (500KB is safe).
    const MAX_SOURCE_SIZE: usize = 500_000;

    /// Add a text source.
    pub async fn add_text_source(&self, title: &str, content: &str) -> Result<()> {
        // Check size limit
        if content.len() > Self::MAX_SOURCE_SIZE {
            bail!(
                "Source '{}' too large ({} bytes, max {}). Consider filtering.",
                title,
                content.len(),
                Self::MAX_SOURCE_SIZE
            );
        }

        let url = format!("{}/api/sources", self.base_url);
        let req = AddSourceRequest {
            source_type: "text".to_string(),
            content: content.to_string(),
            notebook: self.notebook.clone(),
            title: Some(title.to_string()),
        };

        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to add source")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Add source failed (HTTP {}): {}", status, body);
        }

        let body = resp.text().await.context("Failed to read response")?;
        let result: ApiResponse = serde_json::from_str(&body)
            .with_context(|| format!("Failed to parse response: {body}"))?;

        if !result.success {
            bail!(
                "Add source failed: {}",
                result
                    .error
                    .unwrap_or_else(|| format!("Unknown error: {body}"))
            );
        }

        Ok(())
    }

    /// Atomic swap: delete [ARCH] sources, upload fresh artifacts.
    pub async fn refresh_context(&self, artifacts: &[Artifact]) -> Result<RefreshResult> {
        let mut result = RefreshResult::default();

        // 1. List all sources
        let sources = self.list_sources().await?;
        result.total_sources = sources.len();

        // 2. Identify managed sources (stale artifacts)
        let stale_indices: Vec<usize> = sources
            .iter()
            .filter(|s| s.title.starts_with("[ARCH]"))
            .map(|s| s.index)
            .collect();
        result.deleted = stale_indices.len();

        // 3. Delete highest index first (avoids shifting)
        for index in stale_indices.iter().rev() {
            match self.delete_source(*index).await {
                Ok(_) => {}
                Err(e) => {
                    // Log but continue
                    tracing::warn!("Failed to delete source {index}: {e}");
                }
            }
        }

        // 4. Upload fresh artifacts
        for artifact in artifacts {
            self.add_text_source(&artifact.title, &artifact.content)
                .await?;
            result.uploaded += 1;
        }

        Ok(result)
    }
}

/// Result of refresh_context operation.
#[derive(Debug, Default)]
pub struct RefreshResult {
    pub total_sources: usize,
    pub deleted: usize,
    pub uploaded: usize,
}

/// Find the notebooklm-mcp CLI.
fn find_nlm_cli() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let cli_path = home.join("notebooklm-mcp/dist/cli/index.js");

    if cli_path.exists() {
        Ok(cli_path)
    } else {
        bail!(
            "NotebookLM CLI not found at {:?}. Install with:\n\
             cd ~ && git clone https://github.com/thetu/notebooklm-mcp && npm install && npm run build",
            cli_path
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_prefix() {
        let artifact = Artifact::new("Churn Metrics", "content".to_string());
        assert_eq!(artifact.title, "[ARCH] Churn Metrics");
    }
}
