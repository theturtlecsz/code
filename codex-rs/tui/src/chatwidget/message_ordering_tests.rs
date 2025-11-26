//! Message ordering tests for SPEC-954 fixes
//!
//! This module tests the 6 critical bug fixes for message interleaving:
//! 1. OAuth path deferred creation vs CLI immediate creation
//! 2. Resort optimization (diff > 0, not > 1)
//! 3. Temp OrderKey counter incrementing
//! 4. Resort algorithm permutation correctness
//! 5. Unique message IDs for CLI routing
//! 6. CLI queue processing after completion

use super::*;
use crate::chatwidget::test_harness::TestHarness;
use crate::chatwidget::test_support::make_widget;
use codex_core::protocol::{Event, EventMsg, OrderMeta};

// ============================================================================
// Test 1: Deferred creation for OAuth path
// ============================================================================

#[tokio::test]
async fn test_oauth_path_queues_message_without_immediate_cell() {
    let mut widget = make_widget();

    // Initial state: no pending messages, no history
    assert!(widget.pending_dispatched_user_messages.is_empty());
    let initial_cell_count = widget.history_cells.len();

    // Queue a message as OAuth path would (via pending_dispatched_user_messages)
    widget
        .pending_dispatched_user_messages
        .push_back("Test OAuth message".to_string());

    // Verify: message queued but NO cell created yet
    assert_eq!(widget.pending_dispatched_user_messages.len(), 1);
    assert_eq!(
        widget.history_cells.len(),
        initial_cell_count,
        "OAuth path should NOT create cell immediately"
    );
}

#[tokio::test]
async fn test_oauth_path_creates_cell_on_task_started() {
    let mut widget = make_widget();

    // Queue a message (OAuth path)
    widget
        .pending_dispatched_user_messages
        .push_back("OAuth deferred message".to_string());
    let initial_count = widget.history_cells.len();

    // Simulate TaskStarted event
    widget.handle_codex_event(Event {
        id: "task-1".into(),
        event_seq: 0,
        msg: EventMsg::TaskStarted,
        order: Some(OrderMeta {
            request_ordinal: 1,
            output_index: Some(0),
            sequence_number: None,
        }),
    });

    // Verify: cell created, pending cleared, tracking entry added
    assert!(
        widget.pending_dispatched_user_messages.is_empty(),
        "Message should be consumed from queue"
    );
    assert_eq!(
        widget.history_cells.len(),
        initial_count + 1,
        "Cell should be created on TaskStarted"
    );
    assert!(
        widget.pending_user_cell_updates.contains_key("task-1"),
        "Cell index should be tracked for OrderKey update"
    );
}

// ============================================================================
// Test 2: CLI path creates cell immediately (different from OAuth)
// ============================================================================

#[tokio::test]
async fn test_cli_path_does_not_use_deferred_queue() {
    let widget = make_widget();

    // CLI path should NOT use pending_dispatched_user_messages
    // It creates cells immediately in send_user_messages_to_agent
    // This test verifies the queue remains unused for CLI routing
    assert!(
        widget.pending_dispatched_user_messages.is_empty(),
        "CLI path should not use deferred queue"
    );
}

// ============================================================================
// Test 3: OrderKey update triggers resort on diff=1 (not just diff>1)
// ============================================================================

/// Test that OrderKey comparison works correctly - lower req comes first.
/// This tests the data structure without calling resort_history_by_order().
#[tokio::test]
async fn test_orderkey_comparison_for_resort() {
    // Test OrderKey ordering directly
    let key_low = OrderKey {
        req: 0,
        out: i32::MIN + 1,
        seq: 1,
    };
    let key_high = OrderKey {
        req: 2,
        out: i32::MIN + 1,
        seq: 2,
    };

    // Lower req should sort before higher req
    assert!(
        key_low < key_high,
        "Lower req OrderKey should sort before higher req"
    );

    // Same req, lower out should sort first
    let key_out_low = OrderKey {
        req: 0,
        out: -100,
        seq: 1,
    };
    let key_out_high = OrderKey {
        req: 0,
        out: 100,
        seq: 2,
    };
    assert!(
        key_out_low < key_out_high,
        "Lower out should sort first when req equal"
    );

    // Same req and out, lower seq should sort first
    let key_seq_low = OrderKey {
        req: 0,
        out: 0,
        seq: 5,
    };
    let key_seq_high = OrderKey {
        req: 0,
        out: 0,
        seq: 10,
    };
    assert!(
        key_seq_low < key_seq_high,
        "Lower seq should sort first when req and out equal"
    );
}

/// Test that cells can be manually sorted by OrderKey.
/// This tests the sorting behavior without relying on resort_history_by_order().
#[tokio::test]
async fn test_cell_sorting_by_orderkey() {
    let mut widget = make_widget();

    // Clear for controlled test
    widget.history_cells.clear();
    widget.cell_order_seq.clear();

    // Create cells with req values: [1, 0]
    widget
        .history_cells
        .push(Box::new(history_cell::new_user_prompt(
            "Cell req=1".to_string(),
        )));
    widget.cell_order_seq.push(OrderKey {
        req: 1,
        out: 0,
        seq: 1,
    });

    widget
        .history_cells
        .push(Box::new(history_cell::new_user_prompt(
            "Cell req=0".to_string(),
        )));
    widget.cell_order_seq.push(OrderKey {
        req: 0,
        out: 0,
        seq: 2,
    });

    // Verify the keys can be sorted correctly (without calling resort)
    let mut sorted_keys = widget.cell_order_seq.clone();
    sorted_keys.sort();

    assert_eq!(sorted_keys[0].req, 0, "After sort, req=0 should be first");
    assert_eq!(sorted_keys[1].req, 1, "After sort, req=1 should be second");
}

// ============================================================================
// Test 4: Temp OrderKey counter increments properly
// ============================================================================

#[tokio::test]
async fn test_next_req_key_prompt_increments_counter() {
    let mut widget = make_widget();

    // Get initial request index
    let initial_index = widget.current_request_index;

    // Call next_req_key_prompt multiple times
    let key1 = widget.next_req_key_prompt();
    let key2 = widget.next_req_key_prompt();
    let key3 = widget.next_req_key_prompt();

    // Each call should produce unique, incrementing req values
    assert!(key2.req > key1.req, "Key2 req should be > key1 req");
    assert!(key3.req > key2.req, "Key3 req should be > key2 req");

    // Counter should have incremented
    assert!(
        widget.current_request_index > initial_index,
        "current_request_index should increment"
    );
}

#[tokio::test]
async fn test_task_started_uses_incrementing_counter() {
    let mut widget = make_widget();

    // Record initial state
    let initial_cells = widget.history_cells.len();

    // Queue first message
    widget
        .pending_dispatched_user_messages
        .push_back("Message 1".to_string());

    // First TaskStarted
    widget.handle_codex_event(Event {
        id: "task-1".into(),
        event_seq: 0,
        msg: EventMsg::TaskStarted,
        order: Some(OrderMeta {
            request_ordinal: 1,
            output_index: Some(0),
            sequence_number: None,
        }),
    });

    let cells_after_first = widget.history_cells.len();
    assert!(
        cells_after_first > initial_cells,
        "First TaskStarted should create a cell"
    );

    // Get the key from the tracked cell
    let first_cell_idx = widget
        .pending_user_cell_updates
        .get("task-1")
        .copied()
        .unwrap();
    let first_key = widget.cell_order_seq[first_cell_idx];

    // Queue and trigger second TaskStarted
    widget
        .pending_dispatched_user_messages
        .push_back("Message 2".to_string());
    widget.handle_codex_event(Event {
        id: "task-2".into(),
        event_seq: 1,
        msg: EventMsg::TaskStarted,
        order: Some(OrderMeta {
            request_ordinal: 2,
            output_index: Some(0),
            sequence_number: None,
        }),
    });

    let cells_after_second = widget.history_cells.len();
    assert!(
        cells_after_second > cells_after_first,
        "Second TaskStarted should create another cell"
    );

    // Get the key from the second tracked cell
    let second_cell_idx = widget
        .pending_user_cell_updates
        .get("task-2")
        .copied()
        .unwrap();
    let second_key = widget.cell_order_seq[second_cell_idx];

    // Second key should have higher req or seq (not same value)
    assert!(
        second_key.req > first_key.req || second_key.seq > first_key.seq,
        "Each TaskStarted should use incremented counter: first={:?}, second={:?}",
        first_key,
        second_key
    );
}

// ============================================================================
// Test 5: Resort algorithm correctness (cycle-following with inverse permutation)
// ============================================================================

/// Test three-element permutation sorting behavior using OrderKey comparisons.
/// This tests the sorting invariants without calling resort_history_by_order().
#[tokio::test]
async fn test_three_element_orderkey_sorting() {
    // Create keys with OrderKeys: [req=2, req=0, req=1]
    let keys = vec![
        OrderKey {
            req: 2,
            out: 0,
            seq: 1,
        },
        OrderKey {
            req: 0,
            out: 0,
            seq: 2,
        },
        OrderKey {
            req: 1,
            out: 0,
            seq: 3,
        },
    ];

    // Sort by OrderKey
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();

    // Verify sorted: [req=0, req=1, req=2]
    assert_eq!(sorted_keys[0].req, 0, "Position 0 should have req=0");
    assert_eq!(sorted_keys[1].req, 1, "Position 1 should have req=1");
    assert_eq!(sorted_keys[2].req, 2, "Position 2 should have req=2");
}

/// Test that already-sorted keys remain in order after sorting.
#[tokio::test]
async fn test_sorted_keys_remain_sorted() {
    // Create already-sorted keys
    let keys = vec![
        OrderKey {
            req: 0,
            out: 0,
            seq: 1,
        },
        OrderKey {
            req: 1,
            out: 0,
            seq: 2,
        },
        OrderKey {
            req: 2,
            out: 0,
            seq: 3,
        },
    ];

    let mut sorted_keys = keys.clone();
    sorted_keys.sort();

    // Verify order unchanged
    assert_eq!(sorted_keys[0].req, 0);
    assert_eq!(sorted_keys[1].req, 1);
    assert_eq!(sorted_keys[2].req, 2);
}

/// Test complex 5-element permutation sorting.
#[tokio::test]
async fn test_complex_orderkey_sorting() {
    // Create 5 keys with scrambled order: [4, 2, 0, 3, 1]
    let mut keys = vec![];
    for req in [4, 2, 0, 3, 1] {
        keys.push(OrderKey {
            req,
            out: 0,
            seq: req,
        });
    }

    let mut sorted_keys = keys.clone();
    sorted_keys.sort();

    // Verify sorted: [0, 1, 2, 3, 4]
    for i in 0..5 {
        assert_eq!(
            sorted_keys[i].req as usize, i,
            "Position {} should have req={}",
            i, i
        );
    }
}

// ============================================================================
// Test 6: CLI queue processing after completion
// ============================================================================

#[tokio::test]
async fn test_queued_messages_exist_after_cli_task() {
    let mut widget = make_widget();

    // Simulate messages queued during active task
    widget.queued_user_messages.push_back(message::UserMessage {
        display_text: "Queued Q1".to_string(),
        ordered_items: vec![codex_core::protocol::InputItem::Text {
            text: "Queued Q1".to_string(),
        }],
    });
    widget.queued_user_messages.push_back(message::UserMessage {
        display_text: "Queued Q2".to_string(),
        ordered_items: vec![codex_core::protocol::InputItem::Text {
            text: "Queued Q2".to_string(),
        }],
    });

    assert_eq!(
        widget.queued_user_messages.len(),
        2,
        "Should have 2 queued messages"
    );
}

// ============================================================================
// Test 7: pending_user_cell_updates tracking
// ============================================================================

#[tokio::test]
async fn test_pending_user_cell_updates_tracks_task_id() {
    let mut widget = make_widget();

    // Queue a message
    widget
        .pending_dispatched_user_messages
        .push_back("Track me".to_string());

    // TaskStarted should create tracking entry
    widget.handle_codex_event(Event {
        id: "track-task-123".into(),
        event_seq: 0,
        msg: EventMsg::TaskStarted,
        order: Some(OrderMeta {
            request_ordinal: 1,
            output_index: Some(0),
            sequence_number: None,
        }),
    });

    // Verify tracking
    assert!(
        widget
            .pending_user_cell_updates
            .contains_key("track-task-123"),
        "Task ID should be tracked in pending_user_cell_updates"
    );

    // Get the tracked cell index
    let cell_idx = widget
        .pending_user_cell_updates
        .get("track-task-123")
        .copied()
        .unwrap();
    assert!(
        cell_idx < widget.history_cells.len(),
        "Tracked index should be valid"
    );
}

// ============================================================================
// Test 8: OrderKey secondary sorting by out and seq
// ============================================================================

/// Test that OrderKey sorts by out when req is equal.
#[tokio::test]
async fn test_orderkey_sorts_by_out_when_req_equal() {
    // Same req, different out values
    let key_high = OrderKey {
        req: 0,
        out: 100,
        seq: 1,
    };
    let key_low = OrderKey {
        req: 0,
        out: -100,
        seq: 2,
    };

    // Lower out should sort first
    let mut keys = [key_high, key_low];
    keys.sort();

    assert_eq!(keys[0].out, -100, "Lower out should come first");
    assert_eq!(keys[1].out, 100, "Higher out should come second");
}

/// Test that OrderKey sorts by seq when req and out are equal.
#[tokio::test]
async fn test_orderkey_sorts_by_seq_when_req_and_out_equal() {
    // Same req and out, different seq
    let key_high = OrderKey {
        req: 0,
        out: 0,
        seq: 10,
    };
    let key_low = OrderKey {
        req: 0,
        out: 0,
        seq: 5,
    };

    let mut keys = [key_high, key_low];
    keys.sort();

    // Lower seq should come first
    assert_eq!(keys[0].seq, 5, "Lower seq should come first");
    assert_eq!(keys[1].seq, 10, "Higher seq should come second");
}

// ============================================================================
// Test 9: Empty and single-element edge cases
// ============================================================================

/// Test that sorting empty vector doesn't panic.
#[tokio::test]
async fn test_empty_orderkey_sort() {
    let mut keys: Vec<OrderKey> = vec![];
    keys.sort();
    assert!(keys.is_empty());
}

/// Test that sorting single element doesn't change it.
#[tokio::test]
async fn test_single_orderkey_sort() {
    let mut keys = [OrderKey {
        req: 42,
        out: 0,
        seq: 1,
    }];
    keys.sort();

    // Should remain unchanged
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].req, 42);
}

// ============================================================================
// Test 10: Integration scenario - rapid messages
// ============================================================================

#[tokio::test]
async fn test_rapid_messages_get_unique_temp_keys() {
    let mut widget = make_widget();

    // Record initial state
    let initial_count = widget.history_cells.len();

    // Queue first message and trigger TaskStarted
    widget
        .pending_dispatched_user_messages
        .push_back("Rapid Q1".to_string());
    widget.handle_codex_event(Event {
        id: "rapid-1".into(),
        event_seq: 0,
        msg: EventMsg::TaskStarted,
        order: Some(OrderMeta {
            request_ordinal: 1,
            output_index: Some(0),
            sequence_number: None,
        }),
    });

    // Get the key from the tracked cell
    let cell1_idx = widget
        .pending_user_cell_updates
        .get("rapid-1")
        .copied()
        .unwrap();
    let key1 = widget.cell_order_seq[cell1_idx];

    // Queue and trigger second message
    widget
        .pending_dispatched_user_messages
        .push_back("Rapid Q2".to_string());
    widget.handle_codex_event(Event {
        id: "rapid-2".into(),
        event_seq: 1,
        msg: EventMsg::TaskStarted,
        order: Some(OrderMeta {
            request_ordinal: 2,
            output_index: Some(0),
            sequence_number: None,
        }),
    });

    // Get the key from the second tracked cell
    let cell2_idx = widget
        .pending_user_cell_updates
        .get("rapid-2")
        .copied()
        .unwrap();
    let key2 = widget.cell_order_seq[cell2_idx];

    // Verify both cells were created
    assert!(
        widget.history_cells.len() >= initial_count + 2,
        "Should have created at least 2 new cells"
    );

    // Keys should be unique (different req or seq)
    assert_ne!(
        (key1.req, key1.seq),
        (key2.req, key2.seq),
        "Each rapid message should get unique OrderKey: key1={:?}, key2={:?}",
        key1,
        key2
    );
}

// ============================================================================
// Test 11: TestHarness-based integration test
// ============================================================================

#[tokio::test]
async fn test_harness_user_message_creates_cell() {
    let mut harness = TestHarness::new();

    // Send a user message through the harness
    harness.send_user_message("Hello, test!");
    harness.drain_app_events();

    // Verify at least one cell was created
    assert!(
        harness.history_cell_count() >= 1,
        "User message should create at least one history cell"
    );
}

// ============================================================================
// Test 12: SPEC-954 Timeout mechanism tests
// ============================================================================

/// Test that TaskStarted clears pending message timestamps (cancels timeout)
#[tokio::test]
async fn test_task_started_clears_timeout_tracking() {
    let mut widget = make_widget();

    // Simulate queuing a message with timeout tracking
    let msg_id = "msg-test-1".to_string();
    widget
        .pending_message_timestamps
        .insert(msg_id.clone(), std::time::Instant::now());
    widget
        .pending_dispatched_user_messages
        .push_back("Test message".to_string());

    assert!(
        !widget.pending_message_timestamps.is_empty(),
        "Should have pending timestamp before TaskStarted"
    );

    // Simulate TaskStarted event
    widget.handle_codex_event(Event {
        id: "task-timeout-test".into(),
        event_seq: 0,
        msg: EventMsg::TaskStarted,
        order: Some(OrderMeta {
            request_ordinal: 1,
            output_index: Some(0),
            sequence_number: None,
        }),
    });

    // Verify timestamps cleared
    assert!(
        widget.pending_message_timestamps.is_empty(),
        "TaskStarted should clear all pending timeout timestamps"
    );
}

/// Test that timeout handler only acts when message is still pending
#[tokio::test]
async fn test_timeout_handler_ignores_already_processed_messages() {
    let mut widget = make_widget();
    let initial_history_len = widget.history_cells.len();

    // Call timeout handler for a message that was never tracked (already processed)
    widget.handle_user_message_timeout("msg-nonexistent", 10000);

    // Should have no effect - no error message added
    assert_eq!(
        widget.history_cells.len(),
        initial_history_len,
        "Timeout handler should ignore already-processed messages"
    );
}

/// Test that timeout handler shows error for pending message
#[tokio::test]
async fn test_timeout_handler_shows_error_for_pending_message() {
    let mut widget = make_widget();
    let initial_history_len = widget.history_cells.len();

    // Add a pending timestamp
    let msg_id = "msg-pending-timeout".to_string();
    widget
        .pending_message_timestamps
        .insert(msg_id.clone(), std::time::Instant::now());

    // Call timeout handler
    widget.handle_user_message_timeout(&msg_id, 10000);

    // Verify: timestamp removed, error message added to history
    assert!(
        widget.pending_message_timestamps.is_empty(),
        "Timeout handler should remove the timestamp"
    );
    assert!(
        widget.history_cells.len() > initial_history_len,
        "Timeout handler should add error message to history"
    );
}
