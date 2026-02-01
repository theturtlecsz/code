//! SPEC-KIT-979 D52: Import local-memory items into memvid capsule
//!
//! ## Safety
//! - Default behavior (no flags): equivalent to --status, no writes
//! - Explicit --all required to perform writes
//! - Conflicts detected but not auto-resolved
//!
//! ## Deduplication
//! - Skip duplicates by (source_backend, source_id)
//! - Detect conflicts: same ID, different content hash

// MAINT-930: Allow format string flexibility in CLI output
#![allow(
    clippy::uninlined_format_args,
    clippy::collapsible_if,
    clippy::ptr_arg,
    clippy::manual_str_repeat,
    clippy::manual_repeat_n
)]

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{MemoryExportRecord, resolve_database_path, row_to_export};

// =============================================================================
// CLI Arguments
// =============================================================================

#[derive(Debug, Parser)]
pub struct ImportArgs {
    /// Show what would be imported (counts + paths), no writes [default]
    #[arg(long)]
    pub status: bool,

    /// Dry run: show full plan without writing
    #[arg(long)]
    pub dry_run: bool,

    /// Perform the import (required to write)
    #[arg(long)]
    pub all: bool,

    /// Post-import verification (counts + spot checks)
    #[arg(long)]
    pub verify: bool,

    /// Override local-memory database path
    #[arg(long = "database")]
    pub database: Option<PathBuf>,

    /// Override capsule path (default: .speckit/memvid/workspace.mv2)
    #[arg(long = "capsule")]
    pub capsule: Option<PathBuf>,

    /// Spec ID for capsule URIs (default: "lm-import")
    #[arg(long = "spec-id")]
    pub spec_id: Option<String>,

    /// Run ID for capsule URIs (default: auto-generated)
    #[arg(long = "run-id")]
    pub run_id: Option<String>,

    /// Filter by domain (can be repeated)
    #[arg(long = "domain", short = 'd')]
    pub domains: Vec<String>,

    /// Filter by minimum importance (1-10)
    #[arg(long = "min-importance")]
    pub min_importance: Option<i64>,
}

// =============================================================================
// Import Artifact Types
// =============================================================================

/// Provenance metadata stored with each imported memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProvenance {
    /// Source backend identifier
    pub source_backend: String,
    /// Original ID in source
    pub source_id: String,
    /// SHA256 hash of raw content
    pub source_hash: String,
    /// Batch identifier (timestamp + source hash prefix)
    pub import_batch_id: String,
    /// Import timestamp
    pub imported_at: DateTime<Utc>,
}

/// Memory metadata preserved from source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMeta {
    pub importance: i64,
    pub tags: Vec<String>,
    pub domain: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub session_id: Option<String>,
}

/// Artifact ready for capsule import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportArtifact {
    /// Original local-memory ID
    pub source_id: String,
    /// Content from local-memory
    pub content: String,
    /// Import provenance metadata
    pub provenance: ImportProvenance,
    /// Original memory metadata
    pub memory_meta: MemoryMeta,
}

/// Classification of memories for import.
#[derive(Debug, Default)]
pub struct ImportClassification {
    /// New memories to import
    pub new: Vec<MemoryExportRecord>,
    /// Memories that already exist (same content)
    pub duplicates: Vec<MemoryExportRecord>,
    /// Memories with same ID but different content
    pub conflicts: Vec<ConflictInfo>,
}

/// Information about a conflicting memory.
#[derive(Debug)]
pub struct ConflictInfo {
    pub memory: MemoryExportRecord,
    pub existing_hash: String,
    pub new_hash: String,
}

/// Summary of import operation.
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct ImportSummary {
    pub source_path: PathBuf,
    pub capsule_path: PathBuf,
    pub spec_id: String,
    pub run_id: String,
    pub total_source: usize,
    pub filtered_count: usize,
    pub existing_count: usize,
    pub new_count: usize,
    pub duplicate_count: usize,
    pub conflict_count: usize,
    pub imported_count: usize,
}

// =============================================================================
// Entry Point
// =============================================================================

/// Main entry point for the import command.
pub async fn run_import(args: ImportArgs) -> Result<()> {
    // Determine mode: default to --status if no flags
    let mode = determine_mode(&args);

    match mode {
        ImportMode::Status => run_status(args).await,
        ImportMode::DryRun => run_dry_run(args).await,
        ImportMode::All => run_all(args).await,
        ImportMode::Verify => run_verify(args).await,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportMode {
    Status,
    DryRun,
    All,
    Verify,
}

fn determine_mode(args: &ImportArgs) -> ImportMode {
    if args.verify {
        ImportMode::Verify
    } else if args.all {
        ImportMode::All
    } else if args.dry_run {
        ImportMode::DryRun
    } else {
        // Default: --status (safe, no writes)
        ImportMode::Status
    }
}

// =============================================================================
// Mode: Status
// =============================================================================

async fn run_status(args: ImportArgs) -> Result<()> {
    let db_path = resolve_database_path(args.database.as_ref())?;
    let capsule_path = resolve_capsule_path(args.capsule.as_ref())?;
    let spec_id = args
        .spec_id
        .clone()
        .unwrap_or_else(|| "lm-import".to_string());

    // Collect memories from source
    let all_memories = collect_memories(&db_path)?;
    let filtered = filter_memories(&all_memories, &args);

    // Check existing in capsule (simplified: scan for provenance)
    let existing = scan_existing_imports(&capsule_path)?;
    let classification = classify_imports(&filtered, &existing);

    // Print status
    println!("Source: {}", db_path.display());
    println!("  Total memories: {}", all_memories.len());
    if !args.domains.is_empty() || args.min_importance.is_some() {
        println!("  Filtered: {}", filtered.len());
    }
    println!();
    println!(
        "Destination: {} (spec: {})",
        capsule_path.display(),
        spec_id
    );
    println!("  Existing memories (this source): {}", existing.len());
    println!();
    println!("Import plan:");
    println!("  New:        {}", classification.new.len());
    println!(
        "  Duplicates: {} (will skip)",
        classification.duplicates.len()
    );
    println!("  Conflicts:  {}", classification.conflicts.len());
    println!();

    if classification.new.is_empty() {
        println!("Nothing new to import.");
    } else {
        println!(
            "Run with --all to import {} new memories.",
            classification.new.len()
        );
    }

    if !classification.conflicts.is_empty() {
        println!();
        println!(
            "WARNING: {} conflicts detected (same ID, different content).",
            classification.conflicts.len()
        );
        println!("Run with --dry-run to see details.");
    }

    Ok(())
}

// =============================================================================
// Mode: Dry Run
// =============================================================================

async fn run_dry_run(args: ImportArgs) -> Result<()> {
    let db_path = resolve_database_path(args.database.as_ref())?;
    let capsule_path = resolve_capsule_path(args.capsule.as_ref())?;

    // Collect and classify
    let all_memories = collect_memories(&db_path)?;
    let filtered = filter_memories(&all_memories, &args);
    let existing = scan_existing_imports(&capsule_path)?;
    let classification = classify_imports(&filtered, &existing);

    println!(
        "DRY RUN: Would import {} memories (skipping {} duplicates)",
        classification.new.len(),
        classification.duplicates.len()
    );
    println!();

    // Show new memories
    if !classification.new.is_empty() {
        println!("NEW:");
        for (i, mem) in classification.new.iter().enumerate().take(20) {
            let domain = mem.domain.as_deref().unwrap_or("none");
            let snippet: String = mem.content.chars().take(60).collect();
            println!(
                "{}. {} [importance={}, domain={}]",
                i + 1,
                mem.id,
                mem.importance,
                domain
            );
            println!("   \"{}...\"", snippet.replace('\n', " "));
        }
        if classification.new.len() > 20 {
            println!("... and {} more", classification.new.len() - 20);
        }
        println!();
    }

    // Show duplicates
    if !classification.duplicates.is_empty() {
        println!("SKIP (duplicate):");
        for mem in classification.duplicates.iter().take(5) {
            println!("- {}: already exists (hash match)", mem.id);
        }
        if classification.duplicates.len() > 5 {
            println!(
                "... and {} more duplicates",
                classification.duplicates.len() - 5
            );
        }
        println!();
    }

    // Show conflicts
    if !classification.conflicts.is_empty() {
        println!("CONFLICT (same ID, different hash):");
        for conflict in &classification.conflicts {
            println!(
                "- {}: existing hash={}, new hash={}",
                conflict.memory.id,
                &conflict.existing_hash[..8],
                &conflict.new_hash[..8]
            );
        }
        println!();
    }

    println!("No changes made.");

    Ok(())
}

// =============================================================================
// Mode: All (Execute Import)
// =============================================================================

async fn run_all(args: ImportArgs) -> Result<()> {
    let db_path = resolve_database_path(args.database.as_ref())?;
    let capsule_path = resolve_capsule_path(args.capsule.as_ref())?;
    let spec_id = args
        .spec_id
        .clone()
        .unwrap_or_else(|| "lm-import".to_string());
    let run_id = args.run_id.clone().unwrap_or_else(generate_run_id);

    println!("Source: {}", db_path.display());
    println!("Destination: {}", capsule_path.display());
    println!("  Spec ID: {}", spec_id);
    println!("  Run ID:  {}", run_id);
    println!();

    // Collect and classify
    let all_memories = collect_memories(&db_path)?;
    let filtered = filter_memories(&all_memories, &args);
    let existing = scan_existing_imports(&capsule_path)?;
    let classification = classify_imports(&filtered, &existing);

    if classification.new.is_empty() {
        println!("Nothing new to import.");
        return Ok(());
    }

    // Generate batch ID for this import
    let batch_id = generate_batch_id();

    // Transform to artifacts
    let artifacts: Vec<ImportArtifact> = classification
        .new
        .iter()
        .map(|m| transform_to_artifact(m, &batch_id))
        .collect();

    // Write to capsule
    println!("Importing {} memories...", artifacts.len());
    let imported = write_to_capsule(&capsule_path, &spec_id, &run_id, &artifacts)?;

    println!();
    println!("Summary:");
    println!("  Imported:   {}", imported);
    println!(
        "  Skipped:    {} (duplicates)",
        classification.duplicates.len()
    );
    println!("  Conflicts:  {}", classification.conflicts.len());

    if !classification.conflicts.is_empty() {
        println!();
        println!(
            "WARNING: {} conflicts detected (same ID, different hash - not imported).",
            classification.conflicts.len()
        );
        println!("Re-run with --dry-run to see details.");
        println!("Future: use --upsert to update existing, or --allow-conflicts to import as new.");
    } else {
        println!();
        println!("Run with --verify to confirm.");
    }

    Ok(())
}

// =============================================================================
// Mode: Verify
// =============================================================================

async fn run_verify(args: ImportArgs) -> Result<()> {
    let db_path = resolve_database_path(args.database.as_ref())?;
    let capsule_path = resolve_capsule_path(args.capsule.as_ref())?;

    println!("Verifying import...");
    println!();

    // Collect source memories
    let all_memories = collect_memories(&db_path)?;
    let filtered = filter_memories(&all_memories, &args);

    // Scan capsule for imported memories
    let existing = scan_existing_imports(&capsule_path)?;

    // Count matches
    let mut matched = 0;
    let mut mismatched = 0;
    let mut missing = 0;

    for mem in &filtered {
        let source_hash = compute_hash(&mem.content);
        match existing.get(&mem.id) {
            Some(existing_hash) if existing_hash == &source_hash => matched += 1,
            Some(_) => mismatched += 1,
            None => missing += 1,
        }
    }

    println!("Source (filtered): {} memories", filtered.len());
    println!("Capsule (this source): {} memories", existing.len());
    println!();

    // Spot checks
    println!("Spot checks (up to 5):");
    let mut checks_done = 0;
    for mem in filtered.iter().take(5) {
        let source_hash = compute_hash(&mem.content);
        match existing.get(&mem.id) {
            Some(existing_hash) if existing_hash == &source_hash => {
                println!("  {}: OK (content hash matches)", mem.id);
                checks_done += 1;
            }
            Some(existing_hash) => {
                println!(
                    "  {}: MISMATCH (expected {}, got {})",
                    mem.id,
                    &source_hash[..8],
                    &existing_hash[..8]
                );
                checks_done += 1;
            }
            None => {
                println!("  {}: MISSING (not in capsule)", mem.id);
                checks_done += 1;
            }
        }
    }

    if checks_done == 0 {
        println!("  (no memories to check)");
    }

    println!();

    if mismatched == 0 && missing == 0 {
        println!("Verification: PASSED");
        println!("  Matched: {}/{}", matched, filtered.len());
    } else {
        println!("Verification: FAILED");
        println!("  Matched:    {}", matched);
        println!("  Mismatched: {}", mismatched);
        println!("  Missing:    {}", missing);
    }

    Ok(())
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Resolve capsule path with fallback to default.
fn resolve_capsule_path(explicit: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path.clone());
    }

    // Default: current directory + .speckit/memvid/workspace.mv2
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    Ok(cwd.join(".speckit").join("memvid").join("workspace.mv2"))
}

/// Collect all memories from the local-memory database.
fn collect_memories(db_path: &PathBuf) -> Result<Vec<MemoryExportRecord>> {
    let conn = Connection::open(db_path).with_context(|| {
        format!(
            "failed to open local-memory database at {}",
            db_path.display()
        )
    })?;

    let mut stmt = conn.prepare(
        "SELECT id, content, source, importance, tags, session_id, domain, \
         created_at, updated_at, agent_type, agent_context, access_scope, slug \
         FROM memories ORDER BY created_at",
    )?;

    let records = stmt.query_map([], row_to_export)?;
    let mut result = Vec::new();
    for record in records {
        result.push(record?);
    }
    Ok(result)
}

/// Filter memories by domain and importance.
fn filter_memories(memories: &[MemoryExportRecord], args: &ImportArgs) -> Vec<MemoryExportRecord> {
    memories
        .iter()
        .filter(|m| {
            // Domain filter
            if !args.domains.is_empty() {
                if let Some(ref domain) = m.domain {
                    if !args.domains.iter().any(|d| domain.contains(d)) {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // Importance filter
            if let Some(min_importance) = args.min_importance {
                if m.importance < min_importance {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect()
}

/// Scan existing imports in the capsule.
///
/// Returns a map of source_id -> content_hash for memories imported from local-memory.
fn scan_existing_imports(capsule_path: &PathBuf) -> Result<HashMap<String, String>> {
    let mut existing = HashMap::new();

    // Check if capsule exists
    if !capsule_path.exists() {
        return Ok(existing);
    }

    // Read the capsule and look for artifacts with local-memory provenance
    // For now, use a simple approach: scan the index file if it exists
    let index_path = capsule_path.with_extension("index.json");
    if index_path.exists() {
        let content = std::fs::read_to_string(&index_path)?;
        if let Ok(index) = serde_json::from_str::<ImportIndex>(&content) {
            for entry in index.entries {
                if entry.source_backend == "local-memory" {
                    existing.insert(entry.source_id, entry.source_hash);
                }
            }
        }
    }

    Ok(existing)
}

/// Classify memories for import.
fn classify_imports(
    memories: &[MemoryExportRecord],
    existing: &HashMap<String, String>,
) -> ImportClassification {
    let mut classification = ImportClassification::default();

    for memory in memories {
        let new_hash = compute_hash(&memory.content);

        match existing.get(&memory.id) {
            None => {
                classification.new.push(memory.clone());
            }
            Some(existing_hash) if existing_hash == &new_hash => {
                classification.duplicates.push(memory.clone());
            }
            Some(existing_hash) => {
                classification.conflicts.push(ConflictInfo {
                    memory: memory.clone(),
                    existing_hash: existing_hash.clone(),
                    new_hash,
                });
            }
        }
    }

    classification
}

/// Transform a memory record to an import artifact.
fn transform_to_artifact(memory: &MemoryExportRecord, batch_id: &str) -> ImportArtifact {
    let source_hash = compute_hash(&memory.content);

    ImportArtifact {
        source_id: memory.id.clone(),
        content: memory.content.clone(),
        provenance: ImportProvenance {
            source_backend: "local-memory".to_string(),
            source_id: memory.id.clone(),
            source_hash,
            import_batch_id: batch_id.to_string(),
            imported_at: Utc::now(),
        },
        memory_meta: MemoryMeta {
            importance: memory.importance,
            tags: memory.tags.clone(),
            domain: memory.domain.clone(),
            created_at: memory.created_at.clone(),
            updated_at: memory.updated_at.clone(),
            session_id: memory.session_id.clone(),
        },
    }
}

/// Write artifacts to the capsule.
fn write_to_capsule(
    capsule_path: &PathBuf,
    spec_id: &str,
    run_id: &str,
    artifacts: &[ImportArtifact],
) -> Result<usize> {
    // Ensure parent directory exists
    if let Some(parent) = capsule_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create or open the capsule file
    let mut capsule_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(capsule_path)?;

    // Write header if file is empty
    let metadata = capsule_file.metadata()?;
    if metadata.len() == 0 {
        use std::io::Write;
        capsule_file.write_all(b"MV2\x00\x01")?;
    }

    // Build index entries
    let mut index_entries = Vec::new();

    // Write each artifact
    for (i, artifact) in artifacts.iter().enumerate() {
        // Serialize artifact to JSON
        let data = serde_json::to_vec(artifact)?;

        // Create metadata for the record
        let uri = format!(
            "mv2://default/{}/{}/artifact/memory/{}.json",
            spec_id, run_id, artifact.source_id
        );
        let record_meta = serde_json::json!({
            "uri": uri,
            "object_type": "memory",
            "metadata": {
                "source_backend": artifact.provenance.source_backend,
                "source_id": artifact.provenance.source_id,
                "source_hash": artifact.provenance.source_hash,
                "import_batch_id": artifact.provenance.import_batch_id,
            }
        });
        let meta_bytes = serde_json::to_vec(&record_meta)?;

        // Write record: [u32 record_len][u8 kind][u32 meta_len][meta][payload]
        use std::io::Write;
        let record_len = 1 + 4 + meta_bytes.len() + data.len();
        capsule_file.write_all(&(record_len as u32).to_le_bytes())?;
        capsule_file.write_all(&[0u8])?; // kind = Artifact
        capsule_file.write_all(&(meta_bytes.len() as u32).to_le_bytes())?;
        capsule_file.write_all(&meta_bytes)?;
        capsule_file.write_all(&data)?;

        // Add to index
        index_entries.push(ImportIndexEntry {
            source_backend: artifact.provenance.source_backend.clone(),
            source_id: artifact.provenance.source_id.clone(),
            source_hash: artifact.provenance.source_hash.clone(),
            uri,
        });

        // Progress indicator
        if (i + 1) % 10 == 0 || i + 1 == artifacts.len() {
            let progress = (i + 1) * 40 / artifacts.len();
            let bar: String = std::iter::repeat('=').take(progress).collect();
            let spaces: String = std::iter::repeat(' ').take(40 - progress).collect();
            eprint!("\r  [{}{}] {}/{}", bar, spaces, i + 1, artifacts.len());
        }
    }
    eprintln!();

    // Update index file
    let index_path = capsule_path.with_extension("index.json");
    let mut existing_index = if index_path.exists() {
        let content = std::fs::read_to_string(&index_path)?;
        serde_json::from_str::<ImportIndex>(&content).unwrap_or_default()
    } else {
        ImportIndex::default()
    };
    existing_index.entries.extend(index_entries);
    std::fs::write(&index_path, serde_json::to_string_pretty(&existing_index)?)?;

    Ok(artifacts.len())
}

/// Compute SHA256 hash of content.
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Generate a unique run ID.
fn generate_run_id() -> String {
    let now = Utc::now();
    let timestamp = now.format("%Y%m%dT%H%M%S").to_string();
    let hash = compute_hash(&format!("{}{:?}", timestamp, std::time::Instant::now()));
    format!("import-{}-{}", timestamp, &hash[..6])
}

/// Generate a batch ID for this import.
fn generate_batch_id() -> String {
    let now = Utc::now();
    let timestamp = now.format("%Y%m%dT%H%M%S").to_string();
    let hash = compute_hash(&format!(
        "batch-{}{:?}",
        timestamp,
        std::time::Instant::now()
    ));
    format!("{}-{}", timestamp, &hash[..8])
}

// =============================================================================
// Index Types
// =============================================================================

#[derive(Debug, Default, Serialize, Deserialize)]
struct ImportIndex {
    entries: Vec<ImportIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImportIndexEntry {
    source_backend: String,
    source_id: String,
    source_hash: String,
    uri: String,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash("hello world");
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex chars
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_generate_run_id() {
        let run_id = generate_run_id();
        assert!(run_id.starts_with("import-"));
        assert!(run_id.len() > 20); // timestamp + hash
    }

    #[test]
    fn test_filter_memories_by_importance() {
        let memories = vec![
            MemoryExportRecord {
                id: "1".to_string(),
                content: "low importance".to_string(),
                source: None,
                importance: 3,
                tags: vec![],
                session_id: None,
                domain: None,
                created_at: "2024-01-01".to_string(),
                updated_at: "2024-01-01".to_string(),
                agent_type: None,
                agent_context: None,
                access_scope: None,
                slug: None,
            },
            MemoryExportRecord {
                id: "2".to_string(),
                content: "high importance".to_string(),
                source: None,
                importance: 9,
                tags: vec![],
                session_id: None,
                domain: None,
                created_at: "2024-01-01".to_string(),
                updated_at: "2024-01-01".to_string(),
                agent_type: None,
                agent_context: None,
                access_scope: None,
                slug: None,
            },
        ];

        let args = ImportArgs {
            status: false,
            dry_run: false,
            all: false,
            verify: false,
            database: None,
            capsule: None,
            spec_id: None,
            run_id: None,
            domains: vec![],
            min_importance: Some(8),
        };

        let filtered = filter_memories(&memories, &args);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "2");
    }

    #[test]
    fn test_classify_imports() {
        let memories = vec![
            MemoryExportRecord {
                id: "new-1".to_string(),
                content: "new content".to_string(),
                source: None,
                importance: 8,
                tags: vec![],
                session_id: None,
                domain: None,
                created_at: "2024-01-01".to_string(),
                updated_at: "2024-01-01".to_string(),
                agent_type: None,
                agent_context: None,
                access_scope: None,
                slug: None,
            },
            MemoryExportRecord {
                id: "dup-1".to_string(),
                content: "duplicate content".to_string(),
                source: None,
                importance: 8,
                tags: vec![],
                session_id: None,
                domain: None,
                created_at: "2024-01-01".to_string(),
                updated_at: "2024-01-01".to_string(),
                agent_type: None,
                agent_context: None,
                access_scope: None,
                slug: None,
            },
            MemoryExportRecord {
                id: "conflict-1".to_string(),
                content: "new version".to_string(),
                source: None,
                importance: 8,
                tags: vec![],
                session_id: None,
                domain: None,
                created_at: "2024-01-01".to_string(),
                updated_at: "2024-01-01".to_string(),
                agent_type: None,
                agent_context: None,
                access_scope: None,
                slug: None,
            },
        ];

        let mut existing = HashMap::new();
        // dup-1 exists with same content
        existing.insert("dup-1".to_string(), compute_hash("duplicate content"));
        // conflict-1 exists with different content
        existing.insert("conflict-1".to_string(), compute_hash("old version"));

        let classification = classify_imports(&memories, &existing);

        assert_eq!(classification.new.len(), 1);
        assert_eq!(classification.new[0].id, "new-1");

        assert_eq!(classification.duplicates.len(), 1);
        assert_eq!(classification.duplicates[0].id, "dup-1");

        assert_eq!(classification.conflicts.len(), 1);
        assert_eq!(classification.conflicts[0].memory.id, "conflict-1");
    }

    #[test]
    fn test_transform_to_artifact() {
        let memory = MemoryExportRecord {
            id: "test-id".to_string(),
            content: "test content".to_string(),
            source: None,
            importance: 9,
            tags: vec!["tag1".to_string()],
            session_id: Some("sess-1".to_string()),
            domain: Some("test-domain".to_string()),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-02".to_string(),
            agent_type: None,
            agent_context: None,
            access_scope: None,
            slug: None,
        };

        let artifact = transform_to_artifact(&memory, "batch-123");

        assert_eq!(artifact.source_id, "test-id");
        assert_eq!(artifact.content, "test content");
        assert_eq!(artifact.provenance.source_backend, "local-memory");
        assert_eq!(artifact.provenance.source_id, "test-id");
        assert_eq!(artifact.provenance.import_batch_id, "batch-123");
        assert_eq!(artifact.memory_meta.importance, 9);
        assert_eq!(artifact.memory_meta.domain, Some("test-domain".to_string()));
    }
}
