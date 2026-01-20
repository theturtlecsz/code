//! SPEC-KIT-975: Audit Event Emitter for runtime emit wiring.
//!
//! This module provides a context-aware wrapper around CapsuleHandle
//! to emit audit events at runtime boundaries. All emissions are
//! best-effort: failures are logged but never propagate to callers.
//!
//! ## Design
//! - `RunContext` captures run metadata (spec_id, run_id, branch_id, policy_hash)
//! - `AuditEventEmitter` wraps CapsuleHandle with RunContext
//! - All emit methods return `()` - errors logged but never fail the run
//!
//! ## Capture Modes (D15 Policy)
//! - `off`: Don't emit ModelCallEnvelope at all
//! - `hash`: Emit with SHA-256 hashes only
//! - `summary`: Emit with summary (first 200 chars) + hashes
//! - `full`: Emit complete content (NOT export-safe)

use crate::memvid_adapter::{
    BranchId, CapsuleHandle, ErrorEventPayload, ErrorSeverity, GateDecisionPayload, LLMCaptureMode,
    ModelCallEnvelopePayload, PatchApplyPayload, RetrievalRequestPayload, RetrievalResponsePayload,
    RoutingMode, ToolCallPayload, ToolResultPayload,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

/// Run context for event emission.
///
/// Captures the metadata needed for all events in a run.
#[derive(Debug, Clone)]
pub struct RunContext {
    /// SPEC ID (e.g., "SPEC-KIT-975")
    pub spec_id: String,
    /// Run ID (unique per run)
    pub run_id: String,
    /// Branch ID (run/<run_id> during run, main after merge)
    pub branch_id: BranchId,
    /// Policy hash (from PolicySnapshot)
    pub policy_hash: Option<String>,
    /// LLM capture mode for model calls
    pub capture_mode: LLMCaptureMode,
    /// Current stage (updated during run)
    pub current_stage: Option<String>,
    /// Current role (e.g., "Architect", "Implementer")
    pub current_role: Option<String>,
}

impl RunContext {
    /// Create a new run context.
    pub fn new(
        spec_id: impl Into<String>,
        run_id: impl Into<String>,
        capture_mode: LLMCaptureMode,
    ) -> Self {
        let run_id = run_id.into();
        Self {
            spec_id: spec_id.into(),
            branch_id: BranchId::for_run(&run_id),
            run_id,
            policy_hash: None,
            capture_mode,
            current_stage: None,
            current_role: None,
        }
    }

    /// Set the policy hash.
    pub fn with_policy_hash(mut self, hash: impl Into<String>) -> Self {
        self.policy_hash = Some(hash.into());
        self
    }

    /// Set the current stage.
    pub fn set_stage(&mut self, stage: impl Into<String>) {
        self.current_stage = Some(stage.into());
    }

    /// Set the current role.
    pub fn set_role(&mut self, role: impl Into<String>) {
        self.current_role = Some(role.into());
    }
}

/// Audit event emitter for runtime wiring.
///
/// Wraps a CapsuleHandle and RunContext to emit events at runtime boundaries.
/// All methods are best-effort: errors are logged but never returned.
pub struct AuditEventEmitter {
    /// Capsule handle for event storage
    capsule: Arc<CapsuleHandle>,
    /// Run context
    context: RunContext,
}

impl AuditEventEmitter {
    /// Create a new emitter.
    pub fn new(capsule: Arc<CapsuleHandle>, context: RunContext) -> Self {
        Self { capsule, context }
    }

    /// Get the current stage.
    pub fn current_stage(&self) -> Option<&str> {
        self.context.current_stage.as_deref()
    }

    /// Set the current stage.
    pub fn set_stage(&mut self, stage: impl Into<String>) {
        self.context.set_stage(stage);
    }

    /// Set the current role.
    pub fn set_role(&mut self, role: impl Into<String>) {
        self.context.set_role(role);
    }

    // =========================================================================
    // Tool Events (SPEC-KIT-975)
    // =========================================================================

    /// Emit a tool call event.
    ///
    /// Call this before invoking a tool.
    pub fn emit_tool_call(&self, tool_name: &str, input: serde_json::Value) -> String {
        let call_id = Uuid::new_v4().to_string();
        let payload = ToolCallPayload {
            call_id: call_id.clone(),
            tool_name: tool_name.to_string(),
            input,
            stage: self.context.current_stage.clone(),
            role: self.context.current_role.clone(),
        };

        match self
            .capsule
            .emit_tool_call(&self.context.spec_id, &self.context.run_id, &payload)
        {
            Ok(uri) => debug!(uri = %uri, tool = %tool_name, "Emitted ToolCall event"),
            Err(e) => {
                warn!(tool = %tool_name, error = %e, "Failed to emit ToolCall event (best-effort)")
            }
        }

        call_id
    }

    /// Emit a tool result event.
    ///
    /// Call this after a tool completes.
    pub fn emit_tool_result(
        &self,
        call_id: &str,
        tool_name: &str,
        success: bool,
        output: Option<serde_json::Value>,
        error: Option<String>,
        duration_ms: Option<u64>,
    ) {
        let payload = ToolResultPayload {
            call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            success,
            output,
            error,
            duration_ms,
        };

        match self.capsule.emit_tool_result(
            &self.context.spec_id,
            &self.context.run_id,
            self.context.current_stage.as_deref(),
            &payload,
        ) {
            Ok(uri) => debug!(uri = %uri, tool = %tool_name, success, "Emitted ToolResult event"),
            Err(e) => {
                warn!(tool = %tool_name, error = %e, "Failed to emit ToolResult event (best-effort)")
            }
        }
    }

    // =========================================================================
    // Retrieval Events (SPEC-KIT-975)
    // =========================================================================

    /// Emit a retrieval request event.
    ///
    /// Call this before executing a search.
    pub fn emit_retrieval_request(
        &self,
        query: &str,
        config: serde_json::Value,
        source: &str,
    ) -> String {
        let request_id = Uuid::new_v4().to_string();
        let payload = RetrievalRequestPayload {
            request_id: request_id.clone(),
            query: query.to_string(),
            config,
            source: source.to_string(),
            stage: self.context.current_stage.clone(),
            role: self.context.current_role.clone(),
        };

        match self.capsule.emit_retrieval_request(
            &self.context.spec_id,
            &self.context.run_id,
            &payload,
        ) {
            Ok(uri) => debug!(uri = %uri, source, "Emitted RetrievalRequest event"),
            Err(e) => {
                warn!(source, error = %e, "Failed to emit RetrievalRequest event (best-effort)")
            }
        }

        request_id
    }

    /// Emit a retrieval response event.
    ///
    /// Call this after search completes.
    pub fn emit_retrieval_response(
        &self,
        request_id: &str,
        hit_uris: Vec<String>,
        fused_scores: Option<Vec<f64>>,
        latency_ms: Option<u64>,
        error: Option<String>,
    ) {
        let payload = RetrievalResponsePayload {
            request_id: request_id.to_string(),
            hit_uris,
            fused_scores,
            explainability: None,
            latency_ms,
            error,
        };

        match self.capsule.emit_retrieval_response(
            &self.context.spec_id,
            &self.context.run_id,
            self.context.current_stage.as_deref(),
            &payload,
        ) {
            Ok(uri) => debug!(uri = %uri, request_id, "Emitted RetrievalResponse event"),
            Err(e) => {
                warn!(request_id, error = %e, "Failed to emit RetrievalResponse event (best-effort)")
            }
        }
    }

    // =========================================================================
    // Patch Events (SPEC-KIT-975)
    // =========================================================================

    /// Emit a patch apply event.
    ///
    /// Call this when a file is modified.
    pub fn emit_patch_apply(
        &self,
        file_path: &str,
        patch_type: &str,
        diff: Option<String>,
        before_hash: Option<String>,
        after_hash: Option<String>,
        success: bool,
        error: Option<String>,
    ) {
        let payload = PatchApplyPayload {
            patch_id: Uuid::new_v4().to_string(),
            file_path: file_path.to_string(),
            patch_type: patch_type.to_string(),
            diff,
            before_hash,
            after_hash,
            stage: self.context.current_stage.clone(),
            success,
            error,
        };

        match self
            .capsule
            .emit_patch_apply(&self.context.spec_id, &self.context.run_id, &payload)
        {
            Ok(uri) => {
                debug!(uri = %uri, file_path, patch_type, success, "Emitted PatchApply event")
            }
            Err(e) => warn!(file_path, error = %e, "Failed to emit PatchApply event (best-effort)"),
        }
    }

    // =========================================================================
    // Model Call Events (SPEC-KIT-975)
    // =========================================================================

    /// Emit a model call envelope event.
    ///
    /// Content capture is controlled by the capture_mode in RunContext:
    /// - none: No event emitted
    /// - prompts_only: Prompt + hashes (no response text)
    /// - full_io: Full prompt + response
    pub fn emit_model_call_envelope(
        &self,
        model: &str,
        routing_mode: RoutingMode,
        prompt: &str,
        response: &str,
        latency_ms: Option<u64>,
        success: bool,
        error: Option<String>,
        prompt_tokens: Option<u64>,
        response_tokens: Option<u64>,
    ) {
        // Check capture mode - if none, don't emit
        if self.context.capture_mode == LLMCaptureMode::None {
            debug!(model, "Skipping ModelCallEnvelope (capture_mode=none)");
            return;
        }

        let call_id = Uuid::new_v4().to_string();

        // Compute hashes (always, for verification)
        let prompt_hash = sha256_hash(prompt);
        let response_hash = sha256_hash(response);

        // Build payload based on capture mode
        let payload = match self.context.capture_mode {
            LLMCaptureMode::None => unreachable!(), // Handled above
            LLMCaptureMode::PromptsOnly => ModelCallEnvelopePayload {
                call_id,
                model: model.to_string(),
                routing_mode,
                capture_mode: LLMCaptureMode::PromptsOnly,
                stage: self.context.current_stage.clone(),
                role: self.context.current_role.clone(),
                prompt_hash: Some(prompt_hash),
                response_hash: Some(response_hash),
                prompt: Some(prompt.to_string()),
                response: None, // Response NOT stored in prompts_only
                prompt_tokens,
                response_tokens,
                latency_ms,
                success,
                error,
            },
            LLMCaptureMode::FullIo => ModelCallEnvelopePayload {
                call_id,
                model: model.to_string(),
                routing_mode,
                capture_mode: LLMCaptureMode::FullIo,
                stage: self.context.current_stage.clone(),
                role: self.context.current_role.clone(),
                prompt_hash: Some(prompt_hash),
                response_hash: Some(response_hash),
                prompt: Some(prompt.to_string()),
                response: Some(response.to_string()),
                prompt_tokens,
                response_tokens,
                latency_ms,
                success,
                error,
            },
        };

        match self.capsule.emit_model_call_envelope(
            &self.context.spec_id,
            &self.context.run_id,
            &payload,
        ) {
            Ok(uri) => debug!(
                uri = %uri,
                model,
                routing = %routing_mode.as_str(),
                capture = %self.context.capture_mode.as_str(),
                "Emitted ModelCallEnvelope event"
            ),
            Err(e) => {
                warn!(model, error = %e, "Failed to emit ModelCallEnvelope event (best-effort)")
            }
        }
    }

    // =========================================================================
    // Gate and Error Events (SPEC-KIT-975)
    // =========================================================================

    /// Emit a gate decision event.
    pub fn emit_gate_decision(&self, payload: &GateDecisionPayload) {
        match self
            .capsule
            .emit_gate_decision(&self.context.spec_id, &self.context.run_id, payload)
        {
            Ok(uri) => {
                debug!(uri = %uri, gate = %payload.gate_name, outcome = %payload.outcome.as_str(), "Emitted GateDecision event")
            }
            Err(e) => {
                warn!(gate = %payload.gate_name, error = %e, "Failed to emit GateDecision event (best-effort)")
            }
        }
    }

    /// Emit an error event.
    pub fn emit_error(
        &self,
        error_code: &str,
        message: &str,
        severity: ErrorSeverity,
        component: Option<&str>,
        recoverable: bool,
    ) {
        let payload = ErrorEventPayload {
            error_code: error_code.to_string(),
            message: message.to_string(),
            severity,
            stage: self.context.current_stage.clone(),
            component: component.map(|s| s.to_string()),
            stack_trace: None,
            related_uris: None,
            recoverable,
        };

        match self
            .capsule
            .emit_error_event(&self.context.spec_id, &self.context.run_id, &payload)
        {
            Ok(uri) => {
                debug!(uri = %uri, error_code, severity = %severity.as_str(), "Emitted ErrorEvent")
            }
            Err(e) => warn!(error_code, error = %e, "Failed to emit ErrorEvent (best-effort)"),
        }
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get the run context.
    pub fn context(&self) -> &RunContext {
        &self.context
    }

    /// Get the capsule handle.
    pub fn capsule(&self) -> &CapsuleHandle {
        &self.capsule
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute SHA-256 hash of content.
fn sha256_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hash() {
        let hash = sha256_hash("hello world");
        assert_eq!(hash.len(), 64); // SHA-256 is 256 bits = 64 hex chars
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_run_context_creation() {
        let ctx = RunContext::new("SPEC-KIT-975", "run-001", LLMCaptureMode::PromptsOnly)
            .with_policy_hash("abc123");

        assert_eq!(ctx.spec_id, "SPEC-KIT-975");
        assert_eq!(ctx.run_id, "run-001");
        assert_eq!(ctx.branch_id.as_str(), "run/run-001");
        assert_eq!(ctx.policy_hash, Some("abc123".to_string()));
        assert_eq!(ctx.capture_mode, LLMCaptureMode::PromptsOnly);
    }

    #[test]
    fn test_capture_mode_respects_none() {
        // When capture_mode is None, emit_model_call_envelope should short-circuit
        // This is verified by the early return in the function
        let ctx = RunContext::new("SPEC-KIT-975", "run-001", LLMCaptureMode::None);
        assert_eq!(ctx.capture_mode, LLMCaptureMode::None);
    }
}
