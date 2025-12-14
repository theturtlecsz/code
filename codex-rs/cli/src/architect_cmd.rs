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
use codex_core::architect::{
    self, HarvesterConfig,
    budget::BudgetTracker,
    chunker::{self, ChunkType, MAX_CHUNK_SIZE},
    mermaid,
    nlm_service::{Artifact, NlmService},
    research::ResearchClient,
};
// ChunkedPart is used internally by chunker, we use Artifact from nlm_service
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

    /// Show vault status and budget (with hourly breakdown).
    Status,

    /// Manage NotebookLM service daemon.
    Service {
        #[command(subcommand)]
        cmd: ServiceCommand,
    },

    /// Manage notebook sources.
    Sources {
        #[command(subcommand)]
        cmd: SourcesCommand,
    },

    /// Research operations (web search via NotebookLM).
    Research {
        #[command(subcommand)]
        cmd: ResearchCommand,
    },

    /// Clear all cached answers (keeps ingest data).
    ClearCache,
}

#[derive(Debug, clap::Subcommand)]
pub enum ServiceCommand {
    /// Start the NotebookLM service daemon.
    Start {
        /// Port to run the service on.
        #[arg(long, default_value = "3456")]
        port: u16,
        /// Run in foreground (don't daemonize).
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the running service.
    Stop,
    /// Check service status and health.
    Status,
}

#[derive(Debug, clap::Subcommand)]
pub enum SourcesCommand {
    /// List sources in the notebook.
    List,
    /// Upload artifacts (atomic swap with [ARCH] prefix).
    Upload {
        /// Skip confirmation prompt.
        #[arg(long, short = 'y')]
        force: bool,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum ResearchCommand {
    /// Quick parallel web search. Returns results immediately.
    Fast {
        /// The research query.
        query: Vec<String>,
        /// Wait for completion (default: true).
        #[arg(long, default_value = "true")]
        wait: bool,
    },
    /// Deep multi-step autonomous research. May take longer.
    Deep {
        /// The research query.
        query: Vec<String>,
        /// Wait for completion (default: true).
        #[arg(long, default_value = "true")]
        wait: bool,
        /// Allow editing the research plan before execution.
        #[arg(long)]
        edit_plan: bool,
    },
    /// Check the status of running research.
    Status,
    /// Get the results of completed research.
    Results {
        /// Output format: summary, full, or sources_only.
        #[arg(long, default_value = "summary")]
        format: String,
    },
    /// Import research results as notebook sources.
    Import,
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

    /// Generate Mermaid call graph diagram.
    #[arg(long)]
    pub graph: bool,

    /// Generate Mermaid module dependency diagram.
    #[arg(long)]
    pub mermaid: bool,

    /// Focus call graph on a specific function.
    #[arg(long, value_name = "FUNCTION")]
    pub focus: Option<String>,

    /// Depth for focused call graph (default: 2).
    #[arg(long, default_value = "2")]
    pub depth: usize,

    /// Use legacy Python scripts instead of native Rust implementation.
    #[arg(long)]
    pub legacy: bool,
}

#[derive(Debug, Parser)]
pub struct AskArgs {
    /// The question to ask the Architect.
    pub query: Vec<String>,

    /// Force a fresh query even if cached (bypass cache).
    #[arg(long, short, visible_alias = "no-cache")]
    pub force: bool,

    /// Skip confirmation prompt for API calls.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Debug, Parser)]
pub struct AuditArgs {
    /// The crate name to audit.
    pub crate_name: String,

    /// Force a fresh audit even if cached (bypass cache).
    #[arg(long, short, visible_alias = "no-cache")]
    pub force: bool,

    /// Skip confirmation prompt for API calls.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

/// Default notebook name for architect operations.
const DEFAULT_NOTEBOOK: &str = "codex-rs-architect";

impl ArchitectCli {
    pub async fn run(self) -> Result<()> {
        let vault = find_vault_root()?;

        match self.cmd {
            ArchitectCommand::Refresh(args) => run_refresh(&vault, args).await,
            ArchitectCommand::Ask(args) => run_ask(&vault, args).await,
            ArchitectCommand::Audit(args) => run_audit(&vault, args).await,
            ArchitectCommand::Status => run_status(&vault).await,
            ArchitectCommand::Service { cmd } => run_service(cmd).await,
            ArchitectCommand::Sources { cmd } => run_sources(&vault, cmd).await,
            ArchitectCommand::Research { cmd } => run_research(cmd).await,
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

    // Generate Mermaid call graph if requested
    if args.graph {
        println!("  [+] Generating call graph (Mermaid)...");
        match mermaid::extract_call_graph(project_root) {
            Ok(graph) => {
                let mermaid_content = if let Some(ref focus) = args.focus {
                    println!("    Focused on: {} (depth {})", focus, args.depth);
                    graph.to_mermaid_focused(focus, args.depth)
                } else {
                    graph.to_mermaid()
                };

                let graph_path = ingest.join("call_graph.mmd");
                fs::write(&graph_path, &mermaid_content).await?;
                println!(
                    "    Call graph: {} functions, {} edges → {}",
                    graph.functions.len(),
                    graph.calls.len(),
                    graph_path.display()
                );
            }
            Err(e) => {
                println!("    Error generating call graph: {}", e);
            }
        }
    }

    // Generate Mermaid module dependencies if requested
    if args.mermaid {
        println!("  [+] Generating module dependencies (Mermaid)...");
        match mermaid::extract_module_deps(project_root) {
            Ok(deps) => {
                let mermaid_content = deps.to_mermaid();
                let deps_path = ingest.join("module_deps.mmd");
                fs::write(&deps_path, &mermaid_content).await?;
                println!(
                    "    Module deps: {} modules, {} imports → {}",
                    deps.modules.len(),
                    deps.imports.len(),
                    deps_path.display()
                );
            }
            Err(e) => {
                println!("    Error generating module deps: {}", e);
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

    // Load budget tracker for status display
    let budget = BudgetTracker::load(vault)?;

    // Check if budget is exhausted
    if budget.is_exhausted() {
        bail!(
            "Daily query limit ({}) reached. Resets in {}.\n\
             Use cached answers or wait for reset.",
            budget.limit(),
            budget.time_until_reset()
        );
    }

    // Show budget warning if past threshold
    if budget.needs_confirmation() && !args.yes {
        println!("WARNING: {} - past 80% threshold", budget.format_status());
    }

    // Cache miss - need to query NotebookLM
    if !args.yes {
        let msg = format!(
            "Answer not cached. This will use 1 query. ({})",
            budget.format_status()
        );
        if !confirm(&msg) {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Try HTTP service first, fall back to CLI
    let mut service = NlmService::new(vault, DEFAULT_NOTEBOOK)?;
    let answer = if service.is_running().await {
        println!("Querying via HTTP service...");
        service.ask(&query).await?
    } else {
        println!("Service not running, using CLI (slower)...");
        println!("Tip: Start service with 'code architect service start' for faster queries.");
        ask_via_cli(vault, &query).await?
    };

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

/// Ask a question using the CLI (fallback when service isn't running).
/// Also records the query in the budget tracker.
async fn ask_via_cli(vault: &Path, query: &str) -> Result<String> {
    let nlm_cli = dirs::home_dir()
        .unwrap_or_default()
        .join("notebooklm-mcp/dist/cli/index.js");

    if !nlm_cli.exists() {
        bail!(
            "NotebookLM CLI not found at {:?}. Install with:\n\
             cd ~ && git clone https://github.com/thetu/notebooklm-mcp && npm install && npm run build",
            nlm_cli
        );
    }

    let output = Command::new("node")
        .arg(&nlm_cli)
        .args(["ask", "-n", DEFAULT_NOTEBOOK, query])
        .output()
        .context("Failed to run NotebookLM CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("NotebookLM query failed: {}", stderr);
    }

    // Record the query in budget tracker
    let mut budget = BudgetTracker::load(vault)?;
    budget.record_query()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn run_audit(vault: &Path, args: AuditArgs) -> Result<()> {
    let cache_path = vault.join("audits").join(format!("{}.md", args.crate_name));

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

    // Budget tracking
    println!("\n--- Budget ---");
    match BudgetTracker::load(vault) {
        Ok(tracker) => {
            println!("{}", tracker.format_status());
            if tracker.is_exhausted() {
                println!("LIMIT REACHED - Resets in {}", tracker.time_until_reset());
            } else if tracker.needs_confirmation() {
                println!("WARNING: Past 80% threshold");
            }
            println!("\n{}", tracker.hourly_breakdown());
            if !tracker.history_summary().contains("No historical") {
                println!("\n{}", tracker.history_summary());
            }
        }
        Err(e) => {
            println!("Budget tracking unavailable: {}", e);
        }
    }

    // Check service status
    println!("\n--- Service ---");
    let service = NlmService::new(vault, DEFAULT_NOTEBOOK);
    match service {
        Ok(svc) => {
            if svc.is_running().await {
                println!("NotebookLM service: RUNNING");
                if let Ok(health) = svc.health().await {
                    if let Some(q) = health.queue {
                        println!(
                            "  Queue: {} pending, {} processing",
                            q.pending, q.processing
                        );
                    }
                    if let Some(s) = health.sessions {
                        println!("  Sessions: {}/{}", s.active, s.max);
                    }
                }
            } else {
                println!("NotebookLM service: NOT RUNNING");
                println!("  Start with: code architect service start");
            }
        }
        Err(_) => {
            println!("NotebookLM service: UNAVAILABLE");
        }
    }

    // Check ingest freshness
    println!("\n--- Ingest ---");
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
    println!("\n--- Cache ---");
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
// Service Commands
// ─────────────────────────────────────────────────────────────────────────────

async fn run_service(cmd: ServiceCommand) -> Result<()> {
    match cmd {
        ServiceCommand::Start { port, foreground } => {
            println!("Starting NotebookLM service on port {}...", port);
            NlmService::start_service(port, foreground)?;

            if !foreground {
                // Wait a bit and check if it started
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let test_url = format!("http://127.0.0.1:{}/health", port);
                let client = reqwest::Client::new();
                match client
                    .get(&test_url)
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        println!("Service started successfully.");
                    }
                    _ => {
                        println!(
                            "Service may be starting... check with: code architect service status"
                        );
                    }
                }
            }
            Ok(())
        }
        ServiceCommand::Stop => {
            println!("Stopping NotebookLM service...");
            NlmService::stop_service()?;
            println!("Service stopped.");
            Ok(())
        }
        ServiceCommand::Status => {
            let status = NlmService::service_status()?;
            println!("{}", status);
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Research Commands
// ─────────────────────────────────────────────────────────────────────────────

async fn run_research(cmd: ResearchCommand) -> Result<()> {
    let client = ResearchClient::with_port(3456, DEFAULT_NOTEBOOK)?;

    if !client.is_running().await {
        bail!(
            "NotebookLM service is not running.\n\
             Start it with: code architect service start"
        );
    }

    match cmd {
        ResearchCommand::Fast { query, wait } => {
            let query_str = query.join(" ");
            if query_str.is_empty() {
                bail!("No query provided. Usage: code architect research fast <query>");
            }

            println!("Starting fast research: \"{}\"", query_str);
            if !wait {
                println!("(running in background)");
            }

            let result = client.fast(&query_str, wait).await?;

            if let Some(status) = &result.status {
                println!("Status: {}", status);
            }
            if let Some(progress) = result.progress {
                println!("Progress: {}%", progress);
            }

            if let Some(results) = &result.results {
                if let Some(summary) = &results.summary {
                    println!("\n--- Results ---\n{}", summary);
                }
                if let Some(sources) = &results.sources {
                    println!("\n--- Sources ({}) ---", sources.len());
                    for source in sources.iter().take(10) {
                        if let Some(title) = &source.title {
                            println!("  • {}", title);
                            if let Some(url) = &source.url {
                                println!("    {}", url);
                            }
                        }
                    }
                    if sources.len() > 10 {
                        println!("  ... and {} more", sources.len() - 10);
                    }
                }
            }

            Ok(())
        }
        ResearchCommand::Deep {
            query,
            wait,
            edit_plan,
        } => {
            let query_str = query.join(" ");
            if query_str.is_empty() {
                bail!("No query provided. Usage: code architect research deep <query>");
            }

            println!("Starting deep research: \"{}\"", query_str);
            if edit_plan {
                println!("(edit_plan mode enabled)");
            }
            if !wait {
                println!("(running in background)");
            }

            let result = client.deep(&query_str, wait, edit_plan).await?;

            if let Some(status) = &result.status {
                println!("Status: {}", status);
            }
            if let Some(progress) = result.progress {
                println!("Progress: {}%", progress);
            }

            if let Some(results) = &result.results {
                if let Some(summary) = &results.summary {
                    println!("\n--- Results ---\n{}", summary);
                }
                if let Some(count) = results.source_count {
                    println!("\nSources found: {}", count);
                }
            }

            println!("\nTip: Use 'code architect research results' for full output.");
            println!("Tip: Use 'code architect research import' to add as notebook sources.");

            Ok(())
        }
        ResearchCommand::Status => {
            println!("Checking research status...");

            let status = client.status().await?;
            println!("Status: {}", status.status);

            if let Some(query) = &status.query {
                println!("Query: \"{}\"", query);
            }
            if let Some(progress) = status.progress {
                println!("Progress: {}%", progress);
            }
            if let Some(started) = &status.started_at {
                println!("Started: {}", started);
            }
            if let Some(completed) = &status.completed_at {
                println!("Completed: {}", completed);
            }
            if let Some(error) = &status.error {
                println!("Error: {}", error);
            }

            Ok(())
        }
        ResearchCommand::Results { format } => {
            println!("Fetching research results (format: {})...", format);

            let result = client.results(&format).await?;

            if let Some(results) = &result.results {
                if let Some(summary) = &results.summary {
                    println!("\n{}", summary);
                }
                if let Some(sources) = &results.sources {
                    println!("\n--- Sources ({}) ---", sources.len());
                    for source in sources {
                        if let Some(title) = &source.title {
                            println!("\n• {}", title);
                        }
                        if let Some(url) = &source.url {
                            println!("  URL: {}", url);
                        }
                        if let Some(snippet) = &source.snippet {
                            println!("  {}", snippet);
                        }
                    }
                }
            } else {
                println!("No results available. Run research first.");
            }

            Ok(())
        }
        ResearchCommand::Import => {
            println!("Importing research results as notebook sources...");

            let result = client.import().await?;

            if let Some(count) = result.imported {
                println!("Imported {} sources.", count);
            }
            if let Some(sources) = &result.sources {
                for name in sources {
                    println!("  • {}", name);
                }
            }

            println!("\nTip: Use 'code architect sources list' to see all sources.");

            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sources Commands
// ─────────────────────────────────────────────────────────────────────────────

async fn run_sources(vault: &Path, cmd: SourcesCommand) -> Result<()> {
    let service = NlmService::new(vault, DEFAULT_NOTEBOOK)?;

    if !service.is_running().await {
        bail!(
            "NotebookLM service is not running.\n\
             Start it with: code architect service start"
        );
    }

    match cmd {
        SourcesCommand::List => {
            println!("Sources in notebook '{}':", DEFAULT_NOTEBOOK);
            println!("─────────────────────────────────");

            let sources = service.list_sources().await?;
            if sources.is_empty() {
                println!("  (no sources)");
            } else {
                for source in &sources {
                    let managed = if source.title.starts_with("[ARCH]") {
                        " (managed)"
                    } else {
                        ""
                    };
                    println!("  [{}] {}{}", source.index, source.title, managed);
                }
                println!("\nTotal: {} sources", sources.len());
                let managed_count = sources
                    .iter()
                    .filter(|s| s.title.starts_with("[ARCH]"))
                    .count();
                if managed_count > 0 {
                    println!("Managed ([ARCH]): {}", managed_count);
                }
            }
            Ok(())
        }
        SourcesCommand::Upload { force } => {
            // Load artifacts from ingest directory
            let ingest_dir = vault.join("ingest");
            if !ingest_dir.exists() {
                bail!("No ingest data found. Run 'code architect refresh' first.");
            }

            let mut artifacts = Vec::new();

            // Load churn matrix (usually small, no chunking needed)
            let churn_path = ingest_dir.join("churn_matrix.md");
            if churn_path.exists() {
                let content = fs::read_to_string(&churn_path).await?;
                if content.len() <= MAX_CHUNK_SIZE {
                    artifacts.push(Artifact::new("Churn Matrix", content));
                } else {
                    let chunks = chunker::chunk_content("Churn Matrix", &content, ChunkType::Lines);
                    println!("  Note: Churn matrix chunked into {} parts", chunks.len());
                    for chunk in chunks {
                        artifacts.push(Artifact::new(&chunk.name, chunk.content));
                    }
                }
            }

            // Load complexity map - filter to critical/high only if too large
            let complexity_path = ingest_dir.join("complexity_map.json");
            if complexity_path.exists() {
                let full_content = fs::read_to_string(&complexity_path).await?;
                if full_content.len() > MAX_CHUNK_SIZE {
                    // Filter to critical/high complexity files only
                    let filtered = filter_complexity_map(&full_content)?;
                    println!(
                        "  Note: Complexity map filtered to critical/high ({} bytes -> {} bytes)",
                        full_content.len(),
                        filtered.len()
                    );
                    if filtered.len() <= MAX_CHUNK_SIZE {
                        artifacts.push(Artifact::new("Complexity Map (Critical/High)", filtered));
                    } else {
                        println!("  Warning: Filtered complexity map still too large, skipping");
                    }
                } else {
                    artifacts.push(Artifact::new("Complexity Map", full_content));
                }
            }

            // Load repo skeleton - chunk if too large (XML chunking)
            let skeleton_path = ingest_dir.join("repo_skeleton.xml");
            if skeleton_path.exists() {
                let content = fs::read_to_string(&skeleton_path).await?;
                if content.len() <= MAX_CHUNK_SIZE {
                    artifacts.push(Artifact::new("Repo Skeleton", content));
                } else {
                    let chunks = chunker::chunk_content("Repo Skeleton", &content, ChunkType::Xml);
                    println!(
                        "  Note: Repo skeleton chunked into {} parts ({} bytes)",
                        chunks.len(),
                        content.len()
                    );
                    for chunk in chunks {
                        artifacts.push(Artifact::new(&chunk.name, chunk.content));
                    }
                }
            }

            // Load call graph - chunk if too large (Mermaid chunking)
            let graph_path = ingest_dir.join("call_graph.mmd");
            if graph_path.exists() {
                let content = fs::read_to_string(&graph_path).await?;
                if content.len() <= MAX_CHUNK_SIZE {
                    artifacts.push(Artifact::new("Call Graph", content));
                } else {
                    let chunks = chunker::chunk_content("Call Graph", &content, ChunkType::Mermaid);
                    println!(
                        "  Note: Call graph chunked into {} parts ({} bytes)",
                        chunks.len(),
                        content.len()
                    );
                    for chunk in chunks {
                        artifacts.push(Artifact::new(&chunk.name, chunk.content));
                    }
                }
            }

            // Load module deps - chunk if too large (Mermaid chunking)
            let deps_path = ingest_dir.join("module_deps.mmd");
            if deps_path.exists() {
                let content = fs::read_to_string(&deps_path).await?;
                if content.len() <= MAX_CHUNK_SIZE {
                    artifacts.push(Artifact::new("Module Dependencies", content));
                } else {
                    let chunks =
                        chunker::chunk_content("Module Dependencies", &content, ChunkType::Mermaid);
                    println!(
                        "  Note: Module deps chunked into {} parts ({} bytes)",
                        chunks.len(),
                        content.len()
                    );
                    for chunk in chunks {
                        artifacts.push(Artifact::new(&chunk.name, chunk.content));
                    }
                }
            }

            if artifacts.is_empty() {
                bail!("No artifacts found to upload.");
            }

            println!("Artifacts to upload:");
            for artifact in &artifacts {
                println!("  - {} ({} bytes)", artifact.title, artifact.content.len());
            }

            if !force {
                println!("\nThis will:");
                println!("  1. Delete all existing [ARCH] sources");
                println!("  2. Upload {} fresh artifacts", artifacts.len());
                if !confirm("Proceed with atomic swap?") {
                    println!("Aborted.");
                    return Ok(());
                }
            }

            println!("\nPerforming atomic swap...");
            let result = service.refresh_context(&artifacts).await?;

            println!("Done!");
            println!("  Deleted: {} old [ARCH] sources", result.deleted);
            println!("  Uploaded: {} new artifacts", result.uploaded);
            println!(
                "  Total sources now: {}",
                result.total_sources - result.deleted + result.uploaded
            );

            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Filter complexity map to only critical/high risk entries.
fn filter_complexity_map(json_content: &str) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(json_content).context("Failed to parse complexity map JSON")?;

    // The complexity map has structure:
    // { "files": [...], "by_risk": { "critical": N, "high": N, ... }, ... }
    if let Some(files) = value.get("files").and_then(|f| f.as_array()) {
        let filtered: Vec<&serde_json::Value> = files
            .iter()
            .filter(|f| {
                f.get("risk")
                    .and_then(|r| r.as_str())
                    .map(|r| r == "critical" || r == "high")
                    .unwrap_or(false)
            })
            .collect();

        // Build filtered output
        let mut output = serde_json::Map::new();
        output.insert(
            "files".to_string(),
            serde_json::Value::Array(filtered.into_iter().cloned().collect()),
        );
        output.insert(
            "note".to_string(),
            serde_json::Value::String("Filtered to critical/high risk files only".to_string()),
        );

        // Preserve summary if present
        if let Some(by_risk) = value.get("by_risk") {
            output.insert("by_risk".to_string(), by_risk.clone());
        }

        Ok(serde_json::to_string_pretty(&output)?)
    } else {
        // Unknown structure, return as-is
        Ok(json_content.to_string())
    }
}

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
        assert_eq!(
            slugify("What is the architecture?"),
            "what-is-the-architecture"
        );
        assert_eq!(
            slugify("How do I refactor the ChatWidget?"),
            "how-do-i-refactor-the-chatwidget"
        );
        assert_eq!(slugify("  spaces  everywhere  "), "spaces-everywhere");
    }
}
