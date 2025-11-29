//! State management for spec-kit automation
//!
//! Extracted from chatwidget.rs to isolate spec-kit code from upstream

use crate::slash_command::{HalMode, SlashCommand};
use crate::spec_prompts::SpecStage;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ============================================================================
// CONSENSUS SEQUENCING (P6-SYNC)
// Pattern ported from Auto Drive: decision sequencing for exactly-once processing
// ============================================================================

/// Consensus sequence tracking for exactly-once agent response processing.
///
/// Prevents duplicate consensus processing that could occur from:
/// - Retry logic producing duplicate responses
/// - Out-of-order completion events
/// - Race conditions in parallel agent execution
///
/// Pattern source: Auto Drive auto_coordinator.rs decision sequencing
#[derive(Debug)]
pub struct ConsensusSequence {
    /// Monotonically increasing sequence number for consensus operations
    decision_seq: AtomicU64,
    /// Sequence numbers that have been fully processed (for duplicate rejection)
    processed_seqs: Mutex<HashSet<u64>>,
    /// Pending acknowledgment (sequence awaiting consensus completion)
    pending_ack_seq: Mutex<Option<u64>>,
}

impl Default for ConsensusSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ConsensusSequence {
    fn clone(&self) -> Self {
        Self {
            decision_seq: AtomicU64::new(self.decision_seq.load(Ordering::SeqCst)),
            processed_seqs: Mutex::new(
                self.processed_seqs
                    .lock()
                    .expect("processed_seqs mutex poisoned")
                    .clone(),
            ),
            pending_ack_seq: Mutex::new(
                *self
                    .pending_ack_seq
                    .lock()
                    .expect("pending_ack_seq mutex poisoned"),
            ),
        }
    }
}

impl ConsensusSequence {
    pub fn new() -> Self {
        Self {
            decision_seq: AtomicU64::new(0),
            processed_seqs: Mutex::new(HashSet::new()),
            pending_ack_seq: Mutex::new(None),
        }
    }

    /// Acquire the next sequence number for a new consensus operation.
    /// Returns (seq, is_duplicate) where is_duplicate indicates if this
    /// sequence was already processed (should be rejected).
    pub fn next_seq(&self) -> u64 {
        self.decision_seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Check if a sequence number should be processed.
    /// Returns false if the sequence was already processed (duplicate).
    pub fn should_process(&self, seq: u64) -> bool {
        let processed = self
            .processed_seqs
            .lock()
            .expect("processed_seqs mutex poisoned");
        !processed.contains(&seq)
    }

    /// Begin processing a sequence. Returns false if already being processed.
    /// Sets the pending acknowledgment sequence.
    pub fn begin_processing(&self, seq: u64) -> bool {
        let processed = self
            .processed_seqs
            .lock()
            .expect("processed_seqs mutex poisoned");
        if processed.contains(&seq) {
            return false;
        }

        let mut pending = self
            .pending_ack_seq
            .lock()
            .expect("pending_ack_seq mutex poisoned");
        if pending.is_some() {
            // Already processing another sequence
            tracing::warn!(
                "Consensus: Attempted to begin seq {} while seq {:?} is pending",
                seq,
                *pending
            );
            return false;
        }
        *pending = Some(seq);
        true
    }

    /// Acknowledge successful processing of a sequence.
    /// Marks the sequence as processed and clears the pending acknowledgment.
    pub fn ack_processed(&self, seq: u64) -> bool {
        let mut pending = self
            .pending_ack_seq
            .lock()
            .expect("pending_ack_seq mutex poisoned");

        if *pending != Some(seq) {
            tracing::warn!(
                "Consensus: Ack for seq {} but pending is {:?}",
                seq,
                *pending
            );
            return false;
        }

        // Clear pending and mark as processed
        *pending = None;
        drop(pending);

        let mut processed = self
            .processed_seqs
            .lock()
            .expect("processed_seqs mutex poisoned");
        processed.insert(seq);

        tracing::debug!("Consensus: Ack seq {} - now processed", seq);
        true
    }

    /// Cancel pending processing (e.g., on error/timeout).
    /// Does NOT mark as processed, allowing retry with same sequence.
    pub fn cancel_pending(&self, seq: u64) -> bool {
        let mut pending = self
            .pending_ack_seq
            .lock()
            .expect("pending_ack_seq mutex poisoned");

        if *pending != Some(seq) {
            return false;
        }

        *pending = None;
        tracing::debug!("Consensus: Cancelled pending seq {}", seq);
        true
    }

    /// Get the current sequence number (latest assigned).
    pub fn current_seq(&self) -> u64 {
        self.decision_seq.load(Ordering::SeqCst)
    }

    /// Get the pending acknowledgment sequence, if any.
    pub fn pending_seq(&self) -> Option<u64> {
        *self
            .pending_ack_seq
            .lock()
            .expect("pending_ack_seq mutex poisoned")
    }

    /// Get the count of processed sequences.
    pub fn processed_count(&self) -> usize {
        self.processed_seqs
            .lock()
            .expect("processed_seqs mutex poisoned")
            .len()
    }

    /// Reset all state (for new pipeline run).
    pub fn reset(&self) {
        self.decision_seq.store(0, Ordering::SeqCst);
        self.processed_seqs
            .lock()
            .expect("processed_seqs mutex poisoned")
            .clear();
        *self
            .pending_ack_seq
            .lock()
            .expect("pending_ack_seq mutex poisoned") = None;
    }
}

/// Result of attempting to begin consensus processing
/// Reserved for future UI integration (showing sequence status in status bar)
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusBeginOutcome {
    /// Started processing with assigned sequence
    Started { seq: u64 },
    /// Duplicate - sequence already processed
    Duplicate { seq: u64 },
    /// Blocked - another sequence is pending
    Blocked { pending_seq: u64 },
}

/// Phase tracking for /speckit.auto pipeline
#[derive(Debug, Clone)]
pub enum SpecAutoPhase {
    Guardrail,
    ExecutingAgents {
        // Track which agents we're waiting for completion
        expected_agents: Vec<String>,
        // Track which agents have completed (populated from AgentStatusUpdateEvent)
        completed_agents: HashSet<String>,
    },
    CheckingConsensus,

    // === Quality Gate Phases (T85) ===
    /// Executing quality gate agents
    QualityGateExecuting {
        checkpoint: QualityCheckpoint,
        gates: Vec<QualityGateType>,
        active_gates: HashSet<QualityGateType>,
        expected_agents: Vec<String>,
        completed_agents: HashSet<String>,
        results: HashMap<String, Value>, // agent_id -> JSON result
        native_agent_ids: Option<Vec<String>>, // SPEC-KIT-900: Track native orchestrator agent IDs
    },

    /// Processing quality gate results (classification)
    QualityGateProcessing {
        checkpoint: QualityCheckpoint,
        auto_resolved: Vec<QualityIssue>,
        escalated: Vec<QualityIssue>,
    },

    /// Validating 2/3 majority answers with GPT-5.1 (async via agent system)
    QualityGateValidating {
        checkpoint: QualityCheckpoint,
        auto_resolved: Vec<QualityIssue>, // Unanimous issues already resolved
        pending_validations: Vec<(QualityIssue, String)>, // (issue, majority_answer)
        completed_validations: HashMap<usize, GPT5ValidationResult>, // index -> validation result
    },

    /// Awaiting human answers for escalated questions
    QualityGateAwaitingHuman {
        checkpoint: QualityCheckpoint,
        escalated_issues: Vec<QualityIssue>, // Store original issues
        escalated_questions: Vec<EscalatedQuestion>, // For UI display
        answers: HashMap<String, String>,    // question_id -> human_answer
    },
}

/// Waiting state for guardrail execution
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GuardrailWait {
    pub stage: SpecStage,
    pub command: SlashCommand,
    pub task_id: Option<String>,
}

/// Execution mode for validate lifecycle tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidateMode {
    Auto,
    Manual,
}

impl ValidateMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }
}

/// Active stage within a validate run lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidateStageStatus {
    Queued,
    Dispatched,
    CheckingConsensus,
}

/// Lifecycle telemetry events for validate runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidateLifecycleEvent {
    Queued,
    Dispatched,
    CheckingConsensus,
    Completed,
    Cancelled,
    Failed,
    Reset,
    Deduped,
}

impl ValidateLifecycleEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Dispatched => "dispatched",
            Self::CheckingConsensus => "checking_consensus",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Reset => "reset",
            Self::Deduped => "deduped",
        }
    }
}

/// Terminal outcome for a validate run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidateCompletionReason {
    Completed,
    Cancelled,
    Failed,
    Reset,
}

impl ValidateCompletionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Reset => "reset",
        }
    }
}

/// Information about an active validate run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateRunInfo {
    pub run_id: String,
    pub attempt: u32,
    pub dedupe_count: u32,
    pub mode: ValidateMode,
    pub status: ValidateStageStatus,
    pub payload_hash: String,
}

/// Details about a completed validate run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateRunCompletion {
    pub run_id: String,
    pub attempt: u32,
    pub dedupe_count: u32,
    pub mode: ValidateMode,
    pub reason: ValidateCompletionReason,
    pub payload_hash: String,
}

/// Result when attempting to begin a validate run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateBeginOutcome {
    Started(ValidateRunInfo),
    Duplicate(ValidateRunInfo),
    Conflict(ValidateRunInfo),
}

#[derive(Debug)]
struct ActiveValidateRun {
    run_id: String,
    payload_hash: String,
    mode: ValidateMode,
    status: ValidateStageStatus,
    dedupe_count: u32,
}

impl ActiveValidateRun {
    fn to_info(&self, attempt: u32) -> ValidateRunInfo {
        ValidateRunInfo {
            run_id: self.run_id.clone(),
            attempt,
            dedupe_count: self.dedupe_count,
            mode: self.mode,
            status: self.status,
            payload_hash: self.payload_hash.clone(),
        }
    }

    fn to_completion(
        &self,
        attempt: u32,
        reason: ValidateCompletionReason,
    ) -> ValidateRunCompletion {
        ValidateRunCompletion {
            run_id: self.run_id.clone(),
            attempt,
            dedupe_count: self.dedupe_count,
            mode: self.mode,
            reason,
            payload_hash: self.payload_hash.clone(),
        }
    }
}

#[derive(Debug, Default)]
struct ValidateLifecycleInner {
    attempt: u32,
    active: Option<ActiveValidateRun>,
    last_completion: Option<ValidateRunCompletion>,
}

/// Thread-safe validate lifecycle guard shared across manual and automated runs.
#[derive(Debug, Clone)]
pub struct ValidateLifecycle {
    spec_id: Arc<String>,
    inner: Arc<Mutex<ValidateLifecycleInner>>,
}

impl ValidateLifecycle {
    pub fn new<S: Into<String>>(spec_id: S) -> Self {
        Self {
            spec_id: Arc::new(spec_id.into()),
            inner: Arc::new(Mutex::new(ValidateLifecycleInner::default())),
        }
    }

    pub fn begin(&self, mode: ValidateMode, payload_hash: &str) -> ValidateBeginOutcome {
        let mut inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        let current_attempt = inner.attempt;

        match inner.active.as_mut() {
            Some(active) => {
                active.dedupe_count = active.dedupe_count.saturating_add(1);
                let attempt = current_attempt;
                if active.payload_hash == payload_hash && active.mode == mode {
                    let info = active.to_info(attempt);
                    ValidateBeginOutcome::Duplicate(info)
                } else {
                    let info = active.to_info(attempt);
                    ValidateBeginOutcome::Conflict(info)
                }
            }
            None => {
                let next_attempt = current_attempt.saturating_add(1);
                inner.attempt = next_attempt;
                let run_id = format!(
                    "validate-{}-{}-attempt-{}-{}",
                    self.spec_id,
                    mode.as_str(),
                    next_attempt,
                    Uuid::new_v4().simple()
                );

                let run = ActiveValidateRun {
                    run_id,
                    payload_hash: payload_hash.to_string(),
                    mode,
                    status: ValidateStageStatus::Queued,
                    dedupe_count: 0,
                };
                let info = run.to_info(next_attempt);
                inner.active = Some(run);
                ValidateBeginOutcome::Started(info)
            }
        }
    }

    pub fn mark_dispatched(&self, run_id: &str) -> Option<ValidateRunInfo> {
        let mut inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        let attempt = inner.attempt;
        let active = inner.active.as_mut()?;
        if active.run_id != run_id {
            return None;
        }
        active.status = ValidateStageStatus::Dispatched;
        Some(active.to_info(attempt))
    }

    pub fn mark_checking_consensus(&self, run_id: &str) -> Option<ValidateRunInfo> {
        let mut inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        let attempt = inner.attempt;
        let active = inner.active.as_mut()?;
        if active.run_id != run_id {
            return None;
        }
        active.status = ValidateStageStatus::CheckingConsensus;
        Some(active.to_info(attempt))
    }

    pub fn complete(
        &self,
        run_id: &str,
        reason: ValidateCompletionReason,
    ) -> Option<ValidateRunCompletion> {
        let mut inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        let active = inner.active.take()?;
        if active.run_id != run_id {
            inner.active = Some(active);
            return None;
        }
        let completion = active.to_completion(inner.attempt, reason);
        inner.last_completion = Some(completion.clone());
        Some(completion)
    }

    pub fn reset_active(&self, reason: ValidateCompletionReason) -> Option<ValidateRunCompletion> {
        let mut inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        let active = inner.active.take()?;
        let completion = active.to_completion(inner.attempt, reason);
        inner.last_completion = Some(completion.clone());
        Some(completion)
    }

    pub fn active(&self) -> Option<ValidateRunInfo> {
        let inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        let attempt = inner.attempt;
        inner.active.as_ref().map(|run| run.to_info(attempt))
    }

    pub fn active_payload_hash(&self) -> Option<String> {
        let inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        inner.active.as_ref().map(|run| run.payload_hash.clone())
    }

    pub fn last_completion(&self) -> Option<ValidateRunCompletion> {
        let inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        inner.last_completion.clone()
    }

    pub fn attempt(&self) -> u32 {
        let inner = self
            .inner
            .lock()
            .expect("validate lifecycle mutex poisoned");
        inner.attempt
    }

    pub fn spec_id(&self) -> &str {
        &self.spec_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_lifecycle_transitions() {
        let lifecycle = ValidateLifecycle::new("SPEC-TEST-069");

        let first = lifecycle.begin(ValidateMode::Auto, "hash-1");
        let info = match first {
            ValidateBeginOutcome::Started(info) => info,
            _ => panic!("expected Started"),
        };
        assert_eq!(info.attempt, 1);
        assert_eq!(info.dedupe_count, 0);
        assert_eq!(info.status, ValidateStageStatus::Queued);

        let duplicate = lifecycle.begin(ValidateMode::Auto, "hash-1");
        match duplicate {
            ValidateBeginOutcome::Duplicate(info) => {
                assert_eq!(info.dedupe_count, 1);
                assert_eq!(info.attempt, 1);
            }
            _ => panic!("expected Duplicate"),
        }

        let dispatched = lifecycle
            .mark_dispatched(&info.run_id)
            .expect("dispatch transition");
        assert_eq!(dispatched.status, ValidateStageStatus::Dispatched);

        let checking = lifecycle
            .mark_checking_consensus(&info.run_id)
            .expect("checking transition");
        assert_eq!(checking.status, ValidateStageStatus::CheckingConsensus);

        let completion = lifecycle
            .complete(&info.run_id, ValidateCompletionReason::Completed)
            .expect("completion");
        assert_eq!(completion.reason, ValidateCompletionReason::Completed);
        assert_eq!(completion.attempt, 1);

        let second = lifecycle.begin(ValidateMode::Auto, "hash-2");
        let info2 = match second {
            ValidateBeginOutcome::Started(info) => info,
            _ => panic!("expected Started"),
        };
        assert_eq!(info2.attempt, 2);
        assert_eq!(info2.dedupe_count, 0);

        let reset = lifecycle
            .reset_active(ValidateCompletionReason::Reset)
            .expect("reset active run");
        assert_eq!(reset.reason, ValidateCompletionReason::Reset);
        assert_eq!(reset.attempt, 2);

        assert!(lifecycle.active().is_none());
    }

    // P6-SYNC: ConsensusSequence tests

    #[test]
    fn consensus_sequence_basic_flow() {
        let seq = ConsensusSequence::new();

        // Initial state
        assert_eq!(seq.current_seq(), 0);
        assert_eq!(seq.pending_seq(), None);
        assert_eq!(seq.processed_count(), 0);

        // Get first sequence
        let s1 = seq.next_seq();
        assert_eq!(s1, 1);
        assert_eq!(seq.current_seq(), 1);

        // Should be processable
        assert!(seq.should_process(s1));

        // Begin processing
        assert!(seq.begin_processing(s1));
        assert_eq!(seq.pending_seq(), Some(s1));

        // Cannot begin another while one is pending
        let s2 = seq.next_seq();
        assert!(!seq.begin_processing(s2));

        // Ack completion
        assert!(seq.ack_processed(s1));
        assert_eq!(seq.pending_seq(), None);
        assert_eq!(seq.processed_count(), 1);

        // s1 is now processed (duplicate)
        assert!(!seq.should_process(s1));
        assert!(!seq.begin_processing(s1));

        // s2 is still processable
        assert!(seq.should_process(s2));
        assert!(seq.begin_processing(s2));
        assert!(seq.ack_processed(s2));
    }

    #[test]
    fn consensus_sequence_cancel_allows_retry() {
        let seq = ConsensusSequence::new();

        let s1 = seq.next_seq();
        assert!(seq.begin_processing(s1));

        // Cancel - should allow retry
        assert!(seq.cancel_pending(s1));
        assert_eq!(seq.pending_seq(), None);
        assert_eq!(seq.processed_count(), 0);

        // Can retry the same sequence
        assert!(seq.should_process(s1));
        assert!(seq.begin_processing(s1));
        assert!(seq.ack_processed(s1));

        // Now it's processed
        assert!(!seq.should_process(s1));
    }

    #[test]
    fn consensus_sequence_reset() {
        let seq = ConsensusSequence::new();

        // Process a few sequences
        let s1 = seq.next_seq();
        seq.begin_processing(s1);
        seq.ack_processed(s1);

        let s2 = seq.next_seq();
        seq.begin_processing(s2);
        seq.ack_processed(s2);

        assert_eq!(seq.current_seq(), 2);
        assert_eq!(seq.processed_count(), 2);

        // Reset
        seq.reset();

        assert_eq!(seq.current_seq(), 0);
        assert_eq!(seq.processed_count(), 0);
        assert_eq!(seq.pending_seq(), None);

        // Fresh start
        let s3 = seq.next_seq();
        assert_eq!(s3, 1);
    }

    #[test]
    fn consensus_sequence_clone() {
        let seq = ConsensusSequence::new();
        let s1 = seq.next_seq();
        seq.begin_processing(s1);
        seq.ack_processed(s1);

        let cloned = seq.clone();
        assert_eq!(cloned.current_seq(), seq.current_seq());
        assert_eq!(cloned.processed_count(), seq.processed_count());
        assert!(!cloned.should_process(s1)); // Still tracked as processed
    }
}

/// State for /speckit.auto pipeline automation
#[derive(Debug, Clone)]
pub struct SpecAutoState {
    pub spec_id: String,
    pub goal: String,
    pub stages: Vec<SpecStage>,
    pub current_index: usize,
    pub phase: SpecAutoPhase,
    pub waiting_guardrail: Option<GuardrailWait>,
    pub pending_prompt_summary: Option<String>,
    pub hal_mode: Option<HalMode>,

    // === Quality Gate State (T85) ===
    pub quality_gates_enabled: bool,
    pub completed_checkpoints: HashSet<QualityCheckpoint>,
    pub quality_gate_processing: Option<QualityCheckpoint>, // Currently processing (prevents recursion)
    pub quality_modifications: Vec<String>,                 // Track files modified by quality gates
    pub quality_auto_resolved: Vec<(QualityIssue, String)>, // All auto-resolutions
    pub quality_escalated: Vec<(QualityIssue, String)>,     // All human-answered questions
    pub quality_checkpoint_outcomes: Vec<(QualityCheckpoint, usize, usize)>, // (checkpoint, auto, escalated)
    pub quality_checkpoint_degradations: HashMap<QualityCheckpoint, Vec<String>>, // missing agents per checkpoint

    // Tracks which stages have already scheduled degraded follow-up checklists
    pub degraded_followups: std::collections::HashSet<SpecStage>,

    // SPEC-KIT-069: Validate lifecycle guard (shared across manual/auto paths)
    pub validate_lifecycle: ValidateLifecycle,

    // SPEC-KIT-070: Track which agents already emitted cost entries per stage
    pub cost_recorded_agents: HashMap<SpecStage, HashSet<String>>,

    // SPEC-KIT-070: Record routing notes per stage
    pub aggregator_effort_notes: HashMap<SpecStage, String>,
    pub escalation_reason_notes: HashMap<SpecStage, String>,

    // ACE Framework Integration (2025-10-29)
    // Cache ACE playbook bullets for current stage to avoid async boundary issues
    pub ace_bullets_cache: Option<Vec<super::ace_client::PlaybookBullet>>,
    // Track which bullet IDs were used (for learning feedback)
    pub ace_bullet_ids_used: Option<Vec<i32>>,

    // SPEC-KIT-070: Execution logging for full pipeline visibility
    pub execution_logger: Arc<super::execution_logger::ExecutionLogger>,
    pub run_id: Option<String>,

    // Agent response cache for consensus (avoids memory dependency)
    // Collected from active_agents after completion, before consensus runs
    pub agent_responses_cache: Option<Vec<(String, String)>>, // (agent_name, response_text)

    // SPEC-948: Pipeline configuration for modular stage execution
    pub pipeline_config: super::pipeline_config::PipelineConfig,

    // P6-SYNC: Consensus sequence tracking for exactly-once processing
    pub consensus_sequence: ConsensusSequence,
}

impl SpecAutoState {
    #[allow(dead_code)]
    pub fn new(
        spec_id: String,
        goal: String,
        resume_from: SpecStage,
        hal_mode: Option<HalMode>,
        pipeline_config: super::pipeline_config::PipelineConfig,
    ) -> Self {
        Self::with_quality_gates(spec_id, goal, resume_from, hal_mode, true, pipeline_config)
    }

    pub fn with_quality_gates(
        spec_id: String,
        goal: String,
        resume_from: SpecStage,
        hal_mode: Option<HalMode>,
        quality_gates_enabled: bool,
        pipeline_config: super::pipeline_config::PipelineConfig,
    ) -> Self {
        // SPEC-948 Task 2.2: Include ALL stages (Plan→Unlock) for skip telemetry tracking
        // Stage filtering happens in advance_spec_auto(), not here
        // This allows us to record telemetry for skipped stages
        let stages: Vec<SpecStage> = vec![
            SpecStage::Plan,
            SpecStage::Tasks,
            SpecStage::Implement,
            SpecStage::Validate,
            SpecStage::Audit,
            SpecStage::Unlock,
        ];

        let start_index = stages
            .iter()
            .position(|stage| *stage == resume_from)
            .unwrap_or(0);

        // Always start with Guardrail phase
        // Quality checkpoints will be triggered by advance_spec_auto when needed
        let initial_phase = SpecAutoPhase::Guardrail;

        let lifecycle = ValidateLifecycle::new(spec_id.clone());
        let logger = Arc::new(super::execution_logger::ExecutionLogger::new());
        let run_id = super::execution_logger::generate_run_id(&spec_id);

        // Initialize logger (recursion fixed in commit 4c537c7e0)
        if let Err(e) = logger.init(&spec_id, run_id.clone()) {
            tracing::warn!("Failed to initialize execution logger: {}", e);
        }

        Self {
            spec_id,
            goal,
            stages,
            current_index: start_index,
            phase: initial_phase,
            waiting_guardrail: None,
            pending_prompt_summary: None,
            hal_mode,
            quality_gates_enabled,
            completed_checkpoints: HashSet::new(),
            quality_gate_processing: None,
            quality_modifications: Vec::new(),
            quality_auto_resolved: Vec::new(),
            quality_escalated: Vec::new(),
            quality_checkpoint_outcomes: Vec::new(),
            quality_checkpoint_degradations: HashMap::new(),
            degraded_followups: std::collections::HashSet::new(),
            validate_lifecycle: lifecycle,
            cost_recorded_agents: HashMap::new(),
            aggregator_effort_notes: HashMap::new(),
            escalation_reason_notes: HashMap::new(),
            // ACE Framework Integration
            ace_bullets_cache: None,
            ace_bullet_ids_used: None,
            // Execution logging
            execution_logger: logger,
            run_id: Some(run_id),
            // Agent response cache
            agent_responses_cache: None,
            // Pipeline configuration (SPEC-948)
            pipeline_config,
            // P6-SYNC: Consensus sequence tracking
            consensus_sequence: ConsensusSequence::new(),
        }
    }

    pub fn current_stage(&self) -> Option<SpecStage> {
        self.stages.get(self.current_index).copied()
    }

    pub fn mark_agent_cost_recorded(&mut self, stage: SpecStage, agent_id: &str) -> bool {
        self.cost_recorded_agents
            .entry(stage)
            .or_default()
            .insert(agent_id.to_string())
    }

    pub fn reset_cost_tracking(&mut self, stage: SpecStage) {
        self.cost_recorded_agents.remove(&stage);
    }

    #[allow(dead_code)]
    pub fn is_executing_agents(&self) -> bool {
        matches!(self.phase, SpecAutoPhase::ExecutingAgents { .. })
    }

    /// Transition to new phase with logging
    pub fn transition_phase(&mut self, new_phase: SpecAutoPhase, trigger: &str) {
        let old_phase_name = format!("{:?}", self.phase);
        let new_phase_name = format!("{:?}", new_phase);

        if let (Some(run_id), Some(stage)) = (&self.run_id, self.current_stage()) {
            self.execution_logger.log_event(
                super::execution_logger::ExecutionEvent::PhaseTransition {
                    run_id: run_id.clone(),
                    from_phase: old_phase_name,
                    to_phase: new_phase_name,
                    stage: stage.display_name().to_string(),
                    trigger: trigger.to_string(),
                    timestamp: super::execution_logger::ExecutionEvent::now(),
                },
            );
        }

        self.phase = new_phase;
    }

    pub fn set_validate_lifecycle(&mut self, lifecycle: ValidateLifecycle) {
        self.validate_lifecycle = lifecycle;
    }

    pub fn begin_validate_run(&self, payload_hash: &str) -> ValidateBeginOutcome {
        self.validate_lifecycle
            .begin(ValidateMode::Auto, payload_hash)
    }

    pub fn mark_validate_dispatched(&self, run_id: &str) -> Option<ValidateRunInfo> {
        self.validate_lifecycle.mark_dispatched(run_id)
    }

    pub fn mark_validate_checking(&self, run_id: &str) -> Option<ValidateRunInfo> {
        self.validate_lifecycle.mark_checking_consensus(run_id)
    }

    pub fn complete_validate_run(
        &self,
        run_id: &str,
        reason: ValidateCompletionReason,
    ) -> Option<ValidateRunCompletion> {
        self.validate_lifecycle.complete(run_id, reason)
    }

    pub fn reset_validate_run(
        &self,
        reason: ValidateCompletionReason,
    ) -> Option<ValidateRunCompletion> {
        self.validate_lifecycle.reset_active(reason)
    }

    pub fn active_validate_run(&self) -> Option<ValidateRunInfo> {
        self.validate_lifecycle.active()
    }

    pub fn validate_attempt(&self) -> u32 {
        self.validate_lifecycle.attempt()
    }

    pub fn current_validate_payload_hash(&self) -> Option<String> {
        self.validate_lifecycle.active_payload_hash()
    }
}

/// Guardrail evaluation result
pub struct GuardrailEvaluation {
    pub success: bool,
    pub summary: String,
    pub failures: Vec<String>,
}

/// Guardrail outcome with telemetry
#[derive(Debug, Clone)]
pub struct GuardrailOutcome {
    pub success: bool,
    pub summary: String,
    pub telemetry_path: Option<PathBuf>,
    pub failures: Vec<String>,
}

// === Quality Gate Types (T85) ===

/// Quality checkpoint in the pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QualityCheckpoint {
    /// Before plan stage (runs clarify to resolve PRD ambiguities early)
    /// Assumes PRD exists from /speckit.specify
    BeforeSpecify,
    /// After plan stage, before tasks (runs checklist to validate PRD+plan quality)
    AfterSpecify,
    /// After tasks stage, before implement (runs analyze for full consistency check)
    AfterTasks,
}

impl QualityCheckpoint {
    pub fn name(&self) -> &'static str {
        match self {
            Self::BeforeSpecify => "before-specify",
            Self::AfterSpecify => "after-specify",
            Self::AfterTasks => "after-tasks",
        }
    }

    pub fn gates(&self) -> &[QualityGateType] {
        match self {
            Self::BeforeSpecify => &[QualityGateType::Clarify],
            Self::AfterSpecify => &[QualityGateType::Checklist],
            Self::AfterTasks => &[QualityGateType::Analyze],
        }
    }
}

/// Type of quality gate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QualityGateType {
    /// Identify and resolve ambiguities
    Clarify,
    /// Score and improve requirements
    Checklist,
    /// Check consistency across artifacts
    Analyze,
}

impl QualityGateType {
    pub fn command_name(&self) -> &'static str {
        match self {
            Self::Clarify => "clarify",
            Self::Checklist => "checklist",
            Self::Analyze => "analyze",
        }
    }
}

/// Agent confidence level (derived from agreement)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// All agents agree (3/3)
    High,
    /// Majority agree (2/3)
    Medium,
    /// No consensus (0-1/3)
    Low,
}

/// Issue magnitude/severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Magnitude {
    /// Blocks progress, affects core functionality
    Critical,
    /// Significant but not blocking
    Important,
    /// Nice-to-have, cosmetic, minor
    Minor,
}

/// Whether agents can resolve the issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolvability {
    /// Straightforward fix, apply immediately
    AutoFix,
    /// Fix available but needs validation
    SuggestFix,
    /// Requires human judgment
    NeedHuman,
}

/// Quality issue identified by agents
#[derive(Debug, Clone)]
pub struct QualityIssue {
    pub id: String,
    pub gate_type: QualityGateType,
    pub issue_type: String,
    pub description: String,
    pub confidence: Confidence,
    pub magnitude: Magnitude,
    pub resolvability: Resolvability,
    pub suggested_fix: Option<String>,
    pub context: String,
    pub affected_artifacts: Vec<String>,
    pub agent_answers: HashMap<String, String>,
    pub agent_reasoning: HashMap<String, String>,
}

/// GPT-5.1 validation result for majority answers
#[derive(Debug, Clone)]
pub struct GPT5ValidationResult {
    pub agrees_with_majority: bool,
    pub reasoning: String,
    pub recommended_answer: Option<String>,
    pub confidence: Confidence,
}

/// Resolution decision for a quality issue
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Auto-apply the answer
    AutoApply {
        answer: String,
        confidence: Confidence,
        reason: String,
        validation: Option<GPT5ValidationResult>,
    },
    /// Escalate to human
    Escalate {
        reason: String,
        all_answers: HashMap<String, String>,
        gpt5_reasoning: Option<String>,
        recommended: Option<String>,
    },
}

/// Escalated question requiring human input
#[derive(Debug, Clone)]
pub struct EscalatedQuestion {
    pub id: String,
    pub gate_type: QualityGateType,
    pub question: String,
    pub context: String,
    pub agent_answers: HashMap<String, String>,
    pub gpt5_reasoning: Option<String>,
    pub magnitude: Magnitude,
    pub suggested_options: Vec<String>,
}

/// Outcome of a quality checkpoint (one or more gates)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct QualityCheckpointOutcome {
    pub checkpoint: QualityCheckpoint,
    pub total_issues: usize,
    pub auto_resolved: usize,
    pub escalated: usize,
    pub escalated_questions: Vec<EscalatedQuestion>,
    pub auto_resolutions: Vec<(QualityIssue, String)>, // (issue, applied_answer)
    pub telemetry_path: Option<PathBuf>,
}

// === Helper Functions ===

pub fn guardrail_for_stage(stage: SpecStage) -> SlashCommand {
    // SPEC-KIT-066: Use native /guardrail.* commands instead of bash scripts
    // Quality gates (SPEC-KIT-068) provide validation; guardrails add lightweight checks
    match stage {
        SpecStage::Plan => SlashCommand::GuardrailPlan,
        SpecStage::Tasks => SlashCommand::GuardrailTasks,
        SpecStage::Implement => SlashCommand::GuardrailImplement,
        SpecStage::Validate => SlashCommand::GuardrailValidate,
        SpecStage::Audit => SlashCommand::GuardrailAudit,
        SpecStage::Unlock => SlashCommand::GuardrailUnlock,
        SpecStage::Clarify | SpecStage::Analyze | SpecStage::Checklist => {
            // Quality commands don't have guardrails (they are quality checks themselves)
            SlashCommand::GuardrailPlan // Fallback (unused)
        }
    }
}

pub fn spec_ops_stage_prefix(stage: SpecStage) -> &'static str {
    match stage {
        SpecStage::Plan => "plan_",
        SpecStage::Tasks => "tasks_",
        SpecStage::Implement => "implement_",
        SpecStage::Validate => "validate_",
        SpecStage::Audit => "audit_",
        SpecStage::Unlock => "unlock_",
        SpecStage::Clarify => "clarify_",
        SpecStage::Analyze => "analyze_",
        SpecStage::Checklist => "checklist_",
    }
}

pub fn expected_guardrail_command(stage: SpecStage) -> &'static str {
    match stage {
        SpecStage::Plan => "spec-ops-plan",
        SpecStage::Tasks => "spec-ops-tasks",
        SpecStage::Implement => "spec-ops-implement",
        SpecStage::Validate => "spec-ops-validate",
        SpecStage::Audit => "spec-ops-audit",
        SpecStage::Unlock => "spec-ops-unlock",
        SpecStage::Clarify => "quality-clarify",
        SpecStage::Analyze => "quality-analyze",
        SpecStage::Checklist => "quality-checklist",
    }
}

/// Validate that guardrail evidence artifacts exist on disk
pub fn validate_guardrail_evidence(
    cwd: &std::path::Path,
    stage: SpecStage,
    telemetry: &Value,
) -> (Vec<String>, usize) {
    if matches!(stage, SpecStage::Validate) {
        return (Vec::new(), 0);
    }

    let Some(artifacts_value) = telemetry.get("artifacts") else {
        return (vec!["No evidence artifacts recorded".to_string()], 0);
    };
    let Some(artifacts) = artifacts_value.as_array() else {
        return (
            vec!["Telemetry artifacts field is not an array".to_string()],
            0,
        );
    };
    if artifacts.is_empty() {
        return (vec!["Telemetry artifacts array is empty".to_string()], 0);
    }

    let mut failures = Vec::new();
    let mut ok_count = 0usize;
    for (idx, artifact_value) in artifacts.iter().enumerate() {
        let path_opt = match artifact_value {
            Value::String(s) => Some(s.as_str()),
            Value::Object(map) => map.get("path").and_then(|p| p.as_str()),
            _ => None,
        };
        let Some(path_str) = path_opt else {
            failures.push(format!("Artifact #{} missing path", idx + 1));
            continue;
        };

        let raw_path = PathBuf::from(path_str);
        let resolved = if raw_path.is_absolute() {
            raw_path.clone()
        } else {
            cwd.join(&raw_path)
        };
        if resolved.exists() {
            ok_count += 1;
        } else {
            failures.push(format!(
                "Artifact #{} not found at {}",
                idx + 1,
                resolved.display()
            ));
        }
    }

    if ok_count == 0 {
        failures.push("No evidence artifacts found on disk".to_string());
    }

    (failures, ok_count)
}

/// Get nested value from JSON object
pub fn get_nested<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

/// Require a non-empty string field from JSON, adding error if missing
pub fn require_string_field<'a>(
    root: &'a Value,
    path: &[&str],
    errors: &mut Vec<String>,
) -> Option<&'a str> {
    let label = path.join(".");
    match get_nested(root, path).and_then(|value| value.as_str()) {
        Some(value) if !value.trim().is_empty() => Some(value),
        Some(_) => {
            errors.push(format!("Field {label} must be a non-empty string"));
            None
        }
        None => {
            errors.push(format!("Missing required string field {label}"));
            None
        }
    }
}

/// Require an object field from JSON, adding error if missing
pub fn require_object<'a>(
    root: &'a Value,
    path: &[&str],
    errors: &mut Vec<String>,
) -> Option<&'a serde_json::Map<String, Value>> {
    let label = path.join(".");
    match get_nested(root, path).and_then(|value| value.as_object()) {
        Some(map) => Some(map),
        None => {
            errors.push(format!("Missing required object field {label}"));
            None
        }
    }
}

use codex_core::config_types::ShellEnvironmentPolicy;

/// Check if spec-kit telemetry is enabled via env or config
pub fn spec_kit_telemetry_enabled(env_policy: &ShellEnvironmentPolicy) -> bool {
    if let Ok(value) = std::env::var("SPEC_KIT_TELEMETRY_ENABLED")
        && super::consensus::telemetry_value_truthy(&value)
    {
        return true;
    }

    if let Some(value) = env_policy.r#set.get("SPEC_KIT_TELEMETRY_ENABLED")
        && super::consensus::telemetry_value_truthy(value)
    {
        return true;
    }

    false
}

/// Check if spec-kit auto-commit is enabled via env or config
/// Defaults to true for automated workflows (SPEC-KIT-922)
pub fn spec_kit_auto_commit_enabled(env_policy: &ShellEnvironmentPolicy) -> bool {
    // Check environment variable first (explicit override)
    if let Ok(value) = std::env::var("SPEC_KIT_AUTO_COMMIT") {
        return super::consensus::telemetry_value_truthy(&value);
    }

    // Check config override
    if let Some(value) = env_policy.r#set.get("SPEC_KIT_AUTO_COMMIT") {
        return super::consensus::telemetry_value_truthy(value);
    }

    // Default to true (enabled by default for clean tree maintenance)
    true
}
