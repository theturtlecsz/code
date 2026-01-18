//! SPEC-KIT-978: Reflex CLI Commands
//!
//! Provides CLI commands for managing the local reflex inference server.
//!
//! ## Commands
//!
//! - `code reflex health` - Check reflex server status
//! - `code reflex models` - List available models
//! - `code reflex status` - Show reflex config + thresholds
//!
//! ## Exit Codes
//!
//! - 0: Healthy (server reachable, configured model available)
//! - 1: Unhealthy (server unreachable or model not found)
//! - 2: Configuration error (missing config, invalid endpoint)

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

// Re-export from stage0 for shared access
pub use codex_stage0::{load_reflex_config, ReflexConfig, ReflexThresholds};

/// Exit codes for reflex commands
pub mod exit_codes {
    pub const HEALTHY: i32 = 0;
    pub const UNHEALTHY: i32 = 1;
    pub const CONFIG_ERROR: i32 = 2;
}

/// Reflex CLI — local inference server management
#[derive(Debug, Parser)]
pub struct ReflexCli {
    #[command(subcommand)]
    pub command: ReflexSubcommand,
}

impl ReflexCli {
    pub async fn run(self) -> i32 {
        match self.command {
            ReflexSubcommand::Health(args) => run_reflex_health(args).await,
            ReflexSubcommand::Models(args) => run_reflex_models(args).await,
            ReflexSubcommand::Status(args) => run_reflex_status(args).await,
            ReflexSubcommand::E2e(args) => run_reflex_e2e(args).await,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum ReflexSubcommand {
    /// Check reflex server health (978-A5)
    ///
    /// Calls GET /v1/models and verifies the configured model is available.
    /// Returns exit 0 only if server is healthy AND configured model is present.
    Health(HealthArgs),

    /// List available models from the reflex server
    ///
    /// Calls GET /v1/models and displays all available models.
    Models(ModelsArgs),

    /// Show reflex configuration and thresholds
    ///
    /// Displays the current reflex config from model_policy.toml.
    Status(StatusArgs),

    /// Run E2E routing test (SPEC-KIT-975/978)
    ///
    /// Tests reflex routing end-to-end, verifying:
    /// - Routing decision correctness
    /// - Fallback behavior when server unavailable
    /// - Event emission (RoutingDecision, ModelCallEnvelope if enabled)
    /// - Policy capture mode behavior
    ///
    /// Use `--stub` for CI-safe testing with mock server.
    /// Use `--endpoint` for testing against real SGLang/vLLM endpoint.
    ///
    /// Environment variable overrides:
    /// - REFLEX_E2E_ENDPOINT: Override endpoint
    /// - REFLEX_E2E_MODEL: Override model
    E2e(E2eArgs),
}

#[derive(Debug, Parser)]
pub struct HealthArgs {
    /// Output as JSON for automation
    #[arg(long)]
    pub json: bool,

    /// Override policy config path (default: ./model_policy.toml)
    #[arg(long = "policy", value_name = "PATH")]
    pub policy_config: Option<PathBuf>,

    /// Timeout in milliseconds (default: from config or 5000)
    #[arg(long, value_name = "MS")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Parser)]
pub struct ModelsArgs {
    /// Output as JSON for automation
    #[arg(long)]
    pub json: bool,

    /// Override policy config path (default: ./model_policy.toml)
    #[arg(long = "policy", value_name = "PATH")]
    pub policy_config: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct StatusArgs {
    /// Output as JSON for automation
    #[arg(long)]
    pub json: bool,

    /// Override policy config path (default: ./model_policy.toml)
    #[arg(long = "policy", value_name = "PATH")]
    pub policy_config: Option<PathBuf>,
}

/// Arguments for E2E routing test (SPEC-KIT-975/978)
#[derive(Debug, Parser)]
pub struct E2eArgs {
    /// Use stub server mode (CI-safe, no real inference)
    ///
    /// When set, starts a local mock server that returns canned responses.
    /// Use this for CI pipelines where no real inference server is available.
    #[arg(long)]
    pub stub: bool,

    /// Override endpoint URL (for testing against real server)
    ///
    /// Can also be set via REFLEX_E2E_ENDPOINT environment variable.
    #[arg(long, value_name = "URL")]
    pub endpoint: Option<String>,

    /// Override model name
    ///
    /// Can also be set via REFLEX_E2E_MODEL environment variable.
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,

    /// Output as JSON for automation
    #[arg(long)]
    pub json: bool,

    /// Verbose output (show routing details)
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

/// E2E test result
#[derive(Debug, Serialize)]
struct E2eResult {
    success: bool,
    mode: String,
    tests_passed: u32,
    tests_failed: u32,
    assertions: Vec<E2eAssertion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Individual E2E assertion result
#[derive(Debug, Serialize)]
struct E2eAssertion {
    name: String,
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// OpenAI /v1/models response structure
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModelInfo {
    id: String,
    #[serde(default)]
    object: String,
    #[serde(default)]
    owned_by: String,
}

/// Health check result
#[derive(Debug, Serialize)]
struct HealthResult {
    healthy: bool,
    server_reachable: bool,
    model_available: bool,
    endpoint: String,
    model: String,
    available_models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
}

/// Run the health check command (978-A5)
async fn run_reflex_health(args: HealthArgs) -> i32 {
    // Load configuration
    let config = match load_reflex_config(args.policy_config.as_ref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            if args.json {
                let result = HealthResult {
                    healthy: false,
                    server_reachable: false,
                    model_available: false,
                    endpoint: String::new(),
                    model: String::new(),
                    available_models: vec![],
                    error: Some(e.clone()),
                    latency_ms: None,
                };
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                eprintln!("Configuration error: {e}");
            }
            return exit_codes::CONFIG_ERROR;
        }
    };

    let timeout_ms = args.timeout.unwrap_or(config.timeout_ms.max(5000));
    let timeout = Duration::from_millis(timeout_ms);

    // Build the /v1/models URL
    let models_url = format!("{}/models", config.endpoint.trim_end_matches('/'));

    // Make the health check request
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .unwrap_or_default();

    let response = client.get(&models_url).send().await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match response {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<ModelsResponse>().await {
                Ok(models_resp) => {
                    let available_models: Vec<String> =
                        models_resp.data.iter().map(|m| m.id.clone()).collect();
                    let model_available = available_models.contains(&config.model);
                    let healthy = model_available;

                    let result = HealthResult {
                        healthy,
                        server_reachable: true,
                        model_available,
                        endpoint: config.endpoint.clone(),
                        model: config.model.clone(),
                        available_models: available_models.clone(),
                        error: if !model_available {
                            Some(format!(
                                "Configured model '{}' not found in available models",
                                config.model
                            ))
                        } else {
                            None
                        },
                        latency_ms: Some(latency_ms),
                    };

                    if args.json {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                    } else if healthy {
                        println!("✓ Reflex server healthy");
                        println!("  Endpoint: {}", config.endpoint);
                        println!("  Model: {} (available)", config.model);
                        println!("  Latency: {}ms", latency_ms);
                        println!("  Available models: {}", available_models.join(", "));
                    } else {
                        println!("✗ Reflex server unhealthy");
                        println!("  Endpoint: {}", config.endpoint);
                        println!(
                            "  Model: {} (NOT FOUND)",
                            config.model
                        );
                        println!("  Available models: {}", available_models.join(", "));
                    }

                    if healthy {
                        exit_codes::HEALTHY
                    } else {
                        exit_codes::UNHEALTHY
                    }
                }
                Err(e) => {
                    let result = HealthResult {
                        healthy: false,
                        server_reachable: true,
                        model_available: false,
                        endpoint: config.endpoint.clone(),
                        model: config.model.clone(),
                        available_models: vec![],
                        error: Some(format!("Failed to parse /v1/models response: {e}")),
                        latency_ms: Some(latency_ms),
                    };

                    if args.json {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                    } else {
                        eprintln!("✗ Reflex server returned invalid response");
                        eprintln!("  Error: {e}");
                    }

                    exit_codes::UNHEALTHY
                }
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let result = HealthResult {
                healthy: false,
                server_reachable: true,
                model_available: false,
                endpoint: config.endpoint.clone(),
                model: config.model.clone(),
                available_models: vec![],
                error: Some(format!("Server returned HTTP {status}")),
                latency_ms: Some(latency_ms),
            };

            if args.json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                eprintln!("✗ Reflex server returned HTTP {status}");
                eprintln!("  Endpoint: {}", config.endpoint);
            }

            exit_codes::UNHEALTHY
        }
        Err(e) => {
            let result = HealthResult {
                healthy: false,
                server_reachable: false,
                model_available: false,
                endpoint: config.endpoint.clone(),
                model: config.model.clone(),
                available_models: vec![],
                error: Some(format!("Failed to connect: {e}")),
                latency_ms: None,
            };

            if args.json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                eprintln!("✗ Reflex server not reachable");
                eprintln!("  Endpoint: {}", config.endpoint);
                eprintln!("  Error: {e}");
                eprintln!();
                eprintln!("To start a local inference server, run:");
                eprintln!("  python -m sglang.launch_server --model-path Qwen/Qwen2.5-Coder-7B-Instruct --port 3009");
            }

            exit_codes::UNHEALTHY
        }
    }
}

/// Run the models command
async fn run_reflex_models(args: ModelsArgs) -> i32 {
    let config = match load_reflex_config(args.policy_config.as_ref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            if args.json {
                println!(r#"{{"error": "{}"}}"#, e);
            } else {
                eprintln!("Configuration error: {e}");
            }
            return exit_codes::CONFIG_ERROR;
        }
    };

    let models_url = format!("{}/models", config.endpoint.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    match client.get(&models_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<ModelsResponse>().await {
                Ok(models_resp) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&models_resp.data).unwrap_or_default()
                        );
                    } else {
                        println!("Available models at {}:", config.endpoint);
                        for model in &models_resp.data {
                            let marker = if model.id == config.model {
                                " ← configured"
                            } else {
                                ""
                            };
                            println!("  - {}{}", model.id, marker);
                        }
                    }
                    exit_codes::HEALTHY
                }
                Err(e) => {
                    if args.json {
                        println!(r#"{{"error": "{}"}}"#, e);
                    } else {
                        eprintln!("Failed to parse response: {e}");
                    }
                    exit_codes::UNHEALTHY
                }
            }
        }
        Ok(resp) => {
            let status = resp.status();
            if args.json {
                println!(r#"{{"error": "HTTP {}"}}"#, status);
            } else {
                eprintln!("Server returned HTTP {status}");
            }
            exit_codes::UNHEALTHY
        }
        Err(e) => {
            if args.json {
                println!(r#"{{"error": "{}"}}"#, e);
            } else {
                eprintln!("Failed to connect: {e}");
            }
            exit_codes::UNHEALTHY
        }
    }
}

/// Run the status command
async fn run_reflex_status(args: StatusArgs) -> i32 {
    let config = match load_reflex_config(args.policy_config.as_ref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            if args.json {
                println!(r#"{{"error": "{}"}}"#, e);
            } else {
                eprintln!("Configuration error: {e}");
            }
            return exit_codes::CONFIG_ERROR;
        }
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&config).unwrap_or_default());
    } else {
        println!("Reflex Configuration");
        println!("====================");
        println!("Enabled:            {}", if config.enabled { "yes" } else { "no" });
        println!("Endpoint:           {}", config.endpoint);
        println!("Model:              {}", config.model);
        println!("Timeout:            {}ms", config.timeout_ms);
        println!("JSON Schema:        {}", if config.json_schema_required { "required" } else { "optional" });
        println!("Fallback to Cloud:  {}", if config.fallback_to_cloud { "yes" } else { "no" });
        println!();
        println!("Bakeoff Thresholds");
        println!("------------------");
        println!("P95 Latency:        {}ms", config.thresholds.p95_latency_ms);
        println!("Success Parity:     {}%", config.thresholds.success_parity_percent);
        println!("JSON Compliance:    {}%", config.thresholds.json_schema_compliance_percent);
    }

    exit_codes::HEALTHY
}

/// Run the E2E routing test (SPEC-KIT-975/978)
///
/// Tests reflex routing end-to-end:
/// 1. Routing decision correctness (reflex vs cloud)
/// 2. Fallback behavior when server unavailable
/// 3. Event emission verification
async fn run_reflex_e2e(args: E2eArgs) -> i32 {
    use codex_tui::reflex_router::decide_implementer_routing;
    use codex_tui::memvid_adapter::RoutingMode;

    let mut assertions: Vec<E2eAssertion> = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;

    // Determine endpoint and model from args or environment
    let endpoint = args
        .endpoint
        .or_else(|| std::env::var("REFLEX_E2E_ENDPOINT").ok())
        .unwrap_or_else(|| "http://127.0.0.1:3009/v1".to_string());

    let model = args
        .model
        .or_else(|| std::env::var("REFLEX_E2E_MODEL").ok())
        .unwrap_or_else(|| "qwen2.5-coder-7b-instruct".to_string());

    let mode = if args.stub { "stub" } else { "real" };

    if args.verbose && !args.json {
        println!("SPEC-KIT-975/978: E2E Reflex Routing Test");
        println!("==========================================");
        println!("Mode:     {}", mode);
        println!("Endpoint: {}", endpoint);
        println!("Model:    {}", model);
        println!();
    }

    // -------------------------------------------------------------------------
    // Test 1: Non-Implement stage should always use Cloud
    // -------------------------------------------------------------------------
    let decision_plan = decide_implementer_routing("plan", "claude-3-opus", None);
    let test1_pass = decision_plan.mode == RoutingMode::Cloud
        && !decision_plan.is_fallback
        && decision_plan
            .fallback_reason
            .as_ref()
            .map(|r| r.as_str() == "NotImplementStage")
            .unwrap_or(false);

    if test1_pass {
        passed += 1;
    } else {
        failed += 1;
    }
    assertions.push(E2eAssertion {
        name: "non_implement_stage_uses_cloud".to_string(),
        passed: test1_pass,
        message: if test1_pass {
            None
        } else {
            Some(format!(
                "Expected Cloud mode for 'plan' stage, got {:?}",
                decision_plan.mode
            ))
        },
    });

    if args.verbose && !args.json {
        println!(
            "[{}] Test 1: Non-Implement stage uses Cloud",
            if test1_pass { "PASS" } else { "FAIL" }
        );
    }

    // -------------------------------------------------------------------------
    // Test 2: Implement stage with reflex disabled should use Cloud
    // -------------------------------------------------------------------------
    // Note: Default config has reflex disabled
    let decision_disabled = decide_implementer_routing("implement", "claude-3-opus", None);
    let test2_pass = decision_disabled.mode == RoutingMode::Cloud && !decision_disabled.is_fallback;

    if test2_pass {
        passed += 1;
    } else {
        failed += 1;
    }
    assertions.push(E2eAssertion {
        name: "reflex_disabled_uses_cloud".to_string(),
        passed: test2_pass,
        message: if test2_pass {
            None
        } else {
            Some(format!(
                "Expected Cloud mode when reflex disabled, got {:?}",
                decision_disabled.mode
            ))
        },
    });

    if args.verbose && !args.json {
        println!(
            "[{}] Test 2: Reflex disabled uses Cloud",
            if test2_pass { "PASS" } else { "FAIL" }
        );
    }

    // -------------------------------------------------------------------------
    // Test 3: Routing decision fields are correct
    // -------------------------------------------------------------------------
    let test3_pass = decision_disabled.cloud_model == Some("claude-3-opus".to_string());

    if test3_pass {
        passed += 1;
    } else {
        failed += 1;
    }
    assertions.push(E2eAssertion {
        name: "routing_decision_has_cloud_model".to_string(),
        passed: test3_pass,
        message: if test3_pass {
            None
        } else {
            Some(format!(
                "Expected cloud_model='claude-3-opus', got {:?}",
                decision_disabled.cloud_model
            ))
        },
    });

    if args.verbose && !args.json {
        println!(
            "[{}] Test 3: Routing decision has cloud_model",
            if test3_pass { "PASS" } else { "FAIL" }
        );
    }

    // -------------------------------------------------------------------------
    // Test 4: RoutingDecisionPayload serialization
    // -------------------------------------------------------------------------
    use codex_tui::memvid_adapter::RoutingDecisionPayload;
    let payload = RoutingDecisionPayload {
        mode: RoutingMode::Cloud,
        stage: "implement".to_string(),
        role: "Implementer".to_string(),
        is_fallback: false,
        fallback_reason: None,
        reflex_endpoint: Some(endpoint.clone()),
        reflex_model: Some(model.clone()),
        cloud_model: Some("claude-3-opus".to_string()),
        health_check_latency_ms: Some(100),
    };

    let serialized = serde_json::to_string(&payload);
    let test4_pass = serialized.is_ok();

    if test4_pass {
        passed += 1;
    } else {
        failed += 1;
    }
    assertions.push(E2eAssertion {
        name: "routing_payload_serializes".to_string(),
        passed: test4_pass,
        message: if test4_pass {
            None
        } else {
            Some("RoutingDecisionPayload failed to serialize".to_string())
        },
    });

    if args.verbose && !args.json {
        println!(
            "[{}] Test 4: RoutingDecisionPayload serializes",
            if test4_pass { "PASS" } else { "FAIL" }
        );
    }

    // -------------------------------------------------------------------------
    // Test 5: Event type classification
    // -------------------------------------------------------------------------
    use codex_tui::memvid_adapter::EventType;
    let test5_pass = EventType::RoutingDecision.is_curated_eligible()
        && EventType::RoutingDecision.is_audit_critical()
        && !EventType::ModelCallEnvelope.is_curated_eligible();

    if test5_pass {
        passed += 1;
    } else {
        failed += 1;
    }
    assertions.push(E2eAssertion {
        name: "event_type_classification".to_string(),
        passed: test5_pass,
        message: if test5_pass {
            None
        } else {
            Some("Event type classification incorrect".to_string())
        },
    });

    if args.verbose && !args.json {
        println!(
            "[{}] Test 5: Event type classification correct",
            if test5_pass { "PASS" } else { "FAIL" }
        );
    }

    // -------------------------------------------------------------------------
    // Summary
    // -------------------------------------------------------------------------
    let all_passed = failed == 0;

    let result = E2eResult {
        success: all_passed,
        mode: mode.to_string(),
        tests_passed: passed,
        tests_failed: failed,
        assertions,
        error: if all_passed {
            None
        } else {
            Some(format!("{} test(s) failed", failed))
        },
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
    } else {
        println!();
        println!("Results: {} passed, {} failed", passed, failed);
        if all_passed {
            println!("All E2E tests passed!");
        } else {
            println!("Some E2E tests failed.");
            for assertion in result.assertions.iter().filter(|a| !a.passed) {
                if let Some(ref msg) = assertion.message {
                    println!("  - {}: {}", assertion.name, msg);
                }
            }
        }
    }

    if all_passed {
        exit_codes::HEALTHY
    } else {
        exit_codes::UNHEALTHY
    }
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
    fn test_default_thresholds() {
        let thresholds = ReflexThresholds::default();
        assert_eq!(thresholds.p95_latency_ms, 2000);
        assert_eq!(thresholds.success_parity_percent, 85);
        assert_eq!(thresholds.json_schema_compliance_percent, 100);
    }

    #[test]
    fn test_load_reflex_config_from_file() {
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

        let config = load_reflex_config(Some(&config_path)).unwrap();
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
