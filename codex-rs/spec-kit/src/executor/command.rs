//! Spec-Kit Command Model
//!
//! SPEC-KIT-921: Shared command enum for TUI and CLI parity.
//!
//! Both adapters parse their input into `SpeckitCommand`, ensuring:
//! - Same business logic for equivalent user intent
//! - Parity tests can verify slash commands match CLI commands

use crate::Stage;

/// Spec-Kit command — the shared command model
///
/// Both TUI slash commands and CLI subcommands parse into this enum.
/// The executor dispatches based on the variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeckitCommand {
    /// Show SPEC status dashboard
    ///
    /// TUI: `/speckit.status <SPEC-ID> [--stale-hours N]`
    /// CLI: `code speckit status --spec <SPEC-ID> [--stale-hours N]`
    Status {
        /// The SPEC identifier (e.g., "SPEC-KIT-921")
        spec_id: String,

        /// Hours after which telemetry is considered stale (default: 24)
        stale_hours: i64,
    },

    /// Review a specific stage's gate artifacts
    ///
    /// TUI: `/review <STAGE> [--strict-artifacts] [--strict-warnings]`
    /// CLI: `code speckit review --spec <SPEC-ID> --stage <STAGE> [--strict-*]`
    Review {
        /// The SPEC identifier
        spec_id: String,

        /// Stage to review (plan, tasks, implement, validate, audit)
        stage: Stage,

        /// Fail if expected artifacts are missing
        strict_artifacts: bool,

        /// Treat PassedWithWarnings as exit 1
        strict_warnings: bool,
    },
    // Future variants (Phase B+):
    // Doctor { format: OutputFormat },
    // Run { spec_id: String, from_stage: Option<Stage>, to_stage: Option<Stage>, ... },
}

impl SpeckitCommand {
    /// Parse from slash command arguments
    ///
    /// Used by TUI to convert raw slash command input to SpeckitCommand.
    pub fn parse_status(raw_args: &str) -> Result<Self, String> {
        let trimmed = raw_args.trim();
        if trimmed.is_empty() {
            return Err("Usage: /speckit.status <SPEC-ID> [--stale-hours N]".to_string());
        }

        let mut spec_id: Option<String> = None;
        let mut stale_hours: i64 = 24; // default

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        let mut idx = 0;

        while idx < tokens.len() {
            let token = tokens[idx];

            if token.starts_with("--stale-hours") {
                let value = if let Some(eq_pos) = token.find('=') {
                    &token[(eq_pos + 1)..]
                } else {
                    idx += 1;
                    tokens
                        .get(idx)
                        .copied()
                        .ok_or_else(|| "`--stale-hours` requires a value".to_string())?
                };
                stale_hours = value
                    .parse::<i64>()
                    .map_err(|_| "invalid value for --stale-hours".to_string())?;
            } else if token.starts_with('-') {
                return Err(format!("Unknown flag `{}`", token));
            } else if spec_id.is_none() {
                spec_id = Some(token.to_string());
            } else {
                return Err(format!("Unexpected extra argument `{}`", token));
            }

            idx += 1;
        }

        let spec_id = spec_id.ok_or_else(|| {
            "/speckit.status requires a SPEC ID (e.g., /speckit.status SPEC-KIT-921)".to_string()
        })?;

        Ok(SpeckitCommand::Status {
            spec_id,
            stale_hours,
        })
    }

    /// Parse review command from slash command arguments
    ///
    /// Used by TUI: `/review <STAGE> [--strict-artifacts] [--strict-warnings]`
    /// Note: spec_id is provided separately (from active spec context)
    pub fn parse_review(spec_id: &str, raw_args: &str) -> Result<Self, String> {
        let trimmed = raw_args.trim();
        if trimmed.is_empty() {
            return Err(
                "Usage: /review <stage> [--strict-artifacts] [--strict-warnings]\n\
                 Stages: plan, tasks, implement, validate, audit"
                    .to_string(),
            );
        }

        let mut stage: Option<Stage> = None;
        let mut strict_artifacts = false;
        let mut strict_warnings = false;

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        for token in tokens {
            match token {
                "--strict-artifacts" => strict_artifacts = true,
                "--strict-warnings" => strict_warnings = true,
                s if s.starts_with('-') => {
                    return Err(format!("Unknown flag `{}`", s));
                }
                s if stage.is_none() => {
                    stage = Some(Self::parse_stage(s)?);
                }
                s => {
                    return Err(format!("Unexpected extra argument `{}`", s));
                }
            }
        }

        let stage = stage.ok_or_else(|| {
            "Stage required. Valid stages: plan, tasks, implement, validate, audit".to_string()
        })?;

        Ok(SpeckitCommand::Review {
            spec_id: spec_id.to_string(),
            stage,
            strict_artifacts,
            strict_warnings,
        })
    }

    /// Parse stage from user input
    fn parse_stage(input: &str) -> Result<Stage, String> {
        match input.to_lowercase().as_str() {
            "specify" => Ok(Stage::Specify),
            "plan" => Ok(Stage::Plan),
            "tasks" => Ok(Stage::Tasks),
            "implement" => Ok(Stage::Implement),
            "validate" => Ok(Stage::Validate),
            "audit" => Ok(Stage::Audit),
            "unlock" => Ok(Stage::Unlock),
            _ => Err(format!(
                "Unknown stage `{}`. Valid stages: specify, plan, tasks, implement, validate, audit, unlock",
                input
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_basic() {
        let cmd = SpeckitCommand::parse_status("SPEC-123").unwrap();
        assert_eq!(
            cmd,
            SpeckitCommand::Status {
                spec_id: "SPEC-123".to_string(),
                stale_hours: 24,
            }
        );
    }

    #[test]
    fn test_parse_status_with_stale_hours_eq() {
        let cmd = SpeckitCommand::parse_status("SPEC-456 --stale-hours=48").unwrap();
        assert_eq!(
            cmd,
            SpeckitCommand::Status {
                spec_id: "SPEC-456".to_string(),
                stale_hours: 48,
            }
        );
    }

    #[test]
    fn test_parse_status_with_stale_hours_space() {
        let cmd = SpeckitCommand::parse_status("SPEC-789 --stale-hours 72").unwrap();
        assert_eq!(
            cmd,
            SpeckitCommand::Status {
                spec_id: "SPEC-789".to_string(),
                stale_hours: 72,
            }
        );
    }

    #[test]
    fn test_parse_status_empty() {
        let result = SpeckitCommand::parse_status("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_status_unknown_flag() {
        let result = SpeckitCommand::parse_status("SPEC-123 --unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown flag"));
    }

    // === Review command tests ===

    #[test]
    fn test_parse_review_basic() {
        let cmd = SpeckitCommand::parse_review("SPEC-123", "plan").unwrap();
        assert_eq!(
            cmd,
            SpeckitCommand::Review {
                spec_id: "SPEC-123".to_string(),
                stage: Stage::Plan,
                strict_artifacts: false,
                strict_warnings: false,
            }
        );
    }

    #[test]
    fn test_parse_review_with_strict_flags() {
        let cmd =
            SpeckitCommand::parse_review("SPEC-456", "audit --strict-artifacts --strict-warnings")
                .unwrap();
        assert_eq!(
            cmd,
            SpeckitCommand::Review {
                spec_id: "SPEC-456".to_string(),
                stage: Stage::Audit,
                strict_artifacts: true,
                strict_warnings: true,
            }
        );
    }

    #[test]
    fn test_parse_review_case_insensitive_stage() {
        let cmd = SpeckitCommand::parse_review("TEST", "PLAN").unwrap();
        assert!(matches!(
            cmd,
            SpeckitCommand::Review {
                stage: Stage::Plan,
                ..
            }
        ));

        let cmd = SpeckitCommand::parse_review("TEST", "Tasks").unwrap();
        assert!(matches!(
            cmd,
            SpeckitCommand::Review {
                stage: Stage::Tasks,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_review_all_stages() {
        for (input, expected) in [
            ("specify", Stage::Specify),
            ("plan", Stage::Plan),
            ("tasks", Stage::Tasks),
            ("implement", Stage::Implement),
            ("validate", Stage::Validate),
            ("audit", Stage::Audit),
            ("unlock", Stage::Unlock),
        ] {
            let cmd = SpeckitCommand::parse_review("TEST", input).unwrap();
            assert!(
                matches!(cmd, SpeckitCommand::Review { stage, .. } if stage == expected),
                "Failed for stage: {}",
                input
            );
        }
    }

    #[test]
    fn test_parse_review_empty() {
        let result = SpeckitCommand::parse_review("SPEC", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Usage:"));
    }

    #[test]
    fn test_parse_review_unknown_stage() {
        let result = SpeckitCommand::parse_review("SPEC", "unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown stage"));
    }

    #[test]
    fn test_parse_review_unknown_flag() {
        let result = SpeckitCommand::parse_review("SPEC", "plan --unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown flag"));
    }
}
