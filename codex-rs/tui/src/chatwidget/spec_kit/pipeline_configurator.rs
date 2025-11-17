//! Pipeline configurator widget for interactive stage selection
//!
//! SPEC-947: Pipeline UI Configurator - Phase 2 Task 2.1
//!
//! Provides an interactive TUI modal for visually selecting which pipeline stages
//! to execute, with real-time cost/time estimates, dependency validation, and
//! warning display. Supports keyboard navigation and saves configuration to
//! per-SPEC `pipeline.toml`.

use super::pipeline_config::{PipelineConfig, StageType, ValidationResult};
use crossterm::event::{KeyCode, KeyEvent};

/// Pipeline configurator state machine
pub struct PipelineConfiguratorState {
    /// SPEC ID being configured
    pub spec_id: String,

    /// Currently selected stage index (for keyboard navigation)
    pub selected_index: usize,

    /// Stage enable/disable states (maps to PipelineConfig.enabled_stages)
    pub stage_states: Vec<bool>,

    /// All available stages (fixed list of 8)
    pub all_stages: Vec<StageType>,

    /// Pending configuration (modified as user toggles stages)
    pub pending_config: PipelineConfig,

    /// Current view mode
    pub view_mode: ViewMode,

    /// Validation warnings (updated on every toggle)
    pub warnings: Vec<String>,
}

/// View mode for configurator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Left pane: checkbox list (default)
    StageList,
    /// Right pane: detailed info (future enhancement)
    StageDetails,
}

/// Configuration action (state machine transitions)
pub enum ConfigAction {
    /// Keep modal open
    Continue,
    /// Show warning confirmation dialog
    ShowConfirmation,
    /// Save pipeline.toml and close
    SaveAndExit,
    /// Discard changes and close
    CancelAndExit,
}

impl PipelineConfiguratorState {
    /// Create new configurator state
    ///
    /// # Arguments
    /// * `spec_id` - SPEC ID to configure
    /// * `initial_config` - Loaded configuration (from TOML or defaults)
    ///
    /// # Returns
    /// New state initialized with current configuration
    pub fn new(spec_id: String, initial_config: PipelineConfig) -> Self {
        let all_stages = vec![
            StageType::New,
            StageType::Specify,
            StageType::Plan,
            StageType::Tasks,
            StageType::Implement,
            StageType::Validate,
            StageType::Audit,
            StageType::Unlock,
        ];

        let stage_states: Vec<bool> = all_stages
            .iter()
            .map(|s| initial_config.is_enabled(*s))
            .collect();

        let mut state = Self {
            spec_id,
            selected_index: 0,
            stage_states,
            all_stages,
            pending_config: initial_config,
            view_mode: ViewMode::StageList,
            warnings: Vec::new(),
        };

        // Initial validation
        state.validate_and_update_warnings();

        state
    }

    /// Toggle stage at index and revalidate
    ///
    /// # Arguments
    /// * `index` - Stage index to toggle (0-7)
    ///
    /// # Effects
    /// - Flips stage_states[index]
    /// - Syncs pending_config.enabled_stages from stage_states
    /// - Re-runs validation and updates warnings
    pub fn toggle_stage(&mut self, index: usize) {
        if index < self.stage_states.len() {
            self.stage_states[index] = !self.stage_states[index];
            self.sync_config_from_states();
            self.validate_and_update_warnings();
        }
    }

    /// Sync pending_config.enabled_stages from stage_states
    ///
    /// Rebuilds enabled_stages vector from current checkbox states
    fn sync_config_from_states(&mut self) {
        self.pending_config.enabled_stages = self
            .all_stages
            .iter()
            .enumerate()
            .filter(|(i, _)| self.stage_states[*i])
            .map(|(_, stage)| *stage)
            .collect();
    }

    /// Run validation and update warnings
    ///
    /// Calls pipeline_config.validate() and updates warnings list
    fn validate_and_update_warnings(&mut self) {
        self.warnings.clear();
        if let Ok(result) = self.pending_config.validate() {
            self.warnings = result.warnings;
        }
    }

    /// Calculate total cost of enabled stages
    ///
    /// # Returns
    /// Sum of cost estimates for all enabled stages (in USD)
    pub fn total_cost(&self) -> f64 {
        self.pending_config
            .enabled_stages
            .iter()
            .map(|s| s.cost_estimate())
            .sum()
    }

    /// Calculate total duration of enabled stages
    ///
    /// # Returns
    /// Sum of duration estimates for all enabled stages (in minutes)
    pub fn total_duration(&self) -> u32 {
        self.pending_config
            .enabled_stages
            .iter()
            .map(|s| s.duration_estimate())
            .sum()
    }

    /// Check if any errors present (block save)
    ///
    /// # Returns
    /// True if any warnings start with "Error:"
    pub fn has_errors(&self) -> bool {
        self.warnings.iter().any(|w| w.starts_with("Error:"))
    }

    /// Check if any warnings present (show confirmation)
    ///
    /// # Returns
    /// True if any warnings start with "⚠" or "Warning:"
    pub fn has_warnings(&self) -> bool {
        self.warnings
            .iter()
            .any(|w| w.starts_with("⚠") || w.starts_with("Warning:"))
    }

    /// Handle keyboard event
    ///
    /// # Arguments
    /// * `key` - Keyboard event from crossterm
    ///
    /// # Returns
    /// ConfigAction indicating what to do next
    pub fn handle_key_event(&mut self, key: KeyEvent) -> ConfigAction {
        match key.code {
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                ConfigAction::Continue
            }
            KeyCode::Down => {
                if self.selected_index < self.all_stages.len() - 1 {
                    self.selected_index += 1;
                }
                ConfigAction::Continue
            }
            KeyCode::Char(' ') => {
                // Toggle selected stage
                self.toggle_stage(self.selected_index);
                ConfigAction::Continue
            }
            KeyCode::Enter => {
                // Future: Switch to StageDetails view
                self.view_mode = ViewMode::StageDetails;
                ConfigAction::Continue
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                // Save and exit (with validation)
                if self.has_errors() {
                    // Errors block save (keep modal open)
                    ConfigAction::Continue
                } else if self.has_warnings() {
                    ConfigAction::ShowConfirmation
                } else {
                    ConfigAction::SaveAndExit
                }
            }
            KeyCode::Esc => {
                // Cancel (discard changes)
                ConfigAction::CancelAndExit
            }
            _ => ConfigAction::Continue,
        }
    }
}
