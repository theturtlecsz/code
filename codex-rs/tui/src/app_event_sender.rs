use tokio::sync::mpsc::UnboundedSender;

use crate::app_event::{AppEvent, BackgroundPlacement};
use crate::session_log;

#[derive(Clone, Debug)]
pub(crate) struct AppEventSender {
    // High‑priority events (input, resize, redraw scheduling) are routed here.
    high_tx: UnboundedSender<AppEvent>,
    // Bulk/streaming events (history inserts, commit ticks, file search, etc.).
    bulk_tx: UnboundedSender<AppEvent>,
}

impl AppEventSender {
    /// Create a sender that splits events by priority across two channels.
    pub(crate) fn new_dual(high_tx: UnboundedSender<AppEvent>, bulk_tx: UnboundedSender<AppEvent>) -> Self {
        Self { high_tx, bulk_tx }
    }
    /// Backward‑compatible constructor for tests/fixtures that expect a single
    /// channel. Routes both high‑priority and bulk events to the same sender.
    #[allow(dead_code)]
    pub(crate) fn new(app_event_tx: UnboundedSender<AppEvent>) -> Self {
        Self {
            high_tx: app_event_tx.clone(),
            bulk_tx: app_event_tx,
        }
    }

    /// Send an event to the app event channel. If it fails, we swallow the
    /// error and log it.
    pub(crate) fn send(&self, event: AppEvent) {
        // Record inbound events for high-fidelity session replay.
        // Avoid double-logging Ops; those are logged at the point of submission.
        if !matches!(event, AppEvent::CodexOp(_)) {
            session_log::log_inbound_app_event(&event);
        }
        let is_high = matches!(
            event,
            AppEvent::KeyEvent(_)
                | AppEvent::MouseEvent(_)
                | AppEvent::Paste(_)
                | AppEvent::RequestRedraw
                | AppEvent::Redraw
                | AppEvent::ExitRequest
                | AppEvent::SetTerminalTitle { .. }
        );

        let tx = if is_high {
            &self.high_tx
        } else {
            &self.bulk_tx
        };
        if let Err(e) = tx.send(event) {
            tracing::error!("failed to send event: {e}");
        }
    }

    /// Emit a background event using the provided placement strategy. Defaults
    /// to appending at the end of the current history window.
    ///
    /// IMPORTANT: UI code should call this (or other history helpers) rather
    /// than constructing `Event { event_seq: 0, .. }` manually. Protocol events
    /// must come from `codex-core` via `Session::make_event` so the per-turn
    /// sequence stays consistent.
    pub(crate) fn send_background_event_with_placement(
        &self,
        message: impl Into<String>,
        placement: BackgroundPlacement,
    ) {
        self.send(AppEvent::InsertBackgroundEvent {
            message: message.into(),
            placement,
        });
    }

    /// Convenience: append a background event at the end of the history.
    pub(crate) fn send_background_event(&self, message: impl Into<String>) {
        self.send_background_event_with_placement(message, BackgroundPlacement::Tail);
    }

    /// Convenience: place a background event before the next provider/tool output.
    pub(crate) fn send_background_event_before_next_output(&self, message: impl Into<String>) {
        self.send_background_event_with_placement(message, BackgroundPlacement::BeforeNextOutput);
    }

    /// Signal CLI routing completion with response (SPEC-KIT-952)
    pub(crate) fn send_cli_route_complete(
        &self,
        provider_name: impl Into<String>,
        model_name: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) {
        self.send(AppEvent::CliRouteComplete {
            provider_name: provider_name.into(),
            model_name: model_name.into(),
            content: content.into(),
            is_error,
        });
    }

    /// Signal native provider streaming started (SPEC-KIT-953)
    pub(crate) fn send_native_stream_start(
        &self,
        provider_name: impl Into<String>,
        model_name: impl Into<String>,
        message_id: impl Into<String>,
    ) {
        self.send(AppEvent::NativeProviderStreamStart {
            provider_name: provider_name.into(),
            model_name: model_name.into(),
            message_id: message_id.into(),
        });
    }

    /// Signal native provider streaming text delta (SPEC-KIT-953)
    pub(crate) fn send_native_stream_delta(&self, text: impl Into<String>) {
        self.send(AppEvent::NativeProviderStreamDelta { text: text.into() });
    }

    /// Signal native provider streaming completed (SPEC-KIT-953)
    pub(crate) fn send_native_stream_complete(
        &self,
        provider_name: impl Into<String>,
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
    ) {
        self.send(AppEvent::NativeProviderStreamComplete {
            provider_name: provider_name.into(),
            input_tokens,
            output_tokens,
        });
    }

    /// Signal native provider streaming error (SPEC-KIT-953)
    pub(crate) fn send_native_stream_error(
        &self,
        provider_name: impl Into<String>,
        error: impl Into<String>,
    ) {
        self.send(AppEvent::NativeProviderStreamError {
            provider_name: provider_name.into(),
            error: error.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    //! Integration tests for AppEventSender behavior (SPEC-955)
    //!
    //! These tests document and validate the event system's behavior before and after
    //! the std::sync::mpsc → tokio::sync::mpsc refactor.
    //!
    //! They ensure that the refactor preserves core functionality:
    //! - Events are delivered in order
    //! - No events are lost
    //! - Background vs foreground placement works correctly
    //! - Multiple events can be sent and received

    use super::*;
    use tokio::sync::mpsc;

    /// Test basic event sender flow: create → send → receive → verify
    #[test]
    fn test_app_event_sender_basic_flow() {
        // Create a tokio unbounded channel
        let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);

        // Send a simple event
        sender.send(AppEvent::RequestRedraw);

        // Verify we can receive it (tokio try_recv is non-blocking)
        let received = rx.try_recv().expect("should receive event");
        match received {
            AppEvent::RequestRedraw => {
                // Success - event received as expected
            }
            other => panic!("Expected RequestRedraw, got {:?}", other),
        }
    }

    /// Test sending multiple events in sequence - order preservation
    #[test]
    fn test_app_event_sender_multiple_events() {
        let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);

        // Send multiple events in specific order
        sender.send(AppEvent::RequestRedraw);
        sender.send(AppEvent::ExitRequest);
        sender.send(AppEvent::Redraw);

        // Verify order is preserved
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert_eq!(events.len(), 3, "should receive all 3 events");

        match (&events[0], &events[1], &events[2]) {
            (AppEvent::RequestRedraw, AppEvent::ExitRequest, AppEvent::Redraw) => {
                // Success - order preserved
            }
            _ => panic!("Events received in wrong order: {:?}", events),
        }
    }

    /// Test background event placement
    #[test]
    fn test_app_event_background_placement() {
        let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);

        // Send background events with different placement strategies
        sender.send_background_event_with_placement("Tail event", BackgroundPlacement::Tail);
        sender.send_background_event_with_placement(
            "Before next output",
            BackgroundPlacement::BeforeNextOutput,
        );
        sender.send_background_event("Default (tail) event");

        // Verify all events were sent
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert_eq!(events.len(), 3, "should receive all 3 background events");

        // Verify they're all background events
        for event in &events {
            match event {
                AppEvent::InsertBackgroundEvent { message, placement } => {
                    // Verify message is not empty
                    assert!(!message.is_empty(), "message should not be empty");
                    // Verify placement is one of the two valid values
                    match placement {
                        BackgroundPlacement::Tail | BackgroundPlacement::BeforeNextOutput => {
                            // Success
                        }
                    }
                }
                other => panic!("Expected InsertBackgroundEvent, got {:?}", other),
            }
        }
    }

    /// Test that try_recv returns error when no events are available
    #[test]
    fn test_no_events_available() {
        let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
        let _sender = AppEventSender::new(tx);

        // Don't send any events

        // Verify try_recv returns error (not blocking)
        match rx.try_recv() {
            Err(mpsc::error::TryRecvError::Empty) => {
                // Success - channel is empty as expected
            }
            Ok(_) => panic!("Should not receive event when none sent"),
            Err(mpsc::error::TryRecvError::Disconnected) => panic!("Channel should not be disconnected"),
        }
    }

    /// Test that sender can be cloned and both clones work
    #[test]
    fn test_sender_clone() {
        let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
        let sender1 = AppEventSender::new(tx);
        let sender2 = sender1.clone();

        // Send from both senders
        sender1.send(AppEvent::RequestRedraw);
        sender2.send(AppEvent::ExitRequest);

        // Verify both events received
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert_eq!(events.len(), 2, "should receive events from both senders");
    }

    /// Test dual-channel setup (high-priority vs bulk events)
    #[test]
    fn test_dual_channel_setup() {
        let (high_tx, mut high_rx) = mpsc::unbounded_channel::<AppEvent>();
        let (bulk_tx, mut bulk_rx) = mpsc::unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new_dual(high_tx, bulk_tx);

        // Send high-priority event (KeyEvent)
        sender.send(AppEvent::KeyEvent(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        )));

        // Send bulk event (background event)
        sender.send_background_event("Test message");

        // Verify high-priority event went to high channel
        let mut high_events = Vec::new();
        while let Ok(event) = high_rx.try_recv() {
            high_events.push(event);
        }
        assert_eq!(high_events.len(), 1, "should receive 1 high-priority event");
        match &high_events[0] {
            AppEvent::KeyEvent(_) => {
                // Success
            }
            other => panic!("Expected KeyEvent, got {:?}", other),
        }

        // Verify bulk event went to bulk channel
        let mut bulk_events = Vec::new();
        while let Ok(event) = bulk_rx.try_recv() {
            bulk_events.push(event);
        }
        assert_eq!(bulk_events.len(), 1, "should receive 1 bulk event");
        match &bulk_events[0] {
            AppEvent::InsertBackgroundEvent { .. } => {
                // Success
            }
            other => panic!("Expected InsertBackgroundEvent, got {:?}", other),
        }
    }

    /// Test that after refactor, events still work the same way
    /// This test validates that the refactor preserves behavior
    #[test]
    fn test_event_system_refactor_compatibility() {
        // This test validates that the API works correctly with tokio channels
        let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);

        // Exercise the full API
        sender.send(AppEvent::RequestRedraw);
        sender.send_background_event("Test");
        sender.send_background_event_with_placement("Test2", BackgroundPlacement::Tail);

        // Verify all events received
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert_eq!(events.len(), 3, "all events should be delivered");
    }
}
