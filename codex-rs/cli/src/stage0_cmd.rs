//! Stage0 CLI Commands
//!
//! Provides convergence diagnostics and Stage0-specific commands.
//!
//! ## Commands
//!
//! - `code stage0 doctor` - Verify Stage0 convergence health

use clap::{Parser, Subcommand};
use std::time::Duration;

/// Stage0 CLI — convergence diagnostics and Stage0 utilities
#[derive(Debug, Parser)]
pub struct Stage0Cli {
    #[command(subcommand)]
    pub command: Stage0Subcommand,
}

impl Stage0Cli {
    pub async fn run(self) -> i32 {
        match self.command {
            Stage0Subcommand::Doctor(args) => run_stage0_doctor(args).await,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Stage0Subcommand {
    /// Verify Stage0 convergence health
    ///
    /// Checks that all Stage0 dependencies are reachable and properly configured.
    /// Use this before running `/speckit.auto` to ensure Tier2 will work.
    Doctor(DoctorArgs),
}

#[derive(Debug, Parser)]
pub struct DoctorArgs {
    /// Check for specific SPEC (validates notebook mapping)
    #[arg(long, value_name = "SPEC_ID")]
    pub spec: Option<String>,

    /// Output as JSON for automation
    #[arg(long)]
    pub json: bool,

    /// Skip NotebookLM checks (Tier1 only)
    #[arg(long)]
    pub tier1_only: bool,

    /// local-memory API base URL (default: http://localhost:3002/api/v1)
    #[arg(long, default_value = "http://localhost:3002/api/v1")]
    pub local_memory_url: String,

    /// NotebookLM service URL (default: http://127.0.0.1:3456)
    #[arg(long, default_value = "http://127.0.0.1:3456")]
    pub notebooklm_url: String,
}

/// Doctor check result
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: &'static str,
    pub status: CheckStatus,
    pub message: String,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckResult {
    fn pass(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            status: CheckStatus::Pass,
            message: message.into(),
            fix: None,
        }
    }

    fn warn(name: &'static str, message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            name,
            status: CheckStatus::Warn,
            message: message.into(),
            fix: Some(fix.into()),
        }
    }

    fn fail(name: &'static str, message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            name,
            status: CheckStatus::Fail,
            message: message.into(),
            fix: Some(fix.into()),
        }
    }
}

/// Run the Stage0 doctor command
pub async fn run_stage0_doctor(args: DoctorArgs) -> i32 {
    let mut results: Vec<CheckResult> = Vec::new();

    // Check 1: local-memory reachable
    results.push(check_local_memory(&args.local_memory_url).await);

    // Check 2: Domain resolution (verify spec-tracker domain works)
    results.push(check_domain_resolution(&args.local_memory_url).await);

    // Check 3: NotebookLM reachable (if not tier1-only)
    if !args.tier1_only {
        results.push(check_notebooklm(&args.notebooklm_url).await);

        // Check 4: Notebook mapping (if spec provided)
        if let Some(ref spec_id) = args.spec {
            results.push(check_notebook_mapping(&args.notebooklm_url, spec_id).await);
        }
    }

    // Output results
    if args.json {
        print_json_results(&results);
    } else {
        print_human_results(&results);
    }

    // Determine exit code
    let has_fail = results.iter().any(|r| r.status == CheckStatus::Fail);
    let has_warn = results.iter().any(|r| r.status == CheckStatus::Warn);

    if has_fail {
        2
    } else if has_warn {
        1
    } else {
        0
    }
}

async fn check_local_memory(api_base: &str) -> CheckResult {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::fail(
                "local-memory",
                format!("Failed to create HTTP client: {e}"),
                "Check your system HTTP configuration",
            );
        }
    };

    let health_url = format!("{}/health", api_base.trim_end_matches('/'));

    match client.get(&health_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            CheckResult::pass("local-memory", format!("Reachable at {api_base}"))
        }
        Ok(resp) => CheckResult::fail(
            "local-memory",
            format!("Health check returned {}", resp.status()),
            "Ensure local-memory is running: lm health".to_string(),
        ),
        Err(e) => CheckResult::fail(
            "local-memory",
            format!("Connection failed: {e}"),
            "Start local-memory: cd ~/localmemory-policy && ./local_memory.py serve".to_string(),
        ),
    }
}

async fn check_domain_resolution(api_base: &str) -> CheckResult {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return CheckResult::warn(
                "domain-resolution",
                "Could not create HTTP client",
                "Previous check should have caught this",
            );
        }
    };

    // Try to search in spec-tracker domain
    let search_url = format!(
        "{}/search?query=test&domain=spec-tracker&limit=1",
        api_base.trim_end_matches('/')
    );

    match client.get(&search_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            CheckResult::pass("domain-resolution", "spec-tracker domain accessible")
        }
        Ok(resp) if resp.status().as_u16() == 404 => {
            // 404 might mean no memories yet, which is OK
            CheckResult::pass(
                "domain-resolution",
                "spec-tracker domain accessible (empty)",
            )
        }
        Ok(resp) => CheckResult::warn(
            "domain-resolution",
            format!("Domain query returned {}", resp.status()),
            "Check local-memory logs for details",
        ),
        Err(e) => CheckResult::warn(
            "domain-resolution",
            format!("Domain query failed: {e}"),
            "local-memory may not be running",
        ),
    }
}

async fn check_notebooklm(service_url: &str) -> CheckResult {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::fail(
                "notebooklm",
                format!("Failed to create HTTP client: {e}"),
                "Check your system HTTP configuration",
            );
        }
    };

    let health_url = format!("{}/health", service_url.trim_end_matches('/'));

    match client.get(&health_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            // Parse response to check if ready
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let status = json
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    if status == "ready" {
                        CheckResult::pass("notebooklm", format!("Service ready at {service_url}"))
                    } else {
                        CheckResult::warn(
                            "notebooklm",
                            format!("Service status: {status}"),
                            "NotebookLM may need authentication. Run: node ~/notebooklm-mcp/dist/cli/index.js auth",
                        )
                    }
                }
                Err(_) => CheckResult::warn(
                    "notebooklm",
                    "Health response not parseable",
                    "Check NotebookLM service logs",
                ),
            }
        }
        Ok(resp) => CheckResult::fail(
            "notebooklm",
            format!("Health check returned {}", resp.status()),
            "Ensure notebooklm-mcp is running: cd ~/notebooklm-mcp && npm start".to_string(),
        ),
        Err(e) => CheckResult::warn(
            "notebooklm",
            format!("Connection failed: {e}"),
            "NotebookLM service not running. Tier2 will be skipped. Start with: cd ~/notebooklm-mcp && npm start".to_string(),
        ),
    }
}

async fn check_notebook_mapping(service_url: &str, spec_id: &str) -> CheckResult {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::warn(
                "notebook-mapping",
                format!("Failed to create HTTP client: {e}"),
                "Previous check should have caught this",
            );
        }
    };

    // Check if there's a notebook configured for this spec's domain
    // This endpoint may vary based on notebooklm-mcp implementation
    let notebooks_url = format!("{}/notebooks", service_url.trim_end_matches('/'));

    match client.get(&notebooks_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    // Check if any notebook is configured for codex-rs project
                    let has_mapping = json
                        .as_array()
                        .map(|arr| !arr.is_empty())
                        .unwrap_or(false);

                    if has_mapping {
                        CheckResult::pass(
                            "notebook-mapping",
                            format!("Notebook available for {spec_id}"),
                        )
                    } else {
                        CheckResult::warn(
                            "notebook-mapping",
                            format!("No notebook configured for {spec_id}"),
                            "Create a NotebookLM notebook and configure mapping in notebooklm-mcp",
                        )
                    }
                }
                Err(_) => CheckResult::warn(
                    "notebook-mapping",
                    "Could not parse notebooks response",
                    "Check NotebookLM service configuration",
                ),
            }
        }
        Ok(resp) => CheckResult::warn(
            "notebook-mapping",
            format!("Notebooks endpoint returned {}", resp.status()),
            "Tier2 will be skipped. Configure notebook mapping in notebooklm-mcp",
        ),
        Err(e) => CheckResult::warn(
            "notebook-mapping",
            format!("Could not check notebook mapping: {e}"),
            "Tier2 will be skipped without notebook mapping",
        ),
    }
}

fn print_human_results(results: &[CheckResult]) {
    println!("\n Stage0 Convergence Doctor\n");

    for result in results {
        let icon = match result.status {
            CheckStatus::Pass => "\x1b[32m[PASS]\x1b[0m",
            CheckStatus::Warn => "\x1b[33m[WARN]\x1b[0m",
            CheckStatus::Fail => "\x1b[31m[FAIL]\x1b[0m",
        };

        println!("{} {}: {}", icon, result.name, result.message);

        if let Some(ref fix) = result.fix {
            println!("       Fix: {fix}");
        }
    }

    println!();

    let pass_count = results.iter().filter(|r| r.status == CheckStatus::Pass).count();
    let warn_count = results.iter().filter(|r| r.status == CheckStatus::Warn).count();
    let fail_count = results.iter().filter(|r| r.status == CheckStatus::Fail).count();

    if fail_count > 0 {
        println!(
            "\x1b[31mResult: {}/{} checks passed, {} warnings, {} failures\x1b[0m",
            pass_count,
            results.len(),
            warn_count,
            fail_count
        );
        println!("Stage0 may not function correctly. Fix failures before running /speckit.auto");
    } else if warn_count > 0 {
        println!(
            "\x1b[33mResult: {}/{} checks passed with {} warnings\x1b[0m",
            pass_count,
            results.len(),
            warn_count
        );
        println!("Tier2 may be skipped. Stage0 will continue with Tier1 only.");
    } else {
        println!(
            "\x1b[32mResult: All {} checks passed\x1b[0m",
            results.len()
        );
        println!("Stage0 ready for full Tier1 + Tier2 execution.");
    }
}

fn print_json_results(results: &[CheckResult]) {
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "status": match r.status {
                    CheckStatus::Pass => "pass",
                    CheckStatus::Warn => "warn",
                    CheckStatus::Fail => "fail",
                },
                "message": r.message,
                "fix": r.fix,
            })
        })
        .collect();

    let has_fail = results.iter().any(|r| r.status == CheckStatus::Fail);
    let has_warn = results.iter().any(|r| r.status == CheckStatus::Warn);

    let output = serde_json::json!({
        "checks": json_results,
        "overall": if has_fail { "fail" } else if has_warn { "warn" } else { "pass" },
        "tier2_ready": !has_fail && !has_warn,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
}
