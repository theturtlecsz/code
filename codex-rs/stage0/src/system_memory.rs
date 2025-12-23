//! System pointer memory storage for Stage0 artifacts
//!
//! CONVERGENCE: Stores Stage0 outputs as pointer memories in local-memory.
//! Per MEMO_codex-rs.md Section 3:
//! - domain: spec-tracker
//! - tags: system:true, spec:<id>, stage:0, artifact:<type>
//! - content: pointers + short summary (no raw Divine Truth text)
//!
//! These memories enable traceability without polluting normal recall.

use crate::errors::{Result, Stage0Error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Artifact types for system pointer memories
#[derive(Debug, Clone, Copy)]
pub enum ArtifactType {
    /// TASK_BRIEF.md from DCC
    TaskBrief,
    /// DIVINE_TRUTH.md from Tier2
    DivineTruth,
    /// Combined pointer for both artifacts
    Combined,
}

impl ArtifactType {
    /// Get the tag value for this artifact type
    pub fn tag(&self) -> &'static str {
        match self {
            Self::TaskBrief => "artifact:task_brief",
            Self::DivineTruth => "artifact:divine_truth",
            Self::Combined => "artifact:combined",
        }
    }
}

/// Response from local-memory store endpoint
#[derive(Debug, Deserialize)]
struct StoreResponse {
    success: bool,
    data: Option<StoreData>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StoreData {
    id: String,
}

/// Compute SHA-256 hash of content for pointer reference
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Store a system pointer memory for Stage0 output
///
/// # Arguments
/// * `api_base` - local-memory REST API base URL
/// * `spec_id` - SPEC identifier (e.g., "SPEC-KIT-102")
/// * `artifact_type` - Type of artifact being referenced
/// * `file_path` - Optional file path where artifact is stored
/// * `content_hash` - SHA-256 hash of the artifact content
/// * `summary_bullets` - 2-5 bullet point summary
///
/// # Returns
/// Memory ID on success
pub async fn store_system_pointer(
    api_base: &str,
    spec_id: &str,
    artifact_type: ArtifactType,
    file_path: Option<&str>,
    content_hash: &str,
    summary_bullets: &[String],
) -> Result<String> {
    let tags = vec![
        "system:true".to_string(),
        format!("spec:{}", spec_id),
        "stage:0".to_string(),
        artifact_type.tag().to_string(),
    ];

    let path_line = file_path
        .map(|p| format!("**Path**: {p}\n"))
        .unwrap_or_default();

    let bullets = summary_bullets
        .iter()
        .map(|b| {
            let trimmed = b.trim_start_matches(['-', '*', ' ']);
            format!("- {trimmed}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let artifact_tag = artifact_type.tag();
    let content = format!(
        "## Stage0 Artifact Pointer: {spec_id} ({artifact_tag})\n\n\
         **Content hash**: {content_hash}\n\
         {path_line}\n\
         ### Summary\n{bullets}\n",
    );

    // POST to local-memory REST API
    let client = reqwest::Client::new();
    let url = format!("{}/memories", api_base.trim_end_matches('/'));

    #[derive(Serialize)]
    struct StoreRequest {
        content: String,
        domain: String,
        tags: Vec<String>,
        importance: u8,
    }

    let body = StoreRequest {
        content,
        domain: "spec-tracker".to_string(),
        tags,
        importance: 5, // Lower importance - metadata only
    };

    let resp = client
        .post(&url)
        .json(&body)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| Stage0Error::local_memory(format!("POST memory failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(Stage0Error::local_memory(format!(
            "POST memory failed: {}",
            resp.status()
        )));
    }

    let parsed: StoreResponse = resp
        .json()
        .await
        .map_err(|e| Stage0Error::local_memory(format!("Parse response failed: {e}")))?;

    if !parsed.success {
        return Err(Stage0Error::local_memory(format!(
            "Store failed: {}",
            parsed.error.unwrap_or_else(|| "unknown error".to_string())
        )));
    }

    let id = parsed
        .data
        .map(|d| d.id)
        .unwrap_or_else(|| "unknown".to_string());

    tracing::debug!(
        spec_id = %spec_id,
        artifact = %artifact_type.tag(),
        memory_id = %id,
        "Stored system pointer memory"
    );

    Ok(id)
}

/// Extract summary bullets from Divine Truth executive summary
pub fn extract_summary_bullets(divine_truth: &str, max_bullets: usize) -> Vec<String> {
    divine_truth
        .lines()
        .filter(|l| l.trim().starts_with('-') || l.trim().starts_with('*'))
        .take(max_bullets)
        .map(|l| l.trim().to_string())
        .collect()
}

/// Tier2 execution status for pointer memory tagging
#[derive(Debug, Clone)]
pub enum Tier2Status {
    /// Tier2 ran successfully
    Success,
    /// Tier2 was skipped with a reason
    Skipped(String),
    /// Tier2 failed with an error
    Error(String),
}

impl Tier2Status {
    /// Get the tag value for this status
    pub fn tag(&self) -> String {
        match self {
            Self::Success => "tier2:success".to_string(),
            Self::Skipped(_) => "tier2:skipped".to_string(),
            Self::Error(_) => "tier2:error".to_string(),
        }
    }

    /// Get the reason/detail if any
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Success => None,
            Self::Skipped(r) | Self::Error(r) => Some(r),
        }
    }
}

/// Information for storing Stage0 execution pointer
#[derive(Debug, Clone)]
pub struct Stage0PointerInfo {
    /// SPEC identifier (e.g., "SPEC-KIT-102")
    pub spec_id: String,
    /// Path to TASK_BRIEF.md (optional if not written)
    pub task_brief_path: Option<String>,
    /// Path to DIVINE_TRUTH.md (optional if not written)
    pub divine_truth_path: Option<String>,
    /// Content hash of task brief
    pub task_brief_hash: String,
    /// Content hash of divine truth (if any)
    pub divine_truth_hash: Option<String>,
    /// Summary bullets extracted from divine truth
    pub summary_bullets: Vec<String>,
    /// Tier2 execution status
    pub tier2_status: Tier2Status,
    /// Optional notebook ID used for Tier2
    pub notebook_id: Option<String>,
    /// Git commit SHA at time of execution (optional)
    pub commit_sha: Option<String>,
}

/// Store a Stage0 execution pointer memory (best-effort)
///
/// This is the main entry point called after Stage0 completes and artifacts
/// are written to disk. It stores a combined pointer memory with all execution
/// metadata.
///
/// # Best-Effort Semantics
/// This function logs errors but never fails the caller. If local-memory is
/// down or returns an error, the Stage0 pipeline continues without blocking.
///
/// # Returns
/// Memory ID if stored successfully, None on any error
pub async fn store_stage0_pointer(api_base: &str, info: &Stage0PointerInfo) -> Option<String> {
    // Build tags
    let spec_id = &info.spec_id;
    let mut tags = vec![
        "system:true".to_string(),
        format!("spec:{spec_id}"),
        "stage:0".to_string(),
        "type:milestone".to_string(),
        info.tier2_status.tag(),
    ];

    if let Some(ref notebook) = info.notebook_id {
        tags.push(format!("notebook:{notebook}"));
    }

    // Build content
    let task_brief_hash = &info.task_brief_hash;
    let mut content_parts = vec![
        format!("## Stage0 Execution Pointer: {spec_id}\n"),
        format!("**Task Brief Hash**: {task_brief_hash}\n"),
    ];

    if let Some(ref hash) = info.divine_truth_hash {
        content_parts.push(format!("**Divine Truth Hash**: {hash}\n"));
    }

    if let Some(ref path) = info.task_brief_path {
        content_parts.push(format!("**Task Brief Path**: {path}\n"));
    }

    if let Some(ref path) = info.divine_truth_path {
        content_parts.push(format!("**Divine Truth Path**: {path}\n"));
    }

    if let Some(ref sha) = info.commit_sha {
        content_parts.push(format!("**Commit**: {sha}\n"));
    }

    // Tier2 status line
    match &info.tier2_status {
        Tier2Status::Success => {
            content_parts.push("**Tier2**: ✓ Success\n".to_string());
        }
        Tier2Status::Skipped(reason) => {
            content_parts.push(format!("**Tier2**: ⊘ Skipped ({reason})\n"));
        }
        Tier2Status::Error(err) => {
            content_parts.push(format!("**Tier2**: ✗ Error ({err})\n"));
        }
    }

    // Summary bullets
    if !info.summary_bullets.is_empty() {
        content_parts.push("\n### Summary\n".to_string());
        for bullet in &info.summary_bullets {
            let normalized = bullet.trim_start_matches(['-', '*', ' ']);
            content_parts.push(format!("- {normalized}\n"));
        }
    }

    let content = content_parts.join("");

    // POST to local-memory REST API
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                spec_id = %info.spec_id,
                error = %e,
                "Failed to create HTTP client for system pointer"
            );
            return None;
        }
    };

    let url = format!("{}/memories", api_base.trim_end_matches('/'));

    #[derive(Serialize)]
    struct StoreRequest {
        content: String,
        domain: String,
        tags: Vec<String>,
        importance: u8,
    }

    let body = StoreRequest {
        content,
        domain: "spec-tracker".to_string(),
        tags,
        importance: 8, // High for traceability (but excluded by system:true anyway)
    };

    let resp = match client.post(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                spec_id = %info.spec_id,
                error = %e,
                "Failed to POST system pointer memory"
            );
            return None;
        }
    };

    if !resp.status().is_success() {
        tracing::warn!(
            spec_id = %info.spec_id,
            status = %resp.status(),
            "System pointer memory POST failed"
        );
        return None;
    }

    let parsed: StoreResponse = match resp.json().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                spec_id = %info.spec_id,
                error = %e,
                "Failed to parse system pointer response"
            );
            return None;
        }
    };

    if !parsed.success {
        tracing::warn!(
            spec_id = %info.spec_id,
            error = ?parsed.error,
            "System pointer memory store returned failure"
        );
        return None;
    }

    let id = parsed.data.map(|d| d.id)?;

    tracing::info!(
        spec_id = %info.spec_id,
        memory_id = %id,
        tier2_status = %info.tier2_status.tag(),
        "Stored Stage0 system pointer memory"
    );

    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash() {
        let hash = compute_content_hash("test content");
        assert_eq!(hash.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn test_artifact_type_tags() {
        assert_eq!(ArtifactType::TaskBrief.tag(), "artifact:task_brief");
        assert_eq!(ArtifactType::DivineTruth.tag(), "artifact:divine_truth");
        assert_eq!(ArtifactType::Combined.tag(), "artifact:combined");
    }

    #[test]
    fn test_extract_summary_bullets() {
        let text = "Executive Summary\n- Point one\n- Point two\n* Point three\nNot a bullet";
        let bullets = extract_summary_bullets(text, 5);
        assert_eq!(bullets.len(), 3);
        assert!(bullets[0].starts_with('-'));
    }

    #[test]
    fn test_tier2_status_tags() {
        assert_eq!(Tier2Status::Success.tag(), "tier2:success");
        assert_eq!(
            Tier2Status::Skipped("test".to_string()).tag(),
            "tier2:skipped"
        );
        assert_eq!(Tier2Status::Error("err".to_string()).tag(), "tier2:error");
    }

    #[test]
    fn test_tier2_status_reason() {
        assert!(Tier2Status::Success.reason().is_none());
        assert_eq!(
            Tier2Status::Skipped("no notebook".to_string()).reason(),
            Some("no notebook")
        );
        assert_eq!(
            Tier2Status::Error("timeout".to_string()).reason(),
            Some("timeout")
        );
    }

    #[test]
    fn test_pointer_info_construction() {
        let info = Stage0PointerInfo {
            spec_id: "SPEC-TEST-001".to_string(),
            task_brief_path: Some("/path/to/TASK_BRIEF.md".to_string()),
            divine_truth_path: None,
            task_brief_hash: "abc123".to_string(),
            divine_truth_hash: None,
            summary_bullets: vec!["Point 1".to_string(), "Point 2".to_string()],
            tier2_status: Tier2Status::Skipped("No notebook configured".to_string()),
            notebook_id: None,
            commit_sha: Some("abc1234".to_string()),
        };

        assert_eq!(info.spec_id, "SPEC-TEST-001");
        assert!(info.task_brief_path.is_some());
        assert!(info.divine_truth_path.is_none());
        assert_eq!(info.summary_bullets.len(), 2);
        assert!(matches!(info.tier2_status, Tier2Status::Skipped(_)));
    }
}
