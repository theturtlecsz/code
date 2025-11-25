// FORK-SPECIFIC (just-every/code): Cost tracking for SPEC-KIT-070
//!
//! Tracks model usage costs, enforces budgets, and provides cost telemetry
//! for multi-agent automation workflows.

#![allow(dead_code)] // Extended cost tracking features pending

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::spec_prompts::SpecStage;

/// Model pricing rates (USD per 1M tokens)
/// Updated: 2025-11-19 from official pricing pages
/// Sources: ai.google.dev/pricing, claude.com/pricing, platform.openai.com/docs/pricing
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
}

impl ModelPricing {
    /// Get pricing for a model by name
    pub fn for_model(model: &str) -> Self {
        match model {
            // Claude models (Updated 2025-11-19)
            // Source: claude.com/pricing
            "claude-haiku" | "claude-haiku-4.5" | "claude-haiku-3.5" | "haiku" => Self {
                input_per_million: 1.0,  // Was 0.25 (4x increase!)
                output_per_million: 5.0, // Was 1.25 (4x increase!)
            },
            "claude-sonnet" | "claude-sonnet-4.5" | "claude-sonnet-4" | "sonnet" => Self {
                input_per_million: 3.0,
                output_per_million: 15.0,
            },
            "claude-opus" | "claude-opus-4.5" | "claude-opus-4.1" | "claude-opus-4" | "opus" => Self {
                input_per_million: 15.0,
                output_per_million: 75.0,
            },

            // Gemini models (Updated 2025-11-19)
            // Source: ai.google.dev/pricing
            // Gemini 3 family (Released 2025-11-18)
            "gemini-3-pro" | "gemini-3.0-pro" => Self {
                input_per_million: 2.0,   // Standard pricing (≤200k tokens)
                output_per_million: 12.0, // Top LMArena model (1501 Elo)
            },
            // Note: Gemini 3 Deep Think not yet publicly priced, rolling out to AI Ultra

            // Gemini 2.5 family
            "gemini-2.5-flash" | "gemini-flash-2.5" | "flash-2.5" => Self {
                input_per_million: 0.30,  // Was 0.10 (3x increase!)
                output_per_million: 2.50, // Was 0.40 (6.25x increase!)
            },
            "gemini-2.0-flash" | "flash-2.0" => Self {
                input_per_million: 0.30,
                output_per_million: 2.50,
            },
            "gemini-1.5-flash" | "flash-1.5" | "flash" => Self {
                input_per_million: 0.075,
                output_per_million: 0.30,
            },
            "gemini-2.5-flash-lite" | "flash-lite" => Self {
                input_per_million: 0.05,
                output_per_million: 0.20,
            },
            "gemini-2.5-pro" | "gemini-pro-2.5" => Self {
                input_per_million: 1.25,
                output_per_million: 10.0, // Was 5.0 (2x increase!)
            },
            "gemini-1.5-pro" | "gemini-pro-1.5" | "gemini-pro" => Self {
                input_per_million: 1.25,
                output_per_million: 10.0, // Updated to match 2.5-pro
            },

            // OpenAI models (Updated 2025-11-19)
            // Source: platform.openai.com/docs/pricing

            // GPT-5 family (Released Aug 2025, GPT-5.1 Nov 2025)
            "gpt-5" | "gpt-5.1" | "gpt5_1" | "gpt5_1_minimal" | "gpt-5.1-minimal"
            | "gpt-5.1-instant" => Self {
                input_per_million: 1.25,  // Was 10.0 (estimate)
                output_per_million: 10.0, // Was 30.0 (estimate)
            },
            "gpt-5-mini" | "gpt5_1_mini" => Self {
                input_per_million: 0.25,
                output_per_million: 2.0,
            },
            "gpt-5-codex" | "gpt5_1_codex" | "gpt-5.1-codex" => Self {
                input_per_million: 1.25, // Same as GPT-5.1 (codex variant)
                output_per_million: 10.0,
            },

            // GPT-4 family (legacy/deprecated - kept for compatibility)
            "gpt-4o" => Self {
                input_per_million: 2.50,
                output_per_million: 10.0,
            },
            "gpt-4o-mini" => Self {
                input_per_million: 0.15,
                output_per_million: 0.60,
            },
            "gpt-4-turbo" | "gpt-4" => Self {
                input_per_million: 10.0,
                output_per_million: 30.0,
            },
            "gpt-3.5-turbo" => Self {
                input_per_million: 0.50,
                output_per_million: 1.50,
            },

            // Unknown model - use expensive default for safety
            _ => Self {
                input_per_million: 10.0,
                output_per_million: 30.0,
            },
        }
    }

    /// Calculate cost for given token counts
    pub fn calculate(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_per_million;
        input_cost + output_cost
    }
}

/// Cost tracker for a single SPEC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecCostTracker {
    pub spec_id: String,
    pub budget: f64,
    pub spent: f64,
    pub per_stage: HashMap<String, f64>,
    pub per_model: HashMap<String, f64>,
    pub call_count: u32,
    pub started_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl SpecCostTracker {
    pub fn new(spec_id: String, budget: f64) -> Self {
        let now = Utc::now();
        Self {
            spec_id,
            budget,
            spent: 0.0,
            per_stage: HashMap::new(),
            per_model: HashMap::new(),
            call_count: 0,
            started_at: now,
            last_updated: now,
        }
    }

    /// Record a model call and return warning if approaching budget
    pub fn record_call(&mut self, stage: SpecStage, model: &str, cost: f64) -> Option<BudgetAlert> {
        self.spent += cost;
        *self
            .per_stage
            .entry(stage.command_name().to_string())
            .or_insert(0.0) += cost;
        *self.per_model.entry(model.to_string()).or_insert(0.0) += cost;
        self.call_count += 1;
        self.last_updated = Utc::now();

        // Check budget thresholds
        let utilization = self.spent / self.budget;

        if utilization >= 1.0 {
            Some(BudgetAlert::Exceeded {
                spec_id: self.spec_id.clone(),
                budget: self.budget,
                spent: self.spent,
                overage: self.spent - self.budget,
            })
        } else if utilization >= 0.9 {
            Some(BudgetAlert::Critical {
                spec_id: self.spec_id.clone(),
                budget: self.budget,
                spent: self.spent,
                remaining: self.budget - self.spent,
            })
        } else if utilization >= 0.8 {
            Some(BudgetAlert::Warning {
                spec_id: self.spec_id.clone(),
                budget: self.budget,
                spent: self.spent,
                remaining: self.budget - self.spent,
            })
        } else {
            None
        }
    }

    /// Get cost breakdown summary
    pub fn summary(&self) -> CostSummary {
        CostSummary {
            spec_id: self.spec_id.clone(),
            total_spent: self.spent,
            budget: self.budget,
            utilization: self.spent / self.budget,
            call_count: self.call_count,
            per_stage: self.per_stage.clone(),
            per_model: self.per_model.clone(),
            duration: Utc::now().signed_duration_since(self.started_at),
        }
    }
}

/// Budget alert levels
#[derive(Debug, Clone)]
pub enum BudgetAlert {
    Warning {
        spec_id: String,
        budget: f64,
        spent: f64,
        remaining: f64,
    },
    Critical {
        spec_id: String,
        budget: f64,
        spent: f64,
        remaining: f64,
    },
    Exceeded {
        spec_id: String,
        budget: f64,
        spent: f64,
        overage: f64,
    },
}

impl BudgetAlert {
    pub fn to_user_message(&self) -> String {
        match self {
            BudgetAlert::Warning {
                spec_id,
                budget,
                spent,
                remaining,
            } => {
                format!(
                    "⚠️  {} budget warning: ${:.2} / ${:.2} spent (80%), ${:.2} remaining",
                    spec_id, spent, budget, remaining
                )
            }
            BudgetAlert::Critical {
                spec_id,
                budget,
                spent,
                remaining,
            } => {
                format!(
                    "🚨 {} budget critical: ${:.2} / ${:.2} spent (90%), only ${:.2} remaining",
                    spec_id, spent, budget, remaining
                )
            }
            BudgetAlert::Exceeded {
                spec_id,
                budget,
                spent,
                overage,
            } => {
                format!(
                    "❌ {} budget EXCEEDED: ${:.2} / ${:.2} spent, ${:.2} over budget",
                    spec_id, spent, budget, overage
                )
            }
        }
    }
}

/// Cost summary for reporting
#[derive(Debug, Clone, Serialize)]
pub struct CostSummary {
    pub spec_id: String,
    pub total_spent: f64,
    pub budget: f64,
    pub utilization: f64,
    pub call_count: u32,
    pub per_stage: HashMap<String, f64>,
    pub per_model: HashMap<String, f64>,
    #[serde(serialize_with = "serialize_duration")]
    pub duration: chrono::Duration,
}

fn serialize_duration<S>(duration: &chrono::Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i64(duration.num_seconds())
}

/// Global cost tracker managing all SPECs
#[derive(Clone)]
pub struct CostTracker {
    specs: Arc<Mutex<HashMap<String, SpecCostTracker>>>,
    default_budget: f64,
    // SPEC-KIT-070: Optional stage routing metadata per SPEC
    stage_notes: Arc<Mutex<HashMap<String, Vec<StageRoutingNote>>>>,
}

impl CostTracker {
    pub fn new(default_budget: f64) -> Self {
        Self {
            specs: Arc::new(Mutex::new(HashMap::new())),
            default_budget,
            stage_notes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record an agent call and return budget alert if needed
    pub fn record_agent_call(
        &self,
        spec_id: &str,
        stage: SpecStage,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> (f64, Option<BudgetAlert>) {
        let pricing = ModelPricing::for_model(model);
        let cost = pricing.calculate(input_tokens, output_tokens);

        let mut specs = self.specs.lock().unwrap();
        let tracker = specs
            .entry(spec_id.to_string())
            .or_insert_with(|| SpecCostTracker::new(spec_id.to_string(), self.default_budget));

        let alert = tracker.record_call(stage, model, cost);

        (cost, alert)
    }

    /// Get summary for a specific SPEC
    pub fn get_summary(&self, spec_id: &str) -> Option<CostSummary> {
        let specs = self.specs.lock().unwrap();
        specs.get(spec_id).map(|t| t.summary())
    }

    /// Get all SPEC summaries
    pub fn get_all_summaries(&self) -> Vec<CostSummary> {
        let specs = self.specs.lock().unwrap();
        specs.values().map(|t| t.summary()).collect()
    }

    /// Write cost summary to evidence directory
    pub fn write_summary(&self, spec_id: &str, evidence_dir: &Path) -> std::io::Result<()> {
        let summary = match self.get_summary(spec_id) {
            Some(s) => s,
            None => return Ok(()), // No costs tracked yet
        };
        // Merge stage notes when present
        let notes = {
            let guard = self.stage_notes.lock().unwrap();
            guard.get(spec_id).cloned().unwrap_or_default()
        };

        #[derive(Serialize)]
        struct ExtendedSummary<'a> {
            #[serde(flatten)]
            base: &'a CostSummary,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            stage_notes: Vec<StageRoutingNote>,
        }

        let ext = ExtendedSummary {
            base: &summary,
            stage_notes: notes,
        };
        let json = serde_json::to_string_pretty(&ext)?;
        let path = evidence_dir.join(format!("{}_cost_summary.json", spec_id));

        std::fs::create_dir_all(evidence_dir)?;
        std::fs::write(path, json)?;

        Ok(())
    }

    /// SPEC-KIT-070: Record aggregator effort/escalation reason for a stage
    pub fn set_stage_routing_note(
        &self,
        spec_id: &str,
        stage: SpecStage,
        aggregator_effort: Option<&str>,
        escalation_reason: Option<&str>,
    ) {
        let mut guard = self.stage_notes.lock().unwrap();
        let entry = guard.entry(spec_id.to_string()).or_insert_with(Vec::new);
        let note = StageRoutingNote {
            stage: stage.command_name().to_string(),
            aggregator_effort: aggregator_effort.map(|s| s.to_string()),
            escalation_reason: escalation_reason.map(|s| s.to_string()),
        };
        // Replace existing note for this stage if any
        if let Some(existing) = entry.iter_mut().find(|n| n.stage == note.stage) {
            *existing = note;
        } else {
            entry.push(note);
        }
    }
}

/// SPEC-KIT-070: Routing annotation per stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRoutingNote {
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregator_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_reason: Option<String>,
}

/// Task complexity classification for model selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskComplexity {
    /// Simple/Deterministic - Use native Rust or single cheap model
    /// Examples: SPEC-ID generation, status checks, file operations
    Simple,

    /// Medium/Judgment - Dual cheap models for validation
    /// Examples: Task decomposition, requirement clarification, quality checks
    Medium,

    /// Complex/Reasoning - Mixed tier (cheap + premium)
    /// Examples: Planning, architecture design, consensus aggregation
    Complex,

    /// Critical/Cannot-Fail - Premium models only
    /// Examples: Security audit, unlock decision, production deployment
    Critical,
}

impl TaskComplexity {
    /// Get recommended model tier for this complexity
    pub fn recommended_tier(&self) -> &'static str {
        match self {
            TaskComplexity::Simple => "native or single-cheap",
            TaskComplexity::Medium => "dual-cheap",
            TaskComplexity::Complex => "mixed-tier",
            TaskComplexity::Critical => "premium-only",
        }
    }

    /// Get budget multiplier for this complexity
    pub fn budget_multiplier(&self) -> f64 {
        match self {
            TaskComplexity::Simple => 0.1,   // $0.02-0.05
            TaskComplexity::Medium => 0.2,   // $0.20-0.40
            TaskComplexity::Complex => 0.5,  // $0.60-1.00
            TaskComplexity::Critical => 1.5, // $2.00-3.00
        }
    }
}

/// Classify command by complexity
pub fn classify_command(command_name: &str) -> TaskComplexity {
    match command_name {
        // Tier S - Simple/Deterministic (native or single cheap model)
        "speckit.status" | "spec-status" => TaskComplexity::Simple,

        // Tier M - Medium/Judgment (dual cheap models)
        "speckit.clarify" | "clarify" => TaskComplexity::Medium,
        "speckit.checklist" | "checklist" => TaskComplexity::Medium,
        "speckit.tasks" | "tasks" => TaskComplexity::Medium,

        // Tier C - Complex/Reasoning (mixed tier: cheap + premium)
        "speckit.new" | "new-spec" => TaskComplexity::Complex,
        "speckit.specify" | "specify" => TaskComplexity::Complex,
        "speckit.plan" | "plan" => TaskComplexity::Complex,
        "speckit.analyze" | "analyze" => TaskComplexity::Complex,
        "speckit.validate" | "validate" => TaskComplexity::Complex,
        "speckit.implement" | "implement" => TaskComplexity::Complex,

        // Tier X - Critical/Cannot-Fail (premium only)
        "speckit.audit" | "audit" => TaskComplexity::Critical,
        "speckit.unlock" | "unlock" => TaskComplexity::Critical,

        // Default to Complex for safety
        _ => TaskComplexity::Complex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_pricing_claude() {
        // Haiku pricing updated 2025-11-19 (4x increase)
        let haiku = ModelPricing::for_model("haiku");
        assert_eq!(haiku.input_per_million, 1.0);
        assert_eq!(haiku.output_per_million, 5.0);

        let sonnet = ModelPricing::for_model("sonnet");
        assert_eq!(sonnet.input_per_million, 3.0);
        assert_eq!(sonnet.output_per_million, 15.0);
    }

    #[test]
    fn test_model_pricing_gemini() {
        // Gemini 2.5 flash pricing updated 2025-11-19 (3x increase)
        let flash_25 = ModelPricing::for_model("gemini-2.5-flash");
        assert_eq!(flash_25.input_per_million, 0.30);

        let flash_15 = ModelPricing::for_model("gemini-1.5-flash");
        assert_eq!(flash_15.input_per_million, 0.075);

        let pro = ModelPricing::for_model("gemini-pro");
        assert_eq!(pro.input_per_million, 1.25);
    }

    #[test]
    fn test_cost_calculation() {
        let haiku = ModelPricing::for_model("haiku");
        // 10k input, 2k output
        let cost = haiku.calculate(10_000, 2_000);
        // (10k / 1M) * 1.0 + (2k / 1M) * 5.0
        // = 0.01 + 0.01 = 0.02 (4x increase from old pricing)
        assert!((cost - 0.02).abs() < 0.0001);
    }

    #[test]
    fn test_spec_cost_tracker() {
        let mut tracker = SpecCostTracker::new("SPEC-TEST-001".to_string(), 2.0);

        // First call - no alert
        let alert1 = tracker.record_call(SpecStage::Plan, "haiku", 0.30);
        assert!(alert1.is_none());
        assert_eq!(tracker.spent, 0.30);

        // Second call - still under 80%
        let alert2 = tracker.record_call(SpecStage::Tasks, "flash", 0.40);
        assert!(alert2.is_none());
        assert_eq!(tracker.spent, 0.70);

        // Third call - hits 80% threshold
        let alert3 = tracker.record_call(SpecStage::Implement, "sonnet", 0.90);
        assert!(matches!(alert3, Some(BudgetAlert::Warning { .. })));
        assert_eq!(tracker.spent, 1.60);

        // Fourth call - hits 90% threshold
        let alert4 = tracker.record_call(SpecStage::Validate, "haiku", 0.20);
        assert!(matches!(alert4, Some(BudgetAlert::Critical { .. })));
        assert_eq!(tracker.spent, 1.80);

        // Fifth call - exceeds budget
        let alert5 = tracker.record_call(SpecStage::Audit, "sonnet", 0.30);
        assert!(matches!(alert5, Some(BudgetAlert::Exceeded { .. })));
        assert_eq!(tracker.spent, 2.10);
    }

    #[test]
    fn test_command_classification() {
        assert_eq!(classify_command("speckit.status"), TaskComplexity::Simple);
        assert_eq!(classify_command("speckit.clarify"), TaskComplexity::Medium);
        assert_eq!(classify_command("speckit.plan"), TaskComplexity::Complex);
        assert_eq!(classify_command("speckit.audit"), TaskComplexity::Critical);
        assert_eq!(classify_command("unknown"), TaskComplexity::Complex); // Safe default
    }

    #[test]
    fn test_complexity_budget_multipliers() {
        assert_eq!(TaskComplexity::Simple.budget_multiplier(), 0.1);
        assert_eq!(TaskComplexity::Medium.budget_multiplier(), 0.2);
        assert_eq!(TaskComplexity::Complex.budget_multiplier(), 0.5);
        assert_eq!(TaskComplexity::Critical.budget_multiplier(), 1.5);
    }

    #[test]
    fn test_cost_tracker_per_stage() {
        let mut tracker = SpecCostTracker::new("SPEC-TEST-002".to_string(), 10.0);

        tracker.record_call(SpecStage::Plan, "haiku", 0.50);
        tracker.record_call(SpecStage::Plan, "flash", 0.30);
        tracker.record_call(SpecStage::Tasks, "haiku", 0.40);

        assert_eq!(tracker.per_stage.get("spec-plan").copied().unwrap(), 0.80);
        assert_eq!(tracker.per_stage.get("spec-tasks").copied().unwrap(), 0.40);
    }

    #[test]
    fn test_cost_tracker_per_model() {
        let mut tracker = SpecCostTracker::new("SPEC-TEST-003".to_string(), 10.0);

        tracker.record_call(SpecStage::Plan, "haiku", 0.50);
        tracker.record_call(SpecStage::Tasks, "haiku", 0.40);
        tracker.record_call(SpecStage::Plan, "flash", 0.30);

        assert_eq!(tracker.per_model.get("haiku").copied().unwrap(), 0.90);
        assert_eq!(tracker.per_model.get("flash").copied().unwrap(), 0.30);
    }
}
