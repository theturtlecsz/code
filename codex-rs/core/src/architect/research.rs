//! Research API client for NotebookLM.
//!
//! Connects to notebooklm-mcp service HTTP API for research operations.
//! - `fast` - Quick parallel web search
//! - `deep` - Multi-step autonomous research
//! - `status` - Check research progress
//! - `results` - Get completed research results
//! - `import` - Import results as notebook sources

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const RESEARCH_TIMEOUT: Duration = Duration::from_secs(300); // 5 min for deep research

/// Request body for fast research.
#[derive(Debug, Serialize)]
struct FastResearchRequest {
    query: String,
    notebook: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    wait: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_ms: Option<u64>,
}

/// Request body for deep research.
#[derive(Debug, Serialize)]
struct DeepResearchRequest {
    query: String,
    notebook: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    wait: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edit_plan: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_ms: Option<u64>,
}

/// Request body for import.
#[derive(Debug, Serialize)]
struct ImportRequest {
    notebook: String,
}

/// Generic API response wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Research result data.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchData {
    /// Research status: "pending", "running", "completed", "failed"
    pub status: Option<String>,
    /// Progress percentage (0-100)
    pub progress: Option<u32>,
    /// Research query
    pub query: Option<String>,
    /// Research results (when completed)
    pub results: Option<ResearchResults>,
    /// Error message (when failed)
    pub error: Option<String>,
}

/// Research results content.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchResults {
    /// Summary of findings
    pub summary: Option<String>,
    /// Sources found
    pub sources: Option<Vec<ResearchSource>>,
    /// Number of sources found
    pub source_count: Option<usize>,
}

/// Research source entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSource {
    pub title: Option<String>,
    pub url: Option<String>,
    pub snippet: Option<String>,
}

/// Research status response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchStatus {
    pub status: String,
    pub progress: Option<u32>,
    pub query: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

/// Import results response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportData {
    pub imported: Option<usize>,
    pub sources: Option<Vec<String>>,
}

/// Research API client.
pub struct ResearchClient {
    base_url: String,
    client: Client,
    notebook: String,
}

impl ResearchClient {
    /// Create a new research client.
    pub fn new(base_url: &str, notebook: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            base_url: base_url.to_string(),
            client,
            notebook: notebook.to_string(),
        })
    }

    /// Create with custom port.
    pub fn with_port(port: u16, notebook: &str) -> Result<Self> {
        Self::new(&format!("http://127.0.0.1:{}", port), notebook)
    }

    /// Check if service is running.
    pub async fn is_running(&self) -> bool {
        let health_url = format!("{}/health", self.base_url);
        let result = self
            .client
            .get(&health_url)
            .timeout(Duration::from_secs(2))
            .send()
            .await;

        match result {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Trigger fast research (quick parallel search).
    pub async fn fast(&self, query: &str, wait: bool) -> Result<ResearchData> {
        let url = format!("{}/api/research/fast", self.base_url);
        let req = FastResearchRequest {
            query: query.to_string(),
            notebook: self.notebook.clone(),
            wait: Some(wait),
            timeout_ms: Some(RESEARCH_TIMEOUT.as_millis() as u64),
        };

        let resp = self
            .client
            .post(&url)
            .timeout(RESEARCH_TIMEOUT)
            .json(&req)
            .send()
            .await
            .context("Failed to send fast research request")?;

        if !resp.status().is_success() {
            bail!("Fast research failed: {}", resp.status());
        }

        let result: ApiResponse<ResearchData> = resp
            .json()
            .await
            .context("Failed to parse research response")?;

        if !result.success {
            bail!(
                "Research error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        result.data.context("No data in response")
    }

    /// Trigger deep research (multi-step autonomous).
    pub async fn deep(&self, query: &str, wait: bool, edit_plan: bool) -> Result<ResearchData> {
        let url = format!("{}/api/research/deep", self.base_url);
        let req = DeepResearchRequest {
            query: query.to_string(),
            notebook: self.notebook.clone(),
            wait: Some(wait),
            edit_plan: Some(edit_plan),
            timeout_ms: Some(RESEARCH_TIMEOUT.as_millis() as u64),
        };

        let resp = self
            .client
            .post(&url)
            .timeout(RESEARCH_TIMEOUT)
            .json(&req)
            .send()
            .await
            .context("Failed to send deep research request")?;

        if !resp.status().is_success() {
            bail!("Deep research failed: {}", resp.status());
        }

        let result: ApiResponse<ResearchData> = resp
            .json()
            .await
            .context("Failed to parse research response")?;

        if !result.success {
            bail!(
                "Research error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        result.data.context("No data in response")
    }

    /// Get current research status.
    pub async fn status(&self) -> Result<ResearchStatus> {
        let url = format!(
            "{}/api/research/status?notebook={}",
            self.base_url,
            urlencoding::encode(&self.notebook)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get research status")?;

        if !resp.status().is_success() {
            bail!("Status check failed: {}", resp.status());
        }

        let result: ApiResponse<ResearchStatus> = resp
            .json()
            .await
            .context("Failed to parse status response")?;

        if !result.success {
            bail!(
                "Status error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        result.data.context("No data in response")
    }

    /// Get research results.
    pub async fn results(&self, format: &str) -> Result<ResearchData> {
        let url = format!(
            "{}/api/research/results?notebook={}&format={}",
            self.base_url,
            urlencoding::encode(&self.notebook),
            format
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get research results")?;

        if !resp.status().is_success() {
            bail!("Results request failed: {}", resp.status());
        }

        let result: ApiResponse<ResearchData> = resp
            .json()
            .await
            .context("Failed to parse results response")?;

        if !result.success {
            bail!(
                "Results error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        result.data.context("No data in response")
    }

    /// Import research results as notebook sources.
    pub async fn import(&self) -> Result<ImportData> {
        let url = format!("{}/api/research/import", self.base_url);
        let req = ImportRequest {
            notebook: self.notebook.clone(),
        };

        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to import research results")?;

        if !resp.status().is_success() {
            bail!("Import failed: {}", resp.status());
        }

        let result: ApiResponse<ImportData> = resp
            .json()
            .await
            .context("Failed to parse import response")?;

        if !result.success {
            bail!(
                "Import error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        result.data.context("No data in response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_client_creation() {
        let client = ResearchClient::new("http://localhost:3456", "test-notebook");
        assert!(client.is_ok());
    }

    #[test]
    fn test_research_client_with_port() {
        let client = ResearchClient::with_port(3456, "test-notebook");
        assert!(client.is_ok());
        assert!(client.unwrap().base_url.contains("3456"));
    }
}
