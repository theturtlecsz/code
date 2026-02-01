//! SPEC-KIT-978: Reflex Routing Decision Module
//!
//! Determines whether the Implementer role should use local reflex inference
//! or fall back to cloud inference. Emits RoutingDecision capsule events
//! for every routing attempt.
//!
//! ## Routing Rules
//!
//! 1. Reflex is ONLY valid for the Implement stage
//! 2. Reflex must be enabled in model_policy.toml
//! 3. Reflex server must be healthy (reachable + model available)
//! 4. If any condition fails, fall back to cloud with recorded reason

use crate::memvid_adapter::{
    CapsuleHandle, RoutingDecisionPayload, RoutingFallbackReason, RoutingMode,
};
use codex_stage0::{ReflexConfig, load_reflex_config};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Result of a reflex health check
#[derive(Debug, Clone)]
pub struct ReflexHealthResult {
    pub healthy: bool,
    pub server_reachable: bool,
    pub model_available: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Synchronous reflex health check
///
/// Calls GET /v1/models and verifies the configured model is available.
pub fn check_reflex_health(config: &ReflexConfig) -> ReflexHealthResult {
    let models_url = format!("{}/models", config.endpoint.trim_end_matches('/'));
    let timeout = Duration::from_millis(config.timeout_ms.max(5000));

    let start = Instant::now();
    let client = match reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ReflexHealthResult {
                healthy: false,
                server_reachable: false,
                model_available: false,
                latency_ms: None,
                error: Some(format!("Failed to create HTTP client: {}", e)),
            };
        }
    };

    match client.get(&models_url).send() {
        Ok(resp) if resp.status().is_success() => {
            let latency_ms = start.elapsed().as_millis() as u64;

            // Parse response to check if model is available
            #[derive(serde::Deserialize)]
            struct ModelsResponse {
                data: Vec<ModelInfo>,
            }
            #[derive(serde::Deserialize)]
            struct ModelInfo {
                id: String,
            }

            match resp.json::<ModelsResponse>() {
                Ok(models) => {
                    let model_available = models.data.iter().any(|m| m.id == config.model);
                    ReflexHealthResult {
                        healthy: model_available,
                        server_reachable: true,
                        model_available,
                        latency_ms: Some(latency_ms),
                        error: if !model_available {
                            Some(format!(
                                "Model '{}' not found in available models",
                                config.model
                            ))
                        } else {
                            None
                        },
                    }
                }
                Err(e) => ReflexHealthResult {
                    healthy: false,
                    server_reachable: true,
                    model_available: false,
                    latency_ms: Some(latency_ms),
                    error: Some(format!("Failed to parse models response: {}", e)),
                },
            }
        }
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            ReflexHealthResult {
                healthy: false,
                server_reachable: true,
                model_available: false,
                latency_ms: Some(latency_ms),
                error: Some(format!("Server returned HTTP {}", resp.status())),
            }
        }
        Err(e) => ReflexHealthResult {
            healthy: false,
            server_reachable: false,
            model_available: false,
            latency_ms: None,
            error: Some(format!("Connection failed: {}", e)),
        },
    }
}

/// Routing decision result
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Selected routing mode
    pub mode: RoutingMode,
    /// Whether this was a fallback from reflex
    pub is_fallback: bool,
    /// Reason for fallback (if applicable)
    pub fallback_reason: Option<RoutingFallbackReason>,
    /// Reflex config (if reflex mode selected or attempted)
    pub reflex_config: Option<ReflexConfig>,
    /// Health check result (if reflex was attempted)
    pub health_result: Option<ReflexHealthResult>,
    /// Cloud model to use (if cloud mode)
    pub cloud_model: Option<String>,
}

/// Make a routing decision for the Implementer role.
///
/// ## Parameters
/// - `stage`: Current pipeline stage (must be "implement" for reflex)
/// - `cloud_model`: Default cloud model to use as fallback
/// - `config_path`: Optional override for model_policy.toml path
///
/// ## Returns
/// A RoutingDecision indicating whether to use reflex or cloud.
pub fn decide_implementer_routing(
    stage: &str,
    cloud_model: &str,
    config_path: Option<&PathBuf>,
) -> RoutingDecision {
    // Rule 1: Reflex is ONLY valid for Implement stage
    if stage.to_lowercase() != "implement" {
        return RoutingDecision {
            mode: RoutingMode::Cloud,
            is_fallback: false, // Not a fallback - reflex isn't applicable
            fallback_reason: Some(RoutingFallbackReason::NotImplementStage),
            reflex_config: None,
            health_result: None,
            cloud_model: Some(cloud_model.to_string()),
        };
    }

    // Load reflex config
    let config = match load_reflex_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to load reflex config: {}", e);
            return RoutingDecision {
                mode: RoutingMode::Cloud,
                is_fallback: false,
                fallback_reason: Some(RoutingFallbackReason::ReflexDisabled),
                reflex_config: None,
                health_result: None,
                cloud_model: Some(cloud_model.to_string()),
            };
        }
    };

    // Rule 2: Reflex must be enabled
    if !config.enabled {
        return RoutingDecision {
            mode: RoutingMode::Cloud,
            is_fallback: false,
            fallback_reason: Some(RoutingFallbackReason::ReflexDisabled),
            reflex_config: Some(config),
            health_result: None,
            cloud_model: Some(cloud_model.to_string()),
        };
    }

    // Rule 3: Check reflex health
    let health = check_reflex_health(&config);

    if !health.healthy {
        let reason = if !health.server_reachable {
            RoutingFallbackReason::ServerUnhealthy
        } else {
            RoutingFallbackReason::ModelNotAvailable
        };

        return RoutingDecision {
            mode: RoutingMode::Cloud,
            is_fallback: true, // This IS a fallback - reflex was attempted
            fallback_reason: Some(reason),
            reflex_config: Some(config),
            health_result: Some(health),
            cloud_model: Some(cloud_model.to_string()),
        };
    }

    // Rule 4: Check bakeoff thresholds (only if we have enough samples)
    if let Some(threshold_failure) = check_bakeoff_thresholds(&config) {
        tracing::info!(
            "SPEC-KIT-978: Reflex threshold failure: {:?}",
            threshold_failure
        );

        return RoutingDecision {
            mode: RoutingMode::Cloud,
            is_fallback: true, // This IS a fallback - threshold check failed
            fallback_reason: Some(threshold_failure),
            reflex_config: Some(config),
            health_result: Some(health),
            cloud_model: Some(cloud_model.to_string()),
        };
    }

    // All checks passed - use reflex
    RoutingDecision {
        mode: RoutingMode::Reflex,
        is_fallback: false,
        fallback_reason: None,
        reflex_config: Some(config),
        health_result: Some(health),
        cloud_model: None,
    }
}

/// Check bakeoff thresholds and return fallback reason if any threshold fails.
///
/// Returns None if thresholds are met or if there aren't enough samples.
/// Returns Some(RoutingFallbackReason) if a threshold is violated.
fn check_bakeoff_thresholds(config: &ReflexConfig) -> Option<RoutingFallbackReason> {
    use super::reflex_metrics::get_metrics_db;

    // Skip threshold check if minimum samples not configured (default 10)
    let min_samples = 10u64;
    let since = Duration::from_secs(24 * 3600); // 24 hours

    let db = match get_metrics_db() {
        Ok(db) => db,
        Err(e) => {
            tracing::debug!(
                "SPEC-KIT-978: Skipping threshold check - metrics DB not available: {}",
                e
            );
            return None; // Can't check thresholds, proceed with reflex
        }
    };

    let stats = match db.compute_bakeoff_stats(since) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!(
                "SPEC-KIT-978: Skipping threshold check - stats computation failed: {}",
                e
            );
            return None;
        }
    };

    let reflex_stats = match &stats.reflex {
        Some(r) => r,
        None => {
            tracing::debug!("SPEC-KIT-978: Skipping threshold check - no reflex samples");
            return None; // No data to check against
        }
    };

    // Check minimum sample count
    if reflex_stats.total_attempts < min_samples {
        tracing::debug!(
            "SPEC-KIT-978: Skipping threshold check - insufficient samples ({} < {})",
            reflex_stats.total_attempts,
            min_samples
        );
        return None; // Not enough data to make a decision
    }

    let thresholds = &config.thresholds;

    // Check P95 latency
    if reflex_stats.p95_latency_ms > thresholds.p95_latency_ms {
        tracing::warn!(
            "SPEC-KIT-978: P95 latency {}ms exceeds threshold {}ms",
            reflex_stats.p95_latency_ms,
            thresholds.p95_latency_ms
        );
        return Some(RoutingFallbackReason::LatencyThresholdExceeded);
    }

    // Check success rate
    if reflex_stats.success_rate < thresholds.success_parity_percent as f64 {
        tracing::warn!(
            "SPEC-KIT-978: Success rate {:.1}% below threshold {}%",
            reflex_stats.success_rate,
            thresholds.success_parity_percent
        );
        return Some(RoutingFallbackReason::SuccessRateBelowThreshold);
    }

    // Check JSON compliance
    if reflex_stats.json_compliance_rate < thresholds.json_schema_compliance_percent as f64 {
        tracing::warn!(
            "SPEC-KIT-978: JSON compliance {:.1}% below threshold {}%",
            reflex_stats.json_compliance_rate,
            thresholds.json_schema_compliance_percent
        );
        return Some(RoutingFallbackReason::JsonComplianceBelowThreshold);
    }

    // All thresholds met
    None
}

/// Emit a RoutingDecision event to the capsule.
///
/// Should be called whenever a routing decision is made during
/// the Implement stage.
pub fn emit_routing_event(
    capsule: &CapsuleHandle,
    spec_id: &str,
    run_id: &str,
    stage: &str,
    role: &str,
    decision: &RoutingDecision,
) -> anyhow::Result<()> {
    let payload = RoutingDecisionPayload {
        mode: decision.mode,
        stage: stage.to_string(),
        role: role.to_string(),
        is_fallback: decision.is_fallback,
        fallback_reason: decision.fallback_reason.clone(),
        reflex_endpoint: decision.reflex_config.as_ref().map(|c| c.endpoint.clone()),
        reflex_model: decision.reflex_config.as_ref().map(|c| c.model.clone()),
        cloud_model: decision.cloud_model.clone(),
        health_check_latency_ms: decision.health_result.as_ref().and_then(|h| h.latency_ms),
    };

    capsule.emit_routing_decision(spec_id, run_id, &payload)?;

    tracing::info!(
        "SPEC-KIT-978: Emitted RoutingDecision event: mode={}, is_fallback={}, stage={}",
        decision.mode.as_str(),
        decision.is_fallback,
        stage
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_not_implement_stage() {
        let decision = decide_implementer_routing("plan", "claude-3-opus", None);
        assert_eq!(decision.mode, RoutingMode::Cloud);
        assert!(!decision.is_fallback);
        assert!(matches!(
            decision.fallback_reason,
            Some(RoutingFallbackReason::NotImplementStage)
        ));
    }

    #[test]
    fn test_routing_reflex_disabled() {
        // With default config (reflex disabled), should fall back to cloud
        let decision = decide_implementer_routing("implement", "claude-3-opus", None);
        assert_eq!(decision.mode, RoutingMode::Cloud);
        // Not a fallback because reflex wasn't attempted (disabled)
        assert!(!decision.is_fallback);
    }

    #[test]
    fn test_routing_decision_payload_serialization() {
        let payload = RoutingDecisionPayload {
            mode: RoutingMode::Cloud,
            stage: "implement".to_string(),
            role: "Implementer".to_string(),
            is_fallback: true,
            fallback_reason: Some(RoutingFallbackReason::ServerUnhealthy),
            reflex_endpoint: Some("http://localhost:3009/v1".to_string()),
            reflex_model: Some("qwen2.5-coder-7b-instruct".to_string()),
            cloud_model: Some("claude-3-opus".to_string()),
            health_check_latency_ms: None,
        };

        let json = serde_json::to_string_pretty(&payload).unwrap();
        assert!(json.contains("\"mode\": \"Cloud\"")); // Note: spaces in pretty-print
        assert!(json.contains("\"is_fallback\": true"));
        assert!(json.contains("\"fallback_reason\": \"ServerUnhealthy\""));
    }

    #[test]
    fn test_routing_decision_mode_branching() {
        // Test that RoutingDecision mode can be matched for branching
        let cloud_decision = RoutingDecision {
            mode: RoutingMode::Cloud,
            is_fallback: false,
            fallback_reason: Some(RoutingFallbackReason::ReflexDisabled),
            reflex_config: None,
            health_result: None,
            cloud_model: Some("claude-3-opus".to_string()),
        };

        // Verify cloud mode is detected correctly
        assert_eq!(cloud_decision.mode, RoutingMode::Cloud);
        assert!(cloud_decision.cloud_model.is_some());

        // Test reflex decision (simulated - would come from healthy reflex)
        let reflex_decision = RoutingDecision {
            mode: RoutingMode::Reflex,
            is_fallback: false,
            fallback_reason: None,
            reflex_config: Some(codex_stage0::ReflexConfig {
                enabled: true,
                endpoint: "http://localhost:3009/v1".to_string(),
                model: "qwen2.5-coder-7b-instruct".to_string(),
                timeout_ms: 30000,
                json_schema_required: true,
                fallback_to_cloud: true,
                thresholds: codex_stage0::ReflexThresholds::default(),
            }),
            health_result: Some(ReflexHealthResult {
                healthy: true,
                server_reachable: true,
                model_available: true,
                latency_ms: Some(50),
                error: None,
            }),
            cloud_model: None,
        };

        // Verify reflex mode is detected correctly
        assert_eq!(reflex_decision.mode, RoutingMode::Reflex);
        assert!(reflex_decision.reflex_config.is_some());
        assert!(reflex_decision.health_result.as_ref().unwrap().healthy);
    }

    #[test]
    fn test_fallback_reason_variants() {
        // Ensure all fallback reasons serialize correctly
        let reasons = vec![
            RoutingFallbackReason::ReflexDisabled,
            RoutingFallbackReason::ServerUnhealthy,
            RoutingFallbackReason::ModelNotAvailable,
            RoutingFallbackReason::LatencyThresholdExceeded,
            RoutingFallbackReason::SuccessRateBelowThreshold,
            RoutingFallbackReason::JsonComplianceBelowThreshold,
            RoutingFallbackReason::NotImplementStage,
        ];

        for reason in reasons {
            let json = serde_json::to_string(&reason).unwrap();
            assert!(!json.is_empty());
            // Should be serializable as a string
            let parsed: RoutingFallbackReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason.as_str(), parsed.as_str());
        }
    }

    #[test]
    fn test_routing_mode_string_representation() {
        assert_eq!(RoutingMode::Cloud.as_str(), "cloud");
        assert_eq!(RoutingMode::Reflex.as_str(), "reflex");
    }
}
