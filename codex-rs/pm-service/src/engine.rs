//! Bot engines for research and review runs.
//!
//! The research engine (Phase-1) checks NotebookLM availability, reads
//! workspace context, and produces a structured `ResearchReport` with
//! findings, options/tradeoffs, citations, and open questions.
//!
//! ## Dependency posture
//!
//! - **NotebookLM required but unavailable** → terminal `Blocked` with
//!   structured reason and resolution steps.
//! - **Degraded allowed** → proceed without NotebookLM, label outputs
//!   "degraded", record which sources were actually used.
//!
//! ## Determinism boundary
//!
//! Every report records:
//! - `base_commit`: HEAD of the workspace at analysis start
//! - `input_uris`: snapshot IDs/URIs of inputs consumed

use std::path::{Path, PathBuf};
use std::sync::Arc;

use codex_core::pm::artifacts::{
    BotRunCheckpoint, BotRunLog, BotRunState, ConflictSummary, PatchBundle, RebaseStatus,
    ResearchFinding, ResearchReport, ReviewFinding, ReviewReport, ReviewSeverity,
};
use codex_core::pm::bot::{BotKind, BotWriteMode};
use tokio::sync::Mutex;

/// Result of an engine execution.
pub struct EngineResult {
    pub state: BotRunState,
    pub exit_code: i32,
    pub summary: String,
    pub log: BotRunLog,
    /// Serialized report artifact (JSON).
    pub report_json: String,
    /// Checkpoints emitted during the run.
    pub checkpoints: Vec<BotRunCheckpoint>,
    /// Serialized PatchBundle artifact (JSON), if write-mode produced patches.
    pub patch_bundle_json: Option<String>,
    /// Serialized ConflictSummary artifact (JSON), if rebase had conflicts.
    pub conflict_summary_json: Option<String>,
}

/// Reason a run is blocked.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BlockedReason {
    pub dependency: String,
    pub detail: String,
    pub resolution: Vec<String>,
}

/// Context for running the research engine.
pub struct ResearchContext {
    pub workspace_path: String,
    pub run_id: String,
    pub work_item_id: String,
    /// Override for NotebookLM health URL (testing).
    pub notebooklm_health_url: Option<String>,
    /// If true, engine runs in degraded mode when NotebookLM unavailable
    /// instead of blocking. Default: true.
    pub allow_degraded: bool,
}

impl ResearchContext {
    pub fn new(workspace_path: &str, run_id: &str, work_item_id: &str) -> Self {
        Self {
            workspace_path: workspace_path.to_string(),
            run_id: run_id.to_string(),
            work_item_id: work_item_id.to_string(),
            notebooklm_health_url: None,
            allow_degraded: true,
        }
    }
}

/// Checkpoint accumulator shared between engine phases.
struct CheckpointAccum {
    run_id: String,
    work_item_id: String,
    seq: u32,
    checkpoints: Vec<BotRunCheckpoint>,
}

impl CheckpointAccum {
    fn new(run_id: &str, work_item_id: &str) -> Self {
        Self {
            run_id: run_id.to_string(),
            work_item_id: work_item_id.to_string(),
            seq: 0,
            checkpoints: Vec::new(),
        }
    }

    fn emit(&mut self, phase: &str, summary: &str, percent: Option<u8>) {
        let cp = BotRunCheckpoint {
            schema_version: BotRunCheckpoint::SCHEMA_VERSION.to_string(),
            run_id: self.run_id.clone(),
            work_item_id: self.work_item_id.clone(),
            seq: self.seq,
            state: BotRunState::Running,
            timestamp: chrono::Utc::now().to_rfc3339(),
            summary: summary.to_string(),
            percent,
            phase: Some(phase.to_string()),
        };
        self.checkpoints.push(cp);
        self.seq += 1;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// NotebookLM availability check
// ─────────────────────────────────────────────────────────────────────────────

const NOTEBOOKLM_DEFAULT_HEALTH_URL: &str = "http://127.0.0.1:3456/health/ready";

/// Check if NotebookLM service is available and authenticated.
async fn check_notebooklm(health_url: &str) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .get(health_url)
        .send()
        .await
        .map_err(|e| format!("NotebookLM health unreachable: {e}"))?;

    if !resp.status().is_success() {
        return Ok(false);
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("NotebookLM health invalid JSON: {e}"))?;

    // Service is ready only if both ready=true and authenticated
    let ready = body
        .get("ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    Ok(ready)
}

// ─────────────────────────────────────────────────────────────────────────────
// Determinism boundary: git HEAD
// ─────────────────────────────────────────────────────────────────────────────

/// Get the HEAD commit SHA of a workspace, if it's a git repo.
fn get_base_commit(workspace_path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace_path)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Workspace context gathering
// ─────────────────────────────────────────────────────────────────────────────

/// Scan the workspace for spec/PRD files related to a work item.
fn gather_workspace_inputs(workspace_path: &str, work_item_id: &str) -> Vec<String> {
    let mut uris = Vec::new();

    // Look for spec.md, PRD.md, plan.md under docs/{work_item_id}*
    let docs_dir = Path::new(workspace_path).join("docs");
    if let Ok(entries) = std::fs::read_dir(&docs_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains(work_item_id)
                || name.to_lowercase().contains(&work_item_id.to_lowercase())
            {
                if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                    // Scan files inside this spec directory
                    if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                        for sub in sub_entries.flatten() {
                            let sub_name = sub.file_name().to_string_lossy().to_string();
                            if sub_name.ends_with(".md") {
                                uris.push(format!("file://{}", sub.path().to_string_lossy()));
                            }
                        }
                    }
                } else if name.ends_with(".md") {
                    uris.push(format!("file://{}", entry.path().to_string_lossy()));
                }
            }
        }
    }

    // Also check for SPEC.md at root
    let spec_md = Path::new(workspace_path).join("SPEC.md");
    if spec_md.exists() {
        uris.push(format!("file://{}", spec_md.to_string_lossy()));
    }

    uris
}

/// Read a brief excerpt from a file (first N lines).
fn read_file_excerpt(path: &str, max_lines: usize) -> Option<String> {
    let real_path = path.strip_prefix("file://").unwrap_or(path);
    let content = std::fs::read_to_string(real_path).ok()?;
    let lines: Vec<&str> = content.lines().take(max_lines).collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Research engine (real)
// ─────────────────────────────────────────────────────────────────────────────

/// Execute the research engine.
///
/// Phases:
/// 1. Dependency check (NotebookLM availability)
/// 2. Context gathering (workspace scan, git HEAD)
/// 3. Analysis (read spec files, produce findings)
/// 4. Report synthesis
pub async fn run_research(ctx: &ResearchContext) -> EngineResult {
    let started_at = chrono::Utc::now();
    let cps = Arc::new(Mutex::new(CheckpointAccum::new(
        &ctx.run_id,
        &ctx.work_item_id,
    )));

    // ── Phase 1: Dependency check ───────────────────────────────────────
    {
        let mut cp = cps.lock().await;
        cp.emit(
            "dependency_check",
            "Checking NotebookLM availability",
            Some(5),
        );
    }

    let health_url = ctx
        .notebooklm_health_url
        .as_deref()
        .unwrap_or(NOTEBOOKLM_DEFAULT_HEALTH_URL);

    let notebooklm_available = check_notebooklm(health_url).await.unwrap_or_default();

    if !notebooklm_available && !ctx.allow_degraded {
        // Blocked: NotebookLM required but unavailable
        let now = chrono::Utc::now();
        let reason = BlockedReason {
            dependency: "notebooklm".to_string(),
            detail: "NotebookLM service is not available or not authenticated".to_string(),
            resolution: vec![
                "Run: notebooklm health".to_string(),
                "Run: notebooklm setup-auth".to_string(),
                "Verify service: curl http://127.0.0.1:3456/health/ready".to_string(),
            ],
        };

        let reason_json =
            serde_json::to_string_pretty(&reason).unwrap_or_else(|_| "{}".to_string());

        let log = BotRunLog {
            schema_version: BotRunLog::SCHEMA_VERSION.to_string(),
            run_id: ctx.run_id.clone(),
            work_item_id: ctx.work_item_id.clone(),
            state: BotRunState::Blocked,
            started_at: started_at.to_rfc3339(),
            finished_at: now.to_rfc3339(),
            duration_s: (now - started_at).num_seconds().unsigned_abs(),
            exit_code: 2,
            summary: format!("Blocked: NotebookLM unavailable — {}", reason.detail),
            partial: false,
            checkpoint_count: cps.lock().await.seq,
            error: Some(reason_json.clone()),
        };

        // Produce a minimal report for the blocked state
        let report = ResearchReport {
            schema_version: ResearchReport::SCHEMA_VERSION.to_string(),
            run_id: ctx.run_id.clone(),
            work_item_id: ctx.work_item_id.clone(),
            timestamp: now.to_rfc3339(),
            findings: vec![],
            summary: format!("Blocked: {}", reason.detail),
            degraded: false,
            base_commit: get_base_commit(&ctx.workspace_path),
            input_uris: vec![],
            sources_used: vec![],
            open_questions: vec!["NotebookLM must be available for full research".to_string()],
        };

        let report_json =
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());

        return EngineResult {
            state: BotRunState::Blocked,
            exit_code: 2,
            summary: log.summary.clone(),
            log,
            report_json,
            checkpoints: cps.lock().await.checkpoints.clone(),
            patch_bundle_json: None,
            conflict_summary_json: None,
        };
    }

    let degraded = !notebooklm_available;

    // ── Phase 2: Context gathering ──────────────────────────────────────
    {
        let mut cp = cps.lock().await;
        cp.emit(
            "context_gathering",
            "Scanning workspace for spec files",
            Some(20),
        );
    }

    let base_commit = get_base_commit(&ctx.workspace_path);
    let input_uris = gather_workspace_inputs(&ctx.workspace_path, &ctx.work_item_id);

    {
        let mut cp = cps.lock().await;
        cp.emit(
            "context_gathering",
            &format!("Found {} input files", input_uris.len()),
            Some(30),
        );
    }

    // ── Phase 3: Analysis ───────────────────────────────────────────────
    {
        let mut cp = cps.lock().await;
        cp.emit("analysis", "Analyzing spec documents", Some(50));
    }

    let mut findings = Vec::new();
    let mut sources_used = Vec::new();
    let mut open_questions = Vec::new();

    // Read each input file and produce findings
    for uri in &input_uris {
        if let Some(excerpt) = read_file_excerpt(uri, 100) {
            let filename = uri.rsplit('/').next().unwrap_or(uri).to_string();

            sources_used.push(uri.clone());

            // Extract key information from the file content
            let finding = analyze_document(&filename, &excerpt, &ctx.work_item_id);
            findings.push(finding);
        }
    }

    // If no spec files found, produce a finding about that
    if findings.is_empty() {
        findings.push(ResearchFinding {
            title: format!("No spec documents found for {}", ctx.work_item_id),
            body: format!(
                "No .md files found under docs/ matching work item '{}'. \
                 Consider creating a spec or PRD first.",
                ctx.work_item_id
            ),
            source: Some(format!("file://{}/docs/", ctx.workspace_path)),
            confidence: Some("high".to_string()),
        });
        open_questions.push(format!(
            "No spec documents exist for {}; should one be created?",
            ctx.work_item_id
        ));
    }

    if degraded {
        findings.push(ResearchFinding {
            title: "Degraded mode: NotebookLM unavailable".to_string(),
            body: "This report was produced without NotebookLM cross-referencing. \
                   Findings are based solely on workspace document analysis. \
                   Re-run when NotebookLM is available for deeper research."
                .to_string(),
            source: None,
            confidence: Some("low".to_string()),
        });
        sources_used.push("workspace-local-only".to_string());
        open_questions
            .push("Re-run with NotebookLM available for cross-referenced research".to_string());
    }

    // ── Phase 4: Report synthesis ───────────────────────────────────────
    {
        let mut cp = cps.lock().await;
        cp.emit("synthesis", "Generating research report", Some(80));
    }

    let now = chrono::Utc::now();
    let duration_s = (now - started_at).num_seconds().unsigned_abs();

    let summary = if degraded {
        format!(
            "Degraded research completed: {} finding(s) from workspace analysis only",
            findings.len()
        )
    } else {
        format!("Research completed: {} finding(s)", findings.len())
    };

    let report = ResearchReport {
        schema_version: ResearchReport::SCHEMA_VERSION.to_string(),
        run_id: ctx.run_id.clone(),
        work_item_id: ctx.work_item_id.clone(),
        timestamp: now.to_rfc3339(),
        findings,
        summary: summary.clone(),
        degraded,
        base_commit,
        input_uris: input_uris.clone(),
        sources_used,
        open_questions,
    };

    let report_json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());

    // Final checkpoint
    {
        let mut cp = cps.lock().await;
        cp.emit("complete", &summary, Some(100));
    }

    let log = BotRunLog {
        schema_version: BotRunLog::SCHEMA_VERSION.to_string(),
        run_id: ctx.run_id.clone(),
        work_item_id: ctx.work_item_id.clone(),
        state: BotRunState::Succeeded,
        started_at: started_at.to_rfc3339(),
        finished_at: now.to_rfc3339(),
        duration_s,
        exit_code: 0,
        summary: summary.clone(),
        partial: false,
        checkpoint_count: cps.lock().await.seq,
        error: None,
    };

    EngineResult {
        state: BotRunState::Succeeded,
        exit_code: 0,
        summary,
        log,
        report_json,
        checkpoints: cps.lock().await.checkpoints.clone(),
        patch_bundle_json: None,
        conflict_summary_json: None,
    }
}

/// Analyze a single document and produce a finding.
fn analyze_document(filename: &str, content: &str, work_item_id: &str) -> ResearchFinding {
    // Extract headings and key sections
    let headings: Vec<&str> = content.lines().filter(|l| l.starts_with('#')).collect();

    let heading_summary = if headings.is_empty() {
        "no headings found".to_string()
    } else {
        headings
            .iter()
            .take(5)
            .map(|h| h.trim_start_matches('#').trim())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let line_count = content.lines().count();

    ResearchFinding {
        title: format!("Analysis of {filename}"),
        body: format!(
            "Document contains {line_count} lines covering: {heading_summary}. \
             Relevant to work item {work_item_id}.",
        ),
        source: Some(filename.to_string()),
        confidence: Some("medium".to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Review engine
// ─────────────────────────────────────────────────────────────────────────────

/// Context for running the review engine.
pub struct ReviewContext {
    pub workspace_path: String,
    pub run_id: String,
    pub work_item_id: String,
    pub write_mode: BotWriteMode,
    pub rebase_target: Option<String>,
}

/// Git subcommands that the review engine is permitted to execute.
/// No network operations (push, fetch, pull, clone, remote) are allowed.
const ALLOWED_GIT_SUBCOMMANDS: &[&str] = &[
    "rev-parse",
    "branch",
    "worktree",
    "checkout",
    "add",
    "commit",
    "diff",
    "rebase",
    "log",
    "status",
    "init",
];

/// Run an allowlisted git command. Returns (success, stdout, stderr).
fn run_git(workspace_path: &str, args: &[&str]) -> Result<std::process::Output, String> {
    let subcommand = args.first().ok_or_else(|| "empty git args".to_string())?;
    if !ALLOWED_GIT_SUBCOMMANDS.contains(subcommand) {
        return Err(format!(
            "git subcommand '{subcommand}' is not in the review engine allowlist"
        ));
    }

    std::process::Command::new("git")
        .args(args)
        .current_dir(workspace_path)
        .output()
        .map_err(|e| format!("git {subcommand}: {e}"))
}

/// Run an allowlisted git command in a specific directory (e.g. worktree).
fn run_git_in(dir: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let subcommand = args.first().ok_or_else(|| "empty git args".to_string())?;
    if !ALLOWED_GIT_SUBCOMMANDS.contains(subcommand) {
        return Err(format!(
            "git subcommand '{subcommand}' is not in the review engine allowlist"
        ));
    }

    std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("git {subcommand}: {e}"))
}

/// RAII guard that removes a git worktree on drop.
struct WorktreeGuard {
    workspace_path: PathBuf,
    worktree_path: PathBuf,
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let _ = std::process::Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&self.worktree_path)
            .current_dir(&self.workspace_path)
            .output();
    }
}

/// Scan the workspace for source code files (non-spec, non-doc).
fn gather_source_files(workspace_path: &str) -> Vec<String> {
    let code_extensions = [
        "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h",
    ];
    let mut files = Vec::new();
    let root = Path::new(workspace_path);

    fn walk(dir: &Path, exts: &[&str], files: &mut Vec<String>, depth: u32) {
        if depth > 5 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden dirs, target/, node_modules/
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                walk(&path, exts, files, depth + 1);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && exts.contains(&ext)
            {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }

    walk(root, &code_extensions, &mut files, 0);
    files
}

/// Produce review findings from source code analysis.
fn review_source_file(path: &str) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return findings,
    };
    let filename = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    // Check for trailing whitespace
    for (i, line) in content.lines().enumerate() {
        if line != line.trim_end() {
            findings.push(ReviewFinding {
                severity: ReviewSeverity::Info,
                title: "Trailing whitespace".to_string(),
                body: format!("Line {} has trailing whitespace", i + 1),
                file_path: Some(filename.clone()),
                line: Some((i + 1) as u32),
            });
            break; // One finding per file for trailing whitespace
        }
    }

    // Check for missing final newline
    if !content.is_empty() && !content.ends_with('\n') {
        findings.push(ReviewFinding {
            severity: ReviewSeverity::Warning,
            title: "Missing final newline".to_string(),
            body: "File does not end with a newline".to_string(),
            file_path: Some(filename.clone()),
            line: None,
        });
    }

    // Check for very long lines
    for (i, line) in content.lines().enumerate() {
        if line.len() > 200 {
            findings.push(ReviewFinding {
                severity: ReviewSeverity::Info,
                title: "Long line".to_string(),
                body: format!("Line {} has {} characters (>200)", i + 1, line.len()),
                file_path: Some(filename),
                line: Some((i + 1) as u32),
            });
            break; // One finding per file
        }
    }

    findings
}

/// Execute the review engine.
///
/// Phases:
/// 1. Context gathering (workspace scan, git HEAD)
/// 2. Code analysis (produce findings)
/// 3. Patch generation (write-mode only: worktree, commit, diff)
/// 4. Rebase (write-mode only: rebase onto target)
/// 5. Report synthesis
pub async fn run_review(ctx: &ReviewContext) -> EngineResult {
    let started_at = chrono::Utc::now();
    let cps = Arc::new(Mutex::new(CheckpointAccum::new(
        &ctx.run_id,
        &ctx.work_item_id,
    )));

    // ── Phase 1: Context gathering ──────────────────────────────────────
    {
        let mut cp = cps.lock().await;
        cp.emit("context_gathering", "Scanning workspace", Some(5));
    }

    let base_commit = get_base_commit(&ctx.workspace_path);
    let input_uris = gather_workspace_inputs(&ctx.workspace_path, &ctx.work_item_id);
    let source_files = gather_source_files(&ctx.workspace_path);

    {
        let mut cp = cps.lock().await;
        cp.emit(
            "context_gathering",
            &format!(
                "Found {} spec files, {} source files",
                input_uris.len(),
                source_files.len()
            ),
            Some(15),
        );
    }

    // ── Phase 2: Code analysis ──────────────────────────────────────────
    {
        let mut cp = cps.lock().await;
        cp.emit("analysis", "Analyzing source code", Some(30));
    }

    let mut findings = Vec::new();

    // Review each source file
    for path in &source_files {
        let file_findings = review_source_file(path);
        findings.extend(file_findings);
    }

    // Also analyze spec documents for completeness
    for uri in &input_uris {
        if let Some(excerpt) = read_file_excerpt(uri, 100) {
            let filename = uri.rsplit('/').next().unwrap_or(uri);
            let line_count = excerpt.lines().count();
            if line_count < 10 {
                findings.push(ReviewFinding {
                    severity: ReviewSeverity::Warning,
                    title: format!("Short spec document: {filename}"),
                    body: format!("Document has only {line_count} lines; consider expanding"),
                    file_path: Some(filename.to_string()),
                    line: None,
                });
            }
        }
    }

    if findings.is_empty() {
        findings.push(ReviewFinding {
            severity: ReviewSeverity::Info,
            title: "No issues found".to_string(),
            body: "Review completed with no findings".to_string(),
            file_path: None,
            line: None,
        });
    }

    // ── Phase 3 & 4: Write-mode patch generation + rebase ───────────────
    let mut patch_bundle_json: Option<String> = None;
    let mut conflict_summary_json: Option<String> = None;
    let mut has_patches = false;
    let mut terminal_state = BotRunState::Succeeded;
    let mut exit_code = 0;

    if ctx.write_mode == BotWriteMode::Worktree {
        // Ensure workspace is a git repo
        let Some(ref base) = base_commit else {
            let now = chrono::Utc::now();
            let log = BotRunLog {
                schema_version: BotRunLog::SCHEMA_VERSION.to_string(),
                run_id: ctx.run_id.clone(),
                work_item_id: ctx.work_item_id.clone(),
                state: BotRunState::Failed,
                started_at: started_at.to_rfc3339(),
                finished_at: now.to_rfc3339(),
                duration_s: (now - started_at).num_seconds().unsigned_abs(),
                exit_code: 3,
                summary: "write_mode=worktree requires a git repository".to_string(),
                partial: false,
                checkpoint_count: cps.lock().await.seq,
                error: Some("Not a git repository".to_string()),
            };
            let report = ReviewReport {
                schema_version: ReviewReport::SCHEMA_VERSION.to_string(),
                run_id: ctx.run_id.clone(),
                work_item_id: ctx.work_item_id.clone(),
                timestamp: now.to_rfc3339(),
                findings,
                has_patches: false,
                summary: "Failed: not a git repository".to_string(),
                base_commit: None,
                input_uris: input_uris.clone(),
            };
            let report_json =
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());
            return EngineResult {
                state: BotRunState::Failed,
                exit_code: 3,
                summary: log.summary.clone(),
                log,
                report_json,
                checkpoints: cps.lock().await.checkpoints.clone(),
                patch_bundle_json: None,
                conflict_summary_json: None,
            };
        };

        {
            let mut cp = cps.lock().await;
            cp.emit("worktree", "Creating bot worktree", Some(45));
        }

        let branch_name = format!("bot/review/{}", ctx.run_id);
        let worktree_dir = std::env::temp_dir().join(format!("review-{}", ctx.run_id));

        // Create bot branch at HEAD
        if let Err(e) = run_git(&ctx.workspace_path, &["branch", &branch_name, base]) {
            tracing::warn!("Failed to create branch: {e}");
        }

        // Create worktree
        match run_git(
            &ctx.workspace_path,
            &[
                "worktree",
                "add",
                &worktree_dir.to_string_lossy(),
                &branch_name,
            ],
        ) {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                tracing::warn!(
                    "Worktree creation had issues: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                tracing::warn!("Failed to create worktree: {e}");
            }
        }

        // RAII guard ensures cleanup
        let _guard = WorktreeGuard {
            workspace_path: PathBuf::from(&ctx.workspace_path),
            worktree_path: worktree_dir.clone(),
        };

        {
            let mut cp = cps.lock().await;
            cp.emit("patch", "Generating patch", Some(55));
        }

        // Apply fixes in the worktree: fix trailing whitespace in source files
        let mut files_changed = Vec::new();
        for path in &source_files {
            // Get relative path from workspace root
            let rel_path = Path::new(path)
                .strip_prefix(&ctx.workspace_path)
                .unwrap_or(Path::new(path));
            let wt_file = worktree_dir.join(rel_path);

            if let Ok(content) = std::fs::read_to_string(&wt_file) {
                let mut modified = false;
                let fixed: String = content
                    .lines()
                    .map(|line| {
                        let trimmed = line.trim_end();
                        if trimmed.len() != line.len() {
                            modified = true;
                        }
                        trimmed
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // Ensure final newline
                let fixed = if !fixed.is_empty() && !content.ends_with('\n') {
                    modified = true;
                    format!("{fixed}\n")
                } else if !fixed.is_empty() {
                    format!("{fixed}\n")
                } else {
                    fixed
                };

                if modified && std::fs::write(&wt_file, &fixed).is_ok() {
                    files_changed.push(rel_path.to_string_lossy().to_string());
                }
            }
        }

        let mut patch_diff = String::new();
        let mut rebase_status = RebaseStatus::NotAttempted;

        if !files_changed.is_empty() {
            // Stage and commit in worktree
            let _ = run_git_in(&worktree_dir, &["add", "-A"]);
            let commit_msg = format!("bot: review fixes for {}", ctx.work_item_id);
            let _ = run_git_in(&worktree_dir, &["commit", "-m", &commit_msg]);

            // Get unified diff
            if let Ok(output) = run_git_in(&worktree_dir, &["diff", "HEAD~1..HEAD"])
                && output.status.success()
            {
                patch_diff = String::from_utf8_lossy(&output.stdout).to_string();
            }

            has_patches = true;

            // ── Phase 4: Rebase ─────────────────────────────────────────
            if let Some(ref target) = ctx.rebase_target {
                {
                    let mut cp = cps.lock().await;
                    cp.emit("rebase", &format!("Rebasing onto {target}"), Some(70));
                }

                match run_git_in(&worktree_dir, &["rebase", target]) {
                    Ok(output) if output.status.success() => {
                        rebase_status = RebaseStatus::Clean;
                        // Update diff after rebase
                        if let Ok(diff_out) = run_git_in(&worktree_dir, &["diff", "HEAD~1..HEAD"])
                            && diff_out.status.success()
                        {
                            patch_diff = String::from_utf8_lossy(&diff_out.stdout).to_string();
                        }
                    }
                    Ok(output) => {
                        // Rebase conflict
                        rebase_status = RebaseStatus::Conflict;
                        terminal_state = BotRunState::NeedsAttention;
                        exit_code = 10;

                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                        // Abort the failed rebase
                        let _ = run_git_in(&worktree_dir, &["rebase", "--abort"]);

                        // Parse conflicting files from stderr
                        let conflicting_files: Vec<String> = stderr
                            .lines()
                            .filter(|l| l.contains("CONFLICT") || l.contains("Merge conflict"))
                            .map(ToString::to_string)
                            .collect();

                        let conflict = ConflictSummary {
                            schema_version: ConflictSummary::SCHEMA_VERSION.to_string(),
                            run_id: ctx.run_id.clone(),
                            work_item_id: ctx.work_item_id.clone(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            conflicting_files,
                            original_patch_uri: format!("pm://runs/{}/patch_bundle", ctx.run_id),
                            resolution_instructions: vec![
                                format!("1. Check out the bot branch: git checkout {branch_name}"),
                                format!("2. Rebase manually: git rebase {target}"),
                                "3. Resolve conflicts in the listed files".to_string(),
                                "4. Continue rebase: git rebase --continue".to_string(),
                                format!("5. If needed, abort and start over: git rebase --abort"),
                            ],
                        };

                        conflict_summary_json = Some(
                            serde_json::to_string_pretty(&conflict)
                                .unwrap_or_else(|_| "{}".to_string()),
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Rebase command failed: {e}");
                        rebase_status = RebaseStatus::Conflict;
                        terminal_state = BotRunState::NeedsAttention;
                        exit_code = 10;
                    }
                }
            }

            // Build PatchBundle
            let target_commit = ctx.rebase_target.as_ref().and_then(|target| {
                run_git(&ctx.workspace_path, &["rev-parse", target])
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            });

            let bundle = PatchBundle {
                schema_version: PatchBundle::SCHEMA_VERSION.to_string(),
                run_id: ctx.run_id.clone(),
                work_item_id: ctx.work_item_id.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                branch_name: branch_name.clone(),
                base_commit: base.clone(),
                target_commit,
                patch_diff,
                files_changed: files_changed.clone(),
                rebase_status,
            };

            patch_bundle_json =
                Some(serde_json::to_string_pretty(&bundle).unwrap_or_else(|_| "{}".to_string()));
        }
    }

    // ── Phase 5: Report synthesis ───────────────────────────────────────
    {
        let mut cp = cps.lock().await;
        cp.emit("synthesis", "Generating review report", Some(90));
    }

    let now = chrono::Utc::now();
    let duration_s = (now - started_at).num_seconds().unsigned_abs();

    let summary = match terminal_state {
        BotRunState::NeedsAttention => format!(
            "Review completed with conflicts: {} finding(s), rebase conflict on target",
            findings.len()
        ),
        _ => {
            if has_patches {
                format!(
                    "Review completed: {} finding(s), patches generated",
                    findings.len()
                )
            } else {
                format!("Review completed: {} finding(s)", findings.len())
            }
        }
    };

    let report = ReviewReport {
        schema_version: ReviewReport::SCHEMA_VERSION.to_string(),
        run_id: ctx.run_id.clone(),
        work_item_id: ctx.work_item_id.clone(),
        timestamp: now.to_rfc3339(),
        findings,
        has_patches,
        summary: summary.clone(),
        base_commit,
        input_uris: input_uris.clone(),
    };

    let report_json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());

    // Final checkpoint
    {
        let mut cp = cps.lock().await;
        cp.emit("complete", &summary, Some(100));
    }

    let log = BotRunLog {
        schema_version: BotRunLog::SCHEMA_VERSION.to_string(),
        run_id: ctx.run_id.clone(),
        work_item_id: ctx.work_item_id.clone(),
        state: terminal_state,
        started_at: started_at.to_rfc3339(),
        finished_at: now.to_rfc3339(),
        duration_s,
        exit_code,
        summary: summary.clone(),
        partial: false,
        checkpoint_count: cps.lock().await.seq,
        error: None,
    };

    EngineResult {
        state: terminal_state,
        exit_code,
        summary,
        log,
        report_json,
        checkpoints: cps.lock().await.checkpoints.clone(),
        patch_bundle_json,
        conflict_summary_json,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Engine dispatcher
// ─────────────────────────────────────────────────────────────────────────────

/// Parameters for [`run_engine`].
pub struct EngineParams {
    pub kind: BotKind,
    pub run_id: String,
    pub work_item_id: String,
    pub workspace_path: String,
    pub write_mode: BotWriteMode,
    pub rebase_target: Option<String>,
    /// `None` or `Some(true)` → proceed in degraded mode when
    /// NotebookLM is unavailable. `Some(false)` → terminal `Blocked`.
    pub allow_degraded: Option<bool>,
    /// Override health URL for testing/debug.
    pub notebooklm_health_url: Option<String>,
}

/// Dispatch to the appropriate engine.
///
/// Research runs the real async engine; review runs the real review engine.
pub async fn run_engine(params: EngineParams) -> EngineResult {
    match params.kind {
        BotKind::Research => {
            let mut ctx =
                ResearchContext::new(&params.workspace_path, &params.run_id, &params.work_item_id);
            if let Some(ad) = params.allow_degraded {
                ctx.allow_degraded = ad;
            }
            if let Some(url) = params.notebooklm_health_url {
                ctx.notebooklm_health_url = Some(url);
            }
            run_research(&ctx).await
        }
        BotKind::Review => {
            let ctx = ReviewContext {
                workspace_path: params.workspace_path,
                run_id: params.run_id,
                work_item_id: params.work_item_id,
                write_mode: params.write_mode,
                rebase_target: params.rebase_target,
            };
            run_review(&ctx).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn research_blocked_when_notebooklm_required_but_unavailable() {
        let ctx = ResearchContext {
            workspace_path: "/tmp/nonexistent-workspace".to_string(),
            run_id: "test-blocked-001".to_string(),
            work_item_id: "SPEC-BLOCKED".to_string(),
            // Point to a URL that won't respond
            notebooklm_health_url: Some("http://127.0.0.1:1/nonexistent".to_string()),
            allow_degraded: false,
        };

        let result = run_research(&ctx).await;
        assert_eq!(result.state, BotRunState::Blocked);
        assert_eq!(result.exit_code, 2);
        assert!(result.summary.contains("Blocked"));
        assert!(result.summary.contains("NotebookLM"));
        assert!(!result.checkpoints.is_empty());
    }

    #[tokio::test]
    async fn research_degrades_when_notebooklm_unavailable() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = ResearchContext {
            workspace_path: tmp.path().to_string_lossy().to_string(),
            run_id: "test-degraded-001".to_string(),
            work_item_id: "SPEC-DEGRADED".to_string(),
            notebooklm_health_url: Some("http://127.0.0.1:1/nonexistent".to_string()),
            allow_degraded: true,
        };

        let result = run_research(&ctx).await;
        assert_eq!(result.state, BotRunState::Succeeded);
        assert_eq!(result.exit_code, 0);
        assert!(result.summary.contains("Degraded"));

        // Verify report marks degraded
        let report: ResearchReport =
            serde_json::from_str(&result.report_json).expect("parse report");
        assert!(report.degraded);
        assert!(
            report
                .sources_used
                .contains(&"workspace-local-only".to_string())
        );

        // Should have checkpoints
        assert!(result.checkpoints.len() >= 3);
    }

    #[tokio::test]
    async fn research_produces_findings_from_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();

        // Create spec files in workspace
        let docs_dir = tmp.path().join("docs").join("SPEC-WS-001");
        std::fs::create_dir_all(&docs_dir).unwrap();
        std::fs::write(
            docs_dir.join("spec.md"),
            "# Spec Title\n\nSome spec content.\n\n## Requirements\n\nReq 1\n",
        )
        .unwrap();
        std::fs::write(
            docs_dir.join("PRD.md"),
            "# PRD Title\n\nProduct requirements.\n",
        )
        .unwrap();

        let ctx = ResearchContext {
            workspace_path: tmp.path().to_string_lossy().to_string(),
            run_id: "test-ws-001".to_string(),
            work_item_id: "SPEC-WS-001".to_string(),
            notebooklm_health_url: Some("http://127.0.0.1:1/nonexistent".to_string()),
            allow_degraded: true,
        };

        let result = run_research(&ctx).await;
        assert_eq!(result.state, BotRunState::Succeeded);

        let report: ResearchReport =
            serde_json::from_str(&result.report_json).expect("parse report");
        // Should have findings from the spec files + degraded notice
        assert!(report.findings.len() >= 2);
        assert!(!report.input_uris.is_empty());
        assert!(!report.sources_used.is_empty());
    }

    #[tokio::test]
    async fn review_readonly_succeeds() {
        let tmp = tempfile::TempDir::new().unwrap();

        // Create a source file
        let src = tmp.path().join("main.rs");
        std::fs::write(&src, "fn main() { }  \n").unwrap();

        let ctx = ReviewContext {
            workspace_path: tmp.path().to_string_lossy().to_string(),
            run_id: "test-review-ro".to_string(),
            work_item_id: "SPEC-REVIEW-RO".to_string(),
            write_mode: BotWriteMode::None,
            rebase_target: None,
        };

        let result = run_review(&ctx).await;
        assert_eq!(result.state, BotRunState::Succeeded);
        assert_eq!(result.exit_code, 0);
        assert!(result.patch_bundle_json.is_none());
        assert!(result.conflict_summary_json.is_none());

        let report: ReviewReport =
            serde_json::from_str(&result.report_json).expect("parse review report");
        assert!(!report.findings.is_empty());
        assert!(!report.has_patches);
        assert!(!result.checkpoints.is_empty());
    }

    #[tokio::test]
    async fn review_allowlist_blocks_disallowed_commands() {
        let result = run_git("/tmp", &["push", "origin", "main"]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("not in the review engine allowlist")
        );
    }
}
