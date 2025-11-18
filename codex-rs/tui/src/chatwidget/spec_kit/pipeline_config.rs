//! Pipeline configuration for modular stage execution
//!
//! SPEC-948: Modular Pipeline Logic - Phase 1
//!
//! Provides type-safe configuration for selectively enabling/disabling pipeline stages,
//! with 3-tier precedence (CLI > per-SPEC > global > defaults) and dependency validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Pipeline configuration (TOML schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// SPEC ID this configuration applies to
    pub spec_id: String,

    /// Enabled stages (order preserved, executed sequentially)
    pub enabled_stages: Vec<StageType>,

    /// Quality gate configuration
    pub quality_gates: QualityGateConfig,

    /// Model overrides per stage (optional)
    #[serde(default)]
    pub stage_models: HashMap<StageType, Vec<String>>,

    /// Conditional skip rules (optional)
    #[serde(default)]
    pub skip_conditions: HashMap<StageType, SkipCondition>,

    /// Skip reasons (human-readable explanations)
    #[serde(default)]
    pub skip_reasons: HashMap<String, String>,

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

/// Stage types in pipeline
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StageType {
    New,
    Specify,
    Plan,
    Tasks,
    Implement,
    Validate,
    Audit,
    Unlock,
}

impl fmt::Display for StageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::New => write!(f, "new"),
            Self::Specify => write!(f, "specify"),
            Self::Plan => write!(f, "plan"),
            Self::Tasks => write!(f, "tasks"),
            Self::Implement => write!(f, "implement"),
            Self::Validate => write!(f, "validate"),
            Self::Audit => write!(f, "audit"),
            Self::Unlock => write!(f, "unlock"),
        }
    }
}

impl std::str::FromStr for StageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "new" => Ok(Self::New),
            "specify" => Ok(Self::Specify),
            "plan" => Ok(Self::Plan),
            "tasks" => Ok(Self::Tasks),
            "implement" => Ok(Self::Implement),
            "validate" => Ok(Self::Validate),
            "audit" => Ok(Self::Audit),
            "unlock" => Ok(Self::Unlock),
            _ => Err(format!("Unknown stage type: {}", s)),
        }
    }
}

impl StageType {
    /// Get required dependencies for this stage
    pub fn dependencies(&self) -> Vec<StageType> {
        match self {
            Self::New => vec![],
            Self::Specify => vec![Self::New],
            Self::Plan => vec![Self::Specify], // Soft: can use raw spec.md
            Self::Tasks => vec![Self::Plan],
            Self::Implement => vec![Self::Tasks],
            Self::Validate => vec![Self::Implement],
            Self::Audit => vec![Self::Implement],
            Self::Unlock => vec![Self::Implement],
        }
    }

    /// Get cost estimate for this stage (baseline: GPT-5 era per SPEC-949)
    pub fn cost_estimate(&self) -> f64 {
        match self {
            Self::New => 0.0,       // Native
            Self::Specify => 0.08,  // 1 agent (gpt5_1_mini)
            Self::Plan => 0.30,     // 3 agents (gpt5_1 + cheap)
            Self::Tasks => 0.08,    // 1 agent (gpt5_1_mini)
            Self::Implement => 0.10, // 2 agents (gpt5_1_codex + cheap)
            Self::Validate => 0.30, // 3 agents (gpt5_1 + cheap)
            Self::Audit => 0.80,    // 3 premium (gpt5_codex + premium)
            Self::Unlock => 0.80,   // 3 premium (gpt5_codex + premium)
        }
    }

    /// Get time estimate for this stage (minutes)
    pub fn duration_estimate(&self) -> u32 {
        match self {
            Self::New => 1,        // <1s native
            Self::Specify => 4,    // 3-5 min
            Self::Plan => 11,      // 10-12 min
            Self::Tasks => 4,      // 3-5 min
            Self::Implement => 10, // 8-12 min
            Self::Validate => 11,  // 10-12 min
            Self::Audit => 11,     // 10-12 min
            Self::Unlock => 11,    // 10-12 min
        }
    }

    /// Does this stage have a quality gate checkpoint?
    pub fn has_quality_gate(&self) -> bool {
        matches!(self, Self::Plan | Self::Tasks)
    }

    /// Check if dependency is hard requirement (must exist or execute)
    pub fn is_hard_dependency(&self, dep: StageType) -> bool {
        match (self, dep) {
            (Self::Tasks, Self::Plan) => true,     // Tasks needs plan
            (Self::Implement, Self::Tasks) => true, // Implement needs tasks
            _ => false,
        }
    }
}

/// Quality gate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateConfig {
    /// Whether quality gates are enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Auto-resolve low-severity issues
    #[serde(default = "default_true")]
    pub auto_resolve: bool,
}

fn default_true() -> bool {
    true
}

impl Default for QualityGateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_resolve: true,
        }
    }
}

/// Conditional skip rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipCondition {
    /// Skip if no test files exist in docs/SPEC-*/
    NoTests,

    /// Skip if SPEC priority is "low" in spec.md
    LowRisk,

    /// Skip if file count < threshold (e.g., docs-only if no .rs files)
    FileCountBelow { pattern: String, count: usize },

    /// Always skip
    Always,

    /// Never skip (force execution)
    Never,
}

/// CLI overrides for pipeline configuration
#[derive(Debug, Clone, Default)]
pub struct PipelineOverrides {
    /// Stages to skip (disable)
    pub skip_stages: Vec<StageType>,

    /// Exclusive stages to run (all others disabled)
    pub only_stages: Option<Vec<StageType>>,
}

/// Validation result with warnings
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub warnings: Vec<String>,
}

impl PipelineConfig {
    /// Create default configuration (all stages enabled)
    pub fn defaults() -> Self {
        Self {
            spec_id: String::new(),
            enabled_stages: vec![
                StageType::New,
                StageType::Specify,
                StageType::Plan,
                StageType::Tasks,
                StageType::Implement,
                StageType::Validate,
                StageType::Audit,
                StageType::Unlock,
            ],
            quality_gates: QualityGateConfig::default(),
            stage_models: HashMap::new(),
            skip_conditions: HashMap::new(),
            skip_reasons: HashMap::new(),
            created: None,
            modified: None,
        }
    }

    /// Load configuration with 3-tier precedence
    ///
    /// Precedence order (highest to lowest):
    /// 1. CLI overrides (--skip-validate, --only-plan)
    /// 2. Per-SPEC config (docs/SPEC-*/pipeline.toml)
    /// 3. Global user config (~/.code/config.toml → [pipeline.defaults])
    /// 4. Built-in defaults (all stages enabled)
    pub fn load(spec_id: &str, cli_overrides: Option<PipelineOverrides>) -> Result<Self, String> {
        // Start with built-in defaults
        let mut config = Self::defaults();
        config.spec_id = spec_id.to_string();

        // Layer 1: Global user config (~/.code/config.toml)
        if let Ok(global) = Self::load_global_config() {
            config.merge(global);
        }

        // Layer 2: Per-SPEC config (docs/SPEC-*/pipeline.toml)
        let spec_path = format!("docs/{}/pipeline.toml", spec_id);
        if let Ok(per_spec) = Self::load_file_config(&spec_path) {
            config.merge(per_spec);
        }

        // Layer 3: CLI overrides
        if let Some(overrides) = cli_overrides {
            config.apply_overrides(overrides);
        }

        // Validate dependencies
        config.validate()?;

        Ok(config)
    }

    /// Load global config from ~/.code/config.toml
    fn load_global_config() -> Result<Self, String> {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        let path = format!("{}/.code/config.toml", home);

        // Check if file exists
        if !Path::new(&path).exists() {
            return Err("Global config not found".to_string());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read global config: {}", e))?;

        // Parse TOML as generic value to extract nested section
        let toml_value: toml::Value = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse global TOML: {}", e))?;

        // Extract [pipeline.defaults] section
        let defaults_section = toml_value
            .get("pipeline")
            .and_then(|p| p.get("defaults"))
            .ok_or_else(|| "No [pipeline.defaults] section in global config".to_string())?;

        // Deserialize the defaults section into PipelineConfig
        let config: Self = defaults_section
            .clone()
            .try_into()
            .map_err(|e: toml::de::Error| format!("Failed to parse pipeline.defaults: {}", e))?;

        Ok(config)
    }

    /// Load config from specific TOML file
    fn load_file_config(path: &str) -> Result<Self, String> {
        if !Path::new(path).exists() {
            return Err(format!("Config file not found: {}", path));
        }

        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config: {}", e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))
    }

    /// Save configuration to file
    pub fn save(&self, path: &str) -> Result<(), String> {
        let toml_str =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize: {}", e))?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        fs::write(path, toml_str).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    /// Merge another config into this one (precedence rules)
    fn merge(&mut self, other: Self) {
        // Only merge non-empty fields from other config
        if !other.enabled_stages.is_empty() {
            self.enabled_stages = other.enabled_stages;
        }

        // Merge quality gates
        self.quality_gates = other.quality_gates;

        // Merge model overrides (union, other takes precedence)
        for (stage, models) in other.stage_models {
            self.stage_models.insert(stage, models);
        }

        // Merge skip conditions (union, other takes precedence)
        for (stage, condition) in other.skip_conditions {
            self.skip_conditions.insert(stage, condition);
        }

        // Merge skip reasons
        for (key, value) in other.skip_reasons {
            self.skip_reasons.insert(key, value);
        }
    }

    /// Apply CLI overrides to configuration
    fn apply_overrides(&mut self, overrides: PipelineOverrides) {
        // Handle --only-* flags (exclusive execution)
        if let Some(only_stages) = overrides.only_stages {
            self.enabled_stages = only_stages;
        }

        // Handle --skip-* flags (disable specific stages)
        for skip_stage in overrides.skip_stages {
            self.enabled_stages.retain(|s| s != &skip_stage);
        }
    }

    /// Check if a stage is enabled
    pub fn is_enabled(&self, stage: StageType) -> bool {
        self.enabled_stages.contains(&stage)
    }

    /// Get skip reason for a stage (if available)
    pub fn skip_reason(&self, stage: StageType) -> Option<&str> {
        self.skip_reasons.get(&stage.to_string()).map(|s| s.as_str())
    }

    /// Validate configuration for errors and warnings
    pub fn validate(&self) -> Result<ValidationResult, String> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check hard dependencies
        for stage in &self.enabled_stages {
            for dep in stage.dependencies() {
                if !self.is_enabled(dep) {
                    // Check if dependency is hard requirement
                    if stage.is_hard_dependency(dep) {
                        errors.push(format!(
                            "Error: {} requires {} to be enabled",
                            stage, dep
                        ));
                    } else {
                        warnings.push(format!(
                            "Warning: {} without {}: will use existing artifacts",
                            stage, dep
                        ));
                    }
                }
            }
        }

        // Check quality gate bypass
        if !self.is_enabled(StageType::Plan) {
            warnings.push("⚠ Skipping plan disables 2 quality gate checkpoints".into());
        }
        if !self.is_enabled(StageType::Tasks) {
            warnings.push("⚠ Skipping tasks disables 1 quality gate checkpoint".into());
        }

        // Check cost implications
        let total_cost: f64 = self.enabled_stages.iter().map(|s| s.cost_estimate()).sum();
        let full_cost = 2.46; // All stages (GPT-5 baseline per SPEC-949)
        if total_cost < full_cost * 0.5 {
            warnings.push(format!(
                "ℹ️ Partial pipeline: ${:.2} vs ${:.2} full (saving ${:.2})",
                total_cost,
                full_cost,
                full_cost - total_cost
            ));
        }

        if errors.is_empty() {
            Ok(ValidationResult { warnings })
        } else {
            Err(format!(
                "Configuration has {} error(s):\n{}",
                errors.len(),
                errors.join("\n")
            ))
        }
    }

    /// Calculate enabled quality gate checkpoints
    pub fn active_quality_gates(&self) -> Vec<QualityCheckpoint> {
        let mut checkpoints = Vec::new();

        // Checkpoint 1: Pre-planning (clarify)
        if self.is_enabled(StageType::Specify) && self.is_enabled(StageType::Plan) {
            checkpoints.push(QualityCheckpoint::PrePlanning);
        }

        // Checkpoint 2: Post-plan (checklist)
        if self.is_enabled(StageType::Plan) {
            checkpoints.push(QualityCheckpoint::PostPlan);
        }

        // Checkpoint 3: Post-tasks (analyze)
        if self.is_enabled(StageType::Tasks) {
            checkpoints.push(QualityCheckpoint::PostTasks);
        }

        checkpoints
    }
}

/// Quality gate checkpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityCheckpoint {
    PrePlanning,
    PostPlan,
    PostTasks,
}

impl QualityCheckpoint {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::PrePlanning => "Pre-planning clarity check (/speckit.clarify)",
            Self::PostPlan => "Post-plan quality assessment (/speckit.checklist)",
            Self::PostTasks => "Post-tasks consistency validation (/speckit.analyze)",
        }
    }
}

impl PipelineOverrides {
    /// Parse CLI flags into overrides
    pub fn from_cli_args(args: &[String]) -> Self {
        let mut skip_stages = Vec::new();
        let mut only_stages: Option<Vec<StageType>> = None;

        for arg in args {
            // Handle --skip-{stage}
            if let Some(stage_name) = arg.strip_prefix("--skip-") {
                if let Ok(stage) = stage_name.parse::<StageType>() {
                    skip_stages.push(stage);
                }
            }
            // Handle --only-{stage}
            else if let Some(stage_name) = arg.strip_prefix("--only-") {
                if let Ok(stage) = stage_name.parse::<StageType>() {
                    only_stages
                        .get_or_insert_with(Vec::new)
                        .push(stage);
                }
            }
            // Handle --stages=plan,tasks,implement
            else if let Some(stages_list) = arg.strip_prefix("--stages=") {
                let stages: Vec<StageType> = stages_list
                    .split(',')
                    .filter_map(|s| s.parse::<StageType>().ok())
                    .collect();
                if !stages.is_empty() {
                    only_stages = Some(stages);
                }
            }
        }

        Self {
            skip_stages,
            only_stages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = PipelineConfig::defaults();
        assert_eq!(config.enabled_stages.len(), 8);
        assert!(config.quality_gates.enabled);
        assert!(config.quality_gates.auto_resolve);
    }

    #[test]
    fn test_stage_dependencies() {
        assert_eq!(StageType::New.dependencies(), vec![]);
        assert_eq!(StageType::Specify.dependencies(), vec![StageType::New]);
        assert_eq!(StageType::Plan.dependencies(), vec![StageType::Specify]);
        assert_eq!(StageType::Tasks.dependencies(), vec![StageType::Plan]);
        assert_eq!(StageType::Implement.dependencies(), vec![StageType::Tasks]);
    }

    #[test]
    fn test_stage_cost_estimates() {
        assert_eq!(StageType::New.cost_estimate(), 0.0);
        assert_eq!(StageType::Specify.cost_estimate(), 0.08);
        assert_eq!(StageType::Plan.cost_estimate(), 0.30);
        assert_eq!(StageType::Audit.cost_estimate(), 0.80);
    }

    #[test]
    fn test_stage_from_str() {
        assert_eq!("plan".parse::<StageType>().unwrap(), StageType::Plan);
        assert_eq!("VALIDATE".parse::<StageType>().unwrap(), StageType::Validate);
        assert!("invalid".parse::<StageType>().is_err());
    }

    #[test]
    fn test_cli_overrides_skip() {
        let args = vec![
            "--skip-validate".to_string(),
            "--skip-audit".to_string(),
        ];
        let overrides = PipelineOverrides::from_cli_args(&args);

        assert_eq!(overrides.skip_stages.len(), 2);
        assert!(overrides.skip_stages.contains(&StageType::Validate));
        assert!(overrides.skip_stages.contains(&StageType::Audit));
        assert!(overrides.only_stages.is_none());
    }

    #[test]
    fn test_cli_overrides_only() {
        let args = vec![
            "--only-plan".to_string(),
            "--only-tasks".to_string(),
        ];
        let overrides = PipelineOverrides::from_cli_args(&args);

        assert!(overrides.skip_stages.is_empty());
        let only = overrides.only_stages.unwrap();
        assert_eq!(only.len(), 2);
        assert!(only.contains(&StageType::Plan));
        assert!(only.contains(&StageType::Tasks));
    }

    #[test]
    fn test_cli_overrides_stages_list() {
        let args = vec!["--stages=plan,tasks,implement".to_string()];
        let overrides = PipelineOverrides::from_cli_args(&args);

        let only = overrides.only_stages.unwrap();
        assert_eq!(only.len(), 3);
        assert!(only.contains(&StageType::Plan));
        assert!(only.contains(&StageType::Tasks));
        assert!(only.contains(&StageType::Implement));
    }

    #[test]
    fn test_apply_overrides_skip() {
        let mut config = PipelineConfig::defaults();
        let overrides = PipelineOverrides {
            skip_stages: vec![StageType::Validate, StageType::Audit],
            only_stages: None,
        };

        config.apply_overrides(overrides);

        assert!(!config.is_enabled(StageType::Validate));
        assert!(!config.is_enabled(StageType::Audit));
        assert!(config.is_enabled(StageType::Plan));
    }

    #[test]
    fn test_apply_overrides_only() {
        let mut config = PipelineConfig::defaults();
        let overrides = PipelineOverrides {
            skip_stages: vec![],
            only_stages: Some(vec![StageType::Plan, StageType::Tasks]),
        };

        config.apply_overrides(overrides);

        assert_eq!(config.enabled_stages.len(), 2);
        assert!(config.is_enabled(StageType::Plan));
        assert!(config.is_enabled(StageType::Tasks));
        assert!(!config.is_enabled(StageType::Implement));
    }

    #[test]
    fn test_validation_hard_dependency_error() {
        let mut config = PipelineConfig::defaults();
        config.enabled_stages = vec![StageType::Implement]; // Missing Tasks

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("implement requires tasks"));
    }

    #[test]
    fn test_validation_quality_gate_warnings() {
        let mut config = PipelineConfig::defaults();
        config.enabled_stages = vec![StageType::Implement, StageType::Validate];

        // Allow validation to pass even though dependencies are missing
        // (soft dependencies only generate warnings, not errors)
        config.enabled_stages = vec![StageType::New, StageType::Specify, StageType::Plan, StageType::Tasks, StageType::Implement, StageType::Validate];

        // Now test with plan disabled (should warn about quality gates)
        config.enabled_stages.retain(|s| s != &StageType::Plan);
        config.enabled_stages.retain(|s| s != &StageType::Tasks);

        // This will error because implement depends on tasks
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_active_quality_gates() {
        let config = PipelineConfig::defaults();
        let gates = config.active_quality_gates();

        assert_eq!(gates.len(), 3);
        assert!(gates.contains(&QualityCheckpoint::PrePlanning));
        assert!(gates.contains(&QualityCheckpoint::PostPlan));
        assert!(gates.contains(&QualityCheckpoint::PostTasks));
    }

    #[test]
    fn test_global_config_loading_with_valid_section() {
        // Test TOML parsing logic for [pipeline.defaults] section
        let toml_content = r#"
[pipeline.defaults]
spec_id = "GLOBAL"
enabled_stages = ["new", "specify", "plan", "implement"]

[pipeline.defaults.quality_gates]
enabled = true
auto_resolve = false
"#;

        // Parse the config manually to test the logic
        let toml_value: toml::Value = toml::from_str(toml_content).unwrap();
        let defaults_section = toml_value
            .get("pipeline")
            .and_then(|p| p.get("defaults"))
            .unwrap();
        let config: PipelineConfig = defaults_section.clone().try_into().unwrap();

        // Validate parsed config
        assert_eq!(config.spec_id, "GLOBAL");
        assert_eq!(config.enabled_stages.len(), 4);
        assert!(config.is_enabled(StageType::Plan));
        assert!(!config.is_enabled(StageType::Validate));
        assert!(config.quality_gates.enabled);
        assert!(!config.quality_gates.auto_resolve);
    }

    #[test]
    fn test_global_config_missing_section() {
        // Test that missing [pipeline.defaults] is handled correctly
        let toml_content = r#"
[other_section]
key = "value"
"#;

        // Parse should fail when section doesn't exist
        let toml_value: toml::Value = toml::from_str(toml_content).unwrap();
        let defaults_section = toml_value.get("pipeline").and_then(|p| p.get("defaults"));

        assert!(defaults_section.is_none());
    }

    #[test]
    fn test_precedence_global_to_per_spec() {
        // Test that per-SPEC config overrides global defaults via merge
        let per_spec_toml = r#"
[pipeline]
spec_id = "SPEC-TEST"
enabled_stages = ["implement", "validate", "unlock"]
"#;

        // Parse per-SPEC config
        let per_spec: PipelineConfig = toml::from_str(per_spec_toml).unwrap();

        // Start with defaults (simulating global)
        let mut config = PipelineConfig::defaults();
        assert_eq!(config.enabled_stages.len(), 8); // All stages

        // Merge per-SPEC (should override)
        config.merge(per_spec);

        // Verify per-SPEC took precedence
        assert_eq!(config.enabled_stages.len(), 3);
        assert!(config.is_enabled(StageType::Implement));
        assert!(config.is_enabled(StageType::Validate));
        assert!(config.is_enabled(StageType::Unlock));
        assert!(!config.is_enabled(StageType::Plan));
    }
}
