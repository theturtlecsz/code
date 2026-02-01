//! SPEC-KIT-978: Reflex Configuration
//!
//! Provides shared configuration types and loading for reflex (local inference) mode.
//! Used by both CLI (codex-cli) and TUI (codex-tui) for consistency.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Reflex configuration from model_policy.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexConfig {
    /// Whether reflex mode is enabled
    pub enabled: bool,
    /// OpenAI-compatible endpoint URL
    pub endpoint: String,
    /// Model to use for reflex inference
    pub model: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether JSON schema enforcement is required
    pub json_schema_required: bool,
    /// Whether to fall back to cloud on failure
    pub fallback_to_cloud: bool,
    /// Bakeoff thresholds
    pub thresholds: ReflexThresholds,
}

/// Bakeoff thresholds for reflex promotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexThresholds {
    /// P95 latency threshold in ms
    pub p95_latency_ms: u64,
    /// Success parity percentage (0-100)
    pub success_parity_percent: u8,
    /// JSON schema compliance percentage (0-100)
    pub json_schema_compliance_percent: u8,
}

impl Default for ReflexConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://127.0.0.1:3009/v1".to_string(),
            model: "qwen2.5-coder-7b-instruct".to_string(),
            timeout_ms: 1500,
            json_schema_required: true,
            fallback_to_cloud: true,
            thresholds: ReflexThresholds::default(),
        }
    }
}

impl Default for ReflexThresholds {
    fn default() -> Self {
        Self {
            p95_latency_ms: 2000,
            success_parity_percent: 85,
            json_schema_compliance_percent: 100,
        }
    }
}

/// Load reflex configuration from model_policy.toml
///
/// Searches in order:
/// 1. Explicit config path (if provided)
/// 2. ./model_policy.toml (current directory)
/// 3. ~/.config/code/model_policy.toml
///
/// Returns defaults if no config found.
pub fn load_reflex_config(config_path: Option<&PathBuf>) -> Result<ReflexConfig, String> {
    let paths_to_try: Vec<PathBuf> = if let Some(path) = config_path {
        vec![path.clone()]
    } else {
        let mut paths = vec![PathBuf::from("model_policy.toml")];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".config").join("code").join("model_policy.toml"));
        }
        paths
    };

    for path in &paths_to_try {
        if path.exists() {
            return parse_model_policy_toml(path);
        }
    }

    // No config found - return defaults with a warning
    tracing::warn!("No model_policy.toml found, using defaults");
    Ok(ReflexConfig::default())
}

/// Parse model_policy.toml and extract reflex configuration
fn parse_model_policy_toml(path: &PathBuf) -> Result<ReflexConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let toml: toml::Value =
        toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {e}"))?;

    // Extract routing.reflex section
    let reflex = toml
        .get("routing")
        .and_then(|r| r.get("reflex"))
        .ok_or("Missing [routing.reflex] section in model_policy.toml")?;

    let thresholds_section = reflex.get("thresholds");

    Ok(ReflexConfig {
        enabled: reflex
            .get("enabled")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false),
        endpoint: reflex
            .get("endpoint")
            .and_then(toml::Value::as_str)
            .unwrap_or("http://127.0.0.1:3009/v1")
            .to_string(),
        model: reflex
            .get("model")
            .and_then(toml::Value::as_str)
            .unwrap_or("qwen2.5-coder-7b-instruct")
            .to_string(),
        timeout_ms: reflex
            .get("timeout_ms")
            .and_then(toml::Value::as_integer)
            .map(|v| v as u64)
            .unwrap_or(1500),
        json_schema_required: reflex
            .get("json_schema_required")
            .and_then(toml::Value::as_bool)
            .unwrap_or(true),
        fallback_to_cloud: reflex
            .get("fallback_to_cloud")
            .and_then(toml::Value::as_bool)
            .unwrap_or(true),
        thresholds: ReflexThresholds {
            p95_latency_ms: thresholds_section
                .and_then(|t| t.get("p95_latency_ms"))
                .and_then(toml::Value::as_integer)
                .map(|v| v as u64)
                .unwrap_or(2000),
            success_parity_percent: thresholds_section
                .and_then(|t| t.get("success_parity_percent"))
                .and_then(toml::Value::as_integer)
                .map(|v| v as u8)
                .unwrap_or(85),
            json_schema_compliance_percent: thresholds_section
                .and_then(|t| t.get("json_schema_compliance_percent"))
                .and_then(toml::Value::as_integer)
                .map(|v| v as u8)
                .unwrap_or(100),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_reflex_config() {
        let config = ReflexConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.endpoint, "http://127.0.0.1:3009/v1");
        assert_eq!(config.model, "qwen2.5-coder-7b-instruct");
        assert_eq!(config.timeout_ms, 1500);
        assert!(config.json_schema_required);
        assert!(config.fallback_to_cloud);
    }

    #[test]
    fn test_parse_model_policy_toml() {
        let toml_content = r#"[routing.reflex]
enabled = true
endpoint = "http://localhost:8080/v1"
model = "test-model"
timeout_ms = 2000
json_schema_required = false
fallback_to_cloud = true

[routing.reflex.thresholds]
p95_latency_ms = 1500
success_parity_percent = 90
json_schema_compliance_percent = 95
"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("model_policy.toml");
        std::fs::write(&config_path, toml_content).unwrap();

        let config = parse_model_policy_toml(&config_path).unwrap();
        assert!(config.enabled);
        assert_eq!(config.endpoint, "http://localhost:8080/v1");
        assert_eq!(config.model, "test-model");
        assert_eq!(config.timeout_ms, 2000);
        assert!(!config.json_schema_required);
        assert!(config.fallback_to_cloud);
        assert_eq!(config.thresholds.p95_latency_ms, 1500);
        assert_eq!(config.thresholds.success_parity_percent, 90);
        assert_eq!(config.thresholds.json_schema_compliance_percent, 95);
    }
}
