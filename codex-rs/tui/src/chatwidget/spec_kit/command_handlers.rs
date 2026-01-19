//! Spec-Kit command entry points
//!
//! This module contains the top-level command handlers that serve as entry points
//! for spec-kit slash commands. These handlers parse arguments, validate input,
//! and delegate to specialized modules for actual implementation.
//!
//! **Command Handlers:**
//! - `/speckit.status` → handle_spec_status (native dashboard via executor)
//! - `/spec-review` → handle_spec_review (stage gate evaluation via executor)
//! - `/guardrail.*` → handle_guardrail (guardrail validation)
//! - Pipeline errors → halt_spec_auto_with_error (error handling)
//!
//! **SPEC-KIT-921**: Status and Review commands now use shared SpeckitExecutor for CLI parity.

use super::super::ChatWidget;
use super::context::SpecKitContext;
use super::state::ValidateCompletionReason;
use crate::app_event::BackgroundPlacement;
use crate::history_cell::HistoryCellType;

// SPEC-KIT-921: Use shared executor for status and review commands (CLI/TUI parity)
use codex_spec_kit::config::policy_toggles::PolicyToggles;
use codex_spec_kit::executor::{
    ExecutionContext, Outcome, PolicySnapshot, SpeckitCommand, SpeckitExecutor, TelemetryMode,
    render_review_dashboard, render_status_dashboard, review_warning, status_degraded_warning,
};

/// Handle /speckit.status command (native dashboard)
///
/// **SPEC-KIT-921**: Uses shared SpeckitExecutor for CLI/TUI parity.
///
/// Displays spec-kit status dashboard with:
/// - Active specs and their stages
/// - Evidence health (conflicts, oversized, stale, missing docs)
/// - HAL validation status
/// - Degradation warnings
pub fn handle_spec_status(widget: &mut ChatWidget, raw_args: String) {
    // Parse command using shared parser
    let command = match SpeckitCommand::parse_status(&raw_args) {
        Ok(cmd) => cmd,
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(err));
            widget.request_redraw();
            return;
        }
    };

    // Resolve policy from env/config at adapter boundary (not in executor)
    let toggles = PolicyToggles::from_env_and_config();
    let policy_snapshot = PolicySnapshot {
        sidecar_critic_enabled: toggles.sidecar_critic_enabled,
        telemetry_mode: TelemetryMode::Disabled,
        legacy_voting_env_detected: toggles.legacy_voting_enabled,
    };

    // Create executor with current working directory and resolved policy
    let executor = SpeckitExecutor::new(ExecutionContext {
        repo_root: widget.config.cwd.clone(),
        policy_snapshot: Some(policy_snapshot),
    });

    // Execute via shared executor (same path as CLI)
    match executor.execute(command) {
        Outcome::Status(report) => {
            let mut lines = render_status_dashboard(&report);
            if let Some(warning) = status_degraded_warning(&report) {
                lines.insert(1, warning);
            }
            let message = lines.join("\n");
            widget.insert_background_event_with_placement(message, BackgroundPlacement::Tail);
            widget.request_redraw();
        }
        Outcome::Error(err) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "spec-status failed: {err}"
            )));
            widget.request_redraw();
        }
        // Status command never returns Review, Stage, Specify, or Run variants
        Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Status command should never return Review outcome")
        }
        Outcome::Stage(_) => {
            unreachable!("Status command should never return Stage outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Status command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Status command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Status command should never return Migrate outcome")
        }
    }
}

/// Handle /spec-review command (stage gate evaluation)
///
/// **SPEC-KIT-921**: Uses shared SpeckitExecutor for CLI/TUI parity.
///
/// Evaluates stage gate artifacts and displays:
/// - Stage review result (Passed/PassedWithWarnings/Failed/Skipped)
/// - Blocking signals (conflicts from consensus)
/// - Advisory signals (errors, warnings)
/// - Evidence refs (repo-relative paths)
///
/// Usage: /spec-review <SPEC-ID> <stage> [--strict-artifacts] [--strict-warnings]
/// Stages: plan, tasks, implement, validate, audit, unlock
pub fn handle_spec_review(widget: &mut ChatWidget, raw_args: String) {
    let trimmed = raw_args.trim();
    if trimmed.is_empty() {
        widget.history_push(crate::history_cell::new_error_event(
            "Usage: /spec-review <SPEC-ID> <stage> [--strict-artifacts] [--strict-warnings]"
                .to_string(),
        ));
        widget.request_redraw();
        return;
    }

    // Parse SPEC-ID from first argument
    let mut parts = trimmed.split_whitespace();
    let Some(spec_id) = parts.next() else {
        widget.history_push(crate::history_cell::new_error_event(
            "Usage: /spec-review <SPEC-ID> <stage>".to_string(),
        ));
        widget.request_redraw();
        return;
    };

    // Remaining args are stage + flags
    let remaining: String = parts.collect::<Vec<_>>().join(" ");
    if remaining.is_empty() {
        widget.history_push(crate::history_cell::new_error_event(
            "Stage required. Valid stages: plan, tasks, implement, validate, audit, unlock"
                .to_string(),
        ));
        widget.request_redraw();
        return;
    }

    // Parse using shared parser (CLI/TUI parity)
    let command = match SpeckitCommand::parse_review(spec_id, &remaining) {
        Ok(cmd) => cmd,
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(err));
            widget.request_redraw();
            return;
        }
    };

    // Resolve policy from env/config at adapter boundary (not in executor)
    let toggles = PolicyToggles::from_env_and_config();
    let policy_snapshot = PolicySnapshot {
        sidecar_critic_enabled: toggles.sidecar_critic_enabled,
        telemetry_mode: TelemetryMode::Disabled,
        legacy_voting_env_detected: toggles.legacy_voting_enabled,
    };

    // Create executor with current working directory and resolved policy
    let executor = SpeckitExecutor::new(ExecutionContext {
        repo_root: widget.config.cwd.clone(),
        policy_snapshot: Some(policy_snapshot),
    });

    // Execute via shared executor (same path as CLI)
    match executor.execute(command) {
        Outcome::Review(result) => {
            let mut lines = render_review_dashboard(&result);
            if let Some(warning) = review_warning(&result) {
                lines.insert(1, warning);
            }
            let message = lines.join("\n");
            widget.insert_background_event_with_placement(message, BackgroundPlacement::Tail);
            widget.request_redraw();
        }
        Outcome::ReviewSkipped {
            stage,
            reason,
            suggestion,
        } => {
            let mut msg = format!("⚠ Review skipped for {:?}: {:?}", stage, reason);
            if let Some(hint) = suggestion {
                msg.push_str(&format!("\n  Suggestion: {hint}"));
            }
            widget.history_push(crate::history_cell::new_warning_event(msg));
            widget.request_redraw();
        }
        Outcome::Error(err) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "spec-review failed: {err}"
            )));
            widget.request_redraw();
        }
        // Review command never returns Status, Stage, Specify, or Run variant
        Outcome::Status(_) => {
            unreachable!("Review command should never return Status outcome")
        }
        Outcome::Stage(_) => {
            unreachable!("Review command should never return Stage outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Review command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Review command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Review command should never return Migrate outcome")
        }
    }
}

/// Halt /speckit.auto pipeline with error message
///
/// FORK-SPECIFIC (just-every/code): FR3 cancellation cleanup for SPEC-KIT-069
///
/// Displays error message with resume hint and cleans up:
/// 1. Active validate lifecycle state (if present)
/// 2. spec_auto_state
/// 3. Shows resume command hint
///
/// Note: This uses SpecKitContext trait for testability. Full cleanup with
/// telemetry emission requires calling cleanup_spec_auto_with_cancel directly
/// with ChatWidget (which has MCP manager access).
pub fn halt_spec_auto_with_error(widget: &mut impl SpecKitContext, reason: String) {
    // Clean up active validate lifecycle state if present
    if let Some(state) = widget.spec_auto_state().as_ref()
        && state.validate_lifecycle.active().is_some()
    {
        // Clean up the validate lifecycle state (mark as cancelled)
        let _ = state
            .validate_lifecycle
            .reset_active(ValidateCompletionReason::Cancelled);
        // Note: Telemetry emission is handled separately by cleanup_spec_auto_with_cancel
        // when called directly with ChatWidget. When called through trait, telemetry
        // is skipped since trait doesn't expose MCP manager access.
    }

    let resume_hint = widget
        .spec_auto_state()
        .as_ref()
        .and_then(|state| {
            state.current_stage().map(|stage| {
                format!(
                    "/speckit.auto {} --from {}",
                    state.spec_id,
                    stage.command_name()
                )
            })
        })
        .unwrap_or_default();

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![
            ratatui::text::Line::from("⚠ /speckit.auto halted"),
            ratatui::text::Line::from(reason),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("Resume with:"),
            ratatui::text::Line::from(resume_hint),
        ],
        HistoryCellType::Error,
    ));

    // SPEC-KIT-920: Signal automation failure for exit code
    widget.send_app_event(crate::app_event::AppEvent::AutomationFailure);

    *widget.spec_auto_state_mut() = None;
    // P6-SYNC Phase 6: Clear spec-kit token metrics from status bar
    widget.set_spec_auto_metrics(None);
}

/// Handle /spec-consensus command (DEPRECATED)
///
/// **DEPRECATED**: Use `/spec-review` instead.
/// This is now a thin wrapper around `handle_spec_review` for backward compatibility.
/// The old MCP-based consensus check has been replaced with the executor-based review.
pub fn handle_spec_consensus(widget: &mut ChatWidget, raw_args: String) {
    // Delegate to the new executor-based review handler
    handle_spec_review(widget, raw_args);
}

/// Handle /guardrail.* commands (guardrail validation)
///
/// Delegates to guardrail module for actual implementation.
/// This handler just provides the entry point routing.
pub fn handle_guardrail(
    widget: &mut ChatWidget,
    command: crate::slash_command::SlashCommand,
    raw_args: String,
    hal_override: Option<crate::slash_command::HalMode>,
) {
    // Delegate to guardrail module implementation
    super::guardrail::handle_guardrail_impl(widget, command, raw_args, hal_override);
}

/// Handle /speckit.reflex commands (SPEC-KIT-978)
///
/// Subcommands:
/// - health: Check reflex server health
/// - status: Show reflex configuration
/// - models: List available models
/// - bakeoff: Show reflex vs cloud comparison metrics
/// - e2e [--stub]: Run E2E routing tests
/// - check [duration]: Validate thresholds (default 24h)
pub fn handle_speckit_reflex(widget: &mut ChatWidget, raw_args: String) {
    use ratatui::text::Line;

    let args: Vec<&str> = raw_args.split_whitespace().collect();
    let subcommand = args.first().map(|s| s.to_lowercase());

    match subcommand.as_deref() {
        Some("health") => handle_reflex_health(widget),
        Some("status") => handle_reflex_status(widget),
        Some("models") => handle_reflex_models(widget),
        Some("bakeoff") => {
            // Parse optional duration argument (default 24h)
            let since = args.get(1).map(|s| s.to_string()).unwrap_or_else(|| "24h".to_string());
            handle_reflex_bakeoff(widget, &since);
        }
        Some("e2e") => {
            // SPEC-KIT-978: E2E routing tests
            let stub = args.iter().any(|a| *a == "--stub");
            handle_reflex_e2e(widget, stub);
        }
        Some("check") => {
            // SPEC-KIT-978: Threshold validation
            let since = args.get(1).map(|s| s.to_string()).unwrap_or_else(|| "24h".to_string());
            handle_reflex_check(widget, &since);
        }
        Some(cmd) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from(format!("Unknown subcommand: {cmd}")),
                    Line::from(""),
                    Line::from("Available subcommands:"),
                    Line::from("  health  - Check reflex server health"),
                    Line::from("  status  - Show reflex configuration"),
                    Line::from("  models  - List available models"),
                    Line::from("  bakeoff - Show reflex vs cloud metrics"),
                    Line::from("  e2e     - Run E2E routing tests"),
                    Line::from("  check   - Validate thresholds"),
                ],
                HistoryCellType::Error,
            ));
        }
        None => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from("Usage: /speckit.reflex <subcommand>"),
                    Line::from(""),
                    Line::from("Subcommands:"),
                    Line::from("  health  - Check reflex server health"),
                    Line::from("  status  - Show reflex configuration"),
                    Line::from("  models  - List available models"),
                    Line::from("  bakeoff - Show reflex vs cloud metrics (24h default)"),
                    Line::from("  e2e     - Run E2E routing tests (--stub for offline)"),
                    Line::from("  check   - Validate bakeoff thresholds (default 24h)"),
                ],
                HistoryCellType::Notice,
            ));
        }
    }

    widget.request_redraw();
}

/// Handle /speckit.reflex health
fn handle_reflex_health(widget: &mut ChatWidget) {
    use codex_stage0::load_reflex_config;
    use ratatui::text::Line;

    // Load configuration
    let config = match load_reflex_config(None) {
        Ok(cfg) => cfg,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from("✗ Configuration error"),
                    Line::from(e),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Show loading message
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![Line::from(format!(
            "Checking reflex server at {}...",
            config.endpoint
        ))],
        HistoryCellType::Notice,
    ));
    widget.request_redraw();

    // Perform synchronous health check using blocking client
    let models_url = format!("{}/models", config.endpoint.trim_end_matches('/'));
    let timeout = std::time::Duration::from_millis(config.timeout_ms.max(5000));

    let start = std::time::Instant::now();
    let client = match reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from("✗ Failed to create HTTP client"),
                    Line::from(format!("  Error: {e}")),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    let result = client.get(&models_url).send();
    let latency_ms = start.elapsed().as_millis() as u64;

    let lines = match result {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<ModelsResponse>() {
                Ok(models_resp) => {
                    let available: Vec<String> =
                        models_resp.data.iter().map(|m| m.id.clone()).collect();
                    let model_available = available.contains(&config.model);

                    if model_available {
                        vec![
                            Line::from("✓ Reflex server healthy"),
                            Line::from(format!("  Endpoint: {}", config.endpoint)),
                            Line::from(format!("  Model: {} (available)", config.model)),
                            Line::from(format!("  Latency: {}ms", latency_ms)),
                            Line::from(format!("  Available models: {}", available.join(", "))),
                        ]
                    } else {
                        vec![
                            Line::from("✗ Reflex server unhealthy"),
                            Line::from(format!("  Endpoint: {}", config.endpoint)),
                            Line::from(format!("  Model: {} (NOT FOUND)", config.model)),
                            Line::from(format!("  Available models: {}", available.join(", "))),
                        ]
                    }
                }
                Err(e) => vec![
                    Line::from("✗ Invalid response from server"),
                    Line::from(format!("  Error: {e}")),
                ],
            }
        }
        Ok(resp) => {
            let status = resp.status();
            vec![
                Line::from("✗ Reflex server returned error"),
                Line::from(format!("  HTTP {status}")),
            ]
        }
        Err(e) => vec![
            Line::from("✗ Reflex server not reachable"),
            Line::from(format!("  Endpoint: {}", config.endpoint)),
            Line::from(format!("  Error: {e}")),
            Line::from(""),
            Line::from("To start a local inference server:"),
            Line::from("  python -m sglang.launch_server --model-path Qwen/Qwen2.5-Coder-7B-Instruct --port 3009"),
        ],
    };

    let cell_type = if lines.first().map(|l| l.to_string().contains('✓')).unwrap_or(false) {
        HistoryCellType::Notice
    } else {
        HistoryCellType::Error
    };

    widget.history_push(crate::history_cell::PlainHistoryCell::new(lines, cell_type));
}

/// OpenAI /v1/models response
#[derive(Debug, serde::Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Debug, serde::Deserialize)]
struct ModelInfo {
    id: String,
}

/// Handle /speckit.reflex status
fn handle_reflex_status(widget: &mut ChatWidget) {
    use codex_stage0::load_reflex_config;
    use ratatui::text::Line;

    let config = match load_reflex_config(None) {
        Ok(cfg) => cfg,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![Line::from(format!("Configuration error: {e}"))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![
            Line::from("Reflex Configuration"),
            Line::from("===================="),
            Line::from(format!(
                "Enabled:            {}",
                if config.enabled { "yes" } else { "no" }
            )),
            Line::from(format!("Endpoint:           {}", config.endpoint)),
            Line::from(format!("Model:              {}", config.model)),
            Line::from(format!("Timeout:            {}ms", config.timeout_ms)),
            Line::from(format!(
                "JSON Schema:        {}",
                if config.json_schema_required {
                    "required"
                } else {
                    "optional"
                }
            )),
            Line::from(format!(
                "Fallback to Cloud:  {}",
                if config.fallback_to_cloud {
                    "yes"
                } else {
                    "no"
                }
            )),
            Line::from(""),
            Line::from("Bakeoff Thresholds"),
            Line::from("------------------"),
            Line::from(format!(
                "P95 Latency:        {}ms",
                config.thresholds.p95_latency_ms
            )),
            Line::from(format!(
                "Success Parity:     {}%",
                config.thresholds.success_parity_percent
            )),
            Line::from(format!(
                "JSON Compliance:    {}%",
                config.thresholds.json_schema_compliance_percent
            )),
        ],
        HistoryCellType::Notice,
    ));
}

/// Handle /speckit.reflex models
fn handle_reflex_models(widget: &mut ChatWidget) {
    use codex_stage0::load_reflex_config;
    use ratatui::text::Line;

    let config = match load_reflex_config(None) {
        Ok(cfg) => cfg,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![Line::from(format!("Configuration error: {e}"))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Perform synchronous request
    let models_url = format!("{}/models", config.endpoint.trim_end_matches('/'));
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![Line::from(format!("Failed to create HTTP client: {e}"))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    let (lines, cell_type) = match client.get(&models_url).send() {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<ModelsResponse>() {
                Ok(models_resp) => {
                    let mut output = vec![Line::from(format!(
                        "Available models at {}:",
                        config.endpoint
                    ))];
                    for model in &models_resp.data {
                        let marker = if model.id == config.model {
                            " ← configured"
                        } else {
                            ""
                        };
                        output.push(Line::from(format!("  - {}{}", model.id, marker)));
                    }
                    (output, HistoryCellType::Notice)
                }
                Err(e) => (
                    vec![
                        Line::from("Failed to parse models response"),
                        Line::from(format!("  Error: {e}")),
                    ],
                    HistoryCellType::Error,
                ),
            }
        }
        Ok(resp) => (
            vec![
                Line::from("Failed to fetch models"),
                Line::from(format!("  HTTP {}", resp.status())),
            ],
            HistoryCellType::Error,
        ),
        Err(e) => (
            vec![
                Line::from("Failed to connect to server"),
                Line::from(format!("  Error: {e}")),
            ],
            HistoryCellType::Error,
        ),
    };

    widget.history_push(crate::history_cell::PlainHistoryCell::new(lines, cell_type));
}

/// Handle /speckit.reflex bakeoff (SPEC-KIT-978)
fn handle_reflex_bakeoff(widget: &mut ChatWidget, since: &str) {
    use super::reflex_metrics::ReflexMetricsDb;
    use ratatui::text::Line;

    // Parse duration
    let duration = match parse_bakeoff_duration(since) {
        Ok(d) => d,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from(format!("Invalid duration: {since}")),
                    Line::from(e),
                    Line::from("Use format like: 1h, 24h, 7d"),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Open metrics database
    let db = match ReflexMetricsDb::init_default() {
        Ok(db) => db,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from("Failed to open metrics database"),
                    Line::from(format!("  Error: {e}")),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Compute stats
    let stats = match db.compute_bakeoff_stats(duration) {
        Ok(s) => s,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from("Failed to compute bakeoff stats"),
                    Line::from(format!("  Error: {e}")),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    let mut lines = vec![
        Line::from(format!("Reflex Bakeoff Statistics (since {})", since)),
        Line::from(format!("Period: {} - {}", stats.period_start, stats.period_end)),
        Line::from(format!("Total Attempts: {}", stats.total_attempts)),
        Line::from(""),
    ];

    // Reflex stats
    if let Some(ref r) = stats.reflex {
        lines.extend([
            Line::from("REFLEX (local inference):"),
            Line::from(format!(
                "  Attempts:        {}",
                r.total_attempts
            )),
            Line::from(format!(
                "  Success Rate:    {:.1}% ({}/{})",
                r.success_rate, r.success_count, r.total_attempts
            )),
            Line::from(format!("  JSON Compliance: {:.1}%", r.json_compliance_rate)),
            Line::from(format!(
                "  P50:  {}ms | P95: {}ms | P99: {}ms",
                r.p50_latency_ms, r.p95_latency_ms, r.p99_latency_ms
            )),
            Line::from(""),
        ]);
    } else {
        lines.push(Line::from("REFLEX: No data"));
        lines.push(Line::from(""));
    }

    // Cloud stats
    if let Some(ref c) = stats.cloud {
        lines.extend([
            Line::from("CLOUD (remote inference):"),
            Line::from(format!(
                "  Attempts:        {}",
                c.total_attempts
            )),
            Line::from(format!(
                "  Success Rate:    {:.1}% ({}/{})",
                c.success_rate, c.success_count, c.total_attempts
            )),
            Line::from(format!("  JSON Compliance: {:.1}%", c.json_compliance_rate)),
            Line::from(format!(
                "  P50:  {}ms | P95: {}ms | P99: {}ms",
                c.p50_latency_ms, c.p95_latency_ms, c.p99_latency_ms
            )),
        ]);
    } else {
        lines.push(Line::from("CLOUD: No data"));
    }

    // Comparison
    if stats.reflex.is_some() && stats.cloud.is_some() {
        let reflex = stats.reflex.as_ref().unwrap();
        let cloud = stats.cloud.as_ref().unwrap();
        let latency_ratio = if reflex.p95_latency_ms > 0 {
            cloud.p95_latency_ms as f64 / reflex.p95_latency_ms as f64
        } else {
            0.0
        };
        lines.push(Line::from(""));
        lines.push(Line::from(format!("Reflex is {:.1}x faster (P95)", latency_ratio)));
    }

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        lines,
        HistoryCellType::Notice,
    ));
}

/// Parse duration string for bakeoff (e.g., "1h", "24h", "7d")
fn parse_bakeoff_duration(s: &str) -> Result<std::time::Duration, String> {
    let s = s.trim().to_lowercase();

    if s.ends_with('h') {
        let hours: u64 = s[..s.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid hours: {s}"))?;
        Ok(std::time::Duration::from_secs(hours * 3600))
    } else if s.ends_with('d') {
        let days: u64 = s[..s.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid days: {s}"))?;
        Ok(std::time::Duration::from_secs(days * 86400))
    } else if s.ends_with('m') {
        let mins: u64 = s[..s.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid minutes: {s}"))?;
        Ok(std::time::Duration::from_secs(mins * 60))
    } else {
        // Try parsing as seconds
        let secs: u64 = s.parse().map_err(|_| format!("Invalid duration format: {s}"))?;
        Ok(std::time::Duration::from_secs(secs))
    }
}

// =============================================================================
// SPEC-KIT-978: E2E and Check Commands
// =============================================================================

/// Handle /speckit.reflex e2e (SPEC-KIT-978)
///
/// Runs end-to-end routing tests to verify reflex routing logic.
/// Use --stub for offline testing without network.
fn handle_reflex_e2e(widget: &mut ChatWidget, stub: bool) {
    use super::reflex_router::decide_implementer_routing;
    use crate::memvid_adapter::{EventType, RoutingDecisionPayload, RoutingMode};
    use ratatui::text::Line;

    let mode = if stub { "stub" } else { "real" };
    let mut lines = vec![
        Line::from("SPEC-KIT-978: E2E Reflex Routing Test"),
        Line::from("======================================"),
        Line::from(format!("Mode: {}", mode)),
        Line::from(""),
    ];

    let mut passed = 0u32;
    let mut failed = 0u32;

    // Test 1: Non-Implement stage should always use Cloud
    let decision_plan = decide_implementer_routing("plan", "claude-3-opus", None);
    let test1_pass = decision_plan.mode == RoutingMode::Cloud
        && decision_plan.fallback_reason.as_ref()
            .map(|r| r.as_str() == "not_implement_stage")
            .unwrap_or(false);
    if test1_pass { passed += 1; } else { failed += 1; }
    lines.push(Line::from(format!(
        "[{}] Test 1: Non-Implement stage uses Cloud",
        if test1_pass { "PASS" } else { "FAIL" }
    )));

    // Test 2: Implement stage with reflex disabled should use Cloud
    let decision_disabled = decide_implementer_routing("implement", "claude-3-opus", None);
    let test2_pass = decision_disabled.mode == RoutingMode::Cloud && !decision_disabled.is_fallback;
    if test2_pass { passed += 1; } else { failed += 1; }
    lines.push(Line::from(format!(
        "[{}] Test 2: Reflex disabled uses Cloud",
        if test2_pass { "PASS" } else { "FAIL" }
    )));

    // Test 3: Routing decision has cloud_model field
    let test3_pass = decision_disabled.cloud_model == Some("claude-3-opus".to_string());
    if test3_pass { passed += 1; } else { failed += 1; }
    lines.push(Line::from(format!(
        "[{}] Test 3: Routing decision has cloud_model",
        if test3_pass { "PASS" } else { "FAIL" }
    )));

    // Test 4: RoutingDecisionPayload serializes correctly
    let payload = RoutingDecisionPayload {
        mode: RoutingMode::Cloud,
        stage: "implement".to_string(),
        role: "Implementer".to_string(),
        is_fallback: false,
        fallback_reason: None,
        reflex_endpoint: Some("http://127.0.0.1:3009/v1".to_string()),
        reflex_model: Some("qwen2.5-coder-7b-instruct".to_string()),
        cloud_model: Some("claude-3-opus".to_string()),
        health_check_latency_ms: Some(100),
    };
    let serialized = serde_json::to_string(&payload);
    let test4_pass = serialized.is_ok();
    if test4_pass { passed += 1; } else { failed += 1; }
    lines.push(Line::from(format!(
        "[{}] Test 4: RoutingDecisionPayload serializes",
        if test4_pass { "PASS" } else { "FAIL" }
    )));

    // Test 5: EventType classification is correct
    let test5_pass = EventType::RoutingDecision.is_curated_eligible()
        && EventType::RoutingDecision.is_audit_critical()
        && !EventType::ModelCallEnvelope.is_curated_eligible();
    if test5_pass { passed += 1; } else { failed += 1; }
    lines.push(Line::from(format!(
        "[{}] Test 5: EventType classification correct",
        if test5_pass { "PASS" } else { "FAIL" }
    )));

    // Summary
    lines.push(Line::from(""));
    lines.push(Line::from(format!("Results: {} passed, {} failed", passed, failed)));

    let cell_type = if failed == 0 {
        lines.push(Line::from("All E2E tests passed!"));
        HistoryCellType::Notice
    } else {
        lines.push(Line::from("Some E2E tests failed."));
        HistoryCellType::Error
    };

    widget.history_push(crate::history_cell::PlainHistoryCell::new(lines, cell_type));
}

/// Handle /speckit.reflex check (SPEC-KIT-978)
///
/// Validates that bakeoff metrics meet configured thresholds.
fn handle_reflex_check(widget: &mut ChatWidget, since: &str) {
    use codex_stage0::load_reflex_config;
    use super::reflex_metrics::ReflexMetricsDb;
    use ratatui::text::Line;

    // Parse duration
    let duration = match parse_bakeoff_duration(since) {
        Ok(d) => d,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from(format!("Invalid duration: {since}")),
                    Line::from(e),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Load configuration
    let config = match load_reflex_config(None) {
        Ok(cfg) => cfg,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from("✗ Configuration error"),
                    Line::from(e),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Open metrics database
    let db = match ReflexMetricsDb::init_default() {
        Ok(db) => db,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from("Failed to open metrics database"),
                    Line::from(format!("  Error: {e}")),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Compute stats
    let stats = match db.compute_bakeoff_stats(duration) {
        Ok(s) => s,
        Err(e) => {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![
                    Line::from("Failed to compute bakeoff stats"),
                    Line::from(format!("  Error: {e}")),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Check thresholds
    let mut checks: Vec<(String, bool, String)> = Vec::new();

    // Minimum samples check
    let min_samples = 10;
    let sample_count = stats.reflex.as_ref().map(|r| r.total_attempts).unwrap_or(0);
    let samples_ok = sample_count >= min_samples;
    checks.push((
        "Minimum Samples".to_string(),
        samples_ok,
        format!("{}/{} required", sample_count, min_samples),
    ));

    if let Some(ref reflex) = stats.reflex {
        // P95 latency check
        let p95_ok = reflex.p95_latency_ms <= config.thresholds.p95_latency_ms;
        checks.push((
            "P95 Latency".to_string(),
            p95_ok,
            format!("{}ms (max {}ms)", reflex.p95_latency_ms, config.thresholds.p95_latency_ms),
        ));

        // Success parity check
        let success_ok = reflex.success_rate >= config.thresholds.success_parity_percent as f64;
        checks.push((
            "Success Rate".to_string(),
            success_ok,
            format!("{:.1}% (min {}%)", reflex.success_rate, config.thresholds.success_parity_percent),
        ));

        // JSON compliance check
        let json_ok = reflex.json_compliance_rate >= config.thresholds.json_schema_compliance_percent as f64;
        checks.push((
            "JSON Compliance".to_string(),
            json_ok,
            format!("{:.1}% (min {}%)", reflex.json_compliance_rate, config.thresholds.json_schema_compliance_percent),
        ));
    } else {
        checks.push((
            "Reflex Data".to_string(),
            false,
            "No reflex data available".to_string(),
        ));
    }

    // Build output
    let all_passed = checks.iter().all(|(_, ok, _)| *ok);
    let mut lines = vec![
        Line::from(format!("Threshold Check (since {})", since)),
        Line::from("=============================="),
        Line::from(""),
    ];

    for (name, ok, detail) in &checks {
        let marker = if *ok { "✓" } else { "✗" };
        lines.push(Line::from(format!("{} {}: {}", marker, name, detail)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(format!(
        "Overall: {}",
        if all_passed { "PASS" } else { "FAIL" }
    )));

    let cell_type = if all_passed {
        HistoryCellType::Notice
    } else {
        HistoryCellType::Error
    };

    widget.history_push(crate::history_cell::PlainHistoryCell::new(lines, cell_type));
}
