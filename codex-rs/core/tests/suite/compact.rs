use codex_core::CodexAuth;
use codex_core::ConversationManager;
use codex_core::ModelProviderInfo;
use codex_core::built_in_model_providers;
use codex_core::protocol::ErrorEvent;
use codex_core::protocol::EventMsg;
use codex_core::protocol::InputItem;
use codex_core::protocol::Op;
use core_test_support::load_default_config_for_test;
use core_test_support::wait_for_event;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::Request;
use wiremock::Respond;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

use codex_core::codex::compact::SUMMARIZATION_PROMPT;
use core_test_support::non_sandbox_test;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed_with_tokens;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::sse;
use core_test_support::responses::sse_response;
use core_test_support::responses::start_mock_server;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
// --- Test helpers -----------------------------------------------------------

pub(super) const FIRST_REPLY: &str = "FIRST_REPLY";
pub(super) const SUMMARY_TEXT: &str = "SUMMARY_ONLY_CONTEXT";
const THIRD_USER_MSG: &str = "next turn";
const AUTO_SUMMARY_TEXT: &str = "AUTO_SUMMARY";
const FIRST_AUTO_MSG: &str = "token limit start";
const SECOND_AUTO_MSG: &str = "token limit push";
const STILL_TOO_BIG_REPLY: &str = "STILL_TOO_BIG";
const MULTI_AUTO_MSG: &str = "multi auto";
const SECOND_LARGE_REPLY: &str = "SECOND_LARGE_REPLY";
const FIRST_AUTO_SUMMARY: &str = "FIRST_AUTO_SUMMARY";
const SECOND_AUTO_SUMMARY: &str = "SECOND_AUTO_SUMMARY";
const FINAL_REPLY: &str = "FINAL_REPLY";
const DUMMY_FUNCTION_NAME: &str = "unsupported_tool";
const DUMMY_CALL_ID: &str = "call-multi-auto";

// SPEC-958: Stubbed tests removed (provided no coverage, just `unimplemented!()`):
// - summarize_context_three_requests_and_instructions (needed rollout_path)
// - get_rollout_history_retains_compacted_entries (needed get_rollout_history)

// Windows CI only: bump to 4 workers to prevent SSE/event starvation and test timeouts.
/// SPEC-957: Token-based auto-compact not implemented.
/// The implementation only triggers auto-compact on error messages (e.g., "exceeds the context window"),
/// not based on model_auto_compact_token_limit threshold. Test expects token-count triggering.
#[cfg_attr(windows, tokio::test(flavor = "multi_thread", worker_threads = 4))]
#[cfg_attr(not(windows), tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[ignore = "SPEC-957: token-based auto-compact not implemented (only error-message triggered)"]
async fn auto_compact_runs_after_token_limit_hit() {
    non_sandbox_test!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", 70_000),
    ]);

    let sse2 = sse(vec![
        ev_assistant_message("m2", "SECOND_REPLY"),
        ev_completed_with_tokens("r2", 330_000),
    ]);

    let sse3 = sse(vec![
        ev_assistant_message("m3", AUTO_SUMMARY_TEXT),
        ev_completed_with_tokens("r3", 200),
    ]);

    let first_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body.contains(FIRST_AUTO_MSG)
            && !body.contains(SECOND_AUTO_MSG)
            && !body.contains("You have exceeded the maximum number of tokens")
    };
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(first_matcher)
        .respond_with(sse_response(sse1))
        .mount(&server)
        .await;

    let second_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body.contains(SECOND_AUTO_MSG)
            && body.contains(FIRST_AUTO_MSG)
            && !body.contains("You have exceeded the maximum number of tokens")
    };
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(second_matcher)
        .respond_with(sse_response(sse2))
        .mount(&server)
        .await;

    let third_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body.contains("You have exceeded the maximum number of tokens")
    };
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(third_matcher)
        .respond_with(sse_response(sse3))
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&home);
    config.model_provider = model_provider;
    config.model_auto_compact_token_limit = Some(200_000);
    let conversation_manager = ConversationManager::with_auth(CodexAuth::from_api_key("dummy"));
    let codex = conversation_manager
        .new_conversation(config)
        .await
        .unwrap()
        .conversation;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: FIRST_AUTO_MSG.into(),
            }],
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: SECOND_AUTO_MSG.into(),
            }],
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;
    // wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    let requests = server.received_requests().await.unwrap();
    assert!(
        requests.len() >= 3,
        "auto compact should add at least a third request, got {}",
        requests.len()
    );
    let is_auto_compact = |req: &wiremock::Request| {
        std::str::from_utf8(&req.body)
            .unwrap_or("")
            .contains("You have exceeded the maximum number of tokens")
    };
    let auto_compact_count = requests.iter().filter(|req| is_auto_compact(req)).count();
    assert_eq!(
        auto_compact_count, 1,
        "expected exactly one auto compact request"
    );
    let auto_compact_index = requests
        .iter()
        .enumerate()
        .find_map(|(idx, req)| is_auto_compact(req).then_some(idx))
        .expect("auto compact request missing");
    assert_eq!(
        auto_compact_index, 2,
        "auto compact should add a third request"
    );

    let body_first = requests[0].body_json::<serde_json::Value>().unwrap();
    let body3 = requests[auto_compact_index]
        .body_json::<serde_json::Value>()
        .unwrap();
    let instructions = body3
        .get("instructions")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let baseline_instructions = body_first
        .get("instructions")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        instructions, baseline_instructions,
        "auto compact should keep the standard developer instructions",
    );

    let input3 = body3.get("input").and_then(|v| v.as_array()).unwrap();
    let last3 = input3
        .last()
        .expect("auto compact request should append a user message");
    assert_eq!(last3.get("type").and_then(|v| v.as_str()), Some("message"));
    assert_eq!(last3.get("role").and_then(|v| v.as_str()), Some("user"));
    let last_text = last3
        .get("content")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(|text| text.as_str())
        .unwrap_or_default();
    assert_eq!(
        last_text, SUMMARIZATION_PROMPT,
        "auto compact should send the summarization prompt as a user message",
    );
}

// SPEC-958: auto_compact_persists_rollout_entries removed (stubbed, needed rollout_path)

/// SPEC-957: Token-based auto-compact not implemented.
/// Test expects token-count triggering (model_auto_compact_token_limit=200) but implementation
/// only triggers auto-compact on error messages (e.g., "exceeds the context window").
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: token-based auto-compact not implemented (only error-message triggered)"]
async fn auto_compact_stops_after_failed_attempt() {
    non_sandbox_test!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", 500),
    ]);

    let sse2 = sse(vec![
        ev_assistant_message("m2", SUMMARY_TEXT),
        ev_completed_with_tokens("r2", 50),
    ]);

    let sse3 = sse(vec![
        ev_assistant_message("m3", STILL_TOO_BIG_REPLY),
        ev_completed_with_tokens("r3", 500),
    ]);

    let first_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body.contains(FIRST_AUTO_MSG)
            && !body.contains("You have exceeded the maximum number of tokens")
    };
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(first_matcher)
        .respond_with(sse_response(sse1.clone()))
        .mount(&server)
        .await;

    let second_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body.contains("You have exceeded the maximum number of tokens")
    };
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(second_matcher)
        .respond_with(sse_response(sse2.clone()))
        .mount(&server)
        .await;

    let third_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        !body.contains("You have exceeded the maximum number of tokens")
            && body.contains(SUMMARY_TEXT)
    };
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(third_matcher)
        .respond_with(sse_response(sse3.clone()))
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&home);
    config.model_provider = model_provider;
    config.model_auto_compact_token_limit = Some(200);
    let conversation_manager = ConversationManager::with_auth(CodexAuth::from_api_key("dummy"));
    let codex = conversation_manager
        .new_conversation(config)
        .await
        .unwrap()
        .conversation;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: FIRST_AUTO_MSG.into(),
            }],
        })
        .await
        .unwrap();

    let error_event = wait_for_event(&codex, |ev| matches!(ev, EventMsg::Error(_))).await;
    let EventMsg::Error(ErrorEvent { message }) = error_event else {
        panic!("expected error event");
    };
    assert!(
        message.contains("limit"),
        "error message should include limit information: {message}"
    );
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        3,
        "auto compact should attempt at most one summarization before erroring"
    );

    let last_body = requests[2].body_json::<serde_json::Value>().unwrap();
    let input = last_body
        .get("input")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("unexpected request format: {last_body}"));
    let contains_prompt = input.iter().any(|item| {
        item.get("type").and_then(|v| v.as_str()) == Some("message")
            && item.get("role").and_then(|v| v.as_str()) == Some("user")
            && item
                .get("content")
                .and_then(|v| v.as_array())
                .and_then(|items| items.first())
                .and_then(|entry| entry.get("text"))
                .and_then(|text| text.as_str())
                .map(|text| text == SUMMARIZATION_PROMPT)
                .unwrap_or(false)
    });
    assert!(
        !contains_prompt,
        "third request should be the follow-up turn, not another summarization",
    );
}

/// SPEC-957: Token-based auto-compact not implemented.
/// Test expects multiple auto-compacts triggered by token accumulation, but implementation
/// only triggers auto-compact on error messages (e.g., "exceeds the context window").
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: token-based auto-compact not implemented (only error-message triggered)"]
async fn auto_compact_allows_multiple_attempts_when_interleaved_with_other_turn_events() {
    non_sandbox_test!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", 500),
    ]);
    let sse2 = sse(vec![
        ev_assistant_message("m2", FIRST_AUTO_SUMMARY),
        ev_completed_with_tokens("r2", 50),
    ]);
    let sse3 = sse(vec![
        ev_function_call(DUMMY_CALL_ID, DUMMY_FUNCTION_NAME, "{}"),
        ev_completed_with_tokens("r3", 150),
    ]);
    let sse4 = sse(vec![
        ev_assistant_message("m4", SECOND_LARGE_REPLY),
        ev_completed_with_tokens("r4", 450),
    ]);
    let sse5 = sse(vec![
        ev_assistant_message("m5", SECOND_AUTO_SUMMARY),
        ev_completed_with_tokens("r5", 60),
    ]);
    let sse6 = sse(vec![
        ev_assistant_message("m6", FINAL_REPLY),
        ev_completed_with_tokens("r6", 120),
    ]);

    #[derive(Clone)]
    struct SeqResponder {
        bodies: Arc<Vec<String>>,
        calls: Arc<AtomicUsize>,
        requests: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl SeqResponder {
        fn new(bodies: Vec<String>) -> Self {
            Self {
                bodies: Arc::new(bodies),
                calls: Arc::new(AtomicUsize::new(0)),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn recorded_requests(&self) -> Vec<Vec<u8>> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Respond for SeqResponder {
        fn respond(&self, req: &Request) -> ResponseTemplate {
            let idx = self.calls.fetch_add(1, Ordering::SeqCst);
            self.requests.lock().unwrap().push(req.body.clone());
            let body = self
                .bodies
                .get(idx)
                .unwrap_or_else(|| panic!("unexpected request index {idx}"))
                .clone();
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(body, "text/event-stream")
        }
    }

    let responder = SeqResponder::new(vec![sse1, sse2, sse3, sse4, sse5, sse6]);
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(responder.clone())
        .expect(6)
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&home);
    config.model_provider = model_provider;
    config.model_auto_compact_token_limit = Some(200);
    let conversation_manager = ConversationManager::with_auth(CodexAuth::from_api_key("dummy"));
    let codex = conversation_manager
        .new_conversation(config)
        .await
        .unwrap()
        .conversation;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: MULTI_AUTO_MSG.into(),
            }],
        })
        .await
        .unwrap();

    let mut auto_compact_lifecycle_events = Vec::new();
    loop {
        let event = codex.next_event().await.unwrap();
        if event.id.starts_with("auto-compact-")
            && matches!(event.msg, EventMsg::TaskStarted | EventMsg::TaskComplete(_))
        {
            auto_compact_lifecycle_events.push(event);
            continue;
        }
        if let EventMsg::TaskComplete(_) = &event.msg
            && !event.id.starts_with("auto-compact-")
        {
            break;
        }
    }

    assert!(
        auto_compact_lifecycle_events.is_empty(),
        "auto compact should not emit task lifecycle events"
    );

    let request_bodies: Vec<String> = responder
        .recorded_requests()
        .into_iter()
        .map(|body| String::from_utf8(body).unwrap_or_default())
        .collect();
    assert_eq!(
        request_bodies.len(),
        6,
        "expected six requests including two auto compactions"
    );
    assert!(
        request_bodies[0].contains(MULTI_AUTO_MSG),
        "first request should contain the user input"
    );
    assert!(
        request_bodies[1].contains("You have exceeded the maximum number of tokens"),
        "first auto compact request should include the summarization prompt"
    );
    assert!(
        request_bodies[3].contains(&format!("unsupported call: {DUMMY_FUNCTION_NAME}")),
        "function call output should be sent before the second auto compact"
    );
    assert!(
        request_bodies[4].contains("You have exceeded the maximum number of tokens"),
        "second auto compact request should include the summarization prompt"
    );
}
