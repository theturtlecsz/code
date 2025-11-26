#![allow(clippy::unwrap_used)]

use codex_core::CodexAuth;
use codex_core::ConversationManager;
use codex_core::ModelProviderInfo;
use codex_core::TOOL_CANDIDATES;
use codex_core::built_in_model_providers;
use codex_core::model_family::find_family_for_model;
use codex_core::protocol::EventMsg;
use codex_core::protocol::InputItem;
use codex_core::protocol::Op;
use codex_core::shell::Shell;
use codex_core::shell::default_user_shell;
use core_test_support::load_default_config_for_test;
use core_test_support::load_sse_fixture_with_id;
use core_test_support::wait_for_event;
use os_info::Type as OsType;
use os_info::Version;
use tempfile::TempDir;
use which::which;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

fn text_user_input(text: String) -> serde_json::Value {
    serde_json::json!({
        "type": "message",
        "role": "user",
        "content": [ { "type": "input_text", "text": text } ]
    })
}

fn render_env_context(
    cwd: Option<String>,
    approval_policy: Option<&str>,
    sandbox_mode: Option<&str>,
    network_access: Option<&str>,
    writable_roots: Vec<String>,
    shell_name: Option<String>,
) -> String {
    let mut lines = vec!["<environment_context>".to_string()];
    if let Some(cwd) = cwd {
        lines.push(format!("  <cwd>{cwd}</cwd>"));
    }
    if let Some(approval_policy) = approval_policy {
        lines.push(format!(
            "  <approval_policy>{approval_policy}</approval_policy>"
        ));
    }
    if let Some(sandbox_mode) = sandbox_mode {
        lines.push(format!("  <sandbox_mode>{sandbox_mode}</sandbox_mode>"));
    }
    if let Some(network_access) = network_access {
        lines.push(format!(
            "  <network_access>{network_access}</network_access>"
        ));
    }
    if !writable_roots.is_empty() {
        lines.push("  <writable_roots>".to_string());
        for root in writable_roots {
            lines.push(format!("    <root>{root}</root>"));
        }
        lines.push("  </writable_roots>".to_string());
    }
    if let Some(os_block) = operating_system_block() {
        lines.push(os_block);
    }
    if let Some(tools_block) = common_tools_block() {
        lines.push(tools_block);
    }
    if let Some(shell_name) = shell_name {
        lines.push(format!("  <shell>{shell_name}</shell>"));
    }
    lines.push("</environment_context>".to_string());
    lines.join("\n")
}

fn operating_system_block() -> Option<String> {
    let info = os_info::get();
    let family = match info.os_type() {
        OsType::Unknown => None,
        other => Some(other.to_string()),
    };
    let version = match info.version() {
        Version::Unknown => None,
        other => {
            let text = other.to_string();
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
    };
    let architecture = {
        let arch = std::env::consts::ARCH;
        if arch.is_empty() {
            None
        } else {
            Some(arch.to_string())
        }
    };

    if family.is_none() && version.is_none() && architecture.is_none() {
        return None;
    }

    let mut lines = vec!["  <operating_system>".to_string()];
    if let Some(family) = family {
        lines.push(format!("    <family>{family}</family>"));
    }
    if let Some(version) = version {
        lines.push(format!("    <version>{version}</version>"));
    }
    if let Some(architecture) = architecture {
        lines.push(format!("    <architecture>{architecture}</architecture>"));
    }
    lines.push("  </operating_system>".to_string());
    Some(lines.join("\n"))
}

fn common_tools_block() -> Option<String> {
    let mut available = Vec::new();
    for candidate in TOOL_CANDIDATES {
        let detection_names = if candidate.detection_names.is_empty() {
            &[candidate.label][..]
        } else {
            candidate.detection_names
        };
        if detection_names.iter().any(|name| which(name).is_ok()) {
            available.push(candidate.label);
        }
    }
    if available.is_empty() {
        return None;
    }

    let mut lines = vec!["  <common_tools>".to_string()];
    for tool in available {
        lines.push(format!("    <tool>{tool}</tool>"));
    }
    lines.push("  </common_tools>".to_string());
    Some(lines.join("\n"))
}

fn default_env_context_str(cwd: &str, shell: &Shell) -> String {
    let shell_name = shell.name();
    render_env_context(
        Some(cwd.to_string()),
        Some("on-request"),
        Some("read-only"),
        Some("restricted"),
        Vec::new(),
        shell_name,
    )
}

/// Build minimal SSE stream with completed marker using the JSON fixture.
fn sse_completed(id: &str) -> String {
    load_sse_fixture_with_id("tests/fixtures/completed_template.json", id)
}

fn assert_tool_names(body: &serde_json::Value, expected_names: &[&str]) {
    assert_eq!(
        body["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>(),
        expected_names
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn codex_mini_latest_tools() {
    use pretty_assertions::assert_eq;

    let server = MockServer::start().await;

    let sse = sse_completed("resp");
    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse, "text/event-stream");

    // Expect two POSTs to /v1/responses
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(template)
        .expect(2)
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let cwd = TempDir::new().unwrap();
    let codex_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&codex_home);
    config.cwd = cwd.path().to_path_buf();
    config.model_provider = model_provider;
    config.user_instructions = Some("be consistent and helpful".to_string());

    let conversation_manager =
        ConversationManager::with_auth(CodexAuth::from_api_key("Test API Key"));
    config.include_apply_patch_tool = false;
    config.model = "codex-mini-latest".to_string();
    config.model_family = find_family_for_model("codex-mini-latest").unwrap();

    let codex = conversation_manager
        .new_conversation(config)
        .await
        .expect("create new conversation")
        .conversation;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "hello 1".into(),
            }],
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "hello 2".into(),
            }],
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2, "expected two POST requests");

    let expected_instructions = [
        include_str!("../../prompt.md"),
        include_str!("../../../apply-patch/apply_patch_tool_instructions.md"),
    ]
    .join("\n");

    let body0 = requests[0].body_json::<serde_json::Value>().unwrap();
    assert_eq!(
        body0["instructions"],
        serde_json::json!(expected_instructions),
    );
    let body1 = requests[1].body_json::<serde_json::Value>().unwrap();
    assert_eq!(
        body1["instructions"],
        serde_json::json!(expected_instructions),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn prompt_tools_are_consistent_across_requests() {
    use pretty_assertions::assert_eq;

    let server = MockServer::start().await;

    let sse = sse_completed("resp");
    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse, "text/event-stream");

    // Expect two POSTs to /v1/responses
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(template)
        .expect(2)
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let cwd = TempDir::new().unwrap();
    let codex_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&codex_home);
    config.cwd = cwd.path().to_path_buf();
    config.model_provider = model_provider;
    config.user_instructions = Some("be consistent and helpful".to_string());
    config.include_apply_patch_tool = true;
    config.include_plan_tool = true;

    let conversation_manager =
        ConversationManager::with_auth(CodexAuth::from_api_key("Test API Key"));
    let expected_instructions = config.model_family.base_instructions.clone();
    let codex = conversation_manager
        .new_conversation(config)
        .await
        .expect("create new conversation")
        .conversation;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "hello 1".into(),
            }],
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "hello 2".into(),
            }],
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2, "expected two POST requests");

    // our internal implementation is responsible for keeping tools in sync
    // with the OpenAI schema, so we just verify the tool presence here
    let expected_tools_names: &[&str] = &["shell", "update_plan", "apply_patch", "view_image"];
    let body0 = requests[0].body_json::<serde_json::Value>().unwrap();
    assert_eq!(
        body0["instructions"],
        serde_json::json!(expected_instructions),
    );
    assert_tool_names(&body0, expected_tools_names);

    let body1 = requests[1].body_json::<serde_json::Value>().unwrap();
    assert_eq!(
        body1["instructions"],
        serde_json::json!(expected_instructions),
    );
    assert_tool_names(&body1, expected_tools_names);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prefixes_context_and_instructions_once_and_consistently_across_requests() {
    use pretty_assertions::assert_eq;

    let server = MockServer::start().await;

    let sse = sse_completed("resp");
    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse, "text/event-stream");

    // Expect two POSTs to /v1/responses
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(template)
        .expect(2)
        .mount(&server)
        .await;

    let model_provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers()["openai"].clone()
    };

    let cwd = TempDir::new().unwrap();
    let codex_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&codex_home);
    config.cwd = cwd.path().to_path_buf();
    config.model_provider = model_provider;
    config.user_instructions = Some("be consistent and helpful".to_string());

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
                text: "hello 1".into(),
            }],
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    codex
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "hello 2".into(),
            }],
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TaskComplete(_))).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2, "expected two POST requests");

    let shell = default_user_shell().await;

    let expected_env_text = render_env_context(
        Some(cwd.path().to_string_lossy().to_string()),
        Some("on-request"),
        Some("read-only"),
        Some("restricted"),
        Vec::new(),
        shell.name(),
    );
    let expected_ui_text =
        "<user_instructions>\n\nbe consistent and helpful\n\n</user_instructions>";

    let expected_env_msg = serde_json::json!({
        "type": "message",
        "id": serde_json::Value::Null,
        "role": "user",
        "content": [ { "type": "input_text", "text": expected_env_text } ]
    });
    let expected_ui_msg = serde_json::json!({
        "type": "message",
        "id": serde_json::Value::Null,
        "role": "user",
        "content": [ { "type": "input_text", "text": expected_ui_text } ]
    });

    let expected_user_message_1 = serde_json::json!({
        "type": "message",
        "id": serde_json::Value::Null,
        "role": "user",
        "content": [ { "type": "input_text", "text": "hello 1" } ]
    });
    let body1 = requests[0].body_json::<serde_json::Value>().unwrap();
    assert_eq!(
        body1["input"],
        serde_json::json!([expected_ui_msg, expected_env_msg, expected_user_message_1])
    );

    let expected_user_message_2 = serde_json::json!({
        "type": "message",
        "id": serde_json::Value::Null,
        "role": "user",
        "content": [ { "type": "input_text", "text": "hello 2" } ]
    });
    let body2 = requests[1].body_json::<serde_json::Value>().unwrap();
    let expected_body2 = serde_json::json!(
        [
            body1["input"].as_array().unwrap().as_slice(),
            [expected_user_message_2].as_slice(),
        ]
        .concat()
    );
    assert_eq!(body2["input"], expected_body2);
}

/// SPEC-957: Op::OverrideTurnContext was removed - test stubbed for compilation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::OverrideTurnContext was removed"]
async fn overrides_turn_context_but_keeps_cached_prefix_and_key_constant() {
    unimplemented!("SPEC-957: Op::OverrideTurnContext was removed from the protocol");
}

/// SPEC-957: Op::UserTurn was removed - test stubbed for compilation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::UserTurn was removed"]
async fn per_turn_overrides_keep_cached_prefix_and_key_constant() {
    unimplemented!("SPEC-957: Op::UserTurn was removed from the protocol");
}

/// SPEC-957: Op::UserTurn was removed - test stubbed for compilation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::UserTurn was removed"]
async fn send_user_turn_with_no_changes_does_not_send_environment_context() {
    unimplemented!("SPEC-957: Op::UserTurn was removed from the protocol");
}

/// SPEC-957: Op::UserTurn was removed - test stubbed for compilation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::UserTurn was removed"]
async fn send_user_turn_with_changes_sends_environment_context() {
    unimplemented!("SPEC-957: Op::UserTurn was removed from the protocol");
}
