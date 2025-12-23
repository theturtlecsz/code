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
        .map(|p| format!("**Path**: {}\n", p))
        .unwrap_or_default();

    let bullets = summary_bullets
        .iter()
        .map(|b| format!("- {}", b.trim_start_matches(|c| c == '-' || c == '*' || c == ' ')))
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        "## Stage0 Artifact Pointer: {} ({})\n\n\
         **Content hash**: {}\n\
         {}\n\
         ### Summary\n{}\n",
        spec_id,
        artifact_type.tag(),
        content_hash,
        path_line,
        bullets,
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
}
