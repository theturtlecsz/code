// SPEC-957: These tests require Op::OverrideTurnContext which exists in codex_protocol::protocol::Op
// but is NOT exposed in codex_core::protocol::Op. The core crate has a different Op enum subset.
// Tests cannot be restored without API changes to expose these variants.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::OverrideTurnContext not exposed in codex_core::protocol::Op"]
async fn override_turn_context_does_not_persist_when_config_exists() {
    // Original test verified Op::OverrideTurnContext doesn't persist model changes to config.toml
    // Requires codex_protocol::protocol::Op::OverrideTurnContext which is not in codex_core
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "SPEC-957: Op::OverrideTurnContext not exposed in codex_core::protocol::Op"]
async fn override_turn_context_does_not_create_config_file() {
    // Original test verified Op::OverrideTurnContext doesn't create config.toml
    // Requires codex_protocol::protocol::Op::OverrideTurnContext which is not in codex_core
}
