//! Compatibility stubs for upstream-only features.
//!
//! This module provides stub implementations for features that exist in upstream
//! but are not available in the local fork. See docs/adr/ADR-001-tui2-local-api-adaptation.md
//! for the architectural decision behind this approach.
//!
//! # Upstream Features Stubbed
//!
//! - OSS provider management (`codex_common::oss`)
//! - Feature flags (`codex_core::features`)
//! - Terminal info detection (`codex_core::terminal`)
//! - Various protocol events and types

// Allow dead code - this module contains stubs for future implementation
#![allow(dead_code)]

use codex_core::config::Config;
use codex_core::protocol::SandboxPolicy;

/// Convert protocol ReasoningEffort to core ReasoningEffort
pub fn convert_reasoning_effort(
    effort: codex_protocol::openai_models::ReasoningEffort,
) -> codex_core::config_types::ReasoningEffort {
    use codex_protocol::openai_models::ReasoningEffort as ProtocolEffort;
    use codex_core::config_types::ReasoningEffort as CoreEffort;
    match effort {
        ProtocolEffort::High | ProtocolEffort::XHigh => CoreEffort::High,
        ProtocolEffort::Medium => CoreEffort::Medium,
        ProtocolEffort::Low => CoreEffort::Low,
        ProtocolEffort::Minimal | ProtocolEffort::None => CoreEffort::Minimal,
    }
}

/// Convert core ReasoningEffort to protocol ReasoningEffort
pub fn convert_reasoning_effort_to_protocol(
    effort: codex_core::config_types::ReasoningEffort,
) -> codex_protocol::openai_models::ReasoningEffort {
    use codex_protocol::openai_models::ReasoningEffort as ProtocolEffort;
    use codex_core::config_types::ReasoningEffort as CoreEffort;
    match effort {
        CoreEffort::High => ProtocolEffort::High,
        CoreEffort::Medium => ProtocolEffort::Medium,
        CoreEffort::Low => ProtocolEffort::Low,
        CoreEffort::Minimal | CoreEffort::None => ProtocolEffort::Minimal,
    }
}

/// Stub for `codex_core::INTERACTIVE_SESSION_SOURCES`
pub const INTERACTIVE_SESSION_SOURCES: &[&str] = &["codex_cli", "codex_tui"];

/// Stub for `codex_protocol::custom_prompts::PROMPTS_CMD_PREFIX`
pub const PROMPTS_CMD_PREFIX: &str = "/";

/// Stub for `codex_core::project_doc::DEFAULT_PROJECT_DOC_FILENAME`
pub const DEFAULT_PROJECT_DOC_FILENAME: &str = "AGENTS.md";

/// Stub OSS provider IDs
pub const OLLAMA_OSS_PROVIDER_ID: &str = "ollama";
pub const LMSTUDIO_OSS_PROVIDER_ID: &str = "lmstudio";

/// Stub default ports
pub const DEFAULT_OLLAMA_PORT: u16 = 11434;
pub const DEFAULT_LMSTUDIO_PORT: u16 = 1234;

/// Stub OSS module
pub mod oss {
    use codex_core::config::Config;

    /// Stub - always returns Ok since OSS is not supported locally
    pub async fn ensure_oss_provider_ready(
        _provider_id: &str,
        _config: &Config,
    ) -> std::io::Result<()> {
        Ok(())
    }

    /// Stub - returns None since OSS is not supported locally
    pub fn get_default_model_for_oss_provider(_provider_id: &str) -> Option<&'static str> {
        None
    }
}

/// Stub terminal module
pub mod terminal {
    /// Minimal terminal info struct
    #[derive(Debug, Clone)]
    pub struct TerminalInfo {
        pub name: TerminalName,
    }

    impl Default for TerminalInfo {
        fn default() -> Self {
            Self {
                name: TerminalName::Unknown,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum TerminalName {
        Unknown,
        Alacritty,
        AppleTerminal,
        Ghostty,
        Iterm2,
        Kitty,
        VsCode,
        WarpTerminal,
        WezTerm,
    }

    /// Stub - returns default terminal info
    pub fn terminal_info() -> TerminalInfo {
        TerminalInfo::default()
    }
}

/// Stub features module
pub mod features {
    use std::collections::HashSet;

    /// Feature enumeration
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Feature {
        ApplyPatchAmendment,
        Elicitation,
        ExecPolicy,
        WindowsSandbox,
    }

    impl Feature {
        pub fn key(&self) -> &'static str {
            match self {
                Feature::ApplyPatchAmendment => "apply_patch_amendment",
                Feature::Elicitation => "elicitation",
                Feature::ExecPolicy => "exec_policy",
                Feature::WindowsSandbox => "windows_sandbox",
            }
        }
    }

    /// Feature flags container
    #[derive(Debug, Clone)]
    pub struct Features {
        enabled: HashSet<Feature>,
    }

    impl Features {
        /// Create with default features (all disabled)
        pub fn with_defaults() -> Self {
            Self {
                enabled: HashSet::new(),
            }
        }

        /// Check if a feature is enabled
        pub fn enabled(&self, feature: Feature) -> bool {
            self.enabled.contains(&feature)
        }

        /// Enable a feature
        pub fn enable(&mut self, feature: Feature) {
            self.enabled.insert(feature);
        }

        /// Disable a feature
        pub fn disable(&mut self, feature: Feature) {
            self.enabled.remove(&feature);
        }
    }

    /// Feature flag for apply patch amendments
    pub fn apply_patch_amendment_enabled() -> bool {
        false
    }

    /// Feature flag for elicitation
    pub fn elicitation_enabled() -> bool {
        false
    }
}

/// Stub auth functions
pub mod auth {
    use codex_core::config::Config;

    /// Stub - no login restrictions enforced locally
    pub async fn enforce_login_restrictions(_config: &Config) -> Result<(), String> {
        Ok(())
    }

    /// Stub - reads OpenAI API key from environment
    pub fn read_openai_api_key_from_env() -> Option<String> {
        std::env::var("OPENAI_API_KEY").ok()
    }

    /// Stub auth credentials store mode
    #[derive(Debug, Clone, Copy, Default)]
    pub enum AuthCredentialsStoreMode {
        #[default]
        Keychain,
    }

    /// Stub auth mode
    #[derive(Debug, Clone, Copy, Default)]
    pub enum AuthMode {
        #[default]
        ApiKey,
        OAuth,
        Session,
    }
}

/// Stub config functions
pub mod config {
    
    
    use std::path::Path;

    /// Stub - returns None since OSS provider resolution is not supported
    pub fn resolve_oss_provider(
        _provider: Option<&str>,
        _config_toml: &codex_core::config::ConfigToml,
        _profile: Option<String>,
    ) -> Option<String> {
        None
    }

    /// Stub - OSS provider setting not supported locally
    pub fn set_default_oss_provider(_codex_home: &Path, _provider: &str) -> std::io::Result<()> {
        Ok(())
    }

    /// Stub - project trust level setting not supported locally
    pub fn set_project_trust_level(
        _codex_home: &Path,
        _path: &Path,
        _level: codex_protocol::config_types::TrustLevel,
    ) -> std::io::Result<()> {
        Ok(())
    }

    /// Stub constraint result
    #[derive(Debug, Clone)]
    pub enum ConstraintResult<T = ()> {
        Ok(T),
        Warning(String),
        Error(String),
    }

    impl<T> ConstraintResult<T> {
        pub fn ok(value: T) -> Self {
            Self::Ok(value)
        }

        pub fn is_ok(&self) -> bool {
            matches!(self, Self::Ok(_))
        }
    }

    /// Stub config edit module
    pub mod edit {
        use std::path::{Path, PathBuf};

        /// Stub ConfigEditsBuilder - operations are no-ops
        #[derive(Debug, Clone, Default)]
        pub struct ConfigEditsBuilder {
            changes: Vec<String>,
        }

        impl ConfigEditsBuilder {
            pub fn new(_codex_home: &Path) -> Self {
                Self::default()
            }

            /// Constructor from codex_home path (stub, ignores path)
            pub fn from_codex_home(_codex_home: &Path) -> std::io::Result<Self> {
                Ok(Self::default())
            }

            pub fn with_profile(&mut self, _profile: Option<&str>) -> &mut Self {
                self
            }

            pub fn set_feature_enabled(&mut self, _feature: &str, _enabled: bool) -> &mut Self {
                self
            }

            pub fn set_model(&mut self, _model: Option<&str>, _effort: Option<&str>) -> &mut Self {
                self
            }

            pub fn set_hide_full_access_warning(&mut self, _hide: bool) -> &mut Self {
                self
            }

            pub fn set_project_config_value(&mut self, _key: &str, _value: &str) -> &mut Self {
                self
            }

            pub fn set_model_migrations(&mut self, _migrations: &[String]) -> &mut Self {
                self
            }

            pub fn set_hide_world_writable_warning(&mut self, _hide: bool) -> &mut Self {
                self
            }

            pub fn set_hide_rate_limit_model_nudge(&mut self, _hide: bool) -> &mut Self {
                self
            }

            pub fn record_model_migration_seen(&mut self, _from_model: &str, _to_model: &str) -> &mut Self {
                self
            }

            pub async fn apply(&self) -> std::io::Result<()> {
                Ok(())
            }

            pub fn write(&self, _path: &PathBuf) -> std::io::Result<()> {
                Ok(())
            }

            pub fn save(&self) -> std::io::Result<()> {
                Ok(())
            }
        }
    }

    /// Stub types module
    pub mod types {
        // Empty - types are accessed directly from codex_core::config_types
    }
}

/// Stub protocol types that don't exist locally
pub mod protocol {
    

    // Re-export from codex_protocol for consistency
    pub use codex_protocol::protocol::RateLimitSnapshot;
    pub use codex_protocol::protocol::RateLimitWindow;

    /// Convert RateLimitSnapshotEvent from codex_core to RateLimitSnapshot from codex_protocol
    pub fn convert_rate_limit_snapshot(
        snapshot: &codex_core::protocol::RateLimitSnapshotEvent,
    ) -> RateLimitSnapshot {
        let primary = RateLimitWindow {
            used_percent: snapshot.primary_used_percent,
            window_minutes: Some(snapshot.primary_window_minutes),
            resets_in_seconds: snapshot.primary_reset_after_seconds,
        };
        let secondary = RateLimitWindow {
            used_percent: snapshot.secondary_used_percent,
            window_minutes: Some(snapshot.secondary_window_minutes),
            resets_in_seconds: snapshot.secondary_reset_after_seconds,
        };
        RateLimitSnapshot {
            primary: Some(primary),
            secondary: Some(secondary),
        }
    }

    /// Stub exec command source
    #[derive(Debug, Clone, Default)]
    pub enum ExecCommandSource {
        #[default]
        Model,
        User,
        UnifiedExecInteraction,
        UserShell,
    }

    /// Stub elicitation action
    #[derive(Debug, Clone)]
    pub enum ElicitationAction {
        Confirm,
        Cancel,
        Input(String),
        Accept,
        Decline,
    }

    /// Stub exec policy amendment
    #[derive(Debug, Clone)]
    pub struct ExecPolicyAmendment {
        pub command_pattern: String,
    }

    impl ExecPolicyAmendment {
        pub fn command(&self) -> &str {
            &self.command_pattern
        }
    }

    /// Stub approved exec policy amendment
    #[derive(Debug, Clone)]
    pub struct ApprovedExecpolicyAmendment {
        pub command_pattern: String,
    }

    impl ApprovedExecpolicyAmendment {
        pub fn new(command_pattern: String) -> Self {
            Self { command_pattern }
        }

        pub fn command(&self) -> &str {
            &self.command_pattern
        }
    }

    // Re-export from codex_protocol for consistency
    pub use codex_protocol::protocol::TurnAbortReason;

    // Stub event types that don't exist locally
    #[derive(Debug, Clone)]
    pub struct DeprecationNoticeEvent {
        pub message: String,
        pub summary: String,
        pub details: String,
    }

    #[derive(Debug, Clone)]
    pub struct StreamErrorEvent {
        pub error: String,
        pub message: String,
    }

    #[derive(Debug, Clone)]
    pub struct TerminalInteractionEvent {
        pub content: String,
    }

    #[derive(Debug, Clone)]
    pub struct McpStartupUpdateEvent {
        pub server_name: String,
        pub status: McpStartupStatus,
        pub server: String,
    }

    /// Represents a failed MCP server during startup
    #[derive(Debug, Clone)]
    pub struct FailedMcpServer {
        pub server: String,
        pub error: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub struct McpStartupCompleteEvent {
        pub servers_ready: usize,
        pub failed: Vec<FailedMcpServer>,
        pub cancelled: Vec<String>,
    }

    #[derive(Debug, Clone)]
    pub enum McpStartupStatus {
        Starting,
        Ready,
        Failed(String),
    }

    #[derive(Debug, Clone)]
    pub struct McpListToolsResponseEvent {
        pub tools: Vec<String>,
        pub server_name: Option<String>,
        pub resources: Vec<String>,
        pub resource_templates: Vec<String>,
        pub auth_statuses: std::collections::HashMap<String, String>,
    }

    #[derive(Debug, Clone)]
    pub struct ListCustomPromptsResponseEvent {
        pub prompts: Vec<String>,
        pub custom_prompts: Vec<String>,
    }

    #[derive(Debug, Clone)]
    pub struct ViewImageToolCallEvent {
        pub path: String,
    }

    #[derive(Debug, Clone)]
    pub struct WebSearchEndEvent {
        pub query: String,
    }

    #[derive(Debug, Clone)]
    pub struct WarningEvent {
        pub message: String,
    }
}

/// Extension trait for SandboxPolicy to provide upstream-compatible methods
pub trait SandboxPolicyExt {
    fn get(&self) -> &SandboxPolicy;
    fn set(&mut self, policy: SandboxPolicy);
}

impl SandboxPolicyExt for SandboxPolicy {
    fn get(&self) -> &SandboxPolicy {
        self
    }

    fn set(&mut self, policy: SandboxPolicy) {
        *self = policy;
    }
}

/// Extension trait for ModelFamily to provide upstream-compatible methods
pub trait ModelFamilyExt {
    fn get_model_slug(&self) -> &str;
    fn context_window(&self) -> Option<u64>;
}

impl ModelFamilyExt for codex_core::model_family::ModelFamily {
    fn get_model_slug(&self) -> &str {
        &self.slug
    }

    fn context_window(&self) -> Option<u64> {
        // Fork doesn't track context window size
        None
    }
}

/// Stub for format_env_display
/// Takes optional env map and optional env_vars, returns formatted string
pub fn format_env_display(
    _env: Option<&std::collections::HashMap<String, String>>,
    _env_vars: &Option<std::collections::HashMap<String, String>>,
) -> String {
    String::from("-")
}

/// Stub for parse_turn_item - returns None since parsing not available locally
pub fn parse_turn_item<T>(_item: &T) -> Option<ParsedTurnItem> {
    None
}

/// Stub parsed turn item type
#[derive(Debug, Clone)]
pub struct ParsedTurnItem {
    pub kind: String,
    pub content: String,
}

/// Stub path_utils module
pub mod path_utils {
    use std::path::Path;

    /// Stub - just returns the path as-is
    /// Returns Result to match expected signature from caller
    pub fn normalize_for_path_comparison(path: &Path) -> Result<std::path::PathBuf, std::io::Error> {
        Ok(path.to_path_buf())
    }
}

/// Stub bash module
pub mod bash {
    /// Stub - returns None since bash command extraction not available locally
    /// Signature matches upstream which takes &[String] and returns Option<(usize, &str)>
    pub fn extract_bash_command<'a>(_command: &'a [String]) -> Option<(usize, &'a str)> {
        None
    }
}

/// Stub parse_command module
pub mod parse_command {
    /// Stub - returns None since shell command extraction not available locally
    /// Signature matches upstream which takes &[String] and returns Option<(usize, &str)>
    pub fn extract_shell_command<'a>(_command: &'a [String]) -> Option<(usize, &'a str)> {
        None
    }
}

/// Stub env module
pub mod env {
    /// Stub - returns false since WSL detection not available locally
    pub fn is_wsl() -> bool {
        false
    }
}

/// Stub config_types that don't exist locally
pub mod config_types {
    use serde::{Deserialize, Serialize};

    /// Stub scroll input mode
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ScrollInputMode {
        Auto,
        Wheel,
        Line,
        Trackpad,
    }

    impl Default for ScrollInputMode {
        fn default() -> Self {
            Self::Auto
        }
    }

    /// Stub MCP server transport config
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum McpServerTransportConfig {
        Stdio {
            command: String,
            args: Vec<String>,
            env: Option<std::collections::HashMap<String, String>>,
            env_vars: Option<std::collections::HashMap<String, String>>,
            cwd: Option<String>,
        },
        StreamableHttp {
            url: String,
            headers: Option<std::collections::HashMap<String, String>>,
            http_headers: Option<std::collections::HashMap<String, String>>,
            env_http_headers: Option<std::collections::HashMap<String, String>>,
        },
    }
}

/// Stub skills module
pub mod skills {
    use std::path::PathBuf;

    /// Stub skill info
    #[derive(Debug, Clone)]
    pub struct SkillInfo {
        pub name: String,
        pub description: String,
    }

    /// Stub skill metadata
    #[derive(Debug, Clone)]
    pub struct SkillMetadata {
        pub name: String,
        pub description: Option<String>,
        pub short_description: Option<String>,
        pub path: Option<PathBuf>,
        pub scope: Option<String>,
    }

    impl SkillMetadata {
        pub fn new(name: String) -> Self {
            Self {
                name,
                description: None,
                short_description: None,
                path: None,
                scope: None,
            }
        }
    }

    /// Stub list_skills function
    pub fn list_skills() -> Vec<SkillMetadata> {
        Vec::new()
    }
}

/// Stub models_manager module
pub mod models_manager {
    use codex_core::config::Config;
    use codex_core::model_family::ModelFamily;
    use codex_protocol::openai_models::ModelPreset;

    /// Stub models manager
    #[derive(Debug, Clone, Copy)]
    pub struct ModelsManager;

    impl ModelsManager {
        /// Stub - returns the model name as-is
        pub async fn get_model(&self, model: &str, _config: &Config) -> String {
            model.to_string()
        }

        /// Stub - constructs a basic model family
        pub async fn construct_model_family(&self, model: &str, _config: &Config) -> ModelFamily {
            use codex_core::config_types::ReasoningSummaryFormat;
            ModelFamily {
                slug: model.to_string(),
                family: model.to_string(),
                needs_special_apply_patch_instructions: false,
                supports_reasoning_summaries: false,
                reasoning_summary_format: ReasoningSummaryFormat::None,
                uses_local_shell_tool: false,
                apply_patch_tool_type: None,
                base_instructions: String::new(),
            }
        }

        /// Stub - returns empty list of model presets (no model migration support)
        pub async fn list_models(&self, _config: &Config) -> Vec<ModelPreset> {
            Vec::new()
        }

        /// Stub - returns None for list of models (sync version)
        pub fn try_list_models(&self, _config: &Config) -> Option<Vec<ModelPreset>> {
            None
        }
    }
}

/// Stub review_prompts module
pub mod review_prompts {
    /// Stub review prompt
    pub fn get_review_prompt(_name: &str) -> Option<&'static str> {
        None
    }

    /// Stub user-facing hint for review targets
    pub fn user_facing_hint(_target: &str) -> String {
        String::from("Review")
    }
}

/// Stub notices config
#[derive(Debug, Clone, Default)]
pub struct NoticesConfig {
    pub hide_rate_limit_model_nudge: Option<bool>,
    pub hide_full_access_warning: Option<bool>,
    pub hide_world_writable_warning: Option<bool>,
    pub hide_gpt5_1_migration_prompt: Option<bool>,
    pub hide_gpt_5_1_codex_max_migration_prompt: Option<bool>,
    pub model_migrations: Vec<String>,
}

/// Extension trait for Config to provide upstream-compatible methods
pub trait ConfigExt {
    fn notices(&self) -> &NoticesConfig;
    fn animations(&self) -> bool;
    fn features(&self) -> crate::compat::features::Features;
    fn disable_paste_burst(&self) -> bool;
    fn tui_scroll_events_per_tick(&self) -> Option<u16>;
    fn tui_scroll_wheel_lines(&self) -> Option<u16>;
    fn tui_scroll_trackpad_lines(&self) -> Option<u16>;
    fn tui_scroll_trackpad_accel_events(&self) -> Option<u16>;
    fn tui_scroll_trackpad_accel_max(&self) -> Option<u16>;
    fn tui_scroll_mode(&self) -> Option<crate::compat::config_types::ScrollInputMode>;
    fn tui_scroll_wheel_tick_detect_max_ms(&self) -> Option<u64>;
    fn tui_scroll_wheel_like_max_duration_ms(&self) -> Option<u64>;
    fn tui_scroll_invert(&self) -> bool;
    fn cli_auth_credentials_store_mode(&self) -> crate::compat::auth::AuthCredentialsStoreMode;
    fn show_tooltips(&self) -> bool;
    fn forced_chatgpt_workspace_id(&self) -> Option<String>;
    fn forced_login_method(&self) -> Option<codex_protocol::config_types::ForcedLoginMethod>;
}

impl ConfigExt for Config {
    fn notices(&self) -> &NoticesConfig {
        // Return a reference to a leaked static instance
        // This is a workaround for the lifetime issue with extension traits
        use std::sync::OnceLock;
        static NOTICES: OnceLock<NoticesConfig> = OnceLock::new();
        NOTICES.get_or_init(|| NoticesConfig::default())
    }

    fn animations(&self) -> bool {
        true
    }

    fn features(&self) -> crate::compat::features::Features {
        crate::compat::features::Features::with_defaults()
    }

    fn disable_paste_burst(&self) -> bool {
        false
    }

    fn tui_scroll_events_per_tick(&self) -> Option<u16> {
        None // Use terminal-detected default
    }

    fn tui_scroll_wheel_lines(&self) -> Option<u16> {
        None // Use terminal-detected default
    }

    fn tui_scroll_trackpad_lines(&self) -> Option<u16> {
        None // Use terminal-detected default
    }

    fn tui_scroll_trackpad_accel_events(&self) -> Option<u16> {
        None // Use terminal-detected default
    }

    fn tui_scroll_trackpad_accel_max(&self) -> Option<u16> {
        None // Use terminal-detected default
    }

    fn tui_scroll_mode(&self) -> Option<crate::compat::config_types::ScrollInputMode> {
        None // Use terminal-detected default
    }

    fn tui_scroll_wheel_tick_detect_max_ms(&self) -> Option<u64> {
        None // Use terminal-detected default
    }

    fn tui_scroll_wheel_like_max_duration_ms(&self) -> Option<u64> {
        None // Use terminal-detected default
    }

    fn tui_scroll_invert(&self) -> bool {
        false
    }

    fn cli_auth_credentials_store_mode(&self) -> crate::compat::auth::AuthCredentialsStoreMode {
        crate::compat::auth::AuthCredentialsStoreMode::Keychain
    }

    fn show_tooltips(&self) -> bool {
        true
    }

    fn forced_chatgpt_workspace_id(&self) -> Option<String> {
        None
    }

    fn forced_login_method(&self) -> Option<codex_protocol::config_types::ForcedLoginMethod> {
        None
    }
}

/// Extension trait for ConversationManager to provide upstream-compatible methods
pub trait ConversationManagerExt {
    fn get_models_manager(&self) -> std::sync::Arc<models_manager::ModelsManager>;
}

impl ConversationManagerExt for codex_core::ConversationManager {
    fn get_models_manager(&self) -> std::sync::Arc<models_manager::ModelsManager> {
        std::sync::Arc::new(models_manager::ModelsManager)
    }
}

/// Extension trait for ExecCommandBeginEvent to provide upstream-compatible fields
pub trait ExecCommandBeginEventExt {
    fn source(&self) -> protocol::ExecCommandSource;
    fn interaction_input(&self) -> Option<String>;
}

impl ExecCommandBeginEventExt for codex_core::protocol::ExecCommandBeginEvent {
    fn source(&self) -> protocol::ExecCommandSource {
        protocol::ExecCommandSource::Model
    }

    fn interaction_input(&self) -> Option<String> {
        None
    }
}

/// Extension trait for ExecCommandEndEvent to provide upstream-compatible fields
pub trait ExecCommandEndEventExt {
    fn source(&self) -> protocol::ExecCommandSource;
    fn interaction_input(&self) -> Option<String>;
    fn command(&self) -> Vec<String>;
    fn parsed_cmd(&self) -> Vec<codex_core::parse_command::ParsedCommand>;
    fn formatted_output(&self) -> Option<String>;
    fn aggregated_output(&self) -> Option<String>;
}

impl ExecCommandEndEventExt for codex_core::protocol::ExecCommandEndEvent {
    fn source(&self) -> protocol::ExecCommandSource {
        protocol::ExecCommandSource::Model
    }

    fn interaction_input(&self) -> Option<String> {
        None
    }

    fn command(&self) -> Vec<String> {
        Vec::new()
    }

    fn parsed_cmd(&self) -> Vec<codex_core::parse_command::ParsedCommand> {
        Vec::new()
    }

    fn formatted_output(&self) -> Option<String> {
        Some(format!("{}\n{}", self.stdout, self.stderr))
    }

    fn aggregated_output(&self) -> Option<String> {
        Some(format!("{}\n{}", self.stdout, self.stderr))
    }
}

/// Extension trait for SessionConfiguredEvent to provide upstream-compatible fields
pub trait SessionConfiguredEventExt {
    fn initial_messages(&self) -> Vec<String>;
}

impl SessionConfiguredEventExt for codex_core::protocol::SessionConfiguredEvent {
    fn initial_messages(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Extension trait for ExecApprovalRequestEvent to provide upstream-compatible fields
pub trait ExecApprovalRequestEventExt {
    fn proposed_execpolicy_amendment(&self) -> Option<protocol::ExecPolicyAmendment>;
}

impl ExecApprovalRequestEventExt for codex_core::protocol::ExecApprovalRequestEvent {
    fn proposed_execpolicy_amendment(&self) -> Option<protocol::ExecPolicyAmendment> {
        None
    }
}
