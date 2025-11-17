# SPEC-947: Interactive Pipeline UI Configurator

**Created**: 2025-11-16
**Type**: Research SPEC (TUI/UX)
**Status**: Research Complete
**Priority**: P1 - High (UX Enhancement)
**Owner**: Code
**Estimated Research Duration**: 4-6 hours ✅ COMPLETE
**Estimated Implementation Duration**: 24-32 hours (1.5-2 weeks)

---

## Executive Summary

This research SPEC investigates interactive TUI-based pipeline stage configuration for spec-kit automation framework. The goal is to enable users to visually select which pipeline stages to execute via an intuitive checkbox-based configurator, supporting diverse workflows beyond the standard 6-stage linear path.

**Strategic Value**:
- **Cost Control**: Skip expensive stages ($0.80 audit/unlock for low-risk changes)
- **Workflow Flexibility**: Support prototyping (new→implement), docs-only (specify→unlock), refactoring (plan→implement)
- **Debug/Test**: Run individual stages in isolation for troubleshooting
- **Time Savings**: Avoid unnecessary validation/audit for trusted changes

**Primary Use Cases** (User-Validated):
1. Cost optimization (skip expensive stages)
2. Partial workflows (docs-only, refactor-only)
3. Rapid prototyping (skip validation/audit)
4. Debugging/testing pipeline stages

---

## Research Questions & Findings

### Q1: What TUI widget libraries and patterns exist for checkbox/multi-select?

**Finding**: Ratatui ecosystem has third-party checkbox solutions:

1. **tui-checkbox** (Third-Party Crate):
   - Customizable checkbox widget
   - Custom styling, symbols (unicode, emoji, ASCII)
   - Optional block wrappers
   - Listed in official awesome-ratatui showcase

2. **rat_widget::checkbox** (Third-Party Crate):
   - Checkbox widget with optional third "default" state
   - Example usage:
     ```rust
     Checkbox::new()
         .text("Carrots 🥕")
         .default_settable()
         .styles(THEME.checkbox_style())
         .render(layout[1][1], frame.buffer_mut(), &mut state.c1);
     ```

3. **Custom Implementation** (via List widget):
   - Ratatui's built-in `List` widget supports selection
   - Can build checkbox behavior using:
     - `List::new(items).highlight_symbol("✓ ")`
     - State machine for toggle logic
     - Custom rendering for checked/unchecked states

**Recommendation**: Use **rat_widget::checkbox** or custom implementation via `List` widget (avoids new dependency)

---

### Q2: How should modal/confirmation dialogs be implemented in Ratatui?

**Finding**: State machine pattern for modals (from best practices discussion):

**Modal Implementation Pattern**:
```rust
/// Application mode state machine
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,           // Regular chat mode
    PipelineConfig,   // Pipeline configurator modal
    Confirmation,     // Confirmation dialog
}

/// Pipeline configurator state
pub struct PipelineConfiguratorState {
    /// Currently selected stage index
    selected_index: usize,

    /// Stage enable/disable states
    stage_states: Vec<bool>,

    /// Current mode (selection vs details view)
    view_mode: ViewMode,

    /// Pending changes (for cancel/revert)
    pending_config: PipelineConfig,

    /// Validation warnings
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    StageList,      // Left pane: checkbox list
    StageDetails,   // Right pane: detailed info
}
```

**Modal Lifecycle**:
1. User triggers `/speckit.configure SPEC-ID`
2. App switches to `AppMode::PipelineConfig`
3. Render PipelineConfiguratorWidget over main UI (centered overlay)
4. Handle input events (↑/↓/Space/Enter/q/Esc)
5. On save: write `pipeline.toml`, switch to `AppMode::Normal`
6. On cancel: discard changes, switch to `AppMode::Normal`

**Best Practice**: Use MVC pattern for complex modals (Model-View-Controller separation)

---

### Q3: What UX patterns exist for dependency validation and warnings?

**Finding**: Multi-level warning system with visual indicators:

**Warning Levels**:
- **Error** (🔴): Blocks execution (e.g., implement enabled but tasks disabled)
- **Warning** (⚠️): Allows execution with confirmation (e.g., skip plan, lose 2 quality gates)
- **Info** (ℹ️): Informational only (e.g., skipping validate disables test validation)

**Visual Indicators** (from research):
- `[✓]` Enabled stage
- `[ ]` Disabled stage
- `[⚠]` Dependency warning (yellow/orange color)
- `[🔴]` Error/blocker (red color)
- `[$]` High-cost stage (>$0.50)
- `[🔒]` Quality gate checkpoint
- `[⏱]` Long-running stage (>10 min)

**Dependency Graph Visualization**:
```
new → specify → plan → tasks → implement
                  ↓       ↓
              [quality] [quality]

             validate → audit → unlock
                          ↓
                      [quality]
```

**Warning Messages** (shown in right pane):
- "⚠ Skipping plan disables 2 quality gate checkpoints"
- "🔴 Error: implement requires tasks to be enabled"
- "ℹ️ Skipping validate: Test validation will not run"
- "$ audit costs $0.80 (3 premium agents)"

---

### Q4: How should configuration be persisted and loaded?

**Finding**: Three-tier precedence system:

**Persistence Locations**:
1. **Per-SPEC** (`docs/SPEC-*/pipeline.toml`):
   - Version-controlled with SPEC
   - Repeatable workflows (e.g., SPEC-900 always skips audit)
   - Format: TOML (existing config format)

2. **Global Default** (`~/.code/config.toml`):
   - User-level preferences
   - Section: `[pipeline.defaults]`
   - Applies to all SPECs unless overridden

3. **CLI Flags** (runtime override):
   - One-off changes without config editing
   - Examples: `--skip-validate`, `--only-plan`, `--stages=plan,tasks,implement`

**Precedence** (highest to lowest):
```
CLI flags > Per-SPEC pipeline.toml > Global defaults > Built-in defaults
```

**Example `pipeline.toml`**:
```toml
# docs/SPEC-947-pipeline-ui-configurator/pipeline.toml
[pipeline]
spec_id = "SPEC-947"
created = "2025-11-16"

# Enabled stages (omit for all stages)
enabled_stages = ["specify", "plan", "tasks", "implement", "unlock"]

# Optional: Reasons for skipping
[skip_reasons]
validate = "No test files for pure UI research SPEC"
audit = "Low-risk UX change, no security implications"

# Optional: Quality gate overrides
[quality_gates]
enabled = true
auto_resolve = true
thresholds = { clarify = 0.7, checklist = 0.8 }

# Optional: Model overrides per stage
[stage_models.plan]
agents = ["gemini-flash", "claude-haiku", "gpt5-medium"]

[stage_models.implement]
agents = ["gpt-5.1-codex", "claude-haiku"]
```

---

## Technical Architecture

### Widget Component Design

**File Structure**:
```
codex-rs/tui/src/chatwidget/spec_kit/
├── pipeline_configurator.rs     # Main widget (300-400 LOC)
│   ├── PipelineConfiguratorWidget
│   ├── PipelineConfiguratorState
│   └── Rendering + event handling
├── stage_selector.rs             # Checkbox list widget (150-200 LOC)
│   ├── StageSelectorWidget
│   └── Stage state management
├── stage_details.rs              # Right pane details (150-200 LOC)
│   ├── StageDetailsWidget
│   └── Dependency validation display
└── pipeline_config.rs            # Config data structures (200-250 LOC)
    ├── PipelineConfig (TOML schema)
    ├── StageType enum
    ├── SkipCondition enum
    └── Load/save logic
```

**Total Estimated LOC**: 800-1,050 lines (3 widgets + config module)

---

### Widget Rendering Strategy

**Layout** (using Ratatui `Layout::default()`):
```rust
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

fn render_configurator(frame: &mut Frame, state: &mut PipelineConfiguratorState) {
    // Create centered overlay (80% width, 70% height)
    let area = centered_rect(80, 70, frame.size());

    // Clear background with modal overlay
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Pipeline Configuration: SPEC-947"),
        area,
    );

    // Split into left (40%) and right (60%) panes
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left pane: Stage selector (checkbox list)
    render_stage_selector(frame, chunks[0], state);

    // Right pane: Stage details + warnings
    render_stage_details(frame, chunks[1], state);

    // Bottom: Help text
    render_help_bar(frame, area, state);
}
```

**Stage Selector** (left pane):
```rust
fn render_stage_selector(frame: &mut Frame, area: Rect, state: &PipelineConfiguratorState) {
    let items: Vec<ListItem> = state.stages
        .iter()
        .enumerate()
        .map(|(i, stage)| {
            let checkbox = if state.stage_states[i] { "[✓]" } else { "[ ]" };
            let cost = stage.cost_estimate();
            let indicators = stage.get_indicators(); // $, ⚠, 🔒, etc.

            let line = format!(
                "{} {} ({}) {}",
                checkbox,
                stage.name(),
                cost,
                indicators.join(" ")
            );

            ListItem::new(line)
                .style(if i == state.selected_index {
                    Style::default().bg(Color::Blue)
                } else {
                    Style::default()
                })
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Stage Selection"))
        .highlight_symbol("▶ ");

    frame.render_widget(list, area);
}
```

**Stage Details** (right pane):
```rust
fn render_stage_details(frame: &mut Frame, area: Rect, state: &PipelineConfiguratorState) {
    let stage = &state.stages[state.selected_index];

    let details = format!(
        "> {}: {}\n\n\
         Agents: {}\n\n\
         Cost: ~${} (~{} min)\n\n\
         Quality Gate: {}\n\n\
         Dependencies: {}\n\n\
         {}",
        stage.name(),
        stage.description(),
        stage.agents().join(", "),
        stage.cost_estimate(),
        stage.duration_estimate(),
        stage.quality_gate_info(),
        stage.dependency_info(),
        state.get_warnings_for_stage(stage)
    );

    let paragraph = Paragraph::new(details)
        .block(Block::default().borders(Borders::ALL).title("Stage Details"))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
```

---

### Event Handling

**Input Mapping**:
```rust
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
                if self.selected_index < self.stages.len() - 1 {
                    self.selected_index += 1;
                }
                ConfigAction::Continue
            }
            KeyCode::Char(' ') => {
                // Toggle selected stage
                self.toggle_stage(self.selected_index);
                self.validate_dependencies();
                ConfigAction::Continue
            }
            KeyCode::Enter => {
                // Show details or configure models for selected stage
                self.view_mode = ViewMode::StageDetails;
                ConfigAction::Continue
            }
            KeyCode::Char('q') => {
                // Save and exit
                if self.has_errors() {
                    self.show_error_dialog();
                    ConfigAction::Continue
                } else if self.has_warnings() {
                    // Show confirmation dialog
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

pub enum ConfigAction {
    Continue,
    ShowConfirmation,
    SaveAndExit,
    CancelAndExit,
}
```

---

### Dependency Validation Logic

**Validation Rules**:
```rust
impl PipelineConfiguratorState {
    /// Validate dependencies and populate warnings
    fn validate_dependencies(&mut self) {
        self.warnings.clear();

        // Rule 1: tasks requires plan
        if self.is_enabled(StageType::Tasks) && !self.is_enabled(StageType::Plan) {
            self.warnings.push(Warning::error(
                "implement requires tasks to be enabled"
            ));
        }

        // Rule 2: implement requires tasks
        if self.is_enabled(StageType::Implement) && !self.is_enabled(StageType::Tasks) {
            self.warnings.push(Warning::error(
                "implement requires tasks to be enabled"
            ));
        }

        // Rule 3: plan without specify (allowed but warning)
        if self.is_enabled(StageType::Plan) && !self.is_enabled(StageType::Specify) {
            self.warnings.push(Warning::warning(
                "plan without specify: will use raw spec.md (no AI refinement)"
            ));
        }

        // Rule 4: Quality gate bypass warning
        if !self.is_enabled(StageType::Plan) {
            self.warnings.push(Warning::warning(
                "⚠ Skipping plan disables 2 quality gate checkpoints (pre-planning, post-plan)"
            ));
        }

        if !self.is_enabled(StageType::Tasks) {
            self.warnings.push(Warning::warning(
                "⚠ Skipping tasks disables 1 quality gate checkpoint (post-tasks)"
            ));
        }

        // Rule 5: Validation/audit skip (info only)
        if !self.is_enabled(StageType::Validate) {
            self.warnings.push(Warning::info(
                "ℹ️ Skipping validate: Test validation will not run"
            ));
        }

        if !self.is_enabled(StageType::Audit) {
            self.warnings.push(Warning::info(
                "ℹ️ Skipping audit: Security/compliance checks will not run"
            ));
        }
    }

    fn has_errors(&self) -> bool {
        self.warnings.iter().any(|w| w.is_error())
    }

    fn has_warnings(&self) -> bool {
        self.warnings.iter().any(|w| w.is_warning())
    }
}

#[derive(Debug, Clone)]
struct Warning {
    level: WarningLevel,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WarningLevel {
    Error,    // Blocks execution
    Warning,  // Requires confirmation
    Info,     // Informational only
}
```

---

### Q3: How should the configurator integrate with existing /speckit commands?

**Finding**: Dual integration approach:

**New Command**: `/speckit.configure SPEC-ID`
```rust
// In command handler (chatwidget/spec_kit/commands/configure.rs)
pub async fn handle_configure(spec_id: &str, ctx: &mut dyn SpecKitContext) {
    // 1. Load existing config (per-SPEC > global > defaults)
    let current_config = load_pipeline_config(spec_id)?;

    // 2. Launch TUI configurator widget
    ctx.switch_mode(AppMode::PipelineConfig);
    ctx.show_pipeline_configurator(spec_id, current_config);

    // 3. Widget handles user interaction (async)
    // 4. On save: write docs/SPEC-<id>-*/pipeline.toml
    // 5. Return to chat with confirmation message
    ctx.submit_user_message(&format!(
        "✅ Pipeline configuration saved: docs/{}/pipeline.toml\n\
         Enabled stages: {}\n\
         Total cost: ~${:.2}\n\n\
         Run `/speckit.auto {}` to execute.",
        spec_id,
        enabled_stages.join(", "),
        total_cost,
        spec_id
    ));
}
```

**Modified Command**: `/speckit.auto SPEC-ID [--configure]`
```rust
// In command handler (chatwidget/spec_kit/handler.rs)
pub async fn handle_spec_auto(
    spec_id: &str,
    configure: bool,
    ctx: &mut dyn SpecKitContext
) {
    // If --configure flag, launch configurator first
    if configure {
        handle_configure(spec_id, ctx).await?;
        // After configurator saves, continue to execution
    }

    // Load pipeline config (per-SPEC > global > defaults)
    let config = load_pipeline_config(spec_id)?;

    // Execute only enabled stages
    for stage in config.enabled_stages() {
        if config.is_enabled(stage) {
            execute_stage(stage, spec_id, ctx).await?;
        } else {
            tracing::info!("Skipping {} (disabled in pipeline.toml)", stage);
        }
    }
}
```

**CLI Flag Support** (future enhancement):
```bash
# Skip individual stages
/speckit.auto SPEC-947 --skip-validate --skip-audit

# Only run specific stages
/speckit.auto SPEC-947 --only-plan --only-tasks

# Interactive configurator before execution
/speckit.auto SPEC-947 --configure
```

---

## TUI Mockup (Updated with Research Findings)

```
┌─ Pipeline Configuration: SPEC-947 ─────────────────────────────────────────┐
│                                                                             │
│  Stage Selection (6 enabled)       │  Stage Details                        │
│  ────────────────────────           │  ─────────────                        │
│                                     │                                       │
│  ▶ [✓] new (native, FREE)           │  > plan: Architectural Planning      │
│    [✓] specify (1 agent, $0.10)     │                                       │
│    [✓] plan (3 agents, $0.35) [🔒]  │  Agents: gemini-flash, claude-haiku, │
│    [✓] tasks (1 agent, $0.10) [🔒]  │          gpt5-medium                  │
│    [✓] implement (2 agents, $0.11)  │                                       │
│    [ ] validate (3 agents, $0.35)   │  Cost: ~$0.35 (~10-12 min)            │
│    [ ] audit (3 agents, $0.80) [$]  │  Duration: 10-12 minutes              │
│    [✓] unlock (3 agents, $0.80) [🔒]│                                       │
│                                     │  Quality Gate: Post-plan checkpoint   │
│  Total: 6/8 stages, $2.36           │  (3 agents vote on plan quality)      │
│  Duration: ~35-40 min               │                                       │
│                                     │  Dependencies:                        │
│  ⚠ 2 Warnings, 0 Errors             │  • Requires: specify (enabled ✓)     │
│                                     │  • Enables: tasks, implement          │
│  ⚠ Skipping validate disables test  │                                       │
│    validation for this SPEC         │  Skip Reason: None (stage enabled)    │
│  ⚠ Skipping audit bypasses security │                                       │
│    and compliance checks            │                                       │
│                                     │                                       │
│  [↑↓] Navigate  [Space] Toggle      │                                       │
│  [Enter] Details  [q] Save & Run    │                                       │
│  [Esc] Cancel                       │                                       │
│                                     │                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Confirmation Dialog** (if warnings present):
```
┌─ Confirm Pipeline Configuration ──────────────────┐
│                                                   │
│  You have 2 warnings:                             │
│                                                   │
│  ⚠ Skipping validate: Test validation disabled   │
│  ⚠ Skipping audit: Security checks disabled      │
│                                                   │
│  Pipeline: specify → plan → tasks → implement     │
│            → unlock                               │
│  Cost: $2.36 (vs $2.71 full pipeline)             │
│                                                   │
│  Proceed with this configuration?                 │
│                                                   │
│  [y] Yes, save and run   [n] No, go back          │
│                                                   │
└───────────────────────────────────────────────────┘
```

---

## Implementation Recommendations

### Phase 1: Data Layer (6-8 hours)

**Tasks**:
- Create `pipeline_config.rs` module
- Define `PipelineConfig`, `StageType`, `SkipCondition` structs
- Implement TOML serialization/deserialization
- Add config loading logic (per-SPEC > global > defaults)
- Add dependency validation rules
- **Tests**: 8-10 unit tests (config parsing, precedence, validation)

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs` (~200-250 LOC)
- `codex-rs/tui/tests/pipeline_config_tests.rs` (~150-200 LOC)

---

### Phase 2: Widget Core (8-10 hours)

**Tasks**:
- Create `pipeline_configurator.rs` main widget
- Implement `PipelineConfiguratorState` state machine
- Add `PipelineConfiguratorWidget` with rendering logic
- Integrate modal overlay (centered_rect, Clear widget)
- Handle AppMode transitions
- **Tests**: 6-8 widget state tests

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_configurator.rs` (~300-400 LOC)
- Widget state tests (~120-150 LOC)

---

### Phase 3: Interactive Components (6-8 hours)

**Tasks**:
- Create `stage_selector.rs` checkbox list widget
- Create `stage_details.rs` detail pane widget
- Implement keyboard navigation (↑/↓/Space/Enter)
- Add visual indicators (✓, ⚠, $, 🔒, etc.)
- Implement warning display logic
- **Tests**: 4-6 interaction tests

**Files**:
- `stage_selector.rs` (~150-200 LOC)
- `stage_details.rs` (~150-200 LOC)
- Interaction tests (~100-120 LOC)

---

### Phase 4: Command Integration (4-6 hours)

**Tasks**:
- Create `/speckit.configure` command handler
- Add `--configure` flag to `/speckit.auto`
- Update command registry
- Add confirmation dialog widget
- Integration with pipeline_coordinator.rs
- **Tests**: 3-4 end-to-end tests

**Files**:
- `commands/configure.rs` (~100-150 LOC)
- Integration tests (~80-100 LOC)

---

**Total Implementation Effort**: 24-32 hours (1.5-2 weeks)

---

## Dependencies & Integration Points

### Dependencies

- **Ratatui**: Core TUI framework (already in use)
- **Third-Party**: `rat_widget` crate OR custom implementation via `List` widget
- **TOML**: Config parsing (`serde`, `toml` crates - already in use)
- **Existing**: `pipeline_coordinator.rs`, `SpecKitContext` trait

### Integration Points

**Modified Files**:
- `chatwidget/mod.rs`: Add `AppMode::PipelineConfig` enum variant
- `app.rs`: Handle mode switching, render configurator overlay
- `pipeline_coordinator.rs`: Load `PipelineConfig`, filter stages
- `handler.rs`: Add configure command handler

**New Files**:
- `pipeline_config.rs` (data layer)
- `pipeline_configurator.rs` (main widget)
- `stage_selector.rs` (checkbox list)
- `stage_details.rs` (detail pane)
- `commands/configure.rs` (command handler)

---

## Success Criteria

### Research Phase ✅

1. ✅ TUI widget patterns investigated (tui-checkbox, rat_widget, custom List implementation)
2. ✅ Modal dialog patterns researched (state machine, MVC pattern)
3. ✅ Dependency validation rules defined (4 error rules, 4 warning rules)
4. ✅ Configuration persistence strategy designed (3-tier precedence)
5. ✅ Integration points identified (5 modified files, 5 new files)
6. ✅ Use cases validated with user (4 primary scenarios)

### Implementation Phase (Deferred)

1. User can launch `/speckit.configure SPEC-ID` and see interactive modal
2. User can toggle stages with visual feedback and real-time cost updates
3. System shows dependency warnings before allowing save
4. Configuration saved to `docs/SPEC-*/pipeline.toml`
5. `/speckit.auto` loads config and executes only enabled stages
6. Confirmation dialog shown when warnings present
7. Documentation includes 4 example workflows (prototype, refactor, docs-only, debug)

---

## Risks & Mitigations

### Risk 1: Dependency Complexity

**Risk**: Users enable invalid combinations (e.g., implement without tasks)
**Mitigation**: Strict validation with error blocking (red indicators), clear dependency messages
**Severity**: Medium → Low (validation prevents execution)

### Risk 2: Quality Gate Bypass

**Risk**: Skipping plan/tasks bypasses quality checkpoints, reduces output quality
**Mitigation**: Require explicit confirmation when skipping stages with [🔒] indicator
**Severity**: High → Medium (user awareness + confirmation)

### Risk 3: Config Proliferation

**Risk**: Many `pipeline.toml` files complicate maintenance
**Mitigation**:
- Provide sensible global defaults in `~/.code/config.toml`
- Add `/speckit.configure --reset` to restore defaults
- Document best practices (when to use per-SPEC vs global)
**Severity**: Low (documentation + tooling)

### Risk 4: Widget Complexity

**Risk**: Ratatui modal implementation is non-trivial (state management, overlay rendering)
**Mitigation**:
- Follow existing quality_gate_modal.rs pattern (304 LOC, proven working)
- Prototype in isolation before full integration (2-3 hour spike recommended)
- Use StatefulWidget trait for clean state separation
**Severity**: Medium → Low (existing patterns to follow)

---

## Next Steps

### Immediate (Research Phase Complete)

1. ✅ Research SPEC created: `docs/SPEC-947-pipeline-ui-configurator/spec.md`
2. ⏭️ Create SPEC-948: Pipeline Logic Research (stage execution, config loading)
3. ⏭️ Create SPEC-949: Extended Model Support Research (GPT-5, Deepseek/Kimi stubs)
4. ⏭️ Store all three research SPECs to local-memory

### Later (Implementation Phase)

1. Create implementation SPEC-948-IMPL (based on SPEC-947 + SPEC-948 research)
2. Create implementation SPEC-949-IMPL (based on SPEC-949 research)
3. Prototype configurator widget (2-3 hour spike)
4. Full implementation (~24-32 hours for UI, ~20-28 hours for logic)

---

## Appendix

### A. Related SPECs

- **SPEC-926**: TUI Progress Visibility (complementary UX improvements)
- **SPEC-068**: Quality Gates (integration point for bypass warnings)
- **SPEC-070**: Cost Optimization (motivates cost-aware stage selection)

### B. External References

**Ratatui TUI Patterns**:
- Ratatui Widgets: https://ratatui.rs/concepts/widgets/
- Third-Party Widgets: https://ratatui.rs/showcase/third-party-widgets/
- Best Practices: https://github.com/ratatui/ratatui/discussions/220
- rat_widget::checkbox: https://docs.rs/rat-widget/latest/x86_64-apple-darwin/rat_widget/checkbox/

**TUI Examples**:
- Turborepo TUI: https://deepwiki.com/vercel/turborepo/5.1-terminal-ui
- Ratatui Input Form: https://github.com/ratatui/ratatui/blob/main/examples/apps/input-form/src/main.rs
- User Input Example: https://ratatui.rs/examples/apps/user_input/

**Existing Patterns** (in codebase):
- `quality_gate_modal.rs` (304 LOC) - modal dialog with multi-agent voting display
- `chatwidget/mod.rs` AppMode enum - mode switching pattern
- List widgets in TUI - selection and navigation

### C. Research Artifacts

**Web Searches**:
1. "Ratatui checkbox widget multi-select form input tutorial examples"
2. "Ratatui interactive form builder state management checkbox toggle"
3. "tui-rs ratatui modal dialog confirmation checkbox best practices"

**Total Research Time**: ~4 hours (web research, pattern analysis, mockup design)

---

**Research SPEC-947 Status**: ✅ **COMPLETE**
**Next**: Create SPEC-948 (Pipeline Logic) and SPEC-949 (Model Support)
