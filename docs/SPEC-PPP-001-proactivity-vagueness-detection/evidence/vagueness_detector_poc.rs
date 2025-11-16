// Proof of Concept: Vagueness Detection & Question Effort Classification
// SPEC: SPEC-PPP-001
// Purpose: Demonstrate heuristic-based proactivity detection for PPP Framework

use regex::Regex;
use std::collections::HashMap;

// ============================================================================
// 1. VAGUENESS DETECTION
// ============================================================================

#[derive(Debug, Clone)]
pub struct VaguenessResult {
    pub is_vague: bool,
    pub score: f32,
    pub indicators: Vec<String>,
}

pub struct VaguenessDetector {
    threshold: f32,
    vague_verbs: Vec<&'static str>,
    ambiguous_quantifiers: Vec<&'static str>,
    missing_oauth_version: Regex,
    missing_db_type: Regex,
    missing_auth_type: Regex,
}

impl Default for VaguenessDetector {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl VaguenessDetector {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            vague_verbs: vec![
                "implement", "add", "make", "create", "do", "build",
                "fix", "update", "change", "improve", "handle", "setup",
            ],
            ambiguous_quantifiers: vec![
                "some", "a few", "several", "many", "better", "good",
                "fast", "slow", "big", "small", "more", "less",
            ],
            missing_oauth_version: Regex::new(r"(?i)\bOAuth\b(?!\s*(2|1\.0))").unwrap(),
            missing_db_type: Regex::new(
                r"(?i)\bdatabase\b(?!\s*(SQL|NoSQL|PostgreSQL|MySQL|MongoDB))"
            ).unwrap(),
            missing_auth_type: Regex::new(
                r"(?i)\bauth(?:entication)?\b(?!\s*(JWT|OAuth|SAML|Basic))"
            ).unwrap(),
        }
    }

    pub fn detect(&self, prompt: &str) -> VaguenessResult {
        let mut score = 0.0;
        let mut indicators = Vec::new();

        let lower = prompt.to_lowercase();

        // Check vague verbs (0.2 per match, max 1 match)
        for verb in &self.vague_verbs {
            if lower.contains(verb) {
                score += 0.2;
                indicators.push(format!("vague-verb:{}", verb));
                break; // Only count first match
            }
        }

        // Check missing context patterns (0.3 per match)
        if self.missing_oauth_version.is_match(prompt) {
            score += 0.3;
            indicators.push("missing-oauth-version".to_string());
        }
        if self.missing_db_type.is_match(prompt) {
            score += 0.3;
            indicators.push("missing-database-type".to_string());
        }
        if self.missing_auth_type.is_match(prompt) {
            score += 0.3;
            indicators.push("missing-auth-type".to_string());
        }

        // Check ambiguous quantifiers (0.1 per match, max 2)
        let quant_matches: Vec<&&str> = self.ambiguous_quantifiers.iter()
            .filter(|q| lower.contains(**q))
            .take(2)
            .collect();

        score += 0.1 * quant_matches.len() as f32;
        for quant in quant_matches {
            indicators.push(format!("ambiguous-quantifier:{}", quant));
        }

        // Clamp score to [0.0, 1.0]
        let final_score = score.min(1.0);
        let is_vague = final_score > self.threshold;

        VaguenessResult {
            is_vague,
            score: final_score,
            indicators,
        }
    }

    pub fn is_vague(&self, prompt: &str) -> bool {
        self.detect(prompt).is_vague
    }

    pub fn vagueness_score(&self, prompt: &str) -> f32 {
        self.detect(prompt).score
    }
}

// ============================================================================
// 2. QUESTION EFFORT CLASSIFICATION
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortLevel {
    Low,    // Selection, accessible context
    Medium, // Research, preferences
    High,   // Investigation, blocking
}

pub struct EffortClassifier {
    low_indicators: Vec<&'static str>,
    high_indicators: Vec<&'static str>,
}

impl Default for EffortClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl EffortClassifier {
    pub fn new() -> Self {
        Self {
            low_indicators: vec!["which", "choose", "select", "prefer", "option"],
            high_indicators: vec![
                "investigate", "research", "before proceeding", "blocking",
                "need to decide", "architecture", "trade-off", "strategy",
                "should we consider", "do you want me to research",
            ],
        }
    }

    pub fn classify(&self, question: &str) -> EffortLevel {
        let word_count = question.split_whitespace().count();
        let lower = question.to_lowercase();

        // High-effort indicators (override length)
        for indicator in &self.high_indicators {
            if lower.contains(indicator) {
                return EffortLevel::High;
            }
        }

        // Low-effort indicators (selection questions)
        let has_options = lower.contains(" or ") || lower.contains("option");
        let has_selection_word = self.low_indicators.iter()
            .any(|ind| lower.contains(ind));

        if (has_options || has_selection_word) && word_count < 15 {
            return EffortLevel::Low;
        }

        // Length-based fallback
        match word_count {
            0..=10 => EffortLevel::Low,
            11..=20 => EffortLevel::Medium,
            _ => EffortLevel::High,
        }
    }
}

// ============================================================================
// 3. PROACTIVITY SCORE CALCULATOR
// ============================================================================

#[derive(Debug, Clone)]
pub struct ProactivityScore {
    pub r_proact: f32,
    pub questions_asked: usize,
    pub low_effort: usize,
    pub medium_effort: usize,
    pub high_effort: usize,
}

pub struct ProactivityCalculator {
    effort_classifier: EffortClassifier,
}

impl Default for ProactivityCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProactivityCalculator {
    pub fn new() -> Self {
        Self {
            effort_classifier: EffortClassifier::new(),
        }
    }

    pub fn calculate(&self, questions: &[String]) -> ProactivityScore {
        if questions.is_empty() {
            return ProactivityScore {
                r_proact: 0.05, // Bonus: no questions
                questions_asked: 0,
                low_effort: 0,
                medium_effort: 0,
                high_effort: 0,
            };
        }

        // Classify each question
        let mut low = 0;
        let mut medium = 0;
        let mut high = 0;

        for question in questions {
            match self.effort_classifier.classify(question) {
                EffortLevel::Low => low += 1,
                EffortLevel::Medium => medium += 1,
                EffortLevel::High => high += 1,
            }
        }

        // Apply PPP formula (arXiv:2511.02208)
        let r_proact = if low == questions.len() {
            0.05 // All low-effort
        } else {
            -0.1 * (medium as f32) - 0.5 * (high as f32)
        };

        ProactivityScore {
            r_proact,
            questions_asked: questions.len(),
            low_effort: low,
            medium_effort: medium,
            high_effort: high,
        }
    }
}

// ============================================================================
// 4. QUESTION EXTRACTOR
// ============================================================================

pub struct QuestionExtractor {
    question_pattern: Regex,
}

impl Default for QuestionExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl QuestionExtractor {
    pub fn new() -> Self {
        Self {
            question_pattern: Regex::new(r"([A-Z][^.!?]*\?)").unwrap(),
        }
    }

    pub fn extract(&self, text: &str) -> Vec<String> {
        self.question_pattern
            .captures_iter(text)
            .map(|cap| cap[1].trim().to_string())
            .filter(|q| q.len() > 10) // Filter out short fragments
            .collect()
    }
}

// ============================================================================
// 5. VALIDATION SCENARIOS
// ============================================================================

#[derive(Debug)]
pub struct ValidationScenario {
    pub name: String,
    pub description: String,
    pub passed: bool,
    pub details: String,
}

pub fn run_validation_scenarios() -> Vec<ValidationScenario> {
    let mut scenarios = Vec::new();

    // Scenario 1: Vagueness Detection - Vague Prompts
    scenarios.push(validate_vague_prompts());

    // Scenario 2: Vagueness Detection - Specific Prompts
    scenarios.push(validate_specific_prompts());

    // Scenario 3: Effort Classification - Low Effort
    scenarios.push(validate_low_effort_questions());

    // Scenario 4: Effort Classification - Medium Effort
    scenarios.push(validate_medium_effort_questions());

    // Scenario 5: Effort Classification - High Effort
    scenarios.push(validate_high_effort_questions());

    // Scenario 6: Proactivity Scoring - No Questions
    scenarios.push(validate_no_questions_bonus());

    // Scenario 7: Proactivity Scoring - All Low Effort
    scenarios.push(validate_all_low_effort_bonus());

    // Scenario 8: Proactivity Scoring - Mixed Effort
    scenarios.push(validate_mixed_effort_penalty());

    // Scenario 9: Question Extraction
    scenarios.push(validate_question_extraction());

    // Scenario 10: End-to-End (Vagueness → Questions → Proactivity)
    scenarios.push(validate_end_to_end());

    scenarios
}

// ============================================================================
// SCENARIO IMPLEMENTATIONS
// ============================================================================

fn validate_vague_prompts() -> ValidationScenario {
    let detector = VaguenessDetector::default();

    let vague_examples = vec![
        "Implement OAuth",
        "Add authentication",
        "Fix the bug",
        "Make it faster",
        "Setup a database",
        "Improve performance",
    ];

    let mut passed_count = 0;
    let mut details = Vec::new();

    for prompt in &vague_examples {
        let result = detector.detect(prompt);
        if result.is_vague {
            passed_count += 1;
            details.push(format!("✓ '{}' → vague (score: {:.2})", prompt, result.score));
        } else {
            details.push(format!("✗ '{}' → specific (score: {:.2}) [FAILED]", prompt, result.score));
        }
    }

    let passed = passed_count == vague_examples.len();

    ValidationScenario {
        name: "Scenario 1: Vague Prompt Detection".to_string(),
        description: format!("Detect {} vague prompts", vague_examples.len()),
        passed,
        details: format!(
            "Detected {}/{} vague prompts\n{}",
            passed_count,
            vague_examples.len(),
            details.join("\n")
        ),
    }
}

fn validate_specific_prompts() -> ValidationScenario {
    let detector = VaguenessDetector::default();

    let specific_examples = vec![
        "Implement OAuth2 with Google provider using PKCE",
        "Add JWT authentication with HS256 signing",
        "Fix null pointer in user_service.rs line 42",
        "Reduce API latency from 200ms to <100ms",
        "Setup PostgreSQL database with connection pooling",
        "Improve query performance by adding index on user_id column",
    ];

    let mut passed_count = 0;
    let mut details = Vec::new();

    for prompt in &specific_examples {
        let result = detector.detect(prompt);
        if !result.is_vague {
            passed_count += 1;
            details.push(format!("✓ '{}' → specific (score: {:.2})", prompt, result.score));
        } else {
            details.push(format!("✗ '{}' → vague (score: {:.2}) [FAILED]", prompt, result.score));
        }
    }

    let passed = passed_count == specific_examples.len();

    ValidationScenario {
        name: "Scenario 2: Specific Prompt Detection".to_string(),
        description: format!("Detect {} specific prompts", specific_examples.len()),
        passed,
        details: format!(
            "Detected {}/{} specific prompts\n{}",
            passed_count,
            specific_examples.len(),
            details.join("\n")
        ),
    }
}

fn validate_low_effort_questions() -> ValidationScenario {
    let classifier = EffortClassifier::new();

    let low_effort_questions = vec![
        "Which database: PostgreSQL or MySQL?",
        "Do you prefer tabs or spaces?",
        "Choose A or B?",
        "Select option 1 or 2?",
    ];

    let mut passed_count = 0;
    let mut details = Vec::new();

    for question in &low_effort_questions {
        let effort = classifier.classify(question);
        if effort == EffortLevel::Low {
            passed_count += 1;
            details.push(format!("✓ '{}' → Low", question));
        } else {
            details.push(format!("✗ '{}' → {:?} [FAILED]", question, effort));
        }
    }

    let passed = passed_count == low_effort_questions.len();

    ValidationScenario {
        name: "Scenario 3: Low-Effort Question Classification".to_string(),
        description: format!("Classify {} low-effort questions", low_effort_questions.len()),
        passed,
        details: format!(
            "Classified {}/{} correctly\n{}",
            passed_count,
            low_effort_questions.len(),
            details.join("\n")
        ),
    }
}

fn validate_medium_effort_questions() -> ValidationScenario {
    let classifier = EffortClassifier::new();

    let medium_effort_questions = vec![
        "What authentication method should we use?",
        "How should we handle errors?",
        "What is your preferred coding style?",
        "Should we use async or sync?",
    ];

    let mut passed_count = 0;
    let mut details = Vec::new();

    for question in &medium_effort_questions {
        let effort = classifier.classify(question);
        if effort == EffortLevel::Medium {
            passed_count += 1;
            details.push(format!("✓ '{}' → Medium", question));
        } else {
            details.push(format!("✗ '{}' → {:?} [FAILED]", question, effort));
        }
    }

    let passed = passed_count == medium_effort_questions.len();

    ValidationScenario {
        name: "Scenario 4: Medium-Effort Question Classification".to_string(),
        description: format!("Classify {} medium-effort questions", medium_effort_questions.len()),
        passed,
        details: format!(
            "Classified {}/{} correctly\n{}",
            passed_count,
            medium_effort_questions.len(),
            details.join("\n")
        ),
    }
}

fn validate_high_effort_questions() -> ValidationScenario {
    let classifier = EffortClassifier::new();

    let high_effort_questions = vec![
        "Should we investigate caching strategies before proceeding?",
        "Do you want me to research distributed tracing solutions?",
        "What architecture patterns should we consider for this microservice?",
        "Should we evaluate different database trade-offs before deciding?",
    ];

    let mut passed_count = 0;
    let mut details = Vec::new();

    for question in &high_effort_questions {
        let effort = classifier.classify(question);
        if effort == EffortLevel::High {
            passed_count += 1;
            details.push(format!("✓ '{}' → High", question));
        } else {
            details.push(format!("✗ '{}' → {:?} [FAILED]", question, effort));
        }
    }

    let passed = passed_count == high_effort_questions.len();

    ValidationScenario {
        name: "Scenario 5: High-Effort Question Classification".to_string(),
        description: format!("Classify {} high-effort questions", high_effort_questions.len()),
        passed,
        details: format!(
            "Classified {}/{} correctly\n{}",
            passed_count,
            high_effort_questions.len(),
            details.join("\n")
        ),
    }
}

fn validate_no_questions_bonus() -> ValidationScenario {
    let calculator = ProactivityCalculator::new();

    let questions: Vec<String> = vec![]; // No questions
    let score = calculator.calculate(&questions);

    let passed = score.r_proact == 0.05
        && score.questions_asked == 0
        && score.low_effort == 0
        && score.medium_effort == 0
        && score.high_effort == 0;

    ValidationScenario {
        name: "Scenario 6: No Questions Bonus".to_string(),
        description: "Agent completes task without asking questions".to_string(),
        passed,
        details: format!(
            "R_Proact: {} (expected: 0.05)\nQuestions asked: {}\nBreakdown: {} low, {} medium, {} high",
            score.r_proact,
            score.questions_asked,
            score.low_effort,
            score.medium_effort,
            score.high_effort
        ),
    }
}

fn validate_all_low_effort_bonus() -> ValidationScenario {
    let calculator = ProactivityCalculator::new();

    let questions = vec![
        "Which database: PostgreSQL or MySQL?".to_string(),
        "Do you prefer tabs or spaces?".to_string(),
    ];

    let score = calculator.calculate(&questions);

    let passed = score.r_proact == 0.05
        && score.questions_asked == 2
        && score.low_effort == 2
        && score.medium_effort == 0
        && score.high_effort == 0;

    ValidationScenario {
        name: "Scenario 7: All Low-Effort Questions Bonus".to_string(),
        description: "Agent asks only selection questions".to_string(),
        passed,
        details: format!(
            "R_Proact: {} (expected: 0.05)\nQuestions asked: {}\nBreakdown: {} low, {} medium, {} high",
            score.r_proact,
            score.questions_asked,
            score.low_effort,
            score.medium_effort,
            score.high_effort
        ),
    }
}

fn validate_mixed_effort_penalty() -> ValidationScenario {
    let calculator = ProactivityCalculator::new();

    let questions = vec![
        "Which provider: Google or GitHub?".to_string(),  // Low
        "What authentication flow should we use?".to_string(),  // Medium
        "Should we investigate distributed caching before proceeding?".to_string(),  // High
    ];

    let score = calculator.calculate(&questions);

    let expected_r_proact = -0.1 * 1.0 - 0.5 * 1.0; // -0.6

    let passed = (score.r_proact - expected_r_proact).abs() < 0.01
        && score.questions_asked == 3
        && score.low_effort == 1
        && score.medium_effort == 1
        && score.high_effort == 1;

    ValidationScenario {
        name: "Scenario 8: Mixed Effort Penalty".to_string(),
        description: "1 low, 1 medium, 1 high effort question".to_string(),
        passed,
        details: format!(
            "R_Proact: {} (expected: {})\nQuestions asked: {}\nBreakdown: {} low, {} medium, {} high",
            score.r_proact,
            expected_r_proact,
            score.questions_asked,
            score.low_effort,
            score.medium_effort,
            score.high_effort
        ),
    }
}

fn validate_question_extraction() -> ValidationScenario {
    let extractor = QuestionExtractor::new();

    let agent_response = "I can help with that. Which provider do you prefer: Google or GitHub? \
                          I'll need to know your authentication flow preferences. \
                          Should we use OAuth2 or SAML?";

    let questions = extractor.extract(agent_response);

    let expected_questions = vec![
        "Which provider do you prefer: Google or GitHub?",
        "Should we use OAuth2 or SAML?",
    ];

    let passed = questions.len() == expected_questions.len();

    ValidationScenario {
        name: "Scenario 9: Question Extraction".to_string(),
        description: "Extract questions from agent response".to_string(),
        passed,
        details: format!(
            "Extracted {} questions (expected: {})\n{}",
            questions.len(),
            expected_questions.len(),
            questions.iter()
                .map(|q| format!("  - {}", q))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

fn validate_end_to_end() -> ValidationScenario {
    // Simulate full workflow
    let vagueness_detector = VaguenessDetector::default();
    let question_extractor = QuestionExtractor::new();
    let proactivity_calculator = ProactivityCalculator::new();

    // Step 1: User provides vague prompt
    let user_prompt = "Implement OAuth";
    let vagueness = vagueness_detector.detect(user_prompt);

    // Step 2: Agent response with clarifying questions
    let agent_response = "I can help implement OAuth. Which version do you need: OAuth 1.0 or OAuth 2.0? \
                          Also, which provider are you integrating with?";

    // Step 3: Extract questions
    let questions = question_extractor.extract(agent_response);

    // Step 4: Calculate proactivity score
    let score = proactivity_calculator.calculate(&questions);

    let passed = vagueness.is_vague
        && questions.len() == 2
        && score.r_proact == 0.05; // All low-effort

    ValidationScenario {
        name: "Scenario 10: End-to-End Workflow".to_string(),
        description: "Vague prompt → Questions → Proactivity scoring".to_string(),
        passed,
        details: format!(
            "Vagueness: {} (score: {:.2})\nQuestions extracted: {}\nR_Proact: {} (expected: 0.05)",
            vagueness.is_vague,
            vagueness.score,
            questions.len(),
            score.r_proact
        ),
    }
}

// ============================================================================
// MAIN: RUN VALIDATION
// ============================================================================

fn main() {
    println!("=============================================================");
    println!("PPP Framework - Proactivity & Vagueness Detection PoC");
    println!("SPEC: SPEC-PPP-001");
    println!("=============================================================\n");

    let scenarios = run_validation_scenarios();

    let total = scenarios.len();
    let passed = scenarios.iter().filter(|s| s.passed).count();

    for (i, scenario) in scenarios.iter().enumerate() {
        println!("{}. {}", i + 1, scenario.name);
        println!("   Description: {}", scenario.description);
        println!("   Status: {}", if scenario.passed { "✓ PASSED" } else { "✗ FAILED" });
        println!("   Details:\n{}", indent(&scenario.details, 6));
        println!();
    }

    println!("=============================================================");
    println!("SUMMARY: {}/{} scenarios passed ({:.1}%)", passed, total, (passed as f32 / total as f32) * 100.0);
    println!("=============================================================");

    if passed == total {
        println!("\n✓ All validation scenarios passed!");
        println!("Phase 1 PoC demonstrates 100% success on test scenarios.");
        println!("Ready for integration with trajectory logging (SPEC-PPP-004).");
    } else {
        println!("\n✗ Some scenarios failed. Review details above.");
        std::process::exit(1);
    }
}

fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vagueness_detector() {
        let detector = VaguenessDetector::default();

        assert!(detector.is_vague("Implement OAuth"));
        assert!(detector.is_vague("Add authentication"));
        assert!(!detector.is_vague("Implement OAuth2 with PKCE"));
    }

    #[test]
    fn test_effort_classifier() {
        let classifier = EffortClassifier::new();

        assert_eq!(
            classifier.classify("Which database: PostgreSQL or MySQL?"),
            EffortLevel::Low
        );
        assert_eq!(
            classifier.classify("What authentication method should we use?"),
            EffortLevel::Medium
        );
        assert_eq!(
            classifier.classify("Should we investigate caching strategies before proceeding?"),
            EffortLevel::High
        );
    }

    #[test]
    fn test_proactivity_calculator() {
        let calculator = ProactivityCalculator::new();

        // No questions
        let score = calculator.calculate(&vec![]);
        assert_eq!(score.r_proact, 0.05);

        // All low-effort
        let score = calculator.calculate(&vec![
            "Which database?".to_string(),
            "Choose A or B?".to_string(),
        ]);
        assert_eq!(score.r_proact, 0.05);

        // Mixed effort
        let score = calculator.calculate(&vec![
            "Which?".to_string(),  // Low
            "What should we use?".to_string(),  // Medium
            "Should we investigate strategies before proceeding?".to_string(),  // High
        ]);
        assert_eq!(score.r_proact, -0.6); // -0.1 * 1 - 0.5 * 1
    }

    #[test]
    fn test_question_extractor() {
        let extractor = QuestionExtractor::new();

        let text = "I can help. Which provider do you prefer? Let me know.";
        let questions = extractor.extract(text);

        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0], "Which provider do you prefer?");
    }

    #[test]
    fn test_all_scenarios_pass() {
        let scenarios = run_validation_scenarios();
        let all_passed = scenarios.iter().all(|s| s.passed);
        assert!(all_passed, "Not all validation scenarios passed");
    }
}
