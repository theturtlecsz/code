//! Integration tests for SPEC-KIT-102 Stage 0 pipeline
//!
//! Tests cover:
//! 1. CLI flag parsing (--no-stage0, --stage0-explain)
//! 2. Stage0 context injection into agent prompts
//! 3. Evidence file writing (TASK_BRIEF.md, DIVINE_TRUTH.md)
//! 4. ExecutionLogger Stage0 events
//! 5. Stage0 disabled behavior

mod common;

use codex_tui::{
    ExecutionEvent, SpecStage, Stage0ExecutionConfig, parse_spec_auto_args,
    write_divine_truth_to_evidence, write_task_brief_to_evidence,
};
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: CLI Flag Parsing
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn stage0_flags_parsed_correctly() {
    // Test --no-stage0 flag
    let invocation = parse_spec_auto_args("SPEC-102 --no-stage0").unwrap();
    assert!(invocation.no_stage0);
    assert!(!invocation.stage0_explain);

    // Test --stage0-explain flag
    let invocation = parse_spec_auto_args("SPEC-102 --stage0-explain").unwrap();
    assert!(!invocation.no_stage0);
    assert!(invocation.stage0_explain);

    // Test both flags together
    let invocation = parse_spec_auto_args("SPEC-102 --no-stage0 --stage0-explain").unwrap();
    assert!(invocation.no_stage0);
    assert!(invocation.stage0_explain);
}

#[test]
fn stage0_flags_default_to_false() {
    let invocation = parse_spec_auto_args("SPEC-102 some goal").unwrap();
    assert!(!invocation.no_stage0);
    assert!(!invocation.stage0_explain);
}

#[test]
fn stage0_flags_with_other_args() {
    // Combine with --from and goal
    let invocation =
        parse_spec_auto_args("SPEC-102 --from tasks --no-stage0 implement feature").unwrap();
    assert!(invocation.no_stage0);
    assert_eq!(invocation.goal, "implement feature");
    assert_eq!(invocation.resume_from, SpecStage::Tasks);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Stage0ExecutionConfig
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn stage0_config_default_values() {
    let config = Stage0ExecutionConfig::default();
    assert!(!config.disabled);
    assert!(!config.explain);
}

#[test]
fn stage0_config_from_flags() {
    let config = Stage0ExecutionConfig {
        disabled: true,
        explain: true,
    };
    assert!(config.disabled);
    assert!(config.explain);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Evidence File Writing
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn write_task_brief_creates_file() {
    let temp_dir = TempDir::new().unwrap();
    let spec_id = "SPEC-TEST-001";
    let task_brief = "# Task Brief\n\nThis is a test task brief.";

    let result = write_task_brief_to_evidence(spec_id, temp_dir.path(), task_brief);
    assert!(result.is_ok());

    let path = result.unwrap();
    assert!(path.exists());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), task_brief);
}

#[test]
fn write_divine_truth_creates_file() {
    let temp_dir = TempDir::new().unwrap();
    let spec_id = "SPEC-TEST-002";
    let divine_truth = "# Divine Truth\n\nThis is the Tier 2 synthesis.";

    let result = write_divine_truth_to_evidence(spec_id, temp_dir.path(), divine_truth);
    assert!(result.is_ok());

    let path = result.unwrap();
    assert!(path.exists());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), divine_truth);
}

#[test]
fn write_both_evidence_files() {
    let temp_dir = TempDir::new().unwrap();
    let spec_id = "SPEC-TEST-003";
    let task_brief = "Task brief content";
    let divine_truth = "Divine truth content";

    // Write both files
    let task_brief_path =
        write_task_brief_to_evidence(spec_id, temp_dir.path(), task_brief).unwrap();
    let divine_truth_path =
        write_divine_truth_to_evidence(spec_id, temp_dir.path(), divine_truth).unwrap();

    // Verify both exist in same evidence directory
    assert!(task_brief_path.exists());
    assert!(divine_truth_path.exists());
    assert_eq!(
        task_brief_path.parent().unwrap(),
        divine_truth_path.parent().unwrap()
    );

    // Verify evidence directory structure
    let evidence_dir = task_brief_path.parent().unwrap();
    assert!(evidence_dir.ends_with("evidence"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: ExecutionLogger Stage0 Events
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn execution_event_stage0_start_serializes() {
    let event = ExecutionEvent::Stage0Start {
        run_id: "run_test_123".to_string(),
        spec_id: "SPEC-102".to_string(),
        tier2_enabled: true,
        explain_enabled: false,
        timestamp: "2025-12-01T00:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("stage0_start"));
    assert!(json.contains("SPEC-102"));
    assert!(json.contains("tier2_enabled"));
}

#[test]
fn execution_event_stage0_complete_serializes() {
    let event = ExecutionEvent::Stage0Complete {
        run_id: "run_test_123".to_string(),
        spec_id: "SPEC-102".to_string(),
        duration_ms: 150,
        tier2_used: true,
        cache_hit: false,
        hybrid_used: true, // P84: Added for V2.5b hybrid retrieval signaling
        memories_used: 5,
        task_brief_written: true,
        skip_reason: None,
        timestamp: "2025-12-01T00:00:01Z".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("stage0_complete"));
    assert!(json.contains("SPEC-102"));
    assert!(json.contains("tier2_used"));
    assert!(json.contains("hybrid_used")); // P84: Verify hybrid_used is serialized
    assert!(json.contains("memories_used"));
    assert!(json.contains("task_brief_written"));
}

#[test]
fn execution_event_stage0_complete_with_skip_reason() {
    let event = ExecutionEvent::Stage0Complete {
        run_id: "run_test_456".to_string(),
        spec_id: "SPEC-103".to_string(),
        duration_ms: 0,
        tier2_used: false,
        cache_hit: false,
        hybrid_used: false, // P84: Added for V2.5b hybrid retrieval signaling
        memories_used: 0,
        task_brief_written: false,
        skip_reason: Some("Stage 0 disabled by flag".to_string()),
        timestamp: "2025-12-01T00:00:02Z".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("stage0_complete"));
    assert!(json.contains("skip_reason"));
    assert!(json.contains("Stage 0 disabled by flag"));
}

#[test]
fn execution_event_run_id_extraction() {
    let event = ExecutionEvent::Stage0Start {
        run_id: "run_extract_test".to_string(),
        spec_id: "SPEC-104".to_string(),
        tier2_enabled: false,
        explain_enabled: true,
        timestamp: "2025-12-01T00:00:00Z".to_string(),
    };

    assert_eq!(event.run_id(), "run_extract_test");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Stage0 Integration with State
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn stage0_result_combined_context_md() {
    use codex_stage0::{DivineTruth, Stage0Result};

    let result = Stage0Result {
        spec_id: "SPEC-105".to_string(),
        divine_truth: DivineTruth {
            executive_summary: "Executive summary".to_string(),
            architectural_guardrails: "Guardrails".to_string(),
            historical_context: "History".to_string(),
            risks_and_questions: "Risks".to_string(),
            suggested_links: vec![],
            raw_markdown: "Divine truth content".to_string(),
        },
        task_brief_md: "Task brief content".to_string(),
        memories_used: vec!["mem-1".to_string()],
        cache_hit: false,
        tier2_used: true,
        latency_ms: 100,
        explain_scores: None,
    };

    let combined = result.combined_context_md();

    // Combined context should include both task brief and divine truth
    assert!(combined.contains("Task brief content"));
    // Divine truth is included when tier2_used is true and not fallback
    assert!(combined.contains("Divine truth content"));
}

#[test]
fn stage0_result_has_context() {
    use codex_stage0::{DivineTruth, Stage0Result};

    // Result with memories
    let result = Stage0Result {
        spec_id: "SPEC-106".to_string(),
        divine_truth: DivineTruth {
            executive_summary: String::new(),
            architectural_guardrails: String::new(),
            historical_context: String::new(),
            risks_and_questions: String::new(),
            suggested_links: vec![],
            raw_markdown: "(Fallback) Tier2 unavailable".to_string(),
        },
        task_brief_md: String::new(),
        memories_used: vec!["mem-1".to_string()],
        cache_hit: false,
        tier2_used: false,
        latency_ms: 50,
        explain_scores: None,
    };

    assert!(result.has_context());

    // Result without context
    let empty_result = Stage0Result {
        spec_id: "SPEC-107".to_string(),
        divine_truth: DivineTruth {
            executive_summary: String::new(),
            architectural_guardrails: String::new(),
            historical_context: String::new(),
            risks_and_questions: String::new(),
            suggested_links: vec![],
            raw_markdown: String::new(),
        },
        task_brief_md: String::new(),
        memories_used: vec![],
        cache_hit: false,
        tier2_used: false,
        latency_ms: 0,
        explain_scores: None,
    };

    assert!(!empty_result.has_context());
}
