use clap::ArgAction;
use clap::Parser;
use codex_common::ApprovalModeCliArg;
use codex_common::CliConfigOverrides;
use std::path::PathBuf;

// =============================================================================
// SPEC-KIT-979: Memory Backend Selection
// =============================================================================

/// Memory backend selection for local-memory sunset phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum MemoryBackendArg {
    /// Local-memory daemon (default in Phase 0, deprecated in Phase 2+)
    #[value(name = "local-memory")]
    LocalMemory,
    /// Memvid capsule storage (default in Phase 1+)
    #[value(name = "memvid")]
    Memvid,
}

impl std::fmt::Display for MemoryBackendArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryBackendArg::LocalMemory => write!(f, "local-memory"),
            MemoryBackendArg::Memvid => write!(f, "memvid"),
        }
    }
}

/// Convert MemoryBackendArg to codex_stage0::config::MemoryBackend for phase enforcement.
impl From<MemoryBackendArg> for codex_stage0::config::MemoryBackend {
    fn from(arg: MemoryBackendArg) -> Self {
        match arg {
            MemoryBackendArg::LocalMemory => codex_stage0::config::MemoryBackend::LocalMemory,
            MemoryBackendArg::Memvid => codex_stage0::config::MemoryBackend::Memvid,
        }
    }
}

#[derive(Parser, Debug, Default)]
#[command(version)]
pub struct Cli {
    /// Optional user prompt to start the session.
    #[arg(value_name = "PROMPT")]
    pub prompt: Option<String>,

    /// Optional image(s) to attach to the initial prompt.
    #[arg(
        long = "image",
        short = 'i',
        value_name = "FILE",
        value_delimiter = ','
    )]
    pub images: Vec<PathBuf>,

    /// Model the agent should use.
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Convenience flag to select the local open source model provider.
    /// Equivalent to -c model_provider=oss; verifies a local Ollama server is
    /// running.
    #[arg(long = "oss", default_value_t = false)]
    pub oss: bool,

    /// Configuration profile from config.toml to specify default options.
    #[arg(long = "profile", short = 'p')]
    pub config_profile: Option<String>,

    /// Select the sandbox policy to use when executing model-generated shell
    /// commands.
    #[arg(long = "sandbox", short = 's')]
    pub sandbox_mode: Option<codex_common::SandboxModeCliArg>,

    /// Configure when the model requires human approval before executing a command.
    #[arg(long = "ask-for-approval", short = 'a')]
    pub approval_policy: Option<ApprovalModeCliArg>,

    /// Convenience alias for low-friction sandboxed automatic execution (-a on-failure, --sandbox workspace-write).
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    /// Skip all confirmation prompts and execute commands without sandboxing.
    /// EXTREMELY DANGEROUS. Intended solely for running in environments that are externally sandboxed.
    #[arg(
        long = "dangerously-bypass-approvals-and-sandbox",
        alias = "yolo",
        default_value_t = false,
        conflicts_with_all = ["approval_policy", "full_auto"]
    )]
    pub dangerously_bypass_approvals_and_sandbox: bool,

    /// Tell the agent to use the specified directory as its working root.
    #[clap(long = "cd", short = 'C', value_name = "DIR")]
    pub cwd: Option<PathBuf>,

    /// Enable web search support. Enabled by default; use --no-search to disable.
    #[arg(long = "search", action = ArgAction::SetTrue)]
    pub enable_web_search: bool,

    /// Disable web search support explicitly.
    #[arg(long = "no-search", action = ArgAction::SetTrue, hide = true)]
    pub disable_web_search: bool,

    /// Effective web search toggle after applying flags.
    #[clap(skip)]
    pub web_search: bool,

    /// Enable debug logging of all LLM requests and responses to files.
    #[clap(long = "debug", short = 'd', default_value_t = false)]
    pub debug: bool,

    /// Show per-cell ordering overlays (request index, order key, window/position) to debug
    /// event ordering. Off by default.
    #[arg(long = "order", default_value_t = false)]
    pub order: bool,

    /// Enable lightweight in-app timing and print a summary report on exit.
    /// This records render/measurement hotspots while the UI runs and writes a
    /// short report to stderr when the program exits.
    #[arg(long = "timing", default_value_t = false)]
    pub timing: bool,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// Start in resume picker mode when true (used by `code resume`).
    #[clap(skip)]
    pub resume_picker: bool,

    /// Resume the most recent session automatically when true.
    #[clap(skip)]
    pub resume_last: bool,

    /// Resume a specific session id when provided.
    #[clap(skip)]
    pub resume_session_id: Option<String>,

    /// Initial slash command to execute after TUI starts (for automation).
    /// Example: --initial-command "/speckit.auto SPEC-KIT-900"
    #[arg(long = "initial-command")]
    pub initial_command: Option<String>,

    /// Exit after initial command completes (for automation).
    #[arg(long = "exit-on-complete", requires = "initial_command")]
    pub exit_on_complete: bool,

    // =========================================================================
    // SPEC-KIT-979: Local-Memory Sunset Flags
    // =========================================================================
    /// Memory backend selection: local-memory (Phase 0 default) or memvid.
    /// Phase 2+: local-memory requires --force-deprecated.
    #[arg(long = "memory-backend", value_name = "BACKEND")]
    pub memory_backend: Option<MemoryBackendArg>,

    /// Run A/B evaluation harness comparing local-memory vs memvid.
    /// Headless mode: runs evaluation and exits with appropriate exit code.
    #[arg(long = "eval-ab", default_value_t = false)]
    pub eval_ab: bool,

    /// Run capsule diagnostics (existence, lock status, integrity, version).
    /// Headless mode: runs diagnostics and exits with appropriate exit code.
    #[arg(long = "capsule-doctor", default_value_t = false)]
    pub capsule_doctor: bool,

    /// Force use of deprecated backend (required for local-memory in Phase 2+).
    /// In Phase 0, this flag is a no-op with an info message.
    #[arg(long = "force-deprecated", default_value_t = false)]
    pub force_deprecated: bool,

    /// Output JSON instead of human-readable format for --eval-ab and --capsule-doctor.
    #[arg(long = "json", default_value_t = false)]
    pub json_output: bool,

    /// Output directory for --eval-ab reports (default: .speckit/eval/).
    #[arg(long = "output-dir", value_name = "DIR", requires = "eval_ab")]
    pub output_dir: Option<PathBuf>,
}

impl Cli {
    pub fn finalize_defaults(&mut self) {
        self.web_search = if self.disable_web_search {
            false
        } else if self.enable_web_search {
            true
        } else {
            true
        };
    }
}
