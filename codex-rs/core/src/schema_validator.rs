//! JSON Schema validator for configuration files.
//!
//! Provides runtime validation of TOML configuration against a JSON Schema.
//! This module supports both embedded default schemas and external schema files.

use jsonschema::{Draft, JSONSchema};
use serde_json::Value;
use std::path::Path;

/// JSON Schema validator for configuration files.
///
/// Validates TOML configuration files against JSON Schema Draft 7 specifications.
/// Can load schemas from external files or use an embedded default schema.
pub struct SchemaValidator {
    #[allow(dead_code)]
    schema: Value,
    compiled: JSONSchema,
}

impl SchemaValidator {
    /// Load JSON Schema from embedded resource or file.
    ///
    /// # Arguments
    ///
    /// * `schema_path` - Optional path to external JSON Schema file.
    ///                   If None, uses embedded default schema.
    ///
    /// # Returns
    ///
    /// Returns a new SchemaValidator instance or an error if schema loading fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use codex_core::schema_validator::SchemaValidator;
    /// use std::path::Path;
    ///
    /// // Use embedded default schema
    /// let validator = SchemaValidator::new(None).unwrap();
    ///
    /// // Load from external file
    /// let custom = SchemaValidator::new(Some(Path::new("config.schema.json"))).unwrap();
    /// ```
    pub fn new(schema_path: Option<&Path>) -> Result<Self, SchemaValidationError> {
        let schema = if let Some(path) = schema_path {
            // Load schema from external file
            let content =
                std::fs::read_to_string(path).map_err(|e| SchemaValidationError::IoError(e))?;
            serde_json::from_str(&content).map_err(|e| SchemaValidationError::JsonError(e))?
        } else {
            // Use embedded default schema
            Self::default_schema()
        };

        // Compile schema with Draft 7 support for efficient validation
        let compiled = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema)
            .map_err(|e| {
                SchemaValidationError::ValidationFailed(format!("Failed to compile schema: {}", e))
            })?;

        Ok(Self { schema, compiled })
    }

    /// Validate TOML configuration against the loaded schema.
    ///
    /// Parses TOML to JSON and validates against the JSON Schema Draft 7.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - TOML configuration string to validate
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if validation passes, or a Vec of validation errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use codex_core::schema_validator::SchemaValidator;
    ///
    /// let validator = SchemaValidator::new(None).unwrap();
    /// let config = "[general]\nname = \"test\"";
    ///
    /// match validator.validate(config) {
    ///     Ok(()) => println!("Config is valid"),
    ///     Err(errors) => eprintln!("Validation failed: {:?}", errors),
    /// }
    /// ```
    pub fn validate(&self, config_toml: &str) -> Result<(), Vec<SchemaValidationError>> {
        // Parse TOML to JSON for schema validation
        let config_value: Value = toml::from_str(config_toml)
            .map_err(|e| vec![SchemaValidationError::TomlParse(e.to_string())])?;

        // Validate against compiled JSON Schema
        if let Err(validation_errors) = self.compiled.validate(&config_value) {
            let errors: Vec<SchemaValidationError> = validation_errors
                .map(|e| {
                    SchemaValidationError::ValidationFailed(format!("{} at {}", e, e.instance_path))
                })
                .collect();

            return Err(errors);
        }

        tracing::debug!(
            "Schema validation passed for config ({} bytes)",
            config_toml.len()
        );

        Ok(())
    }

    /// Returns the embedded default JSON Schema.
    ///
    /// Provides a comprehensive JSON Schema Draft 7 schema for configuration validation.
    /// Includes quality gates, hot-reload, and validation settings for SPEC-939.
    fn default_schema() -> Value {
        serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "$id": "https://github.com/theturtlecsz/code/schemas/config.schema.json",
            "title": "Codex Configuration Schema",
            "description": "JSON Schema for codex-rs configuration validation (SPEC-939)",
            "type": "object",
            "properties": {
                "quality_gates": {
                    "type": "object",
                    "description": "Quality gate agent configuration per checkpoint",
                    "properties": {
                        "plan": {
                            "type": "array",
                            "description": "Agent names for plan checkpoint",
                            "items": { "type": "string" },
                            "minItems": 1,
                            "maxItems": 5
                        },
                        "tasks": {
                            "type": "array",
                            "description": "Agent names for tasks checkpoint",
                            "items": { "type": "string" },
                            "minItems": 1,
                            "maxItems": 5
                        },
                        "validate": {
                            "type": "array",
                            "description": "Agent names for validate checkpoint",
                            "items": { "type": "string" },
                            "minItems": 1,
                            "maxItems": 5
                        },
                        "audit": {
                            "type": "array",
                            "description": "Agent names for audit checkpoint",
                            "items": { "type": "string" },
                            "minItems": 1,
                            "maxItems": 5
                        },
                        "unlock": {
                            "type": "array",
                            "description": "Agent names for unlock checkpoint",
                            "items": { "type": "string" },
                            "minItems": 1,
                            "maxItems": 5
                        }
                    },
                    "required": ["plan", "tasks", "validate", "audit", "unlock"],
                    "additionalProperties": false
                },
                "hot_reload": {
                    "type": "object",
                    "description": "Hot-reload configuration for live config updates",
                    "properties": {
                        "enabled": {
                            "type": "boolean",
                            "description": "Enable hot-reload of configuration files",
                            "default": true
                        },
                        "debounce_ms": {
                            "type": "integer",
                            "description": "Debounce delay in milliseconds",
                            "minimum": 500,
                            "maximum": 10000,
                            "default": 2000
                        },
                        "watch_paths": {
                            "type": "array",
                            "description": "Additional paths to watch for changes",
                            "items": { "type": "string" },
                            "default": []
                        }
                    },
                    "additionalProperties": false
                },
                "validation": {
                    "type": "object",
                    "description": "Startup validation configuration",
                    "properties": {
                        "check_api_keys": {
                            "type": "boolean",
                            "description": "Validate API keys at startup",
                            "default": true
                        },
                        "check_commands": {
                            "type": "boolean",
                            "description": "Validate slash commands at startup",
                            "default": true
                        },
                        "strict_schema": {
                            "type": "boolean",
                            "description": "Enable strict JSON Schema validation",
                            "default": true
                        }
                    },
                    "additionalProperties": false
                }
            },
            "additionalProperties": true
        })
    }
}

/// Errors that can occur during schema validation.
#[derive(Debug)]
pub enum SchemaValidationError {
    /// I/O error reading schema file
    IoError(std::io::Error),

    /// JSON parsing error in schema file
    JsonError(serde_json::Error),

    /// TOML parsing error in config file
    TomlParse(String),

    /// Schema validation failed
    ValidationFailed(String),
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaValidationError::IoError(e) => write!(f, "Schema file I/O error: {}", e),
            SchemaValidationError::JsonError(e) => write!(f, "Schema JSON parse error: {}", e),
            SchemaValidationError::TomlParse(msg) => write!(f, "Config TOML parse error: {}", msg),
            SchemaValidationError::ValidationFailed(msg) => {
                write!(f, "Schema validation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for SchemaValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SchemaValidationError::IoError(e) => Some(e),
            SchemaValidationError::JsonError(e) => Some(e),
            SchemaValidationError::TomlParse(_) => None,
            SchemaValidationError::ValidationFailed(_) => None,
        }
    }
}

impl From<std::io::Error> for SchemaValidationError {
    fn from(err: std::io::Error) -> Self {
        SchemaValidationError::IoError(err)
    }
}

impl From<serde_json::Error> for SchemaValidationError {
    fn from(err: serde_json::Error) -> Self {
        SchemaValidationError::JsonError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_compilation() {
        // Verify schema compiles successfully with Draft 7
        let validator = SchemaValidator::new(None);
        assert!(validator.is_ok(), "Schema should compile without errors");
    }

    #[test]
    fn test_default_schema_is_valid_json() {
        let schema = SchemaValidator::default_schema();
        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(schema["type"], "object");

        // Verify SPEC-939 properties exist
        assert!(schema["properties"]["quality_gates"].is_object());
        assert!(schema["properties"]["hot_reload"].is_object());
        assert!(schema["properties"]["validation"].is_object());
    }

    #[test]
    fn test_valid_quality_gates_config_passes() {
        let validator = SchemaValidator::new(None).unwrap();
        let config = r#"
[quality_gates]
plan = ["gemini", "claude"]
tasks = ["gemini"]
validate = ["gemini", "claude", "code"]
audit = ["gemini"]
unlock = ["gemini", "claude"]
        "#;

        let result = validator.validate(config);
        assert!(
            result.is_ok(),
            "Valid quality_gates config should pass validation"
        );
    }

    #[test]
    fn test_valid_hot_reload_config_passes() {
        let validator = SchemaValidator::new(None).unwrap();
        let config = r#"
[hot_reload]
enabled = true
debounce_ms = 2000
watch_paths = ["config.toml", "custom.toml"]
        "#;

        let result = validator.validate(config);
        assert!(
            result.is_ok(),
            "Valid hot_reload config should pass validation"
        );
    }

    #[test]
    fn test_invalid_quality_gates_empty_array_fails() {
        let validator = SchemaValidator::new(None).unwrap();
        let config = r#"
[quality_gates]
plan = []
tasks = ["gemini"]
validate = ["gemini"]
audit = ["gemini"]
unlock = ["gemini"]
        "#;

        let result = validator.validate(config);
        assert!(
            result.is_err(),
            "Empty plan array should violate minItems constraint"
        );

        if let Err(errors) = result {
            assert!(!errors.is_empty());
            let error_str = format!("{:?}", errors[0]);
            assert!(error_str.contains("ValidationFailed"));
        }
    }

    #[test]
    fn test_invalid_quality_gates_too_many_agents_fails() {
        let validator = SchemaValidator::new(None).unwrap();
        let config = r#"
[quality_gates]
plan = ["gemini", "claude", "code", "gpt4", "gpt5", "extra"]
tasks = ["gemini"]
validate = ["gemini"]
audit = ["gemini"]
unlock = ["gemini"]
        "#;

        let result = validator.validate(config);
        assert!(
            result.is_err(),
            "Plan array with >5 items should violate maxItems constraint"
        );
    }

    #[test]
    fn test_invalid_debounce_ms_out_of_range_fails() {
        let validator = SchemaValidator::new(None).unwrap();
        let config = r#"
[hot_reload]
enabled = true
debounce_ms = 100
        "#;

        let result = validator.validate(config);
        assert!(
            result.is_err(),
            "debounce_ms < 500 should violate minimum constraint"
        );
    }

    #[test]
    fn test_invalid_type_fails() {
        let validator = SchemaValidator::new(None).unwrap();
        let config = r#"
[hot_reload]
enabled = "yes"
        "#;

        let result = validator.validate(config);
        assert!(
            result.is_err(),
            "String value for boolean field should fail validation"
        );
    }

    #[test]
    fn test_missing_required_quality_gate_field_fails() {
        let validator = SchemaValidator::new(None).unwrap();
        let config = r#"
[quality_gates]
plan = ["gemini"]
tasks = ["gemini"]
validate = ["gemini"]
audit = ["gemini"]
        "#;

        let result = validator.validate(config);
        assert!(
            result.is_err(),
            "Missing required 'unlock' field should fail validation"
        );
    }

    #[test]
    fn test_validate_rejects_invalid_toml() {
        let validator = SchemaValidator::new(None).unwrap();
        let invalid_config = "[[[ invalid toml";

        let result = validator.validate(invalid_config);
        assert!(result.is_err());

        if let Err(errors) = result {
            assert_eq!(errors.len(), 1);
            assert!(matches!(errors[0], SchemaValidationError::TomlParse(_)));
        }
    }

    #[test]
    fn test_full_spec939_config_passes() {
        let validator = SchemaValidator::new(None).unwrap();
        let config = r#"
[quality_gates]
plan = ["gemini", "claude"]
tasks = ["gemini"]
validate = ["gemini", "claude"]
audit = ["gemini"]
unlock = ["gemini", "claude"]

[hot_reload]
enabled = true
debounce_ms = 2000
watch_paths = ["config.toml"]

[validation]
check_api_keys = true
check_commands = true
strict_schema = true
        "#;

        let result = validator.validate(config);
        assert!(
            result.is_ok(),
            "Full SPEC-939 config should pass validation"
        );
    }
}
