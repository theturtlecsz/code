//! Pipeline configurator widget for interactive stage selection
//!
//! SPEC-947: Pipeline UI Configurator - Phase 2 Tasks 2.1 & 2.3
//!
//! Provides an interactive TUI modal for visually selecting which pipeline stages
//! to execute, with real-time cost/time estimates, dependency validation, and
//! warning display. Supports keyboard navigation and saves configuration to
//! per-SPEC `pipeline.toml`.

use super::pipeline_config::{PipelineConfig, StageType, ValidationResult};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::{Block, Borders, Clear, Widget},
    Frame,
};

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

    /// Model selection mode (when in StageDetails view)
    pub model_selection_mode: bool,

    /// Currently selected slot index (for navigation in model selection)
    pub selected_model_index: usize,

    /// Model picker mode (when choosing model for a specific slot)
    pub model_picker_mode: bool,

    /// Currently selected model in picker
    pub picker_selected_index: usize,
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
            model_selection_mode: false,
            selected_model_index: 0,
            model_picker_mode: false,
            picker_selected_index: 0,
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

    /// Get ALL available models (complete registry)
    ///
    /// Returns complete list of all models that can be used for any role
    /// VALIDATED: Only models available through our MCP integrations
    /// REMOVED: GPT-4, GPT-5 (no OpenAI API keys in this project)
    /// TODO: Add Gemini 3 models (released 2025-11-18) - see MODEL_ASSESSMENT_SPEC_PROMPT.md
    ///
    /// # Returns
    /// Vector of all model names (cheap to expensive)
    pub fn get_all_available_models() -> Vec<String> {
        vec![
            // Cheap models (Tier 0-1) - Native/Fast
            "gemini".to_string(),
            "claude".to_string(),
            "code".to_string(),

            // Cheap models (Tier 1) - Cost-optimized
            "gpt5_1_mini".to_string(),
            "gemini-flash".to_string(),
            "claude-haiku".to_string(),
            "gpt5_1".to_string(),

            // Premium models (Tier 3) - High-capability
            "claude-sonnet".to_string(),
            "gemini-pro".to_string(),
            "gpt5_1_codex".to_string(),
            "claude-opus".to_string(),
        ]
    }

    /// Get default models for a stage (for initialization)
    ///
    /// Returns default agent lineup from subagent_defaults.rs
    ///
    /// # Arguments
    /// * `stage` - Stage type to get models for
    ///
    /// # Returns
    /// Vector of model names (e.g., ["gemini-flash", "claude-haiku", "gpt5_1"])
    pub fn get_default_models(stage: &StageType) -> Vec<String> {
        match stage {
            StageType::New => vec!["gemini".to_string(), "claude".to_string(), "code".to_string()],
            StageType::Specify => vec!["gpt5_1_mini".to_string()],
            StageType::Plan => vec!["gemini-flash".to_string(), "claude-haiku".to_string(), "gpt5_1".to_string()],
            StageType::Tasks => vec!["gpt5_1_mini".to_string()],
            StageType::Implement => vec!["gpt5_1_codex".to_string(), "claude-haiku".to_string()],
            StageType::Validate => vec!["gemini-flash".to_string(), "claude-haiku".to_string(), "gpt5_1".to_string()],
            StageType::Audit => vec!["gpt5_codex".to_string(), "claude-sonnet".to_string(), "gemini-pro".to_string()],
            StageType::Unlock => vec!["gpt5_codex".to_string(), "claude-sonnet".to_string(), "gemini-pro".to_string()],
        }
    }

    /// Get number of model slots for a stage
    ///
    /// # Arguments
    /// * `stage` - Stage type
    ///
    /// # Returns
    /// Number of model slots/roles for this stage
    pub fn get_stage_slot_count(stage: &StageType) -> usize {
        Self::get_default_models(stage).len()
    }

    /// Get selected models for a stage
    ///
    /// Returns models from pending_config.stage_models or defaults
    ///
    /// # Arguments
    /// * `stage` - Stage type to get selected models for
    ///
    /// # Returns
    /// Vector of selected model names
    pub fn get_selected_models(&self, stage: &StageType) -> Vec<String> {
        self.pending_config
            .stage_models
            .get(stage)
            .cloned()
            .unwrap_or_else(|| Self::get_default_models(stage))
    }

    /// Assign model to a specific slot
    ///
    /// # Arguments
    /// * `slot_index` - Index of the slot/role (0, 1, 2 for 3-agent stages)
    /// * `model` - Model name to assign to this slot
    ///
    /// # Effects
    /// - Updates pending_config.stage_models[stage][slot_index] = model
    pub fn assign_model_to_slot(&mut self, slot_index: usize, model: String) {
        let stage = self.all_stages[self.selected_index];
        let slot_count = Self::get_stage_slot_count(&stage);

        if slot_index >= slot_count {
            return;
        }

        let models = self.pending_config
            .stage_models
            .entry(stage)
            .or_insert_with(|| Self::get_default_models(&stage));

        // Ensure models vec has enough slots
        while models.len() < slot_count {
            models.push(Self::get_default_models(&stage)[models.len()].clone());
        }

        // Assign model to specific slot
        models[slot_index] = model;
    }

    /// Handle keyboard event
    ///
    /// # Arguments
    /// * `key` - Keyboard event from crossterm
    ///
    /// # Returns
    /// ConfigAction indicating what to do next
    pub fn handle_key_event(&mut self, key: KeyEvent) -> ConfigAction {
        // Model picker mode (when choosing model for a specific slot)
        if self.model_picker_mode {
            return self.handle_model_picker_key(key);
        }

        // Model selection mode (when in StageDetails view)
        if self.model_selection_mode {
            return self.handle_model_selection_key(key);
        }

        // Stage details view mode
        if self.view_mode == ViewMode::StageDetails {
            return self.handle_stage_details_key(key);
        }

        // Stage list view mode (default)
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
                // Switch to StageDetails view
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

    /// Handle keyboard events in StageDetails view
    fn handle_stage_details_key(&mut self, key: KeyEvent) -> ConfigAction {
        match key.code {
            KeyCode::Char('m') | KeyCode::Enter => {
                // Activate model selection mode
                self.model_selection_mode = true;
                self.selected_model_index = 0;
                ConfigAction::Continue
            }
            KeyCode::Esc => {
                // Go back to StageList view
                self.view_mode = ViewMode::StageList;
                ConfigAction::Continue
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                // Save and exit
                if self.has_errors() {
                    ConfigAction::Continue
                } else if self.has_warnings() {
                    ConfigAction::ShowConfirmation
                } else {
                    ConfigAction::SaveAndExit
                }
            }
            _ => ConfigAction::Continue,
        }
    }

    /// Handle keyboard events in model selection mode (slot navigation)
    fn handle_model_selection_key(&mut self, key: KeyEvent) -> ConfigAction {
        let stage = self.all_stages[self.selected_index];
        let slot_count = Self::get_stage_slot_count(&stage);

        match key.code {
            KeyCode::Up => {
                if self.selected_model_index > 0 {
                    self.selected_model_index -= 1;
                }
                ConfigAction::Continue
            }
            KeyCode::Down => {
                if self.selected_model_index < slot_count.saturating_sub(1) {
                    self.selected_model_index += 1;
                }
                ConfigAction::Continue
            }
            KeyCode::Enter => {
                // Open model picker for current slot
                self.model_picker_mode = true;
                // Find current model in all_available_models to set picker index
                let current_models = self.get_selected_models(&stage);
                let current_model = &current_models[self.selected_model_index];
                let all_models = Self::get_all_available_models();
                self.picker_selected_index = all_models
                    .iter()
                    .position(|m| m == current_model)
                    .unwrap_or(0);
                ConfigAction::Continue
            }
            KeyCode::Char('m') | KeyCode::Esc => {
                // Exit model selection mode
                self.model_selection_mode = false;
                self.selected_model_index = 0;
                ConfigAction::Continue
            }
            _ => ConfigAction::Continue,
        }
    }

    /// Handle keyboard events in model picker mode
    fn handle_model_picker_key(&mut self, key: KeyEvent) -> ConfigAction {
        let all_models = Self::get_all_available_models();

        match key.code {
            KeyCode::Up => {
                if self.picker_selected_index > 0 {
                    self.picker_selected_index -= 1;
                }
                ConfigAction::Continue
            }
            KeyCode::Down => {
                if self.picker_selected_index < all_models.len().saturating_sub(1) {
                    self.picker_selected_index += 1;
                }
                ConfigAction::Continue
            }
            KeyCode::Enter => {
                // Assign selected model to current slot
                let model = all_models[self.picker_selected_index].clone();
                self.assign_model_to_slot(self.selected_model_index, model);
                // Exit picker mode
                self.model_picker_mode = false;
                self.picker_selected_index = 0;
                ConfigAction::Continue
            }
            KeyCode::Esc => {
                // Cancel picker without changing
                self.model_picker_mode = false;
                self.picker_selected_index = 0;
                ConfigAction::Continue
            }
            _ => ConfigAction::Continue,
        }
    }
}

// ============================================================================
// Widget Rendering (Phase 2 Task 2.3)
// ============================================================================

/// Pipeline configurator widget
///
/// Renders interactive modal for stage selection with centered overlay
pub struct PipelineConfiguratorWidget;

impl PipelineConfiguratorWidget {
    /// Render configurator modal
    ///
    /// # Arguments
    /// * `frame` - Ratatui frame for rendering
    /// * `state` - Configurator state (mutable for future interactivity)
    ///
    /// # Layout
    /// - Centered overlay (80% width, 70% height)
    /// - Main border block with SPEC ID title
    /// - Split into left pane (40%) and right pane (60%)
    /// - Left: Stage selector (Phase 3)
    /// - Right: Stage details + warnings (Phase 3)
    /// - Bottom: Help bar (Phase 3)
    pub fn render(frame: &mut Frame, state: &mut PipelineConfiguratorState) {
        // Create centered overlay (80% width, 70% height)
        let area = centered_rect(80, 70, frame.size());

        // Clear background for modal overlay
        frame.render_widget(Clear, area);

        // Main border block with title
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Pipeline Configuration: {} ", state.spec_id));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Split into left (40%) and right (60%) panes
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner_area);

        // Left pane: Stage selector (checkbox list) - Phase 3 Task 3.1 ✅
        super::stage_selector::render_stage_selector(frame, chunks[0], state);

        // Right pane: Stage details + warnings - Phase 3 Task 3.2 ✅
        super::stage_details::render_stage_details(frame, chunks[1], state);

        // Bottom: Help bar - Phase 3
        // Note: Help bar will be rendered in a separate bottom section in Phase 3
    }
}

/// Create centered rectangle
///
/// # Arguments
/// * `percent_x` - Width percentage (0-100)
/// * `percent_y` - Height percentage (0-100)
/// * `r` - Available area
///
/// # Returns
/// Centered rectangle with specified dimensions
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ============================================================================
// Placeholder Render Functions (Phase 3 Implementation)
// ============================================================================

/// Render help bar (bottom of modal) - Phase 3 Task 3.3
///
/// Will render:
/// - Key bindings: [↑↓] Navigate, [Space] Toggle, [Enter] Details, [q] Save & Run, [Esc] Cancel
/// - Conditional error message if save blocked
#[allow(dead_code)]
fn render_help_bar(_frame: &mut Frame, _area: Rect, _state: &PipelineConfiguratorState) {
    // TODO: Phase 3 Task 3.3 - Implement help bar widget
    // See implementation plan lines 580-590 for details
}
