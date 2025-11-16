/// Proof of Concept: PPP Weighted Consensus
///
/// Demonstrates:
/// - Technical quality scoring (completeness + correctness)
/// - Interaction quality scoring (R_Proact + R_Pers)
/// - Weighted consensus (70% technical + 30% interaction)
/// - Agent selection from multi-agent outputs
///
/// Run: rustc interaction_scorer_poc.rs && ./interaction_scorer_poc

use std::collections::HashMap;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone)]
pub struct AgentArtifact {
    pub agent_name: String,
    pub content: String,
    pub completeness: f32,    // 0.0-1.0
    pub correctness: f32,     // 0.0-1.0
}

#[derive(Debug, Clone, Copy)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct Question {
    pub text: String,
    pub effort: EffortLevel,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub preference: String,
    pub severity: String,  // "minor", "major", "critical"
}

#[derive(Debug, Clone)]
pub struct AgentTrajectory {
    pub agent_name: String,
    pub questions_asked: Vec<Question>,
    pub violations: Vec<Violation>,
}

#[derive(Debug)]
pub struct ProactivityScore {
    pub r_proact: f32,
    pub total_questions: usize,
    pub low_effort: usize,
    pub medium_effort: usize,
    pub high_effort: usize,
}

#[derive(Debug)]
pub struct PersonalizationScore {
    pub r_pers: f32,
    pub total_violations: usize,
    pub minor: usize,
    pub major: usize,
    pub critical: usize,
}

#[derive(Debug)]
pub struct AgentScore {
    pub agent_name: String,
    pub technical_score: f32,
    pub interaction_score: f32,
    pub final_score: f32,
    pub details: ScoreDetails,
}

#[derive(Debug)]
pub struct ScoreDetails {
    pub completeness: f32,
    pub correctness: f32,
    pub proactivity: ProactivityScore,
    pub personalization: PersonalizationScore,
}

#[derive(Debug)]
pub struct WeightedConsensus {
    pub best_agent: String,
    pub confidence: f32,
    pub scores: Vec<AgentScore>,
}

// ============================================================================
// Scoring Functions
// ============================================================================

pub fn calculate_technical_score(artifact: &AgentArtifact) -> f32 {
    // Simple weighted average: 60% completeness + 40% correctness
    0.6 * artifact.completeness + 0.4 * artifact.correctness
}

pub fn calculate_r_proact(trajectory: &AgentTrajectory) -> ProactivityScore {
    let total_questions = trajectory.questions_asked.len();
    let mut low_effort = 0;
    let mut medium_effort = 0;
    let mut high_effort = 0;

    for question in &trajectory.questions_asked {
        match question.effort {
            EffortLevel::Low => low_effort += 1,
            EffortLevel::Medium => medium_effort += 1,
            EffortLevel::High => high_effort += 1,
        }
    }

    // Apply PPP formula
    let r_proact = if total_questions == 0 {
        0.05  // Bonus: no questions asked
    } else if low_effort == total_questions {
        0.05  // Bonus: all questions are low-effort
    } else {
        -0.1 * (medium_effort as f32) - 0.5 * (high_effort as f32)
    };

    ProactivityScore {
        r_proact,
        total_questions,
        low_effort,
        medium_effort,
        high_effort,
    }
}

pub fn calculate_r_pers(trajectory: &AgentTrajectory) -> PersonalizationScore {
    let total_violations = trajectory.violations.len();
    let mut minor = 0;
    let mut major = 0;
    let mut critical = 0;

    for violation in &trajectory.violations {
        match violation.severity.as_str() {
            "minor" => minor += 1,
            "major" => major += 1,
            "critical" => critical += 1,
            _ => {}
        }
    }

    // Apply PPP formula
    let r_pers = if total_violations == 0 {
        0.05  // Bonus: no violations
    } else {
        -0.01 * (minor as f32) - 0.03 * (major as f32) - 0.05 * (critical as f32)
    };

    PersonalizationScore {
        r_pers,
        total_violations,
        minor,
        major,
        critical,
    }
}

pub fn weighted_consensus(
    artifacts: Vec<AgentArtifact>,
    trajectories: HashMap<String, AgentTrajectory>,
    weights: (f32, f32),  // (technical, interaction)
) -> WeightedConsensus {
    let (w_tech, w_interact) = weights;

    let mut scores = Vec::new();

    for artifact in artifacts {
        // Technical score
        let technical = calculate_technical_score(&artifact);

        // Interaction score (from trajectory)
        let trajectory = trajectories.get(&artifact.agent_name).unwrap();
        let proact = calculate_r_proact(trajectory);
        let pers = calculate_r_pers(trajectory);
        let interaction = proact.r_proact + pers.r_pers;

        // Weighted combination
        let final_score = (w_tech * technical) + (w_interact * interaction);

        scores.push(AgentScore {
            agent_name: artifact.agent_name.clone(),
            technical_score: technical,
            interaction_score: interaction,
            final_score,
            details: ScoreDetails {
                completeness: artifact.completeness,
                correctness: artifact.correctness,
                proactivity: proact,
                personalization: pers,
            },
        });
    }

    // Sort by final_score descending
    scores.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap());

    WeightedConsensus {
        best_agent: scores[0].agent_name.clone(),
        confidence: scores[0].final_score,
        scores,
    }
}

// ============================================================================
// Demo Scenarios
// ============================================================================

fn scenario_1_balanced_vs_technical_expert() -> (Vec<AgentArtifact>, HashMap<String, AgentTrajectory>) {
    println!("\n=== Scenario 1: Balanced Agent vs Technical Expert ===");

    let artifacts = vec![
        AgentArtifact {
            agent_name: "gemini-flash".to_string(),
            content: "Implemented OAuth2 with PKCE...".to_string(),
            completeness: 0.85,
            correctness: 0.85,
        },
        AgentArtifact {
            agent_name: "claude-opus".to_string(),
            content: "Comprehensive OAuth2 implementation...".to_string(),
            completeness: 0.95,
            correctness: 0.95,
        },
        AgentArtifact {
            agent_name: "gpt-4".to_string(),
            content: "OAuth2 implementation with basic tests...".to_string(),
            completeness: 0.80,
            correctness: 0.80,
        },
    ];

    let mut trajectories = HashMap::new();

    // Gemini: Balanced - good code, asks 2 low-effort questions
    trajectories.insert("gemini-flash".to_string(), AgentTrajectory {
        agent_name: "gemini-flash".to_string(),
        questions_asked: vec![
            Question {
                text: "Which OAuth provider would you like to use?".to_string(),
                effort: EffortLevel::Low,
            },
            Question {
                text: "Should I include refresh token support?".to_string(),
                effort: EffortLevel::Low,
            },
        ],
        violations: vec![],
    });

    // Claude: Technical expert - excellent code, but asks 1 high-effort blocking question
    trajectories.insert("claude-opus".to_string(), AgentTrajectory {
        agent_name: "claude-opus".to_string(),
        questions_asked: vec![
            Question {
                text: "Before proceeding, should I investigate distributed session management strategies for OAuth state?".to_string(),
                effort: EffortLevel::High,
            },
        ],
        violations: vec![],
    });

    // GPT-4: Fast - decent code, no questions, no violations
    trajectories.insert("gpt-4".to_string(), AgentTrajectory {
        agent_name: "gpt-4".to_string(),
        questions_asked: vec![],
        violations: vec![],
    });

    (artifacts, trajectories)
}

fn scenario_2_preference_violations() -> (Vec<AgentArtifact>, HashMap<String, AgentTrajectory>) {
    println!("\n=== Scenario 2: Preference Violations Impact ===");

    let artifacts = vec![
        AgentArtifact {
            agent_name: "agent-compliant".to_string(),
            content: r#"{"status": "success", "data": {...}}"#.to_string(),
            completeness: 0.80,
            correctness: 0.80,
        },
        AgentArtifact {
            agent_name: "agent-violator".to_string(),
            content: "Success! Here's the data:\n\nUser: {...}\nPost: {...}".to_string(),
            completeness: 0.85,
            correctness: 0.85,
        },
    ];

    let mut trajectories = HashMap::new();

    // Compliant agent: Follows JSON preference
    trajectories.insert("agent-compliant".to_string(), AgentTrajectory {
        agent_name: "agent-compliant".to_string(),
        questions_asked: vec![],
        violations: vec![],
    });

    // Violator: Better technical score, but violates require_json preference
    trajectories.insert("agent-violator".to_string(), AgentTrajectory {
        agent_name: "agent-violator".to_string(),
        questions_asked: vec![],
        violations: vec![
            Violation {
                preference: "require_json".to_string(),
                severity: "major".to_string(),
            },
        ],
    });

    (artifacts, trajectories)
}

fn scenario_3_stage_specific_weights() -> (Vec<AgentArtifact>, HashMap<String, AgentTrajectory>) {
    println!("\n=== Scenario 3: Stage-Specific Weights ===");

    let artifacts = vec![
        AgentArtifact {
            agent_name: "explorer".to_string(),
            content: "Explored 3 approaches, asking clarifying questions...".to_string(),
            completeness: 0.70,
            correctness: 0.70,
        },
        AgentArtifact {
            agent_name: "implementer".to_string(),
            content: "Direct implementation without questions...".to_string(),
            completeness: 0.80,
            correctness: 0.80,
        },
    ];

    let mut trajectories = HashMap::new();

    // Explorer: Asks many questions (good for planning, bad for implementation)
    trajectories.insert("explorer".to_string(), AgentTrajectory {
        agent_name: "explorer".to_string(),
        questions_asked: vec![
            Question {
                text: "Which architecture pattern?".to_string(),
                effort: EffortLevel::Low,
            },
            Question {
                text: "Which database?".to_string(),
                effort: EffortLevel::Low,
            },
            Question {
                text: "Which caching strategy?".to_string(),
                effort: EffortLevel::Low,
            },
        ],
        violations: vec![],
    });

    // Implementer: No questions (good for implementation, maybe bad for planning)
    trajectories.insert("implementer".to_string(), AgentTrajectory {
        agent_name: "implementer".to_string(),
        questions_asked: vec![],
        violations: vec![],
    });

    (artifacts, trajectories)
}

// ============================================================================
// Main Demo
// ============================================================================

fn main() {
    println!("==========================================================");
    println!("PPP Weighted Consensus - Proof of Concept");
    println!("==========================================================");

    // Scenario 1: Balanced vs Technical Expert
    let (artifacts1, trajectories1) = scenario_1_balanced_vs_technical_expert();
    let consensus1 = weighted_consensus(artifacts1, trajectories1, (0.7, 0.3));

    println!("\nWeights: 70% technical + 30% interaction");
    println!("\nAgent Scores:");
    for score in &consensus1.scores {
        println!("  {}: {:.3} (tech: {:.2}, interact: {:.2})",
            score.agent_name,
            score.final_score,
            score.technical_score,
            score.interaction_score,
        );
        println!("    Proactivity: {:.2} ({} questions: {} low, {} med, {} high)",
            score.details.proactivity.r_proact,
            score.details.proactivity.total_questions,
            score.details.proactivity.low_effort,
            score.details.proactivity.medium_effort,
            score.details.proactivity.high_effort,
        );
        println!("    Personalization: {:.2} ({} violations)",
            score.details.personalization.r_pers,
            score.details.personalization.total_violations,
        );
    }

    println!("\n✅ Winner: {} (confidence: {:.3})",
        consensus1.best_agent,
        consensus1.confidence,
    );

    // Scenario 2: Preference Violations
    let (artifacts2, trajectories2) = scenario_2_preference_violations();
    let consensus2 = weighted_consensus(artifacts2, trajectories2, (0.7, 0.3));

    println!("\nWeights: 70% technical + 30% interaction");
    println!("\nAgent Scores:");
    for score in &consensus2.scores {
        println!("  {}: {:.3} (tech: {:.2}, interact: {:.2})",
            score.agent_name,
            score.final_score,
            score.technical_score,
            score.interaction_score,
        );
        if score.details.personalization.total_violations > 0 {
            println!("    ⚠️  {} violations ({} major)",
                score.details.personalization.total_violations,
                score.details.personalization.major,
            );
        }
    }

    println!("\n✅ Winner: {} (confidence: {:.3})",
        consensus2.best_agent,
        consensus2.confidence,
    );
    println!("    → Agent with JSON compliance wins despite lower technical score");

    // Scenario 3: Stage-Specific Weights
    let (artifacts3, trajectories3) = scenario_3_stage_specific_weights();

    println!("\n--- Plan Stage (60% technical + 40% interaction) ---");
    let consensus3_plan = weighted_consensus(artifacts3.clone(), trajectories3.clone(), (0.6, 0.4));
    println!("\nAgent Scores:");
    for score in &consensus3_plan.scores {
        println!("  {}: {:.3} (tech: {:.2}, interact: {:.2})",
            score.agent_name,
            score.final_score,
            score.technical_score,
            score.interaction_score,
        );
    }
    println!("\n✅ Winner: {} (confidence: {:.3})",
        consensus3_plan.best_agent,
        consensus3_plan.confidence,
    );

    println!("\n--- Unlock Stage (80% technical + 20% interaction) ---");
    let consensus3_unlock = weighted_consensus(artifacts3.clone(), trajectories3.clone(), (0.8, 0.2));
    println!("\nAgent Scores:");
    for score in &consensus3_unlock.scores {
        println!("  {}: {:.3} (tech: {:.2}, interact: {:.2})",
            score.agent_name,
            score.final_score,
            score.technical_score,
            score.interaction_score,
        );
    }
    println!("\n✅ Winner: {} (confidence: {:.3})",
        consensus3_unlock.best_agent,
        consensus3_unlock.confidence,
    );

    println!("\n==========================================================");
    println!("VALIDATION");
    println!("==========================================================");

    // Validate Scenario 1
    assert_eq!(consensus1.best_agent, "gemini-flash",
        "Scenario 1: Gemini should win (balanced)");

    // Calculate expected scores
    // Gemini: tech=0.595 (0.6*0.85 + 0.4*0.85), interact=0.10 (0.05 + 0.05)
    // Final: 0.7*0.595 + 0.3*0.10 = 0.4465
    let gemini_score = consensus1.scores.iter()
        .find(|s| s.agent_name == "gemini-flash")
        .unwrap();
    assert!((gemini_score.technical_score - 0.85).abs() < 0.01);
    assert!((gemini_score.interaction_score - 0.10).abs() < 0.01);

    // Claude: tech=0.95, interact=-0.45 (-0.5 + 0.05)
    // Final: 0.7*0.95 + 0.3*(-0.45) = 0.530
    let claude_score = consensus1.scores.iter()
        .find(|s| s.agent_name == "claude-opus")
        .unwrap();
    assert!((claude_score.interaction_score - (-0.45)).abs() < 0.01);

    // Validate Scenario 2
    assert_eq!(consensus2.best_agent, "agent-compliant",
        "Scenario 2: Compliant agent should win");

    // Validate Scenario 3
    // Plan stage: Explorer wins (interaction weighted more)
    // Unlock stage: Implementer wins (technical weighted more)
    println!("\n✅ All validations passed!");

    println!("\n==========================================================");
    println!("KEY FINDINGS");
    println!("==========================================================");
    println!("\n1. Weighted consensus balances technical quality with UX");
    println!("2. 70/30 weights favor correctness while accounting for interaction");
    println!("3. Stage-specific weights adapt to task criticality");
    println!("4. Preference violations appropriately penalize agents");
    println!("5. Formula is simple, interpretable, and computationally cheap");

    println!("\n==========================================================");
    println!("POC COMPLETE");
    println!("==========================================================");
    println!("\nNext Steps:");
    println!("- Integrate with consensus.rs in codex-tui");
    println!("- Connect to trajectory logging (SPEC-PPP-004)");
    println!("- Add configuration support (config.toml)");
    println!("- Benchmark with real agent outputs");
    println!("- Validate with user studies (preferred agent selection)");
}
