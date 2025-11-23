//! Test support utilities for ChatWidget testing
//!
//! This module provides helper functions and test fixtures used across
//! ChatWidget tests. Extracted from mod.rs to improve maintainability.

use super::*;
use codex_core::config::{Config, ConfigOverrides, ConfigToml};
use std::path::Path;
use std::sync::Arc;

/// Create a test config with default settings
pub(crate) fn test_config() -> Config {
    test_config_with_cwd(std::env::temp_dir().as_path())
}

/// Create a test config with a specific current working directory
pub(crate) fn test_config_with_cwd(cwd: &Path) -> Config {
    let mut overrides = ConfigOverrides::default();
    overrides.cwd = Some(cwd.to_path_buf());
    codex_core::config::Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        overrides,
        cwd.to_path_buf(),
    )
    .expect("cfg")
}

/// Create a minimal ChatWidget for testing with default settings
pub(crate) fn make_widget() -> ChatWidget<'static> {
    make_widget_with_dir(std::env::temp_dir().as_path())
}

/// Create a minimal ChatWidget for testing with a specific directory
pub(crate) fn make_widget_with_dir(cwd: &Path) -> ChatWidget<'static> {
    let (tx_raw, _rx) = std::sync::mpsc::channel::<AppEvent>();
    let app_event_tx = AppEventSender::new(tx_raw);
    let cfg = test_config_with_cwd(cwd);
    let term = crate::tui::TerminalInfo {
        picker: None,
        font_size: (8, 16),
    };
    ChatWidget::new(
        cfg,
        app_event_tx,
        None,
        Vec::new(),
        false,
        term,
        false,
        None,
        Arc::new(tokio::sync::Mutex::new(None)), // Test: no MCP manager needed
        None,                                    // initial_command (SPEC-KIT-920)
    )
}

/// Helper to create a minimal ChatWidget for key generation testing
/// Reuses make_widget() for consistency
pub(crate) fn create_test_widget_for_keygen() -> ChatWidget<'static> {
    make_widget()
}
