// SPEC-958 Phase 2: Op::OverrideTurnContext now exposed in codex_core::protocol::Op
// but has PARTIAL implementation (overrides are logged but not persisted to Session).
//
// Tests that need full implementation:
// - override_turn_context_does_not_persist_when_config_exists
// - override_turn_context_does_not_create_config_file
//
// Full implementation requires making Session fields (cwd, approval_policy, sandbox_policy)
// mutable via RwLock. This is a separate architectural change.
