//! Tests for JSON structured output via ConfigureSession.output_schema
//!
//! SPEC-958 Session 8: Restored using ConfigureSession.output_schema pattern.
//! Previously these tests used Op::UserTurn.final_output_json_schema which is
//! only available in codex_protocol::protocol::Op (external wire protocol).

#![allow(clippy::unwrap_used)]

use codex_core::CodexAuth;
use codex_core::ConversationManager;
use codex_core::ModelProviderInfo;
use codex_core::built_in_model_providers;
use codex_core::protocol::EventMsg;
use codex_core::protocol::InputItem;
use codex_core::protocol::Op;
use core_test_support::load_default_config_for_test;
use core_test_support::load_sse_fixture_with_id;
use core_test_support::non_sandbox_test;
use core_test_support::wait_for_event;
use serde_json::json;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

/// Build minimal SSE stream with completed marker using the JSON fixture.
fn sse_completed(id: &str) -> String {
    load_sse_fixture_with_id("tests/fixtures/completed_template.json", id)
}

/// Verify that ConfigureSession.output_schema is sent to the API as json_schema text format.
/// This tests the gpt-5 model which supports structured JSON output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_output_schema_sends_json_schema_format_for_gpt5() {
    non_sandbox_test!();

    let server = MockServer::start().await;

    let sse = sse_completed("resp1");
    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse, "text/event-stream");

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(template)
        .expect(1)
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let output_schema = json!({
        "type": "object",
        "properties": {
            "answer": { "type": "string" },
            "confidence": { "type": "number" }
        },
        "required": ["answer"],
        "additionalProperties": false
    });

    let codex_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&codex_home);
    config.cwd = codex_home.path().to_path_buf();
    config.model_provider = model_provider;
    config.model = "gpt-5".to_string();
    config.output_schema = Some(output_schema.clone());

    let conversation_manager =
        ConversationManager::with_auth(CodexAuth::from_api_key("Test API Key"));
    let codex = conversation_manager
        .new_conversation(config)
        .await
        .expect("create new conversation")
        .conversation;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "What is the answer?".into(),
            }],
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "expected exactly one request");

    let request_body = requests[0].body_json::<serde_json::Value>().unwrap();

    // The output_schema should be converted to text.format with json_schema type
    let text = request_body.get("text");
    assert!(
        text.is_some(),
        "request should include text field when output_schema is set"
    );

    let format = text.unwrap().get("format");
    assert!(
        format.is_some(),
        "request should include text.format field when output_schema is set"
    );

    let format_value = format.unwrap();
    assert_eq!(
        format_value
            .get("type")
            .and_then(serde_json::Value::as_str),
        Some("json_schema"),
        "format type should be json_schema"
    );
    assert_eq!(
        format_value
            .get("name")
            .and_then(serde_json::Value::as_str),
        Some("codex_output_schema"),
        "format name should be codex_output_schema"
    );
    assert_eq!(
        format_value
            .get("strict")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "format strict should be true"
    );
    assert_eq!(
        format_value.get("schema"),
        Some(&output_schema),
        "format schema should match the provided output_schema"
    );
}

/// Verify that output_schema works with gpt-5-codex model (the code-focused variant).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_output_schema_sends_json_schema_format_for_gpt5_codex() {
    non_sandbox_test!();

    let server = MockServer::start().await;

    let sse = sse_completed("resp2");
    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse, "text/event-stream");

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(template)
        .expect(1)
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let output_schema = json!({
        "type": "object",
        "properties": {
            "code": { "type": "string" },
            "language": { "type": "string" }
        },
        "required": ["code", "language"],
        "additionalProperties": false
    });

    let codex_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&codex_home);
    config.cwd = codex_home.path().to_path_buf();
    config.model_provider = model_provider;
    config.model = "gpt-5-codex".to_string();
    config.output_schema = Some(output_schema.clone());

    let conversation_manager =
        ConversationManager::with_auth(CodexAuth::from_api_key("Test API Key"));
    let codex = conversation_manager
        .new_conversation(config)
        .await
        .expect("create new conversation")
        .conversation;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "Write hello world in Python".into(),
            }],
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "expected exactly one request");

    let request_body = requests[0].body_json::<serde_json::Value>().unwrap();

    // The output_schema should be converted to text.format with json_schema type
    let text = request_body.get("text");
    assert!(
        text.is_some(),
        "request should include text field when output_schema is set"
    );

    let format = text.unwrap().get("format");
    assert!(
        format.is_some(),
        "request should include text.format field when output_schema is set"
    );

    let format_value = format.unwrap();
    assert_eq!(
        format_value
            .get("type")
            .and_then(serde_json::Value::as_str),
        Some("json_schema"),
        "format type should be json_schema"
    );
    assert_eq!(
        format_value.get("schema"),
        Some(&output_schema),
        "format schema should match the provided output_schema"
    );
}

/// Verify that when output_schema is NOT set, text.format is not included
/// (for non-ChatGPT auth).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_without_output_schema_omits_json_schema_format() {
    non_sandbox_test!();

    let server = MockServer::start().await;

    let sse = sse_completed("resp3");
    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse, "text/event-stream");

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(template)
        .expect(1)
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let codex_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&codex_home);
    config.cwd = codex_home.path().to_path_buf();
    config.model_provider = model_provider;
    config.model = "gpt-5".to_string();
    // Explicitly NOT setting output_schema

    let conversation_manager =
        ConversationManager::with_auth(CodexAuth::from_api_key("Test API Key"));
    let codex = conversation_manager
        .new_conversation(config)
        .await
        .expect("create new conversation")
        .conversation;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "Just a regular message".into(),
            }],
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "expected exactly one request");

    let request_body = requests[0].body_json::<serde_json::Value>().unwrap();

    // For gpt-5 family with API key auth, text field may be present for verbosity
    // but format should NOT be present when output_schema is not set
    if let Some(text) = request_body.get("text") {
        let format = text.get("format");
        assert!(
            format.is_none(),
            "text.format should not be present when output_schema is not set"
        );
    }
}
