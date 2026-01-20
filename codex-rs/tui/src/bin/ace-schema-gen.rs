//! ACE Frame JSON Schema generator
//!
//! Generates JSON Schema for ACE Frame types (ReflectionResult, ReflectedPattern).
//! Used by CI to verify schema stability.
//!
//! Usage:
//!   cargo run --bin ace-schema-gen -p codex-tui -- -o path/to/schemas/

use anyhow::{Context, Result};
use clap::Parser;
use codex_tui::{ACE_FRAME_SCHEMA_VERSION, ReflectionResult};
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

    // Generate schema for ReflectionResult (the ACE Frame)
    let schema = schema_for!(ReflectionResult);
    let mut schema_value =
        serde_json::to_value(schema).context("serializing schema to JSON value")?;

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
        serde_json::to_string_pretty(&schema_value).context("formatting schema as pretty JSON")?;
    std::fs::write(&schema_path, &json)
        .with_context(|| format!("writing schema to {}", schema_path.display()))?;

    #[allow(clippy::print_stdout)]
    {
        println!("Generated: {}", schema_path.display());
    }
    Ok(())
}
