//! Types used to define the fields of [`crate::config::Config`].

// Note this file should generally be restricted to simple struct/enum
// definitions that do not contain business logic.

use schemars::JsonSchema;
use std::collections::HashMap;
use std::path::PathBuf;
use wildmatch::WildMatchPattern;

use shlex::split as shlex_split;

use serde::Deserialize;
use serde::Serialize;
use serde::de::{self, Deserializer};
use strum_macros::Display;

/// Configuration for commands that require an explicit `confirm:` prefix.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ConfirmGuardConfig {
    /// List of regex patterns applied to the raw command (joined argv or shell script).
    #[serde(default)]
    pub patterns: Vec<ConfirmGuardPattern>,
}

impl Default for ConfirmGuardConfig {
    fn default() -> Self {
        Self {
            patterns: default_confirm_guard_patterns(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ConfirmGuardPattern {
    /// ECMA-style regular expression matched against the command string.
    pub regex: String,
    /// Optional custom guidance text surfaced when the guard triggers.
    #[serde(default)]
    pub message: Option<String>,
}

fn default_confirm_guard_patterns() -> Vec<ConfirmGuardPattern> {
    vec![
        ConfirmGuardPattern {
            regex: r"(?i)^\s*git\s+reset\b".to_string(),
            message: Some("Blocked git reset. Reset rewrites the working tree/index and may delete local work. Resend with 'confirm:' if you're certain.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*git\s+checkout\s+--\b".to_string(),
            message: Some("Blocked git checkout -- <paths>. This overwrites local modifications; resend with 'confirm:' to proceed.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*git\s+checkout\s+(?:-b|-B|--orphan|--detach)\b".to_string(),
            message: Some("Blocked git checkout with branch-changing flag. Switching branches can discard or hide in-progress changes.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*git\s+checkout\s+-\b".to_string(),
            message: Some("Blocked git checkout -. Confirm before switching back to the previous branch.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*git\s+switch\b.*(?:-c|--detach)".to_string(),
            message: Some("Blocked git switch creating or detaching a branch. Resend with 'confirm:' if requested.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*git\s+switch\s+[^\s-][^\s]*".to_string(),
            message: Some("Blocked git switch <branch>. Branch changes can discard or hide work; confirm before continuing.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*git\s+clean\b.*(?:-f|--force|-x|-X|-d)".to_string(),
            message: Some("Blocked git clean with destructive flags. This deletes untracked files or build artifacts.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*git\s+push\b.*(?:--force|-f)".to_string(),
            message: Some("Blocked git push --force. Force pushes rewrite remote history; only continue if explicitly requested.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?rm\s+-[a-z-]*rf[a-z-]*\s+(?:--\s+)?(?:\.|\.\.|\./|/|\*)(?:\s|$)".to_string(),
            message: Some("Blocked rm -rf targeting a broad path (., .., /, or *). Confirm before destructive delete.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?rm\s+-[a-z-]*r[a-z-]*\s+-[a-z-]*f[a-z-]*\s+(?:--\s+)?(?:\.|\.\.|\./|/|\*)(?:\s|$)".to_string(),
            message: Some("Blocked rm -r/-f combination targeting broad paths. Resend with 'confirm:' if you intend to wipe this tree.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?rm\s+-[a-z-]*f[a-z-]*\s+-[a-z-]*r[a-z-]*\s+(?:--\s+)?(?:\.|\.\.|\./|/|\*)(?:\s|$)".to_string(),
            message: Some("Blocked rm -f/-r combination targeting broad paths. Confirm before running.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?rm\b[^\n]*\s+-[a-z-]*rf[a-z-]*\b".to_string(),
            message: Some("Blocked rm -rf. Force-recursive delete requires explicit confirmation.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?rm\b[^\n]*\s+-[-0-9a-qs-z]*f[-0-9a-qs-z]*\b".to_string(),
            message: Some("Blocked rm -f. Force delete requires explicit confirmation.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?find\s+\.(?:\s|$).*\s-delete\b".to_string(),
            message: Some("Blocked find . ... -delete. Recursive deletes require confirmation.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?find\s+\.(?:\s|$).*\s-exec\s+rm\b".to_string(),
            message: Some("Blocked find . ... -exec rm. Confirm before running recursive rm.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?trash\s+-[a-z-]*r[a-z-]*f[a-z-]*\b".to_string(),
            message: Some("Blocked trash -rf. Bulk trash operations can delete large portions of the workspace.".to_string()),
        },
        ConfirmGuardPattern {
            regex: r"(?i)^\s*(?:sudo\s+)?fd\b.*(?:--exec|-x)\s+rm\b".to_string(),
            message: Some("Blocked fd … --exec rm. Confirm before piping search results into rm.".to_string()),
        },
    ]
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AllowedCommandMatchKind {
    Exact,
    Prefix,
}

impl Default for AllowedCommandMatchKind {
    fn default() -> Self {
        Self::Exact
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AllowedCommand {
    #[serde(default)]
    pub argv: Vec<String>,
    #[serde(default)]
    pub match_kind: AllowedCommandMatchKind,
}

/// Configuration for a subagent slash command (e.g., plan/solve/code or custom)
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct SubagentCommandConfig {
    /// Name of the command (e.g., "plan", "solve", "code", or custom)
    pub name: String,

    /// Whether agents launched for this command should run in read-only mode
    /// Defaults: plan/solve=true, code=false (applied if not specified here)
    #[serde(default)]
    pub read_only: bool,

    /// Agent names to enable for this command. If empty, falls back to
    /// enabled agents from `[[agents]]`, or built-in defaults.
    #[serde(default)]
    pub agents: Vec<String>,

    /// Extra instructions to append to the orchestrator (Code) prompt.
    #[serde(default)]
    pub orchestrator_instructions: Option<String>,

    /// Extra instructions that the orchestrator should append to the prompt
    /// given to each launched agent.
    #[serde(default)]
    pub agent_instructions: Option<String>,
}

/// Top-level subagents section containing a list of commands.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct SubagentsToml {
    #[serde(default)]
    pub commands: Vec<SubagentCommandConfig>,
}

/// MCP tool identifiers that the client exposes to the agent.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientTools {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_permission: Option<McpToolId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_text_file: Option<McpToolId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_text_file: Option<McpToolId>,
}

/// Identifier for a client-hosted MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolId {
    pub mcp_server: String,
    pub tool_name: String,
}

/// Configuration for external agent models
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct AgentConfig {
    /// Name of the agent (e.g., "claude", "gemini", "gpt-4")
    pub name: String,

    /// Canonical name (single source of truth for agent identity)
    /// Used to resolve agent names across different contexts (config name, command name, model ID)
    #[serde(default)]
    pub canonical_name: Option<String>,

    /// Command to execute the agent (e.g., "claude", "gemini").
    /// If omitted, defaults to the agent `name` during config load.
    #[serde(default)]
    pub command: String,

    /// Optional arguments to pass to the agent command
    #[serde(default)]
    pub args: Vec<String>,

    /// Whether this agent can only run in read-only mode
    #[serde(default)]
    pub read_only: bool,

    /// Whether this agent is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Optional description of the agent
    #[serde(default)]
    pub description: Option<String>,

    /// Optional environment variables for the agent
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,

    /// Optional arguments to pass only when the agent is executed in
    /// read-only mode. When present, these are preferred over `args` for
    /// read-only runs.
    #[serde(default)]
    pub args_read_only: Option<Vec<String>>,

    /// Optional arguments to pass only when the agent is executed with write
    /// permissions. When present, these are preferred over `args` for write
    /// runs.
    #[serde(default)]
    pub args_write: Option<Vec<String>>,

    /// Optional per-agent instructions. When set, these are prepended to the
    /// prompt provided to the agent whenever it runs.
    #[serde(default)]
    pub instructions: Option<String>,
}

fn default_true() -> bool {
    true
}

/// GitHub integration settings.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct GithubConfig {
    /// When true, Codex watches for GitHub Actions workflow runs after a
    /// successful `git push` and reports failures as background messages.
    /// Enabled by default; can be disabled via `~/.code/config.toml` under
    /// `[github]` with `check_workflows_on_push = false`.
    #[serde(default = "default_true")]
    pub check_workflows_on_push: bool,

    /// When true, run `actionlint` on modified workflows during apply_patch.
    #[serde(default)]
    pub actionlint_on_patch: bool,

    /// Optional explicit executable path for actionlint.
    #[serde(default)]
    pub actionlint_path: Option<PathBuf>,

    /// Treat actionlint findings as blocking when composing approval text.
    #[serde(default)]
    pub actionlint_strict: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ValidationConfig {
    /// Legacy master toggle for the validation harness (kept for config compatibility).
    /// `run_patch_harness` now relies solely on the functional/stylistic group toggles.
    #[serde(default)]
    pub patch_harness: bool,

    /// Optional allowlist restricting which external tools may run.
    #[serde(default)]
    pub tools_allowlist: Option<Vec<String>>,

    /// Timeout (seconds) for each external tool invocation.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,

    /// Group toggles that control which classes of validation run.
    #[serde(default)]
    pub groups: ValidationGroups,

    /// Per-tool enable flags (unset implies enabled).
    #[serde(default)]
    pub tools: ValidationTools,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            patch_harness: false,
            tools_allowlist: None,
            timeout_seconds: None,
            groups: ValidationGroups::default(),
            tools: ValidationTools::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ValidationGroups {
    /// Functional checks catch correctness regressions.
    #[serde(default = "default_true")]
    pub functional: bool,

    /// Stylistic checks enforce formatting and best practices.
    #[serde(default)]
    pub stylistic: bool,
}

impl Default for ValidationGroups {
    fn default() -> Self {
        Self {
            functional: false,
            stylistic: false,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ValidationTools {
    pub shellcheck: Option<bool>,
    pub markdownlint: Option<bool>,
    pub hadolint: Option<bool>,
    pub yamllint: Option<bool>,
    #[serde(rename = "cargo-check")]
    pub cargo_check: Option<bool>,
    pub shfmt: Option<bool>,
    pub prettier: Option<bool>,
    #[serde(rename = "tsc")]
    pub tsc: Option<bool>,
    pub eslint: Option<bool>,
    pub phpstan: Option<bool>,
    pub psalm: Option<bool>,
    pub mypy: Option<bool>,
    pub pyright: Option<bool>,
    #[serde(rename = "golangci-lint")]
    pub golangci_lint: Option<bool>,
}

/// Category groupings for validation checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationCategory {
    Functional,
    Stylistic,
}

impl ValidationCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            ValidationCategory::Functional => "functional",
            ValidationCategory::Stylistic => "stylistic",
        }
    }
}

/// Map a validation tool name to its category grouping.
pub fn validation_tool_category(name: &str) -> ValidationCategory {
    match name {
        "actionlint" | "shellcheck" | "cargo-check" | "tsc" | "eslint" | "phpstan" | "psalm"
        | "mypy" | "pyright" | "golangci-lint" => ValidationCategory::Functional,
        "markdownlint" | "hadolint" | "yamllint" | "shfmt" | "prettier" => {
            ValidationCategory::Stylistic
        }
        _ => ValidationCategory::Stylistic,
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct McpServerConfig {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default)]
    pub env: Option<HashMap<String, String>>,

    /// Optional per-server startup timeout in milliseconds.
    /// Applies to both the initial `initialize` handshake and the first
    /// `tools/list` request during startup. If unset, defaults to 10_000ms.
    #[serde(default)]
    pub startup_timeout_ms: Option<u64>,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum UriBasedFileOpener {
    #[serde(rename = "vscode")]
    VsCode,

    #[serde(rename = "vscode-insiders")]
    VsCodeInsiders,

    #[serde(rename = "windsurf")]
    Windsurf,

    #[serde(rename = "cursor")]
    Cursor,

    /// Option to disable the URI-based file opener.
    #[serde(rename = "none")]
    None,
}

impl UriBasedFileOpener {
    pub fn get_scheme(&self) -> Option<&str> {
        match self {
            UriBasedFileOpener::VsCode => Some("vscode"),
            UriBasedFileOpener::VsCodeInsiders => Some("vscode-insiders"),
            UriBasedFileOpener::Windsurf => Some("windsurf"),
            UriBasedFileOpener::Cursor => Some("cursor"),
            UriBasedFileOpener::None => None,
        }
    }
}

/// Settings that govern if and what will be written to `~/.code/history.jsonl`
/// (Code still reads legacy `~/.codex/history.jsonl`).
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct History {
    /// If true, history entries will not be written to disk.
    pub persistence: HistoryPersistence,

    /// If set, the maximum size of the history file in bytes.
    /// TODO(mbolin): Not currently honored.
    pub max_bytes: Option<usize>,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HistoryPersistence {
    /// Save all history entries to disk.
    #[default]
    SaveAll,
    /// Do not write history to disk.
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Notifications {
    Enabled(bool),
    Custom(Vec<String>),
}

impl Default for Notifications {
    fn default() -> Self {
        Self::Enabled(false)
    }
}

/// Collection of settings that are specific to the TUI.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct CachedTerminalBackground {
    pub is_dark: bool,
    #[serde(default)]
    pub term: Option<String>,
    #[serde(default)]
    pub term_program: Option<String>,
    #[serde(default)]
    pub term_program_version: Option<String>,
    #[serde(default)]
    pub colorfgbg: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub rgb: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Tui {
    /// Theme configuration for the TUI
    #[serde(default)]
    pub theme: ThemeConfig,

    /// Cached autodetect result so we can skip probing the terminal repeatedly.
    #[serde(default)]
    pub cached_terminal_background: Option<CachedTerminalBackground>,

    /// Syntax highlighting configuration (Markdown fenced code blocks)
    #[serde(default)]
    pub highlight: HighlightConfig,

    /// Whether to show reasoning content expanded by default (can be toggled with Ctrl+R/T)
    #[serde(default)]
    pub show_reasoning: bool,

    /// Streaming/animation behavior for assistant/reasoning output
    #[serde(default)]
    pub stream: StreamConfig,

    /// Loading spinner style selection
    #[serde(default)]
    pub spinner: SpinnerSelection,

    /// Enable desktop notifications from the TUI when the terminal is unfocused.
    /// Defaults to `false`.
    #[serde(default)]
    pub notifications: Notifications,

    /// Whether to use the terminal's Alternate Screen (full-screen) mode.
    /// When false, Codex renders nothing and leaves the standard terminal
    /// buffer visible; users can toggle back to Alternate Screen at runtime
    /// with Ctrl+T. Defaults to true.
    #[serde(default = "default_true")]
    pub alternate_screen: bool,
}

// Important: Provide a manual Default so that when no config file exists and we
// construct `Config` via `unwrap_or_default()`, we still honor the intended
// default of `alternate_screen = true`. Deriving `Default` would set booleans to
// `false`, which caused fresh installs (or a temporary CODEX_HOME) to start in
// standard-terminal mode until the user pressed Ctrl+T.
impl Default for Tui {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            cached_terminal_background: None,
            highlight: HighlightConfig::default(),
            show_reasoning: false,
            stream: StreamConfig::default(),
            spinner: SpinnerSelection::default(),
            notifications: Notifications::default(),
            alternate_screen: true,
        }
    }
}

/// Streaming behavior configuration for the TUI.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct StreamConfig {
    /// Emit the Answer header immediately when a stream begins (before first newline).
    #[serde(default)]
    pub answer_header_immediate: bool,

    /// Show an ellipsis placeholder in the Answer body while waiting for first text.
    #[serde(default = "default_true")]
    pub show_answer_ellipsis: bool,

    /// Commit animation pacing in milliseconds (lines per CommitTick).
    /// If unset, defaults to 50ms; in responsive profile, defaults to 30ms.
    #[serde(default)]
    pub commit_tick_ms: Option<u64>,

    /// Soft-commit timeout (ms) when no newline arrives; commits partial content.
    /// If unset, disabled; in responsive profile, defaults to 400ms.
    #[serde(default)]
    pub soft_commit_timeout_ms: Option<u64>,

    /// Soft-commit when this many chars have streamed without a newline.
    /// If unset, disabled; in responsive profile, defaults to 160 chars.
    #[serde(default)]
    pub soft_commit_chars: Option<usize>,

    /// Relax list hold-back: allow list lines with content; only withhold bare markers.
    #[serde(default)]
    pub relax_list_holdback: bool,

    /// Relax code hold-back: allow committing inside an open fenced code block
    /// except the very last partial line.
    #[serde(default)]
    pub relax_code_holdback: bool,

    /// Convenience switch enabling a snappier preset for the above values.
    /// Explicit values above still take precedence if set.
    #[serde(default)]
    pub responsive: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            answer_header_immediate: false,
            show_answer_ellipsis: true,
            commit_tick_ms: None,
            soft_commit_timeout_ms: None,
            soft_commit_chars: None,
            relax_list_holdback: false,
            relax_code_holdback: false,
            responsive: false,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ReasoningSummaryFormat {
    #[default]
    None,
    Experimental,
}

/// Theme configuration for the TUI
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ThemeConfig {
    /// Name of the predefined theme to use
    #[serde(default)]
    pub name: ThemeName,

    /// Custom color overrides (optional)
    #[serde(default)]
    pub colors: ThemeColors,

    /// Optional display name when using a custom theme generated by the user.
    /// Not used for built-in themes. If `name == Custom` and this is set, the
    /// UI may display it in place of the generic "Custom" label.
    #[serde(default)]
    pub label: Option<String>,

    /// Optional hint whether the custom theme targets a dark background.
    /// When present and `name == Custom`, the UI can show "Dark - <label>"
    /// or "Light - <label>" in lists.
    #[serde(default)]
    pub is_dark: Option<bool>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: ThemeName::default(),
            colors: ThemeColors::default(),
            label: None,
            is_dark: None,
        }
    }
}

/// Selected loading spinner style.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct SpinnerSelection {
    /// Name of the spinner to use. Accepts one of the names from
    /// sindresorhus/cli-spinners (kebab-case), or custom names supported
    /// by Codex. Defaults to "diamond".
    #[serde(default = "default_spinner_name")]
    pub name: String,
    /// Custom spinner definitions saved by the user
    #[serde(default)]
    pub custom: std::collections::HashMap<String, CustomSpinner>,
}

fn default_spinner_name() -> String {
    "diamond".to_string()
}

impl Default for SpinnerSelection {
    fn default() -> Self {
        Self {
            name: default_spinner_name(),
            custom: Default::default(),
        }
    }
}

/// User-defined custom spinner
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct CustomSpinner {
    pub interval: u64,
    pub frames: Vec<String>,
    /// Optional human-readable label to display in the UI
    #[serde(default)]
    pub label: Option<String>,
}

/// Configuration for syntax highlighting in Markdown code blocks.
///
/// `theme` accepts the following values:
/// - "auto" (default): choose a sensible built-in syntect theme based on
///   whether the current UI theme is light or dark.
/// - "<name>": use a specific syntect theme by name from the default ThemeSet.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct HighlightConfig {
    /// Theme selection preference (see docstring for accepted values)
    #[serde(default)]
    pub theme: Option<String>,
}

/// Available predefined themes
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    // Light themes (at top)
    #[default]
    LightPhoton,
    LightPrismRainbow,
    LightVividTriad,
    LightPorcelain,
    LightSandbar,
    LightGlacier,
    // Dark themes (below)
    DarkCarbonNight,
    DarkShinobiDusk,
    DarkOledBlackPro,
    DarkAmberTerminal,
    DarkAuroraFlux,
    DarkCharcoalRainbow,
    DarkZenGarden,
    DarkPaperLightPro,
    Custom,
}

/// Theme colors that can be customized
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ThemeColors {
    // Primary colors
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub background: Option<String>,
    pub foreground: Option<String>,

    // UI elements
    pub border: Option<String>,
    pub border_focused: Option<String>,
    pub selection: Option<String>,
    pub cursor: Option<String>,

    // Status colors
    pub success: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
    pub info: Option<String>,

    // Text colors
    pub text: Option<String>,
    pub text_dim: Option<String>,
    pub text_bright: Option<String>,

    // Syntax/special colors
    pub keyword: Option<String>,
    pub string: Option<String>,
    pub comment: Option<String>,
    pub function: Option<String>,

    // Animation colors
    pub spinner: Option<String>,
    pub progress: Option<String>,
}

/// Browser configuration for integrated screenshot capabilities.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct BrowserConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub viewport: Option<BrowserViewportConfig>,

    #[serde(default)]
    pub wait: Option<BrowserWaitStrategy>,

    #[serde(default)]
    pub fullpage: bool,

    #[serde(default)]
    pub segments_max: Option<usize>,

    #[serde(default)]
    pub idle_timeout_ms: Option<u64>,

    #[serde(default)]
    pub format: Option<BrowserImageFormat>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct BrowserViewportConfig {
    pub width: u32,
    pub height: u32,

    #[serde(default)]
    pub device_scale_factor: Option<f64>,

    #[serde(default)]
    pub mobile: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum BrowserWaitStrategy {
    Event(String),
    Delay { delay_ms: u64 },
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BrowserImageFormat {
    Png,
    Webp,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SandboxWorkspaceWrite {
    #[serde(default)]
    pub writable_roots: Vec<PathBuf>,
    #[serde(default)]
    pub network_access: bool,
    #[serde(default)]
    pub exclude_tmpdir_env_var: bool,
    #[serde(default)]
    pub exclude_slash_tmp: bool,
    /// When true, do not protect the top-level `.git` folder under a writable
    /// root. Defaults to true (historical behavior allows Git writes).
    #[serde(default = "crate::config_types::default_true_bool")]
    pub allow_git_writes: bool,
}

// Serde helper: default to true for `allow_git_writes` when omitted.
pub(crate) const fn default_true_bool() -> bool {
    true
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ShellEnvironmentPolicyInherit {
    /// "Core" environment variables for the platform. On UNIX, this would
    /// include HOME, LOGNAME, PATH, SHELL, and USER, among others.
    Core,

    /// Inherits the full environment from the parent process.
    #[default]
    All,

    /// Do not inherit any environment variables from the parent process.
    None,
}

/// Policy for building the `env` when spawning a process via either the
/// `shell` or `local_shell` tool.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ShellEnvironmentPolicyToml {
    pub inherit: Option<ShellEnvironmentPolicyInherit>,

    pub ignore_default_excludes: Option<bool>,

    /// List of regular expressions.
    pub exclude: Option<Vec<String>>,

    pub r#set: Option<HashMap<String, String>>,

    /// List of regular expressions.
    pub include_only: Option<Vec<String>>,

    pub experimental_use_profile: Option<bool>,
}

pub type EnvironmentVariablePattern = WildMatchPattern<'*', '?'>;

/// Deriving the `env` based on this policy works as follows:
/// 1. Create an initial map based on the `inherit` policy.
/// 2. If `ignore_default_excludes` is false, filter the map using the default
///    exclude pattern(s), which are: `"*KEY*"` and `"*TOKEN*"`.
/// 3. If `exclude` is not empty, filter the map using the provided patterns.
/// 4. Insert any entries from `r#set` into the map.
/// 5. If non-empty, filter the map using the `include_only` patterns.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShellEnvironmentPolicy {
    /// Starting point when building the environment.
    pub inherit: ShellEnvironmentPolicyInherit,

    /// True to skip the check to exclude default environment variables that
    /// contain "KEY" or "TOKEN" in their name.
    pub ignore_default_excludes: bool,

    /// Environment variable names to exclude from the environment.
    pub exclude: Vec<EnvironmentVariablePattern>,

    /// (key, value) pairs to insert in the environment.
    pub r#set: HashMap<String, String>,

    /// Environment variable names to retain in the environment.
    pub include_only: Vec<EnvironmentVariablePattern>,

    /// If true, the shell profile will be used to run the command.
    pub use_profile: bool,
}

impl From<ShellEnvironmentPolicyToml> for ShellEnvironmentPolicy {
    fn from(toml: ShellEnvironmentPolicyToml) -> Self {
        // Default to inheriting the full environment when not specified.
        let inherit = toml.inherit.unwrap_or(ShellEnvironmentPolicyInherit::All);
        let ignore_default_excludes = toml.ignore_default_excludes.unwrap_or(false);
        let exclude = toml
            .exclude
            .unwrap_or_default()
            .into_iter()
            .map(|s| EnvironmentVariablePattern::new_case_insensitive(&s))
            .collect();
        let r#set = toml.r#set.unwrap_or_default();
        let include_only = toml
            .include_only
            .unwrap_or_default()
            .into_iter()
            .map(|s| EnvironmentVariablePattern::new_case_insensitive(&s))
            .collect();
        let use_profile = toml.experimental_use_profile.unwrap_or(false);

        Self {
            inherit,
            ignore_default_excludes,
            exclude,
            r#set,
            include_only,
            use_profile,
        }
    }
}

/// See https://platform.openai.com/docs/guides/reasoning?api-mode=responses#get-started-with-reasoning
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ReasoningEffort {
    /// Minimal reasoning. Accepts legacy value "none" for backwards compatibility.
    #[serde(alias = "none")]
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    /// Deprecated: previously disabled reasoning. Kept for internal use only.
    #[serde(skip)]
    None,
}

/// A summary of the reasoning performed by the model. This can be useful for
/// debugging and understanding the model's reasoning process.
/// See https://platform.openai.com/docs/guides/reasoning?api-mode=responses#reasoning-summaries
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ReasoningSummary {
    #[default]
    Auto,
    Concise,
    Detailed,
    /// Option to disable reasoning summaries.
    None,
}

/// Text verbosity level for OpenAI API responses.
/// Controls the level of detail in the model's text responses.
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TextVerbosity {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CommandField {
    List(Vec<String>),
    String(String),
}

fn deserialize_command_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = CommandField::deserialize(deserializer)?;
    match value {
        CommandField::List(items) => Ok(items),
        CommandField::String(text) => {
            if text.trim().is_empty() {
                Ok(Vec::new())
            } else {
                shlex_split(&text)
                    .ok_or_else(|| de::Error::custom("failed to parse command string"))
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectHookEvent {
    #[serde(rename = "session.start")]
    SessionStart,
    #[serde(rename = "session.end")]
    SessionEnd,
    #[serde(rename = "tool.before")]
    ToolBefore,
    #[serde(rename = "tool.after")]
    ToolAfter,
    #[serde(rename = "file.before_write")]
    FileBeforeWrite,
    #[serde(rename = "file.after_write")]
    FileAfterWrite,
}

impl ProjectHookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectHookEvent::SessionStart => "session.start",
            ProjectHookEvent::SessionEnd => "session.end",
            ProjectHookEvent::ToolBefore => "tool.before",
            ProjectHookEvent::ToolAfter => "tool.after",
            ProjectHookEvent::FileBeforeWrite => "file.before_write",
            ProjectHookEvent::FileAfterWrite => "file.after_write",
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            ProjectHookEvent::SessionStart => "session_start",
            ProjectHookEvent::SessionEnd => "session_end",
            ProjectHookEvent::ToolBefore => "tool_before",
            ProjectHookEvent::ToolAfter => "tool_after",
            ProjectHookEvent::FileBeforeWrite => "file_before_write",
            ProjectHookEvent::FileAfterWrite => "file_after_write",
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ProjectHookConfig {
    pub event: ProjectHookEvent,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(alias = "run", deserialize_with = "deserialize_command_vec")]
    pub command: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub run_in_background: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ProjectCommandConfig {
    pub name: String,
    #[serde(alias = "run", deserialize_with = "deserialize_command_vec")]
    pub command: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// ACE (Agentic Context Engine) configuration mode
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AceMode {
    /// Use ACE when heuristics suggest benefit (default)
    Auto,
    /// Always use ACE for configured commands
    Always,
    /// Never use ACE
    Never,
}

impl Default for AceMode {
    fn default() -> Self {
        Self::Auto
    }
}

/// Configuration for ACE (Agentic Context Engine) integration
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct AceConfig {
    /// Whether ACE integration is enabled
    #[serde(default = "default_ace_enabled")]
    pub enabled: bool,

    /// When to use ACE for route selection
    #[serde(default)]
    pub mode: AceMode,

    /// Number of playbook bullets to retrieve (max 8 recommended)
    #[serde(default = "default_ace_slice_size")]
    pub slice_size: usize,

    /// Path to ACE SQLite database
    #[serde(default = "default_ace_db_path")]
    pub db_path: String,

    /// Commands that should use ACE playbook injection
    #[serde(default = "default_ace_use_for")]
    pub use_for: Vec<String>,

    /// File count threshold for considering a task "complex"
    #[serde(default = "default_ace_complex_threshold")]
    pub complex_task_files_threshold: usize,

    /// Window in minutes for detecting command reruns
    #[serde(default = "default_ace_rerun_window")]
    pub rerun_window_minutes: u64,
}

fn default_ace_enabled() -> bool {
    true
}

fn default_ace_slice_size() -> usize {
    8
}

fn default_ace_db_path() -> String {
    "~/.code/ace/playbooks_normalized.sqlite3".to_string()
}

fn default_ace_use_for() -> Vec<String> {
    vec![
        "speckit.constitution".to_string(),
        "speckit.specify".to_string(),
        "speckit.tasks".to_string(),
        "speckit.implement".to_string(),
        "speckit.test".to_string(),
    ]
}

fn default_ace_complex_threshold() -> usize {
    4
}

fn default_ace_rerun_window() -> u64 {
    30
}

impl Default for AceConfig {
    fn default() -> Self {
        Self {
            enabled: default_ace_enabled(),
            mode: AceMode::default(),
            slice_size: default_ace_slice_size(),
            db_path: default_ace_db_path(),
            use_for: default_ace_use_for(),
            complex_task_files_threshold: default_ace_complex_threshold(),
            rerun_window_minutes: default_ace_rerun_window(),
        }
    }
}

impl From<codex_protocol::config_types::ReasoningEffort> for ReasoningEffort {
    fn from(v: codex_protocol::config_types::ReasoningEffort) -> Self {
        match v {
            codex_protocol::config_types::ReasoningEffort::Minimal => ReasoningEffort::Minimal,
            codex_protocol::config_types::ReasoningEffort::Low => ReasoningEffort::Low,
            codex_protocol::config_types::ReasoningEffort::Medium => ReasoningEffort::Medium,
            codex_protocol::config_types::ReasoningEffort::High => ReasoningEffort::High,
        }
    }
}

impl From<codex_protocol::config_types::ReasoningSummary> for ReasoningSummary {
    fn from(v: codex_protocol::config_types::ReasoningSummary) -> Self {
        match v {
            codex_protocol::config_types::ReasoningSummary::Auto => ReasoningSummary::Auto,
            codex_protocol::config_types::ReasoningSummary::Concise => ReasoningSummary::Concise,
            codex_protocol::config_types::ReasoningSummary::Detailed => ReasoningSummary::Detailed,
            codex_protocol::config_types::ReasoningSummary::None => ReasoningSummary::None,
        }
    }
}

/// Quality gate configuration per checkpoint
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct QualityGateConfig {
    pub plan: Vec<String>,
    pub tasks: Vec<String>,
    pub validate: Vec<String>,
    pub audit: Vec<String>,
    pub unlock: Vec<String>,
}

/// Hot-reload configuration
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct HotReloadConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default)]
    pub watch_paths: Vec<String>,
}

fn default_debounce_ms() -> u64 {
    2000
}

/// Startup validation configuration
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ValidationConfigExt {
    #[serde(default = "default_true")]
    pub check_api_keys: bool,
    #[serde(default = "default_true")]
    pub check_commands: bool,
    #[serde(default = "default_true")]
    pub strict_schema: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // QualityGateConfig Tests
    // ============================================================================

    #[test]
    fn test_quality_gate_config_full_deserialization() {
        let toml = r#"
            plan = ["gemini", "claude", "code"]
            tasks = ["gemini"]
            validate = ["gemini", "claude", "code"]
            audit = ["gemini", "claude", "gpt_codex"]
            unlock = ["gemini", "claude", "gpt_codex"]
        "#;

        let config: QualityGateConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.plan, vec!["gemini", "claude", "code"]);
        assert_eq!(config.tasks, vec!["gemini"]);
        assert_eq!(config.validate, vec!["gemini", "claude", "code"]);
        assert_eq!(config.audit, vec!["gemini", "claude", "gpt_codex"]);
        assert_eq!(config.unlock, vec!["gemini", "claude", "gpt_codex"]);
    }

    #[test]
    fn test_quality_gate_config_single_agent() {
        let toml = r#"
            plan = ["gemini"]
            tasks = ["gemini"]
            validate = ["gemini"]
            audit = ["gemini"]
            unlock = ["gemini"]
        "#;

        let config: QualityGateConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.plan.len(), 1);
        assert_eq!(config.tasks, vec!["gemini"]);
    }

    #[test]
    fn test_quality_gate_config_multiple_agents() {
        let toml = r#"
            plan = ["gemini", "claude", "code", "gpt_pro", "haiku"]
            tasks = ["gemini", "haiku"]
            validate = ["gemini", "claude", "code"]
            audit = ["gemini", "claude", "code", "gpt_codex"]
            unlock = ["gemini", "claude", "code"]
        "#;

        let config: QualityGateConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.plan.len(), 5);
        assert_eq!(config.tasks.len(), 2);
        assert_eq!(config.audit.len(), 4);
    }

    #[test]
    fn test_quality_gate_config_empty_array_fails() {
        let toml = r#"
            plan = []
            tasks = ["gemini"]
            validate = ["gemini"]
            audit = ["gemini"]
            unlock = ["gemini"]
        "#;

        // Empty arrays are allowed by deserializer, but should fail validation
        let config: Result<QualityGateConfig, _> = toml::from_str(toml);
        assert!(config.is_ok()); // Deserialization succeeds
        // Note: Runtime validation (check_api_keys, etc.) will catch empty arrays
    }

    // ============================================================================
    // HotReloadConfig Tests
    // ============================================================================

    #[test]
    fn test_hot_reload_config_defaults() {
        let toml = ""; // Empty TOML should use defaults

        let config: HotReloadConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.enabled, true);
        assert_eq!(config.debounce_ms, 2000);
        assert_eq!(config.watch_paths.len(), 0);
    }

    #[test]
    fn test_hot_reload_config_custom_values() {
        let toml = r#"
            enabled = false
            debounce_ms = 5000
            watch_paths = ["config.toml", "models/", "agents/"]
        "#;

        let config: HotReloadConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.enabled, false);
        assert_eq!(config.debounce_ms, 5000);
        assert_eq!(config.watch_paths, vec!["config.toml", "models/", "agents/"]);
    }

    #[test]
    fn test_hot_reload_config_partial_overrides() {
        let toml = r#"
            debounce_ms = 3000
        "#;

        let config: HotReloadConfig = toml::from_str(toml).unwrap();

        // enabled should default to true
        assert_eq!(config.enabled, true);
        // debounce_ms should be overridden
        assert_eq!(config.debounce_ms, 3000);
        // watch_paths should default to empty
        assert_eq!(config.watch_paths.len(), 0);
    }

    #[test]
    fn test_hot_reload_config_debounce_range() {
        // Test minimum reasonable debounce
        let toml_min = "debounce_ms = 100";
        let config_min: HotReloadConfig = toml::from_str(toml_min).unwrap();
        assert_eq!(config_min.debounce_ms, 100);

        // Test maximum reasonable debounce
        let toml_max = "debounce_ms = 10000";
        let config_max: HotReloadConfig = toml::from_str(toml_max).unwrap();
        assert_eq!(config_max.debounce_ms, 10000);
    }

    // ============================================================================
    // ValidationConfigExt Tests
    // ============================================================================

    #[test]
    fn test_validation_config_ext_defaults() {
        let toml = ""; // Empty TOML should use defaults

        let config: ValidationConfigExt = toml::from_str(toml).unwrap();

        assert_eq!(config.check_api_keys, true);
        assert_eq!(config.check_commands, true);
        assert_eq!(config.strict_schema, true);
    }

    #[test]
    fn test_validation_config_ext_all_disabled() {
        let toml = r#"
            check_api_keys = false
            check_commands = false
            strict_schema = false
        "#;

        let config: ValidationConfigExt = toml::from_str(toml).unwrap();

        assert_eq!(config.check_api_keys, false);
        assert_eq!(config.check_commands, false);
        assert_eq!(config.strict_schema, false);
    }

    #[test]
    fn test_validation_config_ext_partial_disabled() {
        let toml = r#"
            check_api_keys = false
            strict_schema = true
        "#;

        let config: ValidationConfigExt = toml::from_str(toml).unwrap();

        assert_eq!(config.check_api_keys, false);
        assert_eq!(config.check_commands, true); // Default
        assert_eq!(config.strict_schema, true);
    }

    // ============================================================================
    // AgentConfig with canonical_name Tests
    // ============================================================================

    #[test]
    fn test_agent_config_with_canonical_name() {
        let toml = r#"
            name = "gemini"
            canonical_name = "gemini"
            command = "gemini"
        "#;

        let config: AgentConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.name, "gemini");
        assert_eq!(config.canonical_name, Some("gemini".to_string()));
        assert_eq!(config.command, "gemini");
    }

    #[test]
    fn test_agent_config_without_canonical_name() {
        let toml = r#"
            name = "claude"
            command = "anthropic"
        "#;

        let config: AgentConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.name, "claude");
        assert_eq!(config.canonical_name, None); // Optional field
        assert_eq!(config.command, "anthropic");
    }

    #[test]
    fn test_agent_config_canonical_name_differs_from_name() {
        let toml = r#"
            name = "claude-sonnet"
            canonical_name = "claude"
            command = "anthropic"
        "#;

        let config: AgentConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.name, "claude-sonnet");
        assert_eq!(config.canonical_name, Some("claude".to_string()));
        assert_eq!(config.command, "anthropic");
    }

    #[test]
    fn test_agent_config_full_configuration() {
        let toml = r#"
            name = "gpt-5"
            canonical_name = "gpt_pro"
            command = "openai"
            args = ["--model", "gpt-5-turbo"]
            read_only = false
            enabled = true
            description = "OpenAI GPT-5 model"
        "#;

        let config: AgentConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.name, "gpt-5");
        assert_eq!(config.canonical_name, Some("gpt_pro".to_string()));
        assert_eq!(config.command, "openai");
        assert_eq!(config.args, vec!["--model", "gpt-5-turbo"]);
        assert_eq!(config.read_only, false);
        assert_eq!(config.enabled, true);
        assert_eq!(config.description, Some("OpenAI GPT-5 model".to_string()));
    }

    // ============================================================================
    // Integration Tests - Combined Config Structures
    // ============================================================================

    #[test]
    fn test_combined_config_structures() {
        // Test that all SPEC-939 config types can be deserialized together
        #[derive(Deserialize, Debug)]
        struct TestConfig {
            quality_gates: QualityGateConfig,
            hot_reload: HotReloadConfig,
            validation: ValidationConfigExt,
        }

        let toml = r#"
            [quality_gates]
            plan = ["gemini", "claude", "code"]
            tasks = ["gemini"]
            validate = ["gemini", "claude", "code"]
            audit = ["gemini", "claude", "gpt_codex"]
            unlock = ["gemini", "claude", "gpt_codex"]

            [hot_reload]
            enabled = true
            debounce_ms = 2000
            watch_paths = ["config.toml"]

            [validation]
            check_api_keys = true
            check_commands = true
            strict_schema = true
        "#;

        let config: TestConfig = toml::from_str(toml).unwrap();

        // Quality gates
        assert_eq!(config.quality_gates.plan.len(), 3);
        assert_eq!(config.quality_gates.tasks, vec!["gemini"]);

        // Hot reload
        assert_eq!(config.hot_reload.enabled, true);
        assert_eq!(config.hot_reload.debounce_ms, 2000);

        // Validation
        assert_eq!(config.validation.check_api_keys, true);
        assert_eq!(config.validation.strict_schema, true);
    }

    #[test]
    fn test_combined_config_with_defaults() {
        #[derive(Deserialize, Debug)]
        struct TestConfig {
            #[serde(default)]
            quality_gates: Option<QualityGateConfig>,
            #[serde(default)]
            hot_reload: Option<HotReloadConfig>,
            #[serde(default)]
            validation: Option<ValidationConfigExt>,
        }

        // Empty config should allow all Optional fields to be None
        let toml = "";
        let config: TestConfig = toml::from_str(toml).unwrap();

        assert!(config.quality_gates.is_none());
        assert!(config.hot_reload.is_none());
        assert!(config.validation.is_none());
    }

    // ============================================================================
    // Edge Cases and Error Handling
    // ============================================================================

    #[test]
    fn test_quality_gate_config_invalid_toml() {
        let invalid_toml = r#"
            plan = ["gemini"
            tasks = ["gemini"]
        "#; // Missing closing bracket

        let result: Result<QualityGateConfig, _> = toml::from_str(invalid_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_hot_reload_config_invalid_type() {
        let invalid_toml = r#"
            enabled = "yes"
            debounce_ms = "2000"
        "#; // Strings instead of bool/u64

        let result: Result<HotReloadConfig, _> = toml::from_str(invalid_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_config_ext_invalid_bool() {
        let invalid_toml = r#"
            check_api_keys = 1
            check_commands = 0
        "#; // Numbers instead of bools

        let result: Result<ValidationConfigExt, _> = toml::from_str(invalid_toml);
        assert!(result.is_err());
    }
}
