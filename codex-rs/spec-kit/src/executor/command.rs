//! Spec-Kit Command Model
//!
//! SPEC-KIT-921: Shared command enum for TUI and CLI parity.
//!
//! Both adapters parse their input into `SpeckitCommand`, ensuring:
//! - Same business logic for equivalent user intent
//! - Parity tests can verify slash commands match CLI commands

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
    // Future variants (Phase B+):
    // Review { spec_id: String, stage: Option<Stage> },
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
}
