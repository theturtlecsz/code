//! Configuration hot-reload handler for TUI integration.
//!
//! This module provides the infrastructure for handling config file changes
//! in the TUI without requiring a restart.
//!
//! Note: Hot-reload watcher infrastructure ready, TUI integration pending.

#![allow(dead_code, unused_imports)] // TUI integration pending

//! # Integration Points
//!
//! To fully integrate config hot-reload into the TUI:
//!
//! 1. **App Initialization** (`tui/src/lib.rs::run_main`):
//!    ```rust,ignore
//!    use codex_spec_kit::config::HotReloadWatcher;
//!
//!    // Create watcher after loading initial config
//!    let watcher = HotReloadWatcher::new(
//!        config_path,
//!        Duration::from_secs(2)
//!    ).await?;
//!    ```
//!
//! 2. **Event Loop** (`tui/src/app.rs` or equivalent):
//!    ```rust,ignore
//!    loop {
//!        tokio::select! {
//!            // Existing event handlers...
//!
//!            // Add config reload handler
//!            Some(event) = watcher.recv_event() => {
//!                handle_config_reload_event(event, &mut app);
//!            }
//!        }
//!    }
//!    ```
//!
//! 3. **UI Notifications**: Show toast/notification for reload events
//!
//! # Example
//!
//! ```no_run
//! use codex_spec_kit::config::{HotReloadWatcher, ConfigReloadEvent};
//! use std::time::Duration;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let watcher = HotReloadWatcher::new(
//!     "~/.code/config.toml",
//!     Duration::from_secs(2)
//! ).await?;
//!
//! loop {
//!     match watcher.recv_event().await {
//!         Some(ConfigReloadEvent::FileChanged(path)) => {
//!             println!("Config changed: {}", path.display());
//!         }
//!         Some(ConfigReloadEvent::ReloadSuccess) => {
//!             println!("✅ Config reloaded");
//!             // Refresh UI components here
//!         }
//!         Some(ConfigReloadEvent::ReloadFailed(err)) => {
//!             eprintln!("❌ Reload failed: {}", err);
//!         }
//!         None => break,
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use codex_spec_kit::config::{AppConfig, ConfigReloadEvent};
use std::sync::Arc;

/// Config reload notification for TUI.
#[derive(Debug, Clone)]
pub enum ReloadNotification {
    /// Config file changed (before reload).
    FileChanged { path: String },

    /// Config successfully reloaded.
    Success {
        /// Number of model configs changed
        models_changed: usize,
        /// Quality gates settings changed
        quality_gates_changed: bool,
        /// Cost limits changed
        cost_changed: bool,
    },

    /// Config reload failed (validation error).
    Failed {
        error: String,
        /// Old config preserved
        preserved: bool,
    },
}

/// Handle config reload event from HotReloadWatcher.
///
/// This function processes reload events and generates TUI-friendly notifications.
///
/// # Arguments
///
/// * `event` - Event from HotReloadWatcher
/// * `old_config` - Previous config for comparison (optional)
///
/// # Returns
///
/// Notification to display in TUI, or None if no notification needed.
///
/// # Example
///
/// ```ignore
/// # use codex_spec_kit::config::{ConfigReloadEvent, AppConfig};
/// # use codex_tui::chatwidget::spec_kit::config_reload::handle_reload_event;
/// # use std::sync::Arc;
/// # fn example(event: ConfigReloadEvent, old_config: Arc<AppConfig>) {
/// if let Some(notification) = handle_reload_event(event, Some(old_config)) {
///     // Show notification in TUI
///     println!("{:?}", notification);
/// }
/// # }
/// ```
pub fn handle_reload_event(
    event: ConfigReloadEvent,
    old_config: Option<Arc<AppConfig>>,
) -> Option<ReloadNotification> {
    match event {
        ConfigReloadEvent::FileChanged(path) => Some(ReloadNotification::FileChanged {
            path: path.display().to_string(),
        }),

        ConfigReloadEvent::ReloadSuccess => {
            // If we have old config, compute what changed
            let notification = if let Some(_old) = old_config {
                // TODO: Implement detailed change detection
                ReloadNotification::Success {
                    models_changed: 0, // Placeholder
                    quality_gates_changed: false,
                    cost_changed: false,
                }
            } else {
                ReloadNotification::Success {
                    models_changed: 0,
                    quality_gates_changed: false,
                    cost_changed: false,
                }
            };

            Some(notification)
        }

        ConfigReloadEvent::ReloadFailed(error) => {
            Some(ReloadNotification::Failed {
                error,
                preserved: true, // HotReloadWatcher always preserves old config
            })
        }
    }
}

/// Detect changes between old and new config.
///
/// Returns a summary of what changed for UI notifications.
///
/// # Arguments
///
/// * `old` - Previous config
/// * `new` - New config
///
/// # Returns
///
/// Tuple of (models_changed, quality_gates_changed, cost_changed)
pub fn detect_config_changes(old: &AppConfig, new: &AppConfig) -> (usize, bool, bool) {
    // Count models that changed
    let models_changed = count_model_changes(old, new);

    // Check if quality gates changed
    let quality_gates_changed = old.quality_gates.enabled != new.quality_gates.enabled
        || (old.quality_gates.consensus_threshold - new.quality_gates.consensus_threshold).abs()
            > f32::EPSILON;

    // Check if cost settings changed
    let cost_changed = old.cost.enabled != new.cost.enabled
        || old.cost.daily_limit_usd != new.cost.daily_limit_usd
        || old.cost.monthly_limit_usd != new.cost.monthly_limit_usd;

    (models_changed, quality_gates_changed, cost_changed)
}

/// Count how many model configs changed between old and new.
fn count_model_changes(old: &AppConfig, new: &AppConfig) -> usize {
    let mut changes = 0;

    // Models added
    for key in new.models.keys() {
        if !old.models.contains_key(key) {
            changes += 1;
        }
    }

    // Models removed
    for key in old.models.keys() {
        if !new.models.contains_key(key) {
            changes += 1;
        }
    }

    // Models modified (simplified check)
    for (key, new_model) in &new.models {
        if let Some(old_model) = old.models.get(key)
            && (old_model.model != new_model.model
                || (old_model.temperature - new_model.temperature).abs() > f32::EPSILON)
        {
            changes += 1;
        }
    }

    changes
}

/// Check if config reload should be deferred.
///
/// Returns true if the TUI is currently busy with operations that
/// shouldn't be interrupted (e.g., quality gate execution).
///
/// # Arguments
///
/// * `quality_gate_active` - Whether a quality gate is currently running
/// * `agent_running` - Whether any agents are currently executing
///
/// # Example
///
/// ```ignore
/// # use codex_tui::chatwidget::spec_kit::config_reload::should_defer_reload;
/// let defer = should_defer_reload(true, false);
/// assert!(defer, "Should defer when quality gate is active");
/// ```
pub fn should_defer_reload(quality_gate_active: bool, agent_running: bool) -> bool {
    // Defer reload if:
    // 1. Quality gate is active (don't interrupt validation)
    // 2. Agents are running (don't interrupt agent execution)
    quality_gate_active || agent_running
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_spec_kit::config::{
        AppConfig, ConsensusConfig, CostConfig, EvidenceConfig, ModelConfig, QualityGateConfig,
    };
    use std::collections::HashMap;

    fn create_test_config() -> AppConfig {
        AppConfig {
            models: HashMap::new(),
            quality_gates: QualityGateConfig {
                enabled: true,
                consensus_threshold: 0.7,
                min_test_coverage: Some(80.0),
                schema_validation: true,
            },
            cost: CostConfig {
                enabled: false,
                daily_limit_usd: Some(10.0),
                monthly_limit_usd: Some(300.0),
                alert_threshold: 0.8,
            },
            evidence: EvidenceConfig {
                enabled: true,
                base_dir: "./evidence".into(),
                max_size_per_spec_mb: 25,
                retention_days: 90,
            },
            consensus: ConsensusConfig {
                min_agents: 2,
                max_agents: 5,
                timeout_seconds: 300,
            },
        }
    }

    #[test]
    fn test_handle_file_changed_event() {
        let event = ConfigReloadEvent::FileChanged("/test/config.toml".into());
        let notification = handle_reload_event(event, None);

        assert!(matches!(
            notification,
            Some(ReloadNotification::FileChanged { .. })
        ));
    }

    #[test]
    fn test_handle_success_event() {
        let event = ConfigReloadEvent::ReloadSuccess;
        let notification = handle_reload_event(event, None);

        assert!(matches!(
            notification,
            Some(ReloadNotification::Success { .. })
        ));
    }

    #[test]
    fn test_handle_failed_event() {
        let event = ConfigReloadEvent::ReloadFailed("Parse error".to_string());
        let notification = handle_reload_event(event, None);

        if let Some(ReloadNotification::Failed { preserved, .. }) = notification {
            assert!(preserved, "Old config should be preserved on failure");
        } else {
            panic!("Expected Failed notification");
        }
    }

    #[test]
    fn test_detect_config_changes_none() {
        let config1 = create_test_config();
        let config2 = create_test_config();

        let (models_changed, qg_changed, cost_changed) = detect_config_changes(&config1, &config2);

        assert_eq!(models_changed, 0);
        assert!(!qg_changed);
        assert!(!cost_changed);
    }

    #[test]
    fn test_detect_quality_gates_change() {
        let config1 = create_test_config();
        let mut config2 = create_test_config();
        config2.quality_gates.enabled = false;

        let (_, qg_changed, _) = detect_config_changes(&config1, &config2);

        assert!(qg_changed, "Quality gates change should be detected");
    }

    #[test]
    fn test_detect_cost_change() {
        let config1 = create_test_config();
        let mut config2 = create_test_config();
        config2.cost.daily_limit_usd = Some(20.0);

        let (_, _, cost_changed) = detect_config_changes(&config1, &config2);

        assert!(cost_changed, "Cost change should be detected");
    }

    #[test]
    fn test_detect_model_addition() {
        let config1 = create_test_config();
        let mut config2 = create_test_config();

        config2.models.insert(
            "openai".to_string(),
            ModelConfig {
                model: "gpt-4".to_string(),
                endpoint: None,
                temperature: 0.7,
                max_tokens: None,
                cost_per_input_million: 0.0,
                cost_per_output_million: 0.0,
                retry: Default::default(),
            },
        );

        let (models_changed, _, _) = detect_config_changes(&config1, &config2);

        assert_eq!(models_changed, 1, "Model addition should be detected");
    }

    #[test]
    fn test_should_defer_when_quality_gate_active() {
        assert!(should_defer_reload(true, false));
    }

    #[test]
    fn test_should_defer_when_agent_running() {
        assert!(should_defer_reload(false, true));
    }

    #[test]
    fn test_should_not_defer_when_idle() {
        assert!(!should_defer_reload(false, false));
    }
}
