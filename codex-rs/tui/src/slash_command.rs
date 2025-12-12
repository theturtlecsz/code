use std::sync::OnceLock;

use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::IntoStaticStr;

use crate::spec_prompts;
use crate::spec_prompts::SpecStage;
use thiserror::Error;

const BUILD_PROFILE: Option<&str> = option_env!("CODEX_PROFILE");

fn demo_command_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let profile_matches = |profile: &str| {
            let normalized = profile.trim().to_ascii_lowercase();
            normalized == "perf" || normalized.starts_with("dev")
        };

        if let Some(profile) = BUILD_PROFILE.or(option_env!("PROFILE"))
            && profile_matches(profile)
        {
            return true;
        }

        if let Ok(exe_path) = std::env::current_exe() {
            let path = exe_path.to_string_lossy().to_ascii_lowercase();
            if path.contains("target/dev-fast/")
                || path.contains("target/dev/")
                || path.contains("target/perf/")
            {
                return true;
            }
        }

        cfg!(debug_assertions)
    })
}

fn pro_command_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let profile_matches = |profile: &str| {
            let normalized = profile.trim().to_ascii_lowercase();
            normalized.starts_with("dev") || normalized == "pref" || normalized == "perf"
        };

        if let Some(profile) = BUILD_PROFILE.or(option_env!("PROFILE"))
            && profile_matches(profile)
        {
            return true;
        }

        if let Ok(exe_path) = std::env::current_exe() {
            let path = exe_path.to_string_lossy().to_ascii_lowercase();
            if path.contains("target/dev-fast/")
                || path.contains("target/dev/")
                || path.contains("target/pref/")
                || path.contains("target/perf/")
            {
                return true;
            }
        }

        false
    })
}

/// Commands that can be invoked by starting a message with a leading slash.
///
/// IMPORTANT: When adding or changing slash commands, also update
/// `docs/slash-commands.md` at the repo root so users can discover them easily.
/// This enum is the source of truth for the list and ordering shown in the UI.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr, IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum SlashCommand {
    // DO NOT ALPHA-SORT! Enum order is presentation order in the popup, so
    // more frequently used commands should be listed first.
    Browser,
    Chrome,
    New,
    Init,
    Compact,
    Undo,
    Review,
    Diff,
    Mention,
    Cmd,
    Status,
    Limits,
    Sessions,
    #[strum(serialize = "update", serialize = "upgrade")]
    Update,
    Theme,
    Model,
    Reasoning,
    Verbosity,
    Prompts,
    Perf,
    Demo,
    Agents,
    Pro,
    Branch,
    Merge,
    Github,
    Validation,
    Mcp,
    Resume,
    Login,
    // P6-SYNC Phase 5: Device code OAuth management
    Auth,
    // SPEC-KIT-963: Upstream /plan, /solve, /code removed.
    // This fork uses /speckit.* namespace exclusively.
    // === FORK-SPECIFIC: spec-kit slash commands ===
    // Upstream: Does not have spec-kit automation
    // Preserve: All spec-kit commands during rebases
    // Phase 3: Standardized /speckit.* namespace
    #[strum(serialize = "speckit.new")]
    SpecKitNew,
    #[strum(serialize = "speckit.specify")]
    SpecKitSpecify,
    #[strum(serialize = "speckit.clarify")]
    SpecKitClarify,
    #[strum(serialize = "speckit.analyze")]
    SpecKitAnalyze,
    #[strum(serialize = "speckit.checklist")]
    SpecKitChecklist,
    #[strum(serialize = "speckit.plan")]
    SpecKitPlan,
    #[strum(serialize = "speckit.tasks")]
    SpecKitTasks,
    #[strum(serialize = "speckit.implement")]
    SpecKitImplement,
    #[strum(serialize = "speckit.validate")]
    SpecKitValidate,
    #[strum(serialize = "speckit.audit")]
    SpecKitAudit,
    #[strum(serialize = "speckit.unlock")]
    SpecKitUnlock,
    #[strum(serialize = "speckit.auto")]
    SpecKitAuto,
    #[strum(serialize = "speckit.status")]
    SpecKitStatus,
    #[strum(serialize = "speckit.configure")]
    SpecKitConfigure,
    #[strum(serialize = "speckit.constitution")]
    SpecKitConstitution,
    #[strum(serialize = "speckit.ace-status")]
    SpecKitAceStatus,
    // SPEC-KIT-960: Project scaffolding
    #[strum(serialize = "speckit.project")]
    SpecKitProject,
    // Verification command
    #[strum(serialize = "speckit.verify")]
    SpecKitVerify,
    // SPEC-KIT-962: Template management commands
    #[strum(serialize = "speckit.install-templates")]
    SpecKitInstallTemplates,
    #[strum(serialize = "speckit.template-status")]
    SpecKitTemplateStatus,
    // Guardrail commands (Phase 3 Week 2)
    #[strum(serialize = "guardrail.plan")]
    GuardrailPlan,
    #[strum(serialize = "guardrail.tasks")]
    GuardrailTasks,
    #[strum(serialize = "guardrail.implement")]
    GuardrailImplement,
    #[strum(serialize = "guardrail.validate")]
    GuardrailValidate,
    #[strum(serialize = "guardrail.audit")]
    GuardrailAudit,
    #[strum(serialize = "guardrail.unlock")]
    GuardrailUnlock,
    #[strum(serialize = "guardrail.auto")]
    GuardrailAuto,
    // SPEC-KIT-902: Legacy /spec-* and /spec-ops-* removed. Use /speckit.* or /guardrail.*
    // Utility commands retained:
    #[strum(serialize = "spec-evidence-stats")]
    SpecEvidenceStats,
    #[strum(serialize = "spec-consensus")]
    SpecConsensus,
    #[strum(serialize = "spec-status")]
    SpecStatus,
    // === END FORK-SPECIFIC: spec-kit commands ===
    // P53-SYNC: Diagnostic feedback export
    Feedback,
    Logout,
    Quit,
    #[cfg(debug_assertions)]
    TestApproval,
}

impl SlashCommand {
    /// User-visible description shown in the popup.
    pub fn description(self) -> &'static str {
        match self {
            SlashCommand::Chrome => "connect to Chrome",
            SlashCommand::Browser => "open internal browser",
            SlashCommand::Resume => "resume a past session for this folder",
            // SPEC-KIT-963: /plan, /solve, /code removed - use /speckit.* commands
            SlashCommand::Reasoning => "change reasoning effort (minimal/low/medium/high)",
            SlashCommand::Verbosity => "change text verbosity (high/medium/low)",
            SlashCommand::New => "start a new chat during a conversation",
            SlashCommand::Init => "create an AGENTS.md file with instructions for Code",
            SlashCommand::Compact => "summarize conversation to prevent hitting the context limit",
            SlashCommand::Undo => "restore the workspace to the last Code snapshot",
            SlashCommand::Review => "review your changes for potential issues",
            SlashCommand::Quit => "exit Code",
            SlashCommand::Diff => "show git diff (including untracked files)",
            SlashCommand::Mention => "mention a file",
            SlashCommand::Cmd => "run a project command",
            SlashCommand::Status => "show current session configuration and token usage",
            SlashCommand::Limits => "visualize weekly and hourly rate limits",
            SlashCommand::Sessions => "list and manage active CLI sessions (Claude/Gemini)",
            SlashCommand::Update => "check for updates and optionally upgrade",
            SlashCommand::Theme => "switch between color themes",
            SlashCommand::Prompts => "show example prompts",
            SlashCommand::Model => "choose model & reasoning effort",
            SlashCommand::Agents => "create and configure agents",
            // SpecKit standardized commands
            SlashCommand::SpecKitNew => "create new SPEC (native, instant, $0)",
            SlashCommand::SpecKitSpecify => "generate PRD with multi-agent consensus",
            SlashCommand::SpecKitClarify => "detect ambiguities (native, <1s, $0)",
            SlashCommand::SpecKitAnalyze => "check consistency (native, <1s, $0)",
            SlashCommand::SpecKitChecklist => "score requirements (native, <1s, $0)",
            SlashCommand::SpecKitPlan => "create work breakdown with multi-agent consensus",
            SlashCommand::SpecKitTasks => "generate task list with validation mapping",
            SlashCommand::SpecKitImplement => "write code with multi-agent consensus",
            SlashCommand::SpecKitValidate => "run test strategy with validation",
            SlashCommand::SpecKitAudit => "compliance review with multi-agent",
            SlashCommand::SpecKitUnlock => "final approval for merge",
            SlashCommand::SpecKitAuto => "full pipeline (native coordinator)",
            SlashCommand::SpecKitStatus => "show progress dashboard (native)",
            SlashCommand::SpecKitConfigure => "configure pipeline stages (interactive modal)",
            SlashCommand::SpecKitConstitution => "extract ACE bullets (native)",
            SlashCommand::SpecKitAceStatus => "show ACE stats (native)",
            SlashCommand::SpecKitProject => {
                "scaffold new project with spec-kit support (native, $0)"
            }
            SlashCommand::SpecKitVerify => "verify spec implementation (native)",
            SlashCommand::SpecKitInstallTemplates => "install templates to project (native)",
            SlashCommand::SpecKitTemplateStatus => "show template resolution status (native)",
            // SPEC-KIT-902: Legacy /spec-* and /spec-ops-* removed
            // Utility commands retained:
            SlashCommand::SpecEvidenceStats => {
                "summarize guardrail/consensus evidence sizes (optional --spec)"
            }
            SlashCommand::SpecConsensus => {
                "check multi-agent consensus via local-memory (requires SPEC ID & stage)"
            }
            SlashCommand::SpecStatus => {
                "show comprehensive SPEC status (guardrails, consensus, agents)"
            }
            // Guardrail commands
            SlashCommand::GuardrailPlan => "run guardrail validation for plan stage",
            SlashCommand::GuardrailTasks => "run guardrail validation for tasks stage",
            SlashCommand::GuardrailImplement => "run guardrail validation for implement stage",
            SlashCommand::GuardrailValidate => "run guardrail validation for validate stage",
            SlashCommand::GuardrailAudit => "run guardrail validation for audit stage",
            SlashCommand::GuardrailUnlock => "run guardrail validation for unlock stage",
            SlashCommand::GuardrailAuto => "run full guardrail pipeline with telemetry",
            SlashCommand::Pro => "manage Pro mode (toggle/status/auto)",
            SlashCommand::Branch => {
                "work in an isolated /branch then /merge when done (great for parallel work)"
            }
            SlashCommand::Merge => "merge current worktree branch back to default",
            SlashCommand::Github => "GitHub Actions watcher (status/on/off)",
            SlashCommand::Validation => "control validation harness (status/on/off)",
            SlashCommand::Mcp => "manage MCP servers (status/on/off/add)",
            SlashCommand::Perf => "performance tracing (on/off/show/reset)",
            SlashCommand::Demo => "populate history with demo cells (dev/perf only)",
            SlashCommand::Login => "manage Code sign-ins (add/select/disconnect)",
            SlashCommand::Auth => "device code OAuth status (status/login/logout <provider>)",
            SlashCommand::Feedback => "export session logs for debugging/bug reports",
            SlashCommand::Logout => "log out of Code",
            #[cfg(debug_assertions)]
            SlashCommand::TestApproval => "test approval request",
        }
    }

    /// Command string without the leading '/'. Provided for compatibility with
    /// existing code that expects a method named `command()`.
    pub fn command(self) -> &'static str {
        self.into()
    }

    /// Returns true if this command should expand into a prompt for the LLM.
    /// SPEC-KIT-963: All prompt-expanding commands removed. Spec-kit uses registry.
    pub fn is_prompt_expanding(self) -> bool {
        false
    }

    /// Returns true if this command requires additional arguments after the command.
    /// SPEC-KIT-963: Upstream /plan, /solve, /code removed.
    pub fn requires_arguments(self) -> bool {
        matches!(
            self,
            // REMOVED: SpecKitNew, SpecKitClarify, SpecKitAnalyze, SpecKitChecklist (registry-only)
            SlashCommand::SpecKitSpecify
                | SlashCommand::SpecKitPlan
                | SlashCommand::SpecKitTasks
                | SlashCommand::SpecKitImplement
                | SlashCommand::SpecKitValidate
                | SlashCommand::SpecKitAudit
                | SlashCommand::SpecKitUnlock
                | SlashCommand::SpecKitAuto
                | SlashCommand::SpecKitStatus
                | SlashCommand::SpecKitConfigure
        )
    }

    /// Returns true when this command maps to Spec Ops automation.
    /// SPEC-KIT-902: Legacy /spec-ops-* removed. Only SpecEvidenceStats remains.
    pub fn is_spec_ops(self) -> bool {
        matches!(self, SlashCommand::SpecEvidenceStats)
    }

    /// Returns Spec Ops metadata for the command.
    /// SPEC-KIT-902: Legacy /spec-ops-* removed. Only SpecEvidenceStats remains.
    pub fn spec_ops(self) -> Option<SpecOpsCommand> {
        match self {
            SlashCommand::SpecEvidenceStats => Some(SpecOpsCommand {
                display: "evidence-stats",
                script: "evidence_stats.sh",
            }),
            _ => None,
        }
    }

    /// SPEC-KIT-902: spec_stage() removed. Use speckit command registry instead.
    #[allow(dead_code)]
    pub fn spec_stage(self) -> Option<SpecStage> {
        // Legacy /spec-* commands removed. This method is kept for compatibility
        // but always returns None. Use the command registry for stage mapping.
        None
    }

    pub fn is_available(self) -> bool {
        match self {
            SlashCommand::Pro => pro_command_enabled(),
            SlashCommand::Demo => demo_command_enabled(),
            _ => true,
        }
    }

    /// Expands a prompt-expanding command into a full prompt for the LLM.
    /// SPEC-KIT-963: All prompt-expanding commands removed. Returns None always.
    /// Spec-kit commands use the registry-based system instead.
    #[allow(unused_variables)]
    pub fn expand_prompt(self, args: &str) -> Option<String> {
        // is_prompt_expanding() returns false for all commands
        None
    }
}

/// Return all built-in commands in a Vec paired with their command string.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::iter()
        .filter(|c| c.is_available())
        .map(|c| (c.command(), c))
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct SpecOpsCommand {
    pub display: &'static str,
    pub script: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalMode {
    Mock,
    Live,
}

impl HalMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "mock" | "skip" => Some(Self::Mock),
            "live" | "real" => Some(Self::Live),
            _ => None,
        }
    }

    pub fn as_env_value(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Live => "live",
        }
    }
}

/// Process a message that might contain a slash command.
/// Returns either the expanded prompt (for prompt-expanding commands) or the original message.
pub fn process_slash_command_message(message: &str) -> ProcessedCommand {
    let trimmed = message.trim();

    if trimmed.is_empty() {
        return ProcessedCommand::NotCommand(message.to_string());
    }

    let has_slash = trimmed.starts_with('/');
    let command_portion = if has_slash { &trimmed[1..] } else { trimmed };
    let parts: Vec<&str> = command_portion.splitn(2, ' ').collect();
    let command_str = parts.first().copied().unwrap_or("");
    let args_raw = parts.get(1).map(|s| s.trim()).unwrap_or("");
    let canonical_command = command_str.to_ascii_lowercase();

    if matches!(canonical_command.as_str(), "quit" | "exit") {
        if !has_slash && !args_raw.is_empty() {
            return ProcessedCommand::NotCommand(message.to_string());
        }

        let command_text = if args_raw.is_empty() {
            format!("/{}", SlashCommand::Quit.command())
        } else {
            format!("/{} {}", SlashCommand::Quit.command(), args_raw)
        };

        return ProcessedCommand::RegularCommand {
            command: SlashCommand::Quit,
            command_text,
            notice: None,
        };
    }

    if !has_slash {
        return ProcessedCommand::NotCommand(message.to_string());
    }

    // Try to parse the command
    if let Ok(command) = canonical_command.parse::<SlashCommand>() {
        if !command.is_available() {
            let command_name = command.command();
            let message = match command {
                SlashCommand::Pro => {
                    "Error: /pro is only available in dev, dev-fast, or pref builds.".to_string()
                }
                SlashCommand::Demo => {
                    format!("Error: /{command_name} is only available in dev or perf builds.")
                }
                _ => format!("Error: /{command_name} is not available in this build."),
            };
            return ProcessedCommand::Error(message);
        }

        if let Some(stage) = command.spec_stage() {
            match spec_prompts::build_stage_prompt(stage, args_raw) {
                Ok(prompt) => return ProcessedCommand::ExpandedPrompt(prompt),
                Err(err) => return ProcessedCommand::Error(err.to_string()),
            }
        }

        // SPEC-KIT-902: SpecAuto removed, use SpecKitAuto for /speckit.auto
        if command == SlashCommand::SpecKitAuto {
            match parse_spec_auto_args(args_raw) {
                Ok(auto) => {
                    return ProcessedCommand::SpecAuto(auto);
                }
                Err(err) => {
                    return ProcessedCommand::Error(err.to_string());
                }
            }
        }

        // Check if it's a prompt-expanding command
        if command.is_prompt_expanding() {
            if args_raw.is_empty() && command.requires_arguments() {
                return ProcessedCommand::Error(format!(
                    "Error: /{} requires a task description. Usage: /{} <task>",
                    command.command(),
                    command.command()
                ));
            }

            if let Some(expanded) = command.expand_prompt(args_raw) {
                return ProcessedCommand::ExpandedPrompt(expanded);
            }
        }

        let command_text = if args_raw.is_empty() {
            format!("/{}", command.command())
        } else {
            format!("/{} {}", command.command(), args_raw)
        };

        ProcessedCommand::RegularCommand {
            command,
            command_text,
            notice: None,
        }
    } else {
        // Unknown command
        ProcessedCommand::NotCommand(message.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum ProcessedCommand {
    /// The message was expanded from a prompt-expanding slash command
    ExpandedPrompt(String),
    /// A regular slash command that should be handled by the TUI. The `String`
    /// contains the canonical command text (with leading slash and trimmed args).
    RegularCommand {
        command: SlashCommand,
        command_text: String,
        notice: Option<String>,
    },
    SpecAuto(SpecAutoInvocation),
    /// Not a slash command, just a regular message
    #[allow(dead_code)]
    NotCommand(String),
    /// Error processing the command
    Error(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SpecAutoInvocation {
    pub spec_id: String,
    pub goal: String,
    pub resume_from: SpecStage,
    pub hal_mode: Option<HalMode>,
    /// SPEC-948: CLI args for pipeline configuration (--skip-*, --stages=, etc.)
    pub cli_args: Vec<String>,
    /// SPEC-KIT-102: Disable Stage 0 context injection
    pub no_stage0: bool,
    /// SPEC-KIT-102: Include score breakdown in TASK_BRIEF
    pub stage0_explain: bool,
}

#[derive(Debug, Error)]
pub enum SpecAutoParseError {
    #[error("`/speckit.auto` requires a SPEC ID (e.g. `/speckit.auto SPEC-KIT-900`)")]
    MissingSpecId,
    #[error("`/speckit.auto --from` requires a stage name")]
    MissingFromStage,
    #[error("Unknown stage '{0}'. Expected plan, tasks, implement, validate, review, or unlock.")]
    UnknownStage(String),
    #[error("Unknown HAL mode '{0}'. Expected 'mock' or 'live'.")]
    UnknownHalMode(String),
}

pub fn parse_spec_auto_args(args: &str) -> Result<SpecAutoInvocation, SpecAutoParseError> {
    let mut tokens = args.split_whitespace();
    let Some(spec_token) = tokens.next() else {
        return Err(SpecAutoParseError::MissingSpecId);
    };

    let mut resume_from = SpecStage::Plan;
    let mut goal_tokens: Vec<String> = Vec::new();
    let mut pending_from = false;
    let mut pending_hal = false;
    let mut hal_mode: Option<HalMode> = None;
    let mut cli_args: Vec<String> = Vec::new(); // SPEC-948: Pipeline config flags
    let mut no_stage0 = false; // SPEC-KIT-102: Stage 0 flags
    let mut stage0_explain = false;

    for token in tokens {
        if pending_from {
            resume_from = parse_stage_name(token)
                .ok_or_else(|| SpecAutoParseError::UnknownStage(token.to_string()))?;
            pending_from = false;
            continue;
        }

        if pending_hal {
            hal_mode = Some(
                HalMode::parse(token)
                    .ok_or_else(|| SpecAutoParseError::UnknownHalMode(token.to_string()))?,
            );
            pending_hal = false;
            continue;
        }

        if let Some((flag, value)) = token.split_once('=') {
            if matches!(flag, "--hal" | "--hal-mode") {
                hal_mode = Some(
                    HalMode::parse(value)
                        .ok_or_else(|| SpecAutoParseError::UnknownHalMode(value.to_string()))?,
                );
                continue;
            }
            // SPEC-948: Collect --stages=plan,tasks,... flags
            if flag == "--stages" {
                cli_args.push(token.to_string());
                continue;
            }
        }

        match token {
            "--from" | "--resume-from" => {
                pending_from = true;
            }
            "--hal" | "--hal-mode" => {
                pending_hal = true;
            }
            // SPEC-948: Collect --skip-* and --only-* pipeline flags
            // SPEC-947: Collect --configure flag for interactive modal
            t if t.starts_with("--skip-") || t.starts_with("--only-") || t == "--configure" => {
                cli_args.push(t.to_string());
            }
            // SPEC-KIT-102: Stage 0 control flags
            "--no-stage0" => {
                no_stage0 = true;
            }
            "--stage0-explain" => {
                stage0_explain = true;
            }
            _ => goal_tokens.push(token.to_string()),
        }
    }

    if pending_from {
        return Err(SpecAutoParseError::MissingFromStage);
    }
    if pending_hal {
        return Err(SpecAutoParseError::UnknownHalMode(String::new()));
    }

    Ok(SpecAutoInvocation {
        spec_id: spec_token.to_string(),
        goal: goal_tokens.join(" "),
        resume_from,
        hal_mode,
        cli_args,       // SPEC-948
        no_stage0,      // SPEC-KIT-102
        stage0_explain, // SPEC-KIT-102
    })
}

fn parse_stage_name(value: &str) -> Option<SpecStage> {
    match value.to_ascii_lowercase().as_str() {
        "plan" | "spec-plan" => Some(SpecStage::Plan),
        "tasks" | "spec-tasks" => Some(SpecStage::Tasks),
        "implement" | "spec-implement" => Some(SpecStage::Implement),
        "validate" | "spec-validate" => Some(SpecStage::Validate),
        "review" | "spec-review" | "audit" | "spec-ops-audit" => Some(SpecStage::Audit),
        "unlock" | "spec-unlock" => Some(SpecStage::Unlock),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SPEC-KIT-902: legacy_spec_alias_emits_notice test removed.
    // /spec-plan variant removed; use /speckit.plan instead.

    #[test]
    fn parse_spec_auto_args_supports_from_flag() {
        let auto = parse_spec_auto_args("SPEC-OPS-007 --from tasks align checkout").unwrap();
        assert_eq!(auto.spec_id, "SPEC-OPS-007");
        assert_eq!(auto.goal, "align checkout");
        assert_eq!(auto.resume_from, SpecStage::Tasks);
        assert!(auto.hal_mode.is_none());
    }

    #[test]
    fn parse_spec_auto_args_supports_hal_flag() {
        let auto = parse_spec_auto_args("SPEC-OPS-010 --hal live investigate").unwrap();
        assert_eq!(auto.spec_id, "SPEC-OPS-010");
        assert_eq!(auto.goal, "investigate");
        assert_eq!(auto.hal_mode, Some(HalMode::Live));
    }

    #[test]
    fn parse_spec_auto_args_rejects_unknown_hal() {
        let err = parse_spec_auto_args("SPEC-OPS-010 --hal banana").unwrap_err();
        assert!(matches!(err, SpecAutoParseError::UnknownHalMode(_)));
    }

    // SPEC-KIT-902: spec_ops_auto_maps_to_spec_auto_script test removed.
    // SpecKitAuto uses the command registry, not spec_ops() shell script metadata.

    #[test]
    fn spec_evidence_stats_maps_to_evidence_stats_script() {
        let command = SlashCommand::SpecEvidenceStats;
        assert!(
            command.is_spec_ops(),
            "/spec-evidence-stats should be recognized as Spec Ops command"
        );
        let meta = command.spec_ops().expect("Spec Ops metadata");
        assert_eq!(meta.display, "evidence-stats");
        assert_eq!(meta.script, "evidence_stats.sh");
    }

    // SPEC-948 Phase 3 Task 3.2: CLI flag parsing tests
    #[test]
    fn parse_spec_auto_args_supports_skip_flags() {
        let auto = parse_spec_auto_args("SPEC-948 --skip-validate --skip-audit").unwrap();
        assert_eq!(auto.spec_id, "SPEC-948");
        assert_eq!(auto.cli_args.len(), 2);
        assert!(auto.cli_args.contains(&"--skip-validate".to_string()));
        assert!(auto.cli_args.contains(&"--skip-audit".to_string()));
    }

    #[test]
    fn parse_spec_auto_args_supports_only_flags() {
        let auto = parse_spec_auto_args("SPEC-948 --only-plan --only-tasks").unwrap();
        assert_eq!(auto.spec_id, "SPEC-948");
        assert_eq!(auto.cli_args.len(), 2);
        assert!(auto.cli_args.contains(&"--only-plan".to_string()));
        assert!(auto.cli_args.contains(&"--only-tasks".to_string()));
    }

    #[test]
    fn parse_spec_auto_args_supports_stages_list() {
        let auto = parse_spec_auto_args("SPEC-948 --stages=plan,tasks,implement").unwrap();
        assert_eq!(auto.spec_id, "SPEC-948");
        assert_eq!(auto.cli_args.len(), 1);
        assert_eq!(auto.cli_args[0], "--stages=plan,tasks,implement");
    }

    #[test]
    fn parse_spec_auto_args_cli_flags_with_goal() {
        let auto = parse_spec_auto_args("SPEC-948 --skip-validate optimize cost").unwrap();
        assert_eq!(auto.spec_id, "SPEC-948");
        assert_eq!(auto.goal, "optimize cost");
        assert_eq!(auto.cli_args.len(), 1);
        assert_eq!(auto.cli_args[0], "--skip-validate");
    }

    #[test]
    fn parse_spec_auto_args_cli_flags_with_from_and_hal() {
        let auto =
            parse_spec_auto_args("SPEC-948 --from tasks --hal live --skip-audit debug").unwrap();
        assert_eq!(auto.spec_id, "SPEC-948");
        assert_eq!(auto.resume_from, SpecStage::Tasks);
        assert_eq!(auto.hal_mode, Some(HalMode::Live));
        assert_eq!(auto.goal, "debug");
        assert_eq!(auto.cli_args.len(), 1);
        assert_eq!(auto.cli_args[0], "--skip-audit");
    }

    #[test]
    fn parse_spec_auto_args_no_cli_flags_empty_vec() {
        let auto = parse_spec_auto_args("SPEC-948 simple goal").unwrap();
        assert_eq!(auto.spec_id, "SPEC-948");
        assert_eq!(auto.goal, "simple goal");
        assert!(auto.cli_args.is_empty());
    }

    // SPEC-KIT-102: Stage 0 flag parsing tests
    #[test]
    fn parse_spec_auto_args_supports_no_stage0() {
        let auto = parse_spec_auto_args("SPEC-102 --no-stage0").unwrap();
        assert_eq!(auto.spec_id, "SPEC-102");
        assert!(auto.no_stage0);
        assert!(!auto.stage0_explain);
    }

    #[test]
    fn parse_spec_auto_args_supports_stage0_explain() {
        let auto = parse_spec_auto_args("SPEC-102 --stage0-explain").unwrap();
        assert_eq!(auto.spec_id, "SPEC-102");
        assert!(!auto.no_stage0);
        assert!(auto.stage0_explain);
    }

    #[test]
    fn parse_spec_auto_args_supports_both_stage0_flags() {
        let auto =
            parse_spec_auto_args("SPEC-102 --no-stage0 --stage0-explain debug goal").unwrap();
        assert_eq!(auto.spec_id, "SPEC-102");
        assert!(auto.no_stage0);
        assert!(auto.stage0_explain);
        assert_eq!(auto.goal, "debug goal");
    }

    #[test]
    fn parse_spec_auto_args_stage0_flags_default_false() {
        let auto = parse_spec_auto_args("SPEC-102 some goal").unwrap();
        assert_eq!(auto.spec_id, "SPEC-102");
        assert!(!auto.no_stage0);
        assert!(!auto.stage0_explain);
        assert_eq!(auto.goal, "some goal");
    }
}
