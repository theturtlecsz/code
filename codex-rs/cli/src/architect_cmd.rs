//! Architect Sidecar CLI - Budget-aware intelligence module.
//!
//! Operates on a "Cache-First, Ask-Later" principle to minimize API costs.
//!
//! # Commands
//! - `code architect refresh` - Update local forensic data (free, local)
//! - `code architect ask <query>` - Get cached answer or query NotebookLM
//! - `code architect audit <crate>` - Investigate a dependency

use anyhow::{Context, Result, bail};
use clap::Parser;
use codex_core::architect::{self, HarvesterConfig};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;

/// The Architect Sidecar - forensic intelligence for codex-rs.
#[derive(Debug, Parser)]
pub struct ArchitectCli {
    #[command(subcommand)]
    pub cmd: ArchitectCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum ArchitectCommand {
    /// Update local forensic maps (churn, complexity, skeleton). Cost: $0.
    Refresh(RefreshArgs),

    /// Ask a question. Uses cache first; prompts before API call.
    Ask(AskArgs),

    /// Audit a Rust crate for security and maintenance. Cached.
    Audit(AuditArgs),

    /// Show vault status (cached answers, freshness).
    Status,

    /// Clear all cached answers (keeps ingest data).
    ClearCache,
}

#[derive(Debug, Parser)]
pub struct RefreshArgs {
    /// Skip git forensics (churn/coupling analysis).
    #[arg(long)]
    pub skip_git: bool,

    /// Skip complexity analysis.
    #[arg(long)]
    pub skip_complexity: bool,

    /// Skip skeleton extraction.
    #[arg(long)]
    pub skip_skeleton: bool,

    /// Use legacy Python scripts instead of native Rust implementation.
    #[arg(long)]
    pub legacy: bool,
}

#[derive(Debug, Parser)]
pub struct AskArgs {
    /// The question to ask the Architect.
    pub query: Vec<String>,

    /// Force a fresh query even if cached.
    #[arg(long, short)]
    pub force: bool,

    /// Skip confirmation prompt for API calls.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Debug, Parser)]
pub struct AuditArgs {
    /// The crate name to audit.
    pub crate_name: String,

    /// Force a fresh audit even if cached.
    #[arg(long, short)]
    pub force: bool,

    /// Skip confirmation prompt for API calls.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

impl ArchitectCli {
    pub async fn run(self) -> Result<()> {
        let vault = find_vault_root()?;

        match self.cmd {
            ArchitectCommand::Refresh(args) => run_refresh(&vault, args).await,
            ArchitectCommand::Ask(args) => run_ask(&vault, args).await,
            ArchitectCommand::Audit(args) => run_audit(&vault, args).await,
            ArchitectCommand::Status => run_status(&vault).await,
            ArchitectCommand::ClearCache => run_clear_cache(&vault).await,
        }
    }
}

/// Find the vault root (.codex/architect/) by walking up from cwd.
fn find_vault_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    loop {
        let vault = current.join(".codex").join("architect");
        if vault.exists() {
            return Ok(vault);
        }
        if !current.pop() {
            bail!(
                "No .codex/architect/ vault found. Run from a codex-rs project directory \
                 or create the vault with: mkdir -p .codex/architect/{{ingest,answers,audits}}"
            );
        }
    }
}

/// Convert a query string to a filesystem-safe slug.
fn slugify(query: &str) -> String {
    let normalized: String = query
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    // Collapse multiple dashes
    let mut result = String::new();
    let mut prev_dash = false;
    for c in normalized.chars() {
        if c == '-' {
            if !prev_dash {
                result.push(c);
            }
            prev_dash = true;
        } else {
            result.push(c);
            prev_dash = false;
        }
    }

    // Trim leading/trailing dashes and limit length
    let trimmed = result.trim_matches('-');
    if trimmed.len() > 80 {
        format!("{}-{:x}", &trimmed[..60], hash_string(query))
    } else {
        trimmed.to_string()
    }
}

fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Prompt user for confirmation. Returns true if user confirms.
fn confirm(prompt: &str) -> bool {
    print!("{} [Y/n] ", prompt);
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    let trimmed = input.trim().to_lowercase();
    trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
}

// ─────────────────────────────────────────────────────────────────────────────
// Command Implementations
// ─────────────────────────────────────────────────────────────────────────────

async fn run_refresh(vault: &Path, args: RefreshArgs) -> Result<()> {
    let ingest = vault.join("ingest");
    fs::create_dir_all(&ingest).await?;

    let project_root = vault.parent().and_then(|p| p.parent()).unwrap_or(vault);

    if args.legacy {
        // Legacy mode: use Python scripts
        println!("Refreshing forensic data (legacy mode) in {:?}", ingest);
        run_refresh_legacy(vault, &args, project_root).await?;
    } else {
        // Native mode: use Rust harvester modules
        println!("Refreshing forensic data (native) in {:?}", ingest);
        run_refresh_native(vault, &args, project_root).await?;
    }

    // Generate a freshness hash
    let hash = generate_repo_hash(project_root).await?;
    fs::write(ingest.join(".repo_hash"), hash).await?;

    println!("Refresh complete.");
    Ok(())
}

/// Native Rust implementation of refresh using codex_core::architect modules.
async fn run_refresh_native(vault: &Path, args: &RefreshArgs, project_root: &Path) -> Result<()> {
    let ingest = vault.join("ingest");
    let config = HarvesterConfig::new();

    if !args.skip_git {
        println!("  [1/3] Generating churn matrix (native)...");
        match architect::churn::analyze(project_root, &config) {
            Ok(report) => {
                let markdown = report.to_markdown();
                fs::write(ingest.join("churn_matrix.md"), markdown).await?;
                println!(
                    "    Analyzed {} files, {} commits, {} coupled pairs",
                    report.file_count,
                    report.commit_count,
                    report.coupling.len()
                );
            }
            Err(e) => {
                println!("    Error: {}. Try --legacy for Python fallback.", e);
            }
        }
    }

    if !args.skip_complexity {
        println!("  [2/3] Generating complexity map (native)...");
        match architect::complexity::analyze(project_root, &config) {
            Ok(report) => {
                let json = report.to_json()?;
                fs::write(ingest.join("complexity_map.json"), json).await?;
                println!(
                    "    Analyzed {} files (critical: {}, high: {}, medium: {}, low: {})",
                    report.file_count,
                    report.by_risk.critical,
                    report.by_risk.high,
                    report.by_risk.medium,
                    report.by_risk.low
                );
            }
            Err(e) => {
                println!("    Error: {}. Try --legacy for Python fallback.", e);
            }
        }
    }

    if !args.skip_skeleton {
        println!("  [3/3] Generating repo skeleton (native)...");
        match architect::skeleton::extract(project_root) {
            Ok(report) => {
                let xml = report.to_xml();
                fs::write(ingest.join("repo_skeleton.xml"), xml).await?;
                println!(
                    "    Extracted {} declarations from {} files",
                    report.declaration_count, report.file_count
                );
            }
            Err(e) => {
                println!("    Error: {}. Try --legacy for Python fallback.", e);
            }
        }
    }

    Ok(())
}

/// Legacy Python script implementation of refresh.
async fn run_refresh_legacy(vault: &Path, args: &RefreshArgs, project_root: &Path) -> Result<()> {
    let ingest = vault.join("ingest");
    let intel_snapshot = project_root.join("scripts").join("architect");

    if !args.skip_git {
        println!("  [1/3] Generating churn matrix (legacy)...");
        let script = intel_snapshot.join("generate_churn.py");
        if script.exists() {
            let status = Command::new("python3")
                .arg(&script)
                .current_dir(project_root)
                .status()
                .context("Failed to run churn analysis")?;
            if status.success() {
                let src = intel_snapshot.join("churn_matrix.md");
                if src.exists() {
                    fs::copy(&src, ingest.join("churn_matrix.md")).await?;
                }
            }
        } else {
            println!("    (skipped - script not found at {:?})", script);
        }
    }

    if !args.skip_complexity {
        println!("  [2/3] Generating complexity map (legacy)...");
        let script = intel_snapshot.join("generate_complexity.py");
        if script.exists() {
            let status = Command::new("python3")
                .arg(&script)
                .current_dir(project_root)
                .status()
                .context("Failed to run complexity analysis")?;
            if status.success() {
                let src = intel_snapshot.join("complexity_map.json");
                if src.exists() {
                    fs::copy(&src, ingest.join("complexity_map.json")).await?;
                }
            }
        } else {
            println!("    (skipped - script not found)");
        }
    }

    if !args.skip_skeleton {
        println!("  [3/3] Generating repo skeleton (legacy)...");
        let script = intel_snapshot.join("generate_skeleton.py");
        if script.exists() {
            let status = Command::new("python3")
                .arg(&script)
                .current_dir(project_root)
                .status()
                .context("Failed to run skeleton extraction")?;
            if status.success() {
                let src = intel_snapshot.join("repo_skeleton.xml");
                if src.exists() {
                    fs::copy(&src, ingest.join("repo_skeleton.xml")).await?;
                }
            }
        } else {
            println!("    (skipped - script not found)");
        }
    }

    Ok(())
}

async fn run_ask(vault: &Path, args: AskArgs) -> Result<()> {
    let query = args.query.join(" ");
    if query.is_empty() {
        bail!("No query provided. Usage: code architect ask <your question>");
    }

    let slug = slugify(&query);
    let cache_path = vault.join("answers").join(format!("{}.md", slug));

    // Check cache first (unless --force)
    if !args.force && cache_path.exists() {
        let content = fs::read_to_string(&cache_path).await?;
        println!("(cached: {})\n", cache_path.display());
        println!("{}", content);
        return Ok(());
    }

    // Cache miss - need to query NotebookLM
    if !args.yes {
        if !confirm("Answer not cached. This will use 1 NotebookLM query. Proceed?") {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("Querying Architect notebook...");

    // Call notebooklm-mcp CLI
    let nlm_cli = dirs::home_dir()
        .unwrap_or_default()
        .join("notebooklm-mcp/dist/cli/index.js");

    if !nlm_cli.exists() {
        bail!(
            "NotebookLM CLI not found at {:?}. Install with: \
             cd ~ && git clone https://github.com/anthropics/notebooklm-mcp && npm install && npm run build",
            nlm_cli
        );
    }

    let output = Command::new("node")
        .arg(&nlm_cli)
        .args(["ask", "-n", "codex-rs-architect", &query])
        .output()
        .context("Failed to run NotebookLM CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("NotebookLM query failed: {}", stderr);
    }

    let answer = String::from_utf8_lossy(&output.stdout);

    // Cache the answer
    fs::create_dir_all(cache_path.parent().unwrap()).await?;
    let cached_content = format!(
        "# {}\n\n_Cached: {}_\n\n---\n\n{}",
        query,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        answer
    );
    fs::write(&cache_path, &cached_content).await?;

    println!("{}", answer);
    println!("\n(answer cached to: {})", cache_path.display());

    Ok(())
}

async fn run_audit(vault: &Path, args: AuditArgs) -> Result<()> {
    let cache_path = vault
        .join("audits")
        .join(format!("{}.md", args.crate_name));

    // Check cache first
    if !args.force && cache_path.exists() {
        let content = fs::read_to_string(&cache_path).await?;
        println!("(cached: {})\n", cache_path.display());
        println!("{}", content);
        return Ok(());
    }

    // Cache miss
    if !args.yes {
        if !confirm(&format!(
            "Audit for '{}' not cached. This will use 1 NotebookLM query. Proceed?",
            args.crate_name
        )) {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("Auditing crate: {}", args.crate_name);

    let nlm_cli = dirs::home_dir()
        .unwrap_or_default()
        .join("notebooklm-mcp/dist/cli/index.js");

    if !nlm_cli.exists() {
        bail!("NotebookLM CLI not found at {:?}", nlm_cli);
    }

    // Use the crate-audit template
    let query = format!(
        "Audit the Rust crate '{}'. Focus on: 1) Recent CVEs or soundness issues, \
         2) Maintenance activity in 2024/2025, 3) Major breaking changes in recent versions, \
         4) Community alternatives.",
        args.crate_name
    );

    let output = Command::new("node")
        .arg(&nlm_cli)
        .args(["ask", "-n", "codex-rs-architect", &query])
        .output()
        .context("Failed to run NotebookLM CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("NotebookLM query failed: {}", stderr);
    }

    let answer = String::from_utf8_lossy(&output.stdout);

    // Cache the audit
    fs::create_dir_all(cache_path.parent().unwrap()).await?;
    let cached_content = format!(
        "# Crate Audit: {}\n\n_Audited: {}_\n\n---\n\n{}",
        args.crate_name,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        answer
    );
    fs::write(&cache_path, &cached_content).await?;

    println!("{}", answer);
    println!("\n(audit cached to: {})", cache_path.display());

    Ok(())
}

async fn run_status(vault: &Path) -> Result<()> {
    println!("Architect Vault Status");
    println!("======================\n");
    println!("Location: {}", vault.display());

    // Check ingest freshness
    let hash_file = vault.join("ingest").join(".repo_hash");
    if hash_file.exists() {
        let stored_hash = fs::read_to_string(&hash_file).await?;
        let project_root = vault.parent().and_then(|p| p.parent()).unwrap_or(vault);
        let current_hash = generate_repo_hash(project_root).await?;

        if stored_hash.trim() == current_hash.trim() {
            println!("Ingest data: FRESH");
        } else {
            println!("Ingest data: STALE (run 'code architect refresh')");
        }
    } else {
        println!("Ingest data: NOT INITIALIZED (run 'code architect refresh')");
    }

    // Count cached answers
    let answers_dir = vault.join("answers");
    let answer_count = if answers_dir.exists() {
        count_files(&answers_dir).await?
    } else {
        0
    };
    println!("Cached answers: {}", answer_count);

    // Count audits
    let audits_dir = vault.join("audits");
    let audit_count = if audits_dir.exists() {
        count_files(&audits_dir).await?
    } else {
        0
    };
    println!("Cached audits: {}", audit_count);

    // List recent answers
    if answer_count > 0 {
        println!("\nRecent answers:");
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&answers_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            entries.push(entry);
        }
        entries.sort_by_key(|e| std::cmp::Reverse(e.path()));
        for entry in entries.into_iter().take(5) {
            let name = entry.file_name();
            println!("  - {}", name.to_string_lossy());
        }
    }

    Ok(())
}

async fn run_clear_cache(vault: &Path) -> Result<()> {
    let answers_dir = vault.join("answers");
    let audits_dir = vault.join("audits");

    let answer_count = if answers_dir.exists() {
        count_files(&answers_dir).await?
    } else {
        0
    };
    let audit_count = if audits_dir.exists() {
        count_files(&audits_dir).await?
    } else {
        0
    };

    if answer_count == 0 && audit_count == 0 {
        println!("Cache is already empty.");
        return Ok(());
    }

    if !confirm(&format!(
        "Clear {} answers and {} audits?",
        answer_count, audit_count
    )) {
        println!("Aborted.");
        return Ok(());
    }

    if answers_dir.exists() {
        fs::remove_dir_all(&answers_dir).await?;
        fs::create_dir_all(&answers_dir).await?;
    }
    if audits_dir.exists() {
        fs::remove_dir_all(&audits_dir).await?;
        fs::create_dir_all(&audits_dir).await?;
    }

    println!("Cache cleared.");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn generate_repo_hash(project_root: &Path) -> Result<String> {
    // Simple hash based on git HEAD and dirty status
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let head = String::from_utf8_lossy(&o.stdout);
            Ok(head.trim().to_string())
        }
        _ => Ok("unknown".to_string()),
    }
}

async fn count_files(dir: &Path) -> Result<usize> {
    let mut count = 0;
    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("What is the architecture?"), "what-is-the-architecture");
        assert_eq!(slugify("How do I refactor the ChatWidget?"), "how-do-i-refactor-the-chatwidget");
        assert_eq!(slugify("  spaces  everywhere  "), "spaces-everywhere");
    }
}
