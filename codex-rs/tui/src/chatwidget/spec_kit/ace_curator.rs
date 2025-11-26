//! ACE Curator - Strategic playbook management with LLM
//!
//! Makes intelligent decisions about playbook updates based on reflection insights.
//! Handles bullet creation, deprecation, merging, and score adjustments.

use super::ace_client::PlaybookBullet;
use super::ace_reflector::{PatternKind, ReflectionResult};
use serde::{Deserialize, Serialize};

/// Curation decision about playbook updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurationDecision {
    /// New bullets to add to playbook
    pub bullets_to_add: Vec<NewBulletSpec>,

    /// Bullet IDs to deprecate (remove from active use)
    pub bullets_to_deprecate: Vec<i32>,

    /// Bullets to merge (combine redundant bullets)
    pub bullets_to_merge: Vec<MergeBulletSpec>,

    /// Score adjustments based on strategic analysis
    pub score_adjustments: Vec<ScoreAdjustment>,

    /// Rationale for decisions
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBulletSpec {
    pub text: String,
    pub kind: String, // "helpful" | "harmful"
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeBulletSpec {
    pub source_ids: Vec<i32>,
    pub merged_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreAdjustment {
    pub bullet_id: i32,
    pub delta: f64,
    pub reason: String,
}

/// Curation prompt builder
pub struct CurationPromptBuilder {
    reflection: ReflectionResult,
    current_bullets: Vec<PlaybookBullet>,
    scope: String,
}

impl CurationPromptBuilder {
    pub fn new(
        reflection: ReflectionResult,
        current_bullets: Vec<PlaybookBullet>,
        scope: String,
    ) -> Self {
        Self {
            reflection,
            current_bullets,
            scope,
        }
    }

    /// Build curation prompt for LLM
    pub fn build(&self) -> String {
        let mut prompt = r#"ROLE: ACE Curator - Strategic playbook management

TASK: Decide how to update the playbook based on reflection insights.

## Reflection Insights

### Patterns Discovered
"#
        .to_string();

        for pattern in &self.reflection.patterns {
            prompt.push_str(&format!(
                "- [{}] {} (confidence: {:.2})\n  Rationale: {}\n",
                match pattern.kind {
                    PatternKind::Helpful => "helpful",
                    PatternKind::Harmful => "harmful",
                    PatternKind::Neutral => "neutral",
                },
                pattern.pattern,
                pattern.confidence,
                pattern.rationale
            ));
        }

        prompt.push_str("\n### Current Playbook\n");

        if self.current_bullets.is_empty() {
            prompt.push_str("(Empty - no bullets yet)\n");
        } else {
            for bullet in &self.current_bullets {
                prompt.push_str(&format!(
                    "- [ID:{}] {} (score: {:.2}, pinned: {})\n",
                    bullet.id.unwrap_or(-1),
                    bullet.text,
                    bullet.confidence,
                    false // We don't have pinned field in our struct yet
                ));
            }
        }

        prompt.push_str(&format!(
            r#"
## Your Task

Decide strategic playbook updates for scope: {}

### Curation Goals
1. **Add valuable patterns**: Create bullets from high-confidence patterns
2. **Remove redundancy**: Deprecate bullets made obsolete by new patterns
3. **Merge duplicates**: Combine similar bullets into clearer statements
4. **Adjust scores**: Strategic scoring beyond simple +/-

### Decision Criteria

**Add bullet when**:
- Pattern confidence ≥ 0.7
- Pattern is generalizable (not task-specific)
- Not redundant with existing bullets
- Adds actionable value

**Deprecate bullet when**:
- Made obsolete by new pattern
- Consistently fails to help
- Too specific to be useful

**Merge bullets when**:
- Multiple bullets say similar things
- Can combine into clearer statement

**Adjust scores when**:
- Strategic importance changed
- Evidence of misalignment

## Output Format

Return JSON:
```json
{{
  "bullets_to_add": [
    {{
      "text": "Use tokio::sync::Mutex in async contexts",
      "kind": "helpful",
      "scope": "implement"
    }}
  ],
  "bullets_to_deprecate": [42, 43],
  "bullets_to_merge": [
    {{
      "source_ids": [15, 16],
      "merged_text": "Validate inputs before processing in all public APIs"
    }}
  ],
  "score_adjustments": [
    {{
      "bullet_id": 10,
      "delta": 0.5,
      "reason": "Pattern proved crucial in preventing async deadlocks"
    }}
  ],
  "rationale": "Adding async pattern, deprecating outdated sync advice, merging validation bullets"
}}
```

## Guidelines

- Be conservative: Only add high-value patterns
- Prefer merging over proliferation
- Deprecate proactively to keep playbook lean
- Score adjustments should be strategic (not just mechanical)
- Maximum 3 new bullets per curation cycle
- Rationale should explain the "why"

## Begin Curation
"#,
            self.scope
        ));

        prompt
    }
}

/// Parse LLM response into CurationDecision
pub fn parse_curation_response(response: &str) -> Result<CurationDecision, String> {
    // Try to extract JSON from response
    let json_start = response.find('{');
    let json_end = response.rfind('}');

    if let (Some(start), Some(end)) = (json_start, json_end) {
        let json_str = &response[start..=end];
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse curation JSON: {}", e))
    } else {
        Err("No JSON found in curation response".to_string())
    }
}

/// Check if curation should be triggered
pub fn should_curate(reflection: &ReflectionResult) -> bool {
    // Curate if we discovered any high-confidence patterns
    reflection.patterns.iter().any(|p| p.confidence >= 0.7)
}

#[cfg(test)]
mod tests {
    use super::super::ace_reflector::ReflectedPattern;

    use super::*;

    #[test]
    fn test_should_curate_with_high_confidence() {
        let reflection = ReflectionResult {
            patterns: vec![ReflectedPattern {
                pattern: "Test pattern".to_string(),
                rationale: "Because".to_string(),
                kind: PatternKind::Helpful,
                confidence: 0.9,
                scope: "implement".to_string(),
            }],
            successes: vec![],
            failures: vec![],
            recommendations: vec![],
            summary: "Good".to_string(),
        };

        assert!(should_curate(&reflection));
    }

    #[test]
    fn test_should_not_curate_with_low_confidence() {
        let reflection = ReflectionResult {
            patterns: vec![ReflectedPattern {
                pattern: "Low confidence pattern".to_string(),
                rationale: "Uncertain".to_string(),
                kind: PatternKind::Neutral,
                confidence: 0.3,
                scope: "global".to_string(),
            }],
            successes: vec![],
            failures: vec![],
            recommendations: vec![],
            summary: "Uncertain".to_string(),
        };

        assert!(!should_curate(&reflection));
    }

    #[test]
    fn test_curation_prompt_includes_reflection() {
        let reflection = ReflectionResult {
            patterns: vec![ReflectedPattern {
                pattern: "Use async".to_string(),
                rationale: "Better perf".to_string(),
                kind: PatternKind::Helpful,
                confidence: 0.8,
                scope: "implement".to_string(),
            }],
            successes: vec!["Fast".to_string()],
            failures: vec!["Slow sync".to_string()],
            recommendations: vec!["Add async".to_string()],
            summary: "Good progress".to_string(),
        };

        let bullets = vec![PlaybookBullet {
            id: Some(1),
            text: "Old bullet".to_string(),
            helpful: true,
            harmful: false,
            confidence: 0.5,
            source: None,
        }];

        let builder = CurationPromptBuilder::new(reflection, bullets, "implement".to_string());
        let prompt = builder.build();

        assert!(prompt.contains("Use async"));
        assert!(prompt.contains("Better perf"));
        assert!(prompt.contains("Old bullet"));
        assert!(prompt.contains("scope: implement"));
    }

    #[test]
    fn test_parse_curation_response() {
        let response = r#"
Based on analysis:

```json
{
  "bullets_to_add": [
    {
      "text": "Use tokio::Mutex",
      "kind": "helpful",
      "scope": "implement"
    }
  ],
  "bullets_to_deprecate": [5],
  "bullets_to_merge": [],
  "score_adjustments": [
    {
      "bullet_id": 10,
      "delta": 0.5,
      "reason": "Proved valuable"
    }
  ],
  "rationale": "Adding async pattern"
}
```
"#;

        let decision = parse_curation_response(response).unwrap();
        assert_eq!(decision.bullets_to_add.len(), 1);
        assert_eq!(decision.bullets_to_add[0].text, "Use tokio::Mutex");
        assert_eq!(decision.bullets_to_deprecate, vec![5]);
        assert_eq!(decision.score_adjustments.len(), 1);
    }
}
