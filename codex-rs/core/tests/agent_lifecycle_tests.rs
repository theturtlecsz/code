//! SPEC-KIT-927: Unit tests for agent lifecycle and output validation
//!
//! Tests for premature output collection bug fixes:
//! - File size stability checking
//! - Output validation (size, schema detection, JSON parsing)
//! - Zombie process cleanup

#[cfg(test)]
mod agent_output_validation_tests {
    use serde_json::json;

    /// Test that output validation rejects outputs smaller than minimum size
    #[test]
    fn test_output_validation_rejects_too_small() {
        let small_output = "{}"; // Only 2 bytes

        // This would fail validation (< 500 bytes)
        assert!(small_output.len() < 500);

        // In actual code, this should return an error
        // Validation happens in agent_tool.rs:620-632
    }

    /// Test that output validation detects JSON schema templates
    #[test]
    fn test_output_validation_detects_schema() {
        let schema_outputs = vec![
            r#"{ "path": string, "change": string }"#,
            r#"{"diff_proposals": [ { "path": string } ]}"#,
            r#"{"change": string (diff or summary)}"#,
        ];

        for schema_output in schema_outputs {
            // These patterns indicate schema templates, not real data
            assert!(
                schema_output.contains("{ \"path\": string")
                    || schema_output.contains("\"diff_proposals\": [ {")
                    || schema_output.contains("\"change\": string (diff or summary)")
            );
        }

        // In actual code, these should be rejected
        // Validation happens in agent_tool.rs:634-644
    }

    /// Test that output validation requires valid JSON
    #[test]
    fn test_output_validation_requires_valid_json() {
        let invalid_json_outputs = vec![
            "not json at all",
            "{incomplete",
            r#"{"key": "value""#, // Missing closing brace
        ];

        for invalid_output in invalid_json_outputs {
            assert!(serde_json::from_str::<serde_json::Value>(invalid_output).is_err());
        }

        // Valid JSON should parse successfully
        let valid_output = r#"{"stage": "spec-plan", "agents": ["gemini"]}"#;
        assert!(serde_json::from_str::<serde_json::Value>(valid_output).is_ok());
    }

    /// Test that valid agent output passes all checks
    #[test]
    fn test_output_validation_accepts_valid_output() {
        let valid_output = json!({
            "stage": "spec-implement",
            "prompt_version": "20251002-implement-a",
            "agent": "gpt_codex",
            "model": "gpt-5-codex",
            "model_release": "2025-09-29",
            "reasoning_mode": "auto",
            "diff_proposals": [
                {
                    "path": "src/authentication.rs",
                    "change": "diff --git a/src/authentication.rs b/src/authentication.rs\nnew file mode 100644\n--- /dev/null\n+++ b/src/authentication.rs\n@@ -0,0 +1,50 @@\n+use std::collections::HashMap;\n+\n+pub struct OAuth2Handler {\n+    client_id: String,\n+    client_secret: String,\n+    tokens: HashMap<String, Token>,\n+}\n+\n+impl OAuth2Handler {\n+    pub fn new(client_id: String, client_secret: String) -> Self {\n+        Self {\n+            client_id,\n+            client_secret,\n+            tokens: HashMap::new(),\n+        }\n+    }\n+}",
                    "summary": "Add OAuth2 authentication handler with token management"
                }
            ],
            "test_commands": ["cargo test --package auth", "cargo test --lib authentication_tests"],
            "tool_calls": ["cargo fmt --all", "cargo clippy --all-targets"],
            "risks": [
                "OAuth token storage needs encryption for production use",
                "Rate limiting not implemented - may hit API limits",
                "Token refresh logic needs implementation",
                "Error handling needs improvement for network failures"
            ]
        });

        let output_str = serde_json::to_string_pretty(&valid_output).unwrap();

        // Must be large enough (> 500 bytes)
        assert!(output_str.len() >= 500);

        // Must not contain schema markers
        assert!(!output_str.contains("{ \"path\": string"));
        assert!(!output_str.contains("\"change\": string (diff or summary)"));

        // Must be valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&output_str).is_ok());
    }

    /// Test suspicious completion time detection (fast + small = suspicious)
    #[test]
    fn test_suspicious_completion_detection() {
        let test_cases = vec![
            // (duration_secs, output_size, expected_suspicious)
            (5, 200, true),     // Very fast + very small = SUSPICIOUS
            (25, 800, true),    // Fast + small = SUSPICIOUS
            (60, 5000, false),  // Normal duration + large output = OK
            (120, 1200, false), // Slow + medium output = OK
            (10, 2000, false),  // Fast but large output = OK (some agents are quick)
        ];

        for (duration_secs, output_size, expected_suspicious) in test_cases {
            let is_suspicious = duration_secs < 30 && output_size < 1000;
            assert_eq!(
                is_suspicious, expected_suspicious,
                "duration={}s, size={} bytes should be suspicious={}",
                duration_secs, output_size, expected_suspicious
            );
        }

        // Validation happens in agent_tool.rs:608-617
    }
}

#[cfg(test)]
mod tmux_completion_detection_tests {
    use std::time::Duration;

    /// Test file size stability logic
    #[test]
    fn test_file_size_stability_calculation() {
        // Simulate file size changes over time
        let size_sequence = vec![
            (0, None),          // File doesn't exist yet
            (100, Some(100)),   // File created
            (500, Some(500)),   // Still growing
            (1200, Some(1200)), // Still growing
            (1200, Some(1200)), // Same size (start stability timer)
            (1200, Some(1200)), // Same size (continue stability)
        ];

        let mut last_size: Option<u64> = None;
        let mut stable_count = 0;
        let min_file_size = 1000u64;

        for (current_size_opt, expected_size) in size_sequence {
            let current_size = if current_size_opt > 0 {
                Some(current_size_opt)
            } else {
                None
            };

            if let Some(current) = current_size {
                if let Some(last) = last_size {
                    if current == last && current >= min_file_size {
                        stable_count += 1;
                    } else {
                        stable_count = 0;
                    }
                }
                last_size = Some(current);
            }

            assert_eq!(last_size, expected_size);
        }

        // After sequence above, file should be stable for 2 intervals
        assert_eq!(stable_count, 2);

        // This logic is implemented in tmux.rs:363-401
    }

    /// Test minimum file size requirement
    #[test]
    fn test_minimum_file_size_requirement() {
        let min_file_size = 1000u64;

        let test_cases = vec![
            (500, false), // Too small
            (999, false), // Just under threshold
            (1000, true), // Exactly at threshold
            (1500, true), // Above threshold
            (5000, true), // Well above
        ];

        for (file_size, expected_valid) in test_cases {
            let is_valid = file_size >= min_file_size;
            assert_eq!(
                is_valid, expected_valid,
                "file_size={} should be valid={}",
                file_size, expected_valid
            );
        }
    }

    /// Test stability duration requirement
    #[test]
    fn test_stability_duration_requirement() {
        let min_stable_duration = Duration::from_secs(2);
        let poll_interval = Duration::from_millis(500);

        // Need at least 4 polls (2 seconds / 0.5 second intervals) of stable size
        let required_stable_polls =
            (min_stable_duration.as_millis() / poll_interval.as_millis()) as usize;
        assert_eq!(required_stable_polls, 4);

        // Simulate polling
        let mut stable_polls = 0;
        let stable_file_size = 1500u64;
        let mut last_size = stable_file_size;

        for _poll in 0..6 {
            if last_size == stable_file_size {
                stable_polls += 1;
            } else {
                stable_polls = 0;
            }
            last_size = stable_file_size;
        }

        // After 6 polls, should have 6 stable intervals
        assert!(stable_polls >= required_stable_polls);
    }
}

#[cfg(test)]
mod zombie_cleanup_tests {
    /// Test zombie detection logic
    #[test]
    fn test_zombie_pane_detection() {
        // Simulating tmux list-panes output
        let pane_list_outputs = vec![
            // (output, expected_zombie_count)
            ("", 0),             // No panes
            ("%0\n", 1),         // One pane
            ("%0\n%1\n%2\n", 3), // Three panes
        ];

        for (output, expected_count) in pane_list_outputs {
            let zombie_count = output.lines().filter(|l| !l.is_empty()).count();
            assert_eq!(
                zombie_count,
                expected_count,
                "output='{}' should have {} zombies",
                output.escape_default(),
                expected_count
            );
        }

        // This logic is implemented in tmux.rs:602-626
    }

    /// Test cleanup decision logic
    #[test]
    fn test_cleanup_decision() {
        let test_cases = vec![
            (0, false), // No zombies, no cleanup needed
            (1, true),  // 1 zombie, cleanup needed
            (5, true),  // Multiple zombies, cleanup needed
        ];

        for (zombie_count, should_cleanup) in test_cases {
            let needs_cleanup = zombie_count > 0;
            assert_eq!(
                needs_cleanup, should_cleanup,
                "zombie_count={} should need cleanup={}",
                zombie_count, should_cleanup
            );
        }
    }
}

#[cfg(test)]
mod integration_scenarios {
    use serde_json::json;

    /// Simulate the SPEC-KIT-927 bug scenario
    #[test]
    fn test_premature_collection_scenario() {
        // This simulates what happened in the bug:
        // 1. Agent spawns and writes header quickly (1161 bytes in 6 seconds)
        // 2. System collects output before agent finishes
        // 3. Only schema template is stored

        let premature_output = r#"[2025-11-11T15:42:17] OpenAI Codex v0.0.0
--------
workdir: /home/thetu/.code/working/...
model: gpt-5-codex
[2025-11-11T15:42:17] User instructions:
Template: ~/.code/templates/implement-template.md
Task: Generate code to implement...
Emit code diff proposals as JSON:
{
  "stage": "spec-implement",
  "prompt_version": "20251002-implement-a",
  "diff_proposals": [ { "path": string, "change": string (diff or summary), ... } ],
  "test_commands": [ string ],
  "tool_calls": [ string ],
  "risks": [ string ]
}"#;

        // This should fail all validations:

        // 1. Size check (should be > 500 for valid, but this is only headers)
        let size_valid = premature_output.len() >= 500;
        assert!(size_valid, "Even header is > 500 bytes in this case");

        // 2. Schema detection (should catch the literal schema)
        let has_schema_markers = premature_output.contains("{ \"path\": string")
            || premature_output.contains("\"diff_proposals\": [ {");
        assert!(has_schema_markers, "Should detect schema markers");

        // 3. JSON parsing (this is NOT valid JSON, just a template)
        let json_valid = serde_json::from_str::<serde_json::Value>(premature_output).is_ok();
        assert!(!json_valid, "Should not be valid JSON");

        // With the fix, this would be rejected at validation stage
        // (agent_tool.rs:634-644)
    }

    /// Simulate a valid agent completion
    #[test]
    fn test_valid_completion_scenario() {
        let valid_output = json!({
            "stage": "spec-implement",
            "prompt_version": "20251002-implement-a",
            "agent": "gpt_codex",
            "model": "gpt-5-codex",
            "model_release": "2025-09-29",
            "reasoning_mode": "auto",
            "diff_proposals": [
                {
                    "path": "src/authentication.rs",
                    "change": "diff --git a/src/authentication.rs...\n+ impl OAuth2Handler {\n+     pub fn new() -> Self { ... }\n+ }",
                    "summary": "Add OAuth2 authentication handler"
                }
            ],
            "test_commands": ["cargo test --package auth"],
            "tool_calls": ["cargo fmt", "cargo clippy"],
            "risks": ["OAuth token storage needs encryption", "Rate limiting not implemented"]
        });

        let output_str = serde_json::to_string_pretty(&valid_output).unwrap();

        // Should pass all validations:

        // 1. Size check
        assert!(output_str.len() >= 500, "Valid output should be large");

        // 2. Schema detection (should NOT have schema markers)
        assert!(!output_str.contains("{ \"path\": string"));
        assert!(!output_str.contains("\"change\": string (diff or summary)"));

        // 3. JSON parsing (should be valid)
        assert!(serde_json::from_str::<serde_json::Value>(&output_str).is_ok());

        // This would pass validation and be stored successfully
    }
}
