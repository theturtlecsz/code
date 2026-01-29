//! Headless pipeline execution for CLI automation (SPEC-KIT-900)
//!
//! This module enables `/speckit.auto` pipeline execution without a terminal/TUI.
//! It is invoked by `code speckit run --execute --headless`.
//!
//! ## Exit Codes (D133)
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0    | SUCCESS - Pipeline completed |
//! | 3    | INFRA_ERROR - System/infrastructure failure |
//! | 10   | NEEDS_INPUT - Missing maieutic in headless mode |
//! | 11   | NEEDS_APPROVAL - Tier-2/3 checkpoint requires approval |
//! | 13   | PROMPT_ATTEMPTED - Any prompt/UI interaction attempted |
//!
//! ## Constraints (D113, D133)
//!
//! - Headless mode MUST NEVER prompt (any attempt exits with 13)
//! - Maieutic input is mandatory (exit 10 if missing)
//! - All decisions must be pre-supplied via maieutic answers

mod event_pump;
pub mod output;
mod runner;

pub use event_pump::{HeadlessEventPump, wait_for_agents};
pub use output::{HeadlessOutput, format_result_json};
pub use runner::{
    HeadlessConfig, HeadlessError, HeadlessPipelineRunner, HeadlessResult, exit_codes,
};
