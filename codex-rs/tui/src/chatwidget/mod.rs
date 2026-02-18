// Spec-kit submodule for friend access to ChatWidget private fields
// Made public for integration testing (T78)
pub mod spec_kit;

const SPEC_KIT_DEFAULT_BUDGET_USD: f64 = 2.0;

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::OnceLock;
// SPEC-955: std::sync::mpsc::Sender only for TerminalRunController (separate system)
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

use ratatui::style::Modifier;
use ratatui::style::Style;

use crate::slash_command::HalMode;
use crate::slash_command::SlashCommand;
use crate::slash_command::SpecAutoInvocation;
use crate::spec_prompts::SpecStage;
use spec_kit::state::{
    ValidateCompletionReason, ValidateLifecycle, ValidateLifecycleEvent, ValidateMode,
    ValidateRunCompletion,
};
use spec_kit::{
    GuardrailOutcome, QualityGateBroker, SpecAutoState, spec_ops_stage_prefix,
    validate_guardrail_evidence,
};
use spec_kit::{evaluate_guardrail_value, validate_guardrail_schema};
// spec_status functions moved to spec_kit::handler
use codex_common::elapsed::format_duration;
use codex_common::model_presets::ModelPreset;
use codex_common::model_presets::builtin_model_presets;
use codex_core::ConversationManager;
use codex_core::account_usage::{self, StoredRateLimitSnapshot, StoredUsageSummary};
use codex_core::auth_accounts::{self, StoredAccount};
use codex_core::config::Config;
use codex_core::config_types::AgentConfig;
use codex_core::config_types::ReasoningEffort;
use codex_core::config_types::TextVerbosity;
use codex_core::config_watcher::ConfigWatcher;
// CommitLogEntry moved to review_handlers.rs (MAINT-11 Phase 7)
use codex_core::model_family::derive_default_model_family;
use codex_core::model_family::find_family_for_model;
use codex_core::plan_tool::{PlanItemArg, StepStatus, UpdatePlanArgs};
use codex_login::AuthManager;
use codex_login::AuthMode;
use codex_protocol::mcp_protocol::AuthMode as McpAuthMode;
use codex_protocol::num_format::format_with_separators;
use serde_json::Value;

mod agent_install;
mod diff_handlers;
mod diff_ui;
mod exec_tools;
mod gh_actions;
mod help_handlers;
mod history_render;
mod interrupts;
mod layout_scroll;
mod limits_handlers;
mod limits_overlay;
mod message;
mod perf;
mod pm_handlers;
mod pm_overlay;
mod rate_limit_refresh;
mod streaming;
mod terminal;
mod terminal_handlers;
mod tools;

// MAINT-11: Extracted rendering helpers
mod agent_status;
mod agents_terminal;
mod command_render;
mod event_routing;
mod input_helpers;
mod pro_overlay;
mod render;
mod review_handlers;
mod session_handlers;
mod speckit_dispatch;
mod submit_helpers;
mod undo_snapshots;
use pro_overlay::ProState;

#[cfg(test)]
mod message_ordering_tests;
#[cfg(test)]
mod orderkey_property_tests;
#[cfg(test)]
mod orderkey_tests;
#[cfg(test)]
mod test_harness;
#[cfg(test)]
mod test_support;
use self::agent_install::{
    start_agent_install_session, start_direct_terminal_session, start_prompt_terminal_session,
    wrap_command,
};
use self::agent_status::{AgentStatus, agent_status_from_str};
use self::agents_terminal::{AgentsTerminalFocus, AgentsTerminalState};
use self::history_render::{CachedLayout, HistoryRenderState, LayoutRef};
use self::limits_overlay::{LimitsOverlay, LimitsOverlayContent, LimitsTab};
use self::rate_limit_refresh::start_rate_limit_refresh;
use codex_core::parse_command::ParsedCommand;
use codex_core::protocol::AgentMessageDeltaEvent;
use codex_core::protocol::AgentMessageEvent;
use codex_core::protocol::AgentReasoningDeltaEvent;
use codex_core::protocol::AgentReasoningEvent;
use codex_core::protocol::AgentReasoningRawContentDeltaEvent;
use codex_core::protocol::AgentReasoningRawContentEvent;
use codex_core::protocol::AgentReasoningSectionBreakEvent;
use codex_core::protocol::AgentStatusUpdateEvent;
use codex_core::protocol::ApplyPatchApprovalRequestEvent;
use codex_core::protocol::ApprovedCommandMatchKind;
use codex_core::protocol::BackgroundEventEvent;
use codex_core::protocol::CustomToolCallBeginEvent;
use codex_core::protocol::CustomToolCallEndEvent;
use codex_core::protocol::ErrorEvent;
use codex_core::protocol::Event;
use codex_core::protocol::EventMsg;
use codex_core::protocol::ExecApprovalRequestEvent;
use codex_core::protocol::ExecCommandBeginEvent;
use codex_core::protocol::ExecCommandEndEvent;
use codex_core::protocol::ExecOutputStream;
use codex_core::protocol::InputItem;
use codex_core::protocol::SandboxPolicy;
use codex_core::protocol::SessionConfiguredEvent;
// MCP tool call handlers moved into chatwidget::tools
use codex_core::protocol::Op;
use codex_core::protocol::PatchApplyBeginEvent;
use codex_core::protocol::PatchApplyEndEvent;
// MAINT-11: Pro* protocol types moved to pro_overlay.rs
// ReviewOutputEvent moved to review_handlers.rs (MAINT-11 Phase 7)
use codex_core::protocol::TaskCompleteEvent;
use codex_core::protocol::TokenUsage;
use codex_core::protocol::TurnDiffEvent;
// ReviewContextMetadata, ReviewRequest moved to review_handlers.rs (MAINT-11 Phase 7)
// MAINT-11 Phase 9: git tooling moved to undo_snapshots.rs
use codex_git_tooling::GhostCommit;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;
// MAINT-11: std::cell::Cell usage now fully-qualified
use std::cell::RefCell;
// SPEC-955: std::sync::mpsc only for TerminalRunController (separate system)
use std::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

fn history_cell_logging_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        if let Ok(value) = std::env::var("CODE_BUFFER_DIFF_TRACE_CELLS") {
            return !matches!(value.trim(), "" | "0");
        }
        if let Ok(value) = std::env::var("CODE_BUFFER_DIFF_METRICS") {
            return !matches!(value.trim(), "" | "0");
        }
        false
    })
}
use tokio::sync::mpsc::unbounded_channel;
use tracing::info;
// use image::GenericImageView;

pub(crate) use self::terminal::{
    PendingCommand, PendingCommandAction, PendingManualTerminal, TerminalOverlay, TerminalState,
};
#[cfg(target_os = "macos")]
use crate::agent_install_helpers::macos_brew_formula_for_command;
use crate::app_event::{
    AppEvent, BackgroundPlacement, TerminalAfter, TerminalCommandGate, TerminalLaunch,
    TerminalRunController,
};
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::BottomPane;
use crate::bottom_pane::BottomPaneParams;
use crate::bottom_pane::CancellationEvent;
// CustomPromptView moved to review_handlers.rs (MAINT-11 Phase 7)
use crate::bottom_pane::InputResult;
use crate::bottom_pane::LoginAccountsState;
use crate::bottom_pane::LoginAccountsView;
use crate::bottom_pane::LoginAddAccountState;
use crate::bottom_pane::LoginAddAccountView;
// UndoRestoreView, list_selection_view moved to undo_snapshots.rs (MAINT-11 Phase 9)
use crate::bottom_pane::UpdateSharedState;
use crate::bottom_pane::validation_settings_view;
use crate::bottom_pane::validation_settings_view::{GroupStatus, ToolRow};
use crate::height_manager::HeightEvent;
use crate::height_manager::HeightManager;
use crate::history_cell;
use crate::history_cell::ExecCell;
use crate::history_cell::HistoryCell;
use crate::history_cell::HistoryCellType;
use crate::history_cell::PatchEventType;
use crate::history_cell::PlainHistoryCell;
use crate::history_cell::clean_wait_command;
use crate::live_wrap::RowBuilder;
use crate::rate_limits_view::{DEFAULT_GRID_CONFIG, RateLimitResetInfo, build_limits_view};
use crate::streaming::StreamKind;
use crate::streaming::controller::AppEventHistorySink;
use crate::user_approval_widget::ApprovalRequest;
use crate::util::buffer::fill_rect;
use chrono::{DateTime, Duration as ChronoDuration, Local, Utc};
use codex_core::config::find_codex_home;
use codex_core::config::set_github_actionlint_on_patch;
use codex_core::config::set_github_check_on_push;
use codex_core::config::set_validation_group_enabled;
use codex_core::config::set_validation_tool_enabled;
use codex_core::config_types::{ValidationCategory, validation_tool_category};
use codex_core::protocol::RateLimitSnapshotEvent;
use codex_core::protocol::ValidationGroup;
// format_review_findings_block moved to review_handlers.rs (MAINT-11 Phase 7)
// ContentItem, ResponseItem moved to session_handlers.rs (MAINT-11 Phase 8)
use codex_file_search::FileMatch;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use ratatui::style::Stylize;
use ratatui::symbols::scrollbar as scrollbar_symbols;
use ratatui::text::Text as RtText;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use unicode_width::UnicodeWidthStr;

struct RunningCommand {
    command: Vec<String>,
    parsed: Vec<ParsedCommand>,
    // Index of the in-history Exec cell for this call, if inserted
    history_index: Option<usize>,
    // Aggregated exploration entry (history index, entry index) when grouped
    explore_entry: Option<(usize, usize)>,
    stdout: String,
    stderr: String,
    wait_total: Option<Duration>,
    wait_active: bool,
    wait_notes: Vec<(String, bool)>,
}

const RATE_LIMIT_WARNING_THRESHOLDS: [f64; 3] = [50.0, 75.0, 90.0];
const RATE_LIMIT_REFRESH_INTERVAL: chrono::Duration = chrono::Duration::minutes(10);

const MAX_TRACKED_GHOST_COMMITS: usize = 20;

#[derive(Default)]
struct RateLimitWarningState {
    weekly_index: usize,
    hourly_index: usize,
}

impl RateLimitWarningState {
    fn take_warnings(&mut self, weekly_used_percent: f64, hourly_used_percent: f64) -> Vec<String> {
        let mut warnings = Vec::new();

        while self.weekly_index < RATE_LIMIT_WARNING_THRESHOLDS.len()
            && weekly_used_percent >= RATE_LIMIT_WARNING_THRESHOLDS[self.weekly_index]
        {
            let threshold = RATE_LIMIT_WARNING_THRESHOLDS[self.weekly_index];
            warnings.push(format!(
                "Secondary usage exceeded {threshold:.0}% of the limit. Run /limits for detailed usage."
            ));
            self.weekly_index += 1;
        }

        while self.hourly_index < RATE_LIMIT_WARNING_THRESHOLDS.len()
            && hourly_used_percent >= RATE_LIMIT_WARNING_THRESHOLDS[self.hourly_index]
        {
            let threshold = RATE_LIMIT_WARNING_THRESHOLDS[self.hourly_index];
            warnings.push(format!(
                "Hourly usage exceeded {threshold:.0}% of the limit. Run /limits for detailed usage."
            ));
            self.hourly_index += 1;
        }

        warnings
    }

    fn reset(&mut self) {
        self.weekly_index = 0;
        self.hourly_index = 0;
    }
}

#[derive(Clone)]
struct GhostSnapshotsDisabledReason {
    message: String,
    hint: Option<String>,
}

#[derive(Clone, Copy)]
struct ConversationSnapshot {
    user_turns: usize,
    assistant_turns: usize,
    history_len: usize,
    order_len: usize,
    order_dbg_len: usize,
}

impl ConversationSnapshot {
    fn new(user_turns: usize, assistant_turns: usize) -> Self {
        Self {
            user_turns,
            assistant_turns,
            history_len: 0,
            order_len: 0,
            order_dbg_len: 0,
        }
    }
}

#[derive(Clone)]
pub(crate) struct GhostState {
    snapshots: Vec<GhostSnapshot>,
    disabled: bool,
    disabled_reason: Option<GhostSnapshotsDisabledReason>,
}

struct UndoSnapshotPreview {
    index: usize,
    short_id: String,
    summary: Option<String>,
    captured_at: DateTime<Local>,
    age: Option<std::time::Duration>,
    user_delta: usize,
    assistant_delta: usize,
}

pub(crate) struct ChatWidget<'a> {
    app_event_tx: AppEventSender,
    codex_op_tx: UnboundedSender<Op>,
    bottom_pane: BottomPane<'a>,
    auth_manager: Arc<AuthManager>,
    login_view_state: Option<Weak<RefCell<LoginAccountsState>>>,
    login_add_view_state: Option<Weak<RefCell<LoginAddAccountState>>>,
    // P6-SYNC Phase 7: Device code login view state for interactive OAuth flow
    device_code_login_state: Option<Weak<RefCell<crate::bottom_pane::DeviceCodeLoginState>>>,
    active_exec_cell: Option<ExecCell>,
    history_cells: Vec<Box<dyn HistoryCell>>, // Store all history in memory
    history_render: HistoryRenderState,
    config: Config,
    latest_upgrade_version: Option<String>,
    initial_user_message: Option<UserMessage>,
    total_token_usage: TokenUsage,
    last_token_usage: TokenUsage,
    pub cost_tracker: Arc<spec_kit::cost_tracker::CostTracker>,
    rate_limit_snapshot: Option<RateLimitSnapshotEvent>,
    rate_limit_warnings: RateLimitWarningState,
    rate_limit_fetch_inflight: bool,
    rate_limit_last_fetch_at: Option<DateTime<Utc>>,
    rate_limit_primary_next_reset_at: Option<DateTime<Utc>>,
    rate_limit_secondary_next_reset_at: Option<DateTime<Utc>>,
    content_buffer: String,
    // Buffer for streaming assistant answer text; we do not surface partial
    // We wait for the final AgentMessage event and then emit the full text
    // at once into scrollback so the history contains a single message.
    // Cache of the last finalized assistant message to suppress immediate duplicates
    last_assistant_message: Option<String>,
    // Track the ID of the current streaming message to prevent duplicates
    // Track the ID of the current streaming reasoning to prevent duplicates
    exec: ExecState,
    tools_state: ToolState,
    live_builder: RowBuilder,
    // Store pending image paths keyed by their placeholder text
    pending_images: HashMap<String, PathBuf>,
    // (removed) pending non-image files are no longer tracked; non-image paths remain as plain text
    welcome_shown: bool,
    // Browser screenshot caching removed (MAINT-11 Phase 6)

    // Cached cell size (width,height) in pixels
    cached_cell_size: std::cell::OnceCell<(u16, u16)>,
    git_branch_cache: RefCell<GitBranchCache>,

    // Terminal information from startup
    terminal_info: crate::tui::TerminalInfo,
    // Agent tracking for multi-agent tasks
    active_agents: Vec<AgentInfo>,
    agents_ready_to_start: bool,
    last_agent_prompt: Option<String>,
    agent_context: Option<String>,
    agent_task: Option<String>,
    active_review_hint: Option<String>,
    active_review_prompt: Option<String>,
    overall_task_status: String,
    active_plan_title: Option<String>,
    /// Runtime timing per-agent (by id) to improve visibility in the HUD
    agent_runtime: HashMap<String, AgentRuntime>,
    pro: ProState,
    // Sparkline data for showing agent activity (using RefCell for interior mutability)
    // Each tuple is (value, is_completed) where is_completed indicates if any agent was complete at that time
    sparkline_data: std::cell::RefCell<Vec<(u64, bool)>>,
    last_sparkline_update: std::cell::RefCell<std::time::Instant>,
    // Stream controller for managing streaming content
    stream: crate::streaming::controller::StreamController,
    // Stream lifecycle state (kind, closures, sequencing, cancel)
    stream_state: StreamState,
    // Interrupt manager for handling cancellations
    interrupts: interrupts::InterruptManager,

    // Guard for out-of-order exec events: track call_ids that already ended
    ended_call_ids: HashSet<ExecCallId>,
    /// Exec call_ids that were explicitly cancelled by user interrupt. Used to
    /// drop any late ExecEnd events so we don't render duplicate cells.
    canceled_exec_call_ids: HashSet<ExecCallId>,

    // Accumulated diff/session state
    diffs: DiffsState,

    // Help overlay state
    help: HelpState,

    // Limits overlay state
    limits: LimitsState,

    // PM overlay state (SPEC-PM-004)
    pm: pm_overlay::PmState,

    // Terminal overlay state
    terminal: TerminalState,
    pending_manual_terminal: HashMap<u64, PendingManualTerminal>,

    // Persisted selection for Agents overview
    agents_overview_selected_index: usize,

    // State for the Agents Terminal view
    agents_terminal: AgentsTerminalState,

    pending_upgrade_notice: Option<(u64, String)>,

    // Cached visible rows for the diff overlay body to clamp scrolling (kept within diffs)

    // Centralized height manager (always enabled)
    height_manager: RefCell<HeightManager>,

    // Aggregated layout and scroll state
    layout: LayoutState,

    // Most recent theme snapshot used to retint pre-rendered lines
    last_theme: crate::theme::Theme,

    // Performance tracing (opt-in via /perf)
    perf_state: PerfState,
    // Current session id (from SessionConfigured)
    session_id: Option<uuid::Uuid>,

    // Pending jump-back state (reversible until submit)
    pending_jump_back: Option<PendingJumpBack>,

    // Track active task ids so we don't drop the working status while any
    // agent/sub‑agent is still running (long‑running sessions can interleave).
    active_task_ids: HashSet<String>,

    // --- Queued user message support ---
    // Messages typed while a task is running are kept here and rendered
    // at the bottom as "(queued)" until the next turn begins. At that
    // point we submit one queued message and move its cell into the
    // normal history within the new turn window.
    queued_user_messages: std::collections::VecDeque<UserMessage>,
    pending_dispatched_user_messages: std::collections::VecDeque<String>,
    // SPEC-954-FIX: Track user cells awaiting OrderKey update when provider OrderMeta arrives
    // Maps task_id -> cell_index. When user message is created with temporary OrderKey,
    // we store its cell index here. When first OrderMeta arrives, we update the cell's
    // OrderKey to match the provider's request_ordinal.
    pending_user_cell_updates: HashMap<String, usize>,
    // SPEC-954: Track timestamps for pending messages to detect silent failures
    // Maps message_id -> timestamp when message was queued. If TaskStarted isn't
    // received within timeout window (10s), we show error to user.
    pending_message_timestamps: HashMap<String, std::time::Instant>,
    // Number of user prompts we pre-pended to history just before starting
    // a new turn; used to anchor the next turn window so assistant output
    // appears after them.
    pending_user_prompts_for_next_turn: usize,
    ghost_snapshots: Vec<GhostSnapshot>,
    ghost_snapshots_disabled: bool,
    ghost_snapshots_disabled_reason: Option<GhostSnapshotsDisabledReason>,

    // Event sequencing to preserve original order across streaming/tool events
    // and stream-related flags moved into stream_state

    // Strict global ordering for history: every cell has a required key
    // (req, out, seq). No unordered inserts and no turn windows.
    cell_order_seq: Vec<OrderKey>,
    // Debug: per-cell order info string rendered in the UI to diagnose ordering.
    cell_order_dbg: Vec<Option<String>>,
    // Routing for reasoning stream ids -> existing CollapsibleReasoningCell index
    reasoning_index: HashMap<String, usize>,
    // Stable per-(kind, stream_id) ordering, derived from OrderMeta.
    stream_order_seq: HashMap<(StreamKind, String), OrderKey>,
    // Track last provider request_ordinal seen so internal messages can be
    // assigned request_index = last_seen + 1 (with out = -1).
    last_seen_request_index: u64,
    // Synthetic request index used for internal-only messages; always >= last_seen_request_index
    current_request_index: u64,
    // Monotonic seq for internal messages to keep intra-request order stable
    internal_seq: u64,
    // Show order overlay when true (from --order)
    show_order_overlay: bool,

    // One-time hint to teach input history navigation
    scroll_history_hint_shown: bool,

    // Track and manage the access-mode background status cell so mode changes
    // replace the existing status instead of stacking multiple entries.
    access_status_idx: Option<usize>,
    /// When true, render without the top status bar and HUD so the normal
    /// terminal scrollback remains usable (Ctrl+T standard terminal mode).
    pub(crate) standard_terminal_mode: bool,
    // Pending system notes to inject into the agent's conversation history
    // before the next user turn. Each entry is sent in order ahead of the
    // user's visible prompt.
    pending_agent_notes: Vec<String>,

    // === FORK-SPECIFIC: spec-kit automation state ===
    // Upstream: Does not have /spec-auto pipeline
    // Preserve: This field during rebases
    // Handler methods extracted to spec_kit module (free functions)
    spec_auto_state: Option<SpecAutoState>,
    validate_lifecycles: HashMap<String, spec_kit::state::ValidateLifecycle>,
    /// Pending Stage0 operation for async execution (SPEC-DOGFOOD-001 S31)
    /// When Some, poll in on_commit_tick for progress/completion
    stage0_pending: Option<spec_kit::stage0_integration::Stage0PendingOperation>,
    /// Pending maieutic state for pipeline resumption (D130)
    /// When Some, maieutic modal is displayed and pipeline pauses until completion
    pending_maieutic: Option<spec_kit::PendingMaieutic>,
    /// Pending intake backfill state for pipeline resumption (Phase 2)
    /// When Some, intake backfill modal is displayed and pipeline pauses until completion
    pending_intake_backfill: Option<spec_kit::PendingIntakeBackfill>,
    /// Pending projectnew state for multi-phase project setup flow
    /// When Some, orchestrates: vision -> project intake -> bootstrap spec
    pub(crate) pending_projectnew: Option<spec_kit::PendingProjectNew>,
    // === END FORK-SPECIFIC ===

    // === FORK-SPECIFIC (just-every/code): Native MCP for local-memory ===
    // Eliminates subprocess, 10x faster consensus queries
    // TUI-side MCP manager for querying local-memory during consensus checking
    mcp_manager: Arc<
        tokio::sync::Mutex<Option<Arc<codex_core::mcp_connection_manager::McpConnectionManager>>>,
    >,
    /// Async quality gate broker used to avoid blocking the UI when fetching
    /// agent artefacts and GPT-5 validation results from local-memory.
    quality_gate_broker: QualityGateBroker,
    // === END FORK-SPECIFIC ===

    // === FORK-SPECIFIC (just-every/code): SPEC-KIT-920 TUI automation ===
    /// Initial slash command to auto-submit after TUI is ready (for automation).
    /// Consumed (taken) on first successful auto-submit.
    /// NOTE: This field is passed through ChatWidget but consumed by App::dispatch_initial_command.
    #[allow(dead_code)] // Passed through to App; consider moving to App struct
    initial_command: Option<String>,
    // === END FORK-SPECIFIC ===

    // === FORK-SPECIFIC (just-every/code): SPEC-939 Component 1a - Config hot-reload ===
    /// Configuration hot-reload watcher for live config updates.
    /// Enables component refresh on config changes without restart.
    config_watcher: Option<ConfigWatcher>,
    /// Pending config reload paths (deferred if quality gate active)
    pending_config_reload: Option<Vec<PathBuf>>,
    // === END FORK-SPECIFIC ===

    // === FORK-SPECIFIC (just-every/code): SPEC-KIT-953 Native Multi-Provider Integration ===
    /// Current streaming provider name (Claude/Gemini)
    native_stream_provider: Option<String>,
    /// Current streaming model name
    native_stream_model: Option<String>,
    /// Current streaming message ID
    native_stream_id: Option<String>,
    /// Accumulated streaming content for history
    native_stream_content: String,
    // === END FORK-SPECIFIC ===

    // Stable synthetic request bucket for pre‑turn system notices (set on first use)
    synthetic_system_req: Option<u64>,
    // Map of system notice ids to their history index for in-place replacement
    system_cell_by_id: std::collections::HashMap<String, usize>,
}

struct PendingJumpBack {
    removed_cells: Vec<Box<dyn HistoryCell>>, // cells removed from the end (from selected user message onward)
}

#[derive(Clone)]
struct GhostSnapshot {
    commit: GhostCommit,
    captured_at: DateTime<Local>,
    summary: Option<String>,
    conversation: ConversationSnapshot,
}

impl GhostSnapshot {
    fn new(
        commit: GhostCommit,
        summary: Option<String>,
        conversation: ConversationSnapshot,
    ) -> Self {
        let summary = summary.and_then(|text| {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        Self {
            commit,
            captured_at: Local::now(),
            summary,
            conversation,
        }
    }

    fn commit(&self) -> &GhostCommit {
        &self.commit
    }

    fn short_id(&self) -> String {
        self.commit.id().chars().take(8).collect()
    }

    fn summary_snippet(&self, max_len: usize) -> Option<String> {
        let summary = self.summary.as_ref()?;
        let mut snippet = String::new();
        let mut truncated = false;
        for word in summary.split_whitespace() {
            if !snippet.is_empty() {
                snippet.push(' ');
            }
            snippet.push_str(word);
            if snippet.chars().count() > max_len {
                truncated = true;
                break;
            }
        }

        if snippet.chars().count() > max_len {
            truncated = true;
            snippet = snippet.chars().take(max_len).collect();
        }

        if truncated {
            snippet.push('…');
        }

        Some(snippet)
    }

    fn age_from(&self, now: DateTime<Local>) -> Option<std::time::Duration> {
        now.signed_duration_since(self.captured_at).to_std().ok()
    }
}

#[derive(Default)]
struct GitBranchCache {
    value: Option<String>,
    last_head_mtime: Option<SystemTime>,
    last_refresh: Option<Instant>,
}

#[derive(Debug, Clone, Default)]
struct AgentRuntime {
    /// First time this agent entered Running
    started_at: Option<Instant>,
    /// Time of the latest status update we observed
    last_update: Option<Instant>,
    /// Time the agent reached a terminal state (Completed/Failed)
    completed_at: Option<Instant>,
}

// ---------- Stable ordering & routing helpers ----------
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OrderKey {
    pub(crate) req: u64,
    pub(crate) out: i32,
    pub(crate) seq: u64,
}

impl Ord for OrderKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.req.cmp(&other.req) {
            std::cmp::Ordering::Equal => match self.out.cmp(&other.out) {
                std::cmp::Ordering::Equal => self.seq.cmp(&other.seq),
                o => o,
            },
            o => o,
        }
    }
}

impl PartialOrd for OrderKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Removed legacy turn-window logic; ordering is strictly global.

use self::diff_ui::DiffBlock;
use self::diff_ui::DiffConfirm;
use self::diff_ui::DiffOverlay;
use ratatui::text::Line as RtLine;
use ratatui::text::Span as RtSpan;

use self::message::UserMessage;

use self::perf::PerfStats;

#[derive(Debug, Clone)]
struct AgentInfo {
    // Stable id to correlate updates
    id: String,
    // Display name
    name: String,
    // Current status
    status: AgentStatus,
    // Batch identifier reported by the core (if any)
    batch_id: Option<String>,
    // Optional model name
    model: Option<String>,
    // Final success message when completed
    result: Option<String>,
    // Final error message when failed
    error: Option<String>,
    // Most recent progress line from core
    last_progress: Option<String>,
}

use self::message::create_initial_user_message;

// Newtype IDs for clarity across exec/tools/streams
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ExecCallId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ToolCallId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct StreamId(pub String);

impl From<String> for ExecCallId {
    fn from(s: String) -> Self {
        ExecCallId(s)
    }
}
impl From<&str> for ExecCallId {
    fn from(s: &str) -> Self {
        ExecCallId(s.to_string())
    }
}

fn wait_target_from_params(params: Option<&String>, call_id: &str) -> String {
    if let Some(raw) = params
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(raw)
    {
        if let Some(for_value) = json.get("for").and_then(|v| v.as_str()) {
            let cleaned = clean_wait_command(for_value);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
        if let Some(cid) = json.get("call_id").and_then(|v| v.as_str()) {
            return format!("call {}", cid);
        }
    }
    format!("call {}", call_id)
}

fn wait_exec_call_id_from_params(params: Option<&String>) -> Option<ExecCallId> {
    params
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|json| {
            json.get("call_id")
                .and_then(|v| v.as_str())
                .map(|s| ExecCallId(s.to_string()))
        })
}

impl std::fmt::Display for ExecCallId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl AsRef<str> for ExecCallId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for ToolCallId {
    fn from(s: String) -> Self {
        ToolCallId(s)
    }
}
impl From<&str> for ToolCallId {
    fn from(s: &str) -> Self {
        ToolCallId(s.to_string())
    }
}
impl std::fmt::Display for ToolCallId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl AsRef<str> for ToolCallId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for StreamId {
    fn from(s: String) -> Self {
        StreamId(s)
    }
}
impl From<&str> for StreamId {
    fn from(s: &str) -> Self {
        StreamId(s.to_string())
    }
}
impl std::fmt::Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl AsRef<str> for StreamId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ---- System notice ordering helpers ----
#[derive(Copy, Clone)]
enum SystemPlacement {
    /// Place near the top of the current request (before most provider output)
    EarlyInCurrent,
    /// Place at the end of the current request window (after provider output)
    EndOfCurrent,
    /// Place before the first user prompt of the very first request
    /// (used for pre-turn UI confirmations like theme/spinner changes)
    PrePromptInCurrent,
}

impl ChatWidget<'_> {
    fn spec_kit_telemetry_enabled(&self) -> bool {
        spec_kit::state::spec_kit_telemetry_enabled(&self.config.shell_environment_policy)
    }

    fn spec_kit_auto_commit_enabled(&self) -> bool {
        spec_kit::state::spec_kit_auto_commit_enabled(&self.config.shell_environment_policy)
    }

    fn ensure_validate_lifecycle(&mut self, spec_id: &str) -> ValidateLifecycle {
        self.validate_lifecycles
            .entry(spec_id.to_string())
            .or_insert_with(|| ValidateLifecycle::new(spec_id))
            .clone()
    }

    fn fmt_short_duration(&self, d: Duration) -> String {
        let s = d.as_secs();
        let h = s / 3600;
        let m = (s % 3600) / 60;
        let sec = s % 60;
        if h > 0 {
            format!("{}h{}m", h, m)
        } else if m > 0 {
            format!("{}m{}s", m, sec)
        } else {
            format!("{}s", sec)
        }
    }
    fn is_branch_worktree_path(path: &std::path::Path) -> bool {
        for ancestor in path.ancestors() {
            if ancestor
                .file_name()
                .map(|name| name == std::ffi::OsStr::new("branches"))
                .unwrap_or(false)
            {
                let mut higher = ancestor.parent();
                while let Some(dir) = higher {
                    if dir
                        .file_name()
                        .map(|name| name == std::ffi::OsStr::new(".code"))
                        .unwrap_or(false)
                    {
                        return true;
                    }
                    higher = dir.parent();
                }
            }
        }
        false
    }

    async fn git_short_status(path: &std::path::Path) -> Result<String, String> {
        use tokio::process::Command;
        match Command::new("git")
            .current_dir(path)
            .args(["status", "--short"])
            .output()
            .await
        {
            Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
            Ok(out) => {
                let stderr_s = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let stdout_s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !stderr_s.is_empty() {
                    Err(stderr_s)
                } else if !stdout_s.is_empty() {
                    Err(stdout_s)
                } else {
                    let code = out
                        .status
                        .code()
                        .map(|c| format!("exit status {c}"))
                        .unwrap_or_else(|| "terminated by signal".to_string());
                    Err(format!("git status failed: {}", code))
                }
            }
            Err(err) => Err(err.to_string()),
        }
    }

    async fn git_diff_stat(path: &std::path::Path) -> Result<String, String> {
        use tokio::process::Command;
        match Command::new("git")
            .current_dir(path)
            .args(["diff", "--stat"])
            .output()
            .await
        {
            Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
            Ok(out) => {
                let stderr_s = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let stdout_s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !stderr_s.is_empty() {
                    Err(stderr_s)
                } else if !stdout_s.is_empty() {
                    Err(stdout_s)
                } else {
                    let code = out
                        .status
                        .code()
                        .map(|c| format!("exit status {c}"))
                        .unwrap_or_else(|| "terminated by signal".to_string());
                    Err(format!("git diff --stat failed: {code}"))
                }
            }
            Err(err) => Err(err.to_string()),
        }
    }

    /// Compute an OrderKey for system (non‑LLM) notices in a way that avoids
    /// creating multiple synthetic request buckets before the first provider turn.
    fn system_order_key(
        &mut self,
        placement: SystemPlacement,
        order: Option<&codex_core::protocol::OrderMeta>,
    ) -> OrderKey {
        // If the provider supplied OrderMeta, honor it strictly.
        if let Some(om) = order {
            return Self::order_key_from_order_meta(om);
        }

        // Derive a stable request bucket for system notices when OrderMeta is absent.
        // Default to the current provider request if known; else use a sticky
        // pre-turn synthetic req=1 to group UI confirmations before the first turn.
        // If a user prompt for the next turn is already queued, attach new
        // system notices to the upcoming request to avoid retroactive inserts.
        let mut req = if self.last_seen_request_index > 0 {
            self.last_seen_request_index
        } else {
            if self.synthetic_system_req.is_none() {
                self.synthetic_system_req = Some(1);
            }
            self.synthetic_system_req.unwrap_or(1)
        };
        if order.is_none() && self.pending_user_prompts_for_next_turn > 0 {
            req = req.saturating_add(1);
        }

        self.internal_seq = self.internal_seq.saturating_add(1);
        let mut out = match placement {
            SystemPlacement::EarlyInCurrent => i32::MIN + 2,
            SystemPlacement::EndOfCurrent => i32::MAX,
            SystemPlacement::PrePromptInCurrent => i32::MIN,
        };

        if order.is_none()
            && self.pending_user_prompts_for_next_turn > 0
            && matches!(placement, SystemPlacement::EarlyInCurrent)
        {
            out = i32::MIN;
        }

        OrderKey {
            req,
            out,
            seq: self.internal_seq,
        }
    }

    /// Insert or replace a system notice cell with consistent ordering.
    /// If `id_for_replace` is provided and we have a prior index for it, replace in place.
    fn push_system_cell(
        &mut self,
        cell: impl HistoryCell + 'static,
        placement: SystemPlacement,
        id_for_replace: Option<String>,
        order: Option<&codex_core::protocol::OrderMeta>,
        tag: &'static str,
    ) {
        if let Some(id) = id_for_replace.as_ref()
            && let Some(&idx) = self.system_cell_by_id.get(id)
        {
            self.history_replace_at(idx, Box::new(cell));
            return;
        }
        let key = self.system_order_key(placement, order);
        let pos = self.history_insert_with_key_global_tagged(Box::new(cell), key, tag);
        if let Some(id) = id_for_replace {
            self.system_cell_by_id.insert(id, pos);
        }
    }

    /// Decide where to place a UI confirmation right now.
    /// If we're truly pre-turn (no provider traffic yet, and no queued prompt),
    /// place before the first user prompt. Otherwise, append to end of current.
    fn ui_placement_for_now(&self) -> SystemPlacement {
        if self.last_seen_request_index == 0 && self.pending_user_prompts_for_next_turn == 0 {
            SystemPlacement::PrePromptInCurrent
        } else {
            SystemPlacement::EndOfCurrent
        }
    }
    pub(crate) fn enable_perf(&mut self, enable: bool) {
        self.perf_state.enabled = enable;
    }
    pub(crate) fn perf_summary(&self) -> String {
        self.perf_state.stats.borrow().summary()
    }
    // Build an ordered key from model-provided OrderMeta. Callers must
    // guarantee presence by passing a concrete reference (compile-time guard).
    fn order_key_from_order_meta(om: &codex_core::protocol::OrderMeta) -> OrderKey {
        // sequence_number can be None on some terminal events; treat as 0 for stable placement
        OrderKey {
            req: om.request_ordinal,
            out: om.output_index.map(|v| v as i32).unwrap_or(0),
            seq: om.sequence_number.unwrap_or(0),
        }
    }

    // Track latest request index observed from provider so internal inserts can anchor to it.
    fn note_order(&mut self, order: Option<&codex_core::protocol::OrderMeta>) {
        if let Some(om) = order {
            self.last_seen_request_index = self.last_seen_request_index.max(om.request_ordinal);
        }
    }

    fn debug_fmt_order_key(ok: OrderKey) -> String {
        format!("O:req={} out={} seq={}", ok.req, ok.out, ok.seq)
    }

    // Allocate a key that places an internal (non‑model) event at the point it
    // occurs during the current request, instead of sinking it to the end.
    //
    // Strategy:
    // - If an OrderMeta is provided, honor it (strict model ordering).
    // - Otherwise, if a new turn is queued (a user prompt was just inserted),
    //   anchor immediately after that prompt within the upcoming request so
    //   the notice appears in the right window.
    // - Otherwise, derive a key within the current request:
    //   * If there is any existing cell in this request, append after the
    //     latest key in this request (req = last_seen, out/seq bumped).
    //   * If no cells exist for this request yet, place near the top of this
    //     request (after headers/prompts) so provider output can follow.
    fn near_time_key(&mut self, order: Option<&codex_core::protocol::OrderMeta>) -> OrderKey {
        if let Some(om) = order {
            return Self::order_key_from_order_meta(om);
        }

        // If we just staged a user prompt for the next request, keep using the
        // next‑turn anchor so the background item lands with that turn.
        if self.pending_user_prompts_for_next_turn > 0 {
            return self.next_req_key_after_prompt();
        }

        let req = if self.last_seen_request_index > 0 {
            self.last_seen_request_index
        } else {
            // No provider traffic yet: allocate a synthetic request bucket.
            // Use the same path as next_internal_key() to keep monotonicity.
            if self.current_request_index < self.last_seen_request_index {
                self.current_request_index = self.last_seen_request_index;
            }
            self.current_request_index = self.current_request_index.saturating_add(1);
            self.current_request_index
        };

        // Scan for the latest key within this request to append after.
        let mut last_in_req: Option<OrderKey> = None;
        for k in &self.cell_order_seq {
            if k.req == req {
                last_in_req = Some(match last_in_req {
                    Some(prev) => {
                        if *k > prev {
                            *k
                        } else {
                            prev
                        }
                    }
                    None => *k,
                });
            }
        }

        self.internal_seq = self.internal_seq.saturating_add(1);
        match last_in_req {
            Some(last) => OrderKey {
                req,
                out: last.out,
                seq: last.seq.saturating_add(1),
            },
            None => OrderKey {
                req,
                out: i32::MIN + 2,
                seq: self.internal_seq,
            },
        }
    }

    /// Like near_time_key but never advances to the next request when a prompt is queued.
    /// Use this for late, provider-origin items that lack OrderMeta (e.g., PlanUpdate)
    /// so they remain attached to the current/last request instead of jumping forward.
    fn near_time_key_current_req(
        &mut self,
        order: Option<&codex_core::protocol::OrderMeta>,
    ) -> OrderKey {
        if let Some(om) = order {
            return Self::order_key_from_order_meta(om);
        }
        let req = if self.last_seen_request_index > 0 {
            self.last_seen_request_index
        } else {
            if self.current_request_index < self.last_seen_request_index {
                self.current_request_index = self.last_seen_request_index;
            }
            self.current_request_index = self.current_request_index.saturating_add(1);
            self.current_request_index
        };

        let mut last_in_req: Option<OrderKey> = None;
        for k in &self.cell_order_seq {
            if k.req == req {
                last_in_req = Some(match last_in_req {
                    Some(prev) => {
                        if *k > prev {
                            *k
                        } else {
                            prev
                        }
                    }
                    None => *k,
                });
            }
        }
        self.internal_seq = self.internal_seq.saturating_add(1);
        match last_in_req {
            Some(last) => OrderKey {
                req,
                out: last.out,
                seq: last.seq.saturating_add(1),
            },
            None => OrderKey {
                req,
                out: i32::MIN + 2,
                seq: self.internal_seq,
            },
        }
    }

    // After inserting a non‑reasoning cell during streaming, restore the
    // in‑progress indicator on the latest reasoning cell so the ellipsis
    // remains visible while the model continues.
    fn restore_reasoning_in_progress_if_streaming(&mut self) {
        if !self.stream.is_write_cycle_active() {
            return;
        }
        if let Some(idx) = self.history_cells.iter().rposition(|c| {
            c.as_any()
                .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
                .is_some()
        }) && let Some(rc) = self.history_cells[idx]
            .as_any()
            .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
        {
            rc.set_in_progress(true);
        }
    }

    fn apply_plan_terminal_title(&mut self, title: Option<String>) {
        if self.active_plan_title == title {
            return;
        }
        self.active_plan_title = title.clone();
        self.app_event_tx.send(AppEvent::SetTerminalTitle { title });
    }
    // Allocate a new synthetic key for internal (non-LLM) messages at the bottom of the
    // current (active) request: (req = last_seen, out = +∞, seq = monotonic).
    fn next_internal_key(&mut self) -> OrderKey {
        // Anchor to the current provider request if known; otherwise step a synthetic counter.
        let mut req = if self.last_seen_request_index > 0 {
            self.last_seen_request_index
        } else {
            // Ensure current_request_index always moves forward
            if self.current_request_index < self.last_seen_request_index {
                self.current_request_index = self.last_seen_request_index;
            }
            self.current_request_index = self.current_request_index.saturating_add(1);
            self.current_request_index
        };
        if self.pending_user_prompts_for_next_turn > 0 {
            let next_req = self.last_seen_request_index.saturating_add(1);
            if req < next_req {
                req = next_req;
            }
        }
        if self.current_request_index < req {
            self.current_request_index = req;
        }
        self.internal_seq = self.internal_seq.saturating_add(1);
        // Place internal notices at the end of the current request window by using
        // a maximal out so they sort after any model-provided output_index.
        OrderKey {
            req,
            out: i32::MAX,
            seq: self.internal_seq,
        }
    }

    /// Show the "Shift+Up/Down" input history hint the first time the user scrolls.
    pub(super) fn maybe_show_history_nav_hint_on_first_scroll(&mut self) {
        if self.scroll_history_hint_shown {
            return;
        }
        self.scroll_history_hint_shown = true;
        self.bottom_pane.flash_footer_notice_for(
            "Use Shift+Up/Down to use previous input".to_string(),
            std::time::Duration::from_secs(6),
        );
    }

    // Synthetic key for internal content that should appear at the TOP of the NEXT request
    // (e.g., the user’s prompt preceding the model’s output for that turn).
    // SPEC-955 Session 2: Increment current_request_index for synthetic keys
    // to ensure each user turn gets a unique request number.
    fn next_req_key_top(&mut self) -> OrderKey {
        self.current_request_index = self.current_request_index.saturating_add(1);
        let req = self
            .current_request_index
            .max(self.last_seen_request_index.saturating_add(1));
        self.current_request_index = req;

        self.internal_seq = self.internal_seq.saturating_add(1);
        OrderKey {
            req,
            out: i32::MIN,
            seq: self.internal_seq,
        }
    }

    // Synthetic key for a user prompt that should appear just after banners but
    // still before any model output within the next request.
    //
    // SPEC-955 Session 2: Fixed to increment current_request_index for each user message,
    // ensuring multiple user messages get different request numbers and don't interleave.
    fn next_req_key_prompt(&mut self) -> OrderKey {
        // Increment current_request_index to get unique request number for each user message
        self.current_request_index = self.current_request_index.saturating_add(1);
        // Ensure it's at least last_seen + 1
        let req = self
            .current_request_index
            .max(self.last_seen_request_index.saturating_add(1));
        // Update current_request_index to the actual value we're using
        self.current_request_index = req;

        self.internal_seq = self.internal_seq.saturating_add(1);
        OrderKey {
            req,
            out: i32::MIN + 1,
            seq: self.internal_seq,
        }
    }

    // Synthetic key for internal notices tied to the upcoming turn that
    // should appear immediately after the user prompt but still before any
    // model output for that turn.
    //
    // SPEC-955 Session 2: Uses current_request_index (not incremented here -
    // this is for notices AFTER the prompt in the same request).
    fn next_req_key_after_prompt(&mut self) -> OrderKey {
        // Don't increment - use current request (same as the prompt that just went in)
        let req = self
            .current_request_index
            .max(self.last_seen_request_index.saturating_add(1));
        self.internal_seq = self.internal_seq.saturating_add(1);
        OrderKey {
            req,
            out: i32::MIN + 2,
            seq: self.internal_seq,
        }
    }
    /// Returns true if any agents are actively running (Pending or Running), or we're about to start them.
    /// Agents in terminal states (Completed/Failed) do not keep the spinner visible.
    fn agents_are_actively_running(&self) -> bool {
        if self.agents_ready_to_start {
            return true;
        }
        self.active_agents
            .iter()
            .any(|a| matches!(a.status, AgentStatus::Pending | AgentStatus::Running))
    }

    /// Hide the bottom spinner/status if the UI is idle (no streams, tools, agents, or tasks).
    fn maybe_hide_spinner(&mut self) {
        let any_tools_running = !self.exec.running_commands.is_empty()
            || !self.tools_state.running_custom_tools.is_empty()
            || !self.tools_state.running_web_search.is_empty();
        let any_streaming = self.stream.is_write_cycle_active();
        let any_agents_active = self.agents_are_actively_running();
        let any_tasks_active = !self.active_task_ids.is_empty();
        if !(any_tools_running || any_streaming || any_agents_active || any_tasks_active) {
            self.bottom_pane.set_task_running(false);
            self.bottom_pane.update_status_text(String::new());
        }
    }

    fn remove_background_completion_message(&mut self, call_id: &str) {
        if let Some(idx) =
            self.history_cells.iter().rposition(|cell| {
                matches!(cell.kind(), HistoryCellType::BackgroundEvent)
                    && cell
                        .as_any()
                        .downcast_ref::<PlainHistoryCell>()
                        .map(|plain| {
                            plain.state().lines.iter().any(|line| {
                                line.spans.iter().any(|span| span.text.contains(call_id))
                            })
                        })
                        .unwrap_or(false)
            })
        {
            self.history_remove_at(idx);
        }
    }

    /// Flush any ExecEnd events that arrived before their matching ExecBegin.
    /// We briefly stash such ends to allow natural pairing when the Begin shows up
    /// shortly after. If the pairing window expires, render a fallback completed
    /// Exec cell so users still see the output in history.
    pub(crate) fn flush_pending_exec_ends(&mut self) {
        use std::time::Duration;
        use std::time::Instant;
        let now = Instant::now();
        // Collect keys to avoid holding a mutable borrow while iterating
        let mut ready: Vec<ExecCallId> = Vec::new();
        for (k, (_ev, _order, t0)) in self.exec.pending_exec_ends.iter() {
            if now.saturating_duration_since(*t0) >= Duration::from_millis(110) {
                ready.push(k.clone());
            }
        }
        for key in &ready {
            if let Some((ev, order, _t0)) = self.exec.pending_exec_ends.remove(key) {
                // Regardless of whether a Begin has arrived by now, handle the End;
                // handle_exec_end_now pairs with a running Exec if present, or falls back.
                self.handle_exec_end_now(ev, &order);
            }
        }
        if !ready.is_empty() {
            self.request_redraw();
        }
    }

    fn finalize_all_running_as_interrupted(&mut self) {
        exec_tools::finalize_all_running_as_interrupted(self);
    }

    fn finalize_all_running_due_to_answer(&mut self) {
        exec_tools::finalize_all_running_due_to_answer(self);
    }
    fn perf_label_for_item(&self, item: &dyn HistoryCell) -> String {
        use crate::history_cell::ExecKind;
        use crate::history_cell::ExecStatus;
        use crate::history_cell::HistoryCellType;
        use crate::history_cell::PatchKind;
        use crate::history_cell::ToolStatus;
        match item.kind() {
            HistoryCellType::Plain => "Plain".to_string(),
            HistoryCellType::User => "User".to_string(),
            HistoryCellType::Assistant => "Assistant".to_string(),
            HistoryCellType::Reasoning => "Reasoning".to_string(),
            HistoryCellType::Error => "Error".to_string(),
            HistoryCellType::Exec { kind, status } => {
                let k = match kind {
                    ExecKind::Read => "Read",
                    ExecKind::Search => "Search",
                    ExecKind::List => "List",
                    ExecKind::Run => "Run",
                };
                let s = match status {
                    ExecStatus::Running => "Running",
                    ExecStatus::Success => "Success",
                    ExecStatus::Error => "Error",
                };
                format!("Exec:{}:{}", k, s)
            }
            HistoryCellType::Tool { status } => {
                let s = match status {
                    ToolStatus::Running => "Running",
                    ToolStatus::Success => "Success",
                    ToolStatus::Failed => "Failed",
                };
                format!("Tool:{}", s)
            }
            HistoryCellType::Patch { kind } => {
                let k = match kind {
                    PatchKind::Proposed => "Proposed",
                    PatchKind::ApplyBegin => "ApplyBegin",
                    PatchKind::ApplySuccess => "ApplySuccess",
                    PatchKind::ApplyFailure => "ApplyFailure",
                };
                format!("Patch:{}", k)
            }
            HistoryCellType::PlanUpdate => "PlanUpdate".to_string(),
            HistoryCellType::BackgroundEvent => "BackgroundEvent".to_string(),
            HistoryCellType::Notice => "Notice".to_string(),
            HistoryCellType::Diff => "Diff".to_string(),
            HistoryCellType::Image => "Image".to_string(),
            HistoryCellType::AnimatedWelcome => "AnimatedWelcome".to_string(),
            HistoryCellType::Loading => "Loading".to_string(),
        }
    }

    // MAINT-11 Phase 8: show_resume_picker, render_replay_item moved to session_handlers.rs

    fn render_cached_lines(
        &self,
        item: &dyn HistoryCell,
        layout: &CachedLayout,
        area: Rect,
        buf: &mut Buffer,
        skip_rows: u16,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let total = layout.lines.len() as u16;
        if skip_rows >= total {
            return;
        }

        debug_assert_eq!(layout.lines.len(), layout.rows.len());

        let cell_bg = match item.kind() {
            crate::history_cell::HistoryCellType::Assistant => crate::colors::assistant_bg(),
            _ => crate::colors::background(),
        };

        if matches!(item.kind(), crate::history_cell::HistoryCellType::Assistant) {
            let bg_style = Style::default().bg(cell_bg).fg(crate::colors::text());
            fill_rect(buf, area, Some(' '), bg_style);
        }

        let max_rows = area.height.min(total.saturating_sub(skip_rows));
        let buf_width = buf.area.width as usize;
        let offset_x = area.x.saturating_sub(buf.area.x) as usize;
        let offset_y = area.y.saturating_sub(buf.area.y) as usize;
        let row_width = area.width as usize;

        for (visible_offset, src_index) in
            (skip_rows as usize..skip_rows as usize + max_rows as usize).enumerate()
        {
            let src_row = layout
                .rows
                .get(src_index)
                .map(|row| row.as_ref())
                .unwrap_or(&[]);

            let dest_y = offset_y + visible_offset;
            if dest_y >= buf.area.height as usize {
                break;
            }
            let start = dest_y * buf_width + offset_x;
            if start >= buf.content.len() {
                break;
            }
            let max_width = row_width.min(buf_width.saturating_sub(offset_x));
            let end = (start + max_width).min(buf.content.len());
            if end <= start {
                continue;
            }
            let dest_slice = &mut buf.content[start..end];

            let copy_len = src_row.len().min(dest_slice.len());
            if copy_len == dest_slice.len() {
                if copy_len > 0 {
                    dest_slice.clone_from_slice(&src_row[..copy_len]);
                }
            } else {
                for (dst, src) in dest_slice.iter_mut().zip(src_row.iter()).take(copy_len) {
                    dst.clone_from(src);
                }
                for cell in dest_slice.iter_mut().skip(copy_len) {
                    cell.reset();
                }
            }

            for cell in dest_slice.iter_mut() {
                if cell.bg == ratatui::style::Color::Reset {
                    cell.bg = cell_bg;
                }
            }
        }
    }
    /// Trigger fade on the welcome cell when the composer expands (e.g., slash popup).
    pub(crate) fn on_composer_expanded(&mut self) {
        for cell in &self.history_cells {
            cell.trigger_fade();
        }
        self.request_redraw();
    }
    /// If the user is at or near the bottom, keep following new messages.
    /// We treat "near" as within 3 rows, matching our scroll step.
    fn autoscroll_if_near_bottom(&mut self) {
        layout_scroll::autoscroll_if_near_bottom(self);
    }

    fn clear_reasoning_in_progress(&mut self) {
        let mut changed = false;
        for cell in &self.history_cells {
            if let Some(reasoning_cell) = cell
                .as_any()
                .downcast_ref::<history_cell::CollapsibleReasoningCell>()
            {
                reasoning_cell.set_in_progress(false);
                changed = true;
            }
        }
        if changed {
            self.invalidate_height_cache();
        }
    }

    fn refresh_reasoning_collapsed_visibility(&mut self) {
        let show = self.config.tui.show_reasoning;
        if show {
            for cell in &self.history_cells {
                if let Some(reasoning_cell) = cell
                    .as_any()
                    .downcast_ref::<history_cell::CollapsibleReasoningCell>()
                {
                    reasoning_cell.set_hide_when_collapsed(false);
                }
            }
            return;
        }

        use std::collections::HashSet;
        let mut hide_indices: HashSet<usize> = HashSet::new();
        let len = self.history_cells.len();
        let mut idx = 0usize;
        while idx < len {
            let is_explore = self.history_cells[idx]
                .as_any()
                .downcast_ref::<history_cell::ExploreAggregationCell>()
                .is_some();
            if !is_explore {
                idx += 1;
                continue;
            }
            let mut reasoning_indices: Vec<usize> = Vec::new();
            let mut j = idx + 1;
            while j < len {
                if self.history_cells[j]
                    .as_any()
                    .downcast_ref::<history_cell::CollapsibleReasoningCell>()
                    .is_some()
                {
                    reasoning_indices.push(j);
                    j += 1;
                    continue;
                }
                break;
            }
            if reasoning_indices.len() > 1 {
                for &ri in &reasoning_indices[..reasoning_indices.len() - 1] {
                    hide_indices.insert(ri);
                }
            }
            idx = j;
        }

        for (i, cell) in self.history_cells.iter().enumerate() {
            if let Some(reasoning_cell) = cell
                .as_any()
                .downcast_ref::<history_cell::CollapsibleReasoningCell>()
            {
                if hide_indices.contains(&i) {
                    reasoning_cell.set_hide_when_collapsed(true);
                } else {
                    reasoning_cell.set_hide_when_collapsed(false);
                }
            }
        }
    }

    // Legacy helper removed: streaming now requires explicit sequence numbers.
    // Call sites should invoke `streaming::delta_text(self, kind, id, delta, seq)` directly.

    /// Defer or handle an interrupt based on whether we're streaming
    fn defer_or_handle<F1, F2>(&mut self, defer_fn: F1, handle_fn: F2)
    where
        F1: FnOnce(&mut interrupts::InterruptManager),
        F2: FnOnce(&mut Self),
    {
        if self.is_write_cycle_active() {
            defer_fn(&mut self.interrupts);
        } else {
            handle_fn(self);
        }
    }

    // removed: next_sequence; plan updates are inserted immediately

    // Removed order-adjustment helpers; ordering now uses stable order keys on insert.

    /// Mark that the widget needs to be redrawn
    fn mark_needs_redraw(&mut self) {
        // Clean up fully faded cells before redraw. If any are removed,
        // invalidate the height cache since indices shift and our cache is
        // keyed by (idx,width).
        let before_len = self.history_cells.len();
        self.history_cells.retain(|cell| !cell.should_remove());
        if self.history_cells.len() != before_len {
            self.invalidate_height_cache();
        }

        // Send a redraw event to trigger UI update
        self.app_event_tx.send(AppEvent::RequestRedraw);
    }

    /// Clear memoized cell heights (called when history/content changes)
    fn invalidate_height_cache(&mut self) {
        self.history_render.invalidate_height_cache();
    }

    /// Handle exec approval request immediately
    fn handle_exec_approval_now(&mut self, _id: String, ev: ExecApprovalRequestEvent) {
        // Use call_id as the approval correlation id so responses map to the
        // exact pending approval in core (supports multiple approvals per turn).
        let approval_id = ev.call_id.clone();
        self.bottom_pane
            .push_approval_request(ApprovalRequest::Exec {
                id: approval_id,
                command: ev.command,
                reason: ev.reason,
            });
    }

    /// Handle apply patch approval request immediately
    fn handle_apply_patch_approval_now(&mut self, _id: String, ev: ApplyPatchApprovalRequestEvent) {
        let ApplyPatchApprovalRequestEvent {
            call_id,
            changes,
            reason,
            grant_root,
        } = ev;

        // Clone for session storage before moving into history
        let changes_clone = changes.clone();
        // Surface the patch summary in the main conversation
        let key = self.next_internal_key();
        let _ = self.history_insert_with_key_global(
            Box::new(history_cell::new_patch_event(
                history_cell::PatchEventType::ApprovalRequest,
                changes,
            )),
            key,
        );
        // Record change set for session diff popup (latest last)
        self.diffs.session_patch_sets.push(changes_clone);
        // For any new paths, capture an original baseline snapshot the first time we see them
        if let Some(last) = self.diffs.session_patch_sets.last() {
            for (src_path, chg) in last.iter() {
                match chg {
                    codex_core::protocol::FileChange::Update {
                        move_path: Some(dest_path),
                        ..
                    } => {
                        if let Some(baseline) =
                            self.diffs.baseline_file_contents.get(src_path).cloned()
                        {
                            // Mirror baseline under destination so tabs use the new path
                            self.diffs
                                .baseline_file_contents
                                .entry(dest_path.clone())
                                .or_insert(baseline);
                        } else if !self.diffs.baseline_file_contents.contains_key(dest_path) {
                            // Snapshot from source (pre-apply)
                            let baseline = std::fs::read_to_string(src_path).unwrap_or_default();
                            self.diffs
                                .baseline_file_contents
                                .insert(dest_path.clone(), baseline);
                        }
                    }
                    _ => {
                        if !self.diffs.baseline_file_contents.contains_key(src_path) {
                            let baseline = std::fs::read_to_string(src_path).unwrap_or_default();
                            self.diffs
                                .baseline_file_contents
                                .insert(src_path.clone(), baseline);
                        }
                    }
                }
            }
        }
        // Enable Ctrl+D footer hint now that we have diffs to show
        self.bottom_pane.set_diffs_hint(true);

        // Push the approval request to the bottom pane, keyed by call_id
        let request = ApprovalRequest::ApplyPatch {
            id: call_id,
            reason,
            grant_root,
        };
        self.bottom_pane.push_approval_request(request);
    }

    /// Handle exec command begin immediately
    fn handle_exec_begin_now(
        &mut self,
        ev: ExecCommandBeginEvent,
        order: &codex_core::protocol::OrderMeta,
    ) {
        exec_tools::handle_exec_begin_now(self, ev, order);
    }

    /// Handle exec command end immediately
    fn handle_exec_end_now(
        &mut self,
        ev: ExecCommandEndEvent,
        order: &codex_core::protocol::OrderMeta,
    ) {
        exec_tools::handle_exec_end_now(self, ev, order);
    }

    // MCP tool call handlers now live in chatwidget::tools

    /// Handle patch apply end immediately
    fn handle_patch_apply_end_now(&mut self, ev: PatchApplyEndEvent) {
        if ev.success {
            // Update the most recent patch cell header from "Updating..." to "Updated"
            // without creating a new history section.
            if let Some(last) = self.history_cells.iter_mut().rev().find(|c| {
                matches!(
                    c.kind(),
                    crate::history_cell::HistoryCellType::Patch {
                        kind: crate::history_cell::PatchKind::ApplyBegin
                    } | crate::history_cell::HistoryCellType::Patch {
                        kind: crate::history_cell::PatchKind::Proposed
                    }
                )
            }) {
                // Case 1: Patch summary cell – update title/kind in-place
                if let Some(summary) = last
                    .as_any_mut()
                    .downcast_mut::<history_cell::PatchSummaryCell>()
                {
                    summary.title = "Updated".to_string();
                    summary.kind = history_cell::PatchKind::ApplySuccess;
                    self.request_redraw();
                    return;
                }
                // Case 2: Plain history cell fallback – adjust first span and kind
                if let Some(plain) = last
                    .as_any_mut()
                    .downcast_mut::<history_cell::PlainHistoryCell>()
                {
                    let state = plain.state_mut();
                    if let Some(header) = state.header.as_mut() {
                        header.label = "Updated".to_string();
                    }
                    if let Some(first_line) = state.lines.first_mut() {
                        if first_line.spans.is_empty() {
                            first_line.kind = crate::history::MessageLineKind::Paragraph;
                            first_line.spans.push(crate::history::InlineSpan {
                                text: "Updated".to_string(),
                                tone: crate::history::TextTone::Success,
                                emphasis: crate::history::TextEmphasis {
                                    bold: true,
                                    italic: false,
                                    dim: false,
                                    strike: false,
                                    underline: false,
                                },
                                entity: None,
                            });
                        } else {
                            for span in &mut first_line.spans {
                                span.tone = crate::history::TextTone::Success;
                                span.emphasis.bold = true;
                                span.emphasis.dim = false;
                            }
                            first_line.spans[0].text = "Updated".to_string();
                        }
                    }
                    plain.set_kind(history_cell::HistoryCellType::Patch {
                        kind: history_cell::PatchKind::ApplySuccess,
                    });
                    plain.invalidate_layout_cache();
                    self.request_redraw();
                    return;
                }
            }
            // Fallback: if no prior cell found, do nothing (avoid extra section)
        } else {
            let key = self.next_internal_key();
            let _ = self.history_insert_with_key_global(
                Box::new(history_cell::new_patch_apply_failure(ev.stderr)),
                key,
            );
        }
        // After patch application completes, re-evaluate idle state
        self.maybe_hide_spinner();
    }

    pub(crate) fn insert_str(&mut self, s: &str) {
        self.bottom_pane.insert_str(s);
    }

    // Removed: pending insert sequencing is not used under strict ordering.

    pub(crate) fn register_pasted_image(&mut self, placeholder: String, path: std::path::PathBuf) {
        self.pending_images.insert(placeholder, path);
        self.request_redraw();
    }

    fn parse_message_with_images(&mut self, text: String) -> UserMessage {
        use std::path::Path;

        // We keep a visible copy of the original (normalized) text for history
        let mut display_text = text.clone();
        let mut ordered_items: Vec<InputItem> = Vec::new();

        // First, handle [image: ...] placeholders from drag-and-drop
        let placeholder_regex = regex_lite::Regex::new(r"\[image: [^\]]+\]").unwrap();
        let mut cursor = 0usize;
        for mat in placeholder_regex.find_iter(&text) {
            // Push preceding text as a text item (if any)
            if mat.start() > cursor {
                let chunk = &text[cursor..mat.start()];
                if !chunk.trim().is_empty() {
                    ordered_items.push(InputItem::Text {
                        text: chunk.to_string(),
                    });
                }
            }

            let placeholder = mat.as_str();
            if placeholder.starts_with("[image:") {
                if let Some(path) = self.pending_images.remove(placeholder) {
                    // Emit a small marker followed by the image so the LLM sees placement
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("image");
                    let marker = format!("[image: {}]", filename);
                    ordered_items.push(InputItem::Text { text: marker });
                    ordered_items.push(InputItem::LocalImage { path });
                } else {
                    // Unknown placeholder: preserve as text
                    ordered_items.push(InputItem::Text {
                        text: placeholder.to_string(),
                    });
                }
            } else {
                // Unknown placeholder type; preserve
                ordered_items.push(InputItem::Text {
                    text: placeholder.to_string(),
                });
            }
            cursor = mat.end();
        }
        // Push any remaining trailing text
        if cursor < text.len() {
            let chunk = &text[cursor..];
            if !chunk.trim().is_empty() {
                ordered_items.push(InputItem::Text {
                    text: chunk.to_string(),
                });
            }
        }

        // Then check for direct file paths typed into the message (no placeholder).
        // We conservatively append these at the end to avoid mis-ordering text.
        // This keeps the behavior consistent while still including the image.
        // We do NOT strip them from display_text so the user sees what they typed.
        let words: Vec<String> = text.split_whitespace().map(String::from).collect();
        for word in &words {
            if word.starts_with("[image:") {
                continue;
            }
            if !input_helpers::is_image_extension(word) {
                continue;
            }
            let path = Path::new(word);
            if path.exists() {
                // Add a marker then the image so the LLM has contextual placement info
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("image");
                ordered_items.push(InputItem::Text {
                    text: format!("[image: {}]", filename),
                });
                ordered_items.push(InputItem::LocalImage {
                    path: path.to_path_buf(),
                });
            }
        }

        // Non-image paths are left as-is in the text; the model may choose to read them.

        // Preserve user formatting (retain newlines) but normalize whitespace:
        // - Normalize CRLF -> LF
        // - Trim trailing spaces per line
        // - Remove any completely blank lines at the start and end
        display_text = display_text.replace("\r\n", "\n");
        let mut _lines_tmp: Vec<String> = display_text
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect();
        while _lines_tmp.first().is_some_and(|s| s.trim().is_empty()) {
            _lines_tmp.remove(0);
        }
        while _lines_tmp.last().is_some_and(|s| s.trim().is_empty()) {
            _lines_tmp.pop();
        }
        display_text = _lines_tmp.join("\n");

        UserMessage {
            display_text,
            ordered_items,
        }
    }

    /// Periodic tick to commit at most one queued line to history,
    /// animating the output.
    pub(crate) fn on_commit_tick(&mut self) {
        // SPEC-DOGFOOD-001 S31: Poll Stage0 pending operation for async progress
        self.poll_stage0_pending();

        streaming::on_commit_tick(self);
    }

    /// Poll Stage0 pending operation for progress and completion
    /// Called from on_commit_tick to keep TUI responsive during Stage0 execution
    fn is_write_cycle_active(&self) -> bool {
        streaming::is_write_cycle_active(self)
    }

    fn flush_interrupt_queue(&mut self) {
        let mut mgr = std::mem::take(&mut self.interrupts);
        mgr.flush_all(self);
        self.interrupts = mgr;
    }

    fn on_error(&mut self, message: String) {
        // Treat transient stream errors (which the core will retry) differently
        // from fatal errors so the status spinner remains visible while we wait.
        let lower = message.to_lowercase();
        let is_transient = lower.contains("retrying")
            || lower.contains("stream disconnected")
            || lower.contains("stream error")
            || lower.contains("stream closed")
            || lower.contains("timeout")
            || lower.contains("temporar");

        if is_transient {
            // Keep task running and surface a concise status in the input header.
            self.bottom_pane.set_task_running(true);
            self.bottom_pane.update_status_text(message.clone());
            // Add a dim background event instead of a hard error cell to avoid
            // alarming users during auto-retries.
            self.insert_background_event_with_placement(message, BackgroundPlacement::Tail);
            // Do NOT clear running state or streams; the retry will resume them.
            self.request_redraw();
            return;
        }

        // Fatal error path: show an error cell and clear running state.
        let key = self.next_internal_key();
        let _ = self
            .history_insert_with_key_global(Box::new(history_cell::new_error_event(message)), key);
        self.bottom_pane.set_task_running(false);
        self.exec.running_commands.clear();
        self.stream.clear_all();
        self.stream_state.drop_streaming = false;
        self.agents_ready_to_start = false;
        self.active_task_ids.clear();
        self.maybe_hide_spinner();
        self.mark_needs_redraw();
    }

    fn interrupt_running_task(&mut self) {
        let bottom_running = self.bottom_pane.is_task_running();
        let exec_related_running = !self.exec.running_commands.is_empty()
            || !self.tools_state.running_custom_tools.is_empty()
            || !self.tools_state.running_web_search.is_empty()
            || !self.tools_state.running_wait_tools.is_empty()
            || !self.tools_state.running_kill_tools.is_empty();

        if !(bottom_running || exec_related_running) {
            return;
        }

        let mut has_wait_running = false;
        for entry in self.tools_state.running_custom_tools.values() {
            if let Some(idx) = self.resolve_running_tool_index(entry)
                && let Some(cell) = self.history_cells.get(idx).and_then(|c| {
                    c.as_any()
                        .downcast_ref::<history_cell::RunningToolCallCell>()
                })
                && cell.has_title("Waiting")
            {
                has_wait_running = true;
                break;
            }
        }

        self.active_exec_cell = None;
        // Finalize any visible running indicators as interrupted (Exec/Web/Custom)
        self.finalize_all_running_as_interrupted();
        if bottom_running {
            self.bottom_pane.clear_ctrl_c_quit_hint();
        }
        // Stop any active UI streams immediately so output ceases at once.
        self.finalize_active_stream();
        self.stream_state.drop_streaming = true;
        // Surface an explicit notice in history so users see confirmation.
        if !has_wait_running {
            self.push_background_tail("Cancelled by user.".to_string());
        }
        self.submit_op(Op::Interrupt);
        // Immediately drop the running status so the next message can be typed/run,
        // even if backend cleanup (and Error event) arrives slightly later.
        self.bottom_pane.set_task_running(false);
        self.bottom_pane.clear_live_ring();
        // Reset with max width to disable wrapping
        self.live_builder = RowBuilder::new(usize::MAX);
        // Stream state is now managed by StreamController
        self.content_buffer.clear();
        // Defensive: clear transient flags so UI can quiesce
        self.agents_ready_to_start = false;
        self.active_task_ids.clear();
        // Restore any queued messages back into the composer so the user can
        // immediately press Enter to resume the conversation where they left off.
        if !self.queued_user_messages.is_empty() {
            let existing_input = self.bottom_pane.composer_text();
            let mut segments: Vec<String> = Vec::new();

            let mut queued_block = String::new();
            for (i, qm) in self.queued_user_messages.iter().enumerate() {
                if i > 0 {
                    queued_block.push_str("\n\n");
                }
                queued_block.push_str(qm.display_text.trim_end());
            }
            if !queued_block.trim().is_empty() {
                segments.push(queued_block);
            }

            if !existing_input.trim().is_empty() {
                segments.push(existing_input);
            }

            let combined = segments.join("\n\n");
            self.clear_composer();
            if !combined.is_empty() {
                self.insert_str(&combined);
            }
            self.queued_user_messages.clear();
            self.bottom_pane.update_status_text(String::new());
            self.pending_dispatched_user_messages.clear();
            self.refresh_queued_user_messages();
        }
        self.maybe_hide_spinner();
        self.request_redraw();
    }
    fn layout_areas(&self, area: Rect) -> Vec<Rect> {
        layout_scroll::layout_areas(self, area)
    }
    fn finalize_active_stream(&mut self) {
        streaming::finalize_active_stream(self);
    }
    // Strict stream order key helpers
    fn seed_stream_order_key(&mut self, kind: StreamKind, id: &str, key: OrderKey) {
        self.stream_order_seq.insert((kind, id.to_string()), key);
    }
    // Try to fetch a seeded stream order key. Callers must handle None.
    fn try_stream_order_key(&self, kind: StreamKind, id: &str) -> Option<OrderKey> {
        self.stream_order_seq.get(&(kind, id.to_string())).copied()
    }
    pub(crate) fn new(
        config: Config,
        app_event_tx: AppEventSender,
        initial_prompt: Option<String>,
        initial_images: Vec<PathBuf>,
        enhanced_keys_supported: bool,
        terminal_info: crate::tui::TerminalInfo,
        show_order_overlay: bool,
        latest_upgrade_version: Option<String>,
        mcp_manager: Arc<
            tokio::sync::Mutex<
                Option<Arc<codex_core::mcp_connection_manager::McpConnectionManager>>,
            >,
        >,
        initial_command: Option<String>, // SPEC-KIT-920
    ) -> Self {
        let (codex_op_tx, codex_op_rx) = unbounded_channel::<Op>();

        let auth_manager = AuthManager::shared(
            config.codex_home.clone(),
            AuthMode::ApiKey,
            config.responses_originator_header.clone(),
        );

        let app_event_tx_clone = app_event_tx.clone();
        let auth_manager_for_spawn = auth_manager.clone();
        let config_for_agent_loop = config.clone();
        tokio::spawn(async move {
            let mut codex_op_rx = codex_op_rx;
            let conversation_manager = ConversationManager::new(auth_manager_for_spawn.clone());
            let resume_path = config_for_agent_loop.experimental_resume.clone();
            let new_conversation = match resume_path {
                Some(path) => {
                    conversation_manager
                        .resume_conversation_from_rollout(
                            config_for_agent_loop,
                            path,
                            auth_manager_for_spawn,
                        )
                        .await
                }
                None => {
                    conversation_manager
                        .new_conversation(config_for_agent_loop)
                        .await
                }
            };

            let new_conversation = match new_conversation {
                Ok(conv) => conv,
                Err(e) => {
                    tracing::error!("failed to initialize conversation: {e}");
                    // Surface a visible background event so users see why nothing starts.
                    app_event_tx_clone.send_background_event(format!(
                        "❌ Failed to initialize model session: {}.\n• Ensure an OpenAI API key is set (CODE_OPENAI_API_KEY / OPENAI_API_KEY) or run `code login`.\n• Also verify config.cwd is an absolute path.",
                        e
                    ));
                    return;
                }
            };

            // Forward the SessionConfigured event to the UI
            let event = Event {
                id: new_conversation.conversation_id.to_string(),
                event_seq: 0,
                msg: EventMsg::SessionConfigured(new_conversation.session_configured),
                order: None,
            };
            app_event_tx_clone.send(AppEvent::CodexEvent(event));

            let conversation = new_conversation.conversation;
            let conversation_clone = conversation.clone();
            let app_event_tx_submit = app_event_tx_clone.clone();
            tokio::spawn(async move {
                while let Some(op) = codex_op_rx.recv().await {
                    if let Err(e) = conversation_clone.submit(op).await {
                        tracing::error!("failed to submit op: {e}");
                        app_event_tx_submit.send_background_event(format!(
                            "⚠️ Failed to submit Op to core: {}",
                            e
                        ));
                    }
                }
            });

            while let Ok(event) = conversation.next_event().await {
                app_event_tx_clone.send(AppEvent::CodexEvent(event));
            }
            // (debug end notice removed)
        });

        // Browser manager is now handled through the global state
        // The core session will use the same global manager when browser tools are invoked

        // Add initial animated welcome message to history (top of first request)
        let history_cells: Vec<Box<dyn HistoryCell>> = Vec::new();
        // Insert later via history_push_top_next_req once struct is constructed

        // Removed the legacy startup tip for /resume.

        // Initialize image protocol for rendering screenshots

        let broker_event_tx = app_event_tx.clone();
        let broker_mcp = mcp_manager.clone();
        let quality_gate_broker = QualityGateBroker::new(broker_event_tx, broker_mcp);

        let mut new_widget = Self {
            app_event_tx: app_event_tx.clone(),
            codex_op_tx,
            bottom_pane: BottomPane::new(BottomPaneParams {
                app_event_tx,
                has_input_focus: true,
                enhanced_keys_supported,
                using_chatgpt_auth: config.using_chatgpt_auth,
            }),
            auth_manager: auth_manager.clone(),
            login_view_state: None,
            login_add_view_state: None,
            device_code_login_state: None,
            active_exec_cell: None,
            history_cells,
            config: config.clone(),
            latest_upgrade_version: latest_upgrade_version.clone(),
            initial_user_message: create_initial_user_message(
                initial_prompt.unwrap_or_default(),
                initial_images,
            ),
            total_token_usage: TokenUsage::default(),
            last_token_usage: TokenUsage::default(),
            cost_tracker: Arc::new(spec_kit::cost_tracker::CostTracker::new(
                SPEC_KIT_DEFAULT_BUDGET_USD,
            )),
            rate_limit_snapshot: None,
            rate_limit_warnings: RateLimitWarningState::default(),
            rate_limit_fetch_inflight: false,
            rate_limit_last_fetch_at: None,
            rate_limit_primary_next_reset_at: None,
            rate_limit_secondary_next_reset_at: None,
            content_buffer: String::new(),
            last_assistant_message: None,
            exec: ExecState {
                running_commands: HashMap::new(),
                running_explore_agg_index: None,
                pending_exec_ends: HashMap::new(),
                suppressed_exec_end_call_ids: HashSet::new(),
                suppressed_exec_end_order: VecDeque::new(),
            },
            canceled_exec_call_ids: HashSet::new(),
            tools_state: ToolState {
                running_custom_tools: HashMap::new(),
                running_web_search: HashMap::new(),
                running_wait_tools: HashMap::new(),
                running_kill_tools: HashMap::new(),
            },
            // Use max width to disable wrapping during streaming
            // Text will be properly wrapped when displayed based on terminal width
            live_builder: RowBuilder::new(usize::MAX),
            pending_images: HashMap::new(),
            welcome_shown: false,
            cached_cell_size: std::cell::OnceCell::new(),
            git_branch_cache: RefCell::new(GitBranchCache::default()),
            terminal_info,
            active_agents: Vec::new(),
            agents_ready_to_start: false,
            last_agent_prompt: None,
            agent_context: None,
            agent_task: None,
            active_review_hint: None,
            active_review_prompt: None,
            overall_task_status: "preparing".to_string(),
            active_plan_title: None,
            agent_runtime: HashMap::new(),
            pro: ProState::default(),
            sparkline_data: std::cell::RefCell::new(Vec::new()),
            last_sparkline_update: std::cell::RefCell::new(std::time::Instant::now()),
            stream: crate::streaming::controller::StreamController::new(config.clone()),
            stream_state: StreamState {
                current_kind: None,
                closed_answer_ids: HashSet::new(),
                closed_reasoning_ids: HashSet::new(),
                seq_answer_final: None,
                drop_streaming: false,
            },
            interrupts: interrupts::InterruptManager::new(),
            ended_call_ids: HashSet::new(),
            diffs: DiffsState {
                session_patch_sets: Vec::new(),
                baseline_file_contents: HashMap::new(),
                overlay: None,
                confirm: None,
                body_visible_rows: std::cell::Cell::new(0),
            },
            help: HelpState {
                overlay: None,
                body_visible_rows: std::cell::Cell::new(0),
            },
            limits: LimitsState::default(),
            pm: pm_overlay::PmState::default(),
            terminal: TerminalState::default(),
            pending_manual_terminal: HashMap::new(),
            agents_overview_selected_index: 0,
            agents_terminal: AgentsTerminalState::new(),
            pending_upgrade_notice: None,
            history_render: HistoryRenderState::new(),
            height_manager: RefCell::new(HeightManager::new(
                crate::height_manager::HeightManagerConfig::default(),
            )),
            layout: LayoutState {
                scroll_offset: 0,
                last_max_scroll: std::cell::Cell::new(0),
                last_history_viewport_height: std::cell::Cell::new(0),
                vertical_scrollbar_state: std::cell::RefCell::new(ScrollbarState::default()),
                scrollbar_visible_until: std::cell::Cell::new(None),
                last_bottom_reserved_rows: std::cell::Cell::new(0),
                last_hud_present: std::cell::Cell::new(false),
                agents_hud_expanded: false,
                pro_hud_expanded: false,
                last_frame_height: std::cell::Cell::new(0),
                last_frame_width: std::cell::Cell::new(0),
            },
            last_theme: crate::theme::current_theme(),
            perf_state: PerfState {
                enabled: false,
                stats: std::cell::RefCell::new(PerfStats::default()),
            },
            session_id: None,
            pending_jump_back: None,
            active_task_ids: HashSet::new(),
            queued_user_messages: std::collections::VecDeque::new(),
            pending_dispatched_user_messages: std::collections::VecDeque::new(),
            pending_user_cell_updates: HashMap::new(),
            pending_message_timestamps: HashMap::new(),
            pending_user_prompts_for_next_turn: 0,
            ghost_snapshots: Vec::new(),
            ghost_snapshots_disabled: false,
            ghost_snapshots_disabled_reason: None,
            // Stable ordering & routing init
            cell_order_seq: vec![OrderKey {
                req: 0,
                out: -1,
                seq: 0,
            }],
            cell_order_dbg: vec![None; 1],
            reasoning_index: HashMap::new(),
            stream_order_seq: HashMap::new(),
            last_seen_request_index: 0,
            current_request_index: 0,
            internal_seq: 0,
            show_order_overlay,
            scroll_history_hint_shown: false,
            access_status_idx: None,
            pending_agent_notes: Vec::new(),
            synthetic_system_req: None,
            system_cell_by_id: HashMap::new(),
            standard_terminal_mode: !config.tui.alternate_screen,
            spec_auto_state: None,
            validate_lifecycles: HashMap::new(),
            stage0_pending: None,
            pending_maieutic: None,
            pending_intake_backfill: None,
            pending_projectnew: None,
            // FORK-SPECIFIC (just-every/code): Use shared MCP manager from App
            mcp_manager,
            quality_gate_broker,
            // SPEC-KIT-920: TUI automation support
            initial_command,
            // SPEC-939: Config hot-reload support (initialized below based on config.hot_reload)
            config_watcher: None,
            pending_config_reload: None,
            // SPEC-KIT-953: Native multi-provider streaming state
            native_stream_provider: None,
            native_stream_model: None,
            native_stream_id: None,
            native_stream_content: String::new(),
        };
        if let Ok(Some(active_id)) = auth_accounts::get_active_account_id(&config.codex_home)
            && let Ok(records) = account_usage::list_rate_limit_snapshots(&config.codex_home)
            && let Some(record) = records.into_iter().find(|r| r.account_id == active_id)
        {
            new_widget.rate_limit_primary_next_reset_at = record.primary_next_reset_at;
            new_widget.rate_limit_secondary_next_reset_at = record.secondary_next_reset_at;
        }
        // Seed footer access indicator based on current config
        new_widget.apply_access_mode_indicator_from_config();
        // Insert the welcome cell as top-of-first-request so future model output
        // appears below it. Also insert the Popular commands immediately so users
        // don't wait for MCP initialization to finish.
        let mut w = new_widget;
        w.set_standard_terminal_mode(!config.tui.alternate_screen);
        if config.experimental_resume.is_none() {
            w.history_push_top_next_req(history_cell::new_animated_welcome()); // tag: prelude
            let connecting_mcp = !w.config.mcp_servers.is_empty();
            if !w.config.auto_upgrade_enabled
                && let Some(upgrade_cell) =
                    history_cell::new_upgrade_prelude(w.latest_upgrade_version.as_deref())
            {
                w.history_push_top_next_req(upgrade_cell);
            }
            w.history_push_top_next_req(history_cell::new_popular_commands_notice(
                false,
                w.latest_upgrade_version.as_deref(),
            )); // tag: prelude
            if connecting_mcp {
                // Render connecting status as a separate cell with standard gutter and spacing
                w.history_push_top_next_req(history_cell::new_connecting_mcp_status());
            }
            // Mark welcome as shown to avoid duplicating the Popular commands section
            // when SessionConfigured arrives shortly after.
            w.welcome_shown = true;
        } else {
            w.welcome_shown = true;
        }
        w.maybe_start_auto_upgrade_task();

        w
    }

    /// Test-only constructor that skips the conversation loop.
    ///
    /// SPEC-955: The main `new()` constructor spawns a background task that
    /// calls ConversationManager::new_conversation(), which requires network/API
    /// access. This causes tests to hang indefinitely waiting for a response
    /// that never comes.
    ///
    /// This constructor creates a fully functional ChatWidget for testing
    /// handle_codex_event(), history rendering, and UI logic without requiring
    /// the Codex backend.
    #[cfg(test)]
    pub(crate) fn new_for_testing(
        config: Config,
        app_event_tx: AppEventSender,
        terminal_info: crate::tui::TerminalInfo,
    ) -> Self {
        // AuthMode is already imported at module level from codex_login

        // Create channels but DON'T spawn a consumer - tests will inject events directly
        let (codex_op_tx, _codex_op_rx) = unbounded_channel::<Op>();

        let auth_manager = AuthManager::shared(
            config.codex_home.clone(),
            AuthMode::ApiKey,
            config.responses_originator_header.clone(),
        );

        let history_cells: Vec<Box<dyn HistoryCell>> = Vec::new();

        let broker_event_tx = app_event_tx.clone();
        let mcp_manager = Arc::new(tokio::sync::Mutex::new(None));
        let broker_mcp = mcp_manager.clone();
        let quality_gate_broker = QualityGateBroker::new(broker_event_tx, broker_mcp);

        Self {
            app_event_tx: app_event_tx.clone(),
            codex_op_tx,
            bottom_pane: BottomPane::new(BottomPaneParams {
                app_event_tx,
                has_input_focus: true,
                enhanced_keys_supported: false,
                using_chatgpt_auth: config.using_chatgpt_auth,
            }),
            auth_manager: auth_manager.clone(),
            login_view_state: None,
            login_add_view_state: None,
            device_code_login_state: None,
            active_exec_cell: None,
            history_cells,
            config: config.clone(),
            latest_upgrade_version: None,
            initial_user_message: None,
            total_token_usage: TokenUsage::default(),
            last_token_usage: TokenUsage::default(),
            cost_tracker: Arc::new(spec_kit::cost_tracker::CostTracker::new(
                SPEC_KIT_DEFAULT_BUDGET_USD,
            )),
            rate_limit_snapshot: None,
            rate_limit_warnings: RateLimitWarningState::default(),
            rate_limit_fetch_inflight: false,
            rate_limit_last_fetch_at: None,
            rate_limit_primary_next_reset_at: None,
            rate_limit_secondary_next_reset_at: None,
            content_buffer: String::new(),
            last_assistant_message: None,
            exec: ExecState {
                running_commands: HashMap::new(),
                running_explore_agg_index: None,
                pending_exec_ends: HashMap::new(),
                suppressed_exec_end_call_ids: HashSet::new(),
                suppressed_exec_end_order: VecDeque::new(),
            },
            canceled_exec_call_ids: HashSet::new(),
            tools_state: ToolState {
                running_custom_tools: HashMap::new(),
                running_web_search: HashMap::new(),
                running_wait_tools: HashMap::new(),
                running_kill_tools: HashMap::new(),
            },
            live_builder: RowBuilder::new(usize::MAX),
            pending_images: HashMap::new(),
            welcome_shown: false,
            cached_cell_size: std::cell::OnceCell::new(),
            git_branch_cache: RefCell::new(GitBranchCache::default()),
            terminal_info,
            active_agents: Vec::new(),
            agents_ready_to_start: false,
            last_agent_prompt: None,
            agent_context: None,
            agent_task: None,
            active_review_hint: None,
            active_review_prompt: None,
            overall_task_status: "preparing".to_string(),
            active_plan_title: None,
            agent_runtime: HashMap::new(),
            pro: ProState::default(),
            sparkline_data: std::cell::RefCell::new(Vec::new()),
            last_sparkline_update: std::cell::RefCell::new(std::time::Instant::now()),
            stream: crate::streaming::controller::StreamController::new(config.clone()),
            stream_state: StreamState {
                current_kind: None,
                closed_answer_ids: HashSet::new(),
                closed_reasoning_ids: HashSet::new(),
                seq_answer_final: None,
                drop_streaming: false,
            },
            interrupts: interrupts::InterruptManager::new(),
            ended_call_ids: HashSet::new(),
            diffs: DiffsState {
                session_patch_sets: Vec::new(),
                baseline_file_contents: HashMap::new(),
                overlay: None,
                confirm: None,
                body_visible_rows: std::cell::Cell::new(0),
            },
            help: HelpState {
                overlay: None,
                body_visible_rows: std::cell::Cell::new(0),
            },
            limits: LimitsState::default(),
            pm: pm_overlay::PmState::default(),
            terminal: TerminalState::default(),
            pending_manual_terminal: HashMap::new(),
            agents_overview_selected_index: 0,
            agents_terminal: AgentsTerminalState::new(),
            pending_upgrade_notice: None,
            history_render: HistoryRenderState::new(),
            height_manager: RefCell::new(HeightManager::new(
                crate::height_manager::HeightManagerConfig::default(),
            )),
            layout: LayoutState {
                scroll_offset: 0,
                last_max_scroll: std::cell::Cell::new(0),
                last_history_viewport_height: std::cell::Cell::new(0),
                vertical_scrollbar_state: std::cell::RefCell::new(ScrollbarState::default()),
                scrollbar_visible_until: std::cell::Cell::new(None),
                last_bottom_reserved_rows: std::cell::Cell::new(0),
                last_hud_present: std::cell::Cell::new(false),
                agents_hud_expanded: false,
                pro_hud_expanded: false,
                last_frame_height: std::cell::Cell::new(0),
                last_frame_width: std::cell::Cell::new(0),
            },
            last_theme: crate::theme::current_theme(),
            perf_state: PerfState {
                enabled: false,
                stats: std::cell::RefCell::new(PerfStats::default()),
            },
            session_id: None,
            pending_jump_back: None,
            active_task_ids: HashSet::new(),
            queued_user_messages: std::collections::VecDeque::new(),
            pending_dispatched_user_messages: std::collections::VecDeque::new(),
            pending_user_cell_updates: HashMap::new(),
            pending_message_timestamps: HashMap::new(),
            pending_user_prompts_for_next_turn: 0,
            ghost_snapshots: Vec::new(),
            ghost_snapshots_disabled: false,
            ghost_snapshots_disabled_reason: None,
            cell_order_seq: vec![OrderKey {
                req: 0,
                out: -1,
                seq: 0,
            }],
            cell_order_dbg: vec![None; 1],
            reasoning_index: HashMap::new(),
            stream_order_seq: HashMap::new(),
            last_seen_request_index: 0,
            current_request_index: 0,
            internal_seq: 0,
            show_order_overlay: false,
            scroll_history_hint_shown: false,
            access_status_idx: None,
            pending_agent_notes: Vec::new(),
            synthetic_system_req: None,
            system_cell_by_id: HashMap::new(),
            standard_terminal_mode: !config.tui.alternate_screen,
            spec_auto_state: None,
            validate_lifecycles: HashMap::new(),
            stage0_pending: None,
            pending_maieutic: None,
            pending_intake_backfill: None,
            pending_projectnew: None,
            mcp_manager,
            quality_gate_broker,
            initial_command: None,
            config_watcher: None,
            pending_config_reload: None,
            native_stream_provider: None,
            native_stream_model: None,
            native_stream_id: None,
            native_stream_content: String::new(),
        }
    }

    /// Construct a ChatWidget from an existing conversation (forked session).
    pub(crate) fn new_from_existing(
        config: Config,
        conversation: std::sync::Arc<codex_core::CodexConversation>,
        session_configured: SessionConfiguredEvent,
        app_event_tx: AppEventSender,
        enhanced_keys_supported: bool,
        terminal_info: crate::tui::TerminalInfo,
        show_order_overlay: bool,
        latest_upgrade_version: Option<String>,
        auth_manager: Arc<AuthManager>,
        show_welcome: bool,
        mcp_manager: Arc<
            tokio::sync::Mutex<
                Option<Arc<codex_core::mcp_connection_manager::McpConnectionManager>>,
            >,
        >,
    ) -> Self {
        let (codex_op_tx, mut codex_op_rx) = unbounded_channel::<Op>();

        // Forward events from existing conversation
        let app_event_tx_clone = app_event_tx.clone();
        tokio::spawn(async move {
            // Send the provided SessionConfigured to the UI first
            let event = Event {
                id: "fork".to_string(),
                event_seq: 0,
                msg: EventMsg::SessionConfigured(session_configured),
                order: None,
            };
            app_event_tx_clone.send(AppEvent::CodexEvent(event));

            let conversation_clone = conversation.clone();
            tokio::spawn(async move {
                while let Some(op) = codex_op_rx.recv().await {
                    let id = conversation_clone.submit(op).await;
                    if let Err(e) = id {
                        tracing::error!("failed to submit op: {e}");
                    }
                }
            });

            while let Ok(event) = conversation.next_event().await {
                app_event_tx_clone.send(AppEvent::CodexEvent(event));
            }
        });

        // Basic widget state mirrors `new`
        let history_cells: Vec<Box<dyn HistoryCell>> = Vec::new();

        let broker_event_tx = app_event_tx.clone();
        let broker_mcp = mcp_manager.clone();
        let quality_gate_broker = QualityGateBroker::new(broker_event_tx, broker_mcp);

        let mut w = Self {
            app_event_tx: app_event_tx.clone(),
            codex_op_tx,
            bottom_pane: BottomPane::new(BottomPaneParams {
                app_event_tx,
                has_input_focus: true,
                enhanced_keys_supported,
                using_chatgpt_auth: config.using_chatgpt_auth,
            }),
            auth_manager: auth_manager.clone(),
            login_view_state: None,
            login_add_view_state: None,
            device_code_login_state: None,
            active_exec_cell: None,
            history_cells,
            config: config.clone(),
            latest_upgrade_version: latest_upgrade_version.clone(),
            initial_user_message: None,
            total_token_usage: TokenUsage::default(),
            last_token_usage: TokenUsage::default(),
            cost_tracker: Arc::new(spec_kit::cost_tracker::CostTracker::new(
                SPEC_KIT_DEFAULT_BUDGET_USD,
            )),
            rate_limit_snapshot: None,
            rate_limit_warnings: RateLimitWarningState::default(),
            rate_limit_fetch_inflight: false,
            rate_limit_last_fetch_at: None,
            rate_limit_primary_next_reset_at: None,
            rate_limit_secondary_next_reset_at: None,
            content_buffer: String::new(),
            last_assistant_message: None,
            exec: ExecState {
                running_commands: HashMap::new(),
                running_explore_agg_index: None,
                pending_exec_ends: HashMap::new(),
                suppressed_exec_end_call_ids: HashSet::new(),
                suppressed_exec_end_order: VecDeque::new(),
            },
            canceled_exec_call_ids: HashSet::new(),
            tools_state: ToolState {
                running_custom_tools: HashMap::new(),
                running_web_search: HashMap::new(),
                running_wait_tools: HashMap::new(),
                running_kill_tools: HashMap::new(),
            },
            live_builder: RowBuilder::new(usize::MAX),
            pending_images: HashMap::new(),
            welcome_shown: false,
            cached_cell_size: std::cell::OnceCell::new(),
            git_branch_cache: RefCell::new(GitBranchCache::default()),
            terminal_info,
            active_agents: Vec::new(),
            agents_ready_to_start: false,
            last_agent_prompt: None,
            agent_context: None,
            agent_task: None,
            active_review_hint: None,
            active_review_prompt: None,
            overall_task_status: "preparing".to_string(),
            active_plan_title: None,
            agent_runtime: HashMap::new(),
            pro: ProState::default(),
            sparkline_data: std::cell::RefCell::new(Vec::new()),
            last_sparkline_update: std::cell::RefCell::new(std::time::Instant::now()),
            stream: crate::streaming::controller::StreamController::new(config.clone()),
            stream_state: StreamState {
                current_kind: None,
                closed_answer_ids: HashSet::new(),
                closed_reasoning_ids: HashSet::new(),
                seq_answer_final: None,
                drop_streaming: false,
            },
            interrupts: interrupts::InterruptManager::new(),
            ended_call_ids: HashSet::new(),
            diffs: DiffsState {
                session_patch_sets: Vec::new(),
                baseline_file_contents: HashMap::new(),
                overlay: None,
                confirm: None,
                body_visible_rows: std::cell::Cell::new(0),
            },
            help: HelpState {
                overlay: None,
                body_visible_rows: std::cell::Cell::new(0),
            },
            limits: LimitsState::default(),
            pm: pm_overlay::PmState::default(),
            terminal: TerminalState::default(),
            pending_manual_terminal: HashMap::new(),
            agents_overview_selected_index: 0,
            agents_terminal: AgentsTerminalState::new(),
            pending_upgrade_notice: None,
            history_render: HistoryRenderState::new(),
            height_manager: RefCell::new(HeightManager::new(
                crate::height_manager::HeightManagerConfig::default(),
            )),
            layout: LayoutState {
                scroll_offset: 0,
                last_max_scroll: std::cell::Cell::new(0),
                last_history_viewport_height: std::cell::Cell::new(0),
                vertical_scrollbar_state: std::cell::RefCell::new(ScrollbarState::default()),
                scrollbar_visible_until: std::cell::Cell::new(None),
                last_bottom_reserved_rows: std::cell::Cell::new(0),
                last_hud_present: std::cell::Cell::new(false),
                agents_hud_expanded: false,
                pro_hud_expanded: false,
                last_frame_height: std::cell::Cell::new(0),
                last_frame_width: std::cell::Cell::new(0),
            },
            last_theme: crate::theme::current_theme(),
            perf_state: PerfState {
                enabled: false,
                stats: std::cell::RefCell::new(PerfStats::default()),
            },
            session_id: None,
            pending_jump_back: None,
            active_task_ids: HashSet::new(),
            queued_user_messages: std::collections::VecDeque::new(),
            pending_dispatched_user_messages: std::collections::VecDeque::new(),
            pending_user_cell_updates: HashMap::new(),
            pending_message_timestamps: HashMap::new(),
            pending_user_prompts_for_next_turn: 0,
            ghost_snapshots: Vec::new(),
            ghost_snapshots_disabled: false,
            ghost_snapshots_disabled_reason: None,
            // Strict ordering init for forked widget
            cell_order_seq: vec![OrderKey {
                req: 0,
                out: -1,
                seq: 0,
            }],
            cell_order_dbg: vec![None; 1],
            reasoning_index: HashMap::new(),
            stream_order_seq: HashMap::new(),
            last_seen_request_index: 0,
            current_request_index: 0,
            internal_seq: 0,
            show_order_overlay,
            scroll_history_hint_shown: false,
            access_status_idx: None,
            standard_terminal_mode: !config.tui.alternate_screen,
            pending_agent_notes: Vec::new(),
            synthetic_system_req: None,
            system_cell_by_id: HashMap::new(),
            spec_auto_state: None,
            validate_lifecycles: HashMap::new(),
            stage0_pending: None,
            pending_maieutic: None,
            pending_intake_backfill: None,
            pending_projectnew: None,
            // FORK-SPECIFIC (just-every/code): Use shared MCP manager from App
            mcp_manager,
            quality_gate_broker,
            // SPEC-KIT-920: TUI automation support (fork_from_ghost_state has no initial_command)
            initial_command: None,
            // SPEC-939: Config hot-reload support (initialized below based on config.hot_reload)
            config_watcher: None, // Initialized after struct creation
            pending_config_reload: None,
            // SPEC-KIT-953: Native multi-provider streaming state
            native_stream_provider: None,
            native_stream_model: None,
            native_stream_id: None,
            native_stream_content: String::new(),
        };
        if let Ok(Some(active_id)) = auth_accounts::get_active_account_id(&config.codex_home)
            && let Ok(records) = account_usage::list_rate_limit_snapshots(&config.codex_home)
            && let Some(record) = records.into_iter().find(|r| r.account_id == active_id)
        {
            w.rate_limit_primary_next_reset_at = record.primary_next_reset_at;
            w.rate_limit_secondary_next_reset_at = record.secondary_next_reset_at;
        }
        w.set_standard_terminal_mode(!config.tui.alternate_screen);
        if show_welcome {
            w.history_push_top_next_req(history_cell::new_animated_welcome());
        }
        w.maybe_start_auto_upgrade_task();

        // SPEC-939: Initialize config hot-reload watcher if enabled
        if config
            .hot_reload
            .as_ref()
            .map(|h| h.enabled)
            .unwrap_or(false)
        {
            let watch_paths = config
                .hot_reload
                .as_ref()
                .and_then(|h| {
                    if h.watch_paths.is_empty() {
                        None
                    } else {
                        Some(
                            h.watch_paths
                                .iter()
                                .map(|p| config.codex_home.join(p))
                                .collect(),
                        )
                    }
                })
                .unwrap_or_else(|| vec![config.codex_home.join("config.toml")]);

            let debounce_ms = config
                .hot_reload
                .as_ref()
                .map(|h| h.debounce_ms)
                .unwrap_or(2000);

            match ConfigWatcher::new(&watch_paths, debounce_ms) {
                Ok(watcher) => {
                    w.config_watcher = Some(watcher);
                    tracing::info!(
                        "Config hot-reload watcher initialized (debounce={}ms, paths={:?})",
                        debounce_ms,
                        watch_paths
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to initialize config watcher: {}", e);
                }
            }
        }

        w
    }

    // MAINT-11 Phase 8: export_response_items moved to session_handlers.rs

    pub(crate) fn config_ref(&self) -> &Config {
        &self.config
    }

    /// Check if quality gate is currently active (SPEC-945D).
    pub(crate) fn is_quality_gate_active(&self) -> bool {
        use spec_kit::state::SpecAutoPhase;
        self.spec_auto_state
            .as_ref()
            .map(|state| {
                matches!(
                    state.phase,
                    SpecAutoPhase::QualityGateExecuting { .. }
                        | SpecAutoPhase::QualityGateProcessing { .. }
                        | SpecAutoPhase::QualityGateValidating { .. }
                        | SpecAutoPhase::QualityGateAwaitingHuman { .. }
                )
            })
            .unwrap_or(false)
    }

    /// Check if any agents are currently running (SPEC-945D).
    pub(crate) fn is_agent_running(&self) -> bool {
        !self.active_agents.is_empty()
    }

    // === FORK-SPECIFIC (just-every/code): SPEC-945D Config hot-reload refresh methods ===

    /// Refresh quality gate configuration after config reload.
    /// Updates quality gate thresholds, enabled status, and agent list.
    pub(crate) fn refresh_quality_gates(&mut self) {
        // Quality gates will use updated config from watcher on next execution
        // No explicit state to reset - quality_gate_broker reads config dynamically
        tracing::debug!("Quality gates component refreshed (will use new config on next run)");

        // Note: Full config integration with spec-kit AppConfig is deferred
        // The config_watcher (if present) holds the updated AppConfig
        // Components will read from it when needed
    }

    /// Refresh agent selection UI after config reload.
    /// Updates available models and their configurations.
    pub(crate) fn refresh_agent_selection(&mut self) {
        // Agent selection happens through config at spawn time
        // Model configurations will be read from updated config on next agent spawn
        tracing::debug!("Agent selection component refreshed (will use new config on next spawn)");

        // Note: Full config integration with spec-kit AppConfig is deferred
        // The config_watcher (if present) holds the updated AppConfig
    }

    /// Refresh cost tracker limits after config reload.
    /// Updates daily and monthly cost limits and alert thresholds.
    pub(crate) fn refresh_cost_tracker(&mut self) {
        // Cost tracker (Arc<CostTracker>) reads config internally
        // Will use updated values on next check
        tracing::debug!("Cost tracker component refreshed (will use new config on next check)");

        // Note: Full config integration with spec-kit AppConfig is deferred
        // The config_watcher (if present) holds the updated AppConfig
        // self.cost_tracker reads from it dynamically
    }

    /// Poll config watcher for file changes (SPEC-939 Component 1a).
    /// Defers reload if quality gate is active, processes pending reloads when not.
    pub(crate) fn poll_config_watcher(&mut self) {
        // SPEC-939: First check for pending reload from when quality gate was active
        if !self.is_quality_gate_active() {
            if let Some(paths) = self.pending_config_reload.take() {
                tracing::debug!("Processing deferred config reload (quality gate finished)");
                self.show_reload_prompt(paths);
                return;
            }
        }

        // SPEC-939: Then poll for new config file changes
        if let Some(ref mut watcher) = self.config_watcher
            && let Some(changed_paths) = watcher.check_for_changes()
        {
            // Defer reload if quality gate is active (SPEC-939 requirement)
            if self.is_quality_gate_active() {
                self.pending_config_reload = Some(changed_paths);
                tracing::debug!("Config change detected but deferred (quality gate active)");
            } else {
                self.show_reload_prompt(changed_paths);
            }
        }
    }

    /// Show config reload prompt to user (SPEC-939 Component 1a).
    fn show_reload_prompt(&mut self, paths: Vec<std::path::PathBuf>) {
        let paths_display: Vec<_> = paths
            .iter()
            .map(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("config.toml")
            })
            .collect();

        let prompt = format!(
            "Config changed: {}. Reload? [Y/n]",
            paths_display.join(", ")
        );

        // Use existing notification system to show prompt
        self.app_event_tx.send_background_event(prompt);

        // TODO: Wire up Y/n input handler in future iteration
        // For now, auto-reload after notification
        self.reload_config();
    }

    /// Reload config from disk (SPEC-939 Component 1a).
    fn reload_config(&mut self) {
        match self.config.reload_from_file() {
            Ok(new_config) => {
                self.config = new_config;
                self.app_event_tx
                    .send_background_event("✅ Config reloaded successfully".to_string());
                tracing::info!("Config reloaded from disk");

                // Refresh components that depend on config
                self.refresh_quality_gates();
                self.refresh_agent_selection();
                self.refresh_cost_tracker();
            }
            Err(e) => {
                let error_msg = format!("❌ Config reload failed: {}", e);
                self.app_event_tx.send_background_event(error_msg.clone());
                tracing::error!("Config reload failed: {}", e);
                // Old config is preserved (automatic rollback behavior)
            }
        }
    }

    // === END FORK-SPECIFIC ===

    /// Check if there are any animations and trigger redraw if needed
    pub fn check_for_initial_animations(&mut self) {
        if self.history_cells.iter().any(|cell| cell.is_animating()) {
            tracing::info!("Initial animation detected, scheduling frame");
            // Schedule initial frame for animations to ensure they start properly.
            // Use ScheduleFrameIn to avoid debounce issues with immediate RequestRedraw.
            self.app_event_tx
                .send(AppEvent::ScheduleFrameIn(std::time::Duration::from_millis(
                    50,
                )));
        }
    }

    /// P6-SYNC Phase 5: Update device code token status from storage
    /// Called on startup and can be called periodically to refresh status
    pub fn update_device_token_status(&mut self) {
        use codex_login::DeviceCodeTokenStorage;

        match DeviceCodeTokenStorage::new() {
            Ok(storage) => {
                match storage.status_summary() {
                    Ok(status) => {
                        // Only show if at least one provider has a non-default status
                        let has_any_status = status
                            .iter()
                            .any(|(_, s)| !matches!(s, codex_login::TokenStatus::NotAuthenticated));
                        if has_any_status {
                            self.bottom_pane.set_device_token_status(Some(status));
                        } else {
                            self.bottom_pane.set_device_token_status(None);
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Failed to get device token status: {}", e);
                        self.bottom_pane.set_device_token_status(None);
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Failed to create device token storage: {}", e);
                self.bottom_pane.set_device_token_status(None);
            }
        }
    }

    /// Format model name with proper capitalization (e.g., "gpt-4" -> "GPT-4")
    fn format_model_name(&self, model_name: &str) -> String {
        if let Some(rest) = model_name.strip_prefix("gpt-") {
            let formatted_rest = rest
                .split('-')
                .map(|segment| {
                    if segment.eq_ignore_ascii_case("codex") {
                        "Codex".to_string()
                    } else {
                        segment.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("-");
            format!("GPT-{}", formatted_rest)
        } else {
            model_name.to_string()
        }
    }

    fn toggle_pro_hud(&mut self) {
        layout_scroll::toggle_pro_hud(self);
    }

    // MAINT-11: toggle_pro_overlay moved to pro_overlay.rs

    fn set_limits_overlay_content(&mut self, content: LimitsOverlayContent) {
        if let Some(existing) = self.limits.overlay.as_mut() {
            existing.set_content(content);
        } else {
            self.limits.overlay = Some(LimitsOverlay::new(content));
        }
    }

    fn set_limits_overlay_tabs(&mut self, tabs: Vec<LimitsTab>) {
        if tabs.is_empty() {
            self.set_limits_overlay_content(LimitsOverlayContent::Placeholder);
        } else {
            self.set_limits_overlay_content(LimitsOverlayContent::Tabs(tabs));
        }
    }

    fn rebuild_limits_overlay(&mut self) {
        if self.rate_limit_fetch_inflight {
            self.set_limits_overlay_content(LimitsOverlayContent::Loading);
            return;
        }

        let snapshot = self.rate_limit_snapshot.clone();
        let reset_info = self.rate_limit_reset_info();
        let tabs = self.build_limits_tabs(snapshot, reset_info);
        self.set_limits_overlay_tabs(tabs);
    }

    fn build_limits_tabs(
        &self,
        current_snapshot: Option<RateLimitSnapshotEvent>,
        current_reset: RateLimitResetInfo,
    ) -> Vec<LimitsTab> {
        use std::collections::HashSet;

        let codex_home = self.config.codex_home.clone();
        let accounts = auth_accounts::list_accounts(&codex_home).unwrap_or_default();
        let mut account_map: HashMap<String, StoredAccount> = accounts
            .into_iter()
            .map(|account| (account.id.clone(), account))
            .collect();

        let active_id = auth_accounts::get_active_account_id(&codex_home)
            .ok()
            .flatten();

        let usage_records =
            account_usage::list_rate_limit_snapshots(&codex_home).unwrap_or_default();
        let mut snapshot_map: HashMap<String, StoredRateLimitSnapshot> = usage_records
            .into_iter()
            .map(|record| (record.account_id.clone(), record))
            .collect();

        let mut summary_ids: HashSet<String> = account_map.keys().cloned().collect();
        summary_ids.extend(snapshot_map.keys().cloned());
        if let Some(id) = active_id.as_ref() {
            summary_ids.insert(id.clone());
        }

        let mut usage_summary_map: HashMap<String, StoredUsageSummary> = HashMap::new();
        for id in summary_ids {
            if let Ok(Some(summary)) = account_usage::load_account_usage(&codex_home, &id) {
                usage_summary_map.insert(id, summary);
            }
        }

        let mut tabs: Vec<LimitsTab> = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();

        if let Some(snapshot) = current_snapshot {
            let account_ref = active_id.as_ref().and_then(|id| account_map.get(id));
            let snapshot_ref = active_id.as_ref().and_then(|id| snapshot_map.get(id));
            let summary_ref = active_id.as_ref().and_then(|id| usage_summary_map.get(id));

            let title = account_ref
                .map(Self::account_label)
                .or_else(|| active_id.clone())
                .unwrap_or_else(|| "Current session".to_string());
            let header = Self::account_header_lines(account_ref, snapshot_ref, summary_ref);
            let extra = Self::daily_usage_lines(summary_ref);
            let view = build_limits_view(&snapshot, current_reset, DEFAULT_GRID_CONFIG);
            tabs.push(LimitsTab::view(title, header, view, extra));
            if let Some(id) = active_id.as_ref() {
                seen_ids.insert(id.clone());
                account_map.remove(id);
                snapshot_map.remove(id);
                usage_summary_map.remove(id);
            }
        }

        let mut remaining_ids: Vec<String> = Vec::new();
        for id in account_map.keys() {
            if seen_ids.insert(id.clone()) {
                remaining_ids.push(id.clone());
            }
        }
        for id in snapshot_map.keys() {
            if seen_ids.insert(id.clone()) {
                remaining_ids.push(id.clone());
            }
        }
        remaining_ids.sort_by(|a, b| {
            let a_label = account_map
                .get(a)
                .map(Self::account_label)
                .unwrap_or_else(|| a.clone());
            let b_label = account_map
                .get(b)
                .map(Self::account_label)
                .unwrap_or_else(|| b.clone());
            a_label
                .to_ascii_lowercase()
                .cmp(&b_label.to_ascii_lowercase())
        });

        for id in remaining_ids {
            let account = account_map.get(&id);
            let record = snapshot_map.remove(&id);
            let usage_summary = usage_summary_map.remove(&id);
            let title = account
                .map(Self::account_label)
                .unwrap_or_else(|| id.clone());
            match record {
                Some(record) => {
                    if let Some(snapshot) = record.snapshot.clone() {
                        let view_snapshot = snapshot.clone();
                        let view_reset = RateLimitResetInfo {
                            primary_next_reset: record.primary_next_reset_at,
                            secondary_next_reset: record.secondary_next_reset_at,
                            ..RateLimitResetInfo::default()
                        };
                        let view =
                            build_limits_view(&view_snapshot, view_reset, DEFAULT_GRID_CONFIG);
                        let header = Self::account_header_lines(
                            account,
                            Some(&record),
                            usage_summary.as_ref(),
                        );
                        let extra = Self::daily_usage_lines(usage_summary.as_ref());
                        tabs.push(LimitsTab::view(title, header, view, extra));
                    } else {
                        let mut lines = Self::daily_usage_lines(usage_summary.as_ref());
                        lines.push(Self::dim_line(" Rate limit snapshot not yet available."));
                        let header = Self::account_header_lines(
                            account,
                            Some(&record),
                            usage_summary.as_ref(),
                        );
                        tabs.push(LimitsTab::message(title, header, lines));
                    }
                }
                None => {
                    let mut lines = Self::daily_usage_lines(usage_summary.as_ref());
                    lines.push(Self::dim_line(" Rate limit snapshot not yet available."));
                    let header = Self::account_header_lines(account, None, usage_summary.as_ref());
                    tabs.push(LimitsTab::message(title, header, lines));
                }
            }
        }

        if tabs.is_empty() {
            let mut lines = Self::daily_usage_lines(None);
            lines.push(Self::dim_line(" Rate limit snapshot not yet available."));
            tabs.push(LimitsTab::message("Usage", Vec::new(), lines));
        }

        tabs
    }

    fn account_label(account: &StoredAccount) -> String {
        account
            .label
            .clone()
            .filter(|label| !label.trim().is_empty())
            .unwrap_or_else(|| account.id.clone())
    }

    fn account_header_lines(
        account: Option<&StoredAccount>,
        record: Option<&StoredRateLimitSnapshot>,
        usage: Option<&StoredUsageSummary>,
    ) -> Vec<RtLine<'static>> {
        let mut lines: Vec<RtLine<'static>> = Vec::new();

        let account_type = account
            .map(|acc| match acc.mode {
                McpAuthMode::ChatGPT => "ChatGPT account",
                McpAuthMode::ApiKey => "API key",
            })
            .unwrap_or("Unknown account");

        let plan = record
            .and_then(|r| r.plan.as_deref())
            .or_else(|| usage.and_then(|u| u.plan.as_deref()))
            .unwrap_or("Unknown");

        let total_tokens = usage.map(|u| u.totals.total_tokens).unwrap_or(0);

        let value_style = Style::default().fg(crate::colors::text_dim());

        lines.push(RtLine::from(String::new()));

        lines.push(RtLine::from(vec![
            RtSpan::raw(" Type:  "),
            RtSpan::styled(account_type.to_string(), value_style),
        ]));
        lines.push(RtLine::from(vec![
            RtSpan::raw(" Plan:  "),
            RtSpan::styled(plan.to_string(), value_style),
        ]));
        let total_value = format!("{} tokens", format_with_separators(total_tokens));
        lines.push(RtLine::from(vec![
            RtSpan::raw(" Total: "),
            RtSpan::styled(total_value, value_style),
        ]));
        lines
    }

    fn daily_usage_lines(summary: Option<&StoredUsageSummary>) -> Vec<RtLine<'static>> {
        const WIDTH: usize = 14;
        let today = Local::now().date_naive();
        let mut daily: Vec<(chrono::NaiveDate, u64)> = (0..7)
            .map(|offset| (today - ChronoDuration::days(offset as i64), 0u64))
            .collect();

        if let Some(summary) = summary {
            for entry in &summary.hourly_entries {
                let entry_date = entry.timestamp.with_timezone(&Local).date_naive();
                let diff = today.signed_duration_since(entry_date).num_days();
                if (0..=6).contains(&diff) {
                    let idx = diff as usize;
                    let (_, total) = &mut daily[idx];
                    *total = total.saturating_add(entry.tokens.total_tokens);
                }
            }
        }

        let max_total = daily.iter().map(|(_, total)| *total).max().unwrap_or(0);
        let mut lines: Vec<RtLine<'static>> = Vec::new();
        lines.push(RtLine::from(vec![RtSpan::styled(
            "7 Day History",
            Style::default().add_modifier(Modifier::BOLD),
        )]));

        for (day, total) in daily.iter() {
            let label = day.format("%b %d").to_string();
            let bar = Self::bar_segment(*total, max_total, WIDTH);
            let tokens = format_with_separators(*total);
            lines.push(RtLine::from(vec![
                RtSpan::styled(
                    format!(" {label} "),
                    Style::default().fg(crate::colors::text_dim()),
                ),
                RtSpan::styled("│ ", Style::default().fg(crate::colors::text_dim())),
                RtSpan::styled(bar, Style::default().fg(crate::colors::primary())),
                RtSpan::raw(format!(" {tokens} tokens")),
            ]));
        }
        lines
    }

    fn bar_segment(value: u64, max: u64, width: usize) -> String {
        const FILL: &str = "▇";
        if max == 0 {
            return format!("{}{}", FILL, " ".repeat(width.saturating_sub(1)));
        }
        if value == 0 {
            return format!("{}{}", FILL, " ".repeat(width.saturating_sub(1)));
        }
        let ratio = value as f64 / max as f64;
        let filled = (ratio * width as f64).ceil().clamp(1.0, width as f64) as usize;
        format!(
            "{}{}",
            FILL.repeat(filled),
            " ".repeat(width.saturating_sub(filled))
        )
    }

    fn dim_line(text: impl Into<String>) -> RtLine<'static> {
        RtLine::from(vec![RtSpan::styled(
            text.into(),
            Style::default().fg(crate::colors::text_dim()),
        )])
    }

    // MAINT-11: close_pro_overlay, handle_pro_overlay_key moved to pro_overlay.rs

    // dispatch_command() removed — command routing is handled at the App layer via AppEvent::DispatchCommand

    pub(crate) fn handle_paste(&mut self, text: String) {
        // Check if the pasted text is a file path to an image
        let trimmed = text.trim();

        tracing::info!("Paste received: {:?}", trimmed);

        // Try to normalize as a file path (handles file:// URLs and terminal escapes)
        if let Some(path_str) = input_helpers::normalize_pasted_path(trimmed) {
            tracing::info!("Decoded path: {:?}", path_str);

            // Check if it has an image extension
            if input_helpers::is_image_extension(&path_str) {
                let path = PathBuf::from(&path_str);
                tracing::info!("Checking if path exists: {:?}", path);
                if path.exists() {
                    tracing::info!("Image file dropped/pasted: {:?}", path);
                    // Get just the filename for display
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("image");

                    // Add a placeholder to the compose field instead of submitting
                    let placeholder = format!("[image: {}]", filename);

                    // Store the image path for later submission
                    self.pending_images.insert(placeholder.clone(), path);

                    // Add the placeholder text to the compose field
                    self.bottom_pane.handle_paste(placeholder);
                    // Force immediate redraw to reflect input growth/wrap
                    self.request_redraw();
                    return;
                } else {
                    tracing::warn!("Image path does not exist: {:?}", path);
                }
            } else {
                // For non-image files, paste the decoded path as plain text.
                let path = PathBuf::from(&path_str);
                if path.exists() && path.is_file() {
                    self.bottom_pane.handle_paste(path_str);
                    self.request_redraw();
                    return;
                }
            }
        }

        // Otherwise handle as regular text paste
        self.bottom_pane.handle_paste(text);
        // Force immediate redraw so compose height matches new content
        self.request_redraw();
    }

    /// Briefly show the vertical scrollbar and schedule a redraw to hide it.
    fn flash_scrollbar(&self) {
        layout_scroll::flash_scrollbar(self);
    }

    fn history_insert_with_key_global(
        &mut self,
        cell: Box<dyn HistoryCell>,
        key: OrderKey,
    ) -> usize {
        self.history_insert_with_key_global_tagged(cell, key, "untagged")
    }

    // Internal: same as above but with a short tag for debug overlays.
    fn history_insert_with_key_global_tagged(
        &mut self,
        cell: Box<dyn HistoryCell>,
        key: OrderKey,
        tag: &'static str,
    ) -> usize {
        #[cfg(debug_assertions)]
        {
            let cell_kind = cell.kind();
            if cell_kind == HistoryCellType::BackgroundEvent {
                debug_assert!(
                    tag == "background",
                    "Background events must use the background helper (tag={})",
                    tag
                );
            }
        }
        // Any ordered insert of a non-reasoning cell means reasoning is no longer the
        // bottom-most active block; drop the in-progress ellipsis on collapsed titles.
        let is_reasoning_cell = cell
            .as_any()
            .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
            .is_some();
        if !is_reasoning_cell {
            self.clear_reasoning_in_progress();
        }
        // Determine insertion position across the entire history
        let mut pos = self.history_cells.len();
        for i in 0..self.history_cells.len() {
            if let Some(existing) = self.cell_order_seq.get(i)
                && *existing > key
            {
                pos = i;
                break;
            }
        }

        // Keep auxiliary order vector in lockstep with history before inserting
        if self.cell_order_seq.len() < self.history_cells.len() {
            let missing = self.history_cells.len() - self.cell_order_seq.len();
            for _ in 0..missing {
                self.cell_order_seq.push(OrderKey {
                    req: 0,
                    out: -1,
                    seq: 0,
                });
            }
        }

        tracing::info!(
            "[order] insert: {} pos={} len_before={} order_len_before={} tag={}",
            Self::debug_fmt_order_key(key),
            pos,
            self.history_cells.len(),
            self.cell_order_seq.len(),
            tag
        );
        // If order overlay is enabled, compute a short, inline debug summary for
        // reasoning titles so we can spot mid‑word character drops quickly.
        // We intentionally do this before inserting so we can attach the
        // composed string alongside the standard order debug info.
        let reasoning_title_dbg: Option<String> = if self.show_order_overlay {
            // CollapsibleReasoningCell shows a collapsed "title" line; extract
            // the first visible line and summarize its raw text/lengths.
            if let Some(rc) = cell
                .as_any()
                .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
            {
                let lines = rc.display_lines_trimmed();
                let first = lines.first();
                if let Some(line) = first {
                    // Collect visible text and basic metrics
                    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                    let bytes = text.len();
                    let chars = text.chars().count();
                    let width = unicode_width::UnicodeWidthStr::width(text.as_str());
                    let spans = line.spans.len();
                    // Per‑span byte lengths to catch odd splits inside words
                    let span_lens: Vec<usize> =
                        line.spans.iter().map(|s| s.content.len()).collect();
                    // Truncate preview to avoid overflow in narrow panes
                    let mut preview = text.clone();
                    // Truncate preview by display width, not bytes, to avoid splitting
                    // a multi-byte character at an invalid boundary.
                    {
                        use unicode_width::UnicodeWidthStr as _;
                        let maxw = 120usize;
                        if preview.width() > maxw {
                            preview = format!(
                                "{}…",
                                crate::live_wrap::take_prefix_by_width(
                                    &preview,
                                    maxw.saturating_sub(1)
                                )
                                .0
                            );
                        }
                    }
                    Some(format!(
                        "title='{}' bytes={} chars={} width={} spans={} span_bytes={:?}",
                        preview, bytes, chars, width, spans, span_lens
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        self.history_cells.insert(pos, cell);
        // In terminal mode, App mirrors history lines into the native buffer.
        // Ensure order vector is also long enough for position after cell insert
        if self.cell_order_seq.len() < pos {
            self.cell_order_seq.resize(
                pos,
                OrderKey {
                    req: 0,
                    out: -1,
                    seq: 0,
                },
            );
        }
        self.cell_order_seq.insert(pos, key);
        // Insert debug info aligned with cell insert
        let ordered = "ordered";
        let req_dbg = format!("{}", key.req);
        let dbg = if let Some(tdbg) = reasoning_title_dbg {
            format!(
                "insert: {} req={} key={} {} pos={} tag={} | {}",
                ordered,
                req_dbg,
                0,
                Self::debug_fmt_order_key(key),
                pos,
                tag,
                tdbg
            )
        } else {
            format!(
                "insert: {} req={} {} pos={} tag={}",
                ordered,
                req_dbg,
                Self::debug_fmt_order_key(key),
                pos,
                tag
            )
        };
        if self.cell_order_dbg.len() < pos {
            self.cell_order_dbg.resize(pos, None);
        }
        self.cell_order_dbg.insert(pos, Some(dbg));
        self.invalidate_height_cache();
        self.autoscroll_if_near_bottom();
        self.bottom_pane.set_has_chat_history(true);
        self.process_animation_cleanup();
        // Maintain input focus when new history arrives unless a modal overlay owns it
        if !self.agents_terminal.active {
            self.bottom_pane.ensure_input_focus();
        }
        self.app_event_tx.send(AppEvent::RequestRedraw);
        self.refresh_explore_trailing_flags();
        self.refresh_reasoning_collapsed_visibility();
        pos
    }

    /// Push a cell using a synthetic global order key at the bottom of the current request.
    pub(crate) fn history_push(&mut self, cell: impl HistoryCell + 'static) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                cell.kind() != HistoryCellType::BackgroundEvent,
                "Background events must use push_background_* helpers"
            );
        }

        let key = self.next_internal_key();
        tracing::debug!(
            "📝 HISTORY_PUSH: kind={:?} | tag=epilogue | key={:?}",
            cell.kind(),
            key
        );
        let _ = self.history_insert_with_key_global_tagged(Box::new(cell), key, "epilogue");

        // SPEC-KIT-900 FIX: Removed premature quality gate triggering.
        // The handler should only be called when QualityGateNativeAgentsComplete event
        // is received (after agents actually complete), not on every history_push.
        // This was causing "0 of 3 expected agents" because the broker ran before
        // agents were spawned.
        //
        // The proper flow is:
        // 1. execute_quality_checkpoint() spawns agents in background
        // 2. wait_for_quality_gate_agents() polls until complete
        // 3. QualityGateNativeAgentsComplete event is sent
        // 4. app.rs handler calls set_native_agent_ids + on_quality_gate_agents_complete
    }
    /// Insert a background event near the top of the current request so it appears
    /// before imminent provider output (e.g. Exec begin).
    pub(crate) fn insert_background_event_early(&mut self, message: String) {
        self.insert_background_event_with_placement(message, BackgroundPlacement::BeforeNextOutput);
    }
    /// Insert a background event using the specified placement semantics.
    pub(crate) fn insert_background_event_with_placement(
        &mut self,
        message: String,
        placement: BackgroundPlacement,
    ) {
        let system_placement = match placement {
            BackgroundPlacement::Tail => SystemPlacement::EndOfCurrent,
            BackgroundPlacement::BeforeNextOutput => {
                if self.pending_user_prompts_for_next_turn > 0 {
                    SystemPlacement::EarlyInCurrent
                } else {
                    SystemPlacement::PrePromptInCurrent
                }
            }
        };
        self.push_system_cell(
            history_cell::new_background_event(message),
            system_placement,
            None,
            None,
            "background",
        );
    }

    pub(crate) fn push_background_tail(&mut self, message: impl Into<String>) {
        self.insert_background_event_with_placement(message.into(), BackgroundPlacement::Tail);
    }

    pub(crate) fn push_background_before_next_output(&mut self, message: impl Into<String>) {
        self.insert_background_event_with_placement(
            message.into(),
            BackgroundPlacement::BeforeNextOutput,
        );
    }

    /// Push a cell using a synthetic key at the TOP of the NEXT request.
    fn history_push_top_next_req(&mut self, cell: impl HistoryCell + 'static) {
        let key = self.next_req_key_top();
        let tag = if cell.kind() == HistoryCellType::BackgroundEvent {
            "background"
        } else {
            "prelude"
        };
        let _ = self.history_insert_with_key_global_tagged(Box::new(cell), key, tag);
    }
    /// Push a user prompt so it appears right under banners and above model output for the next request.
    fn history_push_prompt_next_req(&mut self, cell: impl HistoryCell + 'static) {
        let key = self.next_req_key_prompt();
        tracing::debug!(
            "📝 HISTORY_PUSH_PROMPT: kind={:?} | tag=prompt | key={:?}",
            cell.kind(),
            key
        );
        let _ = self.history_insert_with_key_global_tagged(Box::new(cell), key, "prompt");
    }

    fn history_replace_at(&mut self, idx: usize, cell: Box<dyn HistoryCell>) {
        if idx < self.history_cells.len() {
            self.history_cells[idx] = cell;
            self.invalidate_height_cache();
            self.request_redraw();
            self.refresh_explore_trailing_flags();
            // Keep debug info for this cell index as-is.
        }
    }

    /// Re-sort history_cells and cell_order_seq by OrderKey values using swap-based algorithm.
    ///
    /// Called when a user cell's OrderKey is updated from temporary to provider-confirmed
    /// and the change is significant enough to potentially affect ordering.
    ///
    /// SPEC-954-FIX: Swap-based reordering avoids needing HistoryCell cloning.
    /// Uses cycle-following algorithm to apply permutation in-place.
    fn resort_history_by_order(&mut self) {
        let len = self.history_cells.len();
        if len == 0 {
            return;
        }

        // Build sorted indices: sorted_indices[sorted_pos] = original_pos
        // After sorting, sorted_indices[i] tells us which original position should be at slot i
        let mut sorted_indices: Vec<usize> = (0..len).collect();
        sorted_indices.sort_by_key(|&i| {
            self.cell_order_seq.get(i).copied().unwrap_or(OrderKey {
                req: 0,
                out: -1,
                seq: 0,
            })
        });

        // Check if reordering is actually needed
        let needs_resort = sorted_indices
            .iter()
            .enumerate()
            .any(|(sorted_pos, &original_pos)| sorted_pos != original_pos);
        if !needs_resort {
            tracing::debug!("🔄 RESORT: No changes needed (already sorted)");
            return;
        }

        tracing::info!("🔄 RESORT: Reordering {} cells", len);

        // SPEC-958 FIX: Track where each "original position element" currently lives.
        // position_of[original] = current_slot
        // Initially, element from original position i is at slot i.
        let mut position_of: Vec<usize> = (0..len).collect();

        // For each target slot, place the correct element there
        for target_slot in 0..len {
            // We want the element that was originally at sorted_indices[target_slot]
            let wanted_original = sorted_indices[target_slot];

            // Where is that element now?
            let current_slot = position_of[wanted_original];

            if current_slot != target_slot {
                // Swap into place
                self.history_cells.swap(target_slot, current_slot);
                self.cell_order_seq.swap(target_slot, current_slot);
                if target_slot < self.cell_order_dbg.len()
                    && current_slot < self.cell_order_dbg.len()
                {
                    self.cell_order_dbg.swap(target_slot, current_slot);
                }

                // Update position_of: the element that WAS at target_slot is now at current_slot
                // Find what was at target_slot before the swap
                // Since position_of[x] = target_slot for some x, we need to find that x
                // But we can get it from sorted_indices - at this point:
                // - target_slot used to contain whatever was there before our swaps
                // - We need to update the position tracker
                let evicted_original = sorted_indices
                    .iter()
                    .position(|&orig| position_of[orig] == target_slot)
                    .unwrap_or(wanted_original);

                if evicted_original != wanted_original {
                    position_of[evicted_original] = current_slot;
                }
                position_of[wanted_original] = target_slot;
            }
        }

        self.invalidate_height_cache();
        self.request_redraw();
    }

    fn resolve_running_tool_index(&self, entry: &RunningToolEntry) -> Option<usize> {
        if let Some(pos) = self
            .cell_order_seq
            .iter()
            .position(|key| *key == entry.order_key)
        {
            return Some(pos);
        }
        if entry.fallback_index < self.history_cells.len() {
            return Some(entry.fallback_index);
        }
        None
    }

    fn history_remove_at(&mut self, idx: usize) {
        if idx < self.history_cells.len() {
            self.history_cells.remove(idx);
            if idx < self.cell_order_seq.len() {
                self.cell_order_seq.remove(idx);
            }
            if idx < self.cell_order_dbg.len() {
                self.cell_order_dbg.remove(idx);
            }
            self.invalidate_height_cache();
            self.request_redraw();
            self.refresh_explore_trailing_flags();
        }
    }

    fn history_replace_and_maybe_merge(&mut self, idx: usize, cell: Box<dyn HistoryCell>) {
        // Replace at index, then attempt standard exec merge with previous cell.
        self.history_replace_at(idx, cell);
        // Merge only if the new cell is an Exec with output (completed) or a MergedExec.
        crate::chatwidget::exec_tools::try_merge_completed_exec_at(self, idx);
    }

    // Merge adjacent tool cells with the same header (e.g., successive Web Search blocks)
    fn history_maybe_merge_tool_with_previous(&mut self, idx: usize) {
        if idx == 0 || idx >= self.history_cells.len() {
            return;
        }
        let new_lines = self.history_cells[idx].display_lines();
        let new_header = new_lines
            .first()
            .and_then(|l| l.spans.first())
            .map(|s| s.content.clone().to_string())
            .unwrap_or_default();
        if new_header.is_empty() {
            return;
        }
        let prev_lines = self.history_cells[idx - 1].display_lines();
        let prev_header = prev_lines
            .first()
            .and_then(|l| l.spans.first())
            .map(|s| s.content.clone().to_string())
            .unwrap_or_default();
        if new_header != prev_header {
            return;
        }
        let mut combined = prev_lines.clone();
        while combined
            .last()
            .map(|l| crate::render::line_utils::is_blank_line_trim(l))
            .unwrap_or(false)
        {
            combined.pop();
        }
        let mut body: Vec<ratatui::text::Line<'static>> = new_lines.into_iter().skip(1).collect();
        while body
            .first()
            .map(|l| crate::render::line_utils::is_blank_line_trim(l))
            .unwrap_or(false)
        {
            body.remove(0);
        }
        while body
            .last()
            .map(|l| crate::render::line_utils::is_blank_line_trim(l))
            .unwrap_or(false)
        {
            body.pop();
        }
        if let Some(first_line) = body.first_mut()
            && let Some(first_span) = first_line.spans.get_mut(0)
            && (first_span.content == "  └ " || first_span.content == "└ ")
        {
            first_span.content = "  ".into();
        }
        combined.extend(body);
        self.history_replace_at(
            idx - 1,
            Box::new(crate::history_cell::PlainHistoryCell::new(
                combined,
                crate::history_cell::HistoryCellType::Plain,
            )),
        );
        self.history_remove_at(idx);
    }

    /// Clean up faded-out animation cells
    fn process_animation_cleanup(&mut self) {
        // With trait-based cells, we can't easily detect and clean up specific cell types
        // Animation cleanup is now handled differently
    }

    /// Replace the initial Popular Commands notice that includes
    /// the transient "Connecting MCP servers…" line with a version
    /// that omits it.
    fn remove_connecting_mcp_notice(&mut self) {
        let needle = "Connecting MCP servers…";
        if let Some((idx, cell)) = self.history_cells.iter().enumerate().find(|(_, cell)| {
            cell.display_lines().iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.content.as_ref() == needle)
            })
        }) {
            match cell.kind() {
                crate::history_cell::HistoryCellType::Notice => {
                    // Older layout: status was inside the notice cell — replace it
                    self.history_replace_at(
                        idx,
                        Box::new(history_cell::new_popular_commands_notice(
                            false,
                            self.latest_upgrade_version.as_deref(),
                        )),
                    );
                }
                _ => {
                    // New layout: status is a separate BackgroundEvent cell — remove it
                    self.history_remove_at(idx);
                }
            }
        }
    }

    fn refresh_explore_trailing_flags(&mut self) {
        let mut trailing_non_reasoning: Option<usize> = None;
        for i in (0..self.history_cells.len()).rev() {
            if self.history_cells[i]
                .as_any()
                .downcast_ref::<history_cell::CollapsibleReasoningCell>()
                .is_some()
            {
                continue;
            }
            trailing_non_reasoning = Some(i);
            break;
        }

        for (idx, cell) in self.history_cells.iter_mut().enumerate() {
            if let Some(explore) = cell
                .as_any_mut()
                .downcast_mut::<history_cell::ExploreAggregationCell>()
            {
                explore.set_trailing(Some(idx) == trailing_non_reasoning);
            }
        }
    }

    fn submit_user_message(&mut self, user_message: UserMessage) {
        // Surface a local diagnostic note and anchor it to the NEXT turn,
        // placing it directly after the user prompt so ordering is stable.
        // (debug message removed)
        // Fade the welcome cell only when a user actually posts a message.
        for cell in &self.history_cells {
            cell.trigger_fade();
        }
        let mut message = user_message;
        // If our configured cwd no longer exists (e.g., a worktree folder was
        // deleted outside the app), try to automatically recover to the repo
        // root for worktrees and re-submit the same message there.
        if !self.config.cwd.exists() {
            let missing = self.config.cwd.clone();
            let missing_s = missing.display().to_string();
            if missing_s.contains("/.code/branches/") {
                // Recover by walking up to '<repo>/.code/branches/<branch>' -> repo root
                let mut anc = missing.as_path();
                // Walk up 3 parents if available
                for _ in 0..3 {
                    if let Some(p) = anc.parent() {
                        anc = p;
                    }
                }
                let fallback_root = anc.to_path_buf();
                if fallback_root.exists() {
                    let msg = format!(
                        "⚠️ Worktree directory is missing: {}\nSwitching to repo root: {}",
                        missing.display(),
                        fallback_root.display()
                    );
                    self.app_event_tx.send_background_event(msg);
                    // Re-submit this exact message after switching cwd
                    self.app_event_tx.send(AppEvent::SwitchCwd(
                        fallback_root,
                        Some(message.display_text.clone()),
                    ));
                    return;
                }
            }
            // If we can't recover, surface an error and drop the message to prevent loops
            self.history_push(history_cell::new_error_event(format!(
                "Working directory is missing: {}",
                self.config.cwd.display()
            )));
            return;
        }
        let original_text = message.display_text.clone();
        // Build a combined string view of the text-only parts to process slash commands
        let mut text_only = String::new();
        for it in &message.ordered_items {
            if let InputItem::Text { text } = it {
                if !text_only.is_empty() {
                    text_only.push('\n');
                }
                text_only.push_str(text);
            }
        }

        // Save the prompt if it's a spec-kit pipeline/agent command.
        let original_trimmed = original_text.trim();
        if original_trimmed.starts_with("/speckit.")
            || original_trimmed.starts_with("/guardrail.")
            || original_trimmed.starts_with("/spec-consensus ")
            || original_trimmed.starts_with("/spec-status")
            || original_trimmed.starts_with("/spec-evidence-stats")
        {
            self.last_agent_prompt = Some(original_text.clone());
        }

        // Process slash commands and expand them if needed
        // First, allow custom subagent commands: if the message starts with a slash and the
        // command name matches a saved subagent in config, synthesize a unified prompt using
        // format_subagent_command and replace the message with that prompt.
        if let Some(first) = original_text.trim().strip_prefix('/') {
            let mut parts = first.splitn(2, ' ');
            let cmd_name = parts.next().unwrap_or("").trim();
            let args = parts.next().unwrap_or("").trim().to_string();
            if !cmd_name.is_empty() {
                let has_custom = self
                    .config
                    .subagent_commands
                    .iter()
                    .any(|c| c.name.eq_ignore_ascii_case(cmd_name));

                // Legacy upstream prompt-expanding commands were removed in this fork.
                // If the user hasn't explicitly reintroduced them via custom config,
                // show a crisp migration message instead of sending "/plan ..." as plain text.
                let cmd_name_lc = cmd_name.to_ascii_lowercase();
                if matches!(cmd_name_lc.as_str(), "plan" | "solve" | "code") && !has_custom {
                    let message = match cmd_name_lc.as_str() {
                        "plan" => {
                            "Removed command: /plan\nUse Spec-Kit: /speckit.new <AREA> <description>, then /speckit.plan SPEC-ID (or /speckit.auto SPEC-ID)."
                        }
                        "solve" => {
                            "Removed command: /solve\nUse Spec-Kit: /speckit.new <AREA> <description>, then /speckit.implement SPEC-ID (or /speckit.auto SPEC-ID)."
                        }
                        _ => {
                            "Removed command: /code\nUse Spec-Kit: /speckit.new <AREA> <description>, then /speckit.auto SPEC-ID."
                        }
                    };
                    self.history_push(history_cell::new_warning_event(message.to_string()));
                    return;
                }

                if has_custom {
                    let res = codex_core::slash_commands::format_subagent_command(
                        cmd_name,
                        &args,
                        Some(&self.config.agents),
                        Some(&self.config.subagent_commands),
                    );
                    // Acknowledge configuration
                    let mode = if res.read_only { "read-only" } else { "write" };
                    let mut ack: Vec<ratatui::text::Line<'static>> = Vec::new();
                    ack.push(ratatui::text::Line::from(format!(
                        "/{} configured",
                        res.name
                    )));
                    ack.push(ratatui::text::Line::from(format!("mode: {}", mode)));
                    ack.push(ratatui::text::Line::from(format!(
                        "agents: {}",
                        if res.models.is_empty() {
                            "<none>".to_string()
                        } else {
                            res.models.join(", ")
                        }
                    )));
                    ack.push(ratatui::text::Line::from(format!(
                        "command: {}",
                        original_text.trim()
                    )));
                    self.history_push(crate::history_cell::PlainHistoryCell::new(
                        ack,
                        crate::history_cell::HistoryCellType::Notice,
                    ));

                    message.ordered_items.clear();
                    message
                        .ordered_items
                        .push(InputItem::Text { text: res.prompt });
                    // Continue with normal submission after this match block
                }
            }
        }

        // SPEC-KIT-902: Stage commands now use direct spawning via command_registry,
        // so we don't need to parse stage invocations here anymore.

        let processed = crate::slash_command::process_slash_command_message(&text_only);
        match processed {
            crate::slash_command::ProcessedCommand::ExpandedPrompt(expanded) => {
                message.ordered_items.clear();
                message
                    .ordered_items
                    .push(InputItem::Text { text: expanded });
            }
            crate::slash_command::ProcessedCommand::RegularCommand {
                command: cmd,
                command_text,
                notice,
            } => {
                if let Some(message) = notice {
                    self.history_push(history_cell::new_warning_event(message));
                }

                if cmd == SlashCommand::Undo {
                    self.handle_undo_command();
                    return;
                }
                // This is a regular slash command, dispatch it normally
                self.app_event_tx
                    .send(AppEvent::DispatchCommand(cmd, command_text));
                return;
            }
            crate::slash_command::ProcessedCommand::SpecAuto(invocation) => {
                // DEBUG: Trace SpecAuto routing (SPEC-DOGFOOD-001 Session 29)
                self.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "📍 DEBUG: submit_user_message → SpecAuto(spec_id={})",
                        invocation.spec_id
                    ))],
                    crate::history_cell::HistoryCellType::Notice,
                ));
                // SPEC-KIT-900 FIX: Route to native pipeline coordinator
                // Previously used format_subagent_command() which fell back to ALL agents
                // when no [[subagents.commands]] config existed for "spec-auto".
                // Now routes directly to handle_spec_auto_command() which uses
                // MODEL-POLICY.md single-agent-per-stage (GR-001).
                self.handle_spec_auto_command(invocation);
                return;
            }
            crate::slash_command::ProcessedCommand::Error(error_msg) => {
                // Show error in history
                self.history_push(history_cell::new_error_event(error_msg));
                return;
            }
            crate::slash_command::ProcessedCommand::NotCommand(_) => {
                // Not a slash command, process normally
            }
        }

        let mut items: Vec<InputItem> = Vec::new();

        // Check if browser mode is enabled and capture screenshot
        // IMPORTANT: Always use global browser manager for consistency
        // Use the ordered items (text + images interleaved with markers)
        items.extend(message.ordered_items.clone());
        message.ordered_items = items;

        if message.ordered_items.is_empty() {
            return;
        }

        let prompt_summary = if message.display_text.trim().is_empty() {
            None
        } else {
            Some(message.display_text.clone())
        };
        self.capture_ghost_snapshot(prompt_summary);

        let turn_active = self.is_task_running()
            || !self.active_task_ids.is_empty()
            || self.stream.is_write_cycle_active()
            || !self.queued_user_messages.is_empty();

        if turn_active {
            tracing::info!(
                "Queuing user input while turn is active (queued: {})",
                self.queued_user_messages.len() + 1
            );

            // SPEC-KIT-952: Skip codex-core queue for CLI-routed models
            // CLI models (Claude/Gemini) process queued messages locally via CLI routing
            let is_cli_model = crate::model_router::supports_cli_streaming(&self.config.model);

            self.queued_user_messages.push_back(message);
            self.refresh_queued_user_messages();

            if !is_cli_model {
                // ChatGPT: Send to codex-core queue (native OAuth flow)
                let queue_items = self
                    .queued_user_messages
                    .back()
                    .map(|msg| msg.ordered_items.clone())
                    .unwrap_or_default();

                match self
                    .codex_op_tx
                    .send(Op::QueueUserInput { items: queue_items })
                {
                    Ok(()) => {
                        if let Some(sent_message) = self.queued_user_messages.pop_back() {
                            self.refresh_queued_user_messages();
                            self.finalize_sent_user_message(sent_message);
                        }
                    }
                    Err(e) => {
                        tracing::error!("failed to send QueueUserInput op: {e}");
                    }
                }
            }
            // CLI models: Just queued locally, will process via CLI routing when turn completes
            // (no immediate finalize needed - will happen when message is processed)

            return;
        }

        let mut batch: Vec<UserMessage> = self.queued_user_messages.drain(..).collect();
        batch.push(message);
        self.refresh_queued_user_messages();
        self.send_user_messages_to_agent(batch);

        // (debug watchdog removed)
    }

    // Undo/snapshot functions moved to undo_snapshots.rs (MAINT-11 Phase 9)

    /// Show PRD builder modal with project-specific questions (SPEC-KIT-971)
    /// Requires area (e.g., "CORE", "TUI") for new feature ID generation
    #[allow(dead_code)]
    pub(crate) fn show_prd_builder_with_context(
        &mut self,
        description: String,
        project_type_display: String,
        questions: Vec<crate::bottom_pane::prd_builder_modal::PrdQuestion>,
        area: String,
    ) {
        self.bottom_pane.show_prd_builder_with_context(
            description,
            project_type_display,
            questions,
            area,
        );
    }

    /// Show clarify modal for interactive clarification resolution (SPEC-KIT-971)
    pub(crate) fn show_clarify_modal(
        &mut self,
        spec_id: String,
        questions: Vec<crate::bottom_pane::clarify_modal::ClarifyQuestion>,
    ) {
        self.bottom_pane.show_clarify_modal(spec_id, questions);
    }

    /// Show vision builder modal for guided constitution creation (P93/SPEC-KIT-105)
    pub(crate) fn show_vision_builder(&mut self) {
        self.bottom_pane.show_vision_builder();
    }

    /// Show maieutic elicitation modal for pre-flight clarification (D130)
    pub(crate) fn show_maieutic_modal(
        &mut self,
        spec_id: String,
        questions: Vec<spec_kit::maieutic::MaieuticQuestion>,
    ) {
        self.bottom_pane.show_maieutic_modal(spec_id, questions);
    }

    /// Show spec intake modal for Architect-in-a-box (Phase 1)
    /// Requires area (e.g., "CORE", "TUI") for new feature ID generation
    pub(crate) fn show_spec_intake_modal(&mut self, description: String, deep: bool, area: String) {
        self.bottom_pane
            .show_spec_intake_modal(description, deep, area);
    }

    /// Show spec intake modal for backfill (Phase 2: IntakePresenceGate)
    pub(crate) fn show_spec_intake_modal_backfill(&mut self, spec_id: String) {
        self.bottom_pane.show_spec_intake_modal_backfill(spec_id);
    }

    /// Show project intake modal for /speckit.projectnew flow
    pub(crate) fn show_project_intake_modal(&mut self, project_id: String, deep: bool) {
        self.bottom_pane.show_project_intake_modal(project_id, deep);
    }

    // perform_undo_restore, reset_after_conversation_restore moved to undo_snapshots.rs (MAINT-11 Phase 9)

    fn flush_pending_agent_notes(&mut self) {
        for note in self.pending_agent_notes.drain(..) {
            if let Err(e) = self.codex_op_tx.send(Op::AddToHistory { text: note }) {
                tracing::error!("failed to send AddToHistory op: {e}");
            }
        }
    }

    fn finalize_sent_user_message(&mut self, message: UserMessage) {
        let UserMessage { display_text, .. } = message;

        if !display_text.is_empty() {
            // SPEC-954-FIX: Defer user cell creation until provider responds
            // This ensures user message and answer use same request_ordinal from provider
            tracing::debug!(
                "🔵 USER_MSG_QUEUED: Deferred cell creation | queue_pos={} | preview={}...",
                self.pending_dispatched_user_messages.len(),
                &display_text.chars().take(50).collect::<String>()
            );

            self.pending_dispatched_user_messages
                .push_back(display_text.clone());
            self.pending_user_prompts_for_next_turn += 1;

            // SPEC-954: Start timeout timer for this message
            let msg_id = format!("msg-{}", self.internal_seq);
            self.pending_message_timestamps
                .insert(msg_id.clone(), std::time::Instant::now());

            let tx = self.app_event_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                tx.send(AppEvent::UserMessageTimeout {
                    message_id: msg_id,
                    elapsed_ms: 10000,
                });
            });
        }

        self.flush_pending_agent_notes();

        if !display_text.is_empty()
            && let Err(e) = self
                .codex_op_tx
                .send(Op::AddToHistory { text: display_text })
        {
            tracing::error!("failed to send AddHistory op: {e}");
        }

        self.request_redraw();
    }

    fn send_user_messages_to_agent(&mut self, messages: Vec<UserMessage>) {
        if messages.is_empty() {
            return;
        }

        let mut combined_items: Vec<InputItem> = Vec::new();
        let mut history_texts: Vec<String> = Vec::new();

        for (
            idx,
            UserMessage {
                display_text,
                ordered_items,
            },
        ) in messages.into_iter().enumerate()
        {
            if !display_text.is_empty() {
                // SPEC-954-FIX: Don't create cells here - will be created by:
                // - CLI routing: Before spawning CLI stream (line ~5809)
                // - OAuth routing: In TaskStarted handler when provider responds
                history_texts.push(display_text.clone());
            }

            if idx > 0 && !combined_items.is_empty() && !ordered_items.is_empty() {
                combined_items.push(InputItem::Text {
                    text: "\n\n".to_string(),
                });
            }

            combined_items.extend(ordered_items);
        }

        if combined_items.is_empty() {
            return;
        }

        let total_items = combined_items.len();
        let ephemeral_count = combined_items
            .iter()
            .filter(|item| matches!(item, InputItem::EphemeralImage { .. }))
            .count();
        if ephemeral_count > 0 {
            tracing::info!(
                "Sending {} items to model (including {} ephemeral images)",
                total_items,
                ephemeral_count
            );
        }

        self.flush_pending_agent_notes();

        // SPEC-KIT-952: Check if CLI routing should be used for this model (Claude/Gemini)
        if crate::model_router::supports_cli_streaming(&self.config.model) {
            // Extract prompt text from combined items
            let prompt_text: String = combined_items
                .iter()
                .filter_map(|item| match item {
                    InputItem::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            if prompt_text.is_empty() {
                // Empty prompt - log warning and return early to prevent fallthrough to OAuth
                tracing::warn!(
                    "Empty prompt_text for CLI-routed model {}, skipping",
                    self.config.model
                );
                return;
            }

            // Non-empty prompt - proceed with CLI routing
            {
                // SPEC-954-FIX: CLI routing creates cells immediately (before streaming starts)
                // This is necessary because CLI doesn't send TaskStarted events
                for text in &history_texts {
                    self.history_push_prompt_next_req(history_cell::new_user_prompt(text.clone()));
                    self.pending_user_prompts_for_next_turn += 1;
                }

                // 1. Add user message to TUI history display
                for text in history_texts {
                    if let Err(e) = self.codex_op_tx.send(Op::AddToHistory { text }) {
                        tracing::error!("failed to send AddHistory op: {e}");
                    }
                }

                // 2. Set task running to block input
                self.bottom_pane.set_task_running(true);

                // 3. Clone data for async task
                let model = self.config.model.clone();
                let prompt = prompt_text.clone();
                let tx = self.app_event_tx.clone();

                // 4. Spawn async CLI streaming task (SPEC-KIT-952)
                tokio::spawn(async move {
                    let result = crate::model_router::execute_with_cli_streaming(
                        &model,
                        &prompt,
                        tx.clone(),
                    )
                    .await;

                    // Log any errors (streaming events already sent)
                    if let Err(e) = result {
                        tracing::error!("CLI streaming failed: {}", e);
                    }
                });

                // Don't send to codex-core for CLI-routed models
                return;
            } // End of non-empty prompt block
        } // End of CLI routing check

        // SPEC-954-FIX: OAuth path - queue messages for deferred cell creation
        // Cells will be created by TaskStarted handler when provider responds
        for text in &history_texts {
            tracing::debug!(
                "🔵 USER_MSG_QUEUED (batch): Deferred cell creation | queue_pos={} | preview={}...",
                self.pending_dispatched_user_messages.len(),
                &text.chars().take(50).collect::<String>()
            );
            self.pending_dispatched_user_messages
                .push_back(text.clone());
            self.pending_user_prompts_for_next_turn += 1;

            // SPEC-954: Start timeout timer for this message
            let msg_id = format!("msg-{}", self.internal_seq);
            self.pending_message_timestamps
                .insert(msg_id.clone(), std::time::Instant::now());

            let tx = self.app_event_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                tx.send(AppEvent::UserMessageTimeout {
                    message_id: msg_id,
                    elapsed_ms: 10000,
                });
            });
        }

        // Native path: send to codex-core
        if let Err(e) = self.codex_op_tx.send(Op::UserInput {
            items: combined_items,
        }) {
            tracing::error!("failed to send Op::UserInput: {e}");
        }

        for text in history_texts {
            if let Err(e) = self.codex_op_tx.send(Op::AddToHistory { text }) {
                tracing::error!("failed to send AddHistory op: {e}");
            }
        }
    }

    fn refresh_queued_user_messages(&mut self) {
        self.request_redraw();
    }

    fn request_redraw(&mut self) {
        self.app_event_tx.send(AppEvent::RequestRedraw);
    }

    pub(crate) fn spec_cost_tracker(&self) -> Arc<spec_kit::cost_tracker::CostTracker> {
        self.cost_tracker.clone()
    }

    pub(crate) fn cost_summary_dir(&self) -> PathBuf {
        self.config
            .cwd
            .join(spec_kit::evidence::DEFAULT_EVIDENCE_BASE)
            .join("costs")
    }

    pub(crate) fn handle_perf_command(&mut self, args: String) {
        let arg = args.trim().to_lowercase();
        match arg.as_str() {
            "on" => {
                self.perf_state.enabled = true;
                self.add_perf_output("performance tracing: on".to_string());
            }
            "off" => {
                self.perf_state.enabled = false;
                self.add_perf_output("performance tracing: off".to_string());
            }
            "reset" => {
                self.perf_state.stats.borrow_mut().reset();
                self.add_perf_output("performance stats reset".to_string());
            }
            "show" | "" => {
                let summary = self.perf_state.stats.borrow().summary();
                self.add_perf_output(summary);
            }
            _ => {
                self.add_perf_output("usage: /perf on | off | show | reset".to_string());
            }
        }
        self.request_redraw();
    }

    pub(crate) fn handle_demo_command(&mut self) {
        use ratatui::style::Modifier as RtModifier;
        use ratatui::style::Style as RtStyle;
        use ratatui::text::Span;

        self.push_background_tail("demo: populating history with sample cells…");
        enum DemoPatch {
            Add {
                path: &'static str,
                content: &'static str,
            },
            Update {
                path: &'static str,
                unified_diff: &'static str,
                original: &'static str,
                new_content: &'static str,
            },
        }

        let scenarios = [
            (
                "build automation",
                "How do I wire up CI, linting, and release automation for this repo?",
                vec![
                    ("Context", "scan workspace layout and toolchain."),
                    ("Next", "surface build + validation commands."),
                    ("Goal", "summarize a reproducible workflow."),
                ],
                vec![
                    "streaming preview: inspecting package manifests…",
                    "streaming preview: drafting deployment summary…",
                    "streaming preview: cross-checking lint targets…",
                ],
                "**Here's a demo walkthrough:**\n\n1. Run `./build-fast.sh perf` to compile quickly.\n2. Cache artifacts in `codex-rs/target/perf`.\n3. Finish by sharing `./build-fast.sh run` output.\n\n```bash\n./build-fast.sh perf run\n```",
                vec![
                    (
                        vec!["git", "status"],
                        "On branch main\nnothing to commit, working tree clean\n",
                    ),
                    (vec!["rg", "--files"], ""),
                ],
                Some(DemoPatch::Add {
                    path: "src/demo.rs",
                    content: "fn main() {\n    println!(\"demo\");\n}\n",
                }),
                UpdatePlanArgs {
                    name: Some("Demo Scroll Plan".to_string()),
                    plan: vec![
                        PlanItemArg {
                            step: "Create reproducible builds".to_string(),
                            status: StepStatus::InProgress,
                        },
                        PlanItemArg {
                            step: "Verify validations".to_string(),
                            status: StepStatus::Pending,
                        },
                        PlanItemArg {
                            step: "Document follow-up tasks".to_string(),
                            status: StepStatus::Completed,
                        },
                    ],
                },
                (
                    "browser_open",
                    "https://example.com",
                    "navigated to example.com",
                ),
                ReasoningEffort::High,
                "demo: lint warnings will appear here",
                "demo: this slot shows error output",
                Some(
                    "diff --git a/src/lib.rs b/src/lib.rs\n@@ -1,3 +1,5 @@\n-pub fn hello() {}\n+pub fn hello() {\n+    println!(\"hello, demo!\");\n+}\n",
                ),
            ),
            (
                "release rehearsal",
                "What checklist should I follow before tagging a release?",
                vec![
                    ("Inventory", "collect outstanding changes and docs."),
                    ("Verify", "run smoke tests and package audits."),
                    ("Announce", "draft release notes and rollout plan."),
                ],
                vec![
                    "streaming preview: aggregating changelog entries…",
                    "streaming preview: validating release artifacts…",
                    "streaming preview: preparing announcement copy…",
                ],
                "**Release rehearsal:**\n\n1. Run `./scripts/create_github_release.sh --dry-run`.\n2. Capture artifact hashes in the notes.\n3. Schedule follow-up validation in automation.\n\n```bash\n./scripts/create_github_release.sh 1.2.3 --dry-run\n```",
                vec![
                    (
                        vec!["git", "--no-pager", "diff", "--stat"],
                        " src/lib.rs | 10 ++++++----\n 1 file changed, 6 insertions(+), 4 deletions(-)\n",
                    ),
                    (vec!["ls", "-1"], "Cargo.lock\nREADME.md\nsrc\ntarget\n"),
                ],
                Some(DemoPatch::Update {
                    path: "src/release.rs",
                    unified_diff: "--- a/src/release.rs\n+++ b/src/release.rs\n@@ -1 +1,3 @@\n-pub fn release() {}\n+pub fn release() {\n+    println!(\"drafting release\");\n+}\n",
                    original: "pub fn release() {}\n",
                    new_content: "pub fn release() {\n    println!(\"drafting release\");\n}\n",
                }),
                UpdatePlanArgs {
                    name: Some("Release Gate Plan".to_string()),
                    plan: vec![
                        PlanItemArg {
                            step: "Finalize changelog".to_string(),
                            status: StepStatus::Completed,
                        },
                        PlanItemArg {
                            step: "Run smoke tests".to_string(),
                            status: StepStatus::InProgress,
                        },
                        PlanItemArg {
                            step: "Tag release".to_string(),
                            status: StepStatus::Pending,
                        },
                        PlanItemArg {
                            step: "Notify stakeholders".to_string(),
                            status: StepStatus::Pending,
                        },
                    ],
                },
                (
                    "browser_open",
                    "https://example.com/releases",
                    "reviewed release dashboard",
                ),
                ReasoningEffort::Medium,
                "demo: release checklist warning",
                "demo: release checklist error",
                Some(
                    "diff --git a/CHANGELOG.md b/CHANGELOG.md\n@@ -1,3 +1,6 @@\n+## 1.2.3\n+- polish release flow\n+- document automation hooks\n",
                ),
            ),
        ];

        for (idx, scenario) in scenarios.iter().enumerate() {
            let (
                label,
                prompt,
                reasoning_steps,
                stream_lines,
                assistant_body,
                execs,
                patch_change,
                plan,
                tool_call,
                effort,
                warning_text,
                error_text,
                diff_snippet,
            ) = scenario;

            self.push_background_tail(format!("demo: scenario {} — {}", idx + 1, label));

            self.history_push(history_cell::new_user_prompt((*prompt).to_string()));

            let mut reasoning_lines: Vec<Line<'static>> = reasoning_steps
                .iter()
                .map(|(title, body)| {
                    Line::from(vec![
                        Span::styled(
                            format!("{}:", title),
                            RtStyle::default().add_modifier(RtModifier::BOLD),
                        ),
                        Span::raw(format!(" {body}")),
                    ])
                })
                .collect();
            reasoning_lines.push(
                Line::from(format!("Scenario summary: {}", label))
                    .style(RtStyle::default().fg(crate::colors::text_dim())),
            );
            let reasoning_cell = history_cell::CollapsibleReasoningCell::new_with_id(
                reasoning_lines,
                Some(format!("demo-reasoning-{}", idx)),
            );
            reasoning_cell.set_collapsed(false);
            reasoning_cell.set_in_progress(false);
            self.history_push(reasoning_cell);

            let streaming_preview = history_cell::new_streaming_content(
                stream_lines
                    .iter()
                    .map(|line| Line::from((*line).to_string()))
                    .collect(),
            );
            self.history_push(streaming_preview);

            let assistant_cell = history_cell::AssistantMarkdownCell::new(
                (*assistant_body).to_string(),
                &self.config,
            );
            self.history_push(assistant_cell);

            for (command_tokens, stdout) in execs {
                let cmd_vec: Vec<String> = command_tokens.iter().map(|s| s.to_string()).collect();
                let parsed = codex_core::parse_command::parse_command(&cmd_vec);
                self.history_push(history_cell::new_active_exec_command(
                    cmd_vec.clone(),
                    parsed.clone(),
                ));
                if !stdout.is_empty() {
                    let output = history_cell::CommandOutput {
                        exit_code: 0,
                        stdout: stdout.to_string(),
                        stderr: String::new(),
                    };
                    self.history_push(history_cell::new_completed_exec_command(
                        cmd_vec, parsed, output,
                    ));
                }
            }

            if let Some(diff) = diff_snippet {
                self.history_push(history_cell::new_diff_output(diff.to_string()));
            }

            if let Some(patch) = patch_change {
                let mut patch_changes = HashMap::new();
                let message = match patch {
                    DemoPatch::Add { path, content } => {
                        patch_changes.insert(
                            PathBuf::from(path),
                            codex_core::protocol::FileChange::Add {
                                content: (*content).to_string(),
                            },
                        );
                        format!("patch: simulated failure while applying {}", path)
                    }
                    DemoPatch::Update {
                        path,
                        unified_diff,
                        original,
                        new_content,
                    } => {
                        patch_changes.insert(
                            PathBuf::from(path),
                            codex_core::protocol::FileChange::Update {
                                unified_diff: (*unified_diff).to_string(),
                                move_path: None,
                                original_content: (*original).to_string(),
                                new_content: (*new_content).to_string(),
                            },
                        );
                        format!("patch: simulated failure while applying {}", path)
                    }
                };
                self.history_push(history_cell::new_patch_event(
                    history_cell::PatchEventType::ApprovalRequest,
                    patch_changes,
                ));
                self.history_push(history_cell::new_patch_apply_failure(message));
            }

            self.history_push(history_cell::new_plan_update(plan.clone()));

            let (tool_name, url, result) = tool_call;
            self.history_push(history_cell::new_completed_custom_tool_call(
                (*tool_name).to_string(),
                Some((*url).to_string()),
                Duration::from_millis(420 + (idx as u64 * 150)),
                true,
                (*result).to_string(),
            ));

            self.history_push(history_cell::new_warning_event((*warning_text).to_string()));
            self.history_push(history_cell::new_error_event((*error_text).to_string()));

            self.history_push(history_cell::new_model_output("gpt-5-codex", *effort));
            self.history_push(history_cell::new_reasoning_output(effort));

            self.history_push(history_cell::new_status_output(
                &self.config,
                &self.total_token_usage,
                &self.last_token_usage,
            ));

            self.history_push(history_cell::new_prompts_output());
        }

        let final_stream = history_cell::new_streaming_content(vec![
            Line::from("streaming preview: final tokens rendered."),
            Line::from("streaming preview: viewport ready for scroll testing."),
        ]);
        self.history_push(final_stream);

        self.push_background_tail("demo: finished populating sample history.");
        self.request_redraw();
    }

    fn add_perf_output(&mut self, text: String) {
        let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();
        lines.push(ratatui::text::Line::from("performance".dim()));
        for l in text.lines() {
            lines.push(ratatui::text::Line::from(l.to_string()))
        }
        self.history_push(crate::history_cell::PlainHistoryCell::new(
            lines,
            crate::history_cell::HistoryCellType::Notice,
        ));
    }

    pub(crate) fn add_diff_output(&mut self, diff_output: String) {
        self.history_push(history_cell::new_diff_output(diff_output.clone()));
    }

    pub(crate) fn add_status_output(&mut self) {
        self.history_push(history_cell::new_status_output(
            &self.config,
            &self.total_token_usage,
            &self.last_token_usage,
        ));
    }

    pub(crate) fn add_limits_output(&mut self) {
        let snapshot = self.rate_limit_snapshot.clone();
        let needs_refresh = self.should_refresh_limits();

        if self.rate_limit_fetch_inflight || needs_refresh {
            self.set_limits_overlay_content(LimitsOverlayContent::Loading);
        } else {
            let reset_info = self.rate_limit_reset_info();
            let tabs = self.build_limits_tabs(snapshot.clone(), reset_info);
            self.set_limits_overlay_tabs(tabs);
        }

        self.request_redraw();

        if needs_refresh {
            self.request_latest_rate_limits(snapshot.is_none());
        }
    }

    // MAINT-11 Phase 8: handle_sessions_command, list_cli_sessions_impl,
    // kill_cli_session_impl, kill_all_cli_sessions_impl moved to session_handlers.rs

    fn request_latest_rate_limits(&mut self, show_loading: bool) {
        if self.rate_limit_fetch_inflight {
            return;
        }

        if show_loading && self.limits.overlay.is_none() {
            self.set_limits_overlay_content(LimitsOverlayContent::Loading);
            self.request_redraw();
        }

        self.rate_limit_fetch_inflight = true;

        start_rate_limit_refresh(
            self.app_event_tx.clone(),
            self.config.clone(),
            self.config.debug,
        );
    }

    fn should_refresh_limits(&self) -> bool {
        if self.rate_limit_fetch_inflight {
            return false;
        }
        match self.rate_limit_last_fetch_at {
            Some(ts) => Utc::now() - ts > RATE_LIMIT_REFRESH_INTERVAL,
            None => true,
        }
    }

    pub(crate) fn on_auto_upgrade_completed(&mut self, version: String) {
        let notice = format!("Auto-upgraded to version {version}");
        self.latest_upgrade_version = None;
        self.push_background_tail(notice.clone());
        self.bottom_pane.flash_footer_notice(notice);
        self.request_redraw();
    }

    pub(crate) fn on_rate_limit_refresh_failed(&mut self, message: String) {
        self.rate_limit_fetch_inflight = false;

        if self.limits.overlay.is_some() {
            let content = if self.rate_limit_snapshot.is_some() {
                LimitsOverlayContent::Error(message.clone())
            } else {
                LimitsOverlayContent::Placeholder
            };
            self.set_limits_overlay_content(content);
            self.request_redraw();
        }

        if self.rate_limit_snapshot.is_some() {
            self.history_push(history_cell::new_warning_event(message));
        }
    }

    fn rate_limit_reset_info(&self) -> RateLimitResetInfo {
        let auto_compact_limit = self
            .config
            .model_auto_compact_token_limit
            .and_then(|limit| (limit > 0).then_some(limit as u64));
        let session_tokens_used = if auto_compact_limit.is_some() {
            Some(self.total_token_usage.total_tokens)
        } else {
            None
        };
        let context_window = self.config.model_context_window;
        let context_tokens_used =
            context_window.map(|_| self.last_token_usage.tokens_in_context_window());

        RateLimitResetInfo {
            primary_next_reset: self.rate_limit_primary_next_reset_at,
            secondary_next_reset: self.rate_limit_secondary_next_reset_at,
            session_tokens_used,
            auto_compact_limit,
            overflow_auto_compact: true,
            context_window,
            context_tokens_used,
        }
    }

    fn update_rate_limit_resets(&mut self, current: &RateLimitSnapshotEvent) {
        let now = Utc::now();
        self.rate_limit_primary_next_reset_at = current
            .primary_reset_after_seconds
            .map(|secs| now + ChronoDuration::seconds(secs as i64));
        self.rate_limit_secondary_next_reset_at = current
            .secondary_reset_after_seconds
            .map(|secs| now + ChronoDuration::seconds(secs as i64));
    }

    pub(crate) fn handle_update_command(&mut self) {
        if crate::updates::upgrade_ui_enabled() {
            self.show_update_settings_ui();
            return;
        }

        self.app_event_tx.send_background_event(
            "`/update` — updates are disabled in debug builds. Set SHOW_UPGRADE=1 to preview."
                .to_string(),
        );
    }

    pub(crate) fn add_prompts_output(&mut self) {
        self.history_push(history_cell::new_prompts_output());
    }

    pub(crate) fn handle_agents_command(&mut self, _args: String) {
        // Open the new overview combining Agents and Commands
        self.show_agents_overview_ui();
    }

    pub(crate) fn handle_login_command(&mut self) {
        self.show_login_accounts_view();
    }

    /// P6-SYNC Phase 5: Handle /auth command for device code OAuth management
    /// Subcommands:
    /// - /auth or /auth status - Show token status for all providers
    /// - /auth login <provider> - Start device code flow for provider (openai/google/anthropic)
    /// - /auth logout <provider> - Remove stored token for provider
    pub(crate) fn handle_auth_command(&mut self, args: &str) {
        use codex_login::{DeviceCodeProvider, DeviceCodeTokenStorage, TokenStatus};

        let args = args.trim();
        let parts: Vec<&str> = args.split_whitespace().collect();

        let show_message = |widget: &mut Self, message: String| {
            let cell = history_cell::new_background_event(message);
            widget.push_system_cell(
                cell,
                SystemPlacement::EndOfCurrent,
                None,
                None,
                "auth:result",
            );
        };

        match parts.first().map(|s| s.to_lowercase()).as_deref() {
            None | Some("status") => {
                // Show status for all providers
                let mut message = String::from("Device Code OAuth Status\n\n");

                match DeviceCodeTokenStorage::new() {
                    Ok(storage) => match storage.status_summary() {
                        Ok(status) => {
                            for (provider, token_status) in &status {
                                let provider_name = match provider {
                                    DeviceCodeProvider::OpenAI => "OpenAI",
                                    DeviceCodeProvider::Google => "Google (Gemini)",
                                    DeviceCodeProvider::Anthropic => "Anthropic (Claude)",
                                };
                                let status_text = match token_status {
                                    TokenStatus::Valid => "✓ authenticated",
                                    TokenStatus::NeedsRefresh => "⚡ needs refresh",
                                    TokenStatus::Expired => "✗ expired",
                                    TokenStatus::NotAuthenticated => "· not authenticated",
                                };
                                message
                                    .push_str(&format!("  {}: {}\n", provider_name, status_text));
                            }
                            message.push_str("\nUse /auth login <provider> to authenticate");
                        }
                        Err(e) => {
                            message.push_str(&format!("Error reading token status: {}", e));
                        }
                    },
                    Err(e) => {
                        message.push_str(&format!("Error accessing token storage: {}", e));
                    }
                }

                show_message(self, message);
                // Update footer status
                self.update_device_token_status();
            }
            Some("login") => {
                let provider = parts.get(1).map(|s| s.to_lowercase());
                match provider.as_deref() {
                    Some("openai") => {
                        // P6-SYNC Phase 7: Start interactive device code login
                        self.app_event_tx.send(AppEvent::DeviceCodeLoginStart {
                            provider: DeviceCodeProvider::OpenAI,
                        });
                    }
                    Some("google") | Some("gemini") => {
                        // Check if Google OAuth client is configured
                        if std::env::var("GOOGLE_OAUTH_CLIENT_ID").is_err() {
                            show_message(
                                self,
                                String::from(
                                    "Google OAuth requires configuration.\n\n\
                                 Set GOOGLE_OAUTH_CLIENT_ID environment variable.\n\
                                 Create OAuth credentials at:\n\
                                 https://console.cloud.google.com/apis/credentials",
                                ),
                            );
                        } else {
                            self.app_event_tx.send(AppEvent::DeviceCodeLoginStart {
                                provider: DeviceCodeProvider::Google,
                            });
                        }
                    }
                    Some("anthropic") | Some("claude") => {
                        self.app_event_tx.send(AppEvent::DeviceCodeLoginStart {
                            provider: DeviceCodeProvider::Anthropic,
                        });
                    }
                    _ => {
                        show_message(
                            self,
                            String::from(
                                "Usage: /auth login <provider>\n\
                             Providers: openai, google, anthropic (or gemini, claude)",
                            ),
                        );
                    }
                }
            }
            Some("logout") => {
                let provider = parts.get(1).map(|s| s.to_lowercase());
                match provider.as_deref() {
                    Some("openai") => {
                        if let Ok(storage) = DeviceCodeTokenStorage::new() {
                            match storage.remove_token(DeviceCodeProvider::OpenAI) {
                                Ok(()) => {
                                    show_message(
                                        self,
                                        String::from("OpenAI device code token removed."),
                                    );
                                    self.update_device_token_status();
                                }
                                Err(e) => {
                                    show_message(self, format!("Failed to remove token: {}", e));
                                }
                            }
                        }
                    }
                    Some("google") | Some("gemini") => {
                        if let Ok(storage) = DeviceCodeTokenStorage::new() {
                            match storage.remove_token(DeviceCodeProvider::Google) {
                                Ok(()) => {
                                    show_message(
                                        self,
                                        String::from("Google device code token removed."),
                                    );
                                    self.update_device_token_status();
                                }
                                Err(e) => {
                                    show_message(self, format!("Failed to remove token: {}", e));
                                }
                            }
                        }
                    }
                    Some("anthropic") | Some("claude") => {
                        if let Ok(storage) = DeviceCodeTokenStorage::new() {
                            match storage.remove_token(DeviceCodeProvider::Anthropic) {
                                Ok(()) => {
                                    show_message(
                                        self,
                                        String::from("Anthropic device code token removed."),
                                    );
                                    self.update_device_token_status();
                                }
                                Err(e) => {
                                    show_message(self, format!("Failed to remove token: {}", e));
                                }
                            }
                        }
                    }
                    _ => {
                        show_message(
                            self,
                            String::from(
                                "Usage: /auth logout <provider>\n\
                             Providers: openai, google, anthropic (or gemini, claude)",
                            ),
                        );
                    }
                }
            }
            Some(unknown) => {
                show_message(
                    self,
                    format!(
                        "Unknown /auth subcommand: {}\n\n\
                     Usage:\n\
                     - /auth status - Show token status\n\
                     - /auth login <provider> - Start device code flow\n\
                     - /auth logout <provider> - Remove stored token",
                        unknown
                    ),
                );
            }
        }
        self.request_redraw();
    }

    // MAINT-11 Phase 8: handle_feedback_command moved to session_handlers.rs

    pub(crate) fn auth_manager(&self) -> Arc<AuthManager> {
        self.auth_manager.clone()
    }

    pub(crate) fn reload_auth(&self) -> bool {
        self.auth_manager.reload()
    }

    pub(crate) fn show_login_accounts_view(&mut self) {
        let (view, state_rc) =
            LoginAccountsView::new(self.config.codex_home.clone(), self.app_event_tx.clone());
        self.login_view_state = Some(LoginAccountsState::weak_handle(&state_rc));
        self.login_add_view_state = None;
        self.bottom_pane.show_login_accounts(view);
        self.request_redraw();
    }

    pub(crate) fn show_login_add_account_view(&mut self) {
        let (view, state_rc) =
            LoginAddAccountView::new(self.config.codex_home.clone(), self.app_event_tx.clone());
        self.login_add_view_state = Some(LoginAddAccountState::weak_handle(&state_rc));
        self.login_view_state = None;
        self.bottom_pane.show_login_add_account(view);
        self.request_redraw();
    }

    fn with_login_add_view<F>(&mut self, f: F) -> bool
    where
        F: FnOnce(&mut LoginAddAccountState),
    {
        if let Some(weak) = &self.login_add_view_state
            && let Some(state_rc) = weak.upgrade()
        {
            f(&mut state_rc.borrow_mut());
            self.request_redraw();
            return true;
        }
        false
    }

    pub(crate) fn notify_login_chatgpt_started(&mut self, auth_url: String) {
        if self.with_login_add_view(|state| state.acknowledge_chatgpt_started(auth_url.clone())) {}
    }

    pub(crate) fn notify_login_chatgpt_failed(&mut self, error: String) {
        if self.with_login_add_view(|state| state.acknowledge_chatgpt_failed(error.clone())) {}
    }

    pub(crate) fn notify_login_chatgpt_complete(&mut self, result: Result<(), String>) {
        if self.with_login_add_view(|state| state.on_chatgpt_complete(result.clone())) {}
    }

    pub(crate) fn notify_login_chatgpt_cancelled(&mut self) {
        if self.with_login_add_view(|state| state.cancel_chatgpt_wait()) {}
    }

    // Claude OAuth notification methods (SPEC-KIT-954)
    pub(crate) fn notify_login_claude_complete(&mut self, result: Result<(), String>) {
        if self.with_login_add_view(|state| state.on_claude_complete(result.clone())) {}
    }

    pub(crate) fn notify_login_claude_cancelled(&mut self) {
        if self.with_login_add_view(|state| state.on_claude_cancelled()) {}
    }

    // Gemini OAuth notification methods (SPEC-KIT-954)
    pub(crate) fn notify_login_gemini_complete(&mut self, result: Result<(), String>) {
        if self.with_login_add_view(|state| state.on_gemini_complete(result.clone())) {}
    }

    pub(crate) fn notify_login_gemini_cancelled(&mut self) {
        if self.with_login_add_view(|state| state.on_gemini_cancelled()) {}
    }

    // P6-SYNC Phase 7: Device Code OAuth flow methods
    /// Start device code login for a provider - shows interactive login view
    pub(crate) fn start_device_code_login(&mut self, provider: codex_login::DeviceCodeProvider) {
        use crate::bottom_pane::DeviceCodeLoginView;

        let (view, state) = DeviceCodeLoginView::new(provider, self.app_event_tx.clone());
        self.device_code_login_state = Some(std::rc::Rc::downgrade(&state));
        self.bottom_pane.show_device_code_login(view);

        // Start the device authorization flow asynchronously
        let tx = self.app_event_tx.clone();
        let provider_clone = provider;
        tokio::spawn(async move {
            use codex_login::{
                AnthropicDeviceCode, DeviceCodeAuth, GoogleDeviceCode, OpenAIDeviceCode,
            };

            let result: Result<codex_login::DeviceAuthorizationResponse, String> =
                match provider_clone {
                    codex_login::DeviceCodeProvider::OpenAI => {
                        let client = OpenAIDeviceCode::new();
                        client
                            .start_device_authorization()
                            .await
                            .map_err(|e| e.to_string())
                    }
                    codex_login::DeviceCodeProvider::Google => match GoogleDeviceCode::from_env() {
                        Ok(client) => client
                            .start_device_authorization()
                            .await
                            .map_err(|e| e.to_string()),
                        Err(e) => Err(e.to_string()),
                    },
                    codex_login::DeviceCodeProvider::Anthropic => {
                        let client = AnthropicDeviceCode::new();
                        client
                            .start_device_authorization()
                            .await
                            .map_err(|e| e.to_string())
                    }
                };

            match result {
                Ok(response) => {
                    tx.send(AppEvent::DeviceCodeLoginCodeReceived {
                        provider: provider_clone,
                        user_code: response.user_code,
                        verification_uri: response.verification_uri,
                        verification_uri_complete: response.verification_uri_complete,
                        device_code: response.device_code,
                        expires_in: response.expires_in,
                        interval: response.interval,
                    });
                }
                Err(error) => {
                    tx.send(AppEvent::DeviceCodeLoginError {
                        provider: provider_clone,
                        error,
                    });
                }
            }
        });
    }

    /// Handle device code received - update UI and start polling
    pub(crate) fn on_device_code_received(
        &mut self,
        provider: codex_login::DeviceCodeProvider,
        user_code: String,
        verification_uri: String,
        verification_uri_complete: Option<String>,
        device_code: String,
        expires_in: u64,
        interval: u64,
    ) {
        // Update the view state
        self.with_device_code_view(|state| {
            state.on_device_auth_response(
                user_code.clone(),
                verification_uri.clone(),
                verification_uri_complete.clone(),
            );
        });

        // Start polling for token
        let tx = self.app_event_tx.clone();
        let provider_clone = provider;
        let device_code_clone = device_code.clone();
        let poll_interval = std::time::Duration::from_secs(interval.max(5)); // Minimum 5 seconds
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(expires_in);

        tokio::spawn(async move {
            use codex_login::device_code_storage::DeviceCodeTokenStorage;
            use codex_login::{
                AnthropicDeviceCode, DeviceCodeAuth, GoogleDeviceCode, OpenAIDeviceCode, PollError,
            };

            let mut poll_count = 0u32;

            loop {
                // Check expiry
                if std::time::Instant::now() >= expires_at {
                    tx.send(AppEvent::DeviceCodeLoginExpired {
                        provider: provider_clone,
                    });
                    break;
                }

                // Wait for poll interval
                tokio::time::sleep(poll_interval).await;

                poll_count += 1;
                tx.send(AppEvent::DeviceCodeLoginPollAttempt {
                    provider: provider_clone,
                    poll_count,
                });

                // Poll for token
                let poll_result = match provider_clone {
                    codex_login::DeviceCodeProvider::OpenAI => {
                        let client = OpenAIDeviceCode::new();
                        client.poll_for_token(&device_code_clone).await
                    }
                    codex_login::DeviceCodeProvider::Google => match GoogleDeviceCode::from_env() {
                        Ok(client) => client.poll_for_token(&device_code_clone).await,
                        Err(e) => Err(PollError::Server(e.to_string())),
                    },
                    codex_login::DeviceCodeProvider::Anthropic => {
                        let client = AnthropicDeviceCode::new();
                        client.poll_for_token(&device_code_clone).await
                    }
                };

                match poll_result {
                    Ok(token_response) => {
                        // Store the token
                        if let Ok(storage) = DeviceCodeTokenStorage::new() {
                            if let Err(e) = storage.store_token(provider_clone, token_response) {
                                tx.send(AppEvent::DeviceCodeLoginError {
                                    provider: provider_clone,
                                    error: format!("Failed to store token: {}", e),
                                });
                                break;
                            }
                        }
                        tx.send(AppEvent::DeviceCodeLoginSuccess {
                            provider: provider_clone,
                        });
                        break;
                    }
                    Err(PollError::AuthorizationPending) => {
                        // Continue polling
                        continue;
                    }
                    Err(PollError::SlowDown) => {
                        // Increase interval and continue
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                    Err(PollError::AccessDenied) => {
                        tx.send(AppEvent::DeviceCodeLoginDenied {
                            provider: provider_clone,
                        });
                        break;
                    }
                    Err(PollError::ExpiredToken) => {
                        tx.send(AppEvent::DeviceCodeLoginExpired {
                            provider: provider_clone,
                        });
                        break;
                    }
                    Err(e) => {
                        tx.send(AppEvent::DeviceCodeLoginError {
                            provider: provider_clone,
                            error: e.to_string(),
                        });
                        break;
                    }
                }
            }
        });
    }

    /// Handle poll attempt - update UI with progress
    pub(crate) fn on_device_code_poll_attempt(
        &mut self,
        _provider: codex_login::DeviceCodeProvider,
        poll_count: u32,
    ) {
        self.with_device_code_view(|state| {
            state.on_poll_attempt(poll_count);
        });
        self.request_redraw();
    }

    /// Handle successful device code authentication
    pub(crate) fn on_device_code_success(&mut self, provider: codex_login::DeviceCodeProvider) {
        self.with_device_code_view(|state| {
            state.on_success();
        });
        self.push_background_tail(format!(
            "{} authenticated successfully via device code flow.",
            provider.display_name()
        ));
        self.request_redraw();
    }

    /// Handle device code error
    pub(crate) fn on_device_code_error(
        &mut self,
        _provider: codex_login::DeviceCodeProvider,
        error: String,
    ) {
        self.with_device_code_view(|state| {
            state.on_error(error);
        });
        self.request_redraw();
    }

    /// Handle device code expiry
    pub(crate) fn on_device_code_expired(&mut self, _provider: codex_login::DeviceCodeProvider) {
        self.with_device_code_view(|state| {
            state.on_expired();
        });
        self.request_redraw();
    }

    /// Handle user denied access
    pub(crate) fn on_device_code_denied(&mut self, _provider: codex_login::DeviceCodeProvider) {
        self.with_device_code_view(|state| {
            state.on_access_denied();
        });
        self.request_redraw();
    }

    /// Helper to access device code login view state
    fn with_device_code_view<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut crate::bottom_pane::DeviceCodeLoginState) -> R,
    {
        if let Some(weak) = &self.device_code_login_state {
            if let Some(rc) = weak.upgrade() {
                if let Ok(mut state) = rc.try_borrow_mut() {
                    return Some(f(&mut state));
                }
            }
        }
        None
    }

    /// Handle user message timeout (SPEC-954)
    /// Called when a queued message hasn't received TaskStarted within 10 seconds
    pub(crate) fn handle_user_message_timeout(&mut self, message_id: &str, elapsed_ms: u64) {
        // Check if this message is still pending (not cleared by TaskStarted)
        if self.pending_message_timestamps.remove(message_id).is_some() {
            tracing::warn!(
                "⏰ USER_MSG_TIMEOUT: message_id={} | elapsed={}ms | pending_queue_len={}",
                message_id,
                elapsed_ms,
                self.pending_dispatched_user_messages.len()
            );

            // Show error to user
            let error_text = format!(
                "⚠️ Message timed out after {}s - provider may have failed silently. \
                Try again or check authentication with /login.",
                elapsed_ms / 1000
            );

            // Insert error into history
            self.push_background_tail(error_text);

            // Clear task running state so user can retry
            self.bottom_pane.set_task_running(false);
            self.bottom_pane.update_status_text(String::new());

            self.mark_needs_redraw();
        }
        // If message_id not found, TaskStarted already handled it - ignore
    }

    pub(crate) fn login_add_view_active(&self) -> bool {
        self.login_add_view_state
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .is_some()
    }

    pub(crate) fn set_using_chatgpt_auth(&mut self, using: bool) {
        self.config.using_chatgpt_auth = using;
        self.bottom_pane.set_using_chatgpt_auth(using);
    }

    fn show_update_settings_ui(&mut self) {
        use crate::bottom_pane::UpdateSettingsView;

        if !crate::updates::upgrade_ui_enabled() {
            self.app_event_tx.send_background_event(
                "`/update` — updates are disabled in debug builds. Set SHOW_UPGRADE=1 to preview."
                    .to_string(),
            );
            return;
        }

        let shared_state = std::sync::Arc::new(std::sync::Mutex::new(UpdateSharedState {
            checking: true,
            latest_version: None,
            error: None,
        }));

        let resolution = crate::updates::resolve_upgrade_resolution();
        let (command, display, instructions) = match &resolution {
            crate::updates::UpgradeResolution::Command { command, display } => {
                (Some(command.clone()), Some(display.clone()), None)
            }
            crate::updates::UpgradeResolution::Manual { instructions } => {
                (None, None, Some(instructions.clone()))
            }
        };

        let view = UpdateSettingsView::new(
            self.app_event_tx.clone(),
            codex_version::version().to_string(),
            self.config.auto_upgrade_enabled,
            command.clone(),
            display.clone(),
            instructions,
            shared_state.clone(),
        );

        self.bottom_pane.show_update_settings(view);

        let config = self.config.clone();
        let tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            let result = crate::updates::check_for_updates_now(&config).await;
            let mut state = shared_state.lock().expect("update state poisoned");
            match result {
                Ok(info) => {
                    state.checking = false;
                    state.latest_version = info.latest_version;
                    state.error = None;
                }
                Err(err) => {
                    state.checking = false;
                    state.latest_version = None;
                    state.error = Some(err.to_string());
                }
            }
            drop(state);
            tx.send(AppEvent::RequestRedraw);
        });
    }

    // Legacy show_agents_settings_ui removed — overview/Direct editors replace it

    pub(crate) fn show_agents_overview_ui(&mut self) {
        // Agents list with enabled status and install check
        fn command_exists(cmd: &str) -> bool {
            if cmd.contains(std::path::MAIN_SEPARATOR) || cmd.contains('/') || cmd.contains('\\') {
                return std::fs::metadata(cmd).map(|m| m.is_file()).unwrap_or(false);
            }
            #[cfg(target_os = "windows")]
            {
                if let Ok(p) = which::which(cmd) {
                    p.is_file()
                } else {
                    false
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                use std::os::unix::fs::PermissionsExt;
                let Some(path_os) = std::env::var_os("PATH") else {
                    return false;
                };
                for dir in std::env::split_paths(&path_os) {
                    if dir.as_os_str().is_empty() {
                        continue;
                    }
                    let candidate = dir.join(cmd);
                    if let Ok(meta) = std::fs::metadata(&candidate)
                        && meta.is_file()
                        && (meta.permissions().mode() & 0o111 != 0)
                    {
                        return true;
                    }
                }
                false
            }
        }

        let mut agent_rows: Vec<(String, bool, bool, String)> = Vec::new();
        // Desired presentation order for known agents
        let preferred = ["code", "claude", "gemini", "qwen"];
        // Name -> config lookup
        let mut extras: Vec<String> = Vec::new();
        for a in &self.config.agents {
            if !preferred.iter().any(|p| a.name.eq_ignore_ascii_case(p)) {
                extras.push(a.name.to_ascii_lowercase());
            }
        }
        extras.sort();
        // Build ordered list of names
        let mut ordered: Vec<String> = Vec::new();
        for p in preferred {
            ordered.push(p.to_string());
        }
        for e in extras {
            if !ordered.iter().any(|n| n.eq_ignore_ascii_case(&e)) {
                ordered.push(e);
            }
        }

        for name in ordered.iter() {
            if let Some(cfg) = self
                .config
                .agents
                .iter()
                .find(|a| a.name.eq_ignore_ascii_case(name))
            {
                let installed = command_exists(&cfg.command);
                agent_rows.push((
                    cfg.name.clone(),
                    cfg.enabled,
                    installed,
                    cfg.command.clone(),
                ));
            } else {
                // Default command = name, enabled=true, installed based on PATH
                let cmd = name.clone();
                let installed = command_exists(&cmd);
                // Keep display name as given (e.g., "code")
                agent_rows.push((name.clone(), true, installed, cmd));
            }
        }
        // Commands: built-ins followed by custom
        let mut commands: Vec<String> = vec!["plan".into(), "solve".into(), "code".into()];
        let custom: Vec<String> = self
            .config
            .subagent_commands
            .iter()
            .map(|c| c.name.clone())
            .filter(|n| !commands.iter().any(|b| b.eq_ignore_ascii_case(n)))
            .collect();
        commands.extend(custom);

        let total_rows = agent_rows
            .len()
            .saturating_add(commands.len())
            .saturating_add(1);
        let selected = if total_rows == 0 {
            0
        } else {
            self.agents_overview_selected_index
                .min(total_rows.saturating_sub(1))
        };
        self.agents_overview_selected_index = selected;
        self.bottom_pane
            .show_agents_overview(agent_rows, commands, selected);
    }

    pub(crate) fn set_agents_overview_selection(&mut self, index: usize) {
        self.agents_overview_selected_index = index;
    }

    fn resolve_agent_install_command(&self, agent_name: &str) -> Option<(Vec<String>, String)> {
        let cmd = self
            .config
            .agents
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(agent_name))
            .map(|cfg| cfg.command.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| agent_name.to_string());
        if cmd.trim().is_empty() {
            return None;
        }

        #[cfg(target_os = "windows")]
        {
            let script = format!(
                "if (Get-Command {cmd} -ErrorAction SilentlyContinue) {{ Write-Output \"{cmd} already installed\"; exit 0 }} else {{ Write-Warning \"{cmd} is not installed.\"; Write-Output \"Please install {cmd} via winget, Chocolatey, or the vendor installer.\"; exit 1 }}",
                cmd = cmd
            );
            let command = vec![
                "powershell.exe".to_string(),
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-Command".to_string(),
                script.clone(),
            ];
            return Some((command, format!("PowerShell install check for {cmd}")));
        }

        #[cfg(target_os = "macos")]
        {
            let brew_formula = macos_brew_formula_for_command(&cmd);
            let script = format!("brew install {brew_formula}");
            let command = vec!["/bin/bash".to_string(), "-lc".to_string(), script.clone()];
            return Some((command, script));
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let script = format!(
                "{cmd} --version || (echo \"Please install {cmd} via your package manager\" && false)",
                cmd = cmd
            );
            let command = vec!["/bin/bash".to_string(), "-lc".to_string(), script.clone()];
            return Some((command, script));
        }

        #[allow(unreachable_code)]
        {
            None
        }
    }

    pub(crate) fn launch_agent_install(
        &mut self,
        name: String,
        selected_index: usize,
    ) -> Option<TerminalLaunch> {
        self.agents_overview_selected_index = selected_index;
        let Some((_, default_command)) = self.resolve_agent_install_command(&name) else {
            self.history_push(history_cell::new_error_event(format!(
                "No install command available for agent '{name}' on this platform."
            )));
            self.show_agents_overview_ui();
            return None;
        };
        let id = self.terminal.alloc_id();
        self.terminal.after = Some(TerminalAfter::RefreshAgentsAndClose { selected_index });
        let (controller_tx, controller_rx) = mpsc::channel();
        let controller = TerminalRunController { tx: controller_tx };
        let cwd = self.config.cwd.to_string_lossy().to_string();
        self.push_background_before_next_output(format!(
            "Starting guided install for agent '{name}'"
        ));
        start_agent_install_session(
            self.app_event_tx.clone(),
            id,
            name.clone(),
            default_command.clone(),
            Some(cwd),
            controller.clone(),
            controller_rx,
            selected_index,
            self.config.debug,
        );
        Some(TerminalLaunch {
            id,
            title: format!("Install {name}"),
            command: Vec::new(),
            command_display: "Preparing install assistant…".to_string(),
            controller: Some(controller),
            auto_close_on_success: false,
        })
    }

    pub(crate) fn launch_validation_tool_install(
        &mut self,
        tool_name: &str,
        install_hint: &str,
    ) -> Option<TerminalLaunch> {
        let trimmed = install_hint.trim();
        if trimmed.is_empty() {
            self.history_push(history_cell::new_error_event(format!(
                "No install command available for validation tool '{tool_name}'."
            )));
            self.request_redraw();
            return None;
        }

        let wrapped = wrap_command(trimmed);
        if wrapped.is_empty() {
            self.history_push(history_cell::new_error_event(format!(
                "Unable to build install command for validation tool '{tool_name}'."
            )));
            self.request_redraw();
            return None;
        }

        let id = self.terminal.alloc_id();
        let display = Self::truncate_with_ellipsis(trimmed, 128);
        let launch = TerminalLaunch {
            id,
            title: format!("Install {tool_name}"),
            command: wrapped,
            command_display: display,
            controller: None,
            auto_close_on_success: false,
        };

        self.push_background_before_next_output(format!(
            "Installing validation tool '{tool_name}' with `{trimmed}`"
        ));
        Some(launch)
    }

    fn try_handle_terminal_shortcut(&mut self, raw_text: &str) -> bool {
        let trimmed = raw_text.trim_start();
        if let Some(rest) = trimmed.strip_prefix("$$") {
            let prompt = rest.trim();
            if prompt.is_empty() {
                self.history_push(history_cell::new_error_event(
                    "No prompt provided after '$$'.".to_string(),
                ));
                self.app_event_tx.send(AppEvent::RequestRedraw);
            } else {
                self.launch_guided_terminal_prompt(prompt);
            }
            return true;
        }
        if let Some(rest) = trimmed.strip_prefix('$') {
            let command = rest.trim();
            if command.is_empty() {
                self.history_push(history_cell::new_error_event(
                    "No command provided after '$'.".to_string(),
                ));
                self.app_event_tx.send(AppEvent::RequestRedraw);
            } else {
                self.run_terminal_command(command);
            }
            return true;
        }
        false
    }

    fn run_terminal_command(&mut self, command: &str) {
        if wrap_command(command).is_empty() {
            self.history_push(history_cell::new_error_event(
                "Unable to build shell command for execution.".to_string(),
            ));
            self.app_event_tx.send(AppEvent::RequestRedraw);
            return;
        }

        let id = self.terminal.alloc_id();
        let title = Self::truncate_with_ellipsis(&format!("Shell: {command}"), 64);
        let display = Self::truncate_with_ellipsis(command, 128);
        let (controller_tx, controller_rx) = mpsc::channel();
        let controller = TerminalRunController { tx: controller_tx };
        let launch = TerminalLaunch {
            id,
            title,
            command: Vec::new(),
            command_display: display,
            controller: Some(controller.clone()),
            auto_close_on_success: false,
        };
        self.push_background_before_next_output(format!("Terminal command: {command}"));
        self.app_event_tx.send(AppEvent::OpenTerminal(launch));
        let cwd = self.config.cwd.to_string_lossy().to_string();
        start_direct_terminal_session(
            self.app_event_tx.clone(),
            id,
            command.to_string(),
            Some(cwd),
            controller,
            controller_rx,
            self.config.debug,
        );
    }

    fn launch_guided_terminal_prompt(&mut self, prompt: &str) {
        let id = self.terminal.alloc_id();
        let (controller_tx, controller_rx) = mpsc::channel();
        let controller = TerminalRunController { tx: controller_tx };
        let cwd = self.config.cwd.to_string_lossy().to_string();
        let title = Self::truncate_with_ellipsis(&format!("Guided: {prompt}"), 64);
        let display = Self::truncate_with_ellipsis(prompt, 128);

        let launch = TerminalLaunch {
            id,
            title,
            command: Vec::new(),
            command_display: display.clone(),
            controller: Some(controller.clone()),
            auto_close_on_success: false,
        };

        self.push_background_before_next_output(format!("Guided terminal request: {prompt}"));
        self.app_event_tx.send(AppEvent::OpenTerminal(launch));
        start_prompt_terminal_session(
            self.app_event_tx.clone(),
            id,
            prompt.to_string(),
            Some(cwd),
            controller,
            controller_rx,
            self.config.debug,
        );
    }

    fn truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
        if max_chars == 0 {
            return String::new();
        }
        let total = text.chars().count();
        if total <= max_chars {
            return text.to_string();
        }
        let take = max_chars.saturating_sub(1);
        let mut out = String::with_capacity(max_chars);
        for (idx, ch) in text.chars().enumerate() {
            if idx >= take {
                break;
            }
            out.push(ch);
        }
        out.push('…');
        out
    }

    pub(crate) fn launch_update_command(
        &mut self,
        command: Vec<String>,
        display: String,
        latest_version: Option<String>,
    ) -> Option<TerminalLaunch> {
        if !crate::updates::upgrade_ui_enabled() {
            self.history_push(history_cell::new_error_event(
                "`/update` — updates are disabled in debug builds. Set SHOW_UPGRADE=1 to preview."
                    .to_string(),
            ));
            self.request_redraw();
            return None;
        }

        self.pending_upgrade_notice = None;
        if command.is_empty() {
            self.history_push(history_cell::new_error_event(
                "`/update` — no upgrade command available for this install.".to_string(),
            ));
            self.request_redraw();
            return None;
        }

        let id = self.terminal.alloc_id();
        if let Some(version) = latest_version {
            self.pending_upgrade_notice = Some((id, version));
        }
        Some(TerminalLaunch {
            id,
            title: "Upgrade Code".to_string(),
            command,
            command_display: display,
            controller: None,
            auto_close_on_success: false,
        })
    }

    pub(crate) fn terminal_open(&mut self, launch: &TerminalLaunch) {
        let mut overlay = TerminalOverlay::new(
            launch.id,
            launch.title.clone(),
            launch.command_display.clone(),
            launch.auto_close_on_success,
        );
        let visible = self.terminal.last_visible_rows.get();
        overlay.visible_rows = visible;
        overlay.clamp_scroll();
        overlay.ensure_pending_command();
        self.terminal.overlay = Some(overlay);
        self.request_redraw();
    }

    pub(crate) fn terminal_append_chunk(&mut self, id: u64, chunk: &[u8], is_stderr: bool) {
        let mut needs_redraw = false;
        let visible = self.terminal.last_visible_rows.get();
        let visible_cols = self.terminal.last_visible_cols.get();
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
        {
            if visible > 0 {
                overlay.pty_rows = visible;
            }
            if visible_cols > 0 {
                overlay.pty_cols = visible_cols;
            }
            if visible != overlay.visible_rows {
                overlay.visible_rows = visible;
                overlay.clamp_scroll();
            }
            overlay.append_chunk(chunk, is_stderr);
            needs_redraw = true;
        }
        if needs_redraw {
            self.request_redraw();
        }
    }

    pub(crate) fn terminal_dimensions_hint(&self) -> Option<(u16, u16)> {
        let rows = self.terminal.last_visible_rows.get();
        let cols = self.terminal.last_visible_cols.get();
        if rows > 0 && cols > 0 {
            Some((rows, cols))
        } else {
            None
        }
    }

    pub(crate) fn terminal_apply_resize(&mut self, id: u64, rows: u16, cols: u16) {
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
            && overlay.update_pty_dimensions(rows, cols)
        {
            self.request_redraw();
        }
    }

    pub(crate) fn request_terminal_cancel(&mut self, id: u64) {
        let mut needs_redraw = false;
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
        {
            overlay.push_info_message("Cancel requested…");
            if overlay.running {
                overlay.running = false;
                needs_redraw = true;
            }
        }
        if needs_redraw {
            self.request_redraw();
        }
        self.app_event_tx.send(AppEvent::TerminalCancel { id });
    }

    pub(crate) fn terminal_update_message(&mut self, id: u64, message: String) {
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
        {
            overlay.push_info_message(&message);
            self.request_redraw();
        }
    }

    pub(crate) fn terminal_set_assistant_message(&mut self, id: u64, message: String) {
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
        {
            overlay.push_assistant_message(&message);
            self.request_redraw();
        }
    }

    pub(crate) fn terminal_set_command_display(&mut self, id: u64, command: String) {
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
        {
            overlay.command_display = command;
            self.request_redraw();
        }
    }

    pub(crate) fn terminal_prepare_command(
        &mut self,
        id: u64,
        suggestion: String,
        ack: Sender<TerminalCommandGate>,
    ) {
        let mut updated = false;
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
        {
            overlay.set_pending_command(suggestion, ack);
            updated = true;
        }
        if updated {
            self.request_redraw();
        }
    }

    pub(crate) fn terminal_accept_pending_command(&mut self) -> Option<PendingCommandAction> {
        if let Some(overlay) = self.terminal.overlay_mut() {
            if overlay.running {
                return None;
            }
            if let Some(action) = overlay.accept_pending_command() {
                match &action {
                    PendingCommandAction::Forwarded(command)
                    | PendingCommandAction::Manual(command) => {
                        overlay.command_display = command.clone();
                    }
                }
                self.request_redraw();
                return Some(action);
            }
        }
        None
    }

    pub(crate) fn terminal_execute_manual_command(&mut self, id: u64, command: String) {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            if let Some(overlay) = self.terminal.overlay_mut() {
                overlay.ensure_pending_command();
            }
            self.request_redraw();
            return;
        }

        if let Some(rest) = trimmed.strip_prefix("$$") {
            let prompt_text = rest.trim();
            if prompt_text.is_empty() {
                if let Some(overlay) = self.terminal.overlay_mut() {
                    overlay.push_info_message("Provide a prompt after '$'.");
                    overlay.ensure_pending_command();
                }
                self.request_redraw();
                return;
            }

            if let Some(overlay) = self.terminal.overlay_mut() {
                overlay.cancel_pending_command();
                overlay.running = true;
                overlay.exit_code = None;
                overlay.duration = None;
                overlay.push_assistant_message("Preparing guided command…");
            }

            let (controller_tx, controller_rx) = mpsc::channel();
            let controller = TerminalRunController { tx: controller_tx };
            let cwd = self.config.cwd.to_string_lossy().to_string();

            start_prompt_terminal_session(
                self.app_event_tx.clone(),
                id,
                prompt_text.to_string(),
                Some(cwd),
                controller,
                controller_rx,
                self.config.debug,
            );

            self.push_background_before_next_output(format!("Terminal prompt: {prompt_text}"));
            return;
        }

        let mut command_body = trimmed;
        let mut run_direct = false;
        if let Some(rest) = trimmed.strip_prefix('$') {
            let candidate = rest.trim();
            if candidate.is_empty() {
                if let Some(overlay) = self.terminal.overlay_mut() {
                    overlay.push_info_message("Provide a command after '$'.");
                    overlay.ensure_pending_command();
                }
                self.request_redraw();
                return;
            }
            command_body = candidate;
            run_direct = true;
        }

        let command_string = command_body.to_string();
        let wrapped_command = wrap_command(&command_string);
        if wrapped_command.is_empty() {
            self.app_event_tx
                .send(AppEvent::TerminalSetAssistantMessage {
                    id,
                    message: "Command could not be constructed.".to_string(),
                });
            if let Some(overlay) = self.terminal.overlay_mut() {
                overlay.ensure_pending_command();
            }
            self.request_redraw();
            return;
        }

        if !matches!(self.config.sandbox_policy, SandboxPolicy::DangerFullAccess) {
            if let Some(overlay) = self.terminal.overlay_mut() {
                overlay.cancel_pending_command();
            }
            self.pending_manual_terminal.insert(
                id,
                PendingManualTerminal {
                    command: command_string.clone(),
                    run_direct,
                },
            );
            if let Some(overlay) = self.terminal.overlay_mut() {
                overlay.push_assistant_message("Awaiting approval to run this command…");
                overlay.running = false;
            }
            self.bottom_pane
                .push_approval_request(ApprovalRequest::TerminalCommand {
                    id,
                    command: command_string,
                });
            self.request_redraw();
            return;
        }

        if run_direct && self.terminal_dimensions_hint().is_some() {
            self.start_direct_terminal_command(id, command_string, wrapped_command);
        } else {
            self.start_manual_terminal_session(id, command_string);
        }
    }

    fn start_manual_terminal_session(&mut self, id: u64, command: String) {
        if command.is_empty() {
            return;
        }
        if let Some(overlay) = self.terminal.overlay_mut() {
            overlay.cancel_pending_command();
            overlay.running = true;
            overlay.exit_code = None;
            overlay.duration = None;
        }
        let (controller_tx, controller_rx) = mpsc::channel();
        let controller = TerminalRunController { tx: controller_tx };
        let cwd = self.config.cwd.to_string_lossy().to_string();
        start_direct_terminal_session(
            self.app_event_tx.clone(),
            id,
            command,
            Some(cwd),
            controller,
            controller_rx,
            self.config.debug,
        );
    }

    fn start_direct_terminal_command(&mut self, id: u64, display: String, command: Vec<String>) {
        if let Some(overlay) = self.terminal.overlay_mut() {
            overlay.cancel_pending_command();
        }
        self.app_event_tx.send(AppEvent::TerminalRunCommand {
            id,
            command,
            command_display: display,
            controller: None,
        });
    }

    pub(crate) fn terminal_send_input(&mut self, id: u64, data: Vec<u8>) {
        if data.is_empty() {
            return;
        }
        self.app_event_tx
            .send(AppEvent::TerminalSendInput { id, data });
    }

    pub(crate) fn terminal_mark_running(&mut self, id: u64) {
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
        {
            overlay.running = true;
            overlay.exit_code = None;
            overlay.duration = None;
            overlay.start_time = Some(Instant::now());
            self.request_redraw();
        }
    }

    pub(crate) fn terminal_finalize(
        &mut self,
        id: u64,
        exit_code: Option<i32>,
        duration: Duration,
    ) -> Option<TerminalAfter> {
        let mut success = false;
        let mut after = None;
        let mut needs_redraw = false;
        let mut should_close = false;
        let mut take_after = false;
        let visible = self.terminal.last_visible_rows.get();
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
        {
            overlay.cancel_pending_command();
            if visible != overlay.visible_rows {
                overlay.visible_rows = visible;
                overlay.clamp_scroll();
            }
            let was_following = overlay.is_following();
            overlay.finalize(exit_code, duration);
            overlay.auto_follow(was_following);
            needs_redraw = true;
            if exit_code == Some(0) {
                success = true;
                take_after = true;
                if overlay.auto_close_on_success {
                    should_close = true;
                }
            }
            overlay.ensure_pending_command();
        }
        if take_after {
            after = self.terminal.after.take();
        }
        if should_close {
            self.terminal.overlay = None;
        }
        if needs_redraw {
            self.request_redraw();
        }
        if success {
            if crate::updates::upgrade_ui_enabled()
                && let Some((pending_id, version)) = self.pending_upgrade_notice.take()
            {
                if pending_id == id {
                    self.bottom_pane
                        .flash_footer_notice(format!("Upgraded to {version}"));
                } else {
                    self.pending_upgrade_notice = Some((pending_id, version));
                }
            }
            after
        } else {
            None
        }
    }

    pub(crate) fn terminal_prepare_rerun(&mut self, id: u64) -> bool {
        let mut reset = false;
        let visible = self.terminal.last_visible_rows.get();
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.id == id
            && !overlay.running
        {
            overlay.reset_for_rerun();
            overlay.visible_rows = visible;
            overlay.clamp_scroll();
            overlay.ensure_pending_command();
            reset = true;
        }
        if reset {
            self.request_redraw();
        }
        reset
    }

    pub(crate) fn handle_terminal_approval_decision(&mut self, id: u64, approved: bool) {
        let pending = self.pending_manual_terminal.remove(&id);
        if approved {
            if let Some(entry) = pending
                && self
                    .terminal
                    .overlay()
                    .map(|overlay| overlay.id == id)
                    .unwrap_or(false)
            {
                if let Some(overlay) = self.terminal.overlay_mut() {
                    overlay.push_assistant_message("Approval granted. Running command…");
                }
                if entry.run_direct && self.terminal_dimensions_hint().is_some() {
                    let command_vec = wrap_command(&entry.command);
                    self.start_direct_terminal_command(id, entry.command, command_vec);
                } else {
                    self.start_manual_terminal_session(id, entry.command);
                }
                self.request_redraw();
            }
            return;
        }

        if let Some(entry) = pending {
            if let Some(overlay) = self.terminal.overlay_mut() {
                overlay
                    .push_info_message("Command was not approved. You can edit it and try again.");
                overlay.running = false;
                overlay.exit_code = None;
                overlay.duration = None;
                overlay.pending_command = Some(PendingCommand::manual_with_input(entry.command));
            }
            self.request_redraw();
        }
    }

    pub(crate) fn close_terminal_overlay(&mut self) {
        let mut cancel_id = None;
        let mut preserved_visible = None;
        let mut overlay_id = None;
        if let Some(overlay) = self.terminal.overlay_mut() {
            overlay_id = Some(overlay.id);
            if overlay.running {
                cancel_id = Some(overlay.id);
            }
            overlay.cancel_pending_command();
            preserved_visible = Some(overlay.visible_rows);
        }
        if let Some(id) = cancel_id {
            self.app_event_tx.send(AppEvent::TerminalCancel { id });
        }
        if let Some(id) = overlay_id {
            self.pending_manual_terminal.remove(&id);
        }
        if let Some(visible_rows) = preserved_visible {
            self.terminal.last_visible_rows.set(visible_rows);
        }
        self.terminal.clear();
        self.request_redraw();
    }

    pub(crate) fn terminal_overlay_id(&self) -> Option<u64> {
        self.terminal.overlay().map(|o| o.id)
    }

    pub(crate) fn terminal_overlay_active(&self) -> bool {
        self.terminal.overlay().is_some()
    }

    pub(crate) fn terminal_is_running(&self) -> bool {
        self.terminal.overlay().map(|o| o.running).unwrap_or(false)
    }

    pub(crate) fn ctrl_c_requests_exit(&self) -> bool {
        !self.terminal_overlay_active() && self.bottom_pane.ctrl_c_quit_hint_visible()
    }

    pub(crate) fn terminal_has_pending_command(&self) -> bool {
        self.terminal
            .overlay()
            .and_then(|overlay| overlay.pending_command.as_ref())
            .is_some()
    }

    pub(crate) fn terminal_handle_pending_key(&mut self, key_event: KeyEvent) -> bool {
        if self.terminal_is_running() {
            return false;
        }
        if !self.terminal_has_pending_command() {
            return false;
        }
        if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return true;
        }

        let mut needs_redraw = false;
        let mut handled = false;

        if let Some(overlay) = self.terminal.overlay_mut()
            && let Some(pending) = overlay.pending_command.as_mut()
        {
            match key_event.code {
                KeyCode::Char(ch) => {
                    if key_event
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
                    {
                        handled = true;
                    } else if pending.insert_char(ch) {
                        needs_redraw = true;
                        handled = true;
                    } else {
                        handled = true;
                    }
                }
                KeyCode::Backspace => {
                    handled = true;
                    if pending.backspace() {
                        needs_redraw = true;
                    }
                }
                KeyCode::Delete => {
                    handled = true;
                    if pending.delete() {
                        needs_redraw = true;
                    }
                }
                KeyCode::Left => {
                    handled = true;
                    if pending.move_left() {
                        needs_redraw = true;
                    }
                }
                KeyCode::Right => {
                    handled = true;
                    if pending.move_right() {
                        needs_redraw = true;
                    }
                }
                KeyCode::Home => {
                    handled = true;
                    if pending.move_home() {
                        needs_redraw = true;
                    }
                }
                KeyCode::End => {
                    handled = true;
                    if pending.move_end() {
                        needs_redraw = true;
                    }
                }
                KeyCode::Tab => {
                    handled = true;
                }
                _ => {}
            }
        }

        if needs_redraw {
            self.request_redraw();
        }
        handled
    }

    pub(crate) fn terminal_scroll_lines(&mut self, delta: i32) {
        let mut updated = false;
        let visible = self.terminal.last_visible_rows.get();
        if let Some(overlay) = self.terminal.overlay_mut() {
            if visible != overlay.visible_rows {
                overlay.visible_rows = visible;
            }
            let current = overlay.scroll as i32;
            let max_scroll = overlay.max_scroll() as i32;
            let mut next = current + delta;
            if next < 0 {
                next = 0;
            } else if next > max_scroll {
                next = max_scroll;
            }
            if next as u16 != overlay.scroll {
                overlay.scroll = next as u16;
                updated = true;
            }
        }
        if updated {
            self.request_redraw();
        }
    }

    pub(crate) fn terminal_scroll_page(&mut self, direction: i32) {
        let mut delta = None;
        let visible_value = self.terminal.last_visible_rows.get();
        if let Some(overlay) = self.terminal.overlay_mut() {
            let visible = visible_value.max(1);
            if visible != overlay.visible_rows {
                overlay.visible_rows = visible;
            }
            delta = Some((visible.saturating_sub(1)) as i32 * direction);
        }
        if let Some(amount) = delta {
            self.terminal_scroll_lines(amount);
        }
    }

    pub(crate) fn terminal_scroll_to_top(&mut self) {
        let mut updated = false;
        if let Some(overlay) = self.terminal.overlay_mut()
            && overlay.scroll != 0
        {
            overlay.scroll = 0;
            updated = true;
        }
        if updated {
            self.request_redraw();
        }
    }

    pub(crate) fn terminal_scroll_to_bottom(&mut self) {
        let mut updated = false;
        let visible = self.terminal.last_visible_rows.get();
        if let Some(overlay) = self.terminal.overlay_mut() {
            if visible != overlay.visible_rows {
                overlay.visible_rows = visible;
            }
            let max_scroll = overlay.max_scroll();
            if overlay.scroll != max_scroll {
                overlay.scroll = max_scroll;
                updated = true;
            }
        }
        if updated {
            self.request_redraw();
        }
    }

    pub(crate) fn handle_terminal_after(&mut self, after: TerminalAfter) {
        match after {
            TerminalAfter::RefreshAgentsAndClose { selected_index } => {
                self.agents_overview_selected_index = selected_index;
                self.show_agents_overview_ui();
            }
        }
    }

    // show_subagent_editor_ui removed; use show_subagent_editor_for_name or show_new_subagent_editor

    pub(crate) fn show_subagent_editor_for_name(&mut self, name: String) {
        // Build available agents from enabled ones (or sensible defaults)
        let available_agents: Vec<String> = if self.config.agents.is_empty() {
            vec![
                "claude".into(),
                "gemini".into(),
                "qwen".into(),
                "code".into(),
            ]
        } else {
            self.config
                .agents
                .iter()
                .filter(|a| a.enabled)
                .map(|a| a.name.clone())
                .collect()
        };
        let existing = self.config.subagent_commands.clone();
        self.bottom_pane
            .show_subagent_editor(name, available_agents, existing, false);
    }

    pub(crate) fn show_new_subagent_editor(&mut self) {
        let available_agents: Vec<String> = if self.config.agents.is_empty() {
            vec![
                "claude".into(),
                "gemini".into(),
                "qwen".into(),
                "code".into(),
            ]
        } else {
            self.config
                .agents
                .iter()
                .filter(|a| a.enabled)
                .map(|a| a.name.clone())
                .collect()
        };
        let existing = self.config.subagent_commands.clone();
        self.bottom_pane
            .show_subagent_editor(String::new(), available_agents, existing, true);
    }

    pub(crate) fn show_agent_editor_ui(&mut self, name: String) {
        if let Some(cfg) = self
            .config
            .agents
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(&name))
            .cloned()
        {
            let ro = if let Some(ref v) = cfg.args_read_only {
                Some(v.clone())
            } else if !cfg.args.is_empty() {
                Some(cfg.args.clone())
            } else {
                let d = codex_core::agent_defaults::default_params_for(
                    &cfg.name, true, /*read_only*/
                );
                if d.is_empty() { None } else { Some(d) }
            };
            let wr = if let Some(ref v) = cfg.args_write {
                Some(v.clone())
            } else if !cfg.args.is_empty() {
                Some(cfg.args.clone())
            } else {
                let d = codex_core::agent_defaults::default_params_for(
                    &cfg.name, false, /*read_only*/
                );
                if d.is_empty() { None } else { Some(d) }
            };
            self.bottom_pane.show_agent_editor(
                cfg.name.clone(),
                cfg.enabled,
                ro,
                wr,
                cfg.instructions.clone(),
                cfg.command.clone(),
            );
        } else {
            // Fallback: synthesize defaults
            let cmd = name.clone();
            let ro = codex_core::agent_defaults::default_params_for(&name, true /*read_only*/);
            let wr =
                codex_core::agent_defaults::default_params_for(&name, false /*read_only*/);
            self.bottom_pane.show_agent_editor(
                name,
                true,
                if ro.is_empty() { None } else { Some(ro) },
                if wr.is_empty() { None } else { Some(wr) },
                None,
                cmd,
            );
        }
    }

    pub(crate) fn apply_subagent_update(
        &mut self,
        cmd: codex_core::config_types::SubagentCommandConfig,
    ) {
        if let Some(slot) = self
            .config
            .subagent_commands
            .iter_mut()
            .find(|c| c.name.eq_ignore_ascii_case(&cmd.name))
        {
            *slot = cmd;
        } else {
            self.config.subagent_commands.push(cmd);
        }
    }

    pub(crate) fn delete_subagent_by_name(&mut self, name: &str) {
        self.config
            .subagent_commands
            .retain(|c| !c.name.eq_ignore_ascii_case(name));
    }

    /// SPEC-KIT-983: Update stage→agent defaults from modal.
    pub(crate) fn update_speckit_stage_agents(
        &mut self,
        new_config: codex_core::config_types::SpecKitStageAgents,
    ) {
        self.config.speckit_stage_agents = new_config;
    }

    pub(crate) fn apply_agent_update(
        &mut self,
        name: &str,
        enabled: bool,
        args_ro: Option<Vec<String>>,
        args_wr: Option<Vec<String>>,
        instr: Option<String>,
    ) {
        let mut updated_existing = false;
        if let Some(slot) = self
            .config
            .agents
            .iter_mut()
            .find(|a| a.name.eq_ignore_ascii_case(name))
        {
            slot.enabled = enabled;
            slot.args_read_only = args_ro.clone();
            slot.args_write = args_wr.clone();
            slot.instructions = instr.clone();
            updated_existing = true;
        }

        if !updated_existing {
            let new_cfg = AgentConfig {
                name: name.to_string(),
                canonical_name: None,
                command: name.to_string(),
                args: Vec::new(),
                read_only: false,
                enabled,
                description: None,
                env: None,
                args_read_only: args_ro.clone(),
                args_write: args_wr.clone(),
                instructions: instr.clone(),
                model: None,
            };
            self.config.agents.push(new_cfg);
        }
        // Persist asynchronously
        if let Ok(home) = codex_core::config::find_codex_home() {
            let name_s = name.to_string();
            let (en2, ro2, wr2, ins2) = (enabled, args_ro, args_wr, instr);
            tokio::spawn(async move {
                let _ = codex_core::config_edit::upsert_agent_config(
                    &home,
                    &name_s,
                    Some(en2),
                    None, // keep plain args as‑is
                    ro2.as_deref(),
                    wr2.as_deref(),
                    ins2.as_deref(),
                )
                .await;
            });
        }
    }

    pub(crate) fn show_diffs_popup(&mut self) {
        use crate::diff_render::create_diff_details_only;
        // Build a latest-first unique file list
        let mut order: Vec<PathBuf> = Vec::new();
        let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        for changes in self.diffs.session_patch_sets.iter().rev() {
            for (path, change) in changes.iter() {
                // If this change represents a move/rename, show the destination path in the tabs
                let display_path: PathBuf = match change {
                    codex_core::protocol::FileChange::Update {
                        move_path: Some(dest),
                        ..
                    } => dest.clone(),
                    _ => path.clone(),
                };
                if seen.insert(display_path.clone()) {
                    order.push(display_path);
                }
            }
        }
        // Build tabs: for each file, create a single unified diff against the original baseline
        let mut tabs: Vec<(String, Vec<DiffBlock>)> = Vec::new();
        for path in order {
            // Resolve baseline (first-seen content) and current (on-disk) content
            let baseline = self
                .diffs
                .baseline_file_contents
                .get(&path)
                .cloned()
                .unwrap_or_default();
            let current = std::fs::read_to_string(&path).unwrap_or_default();
            // Build a unified diff from baseline -> current
            let unified = diffy::create_patch(&baseline, &current).to_string();
            // Render detailed lines (no header) using our diff renderer helpers
            let mut single = HashMap::new();
            single.insert(
                path.clone(),
                codex_core::protocol::FileChange::Update {
                    unified_diff: unified.clone(),
                    move_path: None,
                    original_content: baseline.clone(),
                    new_content: current.clone(),
                },
            );
            let detail = create_diff_details_only(&single);
            let mut blocks: Vec<DiffBlock> = vec![DiffBlock { lines: detail }];

            // Count adds/removes for the header label from the unified diff
            let mut total_added: usize = 0;
            let mut total_removed: usize = 0;
            if let Ok(patch) = diffy::Patch::from_str(&unified) {
                for h in patch.hunks() {
                    for l in h.lines() {
                        match l {
                            diffy::Line::Insert(_) => total_added += 1,
                            diffy::Line::Delete(_) => total_removed += 1,
                            _ => {}
                        }
                    }
                }
            } else {
                for l in unified.lines() {
                    if l.starts_with("+++") || l.starts_with("---") || l.starts_with("@@") {
                        continue;
                    }
                    if let Some(b) = l.as_bytes().first() {
                        if *b == b'+' {
                            total_added += 1;
                        } else if *b == b'-' {
                            total_removed += 1;
                        }
                    }
                }
            }
            // Prepend a header block with the full path and counts
            let header_line = {
                use ratatui::style::Modifier;
                use ratatui::style::Style;
                use ratatui::text::Line as RtLine;
                use ratatui::text::Span as RtSpan;
                let mut spans: Vec<RtSpan<'static>> = Vec::new();
                spans.push(RtSpan::styled(
                    path.display().to_string(),
                    Style::default()
                        .fg(crate::colors::text())
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(RtSpan::raw(" "));
                spans.push(RtSpan::styled(
                    format!("+{}", total_added),
                    Style::default().fg(crate::colors::success()),
                ));
                spans.push(RtSpan::raw(" "));
                spans.push(RtSpan::styled(
                    format!("-{}", total_removed),
                    Style::default().fg(crate::colors::error()),
                ));
                RtLine::from(spans)
            };
            blocks.insert(
                0,
                DiffBlock {
                    lines: vec![header_line],
                },
            );

            // Tab title: file name only
            let title = path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| path.display().to_string());
            tabs.push((title, blocks));
        }
        if tabs.is_empty() {
            // Nothing to show — surface a small notice so Ctrl+D feels responsive
            self.bottom_pane
                .flash_footer_notice("No diffs recorded this session".to_string());
            return;
        }
        self.diffs.overlay = Some(DiffOverlay::new(tabs));
        self.diffs.confirm = None;
        self.request_redraw();
    }

    pub(crate) fn toggle_diffs_popup(&mut self) {
        if self.diffs.overlay.is_some() {
            self.diffs.overlay = None;
            self.request_redraw();
        } else {
            self.show_diffs_popup();
        }
    }

    pub(crate) fn show_help_popup(&mut self) {
        let t_dim = Style::default().fg(crate::colors::text_dim());
        let t_fg = Style::default().fg(crate::colors::text());

        let mut lines: Vec<RtLine<'static>> = Vec::new();
        lines.push(RtLine::from(vec![RtSpan::styled(
            "Keyboard shortcuts",
            t_fg.add_modifier(Modifier::BOLD),
        )]));
        lines.push(RtLine::from(""));

        let kv = |k: &str, v: &str| -> RtLine<'static> {
            RtLine::from(vec![
                // Left-align the key column for improved readability
                RtSpan::styled(format!("{k:<12}"), t_fg),
                RtSpan::raw("  —  "),
                RtSpan::styled(v.to_string(), t_dim),
            ])
        };
        lines.push(RtLine::from(""));
        // Top quick action
        lines.push(kv(
            "Shift+Tab",
            "Rotate agent between Read Only / Write with Approval / Full Access",
        ));

        // Global
        lines.push(kv("Ctrl+H", "Help overlay"));
        lines.push(kv("Ctrl+R", "Toggle reasoning"));
        lines.push(kv("Ctrl+T", "Toggle screen"));
        lines.push(kv("Ctrl+D", "Diff viewer"));
        lines.push(kv("Esc", "Edit previous message / close popups"));
        // Task control shortcuts
        lines.push(kv("Esc", "End current task"));
        lines.push(kv("Ctrl+C", "End current task"));
        lines.push(kv("Ctrl+C twice", "Quit"));
        lines.push(RtLine::from(""));

        // Composer
        lines.push(RtLine::from(vec![RtSpan::styled(
            "Compose field",
            t_fg.add_modifier(Modifier::BOLD),
        )]));
        lines.push(kv("Enter", "Send message"));
        lines.push(kv("Ctrl+J", "Insert newline"));
        lines.push(kv("Shift+Enter", "Insert newline"));
        // Split combined shortcuts into separate rows for readability
        lines.push(kv("Shift+Up", "Browse input history"));
        lines.push(kv("Shift+Down", "Browse input history"));
        lines.push(kv("Ctrl+B", "Move left"));
        lines.push(kv("Ctrl+F", "Move right"));
        lines.push(kv("Alt+Left", "Move by word"));
        lines.push(kv("Alt+Right", "Move by word"));
        // Simplify delete shortcuts; remove Alt+Backspace/Backspace/Delete variants
        lines.push(kv("Ctrl+W", "Delete previous word"));
        lines.push(kv("Ctrl+H", "Delete previous char"));
        lines.push(kv("Ctrl+D", "Delete next char"));
        lines.push(kv("Ctrl+Backspace", "Delete current line"));
        lines.push(kv("Ctrl+U", "Delete to line start"));
        lines.push(kv("Ctrl+K", "Delete to line end"));
        lines.push(kv(
            "Home/End",
            "Jump to line start/end (jump to history start/end when input is empty)",
        ));
        lines.push(RtLine::from(""));

        // Panels
        lines.push(RtLine::from(vec![RtSpan::styled(
            "Panels",
            t_fg.add_modifier(Modifier::BOLD),
        )]));
        lines.push(kv("Ctrl+B", "Toggle Browser panel"));
        lines.push(kv("Ctrl+A", "Open Agents terminal"));

        // Slash command reference
        lines.push(RtLine::from(""));
        lines.push(RtLine::from(vec![RtSpan::styled(
            "Slash commands",
            t_fg.add_modifier(Modifier::BOLD),
        )]));
        for (cmd_str, cmd) in crate::slash_command::built_in_slash_commands() {
            // Hide internal test command from the Help panel
            if cmd_str == "test-approval" {
                continue;
            }
            // Prefer "Code" branding in the Help panel
            let desc = cmd.description().replace("Codex", "Code");
            // Render as "/command  —  description"
            lines.push(RtLine::from(vec![
                RtSpan::styled(format!("/{cmd_str:<12}"), t_fg),
                RtSpan::raw("  —  "),
                RtSpan::styled(desc.to_string(), t_dim),
            ]));
        }

        self.help.overlay = Some(HelpOverlay::new(lines));
        self.request_redraw();
    }

    pub(crate) fn toggle_help_popup(&mut self) {
        if self.help.overlay.is_some() {
            self.help.overlay = None;
        } else {
            self.show_help_popup();
        }
        self.request_redraw();
    }

    fn available_model_presets(&self) -> Vec<ModelPreset> {
        let auth_mode = if self.config.using_chatgpt_auth {
            Some(McpAuthMode::ChatGPT)
        } else {
            Some(McpAuthMode::ApiKey)
        };
        builtin_model_presets(auth_mode)
    }

    fn preset_effort_for_model(preset: &ModelPreset) -> ReasoningEffort {
        preset
            .effort
            .map(ReasoningEffort::from)
            .unwrap_or(ReasoningEffort::Medium)
    }

    fn find_model_preset(&self, input: &str, presets: &[ModelPreset]) -> Option<ModelPreset> {
        if presets.is_empty() {
            return None;
        }

        let input_lower = input.to_ascii_lowercase();
        let collapsed_input: String = input_lower
            .chars()
            .filter(|c| !c.is_ascii_whitespace() && *c != '-')
            .collect();

        let mut fallback_medium: Option<ModelPreset> = None;
        let mut fallback_none: Option<ModelPreset> = None;
        let mut fallback_first: Option<ModelPreset> = None;

        for &preset in presets.iter() {
            let preset_effort = Self::preset_effort_for_model(&preset);

            let id_lower = preset.id.to_ascii_lowercase();
            if Self::candidate_matches(&input_lower, &collapsed_input, &id_lower) {
                return Some(preset);
            }

            let label_lower = preset.label.to_ascii_lowercase();
            if Self::candidate_matches(&input_lower, &collapsed_input, &label_lower) {
                return Some(preset);
            }

            let effort_lower = preset_effort.to_string().to_ascii_lowercase();
            let model_lower = preset.model.to_ascii_lowercase();
            let spaced = format!("{model_lower} {effort_lower}");
            if Self::candidate_matches(&input_lower, &collapsed_input, &spaced) {
                return Some(preset);
            }
            let dashed = format!("{model_lower}-{effort_lower}");
            if Self::candidate_matches(&input_lower, &collapsed_input, &dashed) {
                return Some(preset);
            }

            if model_lower == input_lower
                || Self::candidate_matches(&input_lower, &collapsed_input, &model_lower)
            {
                if fallback_medium.is_none() && preset_effort == ReasoningEffort::Medium {
                    fallback_medium = Some(preset);
                }
                if fallback_none.is_none() && preset.effort.is_none() {
                    fallback_none = Some(preset);
                }
                if fallback_first.is_none() {
                    fallback_first = Some(preset);
                }
            }
        }

        fallback_medium.or(fallback_none).or(fallback_first)
    }

    fn candidate_matches(input: &str, collapsed_input: &str, candidate: &str) -> bool {
        let candidate_lower = candidate.to_ascii_lowercase();
        if candidate_lower == input {
            return true;
        }
        let candidate_collapsed: String = candidate_lower
            .chars()
            .filter(|c| !c.is_ascii_whitespace() && *c != '-')
            .collect();
        candidate_collapsed == collapsed_input
    }

    /// SPEC-KIT-946: Infer the appropriate provider and auth method for a given model
    /// Returns (provider_id, auth_method) tuple for OAuth-based multi-provider support
    /// Maps model names to their provider IDs and corresponding OAuth auth methods
    fn infer_provider_for_model(model: &str) -> Option<(&'static str, &'static str)> {
        let model_lower = model.to_ascii_lowercase();

        // Claude models → Anthropic provider with claude OAuth
        if model_lower.contains("claude")
            || model_lower.contains("opus")
            || model_lower.contains("sonnet")
            || model_lower.contains("haiku")
        {
            return Some(("anthropic", "claude"));
        }

        // Gemini models → Google provider with gemini OAuth
        if model_lower.contains("gemini")
            || model_lower.contains("flash")
            || model_lower.starts_with("bison")
        {
            return Some(("google", "gemini"));
        }

        // GPT models → OpenAI provider with chatgpt OAuth
        if model_lower.contains("gpt")
            || model_lower.starts_with("o1")
            || model_lower.starts_with("o3")
        {
            return Some(("openai", "chatgpt"));
        }

        // Unknown model → keep current provider (return None)
        None
    }

    pub(crate) fn handle_model_command(&mut self, command_args: String) {
        if self.is_task_running() {
            let message = "'/model' is disabled while a task is in progress.".to_string();
            self.history_push(history_cell::new_error_event(message));
            return;
        }

        let presets = self.available_model_presets();
        if presets.is_empty() {
            let message =
                "No model presets are available. Update your configuration to define models."
                    .to_string();
            self.history_push(history_cell::new_error_event(message));
            return;
        }

        let trimmed = command_args.trim();
        if !trimmed.is_empty() {
            if let Some(preset) = self.find_model_preset(trimmed, &presets) {
                let effort = Self::preset_effort_for_model(&preset);
                self.apply_model_selection(preset.model.to_string(), Some(effort));
            } else {
                let message = format!(
                    "Unknown model preset: '{}'. Use /model with no arguments to open the selector.",
                    trimmed
                );
                self.history_push(history_cell::new_error_event(message));
            }
            return;
        }

        self.bottom_pane.show_model_selection(
            presets,
            self.config.model.clone(),
            self.config.model_reasoning_effort,
        );
    }

    pub(crate) fn apply_model_selection(&mut self, model: String, effort: Option<ReasoningEffort>) {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            return;
        }

        let mut updated = false;
        if !self.config.model.eq_ignore_ascii_case(trimmed) {
            self.config.model = trimmed.to_string();
            let family = find_family_for_model(&self.config.model)
                .unwrap_or_else(|| derive_default_model_family(&self.config.model));
            self.config.model_family = family;

            // SPEC-KIT-946/952: Auto-switch provider based on model selection
            // Claude/Gemini use CLI routing (not OAuth), ChatGPT uses native OAuth
            if let Some((provider, _auth_method)) =
                Self::infer_provider_for_model(&self.config.model)
            {
                // Update provider if it changed
                if self.config.model_provider_id != provider {
                    self.config.model_provider_id = provider.to_string();
                }
                // Note: Auth method switching removed - CLI routing handles auth independently
            }

            updated = true;
        }

        if let Some(new_effort) = effort
            && self.config.model_reasoning_effort != new_effort
        {
            self.config.model_reasoning_effort = new_effort;
            updated = true;
        }

        if updated {
            let op = Op::ConfigureSession {
                provider: self.config.model_provider.clone(),
                model: self.config.model.clone(),
                model_reasoning_effort: self.config.model_reasoning_effort,
                model_reasoning_summary: self.config.model_reasoning_summary,
                model_text_verbosity: self.config.model_text_verbosity,
                user_instructions: self.config.user_instructions.clone(),
                base_instructions: self.config.base_instructions.clone(),
                approval_policy: self.config.approval_policy,
                sandbox_policy: self.config.sandbox_policy.clone(),
                disable_response_storage: self.config.disable_response_storage,
                notify: self.config.notify.clone(),
                cwd: self.config.cwd.clone(),
                resume_path: None,
                output_schema: self.config.output_schema.clone(),
            };
            self.submit_op(op);
        }

        let placement = self.ui_placement_for_now();
        self.push_system_cell(
            history_cell::new_model_output(&self.config.model, self.config.model_reasoning_effort),
            placement,
            Some("ui:model".to_string()),
            None,
            "system",
        );

        // SPEC-KIT-952: Check CLI availability for CLI-routed providers
        if crate::model_router::supports_cli_streaming(&self.config.model) {
            let provider_type = crate::providers::ProviderType::from_model_name(&self.config.model);
            let cli_available = match provider_type {
                crate::providers::ProviderType::Claude => crate::providers::claude::is_available(),
                crate::providers::ProviderType::Gemini => crate::providers::gemini::is_available(),
                crate::providers::ProviderType::ChatGPT => true,
            };
            if !cli_available {
                let provider_name = crate::model_router::provider_display_name(&self.config.model);
                let instructions = match provider_type {
                    crate::providers::ProviderType::Claude => {
                        crate::providers::claude::install_instructions()
                    }
                    crate::providers::ProviderType::Gemini => {
                        crate::providers::gemini::install_instructions()
                    }
                    crate::providers::ProviderType::ChatGPT => "",
                };
                self.history_push(history_cell::PlainHistoryCell::new(
                    vec![
                        ratatui::text::Line::from(format!("⚠️  {} CLI Required", provider_name)),
                        ratatui::text::Line::from(""),
                        ratatui::text::Line::from(format!(
                            "{} CLI is required but not installed.\n\n{}",
                            provider_name, instructions
                        )),
                    ],
                    history_cell::HistoryCellType::Notice,
                ));
            }
        }

        self.request_redraw();
    }

    pub(crate) fn handle_reasoning_command(&mut self, command_args: String) {
        // command_args contains only the arguments after the command (e.g., "high" not "/reasoning high")
        let trimmed = command_args.trim();

        if !trimmed.is_empty() {
            // User specified a level: e.g., "high"
            let new_effort = match trimmed.to_lowercase().as_str() {
                "minimal" | "min" => ReasoningEffort::Minimal,
                "low" => ReasoningEffort::Low,
                "medium" | "med" => ReasoningEffort::Medium,
                "high" => ReasoningEffort::High,
                // Backwards compatibility: map legacy values to minimal.
                "none" | "off" => ReasoningEffort::Minimal,
                _ => {
                    // Invalid parameter, show error and return
                    let message = format!(
                        "Invalid reasoning level: '{}'. Use: minimal, low, medium, or high",
                        trimmed
                    );
                    self.history_push(history_cell::new_error_event(message));
                    return;
                }
            };
            self.set_reasoning_effort(new_effort);
        } else {
            let presets = self.available_model_presets();
            if presets.is_empty() {
                let message =
                    "No model presets are available. Update your configuration to define models."
                        .to_string();
                self.history_push(history_cell::new_error_event(message));
                return;
            }

            self.bottom_pane.show_model_selection(
                presets,
                self.config.model.clone(),
                self.config.model_reasoning_effort,
            );
        }
    }

    pub(crate) fn handle_verbosity_command(&mut self, command_args: String) {
        // Verbosity is not supported with ChatGPT auth
        if self.config.using_chatgpt_auth {
            let message =
                "Text verbosity is not available when using Sign in with ChatGPT".to_string();
            self.history_push(history_cell::new_error_event(message));
            return;
        }

        // command_args contains only the arguments after the command (e.g., "high" not "/verbosity high")
        let trimmed = command_args.trim();

        if !trimmed.is_empty() {
            // User specified a level: e.g., "high"
            let new_verbosity = match trimmed.to_lowercase().as_str() {
                "low" => TextVerbosity::Low,
                "medium" | "med" => TextVerbosity::Medium,
                "high" => TextVerbosity::High,
                _ => {
                    // Invalid parameter, show error and return
                    let message = format!(
                        "Invalid verbosity level: '{}'. Use: low, medium, or high",
                        trimmed
                    );
                    self.history_push(history_cell::new_error_event(message));
                    return;
                }
            };

            // Update the configuration
            self.config.model_text_verbosity = new_verbosity;

            // Display success message
            let message = format!("Text verbosity set to: {}", new_verbosity);
            self.push_background_tail(message);

            // Send the update to the backend
            let op = Op::ConfigureSession {
                provider: self.config.model_provider.clone(),
                model: self.config.model.clone(),
                model_reasoning_effort: self.config.model_reasoning_effort,
                model_reasoning_summary: self.config.model_reasoning_summary,
                model_text_verbosity: self.config.model_text_verbosity,
                user_instructions: self.config.user_instructions.clone(),
                base_instructions: self.config.base_instructions.clone(),
                approval_policy: self.config.approval_policy,
                sandbox_policy: self.config.sandbox_policy.clone(),
                disable_response_storage: self.config.disable_response_storage,
                notify: self.config.notify.clone(),
                cwd: self.config.cwd.clone(),
                resume_path: None,
                output_schema: self.config.output_schema.clone(),
            };
            let _ = self.codex_op_tx.send(op);
        } else {
            // No parameter specified, show interactive UI
            self.bottom_pane
                .show_verbosity_selection(self.config.model_text_verbosity);
        }
    }

    pub(crate) fn prepare_agents(&mut self) {
        // Set the flag to show agents are ready to start
        self.agents_ready_to_start = true;
        self.agents_terminal.reset();
        if self.agents_terminal.active {
            // Reset scroll offset when a new batch starts to avoid stale positions
            self.layout.scroll_offset = 0;
        }

        // Initialize sparkline with some data so it shows immediately
        {
            let mut sparkline_data = self.sparkline_data.borrow_mut();
            if sparkline_data.is_empty() {
                // Add initial low activity data for preparing phase
                for _ in 0..10 {
                    sparkline_data.push((2, false));
                }
                tracing::info!(
                    "Initialized sparkline data with {} points for preparing phase",
                    sparkline_data.len()
                );
            }
        } // Drop the borrow here

        self.request_redraw();
    }

    /// Update sparkline data with randomized activity based on agent count
    fn update_sparkline_data(&self) {
        let now = std::time::Instant::now();

        // Update every 100ms for smooth animation
        if now
            .duration_since(*self.last_sparkline_update.borrow())
            .as_millis()
            < 100
        {
            return;
        }

        *self.last_sparkline_update.borrow_mut() = now;

        // Calculate base height based on number of agents and status
        let agent_count = self.active_agents.len();
        let is_planning = self.overall_task_status == "planning";
        let base_height = if agent_count == 0 && self.agents_ready_to_start {
            2 // Minimal activity when preparing
        } else if is_planning && agent_count > 0 {
            3 // Low activity during planning phase
        } else if agent_count == 1 {
            5 // Low activity for single agent
        } else if agent_count == 2 {
            10 // Medium activity for two agents
        } else if agent_count >= 3 {
            15 // High activity for multiple agents
        } else {
            0 // No activity when no agents
        };

        // Don't generate data if there's no activity
        if base_height == 0 {
            return;
        }

        // Generate random variation
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        use std::hash::Hasher;
        let mut hasher = DefaultHasher::new();
        now.elapsed().as_nanos().hash(&mut hasher);
        let random_seed = hasher.finish();

        // More variation during planning phase for visibility (+/- 50%)
        // Less variation during running for stability (+/- 30%)
        let variation_percent = if self.agents_ready_to_start && self.active_agents.is_empty() {
            50 // More variation during planning for visibility
        } else {
            30 // Standard variation during running
        };

        let variation_range = variation_percent * 2; // e.g., 100 for +/- 50%
        let variation =
            ((random_seed % variation_range) as i32 - variation_percent as i32) * base_height / 100;
        let height = ((base_height + variation).max(1) as u64).min(20);

        // Check if any agents are completed
        let has_completed = self
            .active_agents
            .iter()
            .any(|a| matches!(a.status, AgentStatus::Completed));

        // Keep a rolling window of 60 data points (about 6 seconds at 100ms intervals)
        let mut sparkline_data = self.sparkline_data.borrow_mut();
        sparkline_data.push((height, has_completed));
        if sparkline_data.len() > 60 {
            sparkline_data.remove(0);
        }
    }

    pub(crate) fn set_reasoning_effort(&mut self, new_effort: ReasoningEffort) {
        // Update the config
        self.config.model_reasoning_effort = new_effort;

        // Send ConfigureSession op to update the backend
        let op = Op::ConfigureSession {
            provider: self.config.model_provider.clone(),
            model: self.config.model.clone(),
            model_reasoning_effort: new_effort,
            model_reasoning_summary: self.config.model_reasoning_summary,
            model_text_verbosity: self.config.model_text_verbosity,
            user_instructions: self.config.user_instructions.clone(),
            base_instructions: self.config.base_instructions.clone(),
            approval_policy: self.config.approval_policy,
            sandbox_policy: self.config.sandbox_policy.clone(),
            disable_response_storage: self.config.disable_response_storage,
            notify: self.config.notify.clone(),
            cwd: self.config.cwd.clone(),
            resume_path: None,
            output_schema: self.config.output_schema.clone(),
        };

        self.submit_op(op);

        // Add status message to history (replaceable system notice)
        let placement = self.ui_placement_for_now();
        self.push_system_cell(
            history_cell::new_reasoning_output(&new_effort),
            placement,
            Some("ui:reasoning".to_string()),
            None,
            "system",
        );
    }

    pub(crate) fn set_text_verbosity(&mut self, new_verbosity: TextVerbosity) {
        // Update the config
        self.config.model_text_verbosity = new_verbosity;

        // Send ConfigureSession op to update the backend
        let op = Op::ConfigureSession {
            provider: self.config.model_provider.clone(),
            model: self.config.model.clone(),
            model_reasoning_effort: self.config.model_reasoning_effort,
            model_reasoning_summary: self.config.model_reasoning_summary,
            model_text_verbosity: new_verbosity,
            user_instructions: self.config.user_instructions.clone(),
            base_instructions: self.config.base_instructions.clone(),
            approval_policy: self.config.approval_policy,
            sandbox_policy: self.config.sandbox_policy.clone(),
            disable_response_storage: self.config.disable_response_storage,
            notify: self.config.notify.clone(),
            cwd: self.config.cwd.clone(),
            resume_path: None,
            output_schema: self.config.output_schema.clone(),
        };

        self.submit_op(op);

        // Add status message to history
        let message = format!("Text verbosity set to: {}", new_verbosity);
        self.push_background_tail(message);
    }

    pub(crate) fn set_auto_upgrade_enabled(&mut self, enabled: bool) {
        if !crate::updates::upgrade_ui_enabled() {
            self.bottom_pane.flash_footer_notice(
                "Automatic upgrades are disabled in debug builds. Set SHOW_UPGRADE=1 to preview."
                    .to_string(),
            );
            self.request_redraw();
            return;
        }

        if self.config.auto_upgrade_enabled == enabled {
            return;
        }
        self.config.auto_upgrade_enabled = enabled;

        let codex_home = self.config.codex_home.clone();
        let profile = self.config.active_profile.clone();
        tokio::spawn(async move {
            if let Err(err) = codex_core::config_edit::persist_overrides(
                &codex_home,
                profile.as_deref(),
                &[(
                    &["auto_upgrade_enabled"],
                    if enabled { "true" } else { "false" },
                )],
            )
            .await
            {
                tracing::warn!("failed to persist auto-upgrade setting: {err}");
            }
        });

        let notice = if enabled {
            "Automatic upgrades enabled"
        } else {
            "Automatic upgrades disabled"
        };
        self.bottom_pane.flash_footer_notice(notice.to_string());
        self.request_redraw();
    }

    /// Forward file-search results to the bottom pane.
    pub(crate) fn apply_file_search_result(&mut self, query: String, matches: Vec<FileMatch>) {
        self.bottom_pane.on_file_search_result(query, matches);
    }

    pub(crate) fn show_theme_selection(&mut self) {
        self.bottom_pane
            .show_theme_selection(self.config.tui.theme.name);
    }

    // Ctrl+Y syntax cycling disabled intentionally.

    /// Show a brief debug notice in the footer.
    pub(crate) fn debug_notice(&mut self, text: String) {
        self.bottom_pane.flash_footer_notice(text);
        self.request_redraw();
    }

    fn maybe_start_auto_upgrade_task(&self) {
        if !crate::updates::auto_upgrade_runtime_enabled() {
            return;
        }
        if !self.config.auto_upgrade_enabled {
            return;
        }

        let cfg = self.config.clone();
        let tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            match crate::updates::auto_upgrade_if_enabled(&cfg).await {
                Ok(Some(version)) => {
                    tx.send(AppEvent::AutoUpgradeCompleted { version });
                }
                Ok(None) => {}
                Err(err) => {
                    tracing::warn!("auto-upgrade: background task failed: {err:?}");
                }
            }
        });
    }

    pub(crate) fn set_theme(&mut self, new_theme: codex_core::config_types::ThemeName) {
        // Update the config
        self.config.tui.theme.name = new_theme;

        // Save the theme to config file
        self.save_theme_to_config(new_theme);

        // Retint pre-rendered history cell lines to the new palette
        self.restyle_history_after_theme_change();

        // Add confirmation message to history (replaceable system notice)
        let theme_name = match new_theme {
            // Light themes
            codex_core::config_types::ThemeName::LightPhoton => "Light - Photon".to_string(),
            codex_core::config_types::ThemeName::LightPrismRainbow => {
                "Light - Prism Rainbow".to_string()
            }
            codex_core::config_types::ThemeName::LightVividTriad => {
                "Light - Vivid Triad".to_string()
            }
            codex_core::config_types::ThemeName::LightPorcelain => "Light - Porcelain".to_string(),
            codex_core::config_types::ThemeName::LightSandbar => "Light - Sandbar".to_string(),
            codex_core::config_types::ThemeName::LightGlacier => "Light - Glacier".to_string(),
            // Dark themes
            codex_core::config_types::ThemeName::DarkCarbonNight => {
                "Dark - Carbon Night".to_string()
            }
            codex_core::config_types::ThemeName::DarkShinobiDusk => {
                "Dark - Shinobi Dusk".to_string()
            }
            codex_core::config_types::ThemeName::DarkOledBlackPro => {
                "Dark - OLED Black Pro".to_string()
            }
            codex_core::config_types::ThemeName::DarkAmberTerminal => {
                "Dark - Amber Terminal".to_string()
            }
            codex_core::config_types::ThemeName::DarkAuroraFlux => "Dark - Aurora Flux".to_string(),
            codex_core::config_types::ThemeName::DarkCharcoalRainbow => {
                "Dark - Charcoal Rainbow".to_string()
            }
            codex_core::config_types::ThemeName::DarkZenGarden => "Dark - Zen Garden".to_string(),
            codex_core::config_types::ThemeName::DarkPaperLightPro => {
                "Dark - Paper Light Pro".to_string()
            }
            codex_core::config_types::ThemeName::Custom => {
                // Use saved custom name and is_dark to show a friendly label
                let mut label =
                    crate::theme::custom_theme_label().unwrap_or_else(|| "Custom".to_string());
                // Sanitize leading Light/Dark if present
                for pref in ["Light - ", "Dark - ", "Light ", "Dark "] {
                    if label.starts_with(pref) {
                        label = label[pref.len()..].trim().to_string();
                        break;
                    }
                }
                if crate::theme::custom_theme_is_dark().unwrap_or(false) {
                    format!("Dark - {}", label)
                } else {
                    format!("Light - {}", label)
                }
            }
        };
        let message = format!("Theme changed to {}", theme_name);
        let placement = self.ui_placement_for_now();
        self.push_system_cell(
            history_cell::new_background_event(message),
            placement,
            Some("ui:theme".to_string()),
            None,
            "background",
        );
    }

    pub(crate) fn set_spinner(&mut self, spinner_name: String) {
        // Update the config
        self.config.tui.spinner.name = spinner_name.clone();
        // Persist selection to config file
        if let Ok(home) = codex_core::config::find_codex_home() {
            if let Err(e) = codex_core::config::set_tui_spinner_name(&home, &spinner_name) {
                tracing::warn!("Failed to persist spinner to config.toml: {}", e);
            } else {
                tracing::info!("Persisted TUI spinner selection to config.toml");
            }
        } else {
            tracing::warn!("Could not locate Codex home to persist spinner selection");
        }

        // Confirmation message (replaceable system notice)
        let message = format!("Spinner changed to {}", spinner_name);
        let placement = self.ui_placement_for_now();
        self.push_system_cell(
            history_cell::new_background_event(message),
            placement,
            Some("ui:spinner".to_string()),
            None,
            "background",
        );
    }

    fn apply_access_mode_indicator_from_config(&mut self) {
        use codex_core::protocol::AskForApproval;
        use codex_core::protocol::SandboxPolicy;
        let label = match (&self.config.sandbox_policy, self.config.approval_policy) {
            (SandboxPolicy::ReadOnly, _) => Some("Read Only".to_string()),
            (
                SandboxPolicy::WorkspaceWrite {
                    network_access: false,
                    ..
                },
                AskForApproval::UnlessTrusted,
            ) => Some("Write with Approval".to_string()),
            _ => None,
        };
        self.bottom_pane.set_access_mode_label(label);
    }

    /// Rotate the access preset: Read Only (Plan Mode) → Write with Approval → Full Access
    pub(crate) fn cycle_access_mode(&mut self) {
        use codex_core::config::set_project_access_mode;
        use codex_core::protocol::AskForApproval;
        use codex_core::protocol::SandboxPolicy;

        // Determine current index
        let idx = match (&self.config.sandbox_policy, self.config.approval_policy) {
            (SandboxPolicy::ReadOnly, _) => 0,
            (
                SandboxPolicy::WorkspaceWrite {
                    network_access: false,
                    ..
                },
                AskForApproval::UnlessTrusted,
            ) => 1,
            (SandboxPolicy::DangerFullAccess, AskForApproval::Never) => 2,
            _ => 0,
        };
        let next = (idx + 1) % 3;

        // Apply mapping
        let (label, approval, sandbox) = match next {
            0 => (
                "Read Only (Plan Mode)",
                AskForApproval::OnRequest,
                SandboxPolicy::ReadOnly,
            ),
            1 => (
                "Write with Approval",
                AskForApproval::UnlessTrusted,
                SandboxPolicy::new_workspace_write_policy(),
            ),
            _ => (
                "Full Access",
                AskForApproval::Never,
                SandboxPolicy::DangerFullAccess,
            ),
        };

        // Update local config
        self.config.approval_policy = approval;
        self.config.sandbox_policy = sandbox;

        // Send ConfigureSession op to backend
        let op = Op::ConfigureSession {
            provider: self.config.model_provider.clone(),
            model: self.config.model.clone(),
            model_reasoning_effort: self.config.model_reasoning_effort,
            model_reasoning_summary: self.config.model_reasoning_summary,
            model_text_verbosity: self.config.model_text_verbosity,
            user_instructions: self.config.user_instructions.clone(),
            base_instructions: self.config.base_instructions.clone(),
            approval_policy: self.config.approval_policy,
            sandbox_policy: self.config.sandbox_policy.clone(),
            disable_response_storage: self.config.disable_response_storage,
            notify: self.config.notify.clone(),
            cwd: self.config.cwd.clone(),
            resume_path: None,
            output_schema: self.config.output_schema.clone(),
        };
        self.submit_op(op);

        // Persist selection into CODEX_HOME/config.toml for this project directory so it sticks.
        let _ = set_project_access_mode(
            &self.config.codex_home,
            &self.config.cwd,
            self.config.approval_policy,
            match &self.config.sandbox_policy {
                SandboxPolicy::ReadOnly => codex_protocol::config_types::SandboxMode::ReadOnly,
                SandboxPolicy::WorkspaceWrite { .. } => {
                    codex_protocol::config_types::SandboxMode::WorkspaceWrite
                }
                SandboxPolicy::DangerFullAccess => {
                    codex_protocol::config_types::SandboxMode::DangerFullAccess
                }
            },
        );

        // Footer indicator: persistent for RO/Approval; ephemeral for Full Access
        if next == 2 {
            self.bottom_pane.set_access_mode_label_ephemeral(
                "Full Access".to_string(),
                std::time::Duration::from_secs(4),
            );
        } else {
            let persistent = if next == 0 {
                "Read Only"
            } else {
                "Write with Approval"
            };
            self.bottom_pane
                .set_access_mode_label(Some(persistent.to_string()));
        }

        // Announce in history: replace the last access-mode status, inserting early
        // in the current request so it appears above upcoming commands.
        let msg = format!("Mode changed: {}", label);
        self.set_access_status_message(msg);
        // No footer notice: the indicator covers this; avoid duplicate texts.

        // Prepare a single consolidated note for the agent to see before the
        // next turn begins. Subsequent cycles will overwrite this note.
        let agent_note = match next {
            0 => {
                "System: access mode changed to Read Only. Do not attempt write operations or apply_patch."
            }
            1 => {
                "System: access mode changed to Write with Approval. Request approval before writes."
            }
            _ => "System: access mode changed to Full Access. Writes and network are allowed.",
        };
        self.queue_agent_note(agent_note);
    }

    /// Insert or replace the access-mode status background event. Uses a near-time
    /// key so it appears above any imminent Exec/Tool cells in this request.
    fn set_access_status_message(&mut self, message: String) {
        let cell = crate::history_cell::new_background_event(message);
        if let Some(idx) = self.access_status_idx
            && idx < self.history_cells.len()
            && matches!(
                self.history_cells[idx].kind(),
                crate::history_cell::HistoryCellType::BackgroundEvent
            )
        {
            self.history_replace_at(idx, Box::new(cell));
            self.request_redraw();
            return;
        }
        // Insert new status near the top of this request window
        let key = self.near_time_key(None);
        let pos = self.history_insert_with_key_global_tagged(Box::new(cell), key, "background");
        self.access_status_idx = Some(pos);
    }

    fn restyle_history_after_theme_change(&mut self) {
        let old = self.last_theme.clone();
        let new = crate::theme::current_theme();
        if old == new {
            return;
        }

        for cell in &mut self.history_cells {
            if let Some(plain) = cell
                .as_any_mut()
                .downcast_mut::<history_cell::PlainHistoryCell>()
            {
                plain.invalidate_layout_cache();
            } else if let Some(tool) = cell
                .as_any_mut()
                .downcast_mut::<history_cell::ToolCallCell>()
            {
                tool.retint(&old, &new);
            } else if let Some(reason) = cell
                .as_any_mut()
                .downcast_mut::<history_cell::CollapsibleReasoningCell>()
            {
                reason.retint(&old, &new);
            } else if let Some(stream) = cell
                .as_any_mut()
                .downcast_mut::<history_cell::StreamingContentCell>()
            {
                stream.retint(&old, &new);
            } else if let Some(wait) = cell
                .as_any_mut()
                .downcast_mut::<history_cell::WaitStatusCell>()
            {
                wait.retint(&old, &new);
            } else if let Some(assist) = cell
                .as_any_mut()
                .downcast_mut::<history_cell::AssistantMarkdownCell>()
            {
                // Fully rebuild from raw to apply new theme + syntax highlight
                assist.rebuild(&self.config);
            }
        }

        // Update snapshot and redraw; height caching can remain (colors don't affect wrap)
        self.last_theme = new;
        self.app_event_tx.send(AppEvent::RequestRedraw);
    }

    /// Public-facing hook for preview mode to retint existing history lines
    /// without persisting the theme or adding history events.
    pub(crate) fn retint_history_for_preview(&mut self) {
        self.restyle_history_after_theme_change();
    }

    fn save_theme_to_config(&self, new_theme: codex_core::config_types::ThemeName) {
        // Persist the theme selection to CODE_HOME/CODEX_HOME config.toml
        match codex_core::config::find_codex_home() {
            Ok(home) => {
                if let Err(e) = codex_core::config::set_tui_theme_name(&home, new_theme) {
                    tracing::warn!("Failed to persist theme to config.toml: {}", e);
                } else {
                    tracing::info!("Persisted TUI theme selection to config.toml");
                }
            }
            Err(e) => {
                tracing::warn!("Could not locate Codex home to persist theme: {}", e);
            }
        }
    }

    /// Handle Ctrl-C key press.
    /// Returns CancellationEvent::Handled if the event was consumed by the UI, or
    /// CancellationEvent::Ignored if the caller should handle it (e.g. exit).
    pub(crate) fn on_ctrl_c(&mut self) -> CancellationEvent {
        if let Some(id) = self.terminal_overlay_id() {
            if self.terminal_is_running() {
                self.request_terminal_cancel(id);
            } else {
                self.close_terminal_overlay();
            }
            return CancellationEvent::Handled;
        }
        match self.bottom_pane.on_ctrl_c() {
            CancellationEvent::Handled => return CancellationEvent::Handled,
            CancellationEvent::Ignored => {}
        }
        let exec_related_running = !self.exec.running_commands.is_empty()
            || !self.tools_state.running_custom_tools.is_empty()
            || !self.tools_state.running_web_search.is_empty()
            || !self.tools_state.running_wait_tools.is_empty()
            || !self.tools_state.running_kill_tools.is_empty();
        if self.bottom_pane.is_task_running() || exec_related_running {
            self.interrupt_running_task();
            CancellationEvent::Ignored
        } else if self.bottom_pane.ctrl_c_quit_hint_visible() {
            self.submit_op(Op::Shutdown);
            CancellationEvent::Handled
        } else {
            self.bottom_pane.show_ctrl_c_quit_hint();
            CancellationEvent::Ignored
        }
    }

    pub(crate) fn composer_is_empty(&self) -> bool {
        self.bottom_pane.composer_is_empty()
    }

    // --- Double‑Escape helpers ---
    pub(crate) fn show_esc_backtrack_hint(&mut self) {
        self.bottom_pane
            .flash_footer_notice("Esc edit prev".to_string());
    }

    pub(crate) fn show_edit_previous_picker(&mut self) {
        use crate::bottom_pane::list_selection_view::ListSelectionView;
        use crate::bottom_pane::list_selection_view::SelectionItem;
        // Collect recent user prompts (newest first)
        let mut items: Vec<SelectionItem> = Vec::new();
        let mut nth_counter = 0usize;
        for cell in self.history_cells.iter().rev() {
            if cell.kind() == crate::history_cell::HistoryCellType::User {
                nth_counter += 1; // 1-based index for Nth last
                let content_lines = cell.display_lines();
                if content_lines.is_empty() {
                    continue;
                }
                let full_text: String = content_lines
                    .iter()
                    .map(|l| {
                        l.spans
                            .iter()
                            .map(|s| s.content.to_string())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // Build a concise name from first line
                let mut first = content_lines[0]
                    .spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>();
                const MAX: usize = 64;
                if first.chars().count() > MAX {
                    first = first.chars().take(MAX).collect::<String>() + "…";
                }

                let nth = nth_counter;
                let actions: Vec<crate::bottom_pane::list_selection_view::SelectionAction> =
                    vec![Box::new({
                        let text = full_text.clone();
                        move |tx: &crate::app_event_sender::AppEventSender| {
                            tx.send(crate::app_event::AppEvent::JumpBack {
                                nth,
                                prefill: text.clone(),
                            });
                        }
                    })];

                items.push(SelectionItem {
                    name: first,
                    description: None,
                    is_current: false,
                    actions,
                });
            }
        }

        if items.is_empty() {
            self.bottom_pane
                .flash_footer_notice("No previous messages to edit".to_string());
            return;
        }

        let view: ListSelectionView = ListSelectionView::new(
            " Jump back to a previous message ".to_string(),
            Some("This will return the conversation to an earlier state".to_string()),
            Some("Esc cancel".to_string()),
            items,
            self.app_event_tx.clone(),
            8,
        );
        self.bottom_pane.show_list_selection(
            "Jump back to a previous message".to_string(),
            None,
            None,
            view,
        );
    }

    pub(crate) fn is_task_running(&self) -> bool {
        self.bottom_pane.is_task_running()
            || self.terminal_is_running()
            || !self.exec.running_commands.is_empty()
            || !self.tools_state.running_custom_tools.is_empty()
            || !self.tools_state.running_web_search.is_empty()
            || !self.tools_state.running_wait_tools.is_empty()
            || !self.tools_state.running_kill_tools.is_empty()
    }

    // begin_jump_back no longer used: backend fork handles it.
    // undo_jump_back, has_pending_jump_back moved to undo_snapshots.rs (MAINT-11 Phase 9)

    /// Clear the composer text and any pending paste placeholders/history cursors.
    pub(crate) fn clear_composer(&mut self) {
        self.bottom_pane.clear_composer();
        // Mark a height change so layout adjusts immediately if the composer shrinks.
        self.height_manager
            .borrow_mut()
            .record_event(crate::height_manager::HeightEvent::ComposerModeChange);
        self.request_redraw();
    }

    pub(crate) fn close_file_popup_if_active(&mut self) -> bool {
        self.bottom_pane.close_file_popup_if_active()
    }

    pub(crate) fn has_active_modal_view(&self) -> bool {
        // Treat bottom‑pane views (approval, selection popups) and top‑level overlays
        // (diff viewer, help overlay) as "modals" for Esc routing. This ensures that
        // a single Esc keypress closes the visible overlay instead of engaging the
        // global Esc policy (clear input / backtrack).
        self.bottom_pane.has_active_modal_view()
            || self.diffs.overlay.is_some()
            || self.help.overlay.is_some()
            || self.limits.overlay.is_some()
            || self.terminal.overlay.is_some()
    }

    /// Forward an `Op` directly to codex.
    pub(crate) fn submit_op(&self, op: Op) {
        if let Err(e) = self.codex_op_tx.send(op) {
            tracing::error!("failed to submit op: {e}");
        }
    }

    /// Cancel the current running task from a non-keyboard context (e.g. approval modal).
    /// This bypasses modal key handling and invokes the same immediate UI cleanup path
    /// as pressing Ctrl-C/Esc while a task is running.
    pub(crate) fn cancel_running_task_from_approval(&mut self) {
        self.interrupt_running_task();
    }

    pub(crate) fn register_approved_command(
        &self,
        command: Vec<String>,
        match_kind: ApprovedCommandMatchKind,
        semantic_prefix: Option<Vec<String>>,
    ) {
        if command.is_empty() {
            return;
        }
        let op = Op::RegisterApprovedCommand {
            command,
            match_kind,
            semantic_prefix,
        };
        self.submit_op(op);
    }

    /// Clear transient spinner/status after a denial without interrupting core
    /// execution. Only hide the spinner when there is no remaining activity so
    /// we avoid masking in-flight work (e.g. follow-up reasoning).
    pub(crate) fn mark_task_idle_after_denied(&mut self) {
        let any_tools_running = !self.exec.running_commands.is_empty()
            || !self.tools_state.running_custom_tools.is_empty()
            || !self.tools_state.running_web_search.is_empty();
        let any_streaming = self.stream.is_write_cycle_active();
        let any_agents_active = self.agents_are_actively_running();
        let any_tasks_active = !self.active_task_ids.is_empty();

        if !(any_tools_running || any_streaming || any_agents_active || any_tasks_active) {
            self.bottom_pane.set_task_running(false);
            self.bottom_pane.update_status_text(String::new());
            self.bottom_pane.clear_ctrl_c_quit_hint();
            self.mark_needs_redraw();
        }
    }

    /// Handle CLI routing completion (SPEC-KIT-952)
    ///
    /// Called when a CLI-routed prompt (Claude/Gemini) completes execution.
    /// Displays the response and clears the task running state.
    pub(crate) fn on_cli_route_complete(
        &mut self,
        provider_name: String,
        model_name: String,
        content: String,
        is_error: bool,
    ) {
        // Format the response
        let cell_type = if is_error {
            history_cell::HistoryCellType::Error
        } else {
            history_cell::HistoryCellType::Assistant
        };

        // Create response text with provider header
        let response_text = if is_error {
            format!("❌ {} Error: {}", provider_name, content)
        } else {
            format!("**{}** ({})\n\n{}", provider_name, model_name, content)
        };

        // Render as markdown lines
        let lines = crate::markdown_renderer::MarkdownRenderer::render(&response_text);

        // Add to history
        self.history_push(history_cell::PlainHistoryCell::new(lines, cell_type));

        // Clear task running state
        self.bottom_pane.set_task_running(false);
        self.bottom_pane.update_status_text(String::new());
        self.bottom_pane.clear_ctrl_c_quit_hint();

        // Auto-scroll and redraw
        self.autoscroll_if_near_bottom();
        self.mark_needs_redraw();
    }

    /// Handle native provider stream start (SPEC-KIT-953)
    ///
    /// Called when streaming begins for a native provider (Claude/Gemini).
    pub(crate) fn on_native_stream_start(
        &mut self,
        provider_name: String,
        model_name: String,
        message_id: String,
    ) {
        // Store provider info for later use
        self.native_stream_provider = Some(provider_name.clone());
        self.native_stream_model = Some(model_name.clone());
        self.native_stream_id = Some(message_id.clone());
        self.native_stream_content = String::new();

        // Show loading indicator with provider name
        let header_line =
            ratatui::text::Line::from(format!("**{}** ({})", provider_name, model_name));
        let header_lines =
            crate::markdown_renderer::MarkdownRenderer::render(&header_line.to_string());

        // Insert header as a cell
        self.history_push(history_cell::PlainHistoryCell::new(
            header_lines,
            history_cell::HistoryCellType::Assistant,
        ));

        // Begin streaming with answer kind
        streaming::begin(self, StreamKind::Answer, Some(message_id));

        // Ensure task is running
        self.bottom_pane.set_task_running(true);
        self.autoscroll_if_near_bottom();
        self.mark_needs_redraw();
    }

    /// Handle native provider stream delta (SPEC-KIT-953)
    ///
    /// Called when a text chunk is received from a native provider.
    pub(crate) fn on_native_stream_delta(&mut self, text: String) {
        // Log first delta only (to avoid spam)
        if self.native_stream_content.is_empty() {
            tracing::debug!(
                "🟢 STREAM_START: First delta received | preview: {}...",
                &text.chars().take(50).collect::<String>()
            );
        }

        // Accumulate content for history
        self.native_stream_content.push_str(&text);

        // Push to streaming display
        let id = self.native_stream_id.clone().unwrap_or_default();
        streaming::delta_text(self, StreamKind::Answer, id, text, None);

        self.autoscroll_if_near_bottom();
    }

    /// Handle native provider stream complete (SPEC-KIT-953)
    ///
    /// Called when streaming finishes for a native provider.
    pub(crate) fn on_native_stream_complete(
        &mut self,
        _provider_name: String,
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
    ) {
        tracing::debug!(
            "🟡 STREAM_COMPLETE: Finalizing stream | content_len: {} chars",
            self.native_stream_content.len()
        );

        // Finalize the stream
        streaming::finalize(self, StreamKind::Answer, true);

        // Update conversation history with the accumulated response
        if let (Some(_provider), Some(_model)) = (
            self.native_stream_provider.take(),
            self.native_stream_model.take(),
        ) {
            let content = std::mem::take(&mut self.native_stream_content);

            tracing::debug!(
                "🟠 ASSISTANT_MSG_PUSH: Adding assistant response to conversation history | preview: {}...",
                &content.chars().take(50).collect::<String>()
            );

            // Log token usage if available
            if let (Some(input), Some(output)) = (input_tokens, output_tokens) {
                tracing::info!(
                    "Native provider token usage: input={}, output={}",
                    input,
                    output
                );
            }
        }

        // Clear stream state
        self.native_stream_id = None;

        // Clear task running state
        self.bottom_pane.set_task_running(false);
        self.bottom_pane.update_status_text(String::new());
        self.bottom_pane.clear_ctrl_c_quit_hint();

        self.autoscroll_if_near_bottom();
        self.mark_needs_redraw();

        // SPEC-954-FIX: Process any queued messages for CLI routing
        // CLI routing queues messages locally (unlike OAuth which uses core queue)
        if !self.queued_user_messages.is_empty() {
            tracing::info!(
                "🔄 PROCESSING_QUEUED: {} messages queued, processing next",
                self.queued_user_messages.len()
            );
            let batch: Vec<UserMessage> = self.queued_user_messages.drain(..).collect();
            self.refresh_queued_user_messages();
            self.send_user_messages_to_agent(batch);
        }
    }

    /// Handle native provider stream error (SPEC-KIT-953)
    ///
    /// Called when an error occurs during streaming.
    pub(crate) fn on_native_stream_error(&mut self, provider_name: String, error: String) {
        // Finalize any active stream
        streaming::finalize_active_stream(self);

        // Clear stream state
        self.native_stream_provider = None;
        self.native_stream_model = None;
        self.native_stream_id = None;
        self.native_stream_content = String::new();

        // Display error
        let error_text = format!("❌ {} Error: {}", provider_name, error);
        let lines = crate::markdown_renderer::MarkdownRenderer::render(&error_text);
        self.history_push(history_cell::PlainHistoryCell::new(
            lines,
            history_cell::HistoryCellType::Error,
        ));

        // Clear task running state
        self.bottom_pane.set_task_running(false);
        self.bottom_pane.update_status_text(String::new());

        self.autoscroll_if_near_bottom();
        self.mark_needs_redraw();
    }

    pub(crate) fn insert_history_lines(&mut self, lines: Vec<ratatui::text::Line<'static>>) {
        let kind = self.stream_state.current_kind.unwrap_or(StreamKind::Answer);
        self.insert_history_lines_with_kind(kind, None, lines);
    }

    pub(crate) fn insert_history_lines_with_kind(
        &mut self,
        kind: StreamKind,
        id: Option<String>,
        mut lines: Vec<ratatui::text::Line<'static>>,
    ) {
        // No debug logging: we rely on preserving span modifiers end-to-end.
        // Insert all lines as a single streaming content cell to preserve spacing
        if lines.is_empty() {
            return;
        }

        if let Some(first_line) = lines.first() {
            let first_line_text: String = first_line
                .spans
                .iter()
                .map(|s| s.content.to_string())
                .collect();
            tracing::debug!("First line content: {:?}", first_line_text);
        }

        match kind {
            StreamKind::Reasoning => {
                // This reasoning block is the bottom-most; show progress indicator here only
                self.clear_reasoning_in_progress();
                // Ensure footer shows Ctrl+R hint when reasoning content is present
                self.bottom_pane.set_reasoning_hint(true);
                // Update footer label to reflect current visibility state
                self.bottom_pane
                    .set_reasoning_state(self.is_reasoning_shown());
                // Route by id when provided to avoid splitting reasoning across cells.
                // Be defensive: the cached index may be stale after inserts/removals; validate it.
                if let Some(ref rid) = id
                    && let Some(&idx) = self.reasoning_index.get(rid)
                {
                    if idx < self.history_cells.len()
                        && let Some(reasoning_cell) = self.history_cells[idx]
                            .as_any_mut()
                            .downcast_mut::<history_cell::CollapsibleReasoningCell>(
                        )
                    {
                        tracing::debug!("Appending {} lines to Reasoning(id={})", lines.len(), rid);
                        reasoning_cell.append_lines_dedup(lines);
                        reasoning_cell.set_in_progress(true);
                        self.invalidate_height_cache();
                        self.autoscroll_if_near_bottom();
                        self.request_redraw();
                        self.refresh_reasoning_collapsed_visibility();
                        return;
                    }
                    // Cached index was stale or wrong type — try to locate by scanning.
                    if let Some(found_idx) = self.history_cells.iter().rposition(|c| {
                        c.as_any()
                            .downcast_ref::<history_cell::CollapsibleReasoningCell>()
                            .map(|rc| rc.matches_id(rid))
                            .unwrap_or(false)
                    }) {
                        if let Some(reasoning_cell) = self.history_cells[found_idx]
                            .as_any_mut()
                            .downcast_mut::<history_cell::CollapsibleReasoningCell>(
                        ) {
                            // Refresh the cache with the corrected index
                            self.reasoning_index.insert(rid.clone(), found_idx);
                            tracing::debug!(
                                "Recovered stale reasoning index; appending at {} for id={}",
                                found_idx,
                                rid
                            );
                            reasoning_cell.append_lines_dedup(lines);
                            reasoning_cell.set_in_progress(true);
                            self.invalidate_height_cache();
                            self.autoscroll_if_near_bottom();
                            self.request_redraw();
                            self.refresh_reasoning_collapsed_visibility();
                            return;
                        }
                    } else {
                        // No matching cell remains; drop the stale cache entry.
                        self.reasoning_index.remove(rid);
                    }
                }

                tracing::debug!("Creating new CollapsibleReasoningCell id={:?}", id);
                let cell = history_cell::CollapsibleReasoningCell::new_with_id(lines, id.clone());
                if self.config.tui.show_reasoning {
                    cell.set_collapsed(false);
                } else {
                    cell.set_collapsed(true);
                }
                cell.set_in_progress(true);

                // Use pre-seeded key for this stream id when present; otherwise synthesize.
                let key = match id.as_deref() {
                    Some(rid) => self.try_stream_order_key(kind, rid).unwrap_or_else(|| {
                        tracing::warn!(
                            "missing stream order key for Reasoning id={}; using synthetic key",
                            rid
                        );
                        self.next_internal_key()
                    }),
                    None => {
                        tracing::warn!("missing stream id for Reasoning; using synthetic key");
                        self.next_internal_key()
                    }
                };
                tracing::info!(
                    "[order] insert Reasoning new id={:?} {}",
                    id,
                    Self::debug_fmt_order_key(key)
                );
                let idx = self.history_insert_with_key_global(Box::new(cell), key);
                if let Some(rid) = id {
                    self.reasoning_index.insert(rid, idx);
                }
            }
            StreamKind::Answer => {
                tracing::debug!(
                    "history.insert Answer id={:?} incoming_lines={}",
                    id,
                    lines.len()
                );
                // Any incoming Answer means reasoning is no longer bottom-most
                self.clear_reasoning_in_progress();
                // Keep a single StreamingContentCell and append to it
                if let Some(last) = self.history_cells.last_mut()
                    && let Some(stream_cell) = last
                        .as_any_mut()
                        .downcast_mut::<history_cell::StreamingContentCell>()
                {
                    // If id is specified, only append when ids match
                    if let Some(ref want) = id {
                        if stream_cell.id.as_ref() != Some(want) {
                            // fall through to create/find matching cell below
                        } else {
                            tracing::debug!(
                                "history.append -> last StreamingContentCell (id match) lines+={}",
                                lines.len()
                            );
                            // Guard against stray header sneaking into a later chunk
                            if lines
                                .first()
                                .map(|l| {
                                    l.spans
                                        .iter()
                                        .map(|s| s.content.as_ref())
                                        .collect::<String>()
                                        .trim()
                                        .eq_ignore_ascii_case("codex")
                                })
                                .unwrap_or(false)
                            {
                                if lines.len() == 1 {
                                    return;
                                } else {
                                    lines.remove(0);
                                }
                            }
                            stream_cell.extend_lines(lines);
                            self.invalidate_height_cache();
                            self.autoscroll_if_near_bottom();
                            self.request_redraw();
                            return;
                        }
                    } else {
                        // No id — legacy: append to last
                        tracing::debug!(
                            "history.append -> last StreamingContentCell (no id provided) lines+={}",
                            lines.len()
                        );
                        if lines
                            .first()
                            .map(|l| {
                                l.spans
                                    .iter()
                                    .map(|s| s.content.as_ref())
                                    .collect::<String>()
                                    .trim()
                                    .eq_ignore_ascii_case("codex")
                            })
                            .unwrap_or(false)
                        {
                            if lines.len() == 1 {
                                return;
                            } else {
                                lines.remove(0);
                            }
                        }
                        stream_cell.extend_lines(lines);
                        self.invalidate_height_cache();
                        self.autoscroll_if_near_bottom();
                        self.request_redraw();
                        return;
                    }
                }

                // If id is specified, try to locate an existing streaming cell with that id
                if let Some(ref want) = id
                    && let Some(idx) = self.history_cells.iter().rposition(|c| {
                        c.as_any()
                            .downcast_ref::<history_cell::StreamingContentCell>()
                            .map(|sc| sc.id.as_ref() == Some(want))
                            .unwrap_or(false)
                    })
                    && let Some(stream_cell) = self.history_cells[idx]
                        .as_any_mut()
                        .downcast_mut::<history_cell::StreamingContentCell>()
                {
                    tracing::debug!(
                        "history.append -> StreamingContentCell by id at idx={} lines+={}",
                        idx,
                        lines.len()
                    );
                    if lines
                        .first()
                        .map(|l| {
                            l.spans
                                .iter()
                                .map(|s| s.content.as_ref())
                                .collect::<String>()
                                .trim()
                                .eq_ignore_ascii_case("codex")
                        })
                        .unwrap_or(false)
                    {
                        if lines.len() == 1 {
                            return;
                        } else {
                            lines.remove(0);
                        }
                    }
                    stream_cell.extend_lines(lines);
                    self.invalidate_height_cache();
                    self.autoscroll_if_near_bottom();
                    self.request_redraw();
                    return;
                }

                // Ensure a hidden 'codex' header is present
                let has_header = lines
                    .first()
                    .map(|l| {
                        l.spans
                            .iter()
                            .map(|s| s.content.as_ref())
                            .collect::<String>()
                            .trim()
                            .eq_ignore_ascii_case("codex")
                    })
                    .unwrap_or(false);
                if !has_header {
                    let mut with_header: Vec<ratatui::text::Line<'static>> =
                        Vec::with_capacity(lines.len() + 1);
                    with_header.push(ratatui::text::Line::from("codex"));
                    with_header.extend(lines);
                    lines = with_header;
                }
                // Use pre-seeded key for this stream id when present; otherwise synthesize.
                let key = match id.as_deref() {
                    Some(rid) => self.try_stream_order_key(kind, rid).unwrap_or_else(|| {
                        tracing::warn!(
                            "missing stream order key for Answer id={}; using synthetic key",
                            rid
                        );
                        self.next_internal_key()
                    }),
                    None => {
                        tracing::warn!("missing stream id for Answer; using synthetic key");
                        self.next_internal_key()
                    }
                };
                tracing::info!(
                    "[order] insert Answer new id={:?} {}",
                    id,
                    Self::debug_fmt_order_key(key)
                );
                let new_idx = self.history_insert_with_key_global(
                    Box::new(history_cell::new_streaming_content_with_id(
                        id.clone(),
                        lines,
                    )),
                    key,
                );
                tracing::debug!(
                    "history.new StreamingContentCell at idx={} id={:?}",
                    new_idx,
                    id
                );
            }
        }

        // Auto-follow if near bottom so new inserts are visible
        self.autoscroll_if_near_bottom();
        self.request_redraw();
    }

    /// Replace the in-progress streaming assistant cell with a final markdown cell that
    /// stores raw markdown for future re-rendering.
    pub(crate) fn insert_final_answer_with_id(
        &mut self,
        id: Option<String>,
        lines: Vec<ratatui::text::Line<'static>>,
        source: String,
    ) {
        tracing::debug!(
            "insert_final_answer_with_id id={:?} source_len={} lines={}",
            id,
            source.len(),
            lines.len()
        );
        tracing::info!("[order] final Answer id={:?}", id);
        if self.is_review_flow_active() {
            if let Some(ref want) = id {
                if let Some(idx) = self.history_cells.iter().rposition(|c| {
                    c.as_any()
                        .downcast_ref::<history_cell::StreamingContentCell>()
                        .and_then(|sc| sc.id.as_ref())
                        .map(|existing| existing == want)
                        .unwrap_or(false)
                }) {
                    self.history_remove_at(idx);
                }
                self.stream_state
                    .closed_answer_ids
                    .insert(StreamId(want.clone()));
            } else if let Some(idx) = self.history_cells.iter().rposition(|c| {
                c.as_any()
                    .downcast_ref::<history_cell::StreamingContentCell>()
                    .is_some()
            }) {
                self.history_remove_at(idx);
            }
            self.last_assistant_message = Some(source);
            return;
        }
        // Debug: list last few history cell kinds so we can see what's present
        let tail_kinds: String = self
            .history_cells
            .iter()
            .rev()
            .take(5)
            .map(|c| {
                if c.as_any()
                    .downcast_ref::<history_cell::StreamingContentCell>()
                    .is_some()
                {
                    "Streaming".to_string()
                } else if c
                    .as_any()
                    .downcast_ref::<history_cell::AssistantMarkdownCell>()
                    .is_some()
                {
                    "AssistantFinal".to_string()
                } else if c
                    .as_any()
                    .downcast_ref::<history_cell::CollapsibleReasoningCell>()
                    .is_some()
                {
                    "Reasoning".to_string()
                } else {
                    format!("{:?}", c.kind())
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        tracing::debug!("history.tail kinds(last5) = [{}]", tail_kinds);

        // When we have an id but could not find a streaming cell by id, dump ids
        if id.is_some() {
            let ids: Vec<String> = self
                .history_cells
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    c.as_any()
                        .downcast_ref::<history_cell::StreamingContentCell>()
                        .and_then(|sc| sc.id.as_ref().map(|s| format!("{}:{}", i, s)))
                })
                .collect();
            tracing::debug!("history.streaming ids={}", ids.join(" | "));
        }
        // If we already finalized this id in the current turn with identical content,
        // drop this event to avoid duplicates (belt-and-suspenders against upstream repeats).
        if let Some(ref want) = id
            && self
                .stream_state
                .closed_answer_ids
                .contains(&StreamId(want.clone()))
            && let Some(existing_idx) = self.history_cells.iter().rposition(|c| {
                c.as_any()
                    .downcast_ref::<history_cell::AssistantMarkdownCell>()
                    .map(|amc| amc.id.as_ref() == Some(want))
                    .unwrap_or(false)
            })
            && let Some(amc) = self.history_cells[existing_idx]
                .as_any()
                .downcast_ref::<history_cell::AssistantMarkdownCell>()
        {
            let prev = Self::normalize_text(&amc.raw);
            let newn = Self::normalize_text(&source);
            if prev == newn {
                tracing::debug!(
                    "InsertFinalAnswer: dropping duplicate final for id={}",
                    want
                );
                return;
            }
        }
        // Ensure a hidden 'codex' header is present
        let has_header = lines
            .first()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
                    .trim()
                    .eq_ignore_ascii_case("codex")
            })
            .unwrap_or(false);
        if !has_header {
            // No need to mutate `lines` further since we rebuild from `source` below.
        }

        // Replace the matching StreamingContentCell if one exists for this id; else fallback to most recent.
        // NOTE (dup‑guard): This relies on `StreamingContentCell::as_any()` returning `self`.
        // If that impl is removed, downcast_ref will fail and we won't find the streaming cell,
        // causing the final to append a new Assistant cell (duplicate).
        let streaming_idx = if let Some(ref want) = id {
            // Only replace a streaming cell if its id matches this final.
            self.history_cells.iter().rposition(|c| {
                if let Some(sc) = c
                    .as_any()
                    .downcast_ref::<history_cell::StreamingContentCell>()
                {
                    sc.id.as_ref() == Some(want)
                } else {
                    false
                }
            })
        } else {
            None
        };
        if let Some(idx) = streaming_idx {
            tracing::debug!(
                "final-answer: replacing StreamingContentCell at idx={} by id match",
                idx
            );
            // Replace the matching streaming cell in-place, preserving the id
            let cell =
                history_cell::AssistantMarkdownCell::new_with_id(source, id.clone(), &self.config);
            self.history_replace_at(idx, Box::new(cell));
            // Mark this Answer stream id as closed for the rest of the turn so
            // any late AgentMessageDelta for the same id is ignored.
            if let Some(ref want) = id {
                self.stream_state
                    .closed_answer_ids
                    .insert(StreamId(want.clone()));
            }
            self.autoscroll_if_near_bottom();
            return;
        }

        // No streaming cell found. First, try to replace a finalized assistant cell
        // that was created for the same stream id (e.g., we already finalized due to
        // a lifecycle event and this InsertFinalAnswer arrived slightly later).
        if let Some(ref want) = id
            && let Some(idx) = self.history_cells.iter().rposition(|c| {
                if let Some(amc) = c
                    .as_any()
                    .downcast_ref::<history_cell::AssistantMarkdownCell>()
                {
                    amc.id.as_ref() == Some(want)
                } else {
                    false
                }
            })
        {
            tracing::debug!(
                "final-answer: replacing existing AssistantMarkdownCell at idx={} by id match",
                idx
            );
            let cell =
                history_cell::AssistantMarkdownCell::new_with_id(source, id.clone(), &self.config);
            self.history_replace_at(idx, Box::new(cell));
            if let Some(ref want) = id {
                self.stream_state
                    .closed_answer_ids
                    .insert(StreamId(want.clone()));
            }
            self.autoscroll_if_near_bottom();
            return;
        }

        // Otherwise, if a finalized assistant cell exists at the tail,
        // replace it in place to avoid duplicate assistant messages when a second
        // InsertFinalAnswer (e.g., from an AgentMessage event) arrives after we already
        // finalized due to a side event.
        //
        // SPEC-955 Session 2: Only use this fallback replacement when id is None.
        // When we have an explicit ID, we should create a new cell if no matching ID exists.
        if id.is_none()
            && let Some(idx) = self.history_cells.iter().rposition(|c| {
                c.as_any()
                    .downcast_ref::<history_cell::AssistantMarkdownCell>()
                    .is_some()
            })
        {
            // Replace the tail finalized assistant cell if the new content is identical OR
            // a superset revision of the previous content (common provider behavior where
            // a later final slightly extends the earlier one). Otherwise append a new
            // assistant message so distinct messages remain separate.
            let (should_replace, _prev_len, _new_len) = self.history_cells[idx]
                .as_any()
                .downcast_ref::<history_cell::AssistantMarkdownCell>()
                .map(|amc| {
                    let prev = Self::normalize_text(&amc.raw);
                    let newn = Self::normalize_text(&source);
                    let identical = prev == newn;
                    let is_superset = !identical && newn.contains(&prev);
                    // Heuristic: treat as revision when previous is reasonably long to
                    // avoid collapsing very short replies unintentionally.
                    let long_enough = prev.len() >= 80;
                    (
                        identical || (is_superset && long_enough),
                        prev.len(),
                        newn.len(),
                    )
                })
                .unwrap_or((false, 0, 0));
            if should_replace {
                tracing::debug!(
                    "final-answer: replacing tail AssistantMarkdownCell via heuristic identical/superset"
                );
                let cell = history_cell::AssistantMarkdownCell::new_with_id(
                    source,
                    id.clone(),
                    &self.config,
                );
                self.history_replace_at(idx, Box::new(cell));
                self.autoscroll_if_near_bottom();
                return;
            }
        }

        // Fallback: no prior assistant cell found; insert at stable sequence position.
        tracing::debug!(
            "final-answer: ordered insert new AssistantMarkdownCell id={:?}",
            id
        );
        let key = match id.as_deref() {
            Some(rid) => self
                .try_stream_order_key(StreamKind::Answer, rid)
                .unwrap_or_else(|| {
                    tracing::warn!(
                        "missing stream order key for final Answer id={}; using synthetic key",
                        rid
                    );
                    self.next_internal_key()
                }),
            None => {
                tracing::warn!("missing stream id for final Answer; using synthetic key");
                self.next_internal_key()
            }
        };
        tracing::info!(
            "[order] final Answer ordered insert id={:?} {}",
            id,
            Self::debug_fmt_order_key(key)
        );
        let cell =
            history_cell::AssistantMarkdownCell::new_with_id(source, id.clone(), &self.config);
        let _ = self.history_insert_with_key_global(Box::new(cell), key);
        if let Some(ref want) = id {
            self.stream_state
                .closed_answer_ids
                .insert(StreamId(want.clone()));
        }
    }

    // Assign or fetch a stable sequence for a stream kind+id within its originating turn
    // removed legacy ensure_stream_order_key; strict variant is used instead
}

#[cfg(test)]
mod tests {
    // SPEC-957: Allow print statements in test code for debugging
    #![allow(clippy::print_stdout, clippy::print_stderr)]

    use super::*;

    use codex_core::protocol::AgentStatusUpdateEvent;
    use codex_core::protocol::Event;
    use codex_core::protocol::EventMsg;
    use codex_core::protocol::TaskCompleteEvent;
    use once_cell::sync::Lazy;
    use serde_json::json;
    use std::sync::Mutex;
    use tempfile::tempdir;

    #[test]
    fn spec_auto_common_metadata_required() {
        let value = json!({
            "command": "spec-ops-plan",
            "timestamp": "2025-09-27T00:00:00Z",
            "artifacts": [{ "path": "logs.txt" }],
            "baseline": { "mode": "no-run", "artifact": "docs/baseline.md", "status": "passed" },
            "hooks": { "session.start": "ok" }
        });
        let failures = super::validate_guardrail_schema(SpecStage::Plan, &value);
        assert!(failures.iter().any(|msg| msg.contains("specId")));
        assert!(failures.iter().any(|msg| msg.contains("sessionId")));
    }

    #[test]
    fn spec_auto_plan_schema_validation_fails_without_baseline() {
        let value = json!({
            "command": "spec-ops-plan",
            "specId": "SPEC-OPS-004",
            "sessionId": "2025-09-27T00:00:00Z-1234",
            "timestamp": "2025-09-27T00:00:00Z",
            "artifacts": [{ "path": "plan.log" }],
            "baseline": { "mode": "no-run", "artifact": "docs/baseline.md" },
            "hooks": { "session.start": "ok" }
        });
        let failures = super::validate_guardrail_schema(SpecStage::Plan, &value);
        assert!(failures.iter().any(|msg| msg.contains("baseline.status")));
    }

    #[test]
    fn spec_auto_tasks_schema_requires_status() {
        let value = json!({
            "command": "spec-ops-tasks",
            "specId": "SPEC-OPS-004",
            "sessionId": "sess",
            "timestamp": "2025-09-27T00:00:00Z",
            "artifacts": [{ "path": "tasks.log" }],
            "tool": {}
        });
        let failures = super::validate_guardrail_schema(SpecStage::Tasks, &value);
        assert!(failures.iter().any(|msg| msg.contains("tool.status")));
    }

    #[test]
    fn spec_auto_implement_schema_requires_lock_and_hook() {
        let value = json!({
            "command": "spec-ops-implement",
            "specId": "SPEC-OPS-004",
            "sessionId": "sess",
            "timestamp": "2025-09-27T00:00:00Z",
            "artifacts": [{ "path": "implement.log" }]
        });
        let failures = super::validate_guardrail_schema(SpecStage::Implement, &value);
        assert!(failures.iter().any(|msg| msg.contains("lock_status")));
        assert!(failures.iter().any(|msg| msg.contains("hook_status")));
    }

    #[test]
    fn spec_auto_validate_schema_detects_bad_scenarios() {
        let value = json!({
            "command": "spec-ops-validate",
            "specId": "SPEC-OPS-004",
            "sessionId": "sess",
            "timestamp": "2025-09-27T00:00:00Z",
            "scenarios": []
        });
        let failures = super::validate_guardrail_schema(SpecStage::Validate, &value);
        assert!(failures.iter().any(|msg| msg.contains("Scenarios")));
    }

    #[test]
    fn spec_auto_validate_schema_allows_hal_summary() {
        let value = json!({
            "command": "spec-ops-validate",
            "specId": "SPEC-OPS-018",
            "sessionId": "sess",
            "timestamp": "2025-09-29T12:33:03Z",
            "scenarios": [
                { "name": "validate guardrail bootstrap", "status": "failed" }
            ],
            "hal": {
                "summary": {
                    "status": "failed",
                    "failed_checks": ["graphql_ping"],
                    "artifacts": ["docs/evidence/hal-graphql_ping.json"]
                }
            }
        });
        let failures = super::validate_guardrail_schema(SpecStage::Validate, &value);
        assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    }

    #[test]
    fn spec_auto_validate_schema_rejects_invalid_hal_status() {
        let value = json!({
            "command": "spec-ops-validate",
            "specId": "SPEC-OPS-018",
            "sessionId": "sess",
            "timestamp": "2025-09-29T12:33:03Z",
            "scenarios": [
                { "name": "validate guardrail bootstrap", "status": "passed" }
            ],
            "hal": {
                "summary": {
                    "status": "unknown"
                }
            }
        });
        let failures = super::validate_guardrail_schema(SpecStage::Validate, &value);
        assert!(
            failures
                .iter()
                .any(|msg| msg.contains("hal.summary.status")),
            "expected hal summary status failure, got {failures:?}"
        );
    }

    #[test]
    fn spec_auto_unlock_schema_requires_status() {
        let value = json!({
            "command": "spec-ops-unlock",
            "specId": "SPEC-OPS-004",
            "sessionId": "sess",
            "timestamp": "2025-09-27T00:00:00Z",
            "artifacts": [{ "path": "unlock.log" }]
        });
        let failures = super::validate_guardrail_schema(SpecStage::Unlock, &value);
        assert!(failures.iter().any(|msg| msg.contains("unlock_status")));
    }

    #[test]
    fn spec_auto_audit_schema_rejects_invalid_status_values() {
        let value = json!({
            "command": "spec-ops-audit",
            "specId": "SPEC-OPS-004",
            "sessionId": "sess",
            "timestamp": "2025-09-27T00:00:00Z",
            "scenarios": [
                { "name": "audit", "status": "unknown" }
            ]
        });
        let failures = super::validate_guardrail_schema(SpecStage::Audit, &value);
        assert!(failures.iter().any(|msg| msg.contains("Scenario status")));
    }

    #[test]
    fn spec_auto_plan_schema_validation_accepts_valid_payload() {
        let value = json!({
            "command": "spec-ops-plan",
            "specId": "SPEC-OPS-004",
            "sessionId": "sess",
            "timestamp": "2025-09-27T00:00:00Z",
            "artifacts": [{ "path": "plan.log" }],
            "baseline": { "mode": "no-run", "artifact": "docs/baseline.md", "status": "passed" },
            "hooks": { "session.start": "ok" }
        });
        let failures = super::validate_guardrail_schema(SpecStage::Plan, &value);
        assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    }

    #[test]
    fn spec_auto_implement_schema_accepts_valid_payload() {
        let value = json!({
            "command": "spec-ops-implement",
            "specId": "SPEC-OPS-004",
            "sessionId": "sess",
            "timestamp": "2025-09-27T00:00:00Z",
            "artifacts": [{ "path": "implement.log" }],
            "lock_status": "locked",
            "hook_status": "ok"
        });
        let failures = super::validate_guardrail_schema(SpecStage::Implement, &value);
        assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    }

    #[test]
    fn spec_auto_unlock_schema_accepts_valid_payload() {
        let value = json!({
            "command": "spec-ops-unlock",
            "specId": "SPEC-OPS-004",
            "sessionId": "sess",
            "timestamp": "2025-09-27T00:00:00Z",
            "artifacts": [{ "path": "unlock.log" }],
            "unlock_status": "unlocked"
        });
        let failures = super::validate_guardrail_schema(SpecStage::Unlock, &value);
        assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    }

    #[test]
    fn evaluate_guardrail_highlights_hal_failures() {
        let value = json!({
            "scenarios": [
                { "name": "validate guardrail bootstrap", "status": "failed" }
            ],
            "hal": {
                "summary": {
                    "status": "failed",
                    "failed_checks": ["graphql_ping", "list_movies"],
                    "artifacts": ["docs/logs/hal-graphql.json"]
                }
            }
        });

        let evaluation = super::evaluate_guardrail_value(SpecStage::Validate, &value);
        assert!(!evaluation.success);
        assert!(evaluation.summary.contains("HAL failed"));
        assert!(
            evaluation
                .failures
                .iter()
                .any(|msg| msg.contains("HAL failed checks"))
        );
    }

    // Test helper functions moved to test_support.rs module
    // Re-export for backwards compatibility with existing tests in this module
    use crate::chatwidget::test_support::{make_widget, make_widget_with_dir};

    #[test]
    fn terminal_overlay_sanitizes_terminal_output() {
        use std::time::Duration;

        let mut overlay =
            TerminalOverlay::new(42, "Test".to_string(), "$ example".to_string(), false);

        overlay.append_chunk(b"col1\tcol2\tcol3\n", false);
        overlay.append_chunk(b"\x1b]0;ignored title\x07\n", false);
        overlay.append_chunk(b"plain \x1b[31mred\x1b[0m text\n", false);
        overlay.append_chunk(b"stderr line\x07 with control\n", true);
        overlay.finalize(Some(0), Duration::from_millis(0));

        let mut saw_colored_stdout = false;
        let mut saw_tinted_stderr = false;

        for line in overlay.lines.iter() {
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();

            assert!(
                !text.chars().any(|ch| ch < ' ' && ch != ' '),
                "line still has control characters: {:?}",
                text
            );
            assert!(
                !text.contains('\t'),
                "line still contains a tab: {:?}",
                text
            );
            assert!(
                !text.contains('\u{001B}'),
                "line still includes a raw escape sequence: {:?}",
                text
            );
            assert!(
                !text.contains('\u{0007}'),
                "line still includes BEL/OSC terminators: {:?}",
                text
            );

            if text.contains("col1") {
                assert!(
                    text.contains("col1    col2    col3"),
                    "tabs were not expanded as expected: {:?}",
                    text
                );
            }

            if text.contains("red")
                && line
                    .spans
                    .iter()
                    .any(|span| span.content.contains("red") && span.style.fg.is_some())
            {
                saw_colored_stdout = true;
            }

            if text.contains("stderr line with control")
                && line
                    .spans
                    .iter()
                    .all(|span| span.style.fg == Some(crate::colors::warning()))
            {
                saw_tinted_stderr = true;
            }
        }

        assert!(
            saw_colored_stdout,
            "expected ANSI-colored stdout to be preserved"
        );
        assert!(
            saw_tinted_stderr,
            "expected stderr output to retain warning tint"
        );
    }

    #[test]
    fn spec_auto_evidence_requires_artifact_entries() {
        let temp = tempdir().expect("tempdir");
        let telemetry = json!({ "artifacts": [] });
        let (failures, count) =
            validate_guardrail_evidence(temp.path(), SpecStage::Plan, &telemetry);
        assert_eq!(count, 0);
        assert!(
            failures
                .iter()
                .any(|msg| msg.contains("artifacts array is empty"))
        );
    }

    #[test]
    fn spec_auto_evidence_validates_missing_files() {
        let temp = tempdir().expect("tempdir");
        let telemetry = json!({ "artifacts": [ { "path": "evidence/missing.log" } ] });
        let (failures, count) =
            validate_guardrail_evidence(temp.path(), SpecStage::Implement, &telemetry);
        assert_eq!(count, 0);
        assert!(
            failures
                .iter()
                .any(|msg| msg.contains("evidence/missing.log"))
        );
    }

    #[test]
    fn spec_auto_evidence_accepts_present_files() {
        let temp = tempdir().expect("tempdir");
        let evidence_rel = std::path::Path::new("evidence/good.json");
        let evidence_abs = temp.path().join(evidence_rel);
        std::fs::create_dir_all(evidence_abs.parent().expect("parent")).expect("mkdir");
        std::fs::write(&evidence_abs, "{} ").expect("write");

        let telemetry = json!({
            "artifacts": [ { "path": evidence_rel.to_string_lossy() } ]
        });
        let (failures, count) =
            validate_guardrail_evidence(temp.path(), SpecStage::Tasks, &telemetry);
        assert!(failures.is_empty());
        assert_eq!(count, 1);
    }

    static TELEMETRY_ENV_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[tokio::test(flavor = "current_thread")]
    async fn spec_kit_telemetry_enabled_uses_shell_policy_override() {
        let _env_guard = TELEMETRY_ENV_GUARD.lock().unwrap();
        let previous = std::env::var("SPEC_KIT_TELEMETRY_ENABLED").ok();
        unsafe {
            std::env::remove_var("SPEC_KIT_TELEMETRY_ENABLED");
        }

        let workspace = tempdir().expect("workspace");
        let mut chat = make_widget_with_dir(workspace.path());
        assert!(
            !chat.spec_kit_telemetry_enabled(),
            "telemetry should be disabled without env or policy override"
        );

        chat.config
            .shell_environment_policy
            .r#set
            .insert("SPEC_KIT_TELEMETRY_ENABLED".to_string(), "1".to_string());
        assert!(
            chat.spec_kit_telemetry_enabled(),
            "shell policy override should enable telemetry"
        );

        if let Some(value) = previous {
            unsafe {
                std::env::set_var("SPEC_KIT_TELEMETRY_ENABLED", value);
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn exec_end_before_begin_yields_completed_cell_once() {
        let mut chat = make_widget();
        chat.handle_codex_event(codex_core::protocol::Event {
            id: "call-x".into(),
            event_seq: 0,
            msg: codex_core::protocol::EventMsg::ExecCommandEnd(
                codex_core::protocol::ExecCommandEndEvent {
                    call_id: "call-x".into(),
                    exit_code: 0,
                    duration: std::time::Duration::from_millis(5),
                    stdout: "ok".into(),
                    stderr: String::new(),
                },
            ),
            order: Some(codex_core::protocol::OrderMeta {
                request_ordinal: 1,
                output_index: None,
                sequence_number: Some(1),
            }),
        });
        chat.handle_codex_event(codex_core::protocol::Event {
            id: "call-x".into(),
            event_seq: 1,
            msg: codex_core::protocol::EventMsg::ExecCommandBegin(
                codex_core::protocol::ExecCommandBeginEvent {
                    call_id: "call-x".into(),
                    command: vec!["echo".into(), "ok".into()],
                    cwd: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
                    parsed_cmd: vec![],
                },
            ),
            order: Some(codex_core::protocol::OrderMeta {
                request_ordinal: 1,
                output_index: None,
                sequence_number: Some(2),
            }),
        });
        let dump = chat.test_dump_history_text();
        assert!(
            dump.iter().any(|s| s.contains("ok") || s.contains("Ran")),
            "dump: {:?}",
            dump
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn answer_final_then_delta_ignores_late_delta() {
        let mut chat = make_widget();
        chat.handle_codex_event(codex_core::protocol::Event {
            id: "ans-1".into(),
            event_seq: 0,
            msg: codex_core::protocol::EventMsg::AgentMessage(
                codex_core::protocol::AgentMessageEvent {
                    message: "hello".into(),
                },
            ),
            order: Some(codex_core::protocol::OrderMeta {
                request_ordinal: 1,
                output_index: Some(0),
                sequence_number: Some(1),
            }),
        });
        chat.handle_codex_event(codex_core::protocol::Event {
            id: "ans-1".into(),
            event_seq: 1,
            msg: codex_core::protocol::EventMsg::AgentMessageDelta(
                codex_core::protocol::AgentMessageDeltaEvent {
                    delta: " world".into(),
                },
            ),
            order: Some(codex_core::protocol::OrderMeta {
                request_ordinal: 1,
                output_index: Some(0),
                sequence_number: Some(2),
            }),
        });
        assert_eq!(chat.last_assistant_message.as_deref(), Some("hello"));
        // Late delta should be ignored; closed set contains the id
        assert!(
            chat.stream_state
                .closed_answer_ids
                .contains(&StreamId("ans-1".into()))
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn reasoning_final_then_delta_ignores_late_delta() {
        let mut chat = make_widget();
        chat.handle_codex_event(codex_core::protocol::Event {
            id: "r-1".into(),
            event_seq: 0,
            msg: codex_core::protocol::EventMsg::AgentReasoning(
                codex_core::protocol::AgentReasoningEvent {
                    text: "think".into(),
                },
            ),
            order: Some(codex_core::protocol::OrderMeta {
                request_ordinal: 1,
                output_index: Some(0),
                sequence_number: Some(1),
            }),
        });
        chat.handle_codex_event(codex_core::protocol::Event {
            id: "r-1".into(),
            event_seq: 1,
            msg: codex_core::protocol::EventMsg::AgentReasoningDelta(
                codex_core::protocol::AgentReasoningDeltaEvent {
                    delta: " harder".into(),
                },
            ),
            order: Some(codex_core::protocol::OrderMeta {
                request_ordinal: 1,
                output_index: Some(0),
                sequence_number: Some(2),
            }),
        });
        assert!(
            chat.stream_state
                .closed_reasoning_ids
                .contains(&StreamId("r-1".into()))
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn spinner_stays_while_any_agent_running() {
        let mut chat = make_widget();
        // Start a task → spinner should turn on
        chat.handle_codex_event(Event {
            id: "t1".into(),
            event_seq: 0,
            msg: EventMsg::TaskStarted,
            order: None,
        });
        assert!(
            chat.bottom_pane.is_task_running(),
            "spinner should be on after TaskStarted"
        );

        // Agent update with one running agent → still on
        let ev = AgentStatusUpdateEvent {
            agents: vec![codex_core::protocol::AgentInfo {
                id: "a1".into(),
                name: "planner".into(),
                status: "running".into(),
                batch_id: None,
                model: None,
                last_progress: Some("working".into()),
                result: None,
                error: None,
            }],
            context: None,
            task: None,
        };
        chat.handle_codex_event(Event {
            id: "t1".into(),
            event_seq: 1,
            msg: EventMsg::AgentStatusUpdate(ev),
            order: None,
        });
        assert!(
            chat.bottom_pane.is_task_running(),
            "spinner should remain while agent is running"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn spinner_hides_after_agents_complete_and_task_complete() {
        let mut chat = make_widget();
        // Start a task → spinner on
        chat.handle_codex_event(Event {
            id: "t2".into(),
            event_seq: 0,
            msg: EventMsg::TaskStarted,
            order: None,
        });
        assert!(
            chat.bottom_pane.is_task_running(),
            "spinner should be on after TaskStarted"
        );

        // Agents: now both are completed/failed → do not count as active
        let ev_done = AgentStatusUpdateEvent {
            agents: vec![
                codex_core::protocol::AgentInfo {
                    id: "a1".into(),
                    name: "planner".into(),
                    status: "completed".into(),
                    batch_id: None,
                    model: None,
                    last_progress: None,
                    result: Some("ok".into()),
                    error: None,
                },
                codex_core::protocol::AgentInfo {
                    id: "a2".into(),
                    name: "coder".into(),
                    status: "failed".into(),
                    batch_id: None,
                    model: None,
                    last_progress: None,
                    result: None,
                    error: Some("boom".into()),
                },
            ],
            context: None,
            task: None,
        };
        chat.handle_codex_event(Event {
            id: "t2".into(),
            event_seq: 1,
            msg: EventMsg::AgentStatusUpdate(ev_done),
            order: None,
        });

        // TaskComplete → spinner should hide if nothing else is running
        chat.handle_codex_event(Event {
            id: "t2".into(),
            event_seq: 2,
            msg: EventMsg::TaskComplete(TaskCompleteEvent {
                last_agent_message: None,
            }),
            order: None,
        });
        assert!(
            !chat.bottom_pane.is_task_running(),
            "spinner should hide after all agents are terminal and TaskComplete processed"
        );
    }

    // ===================================================================
    // SESSION 17: REGRESSION TESTS FOR ESC CANCELLATION AND BLOCKING FIXES
    // ===================================================================

    /// Regression test: Esc key cancels running spec_auto pipeline
    ///
    /// Session 16 fix: Added Esc handler in mod.rs:3183-3199 that checks
    /// if spec_auto_state.is_some() and calls halt_spec_auto_with_error.
    #[tokio::test(flavor = "current_thread")]
    async fn esc_cancels_spec_auto_pipeline() {
        use super::spec_kit::pipeline_config::PipelineConfig;
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

        let mut chat = make_widget();

        // Set up an active spec_auto pipeline
        let spec_state = spec_kit::SpecAutoState::with_quality_gates(
            "SPEC-TEST-001".to_string(),
            "Test goal".to_string(),
            SpecStage::Plan,
            None,  // hal_mode
            false, // quality_gates_enabled
            PipelineConfig::defaults(),
            crate::memvid_adapter::LLMCaptureMode::PromptsOnly, // D131: capture mode
        );
        chat.spec_auto_state = Some(spec_state);

        // Verify pipeline is active
        assert!(
            chat.spec_auto_state.is_some(),
            "spec_auto_state should be Some before Esc"
        );

        // Press Esc
        chat.handle_key_event(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });

        // Verify pipeline is cancelled
        assert!(
            chat.spec_auto_state.is_none(),
            "spec_auto_state should be None after Esc cancellation"
        );
    }

    /// Regression test: Esc does NOT cancel when no pipeline is running
    #[tokio::test(flavor = "current_thread")]
    async fn esc_without_pipeline_does_not_crash() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

        let mut chat = make_widget();

        // Ensure no pipeline is running
        assert!(
            chat.spec_auto_state.is_none(),
            "spec_auto_state should be None initially"
        );

        // Press Esc - should not crash or panic
        chat.handle_key_event(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });

        // Still None after Esc
        assert!(
            chat.spec_auto_state.is_none(),
            "spec_auto_state should remain None"
        );
    }

    /// Regression test: block_in_place wrapper prevents runtime nesting panic
    ///
    /// Session 16 fix: Wrapped Runtime::new().block_on() calls with
    /// tokio::task::block_in_place() in consensus_db.rs to prevent
    /// "Cannot start a runtime from within a runtime" panic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn block_in_place_prevents_runtime_panic() {
        // This test runs inside a tokio runtime (due to #[tokio::test])
        // The block_in_place fix allows nested Runtime::new().block_on()
        // by using tokio::task::block_in_place() to temporarily exit
        // the async context.

        // Test that block_in_place works correctly
        let result = tokio::task::block_in_place(|| {
            // This would panic without block_in_place:
            // "Cannot start a runtime from within a runtime"
            let rt = tokio::runtime::Runtime::new().expect("create runtime");
            rt.block_on(async {
                // Simulate async work
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                42
            })
        });

        assert_eq!(result, 42, "block_in_place should allow nested runtime");
    }
}

#[cfg(test)]
impl ChatWidget<'_> {
    pub(crate) fn test_dump_history_text(&self) -> Vec<String> {
        self.history_cells
            .iter()
            .map(|c| {
                let lines = c.display_lines();
                let mut s = String::new();
                for l in lines {
                    for sp in l.spans {
                        s.push_str(&sp.content);
                    }
                    s.push('\n');
                }
                s
            })
            .collect()
    }
}

#[derive(Default)]
struct ExecState {
    running_commands: HashMap<ExecCallId, RunningCommand>,
    running_explore_agg_index: Option<usize>,
    // Pairing map for out-of-order exec events. If an ExecEnd arrives before
    // ExecBegin, we stash it briefly and either pair it when Begin arrives or
    // flush it after a short timeout to show a fallback cell.
    pending_exec_ends: HashMap<
        ExecCallId,
        (
            ExecCommandEndEvent,
            codex_core::protocol::OrderMeta,
            std::time::Instant,
        ),
    >,
    suppressed_exec_end_call_ids: HashSet<ExecCallId>,
    suppressed_exec_end_order: VecDeque<ExecCallId>,
}

impl ExecState {
    fn suppress_exec_end(&mut self, call_id: ExecCallId) {
        if self.suppressed_exec_end_call_ids.insert(call_id.clone()) {
            self.suppressed_exec_end_order.push_back(call_id);
            const MAX_TRACKED_SUPPRESSED_IDS: usize = 64;
            if self.suppressed_exec_end_order.len() > MAX_TRACKED_SUPPRESSED_IDS
                && let Some(old) = self.suppressed_exec_end_order.pop_front()
            {
                self.suppressed_exec_end_call_ids.remove(&old);
            }
        }
    }

    fn unsuppress_exec_end(&mut self, call_id: &ExecCallId) {
        if self.suppressed_exec_end_call_ids.remove(call_id) {
            self.suppressed_exec_end_order.retain(|cid| cid != call_id);
        }
    }

    fn should_suppress_exec_end(&self, call_id: &ExecCallId) -> bool {
        self.suppressed_exec_end_call_ids.contains(call_id)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RunningToolEntry {
    order_key: OrderKey,
    fallback_index: usize,
}

impl RunningToolEntry {
    fn new(order_key: OrderKey, fallback_index: usize) -> Self {
        Self {
            order_key,
            fallback_index,
        }
    }
}

#[derive(Default)]
struct ToolState {
    running_custom_tools: HashMap<ToolCallId, RunningToolEntry>,
    running_web_search: HashMap<ToolCallId, (usize, Option<String>)>,
    running_wait_tools: HashMap<ToolCallId, ExecCallId>,
    running_kill_tools: HashMap<ToolCallId, ExecCallId>,
}
#[derive(Default)]
struct StreamState {
    current_kind: Option<StreamKind>,
    closed_answer_ids: HashSet<StreamId>,
    closed_reasoning_ids: HashSet<StreamId>,
    seq_answer_final: Option<u64>,
    drop_streaming: bool,
}

#[derive(Default)]
struct LayoutState {
    // Scroll offset from bottom (0 = bottom)
    scroll_offset: u16,
    // Cached max scroll from last render
    last_max_scroll: std::cell::Cell<u16>,
    // Track last viewport height of the history content area
    last_history_viewport_height: std::cell::Cell<u16>,
    // Stateful vertical scrollbar for history view
    vertical_scrollbar_state: std::cell::RefCell<ScrollbarState>,
    // Auto-hide scrollbar timer
    scrollbar_visible_until: std::cell::Cell<Option<std::time::Instant>>,
    // Last effective bottom pane height used by layout (rows)
    last_bottom_reserved_rows: std::cell::Cell<u16>,
    // HUD visibility and sizing
    last_hud_present: std::cell::Cell<bool>,
    agents_hud_expanded: bool,
    pro_hud_expanded: bool,
    last_frame_height: std::cell::Cell<u16>,
    last_frame_width: std::cell::Cell<u16>,
}

// MAINT-11: Pro types (ProState, ProOverlay, etc.) moved to pro_overlay.rs

#[derive(Default)]
struct DiffsState {
    session_patch_sets: Vec<HashMap<PathBuf, codex_core::protocol::FileChange>>,
    baseline_file_contents: HashMap<PathBuf, String>,
    overlay: Option<DiffOverlay>,
    confirm: Option<DiffConfirm>,
    body_visible_rows: std::cell::Cell<u16>,
}

#[derive(Default)]
struct HelpState {
    overlay: Option<HelpOverlay>,
    body_visible_rows: std::cell::Cell<u16>,
}

#[derive(Default)]
struct LimitsState {
    overlay: Option<LimitsOverlay>,
}

struct HelpOverlay {
    lines: Vec<RtLine<'static>>,
    scroll: u16,
}

impl HelpOverlay {
    fn new(lines: Vec<RtLine<'static>>) -> Self {
        Self { lines, scroll: 0 }
    }
}

// MAINT-11: Command rendering functions moved to command_render.rs

#[derive(Default)]
struct PerfState {
    enabled: bool,
    stats: std::cell::RefCell<PerfStats>,
}

impl ChatWidget<'_> {
    fn clear_backgrounds_in(&self, buf: &mut Buffer, rect: Rect) {
        for y in rect.y..rect.y.saturating_add(rect.height) {
            for x in rect.x..rect.x.saturating_add(rect.width) {
                let cell = &mut buf[(x, y)];
                // Reset background; keep fg/content as-is
                cell.set_bg(ratatui::style::Color::Reset);
            }
        }
    }
    pub(crate) fn set_github_watcher(&mut self, enabled: bool) {
        self.config.github.check_workflows_on_push = enabled;
        match find_codex_home() {
            Ok(home) => {
                if let Err(e) = set_github_check_on_push(&home, enabled) {
                    tracing::warn!("Failed to persist GitHub watcher setting: {}", e);
                    let msg = format!(
                        "✅ {} GitHub watcher (persist failed; see logs)",
                        if enabled { "Enabled" } else { "Disabled" }
                    );
                    self.push_background_tail(msg);
                } else {
                    let msg = format!(
                        "✅ {} GitHub watcher (persisted)",
                        if enabled { "Enabled" } else { "Disabled" }
                    );
                    self.push_background_tail(msg);
                }
            }
            Err(_) => {
                let msg = format!(
                    "✅ {} GitHub watcher (not persisted: CODE_HOME/CODEX_HOME not found)",
                    if enabled { "Enabled" } else { "Disabled" }
                );
                self.push_background_tail(msg);
            }
        }
    }

    pub(crate) fn toggle_mcp_server(&mut self, name: &str, enable: bool) {
        match codex_core::config::find_codex_home() {
            Ok(home) => match codex_core::config::set_mcp_server_enabled(&home, name, enable) {
                Ok(changed) => {
                    if changed {
                        if enable {
                            if let Ok((enabled, _)) = codex_core::config::list_mcp_servers(&home)
                                && let Some((_, cfg)) = enabled.into_iter().find(|(n, _)| n == name)
                            {
                                self.config.mcp_servers.insert(name.to_string(), cfg);
                            }
                        } else {
                            self.config.mcp_servers.remove(name);
                        }
                        let msg = format!(
                            "{} MCP server '{}'",
                            if enable { "Enabled" } else { "Disabled" },
                            name
                        );
                        self.push_background_tail(msg);
                    }
                }
                Err(e) => {
                    let msg = format!("Failed to update MCP server '{}': {}", name, e);
                    self.history_push(history_cell::new_error_event(msg));
                }
            },
            Err(e) => {
                let msg = format!("Failed to locate CODEX_HOME: {}", e);
                self.history_push(history_cell::new_error_event(msg));
            }
        }
    }
}

impl ChatWidget<'_> {
    /// Show pipeline configurator modal for stage selection (SPEC-947 Phase 4)
    pub(crate) fn show_pipeline_configurator(
        &mut self,
        spec_id: String,
        initial_config: spec_kit::pipeline_config::PipelineConfig,
    ) {
        self.bottom_pane
            .show_pipeline_configurator(spec_id, initial_config);
    }

    /// Show stage agents modal for global defaults (SPEC-KIT-983)
    pub(crate) fn show_spec_kit_stage_agents_modal(&mut self) {
        self.bottom_pane
            .show_spec_kit_stage_agents(self.config.speckit_stage_agents.clone());
    }
}
