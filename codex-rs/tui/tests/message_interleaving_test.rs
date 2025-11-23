//! Test for message interleaving investigation
//!
//! Simulates user messages and streaming responses to inspect
//! the order in which history cells are added.

use codex_tui::chatwidget::ChatWidget;
// SPEC-955: Migrated to tokio channels
use tokio::sync::mpsc::unbounded_channel;

/// Create a minimal ChatWidget for testing
fn make_test_widget() -> ChatWidget<'static> {
    use codex_core::config::Config;
    use codex_core::config::ConfigOverrides;
    use codex_core::config::ConfigToml;
    use codex_tui::app_event_sender::AppEventSender;
    use codex_tui::bottom_pane::{BottomPane, BottomPaneParams};
    use codex_tui::streaming::StreamController;
    use std::collections::HashMap;

    let (tx_raw, _rx) = unbounded_channel();
    let app_event_tx = AppEventSender::new(tx_raw);
    let (op_tx, _op_rx) = unbounded_channel();

    let cfg = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        std::env::temp_dir(),
    )
    .expect("config");

    let bottom = BottomPane::new(BottomPaneParams {
        app_event_tx: app_event_tx.clone(),
        has_input_focus: true,
        enhanced_keys_supported: false,
        using_chatgpt_auth: false,
    });

    ChatWidget {
        app_event_tx,
        codex_op_tx: op_tx,
        bottom_pane: bottom,
        active_exec_cell: None,
        config: cfg.clone(),
        latest_upgrade_version: None,
        initial_user_message: None,
        total_token_usage: Default::default(),
        last_token_usage: Default::default(),
        rate_limit_snapshot: None,
        rate_limit_warnings: Default::default(),
        rate_limit_fetch_inflight: false,
        rate_limit_fetch_placeholder: None,
        rate_limit_fetch_ack_pending: false,
        #[cfg(not(feature = "legacy_tests"))]
        ghost_snapshots: Vec::new(),
        #[cfg(not(feature = "legacy_tests"))]
        ghost_snapshots_disabled: false,
        #[cfg(not(feature = "legacy_tests"))]
        ghost_snapshots_disabled_reason: None,
        stream: StreamController::new(cfg),
        last_stream_kind: None,
        running_commands: HashMap::new(),
        pending_exec_completions: Vec::new(),
        task_complete_pending: false,
        interrupts: codex_tui::interrupts::InterruptManager::new(),
        needs_redraw: false,
        agents_terminal: codex_tui::agents_terminal::AgentsTerminalState::new(),
        // Add missing fields as they're discovered during compilation
    }
}

#[test]
fn test_message_history_order() {
    // This test will fail to compile initially - we need to add missing fields
    // Once it compiles, we can inspect the history order

    let _widget = make_test_widget();

    // TODO: Once compilation succeeds, add test logic:
    // 1. Simulate user message via finalize_sent_user_message
    // 2. Simulate streaming response via on_native_stream_delta
    // 3. Finalize via on_native_stream_complete
    // 4. Inspect widget.history_cells to see order
    // 5. Print keys/tags/types

    println!("Test compiles! Ready to add message simulation logic.");
}
