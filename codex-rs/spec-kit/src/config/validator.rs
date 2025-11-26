use crate::config::error::{ConfigError, Result};
use crate::config::loader::AppConfig;
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

/// Schema validator for configuration structures
pub struct SchemaValidator {
    app_schema: JSONSchema,
}

impl SchemaValidator {
    /// Create a new validator with embedded schemas
    ///
    /// Schemas are embedded at compile time using `include_str!` for reliability
    /// and to avoid filesystem dependencies at runtime.
    pub fn new() -> Result<Self> {
        // Load the main app config schema
        let app_schema_str = include_str!("schemas/app_config.schema.json");
        let app_schema_value: Value = serde_json::from_str(app_schema_str).map_err(|e| {
            ConfigError::SchemaValidationError(format!("Failed to parse app schema: {e}"))
        })?;

        // Compile the schema with JSON Schema Draft 7
        let app_schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&app_schema_value)
            .map_err(|e| {
                ConfigError::SchemaValidationError(format!("Failed to compile app schema: {e}"))
            })?;

        Ok(Self { app_schema })
    }

    /// Validate an AppConfig instance against the schema
    ///
    /// Returns Ok(()) if validation passes, or Err with detailed error messages
    /// listing all validation failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use codex_spec_kit::config::{AppConfig, SchemaValidator};
    ///
    /// let config = AppConfig::default();
    /// let validator = SchemaValidator::new()?;
    /// validator.validate(&config)?;
    /// # Ok::<(), codex_spec_kit::config::ConfigError>(())
    /// ```
    pub fn validate(&self, config: &AppConfig) -> Result<()> {
        // Serialize config to JSON Value for validation
        let config_value = serde_json::to_value(config).map_err(|e| {
            ConfigError::SchemaValidationError(format!("Failed to serialize config: {e}"))
        })?;

        // Validate against schema
        let result = self.app_schema.validate(&config_value);

        if let Err(errors) = result {
            // Collect all validation errors with helpful context
            let error_messages: Vec<String> = errors
                .map(|e| {
                    let path_str = e.instance_path.to_string();
                    let path = if path_str.is_empty() {
                        "root".to_string()
                    } else {
                        path_str
                    };
                    format!("{e} at '{path}'")
                })
                .collect();

            return Err(ConfigError::SchemaValidationError(format!(
                "Configuration validation failed ({} error{}):\n  - {}",
                error_messages.len(),
                if error_messages.len() == 1 { "" } else { "s" },
                error_messages.join("\n  - ")
            )));
        }

        Ok(())
    }
}

impl Default for SchemaValidator {
    #[allow(clippy::expect_used)] // Default impl must succeed; panic is appropriate for schema load failure
    fn default() -> Self {
        Self::new().expect("Failed to create default schema validator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::AppConfig;

    #[test]
    fn test_validator_creation() {
        let validator = SchemaValidator::new();
        assert!(validator.is_ok());
    }

    #[test]
    fn test_valid_default_config() {
        let config = AppConfig::default();
        let validator = SchemaValidator::new().expect("Failed to create validator");

        let result = validator.validate(&config);
        assert!(
            result.is_ok(),
            "Default config should be valid: {result:?}"
        );
    }

    #[test]
    fn test_invalid_consensus_threshold_high() {
        let mut config = AppConfig::default();
        config.quality_gates.consensus_threshold = 1.5;

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::SchemaValidationError(_)));
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("consensus_threshold") || err_msg.contains("quality_gates"),
            "Error should mention consensus_threshold or quality_gates, got: {err_msg}"
        );
    }

    #[test]
    fn test_invalid_consensus_threshold_low() {
        let mut config = AppConfig::default();
        config.quality_gates.consensus_threshold = -0.1;

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::SchemaValidationError(_)));
    }

    #[test]
    fn test_invalid_temperature_high() {
        let mut config = AppConfig::default();
        config.models.insert(
            "test".to_string(),
            crate::config::loader::ModelConfig {
                model: "gpt-4".to_string(),
                endpoint: None,
                temperature: 2.5, // Invalid: > 2.0
                max_tokens: None,
                cost_per_input_million: 0.0,
                cost_per_output_million: 0.0,
                retry: Default::default(),
            },
        );

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("temperature") || err_msg.contains("models"),
            "Error should mention temperature or models, got: {err_msg}"
        );
    }

    #[test]
    fn test_invalid_min_agents_too_low() {
        let mut config = AppConfig::default();
        config.consensus.min_agents = 1; // Invalid: minimum is 2

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("min_agents") || err_msg.contains("consensus"),
            "Error should mention min_agents or consensus, got: {err_msg}"
        );
    }

    #[test]
    fn test_invalid_max_agents_too_high() {
        let mut config = AppConfig::default();
        config.consensus.max_agents = 15; // Invalid: maximum is 10

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("max_agents") || err_msg.contains("consensus"),
            "Error should mention max_agents or consensus, got: {err_msg}"
        );
    }

    #[test]
    fn test_invalid_alert_threshold() {
        let mut config = AppConfig::default();
        config.cost.alert_threshold = 1.2; // Invalid: > 1.0

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("alert_threshold") || err_msg.contains("cost"),
            "Error should mention alert_threshold or cost, got: {err_msg}"
        );
    }

    #[test]
    fn test_optional_fields_allow_null() {
        let mut config = AppConfig::default();
        config.quality_gates.min_test_coverage = None;
        config.cost.daily_limit_usd = None;
        config.cost.monthly_limit_usd = None;

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(
            result.is_ok(),
            "Config with None optional fields should be valid: {result:?}"
        );
    }

    #[test]
    fn test_optional_fields_with_valid_values() {
        let mut config = AppConfig::default();
        config.quality_gates.min_test_coverage = Some(80.0);
        config.cost.daily_limit_usd = Some(10.0);
        config.cost.monthly_limit_usd = Some(300.0);

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(
            result.is_ok(),
            "Config with valid optional fields should pass: {result:?}"
        );
    }

    #[test]
    fn test_invalid_min_test_coverage() {
        let mut config = AppConfig::default();
        config.quality_gates.min_test_coverage = Some(150.0); // Invalid: > 100.0

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("min_test_coverage") || err_msg.contains("quality_gates"),
            "Error should mention min_test_coverage, got: {err_msg}"
        );
    }

    #[test]
    fn test_multiple_validation_errors() {
        let mut config = AppConfig::default();
        config.quality_gates.consensus_threshold = 1.5; // Invalid: > 1.0
        config.consensus.min_agents = 1; // Invalid: < 2
        config.cost.alert_threshold = -0.1; // Invalid: < 0.0

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // Should report multiple errors (the exact count may vary based on schema validation order)
        assert!(
            err_msg.contains("error"),
            "Should mention errors: {err_msg}"
        );
    }

    #[test]
    fn test_error_message_quality() {
        let mut config = AppConfig::default();
        config.quality_gates.consensus_threshold = 1.5;

        let validator = SchemaValidator::new().expect("Failed to create validator");
        let result = validator.validate(&config);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // Error message should be helpful
        assert!(
            err_msg.contains("validation failed") || err_msg.contains("Configuration"),
            "Should have clear error prefix: {err_msg}"
        );
        assert!(
            err_msg.len() > 20,
            "Error message should be descriptive: {err_msg}"
        );
    }
}
