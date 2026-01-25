//! ACE Frame JSON Schema generator
//!
//! Generates JSON Schema for ACE Frame types:
//! - ace_frame@1.0 (ReflectionResult) - execution reflection
//! - ace_intake_frame@1.0 (AceIntakeFrame) - intake decision explainability
//!
//! Used by CI to verify schema stability.
//!
//! Usage:
//!   cargo run --bin ace-schema-gen -p codex-tui -- -o path/to/schemas/

use anyhow::{Context, Result};
use clap::Parser;
use codex_tui::{
    AceIntakeFrame, ReflectionResult, ACE_FRAME_SCHEMA_VERSION, ACE_INTAKE_FRAME_SCHEMA_VERSION,
};
use schemars::schema_for;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = "Generate JSON Schema for ACE Frame types")]
struct Args {
    /// Output directory for schema files
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Ensure output directory exists
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating output directory {}", args.out_dir.display()))?;

    // =========================================================================
    // ACE Frame schema (ace_frame@1.0) - execution reflection
    // =========================================================================

    let schema = schema_for!(ReflectionResult);
    let mut schema_value =
        serde_json::to_value(schema).context("serializing ace_frame schema to JSON value")?;

    // Add Draft-07 metadata
    if let Some(obj) = schema_value.as_object_mut() {
        obj.insert(
            "$schema".to_string(),
            serde_json::json!("http://json-schema.org/draft-07/schema#"),
        );
        obj.insert("title".to_string(), serde_json::json!("AceFrame"));
        obj.insert(
            "description".to_string(),
            serde_json::json!(format!(
                "ACE Frame {} - reflection result from spec-kit execution analysis",
                ACE_FRAME_SCHEMA_VERSION
            )),
        );
    }

    let schema_path = args.out_dir.join("ace_frame.schema.v1.json");
    let json =
        serde_json::to_string_pretty(&schema_value).context("formatting ace_frame schema as pretty JSON")?;
    std::fs::write(&schema_path, &json)
        .with_context(|| format!("writing ace_frame schema to {}", schema_path.display()))?;

    #[allow(clippy::print_stdout)]
    {
        println!("Generated: {}", schema_path.display());
    }

    // =========================================================================
    // ACE Intake Frame schema (ace_intake_frame@1.0) - intake decision explainability
    // =========================================================================

    let intake_schema = schema_for!(AceIntakeFrame);
    let mut intake_schema_value = serde_json::to_value(intake_schema)
        .context("serializing ace_intake_frame schema to JSON value")?;

    // Add Draft-07 metadata
    if let Some(obj) = intake_schema_value.as_object_mut() {
        obj.insert(
            "$schema".to_string(),
            serde_json::json!("http://json-schema.org/draft-07/schema#"),
        );
        obj.insert("title".to_string(), serde_json::json!("AceIntakeFrame"));
        obj.insert(
            "description".to_string(),
            serde_json::json!(format!(
                "ACE Intake Frame {} - decision explainability for spec-kit intake",
                ACE_INTAKE_FRAME_SCHEMA_VERSION
            )),
        );
    }

    let intake_schema_path = args.out_dir.join("ace_intake_frame.schema.v1.json");
    let intake_json = serde_json::to_string_pretty(&intake_schema_value)
        .context("formatting ace_intake_frame schema as pretty JSON")?;
    std::fs::write(&intake_schema_path, &intake_json)
        .with_context(|| format!("writing ace_intake_frame schema to {}", intake_schema_path.display()))?;

    #[allow(clippy::print_stdout)]
    {
        println!("Generated: {}", intake_schema_path.display());
    }

    Ok(())
}
