# SPEC-947-IMPL: Pipeline UI Configurator Implementation Plan

**Research SPEC**: SPEC-947 (complete)
**Implementation Sequence**: 3/3 (Final - user-facing feature)
**Estimated Duration**: 24-32 hours (1.5-2 weeks)
**Dependencies**: SPEC-948 pipeline_config.rs (HARD DEPENDENCY)
**Created**: 2025-11-16
**Priority**: P1 - High (UX Enhancement)

---

## Executive Summary

This implementation delivers an interactive TUI-based pipeline configurator modal, enabling users to visually select which stages to execute via checkbox-based interface. Builds on SPEC-948's `pipeline_config.rs` backend, creating 4 new widget files (800-1,050 LOC) with real-time cost/time estimates, dependency validation, and warning display. Supports keyboard navigation (↑/↓/Space/Enter/q/Esc) and saves configuration to per-SPEC `pipeline.toml`.

**Cost Baseline Note**: Assumes SPEC-949 GPT-5 migration complete (baseline $2.36). Pre-SPEC-949 baseline was $2.71 (GPT-4).

**Strategic Impact**:
- **User Experience**: Visual stage selection vs manual TOML editing
- **Cost Awareness**: Real-time cost display ($0.66-$2.71 as stages toggle)
- **Safety**: Dependency warnings prevent invalid configurations
- **Workflow Discovery**: Users discover workflow patterns through UI exploration

---

## Implementation Phases

### Phase 1: Reuse SPEC-948 Config Layer (0 hours - Already Done)

**Objective**: Verify SPEC-948's `pipeline_config.rs` provides required API

**Verification Checklist**:
- ✅ **PipelineConfig::load()** exists (precedence loading)
- ✅ **PipelineConfig::save()** exists (write to TOML)
- ✅ **PipelineConfig::validate()** exists (dependency validation)
- ✅ **PipelineConfig::is_enabled(stage)** exists (check stage state)
- ✅ **StageType** enum exists (8 variants)
- ✅ **ValidationResult** struct exists (warnings list)

**No Implementation Needed**: SPEC-948 Phase 1 provides complete data layer

**Risk**: If SPEC-948 API incomplete, extend pipeline_config.rs before proceeding

---

### Phase 2: Widget Core (Week 3-4, Days 1-3, 8-10 hours)

**Objective**: Create main `PipelineConfiguratorWidget` with state machine and rendering

**Tasks**:

**Task 2.1**: Create state machine data structure
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_configurator.rs` (NEW)
- **Action**: Define configurator state
- **Changes**:
  - `PipelineConfiguratorState` struct (selected_index, stage_states: Vec<bool>, pending_config, warnings)
  - `ViewMode` enum (StageList, StageDetails)
  - `ConfigAction` enum (Continue, ShowConfirmation, SaveAndExit, CancelAndExit)
  - State initialization: `new(spec_id: String, initial_config: PipelineConfig) -> Self`
- **LOC**: ~80-100 lines (state structures)
- **Rationale**: State machine separates logic from rendering, enables testing
- **Dependencies**: SPEC-948 pipeline_config.rs (imports PipelineConfig)

```rust
// Example state structure (lines 1-100):
use super::pipeline_config::{PipelineConfig, StageType, ValidationResult};
use ratatui::style::{Color, Style};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    StageList,      // Left pane: checkbox list (default)
    StageDetails,   // Right pane: detailed info (future enhancement)
}

pub enum ConfigAction {
    Continue,             // Keep modal open
    ShowConfirmation,     // Show warning confirmation dialog
    SaveAndExit,          // Save pipeline.toml and close
    CancelAndExit,        // Discard changes and close
}

impl PipelineConfiguratorState {
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

        Self {
            spec_id,
            selected_index: 0,
            stage_states,
            all_stages,
            pending_config: initial_config,
            view_mode: ViewMode::StageList,
            warnings: Vec::new(),
        }
    }

    /// Toggle stage at index and revalidate
    pub fn toggle_stage(&mut self, index: usize) {
        if index < self.stage_states.len() {
            self.stage_states[index] = !self.stage_states[index];
            self.sync_config_from_states();
            self.validate_and_update_warnings();
        }
    }

    /// Sync pending_config.enabled_stages from stage_states
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
    fn validate_and_update_warnings(&mut self) {
        self.warnings.clear();
        if let Ok(result) = self.pending_config.validate() {
            self.warnings = result.warnings;
        }
    }

    /// Calculate total cost of enabled stages
    pub fn total_cost(&self) -> f64 {
        self.pending_config
            .enabled_stages
            .iter()
            .map(|s| s.cost_estimate())
            .sum()
    }

    /// Calculate total duration of enabled stages (minutes)
    pub fn total_duration(&self) -> u32 {
        self.pending_config
            .enabled_stages
            .iter()
            .map(|s| s.duration_estimate())
            .sum()
    }

    pub fn has_errors(&self) -> bool {
        // Check if any warnings are errors (start with "Error:")
        self.warnings.iter().any(|w| w.starts_with("Error:"))
    }

    pub fn has_warnings(&self) -> bool {
        self.warnings.iter().any(|w| w.starts_with("⚠"))
    }
}
```

**Task 2.2**: Implement event handling
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_configurator.rs` (continuation)
- **Action**: Add keyboard event handling
- **Changes**:
  - `handle_key_event(&mut self, key: KeyEvent) -> ConfigAction`
  - Navigation: ↑/↓ (move selection), Space (toggle), Enter (show details)
  - Actions: q (save & exit), Esc (cancel & exit)
  - Validation: Block save if errors, show confirmation if warnings
- **LOC**: ~60-80 lines (event handling)
- **Rationale**: Keyboard-driven navigation standard in TUI apps
- **Dependencies**: Task 2.1 (state structure exists)

```rust
// Example event handling (lines 150-230):
use crossterm::event::{KeyCode, KeyEvent};

impl PipelineConfiguratorState {
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
            KeyCode::Char('q') => {
                // Save and exit (with validation)
                if self.has_errors() {
                    // Errors block save (show error modal or flash message)
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
```

**Task 2.3**: Implement widget rendering
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_configurator.rs` (continuation)
- **Action**: Create main widget with centered overlay
- **Changes**:
  - `PipelineConfiguratorWidget` struct (implements ratatui::widgets::Widget or StatefulWidget)
  - `render(area: Rect, buf: &mut Buffer, state: &mut PipelineConfiguratorState)`
  - Layout: Centered overlay (80% width, 70% height), split into left (40%) and right (60%) panes
  - Background: Clear area, bordered block with title
- **LOC**: ~80-100 lines (main widget rendering)
- **Rationale**: Ratatui's widget pattern for composable UI
- **Dependencies**: Task 2.1 (state), Task 2.2 (event handling)

```rust
// Example widget rendering (lines 250-350):
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear},
    Frame,
};

pub struct PipelineConfiguratorWidget;

impl PipelineConfiguratorWidget {
    pub fn render(frame: &mut Frame, state: &mut PipelineConfiguratorState) {
        // Create centered overlay (80% width, 70% height)
        let area = centered_rect(80, 70, frame.size());

        // Clear background with modal overlay
        frame.render_widget(Clear, area);

        // Main border block
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Pipeline Configuration: {}", state.spec_id));
        frame.render_widget(block, area);

        // Split into left (40%) and right (60%) panes
        let inner_area = area.inner(&ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner_area);

        // Left pane: Stage selector (checkbox list)
        render_stage_selector(frame, chunks[0], state);

        // Right pane: Stage details + warnings
        render_stage_details(frame, chunks[1], state);

        // Bottom: Help text
        render_help_bar(frame, area, state);
    }
}

/// Helper: Create centered rectangle
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
```

**Deliverables**:
- `pipeline_configurator.rs` created (~300-400 LOC)
- Widget state tests (~6-8 widget state tests: toggle, navigation, validation)

**Validation**:
```bash
# Compilation check
cd codex-rs && cargo build -p codex-tui

# Widget state tests
cargo test -p codex-tui pipeline_configurator::tests
```

**Success Criteria**:
- State machine compiles without errors
- Event handling covers all key codes (↑/↓/Space/Enter/q/Esc)
- Widget renders centered overlay (80×70% of terminal)
- 6-8 widget state tests passing

**Milestone 1**: Widget core functional, ready for interactive components

---

### Phase 3: Interactive Components (Week 4, Days 4-6, 6-8 hours)

**Objective**: Create stage selector (left pane) and stage details (right pane) widgets

**Tasks**:

**Task 3.1**: Create stage selector widget (checkbox list)
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/stage_selector.rs` (NEW)
- **Action**: Render checkbox list with indicators
- **Changes**:
  - `render_stage_selector(frame, area, state)` function
  - For each stage: `[✓]` or `[ ]`, stage name, cost, indicators ($, ⚠, 🔒)
  - Highlight selected row (background color)
  - Total cost/duration footer
- **LOC**: ~150-200 lines (list rendering + styling)
- **Rationale**: Left pane shows all stages at a glance
- **Dependencies**: Phase 2 Task 2.3 (rendering integration)

```rust
// Example stage selector (complete file 150-200 LOC):
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::pipeline_configurator::PipelineConfiguratorState;

pub fn render_stage_selector(frame: &mut Frame, area: Rect, state: &PipelineConfiguratorState) {
    let items: Vec<ListItem> = state
        .all_stages
        .iter()
        .enumerate()
        .map(|(i, stage)| {
            let checkbox = if state.stage_states[i] {
                "[✓]"
            } else {
                "[ ]"
            };

            let cost = stage.cost_estimate();
            let mut indicators = Vec::new();

            // High-cost indicator
            if cost > 0.50 {
                indicators.push("[$]");
            }

            // Quality gate indicator
            if stage.has_quality_gate() {
                indicators.push("[🔒]");
            }

            // Warning indicator (if stage has dependency issues)
            // (Check state.warnings for this stage)

            let line = format!(
                "{} {} (${:.2}) {}",
                checkbox,
                stage.display_name(),
                cost,
                indicators.join(" ")
            );

            let style = if i == state.selected_index {
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    // Footer with totals
    let total_cost = state.total_cost();
    let total_duration = state.total_duration();
    let enabled_count = state.stage_states.iter().filter(|&&s| s).count();

    let footer = format!(
        "\nTotal: {}/{} stages, ${:.2}, ~{} min",
        enabled_count,
        state.all_stages.len(),
        total_cost,
        total_duration
    );

    items.push(ListItem::new(footer).style(Style::default().fg(Color::Gray)));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Stage Selection"),
        )
        .highlight_symbol("▶ ");

    frame.render_widget(list, area);
}
```

**Task 3.2**: Create stage details widget (right pane)
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/stage_details.rs` (NEW)
- **Action**: Render detailed info for selected stage
- **Changes**:
  - `render_stage_details(frame, area, state)` function
  - Selected stage: name, description, agents, cost, duration, quality gate info, dependencies
  - Warnings section: Display validation warnings (errors in red, warnings in yellow)
  - Help text: Key bindings (Space to toggle, q to save, Esc to cancel)
- **LOC**: ~150-200 lines (detail rendering)
- **Rationale**: Right pane provides context for decision-making
- **Dependencies**: Phase 2 Task 2.3 (rendering integration)

```rust
// Example stage details (complete file 150-200 LOC):
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::pipeline_configurator::PipelineConfiguratorState;

pub fn render_stage_details(frame: &mut Frame, area: Rect, state: &PipelineConfiguratorState) {
    let selected_stage = &state.all_stages[state.selected_index];

    // Build detail text
    let mut lines = Vec::new();

    // Stage header
    lines.push(Line::from(vec![
        Span::styled(
            format!("> {}: ", selected_stage.display_name()),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(selected_stage.description()),
    ]));

    lines.push(Line::raw(""));

    // Agents (if multi-agent stage)
    lines.push(Line::from(vec![
        Span::styled("Agents: ", Style::default().fg(Color::Yellow)),
        Span::raw(selected_stage.agents().join(", ")),
    ]));

    lines.push(Line::raw(""));

    // Cost and duration
    lines.push(Line::from(vec![
        Span::styled("Cost: ", Style::default().fg(Color::Green)),
        Span::raw(format!("~${:.2}", selected_stage.cost_estimate())),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Duration: ", Style::default().fg(Color::Green)),
        Span::raw(format!("~{} min", selected_stage.duration_estimate())),
    ]));

    lines.push(Line::raw(""));

    // Quality gate
    if selected_stage.has_quality_gate() {
        lines.push(Line::from(vec![
            Span::styled("Quality Gate: ", Style::default().fg(Color::Magenta)),
            Span::raw("Post-stage checkpoint (3 agents vote)"),
        ]));
        lines.push(Line::raw(""));
    }

    // Dependencies
    let deps = selected_stage.dependencies();
    if !deps.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Dependencies: ", Style::default().fg(Color::Blue)),
        ]));
        for dep in deps {
            let dep_enabled = state.pending_config.is_enabled(dep);
            let status = if dep_enabled { "✓" } else { "✗" };
            lines.push(Line::from(vec![
                Span::raw(format!("  • {} ", status)),
                Span::raw(dep.display_name()),
            ]));
        }
        lines.push(Line::raw(""));
    }

    // Warnings section
    if !state.warnings.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Warnings:",
            Style::default().fg(Color::Red),
        )]));

        for warning in &state.warnings {
            let style = if warning.starts_with("Error:") {
                Style::default().fg(Color::Red)
            } else if warning.starts_with("⚠") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::from(vec![Span::styled(warning, style)]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Stage Details"),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
```

**Task 3.3**: Create help bar widget (bottom of modal)
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_configurator.rs` (extend)
- **Action**: Render help text at bottom of modal
- **Changes**:
  - `render_help_bar(frame, area, state)` function
  - Key bindings: [↑↓] Navigate, [Space] Toggle, [Enter] Details, [q] Save & Run, [Esc] Cancel
  - Conditional: If errors, show "Cannot save (errors present)"
- **LOC**: ~40-60 lines (help text rendering)
- **Rationale**: User discoverability of key bindings
- **Dependencies**: Phase 2 Task 2.3 (rendering integration)

**Deliverables**:
- `stage_selector.rs` created (~150-200 LOC)
- `stage_details.rs` created (~150-200 LOC)
- Help bar rendering (~40-60 LOC)
- Interaction tests (~4-6 tests: selection, toggle, warning display)

**Validation**:
```bash
# Interaction tests
cargo test -p codex-tui spec_kit::stage_selector::tests

# Manual TUI testing (launch configurator modal, navigate, toggle)
```

**Success Criteria**:
- Stage selector renders checkbox list with indicators
- Stage details shows selected stage info
- Help bar displays key bindings
- Real-time updates: Toggle stage → cost recalculates → warnings update
- 4-6 interaction tests passing

**Milestone 2**: Interactive components complete, full UI functional

---

### Phase 4: Command Integration (Week 4, Days 7-8, 4-6 hours)

**Objective**: Create `/speckit.configure` command and integrate with main TUI

**Tasks**:

**Task 4.1**: Create `/speckit.configure` command handler
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/commands/configure.rs` (NEW)
- **Action**: Add command to launch configurator modal
- **Changes**:
  - `handle_configure(spec_id: &str, widget: &mut ChatWidget) -> Result<(), String>`
  - Load existing config (per-SPEC > global > defaults)
  - Switch AppMode to PipelineConfig (or equivalent modal state)
  - Display configurator widget
  - On save: Write `docs/SPEC-{id}/pipeline.toml`, return to chat
  - On cancel: Discard changes, return to chat
- **LOC**: ~100-150 lines (command handler)
- **Rationale**: User entry point for configurator
- **Dependencies**: Phase 3 (widgets exist), SPEC-948 pipeline_config.rs (load/save)

```rust
// Example command handler (new file 100-150 LOC):
use super::super::super::ChatWidget;
use super::super::pipeline_config::PipelineConfig;
use super::super::pipeline_configurator::{
    PipelineConfiguratorState, PipelineConfiguratorWidget,
};

pub async fn handle_configure(
    spec_id: &str,
    widget: &mut ChatWidget,
) -> Result<(), String> {
    // 1. Load existing config (per-SPEC > global > defaults)
    let current_config = PipelineConfig::load(spec_id, None)?;

    // 2. Initialize configurator state
    let mut configurator_state = PipelineConfiguratorState::new(
        spec_id.to_string(),
        current_config,
    );

    // 3. Launch modal (switch AppMode or set widget.show_pipeline_configurator = true)
    // (Implementation depends on existing TUI architecture)

    // 4. Event loop (handle keyboard events until SaveAndExit or CancelAndExit)
    loop {
        // Render widget
        PipelineConfiguratorWidget::render(frame, &mut configurator_state);

        // Handle input
        if let Some(key_event) = poll_key_event() {
            match configurator_state.handle_key_event(key_event) {
                ConfigAction::SaveAndExit => {
                    // Save pipeline.toml
                    let config_path = format!("docs/{}/pipeline.toml", spec_id);
                    configurator_state.pending_config.save(&config_path)?;

                    // Show confirmation message
                    widget.submit_user_message(&format!(
                        "✅ Pipeline configuration saved: {}\n\
                         Enabled stages: {}\n\
                         Total cost: ~${:.2}\n\n\
                         Run `/speckit.auto {}` to execute.",
                        config_path,
                        configurator_state
                            .pending_config
                            .enabled_stages
                            .iter()
                            .map(|s| s.display_name())
                            .collect::<Vec<_>>()
                            .join(", "),
                        configurator_state.total_cost(),
                        spec_id
                    ));

                    break;
                }
                ConfigAction::CancelAndExit => {
                    // Discard changes, return to chat
                    break;
                }
                ConfigAction::ShowConfirmation => {
                    // Show confirmation dialog (future enhancement)
                    // For now, save anyway
                    // (Could integrate with existing confirmation dialog pattern)
                }
                ConfigAction::Continue => {
                    // Keep modal open
                }
            }
        }
    }

    // 5. Return to normal chat mode
    Ok(())
}
```

**Task 4.2**: Register `/speckit.configure` in command registry
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs` (follow registration pattern at line 280)
- **Action**: Add command to router/dispatcher
- **Changes**:
  - Add `"/speckit.configure" => handle_configure(spec_id, widget).await`
  - Validate spec_id exists before launching modal
- **LOC**: ~+10 lines (command registration)
- **Rationale**: Wire command into TUI command system
- **Dependencies**: Task 4.1 (handler exists)

**Task 4.3**: Add `--configure` flag to `/speckit.auto`
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` (extend /speckit.auto)
- **Action**: Add optional flag to launch configurator before execution
- **Changes**:
  - Parse `--configure` flag from command args
  - If present: Call handle_configure(spec_id) first
  - After save: Continue to pipeline execution
- **LOC**: ~+20 lines (flag parsing + integration)
- **Rationale**: Inline configuration workflow
- **Dependencies**: Task 4.1 (handler exists), SPEC-948 Phase 3 (CLI parsing)

**Task 4.4**: Create confirmation dialog widget (optional enhancement)
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/confirmation_dialog.rs` (NEW, optional)
- **Action**: Render confirmation dialog when warnings present
- **Changes**:
  - Modal overlay (smaller than configurator, centered)
  - Show warnings list
  - Buttons: [y] Yes, save and run | [n] No, go back
  - Integrate with handle_configure on ConfigAction::ShowConfirmation
- **LOC**: ~80-100 lines (optional)
- **Rationale**: Explicit user consent for quality gate bypass
- **Dependencies**: None (optional enhancement, can defer to later)

**Deliverables**:
- `commands/configure.rs` created (~100-150 LOC)
- Command registration (~+10 LOC)
- `--configure` flag integration (~+20 LOC)
- Optional: Confirmation dialog (~80-100 LOC)
- End-to-end tests (~3-4 tests: launch modal, save, cancel, --configure flag)

**Validation**:
```bash
# End-to-end tests
cargo test -p codex-tui spec_kit::commands::configure::tests

# Manual TUI testing:
# 1. Launch: /speckit.configure SPEC-947
# 2. Toggle stages, verify cost updates
# 3. Press 'q', verify pipeline.toml written
# 4. Run: /speckit.auto SPEC-947 --configure
# 5. Verify configurator launches before execution
```

**Success Criteria**:
- `/speckit.configure SPEC-ID` launches modal
- Modal displays current config (loaded from TOML if exists)
- Save writes `docs/SPEC-{id}/pipeline.toml` correctly
- Cancel discards changes without writing
- `--configure` flag works with /speckit.auto
- 3-4 end-to-end tests passing

**Milestone 3**: Full feature integration complete, user-facing

---

## Complete File Manifest

### New Files (SPEC-947-IMPL)

| File Path | Purpose | LOC | Tests | Phase |
|-----------|---------|-----|-------|-------|
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_configurator.rs` | Main widget + state | 300-400 | 6-8 state | Phase 2 |
| `codex-rs/tui/src/chatwidget/spec_kit/stage_selector.rs` | Checkbox list widget | 150-200 | - | Phase 3 |
| `codex-rs/tui/src/chatwidget/spec_kit/stage_details.rs` | Detail pane widget | 150-200 | - | Phase 3 |
| `codex-rs/tui/src/chatwidget/spec_kit/commands/configure.rs` | Command handler | 100-150 | 3-4 E2E | Phase 4 |
| `codex-rs/tui/src/chatwidget/spec_kit/confirmation_dialog.rs` | Confirmation modal (optional) | 80-100 | - | Phase 4 (opt) |

**Total New**: 4-5 files, ~780-1,050 LOC (widgets + command)

### Modified Files (SPEC-947-IMPL)

| File Path | Changes | LOC Change | Rationale | Risk | Phase |
|-----------|---------|------------|-----------|------|-------|
| `codex-rs/tui/src/chatwidget/mod.rs` | Add AppMode::PipelineConfig variant | +10/-0 | Mode switching | Low | Phase 2 |
| `codex-rs/tui/src/app.rs` | Handle mode switching, render configurator | +30/-0 | App integration | Medium | Phase 2 |
| `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` | Register /speckit.configure, --configure flag | +30/-0 | Command routing | Low | Phase 4 |

**Total Modified**: 3 files, ~+70/-0 LOC

---

## Test Coverage Plan

### Unit Test Matrix

| Module | Coverage Target | Test Count | Key Scenarios |
|--------|-----------------|------------|---------------|
| pipeline_configurator (state) | 80%+ | 6-8 | Toggle, navigation, validation, cost calculation |
| stage_selector (rendering) | 60%+ | 2-3 | Render list, highlight selection |
| stage_details (rendering) | 60%+ | 2-3 | Render details, warnings display |

**Total Unit Tests**: 10-14 tests (~150-200 lines)

### Integration Test Scenarios

1. **Modal Launch and Display**:
   - Given: Existing SPEC with no pipeline.toml
   - When: Execute /speckit.configure SPEC-947
   - Then: Modal displays with all 8 stages enabled (defaults)
   - Validates: Phase 4 Task 4.1 (command handler)

2. **Stage Toggle and Cost Update**:
   - Given: Modal open with all stages enabled
   - When: Toggle validate (Space key on row 5)
   - Then: Cost updates from $2.71 → $2.36 instantly
   - Validates: Phase 2 Task 2.1 (state toggle), Phase 3 Task 3.1 (cost display)

3. **Dependency Warning Display**:
   - Given: Modal open, implement stage enabled
   - When: Disable tasks stage
   - Then: Warning appears "Error: implement requires tasks"
   - Validates: Phase 3 Task 3.2 (warning display), SPEC-948 validation

4. **Save Configuration**:
   - Given: Modal open, stages toggled
   - When: Press 'q' (save)
   - Then: `docs/SPEC-947/pipeline.toml` written with correct enabled_stages
   - Validates: Phase 4 Task 4.1 (save logic)

5. **Cancel Without Saving**:
   - Given: Modal open, stages toggled
   - When: Press 'Esc' (cancel)
   - Then: No TOML file written, changes discarded
   - Validates: Phase 4 Task 4.1 (cancel logic)

6. **Load Existing Configuration**:
   - Given: `docs/SPEC-947/pipeline.toml` exists (validate, audit disabled)
   - When: Execute /speckit.configure SPEC-947
   - Then: Modal shows validate, audit unchecked (loads existing config)
   - Validates: SPEC-948 PipelineConfig::load(), Phase 2 state init

7. **--configure Flag Integration**:
   - Given: No existing pipeline.toml
   - When: Execute /speckit.auto SPEC-947 --configure
   - Then: Configurator launches first, after save → pipeline executes
   - Validates: Phase 4 Task 4.3 (flag integration)

**Total Integration Tests**: 7 tests (~200-250 lines)

### Manual TUI Validation Checklist

**Pre-Release Validation**:
- [ ] Modal renders centered overlay (80×70% terminal)
- [ ] All 8 stages listed with correct names
- [ ] Checkbox toggles with Space key
- [ ] Navigation works (↑/↓ keys)
- [ ] Cost updates in real-time (instant recalculation)
- [ ] Warnings appear when dependencies violated
- [ ] 'q' saves TOML file correctly
- [ ] 'Esc' cancels without writing
- [ ] Existing config loads correctly
- [ ] /speckit.auto --configure works end-to-end

---

## Migration & Rollback Plan

### Incremental Deployment

**Phase 2 Complete → Deploy**:
- Merge: Widget core (pipeline_configurator.rs)
- Validate: Widget state tests pass (6-8 tests)
- Checkpoint: State machine logic correct
- No user impact: Widget exists but not accessible yet

**Phase 3 Complete → Deploy**:
- Merge: Interactive components (stage_selector.rs, stage_details.rs)
- Validate: Interaction tests pass (4-6 tests)
- Checkpoint: Rendering works, cost updates in real-time
- Limited user impact: Widget functional but no command to launch

**Phase 4 Complete → Production**:
- Merge: Command integration (/speckit.configure)
- Validate: End-to-end tests pass (3-4 tests)
- Checkpoint: Full feature operational
- User Testing: Test with 2-3 real SPECs
- Production Release: Feature complete, announce to users

### Backward Compatibility

**Preserved**:
- All existing /speckit.* commands work unchanged
- No command syntax changes (new command is additive)
- Existing pipeline.toml files continue to work (SPEC-948 backward compatible)
- Users without pipeline.toml still execute all stages (defaults)

**Optional**:
- /speckit.configure is opt-in (users can ignore TUI configurator)
- CLI flags (--skip-*, --stages=) still work (alternative to TUI)
- Manual TOML editing still supported (TUI is convenience layer)

**Breaking Changes**: None (fully backward compatible)

### Rollback Strategy

**Rollback Trigger: Modal rendering issues**
- **Condition**: Terminal glitches, layout broken on certain sizes
- **Action**:
  1. Disable /speckit.configure command (comment out registration)
  2. Keep widgets in codebase (no harm)
  3. Fix rendering issues (test on various terminal sizes)
- **Recovery Time**: <1 hour (disable command) + bug fix time

**Rollback Trigger: State machine bugs**
- **Condition**: Toggle doesn't update cost, warnings incorrect
- **Action**:
  1. Revert Phase 2-3 changes (widget logic)
  2. Keep SPEC-948 pipeline_config.rs (backend still works)
  3. Fix state machine bug (unit test reproduction)
- **Recovery Time**: <1 hour (revert) + bug fix time

**Rollback Trigger: Command integration breaks TUI**
- **Condition**: /speckit.configure crashes TUI or freezes
- **Action**:
  1. Remove /speckit.configure from command registry
  2. Advise users to use CLI flags or manual TOML editing
  3. Fix command handler (async issues, event loop)
- **Recovery Time**: <30 minutes (remove command) + handler fix

**Rollback Procedure**:
```bash
# 1. Identify problematic phase
git log --oneline --grep="SPEC-947" | head -10

# 2. Revert specific phase commit
git revert <phase-commit-hash>

# 3. Rebuild
cd codex-rs && cargo build -p codex-tui

# 4. Test rollback
# (TUI should still work, /speckit.configure missing but other commands work)

# 5. Document rollback
echo "Rollback: $(date) - Phase X - Reason: <issue>" >> docs/SPEC-947-.../evidence/rollback.log
```

---

## Timeline & Milestones

### Week 3-4 (Days 1-8, 18-24 hours total)

**Day 1 (Phase 1: Verification, 0h)**:
- Verify SPEC-948 pipeline_config.rs provides required API
- Review API: load(), save(), validate(), is_enabled()
- **Deliverable**: API verification checklist complete
- No implementation (SPEC-948 already done)

**Days 2-4 (Phase 2: Widget Core, 8-10h)**:
- Tue AM: Create state machine structures (80-100 LOC)
- Tue PM: Implement event handling (60-80 LOC)
- Wed AM: Implement widget rendering (80-100 LOC)
- Wed PM: Write 6-8 widget state tests
- Thu AM: Run tests, fix state machine bugs
- Thu PM: Code review, commit Phase 2
- **Deliverable**: pipeline_configurator.rs (~300-400 LOC), tests passing
- **Validation**: `cargo test -p codex-tui pipeline_configurator`

**Days 5-7 (Phase 3: Interactive Components, 6-8h)**:
- Fri AM: Create stage selector widget (150-200 LOC)
- Fri PM: Create stage details widget (150-200 LOC)
- Sat AM: Create help bar rendering (40-60 LOC)
- Sat PM: Write 4-6 interaction tests
- Sun: Run tests, manual TUI validation, commit Phase 3
- **Deliverable**: stage_selector.rs + stage_details.rs (~340-460 LOC), tests passing
- **Validation**: Manual TUI testing (navigate, toggle, see cost update)

**Days 8-9 (Phase 4: Command Integration, 4-6h)**:
- Mon AM: Create /speckit.configure command handler (100-150 LOC)
- Mon PM: Register command, add --configure flag (+30 LOC)
- Tue AM: Write 3-4 end-to-end tests
- Tue PM: Run tests, manual testing, commit Phase 4
- **Deliverable**: commands/configure.rs (~130-180 LOC), tests passing
- **Validation**: `/speckit.configure SPEC-947`, `/speckit.auto SPEC-947 --configure`

**Milestone (End of Week 4)**: SPEC-947 implementation complete, full feature operational

---

## Risk Assessment & Mitigation

### Technical Risks

**Risk 1: TUI Rendering Complexity**
- **Severity**: Medium
- **Probability**: Medium (Ratatui learning curve)
- **Impact**: Modal doesn't render correctly, layout broken
- **Mitigation**:
  - Follow existing quality_gate_modal.rs pattern (304 LOC proven working)
  - Prototype in isolation before full integration (2-hour spike)
  - Test on multiple terminal sizes (80×24, 120×40, 200×50)
- **Contingency**: Simplify UI (remove right pane, keep checkbox list only)

**Risk 2: State Machine Bugs**
- **Severity**: Medium
- **Probability**: Low (state machine is straightforward)
- **Impact**: Toggle doesn't update cost, warnings incorrect
- **Mitigation**:
  - Comprehensive unit tests for state transitions (6-8 tests)
  - Immutable state updates (create new config on toggle)
  - Validation after every toggle
- **Contingency**: Add debug logging, fix state sync issues

**Risk 3: Event Loop Integration**
- **Severity**: High
- **Probability**: Low (existing modal patterns to follow)
- **Impact**: Modal freezes TUI, keyboard input unresponsive
- **Mitigation**:
  - Use existing TUI event loop pattern (crossterm events)
  - Non-blocking input handling
  - Timeout on event polling (100ms)
- **Contingency**: Revert to CLI-only configuration (SPEC-948 still works)

### Integration Risks

**Risk 1: SPEC-948 API Mismatch**
- **Severity**: High
- **Probability**: Very Low (SPEC-947 designed around SPEC-948 API)
- **Impact**: TUI can't load/save configurations
- **Mitigation**:
  - Phase 1 verification step (API checklist)
  - If mismatch: Extend pipeline_config.rs before Phase 2
  - Integration tests validate load/save round-trip
- **Contingency**: Modify SPEC-948 API if needed (backward compatible extension)

**Risk 2: Terminal Size Variations**
- **Severity**: Low
- **Probability**: High (users have different terminal sizes)
- **Impact**: Modal too large or too small, text truncated
- **Mitigation**:
  - Use percentage-based layout (80×70% of available space)
  - Test on small (80×24) and large (200×50) terminals
  - Handle resize events (re-render on terminal size change)
- **Contingency**: Add min/max size constraints (require 80×24 minimum)

**Risk 3: User Confusion (Too Many Options)**
- **Severity**: Low
- **Probability**: Medium (8 stages × indicators × warnings = complexity)
- **Impact**: Users don't understand what to configure
- **Mitigation**:
  - Clear visual hierarchy (enabled stages bold, disabled gray)
  - Help text explains key bindings
  - PIPELINE_CONFIGURATION_GUIDE.md (SPEC-948) provides context
  - Workflow examples show common patterns
- **Contingency**: Add "preset" buttons (rapid prototype, docs-only, etc.)

---

## Success Criteria

### Phase-Level Criteria

**Phase 1 Success**:
1. API verification checklist complete
2. SPEC-948 pipeline_config.rs provides all required methods
3. No implementation needed (SPEC-948 already done)

**Phase 2 Success**:
1. State machine compiles without errors
2. Event handling covers all key codes (↑/↓/Space/Enter/q/Esc)
3. Widget renders centered overlay (80×70% of terminal)
4. Toggle updates state correctly (stage_states reflects changes)
5. Validation runs after every toggle
6. 6-8 widget state tests passing

**Phase 3 Success**:
1. Stage selector renders checkbox list with indicators
2. Stage details shows selected stage info (cost, duration, agents, dependencies)
3. Help bar displays key bindings
4. Real-time updates work (toggle → cost recalculates → warnings update)
5. 4-6 interaction tests passing

**Phase 4 Success**:
1. /speckit.configure SPEC-ID launches modal
2. Modal displays current config (loaded from TOML if exists)
3. Save writes `docs/SPEC-{id}/pipeline.toml` correctly
4. Cancel discards changes without writing
5. --configure flag works with /speckit.auto
6. 3-4 end-to-end tests passing

### Overall SPEC Criteria

1. **Phases Complete**: All 4 phases 100% complete
2. **Tests Passing**: 100% pass rate maintained (634+ existing [includes SPEC-948's 24-30 tests] + 17-21 new = 651-655 total)
3. **Backward Compatible**: Existing SPECs execute unchanged
4. **Documentation Complete**: Inline help text, CLAUDE.md update
5. **No Regressions**: All existing /speckit.* commands work
6. **User Acceptance**: 2-3 real SPECs configured and executed successfully
7. **Evidence Captured**: Modal screenshots (optional), test execution logs

---

## Documentation Requirements

### User-Facing Documentation

1. **Inline Help Text** (in modal):
   - Key bindings (↑↓/Space/Enter/q/Esc)
   - Indicators explanation ([$] high cost, [🔒] quality gate)
   - Warnings explanation (⚠ = warning, Error: = blocker)

2. **Command Reference Update** (`CLAUDE.md`):
   - Document /speckit.configure SPEC-ID
   - Document --configure flag for /speckit.auto
   - Add example workflow: "Interactive configuration → execute"
   - Link to PIPELINE_CONFIGURATION_GUIDE.md (SPEC-948)

3. **TUI Usage Guide** (optional, separate doc):
   - Screenshot/mockup of modal
   - Step-by-step walkthrough (launch, toggle, save)
   - Common workflows demonstration (4 patterns)

### Developer Documentation

1. **Inline Code Comments**:
   - State machine transitions explained
   - Rendering logic (layout calculations)
   - Event handling flow (key → action → state update → re-render)

2. **Widget API Documentation** (rustdoc):
   - PipelineConfiguratorState public methods
   - PipelineConfiguratorWidget::render() usage
   - Integration with existing TUI app (AppMode switching)

3. **CHANGELOG Entry**:
   - SPEC-947: Pipeline UI Configurator
   - Added: /speckit.configure command (interactive TUI modal)
   - Added: --configure flag for /speckit.auto
   - Feature: Visual stage selection with real-time cost/time estimates
   - UX: Keyboard-driven navigation, dependency warnings, help text

---

**SPEC-947-IMPL Status**: Ready for implementation
**Estimated Total Effort**: 24-32 hours (1.5-2 weeks)
**Depends On**: SPEC-948-IMPL (pipeline_config.rs - HARD DEPENDENCY)
**Final Deliverable**: User-facing interactive pipeline configurator
