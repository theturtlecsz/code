use crate::local_memory_util::{LocalMemorySearchResponse, LocalMemorySearchResult};
use std::process::Command;
use std::time::Duration;

const LOCAL_MEMORY_REST_HEALTH_URL: &str = "http://localhost:3002/api/v1/health";

fn local_memory_bin() -> String {
    std::env::var("LOCAL_MEMORY_BIN").unwrap_or_else(|_| "local-memory".to_string())
}

pub(crate) fn local_memory_daemon_healthy_blocking(timeout: Duration) -> bool {
    if std::env::var("LOCAL_MEMORY_SKIP_HEALTHCHECK")
        .ok()
        .is_some_and(|v| v == "1")
    {
        return true;
    }

    // SPEC-KIT-900 FIX: Use block_in_place to allow blocking reqwest calls
    // within an async tokio context. Without this, reqwest::blocking::Client::new()
    // tries to create its own runtime and panics with "Cannot drop a runtime
    // in a context where blocking is not allowed".
    tokio::task::block_in_place(|| {
        reqwest::blocking::Client::new()
            .get(LOCAL_MEMORY_REST_HEALTH_URL)
            .timeout(timeout)
            .send()
            .map(|resp| resp.status().is_success())
            .unwrap_or(false)
    })
}

pub(crate) async fn local_memory_daemon_healthy(timeout: Duration) -> bool {
    if std::env::var("LOCAL_MEMORY_SKIP_HEALTHCHECK")
        .ok()
        .is_some_and(|v| v == "1")
    {
        return true;
    }

    let client = reqwest::Client::new();
    client
        .get(LOCAL_MEMORY_REST_HEALTH_URL)
        .timeout(timeout)
        .send()
        .await
        .map(|resp| resp.status().is_success())
        .unwrap_or(false)
}

fn parse_search_stdout(stdout: &[u8]) -> Result<Vec<LocalMemorySearchResult>, String> {
    let response: LocalMemorySearchResponse = serde_json::from_slice(stdout)
        .map_err(|e| format!("local-memory JSON parse failed: {e}"))?;
    if response.success {
        Ok(response.data.map(|d| d.results).unwrap_or_default())
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "local-memory search failed".to_string()))
    }
}

pub(crate) fn search_blocking(
    query: &str,
    limit: usize,
    tags: &[String],
    domain: Option<&str>,
    max_content_length: usize,
) -> Result<Vec<LocalMemorySearchResult>, String> {
    let mut cmd = Command::new(local_memory_bin());
    cmd.arg("search")
        .arg(query)
        .arg("--limit")
        .arg(limit.to_string())
        .arg("--json")
        .arg("--fields")
        .arg("id,content,importance,tags,domain,created_at,updated_at")
        .arg("--max_content_length")
        .arg(max_content_length.to_string());

    if !tags.is_empty() {
        cmd.arg("--tags").arg(tags.join(","));
    }
    if let Some(domain) = domain {
        cmd.arg("--domain").arg(domain);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to execute local-memory search: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "local-memory search failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    parse_search_stdout(&output.stdout)
}

pub(crate) async fn search(
    query: &str,
    limit: usize,
    tags: &[String],
    domain: Option<&str>,
    max_content_length: usize,
) -> Result<Vec<LocalMemorySearchResult>, String> {
    let mut cmd = tokio::process::Command::new(local_memory_bin());
    cmd.arg("search")
        .arg(query)
        .arg("--limit")
        .arg(limit.to_string())
        .arg("--json")
        .arg("--fields")
        .arg("id,content,importance,tags,domain,created_at,updated_at")
        .arg("--max_content_length")
        .arg(max_content_length.to_string());

    if !tags.is_empty() {
        cmd.arg("--tags").arg(tags.join(","));
    }
    if let Some(domain) = domain {
        cmd.arg("--domain").arg(domain);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("failed to execute local-memory search: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "local-memory search failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    parse_search_stdout(&output.stdout)
}
