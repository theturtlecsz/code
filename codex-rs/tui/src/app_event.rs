use crate::streaming::StreamKind;
use codex_core::config_types::ReasoningEffort;
use codex_core::config_types::TextVerbosity;
use codex_core::config_types::ThemeName;
use codex_core::git_info::CommitLogEntry;
use codex_core::protocol::ApprovedCommandMatchKind;
use codex_core::protocol::Event;
use codex_core::protocol::ReviewContextMetadata;
use codex_core::protocol::ValidationGroup;
use codex_file_search::FileMatch;
use crossterm::event::KeyEvent;
use crossterm::event::MouseEvent;
use ratatui::text::Line;
use std::time::Duration;

use crate::app::ChatWidgetArgs;
use crate::chatwidget::spec_kit::{QualityGateBrokerResult, QualityGateValidationResult};
use crate::slash_command::SlashCommand;
use codex_protocol::models::ResponseItem;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::Sender as StdSender;

/// Wrapper to allow including non-Debug types in Debug enums without leaking internals.
pub(crate) struct Redacted<T>(pub T);

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TerminalRunController {
    pub tx: StdSender<TerminalRunEvent>,
}

#[derive(Debug, Clone)]
pub(crate) struct TerminalLaunch {
    pub id: u64,
    pub title: String,
    pub command: Vec<String>,
    pub command_display: String,
    pub controller: Option<TerminalRunController>,
    pub auto_close_on_success: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum TerminalRunEvent {
    Chunk {
        data: Vec<u8>,
        _is_stderr: bool,
    },
    Exit {
        exit_code: Option<i32>,
        _duration: Duration,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum TerminalCommandGate {
    Run(String),
    Cancel,
}

#[derive(Debug, Clone)]
pub(crate) enum TerminalAfter {
    RefreshAgentsAndClose { selected_index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundPlacement {
    /// Default: append to the end of the current request/history window.
    Tail,
    /// Display immediately before the next provider/tool output for the active request.
    BeforeNextOutput,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum AppEvent {
    CodexEvent(Event),

    /// Request a redraw which will be debounced by the [`App`].
    RequestRedraw,

    /// Actually draw the next frame.
    Redraw,

    /// Update the terminal title override. `None` restores the default title.
    SetTerminalTitle {
        title: Option<String>,
    },

    /// Schedule a one-shot animation frame roughly after the given duration.
    /// Multiple requests are coalesced by the central frame scheduler.
    ScheduleFrameIn(Duration),

    /// Internal: flush any pending out-of-order ExecEnd events that did not
    /// receive a matching ExecBegin within a short pairing window. This lets
    /// the TUI render a fallback "Ran call_<id>" cell so output is not lost.
    FlushPendingExecEnds,

    KeyEvent(KeyEvent),

    MouseEvent(MouseEvent),

    /// Text pasted from the terminal clipboard.
    Paste(String),

    /// Request to exit the application gracefully.
    ExitRequest,

    /// Automation completed successfully (SPEC-KIT-920: headless automation)
    AutomationSuccess,

    /// Automation failed or was aborted (SPEC-KIT-920: headless automation)
    AutomationFailure,

    /// Forward an `Op` to the Agent. Using an `AppEvent` for this avoids
    /// bubbling channels through layers of widgets.
    CodexOp(codex_core::protocol::Op),

    /// Async completion from the quality gate broker, delivering agent payloads
    /// retrieved via local-memory without blocking the TUI thread.
    SpecKitQualityGateResults {
        broker_result: QualityGateBrokerResult,
    },

    /// Async completion from the GPT-5 validation broker fetch.
    SpecKitQualityGateValidationResults {
        broker_result: QualityGateValidationResult,
    },

    /// Pipeline configurator: configuration saved successfully
    PipelineConfigurationSaved {
        spec_id: String,
        config_path: String,
        enabled_count: usize,
        total_cost: f64,
        total_duration: u32,
    },

    /// Pipeline configurator: error occurred during save
    PipelineConfigurationError {
        spec_id: String,
        error: String,
    },

    /// Pipeline configurator: user cancelled configuration
    PipelineConfigurationCancelled {
        spec_id: String,
    },

    /// Dispatch a recognized slash command from the UI (composer) to the app
    /// layer so it can be handled centrally. Includes the full command text.
    DispatchCommand(SlashCommand, String),
    /// Submit prompt with ACE-enhanced content (async completion)
    SubmitPreparedPrompt {
        display: String,
        prompt: String,
    },

    /// Open undo options for a previously captured snapshot.
    ShowUndoOptions {
        index: usize,
    },
    /// Restore workspace state according to the chosen undo scope.
    PerformUndoRestore {
        index: usize,
        restore_files: bool,
        restore_conversation: bool,
    },

    /// Switch to a new working directory by rebuilding the chat widget with
    /// the same configuration but a different `cwd`. Optionally submits an
    /// initial prompt once the new session is ready.
    SwitchCwd(std::path::PathBuf, Option<String>),

    /// Signal that agents are about to start (triggered by multi-agent workflows)
    PrepareAgents,

    /// Update the model and optional reasoning effort preset
    UpdateModelSelection {
        model: String,
        effort: Option<ReasoningEffort>,
    },

    /// Update the text verbosity level
    UpdateTextVerbosity(TextVerbosity),

    /// Update GitHub workflow monitoring toggle
    UpdateGithubWatcher(bool),
    /// Enable/disable a specific validation tool
    UpdateValidationTool {
        name: String,
        enable: bool,
    },
    /// Enable/disable an entire validation group
    UpdateValidationGroup {
        group: ValidationGroup,
        enable: bool,
    },
    /// Start installing a validation tool through the terminal overlay
    RequestValidationToolInstall {
        name: String,
        command: String,
    },

    /// Enable/disable a specific MCP server
    UpdateMcpServer {
        name: String,
        enable: bool,
    },

    /// Prefill the composer input with the given text
    PrefillComposer(String),

    /// Submit a message with hidden preface instructions
    SubmitTextWithPreface {
        visible: String,
        preface: String,
    },

    /// Run a review with an explicit prompt/hint pair (used by TUI selections)
    RunReviewWithScope {
        prompt: String,
        hint: String,
        preparation_label: Option<String>,
        metadata: Option<ReviewContextMetadata>,
    },

    /// Run the review command with the given argument string (mirrors `/review <args>`)
    RunReviewCommand(String),

    /// Open a bottom-pane form that lets the user select a commit to review.
    StartReviewCommitPicker,
    /// Populate the commit picker with retrieved commit entries.
    PresentReviewCommitPicker {
        commits: Vec<CommitLogEntry>,
    },
    /// Open a bottom-pane form that lets the user select a base branch to diff against.
    StartReviewBranchPicker,
    /// Populate the branch picker with branch metadata once loaded asynchronously.
    PresentReviewBranchPicker {
        current_branch: Option<String>,
        branches: Vec<String>,
    },

    /// Show the multi-line prompt input to collect custom review instructions.
    OpenReviewCustomPrompt,

    /// Update the theme (with history event)
    UpdateTheme(ThemeName),
    /// Add or update a subagent command in memory (UI already persisted to config.toml)
    UpdateSubagentCommand(codex_core::config_types::SubagentCommandConfig),
    /// Remove a subagent command from memory (UI already deleted from config.toml)
    DeleteSubagentCommand(String),
    /// Update stage→agent defaults from modal (SPEC-KIT-983)
    /// Persists to root config.toml under [speckit.stage_agents]
    UpdateSpecKitStageAgents(codex_core::config_types::SpecKitStageAgents),
    /// Return to the Agents settings list view
    // ShowAgentsSettings removed; overview replaces it
    /// Return to the Agents overview (Agents + Commands)
    ShowAgentsOverview,
    /// Open the agent editor form for a specific agent name
    ShowAgentEditor {
        name: String,
    },
    // ShowSubagentEditor removed; use ShowSubagentEditorForName or ShowSubagentEditorNew
    /// Open the subagent editor for a specific command name; ChatWidget supplies data
    ShowSubagentEditorForName {
        name: String,
    },
    /// Open a blank subagent editor to create a new command
    ShowSubagentEditorNew,

    /// Preview theme (no history event)
    PreviewTheme(ThemeName),
    /// Update the loading spinner style (with history event)
    UpdateSpinner(String),
    /// Preview loading spinner (no history event)
    PreviewSpinner(String),
    /// Rotate access/safety preset (Read Only → Write with Approval → Full Access)
    CycleAccessMode,
    /// Bottom composer expanded (e.g., slash command popup opened)
    ComposerExpanded,

    /// Show the main account picker view for /login
    ShowLoginAccounts,
    /// Show the add-account flow for /login
    ShowLoginAddAccount,

    /// Kick off an asynchronous file search for the given query (text after
    /// the `@`). Previous searches may be cancelled by the app layer so there
    /// is at most one in-flight search.
    StartFileSearch(String),

    /// Result of a completed asynchronous file search. The `query` echoes the
    /// original search term so the UI can decide whether the results are
    /// still relevant.
    FileSearchResult {
        query: String,
        matches: Vec<FileMatch>,
    },

    /// Result of computing a `/diff` command.
    #[allow(dead_code)]
    DiffResult(String),

    InsertHistory(Vec<Line<'static>>),
    InsertHistoryWithKind {
        id: Option<String>,
        kind: StreamKind,
        lines: Vec<Line<'static>>,
    },
    /// Finalized assistant answer with raw markdown for re-rendering under theme changes.
    InsertFinalAnswer {
        id: Option<String>,
        lines: Vec<Line<'static>>,
        source: String,
    },
    /// Insert a background event with explicit placement semantics.
    InsertBackgroundEvent {
        message: String,
        placement: BackgroundPlacement,
    },

    /// CLI routing completed with response (SPEC-KIT-952)
    /// Used when prompts are routed through CLI providers (Claude, Gemini)
    #[allow(dead_code)]
    CliRouteComplete {
        /// Provider display name (e.g., "Claude", "Gemini")
        provider_name: String,
        /// Model identifier used
        model_name: String,
        /// Response content (may contain markdown)
        content: String,
        /// Whether this is an error response
        is_error: bool,
    },

    /// Native provider streaming started (SPEC-KIT-953)
    /// Used when prompts are routed through native API clients (Claude, Gemini)
    NativeProviderStreamStart {
        /// Provider display name (e.g., "Claude", "Gemini")
        provider_name: String,
        /// Model identifier used
        model_name: String,
        /// Message ID from provider
        message_id: String,
    },

    /// Native provider streaming delta (SPEC-KIT-953)
    NativeProviderStreamDelta {
        /// Text content delta
        text: String,
    },

    /// Native provider streaming completed (SPEC-KIT-953)
    NativeProviderStreamComplete {
        /// Provider display name
        provider_name: String,
        /// Input tokens consumed (if available)
        input_tokens: Option<u32>,
        /// Output tokens generated (if available)
        output_tokens: Option<u32>,
    },

    /// Native provider streaming error (SPEC-KIT-953)
    NativeProviderStreamError {
        /// Provider display name
        provider_name: String,
        /// Error message
        error: String,
    },

    /// Timeout for user message that never received TaskStarted (SPEC-954)
    /// Fires when a queued message hasn't been acknowledged within timeout window
    UserMessageTimeout {
        /// Identifier for the message that timed out
        message_id: String,
        /// Time elapsed before timeout (milliseconds)
        elapsed_ms: u64,
    },

    AutoUpgradeCompleted {
        version: String,
    },

    /// Background rate limit refresh failed (threaded request).
    RateLimitFetchFailed {
        message: String,
    },

    #[allow(dead_code)]
    StartCommitAnimation,
    #[allow(dead_code)]
    StopCommitAnimation,
    CommitTick,

    /// Onboarding: result of login_with_chatgpt.
    OnboardingAuthComplete(Result<(), String>),
    OnboardingComplete(ChatWidgetArgs),

    /// Begin ChatGPT login flow from the in-app login manager.
    LoginStartChatGpt,
    /// Cancel an in-progress ChatGPT login flow triggered via `/login`.
    LoginCancelChatGpt,
    /// ChatGPT login flow has completed (success or failure).
    LoginChatGptComplete {
        result: Result<(), String>,
    },
    /// The active authentication mode changed (e.g., switched accounts).
    LoginUsingChatGptChanged {
        using_chatgpt_auth: bool,
    },

    /// Begin Claude OAuth login flow from the in-app login manager.
    LoginStartClaude,
    /// Cancel an in-progress Claude login flow triggered via `/login`.
    #[allow(dead_code)]
    LoginCancelClaude,
    /// Claude login flow has completed (success or failure).
    LoginClaudeComplete {
        result: Result<(), String>,
    },

    /// Begin Gemini OAuth login flow from the in-app login manager.
    LoginStartGemini,
    /// Cancel an in-progress Gemini login flow triggered via `/login`.
    #[allow(dead_code)]
    LoginCancelGemini,
    /// Gemini login flow has completed (success or failure).
    LoginGeminiComplete {
        result: Result<(), String>,
    },

    // === FORK-SPECIFIC: Device Code OAuth Events (P6-SYNC Phase 7) ===
    /// Start device code OAuth flow for a provider
    DeviceCodeLoginStart {
        provider: codex_login::DeviceCodeProvider,
    },

    /// Device code received from authorization server
    DeviceCodeLoginCodeReceived {
        provider: codex_login::DeviceCodeProvider,
        user_code: String,
        verification_uri: String,
        verification_uri_complete: Option<String>,
        device_code: String,
        expires_in: u64,
        interval: u64,
    },

    /// Device code poll attempt made
    DeviceCodeLoginPollAttempt {
        provider: codex_login::DeviceCodeProvider,
        poll_count: u32,
    },

    /// Device code flow completed successfully
    DeviceCodeLoginSuccess {
        provider: codex_login::DeviceCodeProvider,
    },

    /// Device code flow failed
    DeviceCodeLoginError {
        provider: codex_login::DeviceCodeProvider,
        error: String,
    },

    /// Device code expired
    DeviceCodeLoginExpired {
        provider: codex_login::DeviceCodeProvider,
    },

    /// User denied access during device code flow
    DeviceCodeLoginDenied {
        provider: codex_login::DeviceCodeProvider,
    },
    // === END FORK-SPECIFIC: Device Code OAuth Events ===
    /// Start a new chat session by resuming from the given rollout file
    ResumeFrom(std::path::PathBuf),

    /// Begin jump-back to the Nth last user message (1 = latest).
    /// Trims visible history up to that point and pre-fills the composer.
    JumpBack {
        nth: usize,
        prefill: String,
    },
    /// Result of an async jump-back fork operation performed off the UI thread.
    /// Carries the forked conversation, trimmed prefix to replay, and composer prefill.
    JumpBackForked {
        cfg: codex_core::config::Config,
        new_conv: Redacted<codex_core::NewConversation>,
        prefix_items: Vec<ResponseItem>,
        prefill: String,
    },

    /// Register an image placeholder inserted by the composer with its backing path
    /// so ChatWidget can resolve it to a LocalImage on submit.
    RegisterPastedImage {
        placeholder: String,
        path: PathBuf,
    },

    /// Immediately cancel any running task in the ChatWidget. This is used by
    /// the approval modal to reflect a user's Abort decision instantly in the UI
    /// (clear spinner/status, finalize running exec/tool cells) while the core
    /// continues its own abort/cleanup in parallel.
    CancelRunningTask,
    /// Register a command pattern as approved, optionally persisting to config.
    RegisterApprovedCommand {
        command: Vec<String>,
        match_kind: ApprovedCommandMatchKind,
        persist: bool,
        semantic_prefix: Option<Vec<String>>,
    },
    /// Indicate that an approval was denied so the UI can clear transient
    /// spinner/status state without interrupting the core task.
    MarkTaskIdle,
    OpenTerminal(TerminalLaunch),
    TerminalChunk {
        id: u64,
        chunk: Vec<u8>,
        _is_stderr: bool,
    },
    TerminalExit {
        id: u64,
        exit_code: Option<i32>,
        _duration: Duration,
    },
    TerminalCancel {
        id: u64,
    },
    TerminalRunCommand {
        id: u64,
        command: Vec<String>,
        command_display: String,
        controller: Option<TerminalRunController>,
    },
    TerminalSendInput {
        id: u64,
        data: Vec<u8>,
    },
    TerminalResize {
        id: u64,
        rows: u16,
        cols: u16,
    },
    TerminalRerun {
        id: u64,
    },
    TerminalUpdateMessage {
        id: u64,
        message: String,
    },
    TerminalForceClose {
        id: u64,
    },
    TerminalAfter(TerminalAfter),
    TerminalSetAssistantMessage {
        id: u64,
        message: String,
    },
    TerminalAwaitCommand {
        id: u64,
        suggestion: String,
        ack: Redacted<StdSender<TerminalCommandGate>>,
    },
    TerminalApprovalDecision {
        id: u64,
        approved: bool,
    },
    RunUpdateCommand {
        command: Vec<String>,
        display: String,
        latest_version: Option<String>,
    },
    SetAutoUpgradeEnabled(bool),
    RequestAgentInstall {
        name: String,
        selected_index: usize,
    },
    AgentsOverviewSelectionChanged {
        index: usize,
    },
    /// Add or update an agent's settings (enabled, params, instructions)
    UpdateAgentConfig {
        name: String,
        enabled: bool,
        args_read_only: Option<Vec<String>>,
        args_write: Option<Vec<String>>,
        instructions: Option<String>,
    },

    // === FORK-SPECIFIC: Quality gate events (T85) ===
    // Upstream: Does not have quality gates
    // Preserve: These event variants during rebases
    /// Quality gate escalation questions have been answered
    QualityGateAnswersSubmitted {
        checkpoint: crate::chatwidget::spec_kit::QualityCheckpoint,
        answers: std::collections::HashMap<String, String>,
    },

    /// Quality gate was cancelled by user
    QualityGateCancelled {
        checkpoint: crate::chatwidget::spec_kit::QualityCheckpoint,
    },

    /// PRD builder questions answered (SPEC-KIT-970)
    #[allow(dead_code)]
    PrdBuilderSubmitted {
        description: String,
        answers: std::collections::HashMap<String, String>,
    },

    /// PRD builder was cancelled by user (SPEC-KIT-970)
    #[allow(dead_code)]
    PrdBuilderCancelled {
        description: String,
    },

    /// Vision builder questions answered (P93/SPEC-KIT-105)
    VisionBuilderSubmitted {
        answers: std::collections::HashMap<String, String>,
    },

    /// Vision builder was cancelled by user (P93/SPEC-KIT-105)
    VisionBuilderCancelled,

    /// Spec intake submitted (Architect-in-a-box)
    SpecIntakeSubmitted {
        description: String,
        deep: bool,
        answers: std::collections::HashMap<String, String>,
        /// If Some, this is a backfill for existing spec (don't generate new ID)
        existing_spec_id: Option<String>,
    },

    /// Spec intake cancelled (Architect-in-a-box)
    SpecIntakeCancelled {
        description: String,
        /// If Some, this was a backfill that was cancelled
        existing_spec_id: Option<String>,
    },

    /// Project intake submitted (/speckit.projectnew flow)
    ProjectIntakeSubmitted {
        project_id: String,
        deep: bool,
        answers: std::collections::HashMap<String, String>,
    },

    /// Project intake cancelled (/speckit.projectnew flow)
    ProjectIntakeCancelled {
        project_id: String,
    },

    /// Maieutic elicitation completed (D130)
    MaieuticSubmitted {
        spec_id: String,
        answers: std::collections::HashMap<String, String>,
        duration_ms: u64,
    },

    /// Maieutic elicitation was cancelled by user (D130)
    MaieuticCancelled {
        spec_id: String,
    },

    /// Clarification markers resolved (SPEC-KIT-971)
    ClarifySubmitted {
        spec_id: String,
        resolutions: Vec<(crate::bottom_pane::clarify_modal::ClarifyQuestion, String)>,
    },

    /// Clarification was cancelled by user (SPEC-KIT-971)
    ClarifyCancelled {
        spec_id: String,
    },

    /// Native quality gate agents completed (SPEC-KIT-900)
    QualityGateNativeAgentsComplete {
        checkpoint: crate::chatwidget::spec_kit::QualityCheckpoint,
        agent_ids: Vec<String>,
    },

    /// Regular stage agents completed (SPEC-KIT-900 Session 2)
    /// Triggered when directly-spawned regular stage agents finish execution
    RegularStageAgentsComplete {
        stage: crate::spec_prompts::SpecStage,
        spec_id: String,
        agent_ids: Vec<String>,
        agent_results: Vec<(String, String)>, // (agent_name, result) - direct from spawn, not active_agents
    },

    /// Native guardrail validation completed (SPEC-KIT-900 Session 3)
    /// Triggered when async guardrail checks finish
    GuardrailComplete {
        spec_id: String,
        stage: crate::spec_prompts::SpecStage,
        success: bool,
        result_json: String, // Serialized GuardrailResult
    },

    /// Config hot-reload event (SPEC-945D Phase 2.3)
    /// Triggered when config file changes are detected and processed
    ConfigReload {
        event: codex_spec_kit::config::ConfigReloadEvent,
    },

    /// Sessions command result (Process Management)
    /// Triggered when /sessions command async work completes
    SessionsCommandResult(String),
    // === END FORK-SPECIFIC ===
}

// No helper constructor; use `AppEvent::CodexEvent(ev)` directly to avoid shadowing.
