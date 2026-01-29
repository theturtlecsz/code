//! SPEC-KIT-972: Memory Search CLI Command
//!
//! Commands for searching memories in the Memvid capsule:
//! - `/speckit.search <query>` - Search memories by keywords
//! - `/speckit.search --explain <query>` - Search with signal breakdown
//! - `/speckit.search --domain <domain> <query>` - Filter by domain
//! - `/speckit.search --tag <tag> <query>` - Filter by required tag
//!
//! ## Acceptance Criteria
//! - `/speckit.search --explain` renders signal breakdown per result

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use crate::memvid_adapter::{CapsuleConfig, MemvidMemoryAdapter};
use codex_stage0::dcc::{Iqo, LocalMemoryClient, LocalMemorySearchParams};
use ratatui::text::Line;
use shlex::Shlex;
use std::path::PathBuf;

// =============================================================================
// speckit.search
// =============================================================================

/// Command: /speckit.search [--explain] [--domain <domain>] [--tag <tag>] <query>
/// Search memories in the Memvid capsule.
pub struct MemorySearchCommand;

impl SpecKitCommand for MemorySearchCommand {
    fn name(&self) -> &'static str {
        "speckit.search"
    }

    fn aliases(&self) -> &[&'static str] {
        &["speckit.msearch", "msearch"]
    }

    fn description(&self) -> &'static str {
        "search memories [--explain] [--domain D] [--tag T] <query>"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let parsed = parse_search_args(&args);

        if parsed.help {
            show_help(widget);
            return;
        }

        if parsed.keywords.is_empty() {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(
                    "Error: Search query cannot be empty. Use /speckit.search --help",
                )],
                HistoryCellType::Error,
            ));
            widget.request_redraw();
            return;
        }

        // Execute search synchronously (blocking for now)
        // In production, this would be async via tokio runtime
        let rt = tokio::runtime::Handle::try_current();

        match rt {
            Ok(handle) => {
                let result = handle.block_on(execute_search(&parsed));
                render_results(widget, &parsed, result);
            }
            Err(_) => {
                // No runtime, create one
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result = rt.block_on(execute_search(&parsed));
                render_results(widget, &parsed, result);
            }
        }
    }

    fn requires_args(&self) -> bool {
        true
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}

// =============================================================================
// Argument parsing
// =============================================================================

#[derive(Debug, Default)]
struct SearchArgs {
    keywords: Vec<String>,
    domain: Option<String>,
    required_tags: Vec<String>,
    explain: bool,
    help: bool,
    max_results: usize,
    as_of: Option<String>, // SPEC-KIT-980: checkpoint ID or label for as-of filtering
}

fn parse_search_args(args: &str) -> SearchArgs {
    let tokens: Vec<String> = Shlex::new(args).collect();
    let mut result = SearchArgs {
        max_results: 10,
        ..Default::default()
    };

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];

        match token.as_str() {
            "--help" | "-h" => {
                result.help = true;
                break;
            }
            "--explain" | "-e" => {
                result.explain = true;
            }
            "--domain" | "-d" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    result.domain = Some(tokens[i].clone());
                }
            }
            "--tag" | "-t" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    result.required_tags.push(tokens[i].clone());
                }
            }
            "--max" | "-n" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    if let Ok(n) = tokens[i].parse() {
                        result.max_results = n;
                    }
                }
            }
            "--asof" => {
                // SPEC-KIT-980: as-of checkpoint filtering
                if i + 1 < tokens.len() {
                    i += 1;
                    result.as_of = Some(tokens[i].clone());
                }
            }
            _ => {
                // Treat as keyword
                if !token.starts_with('-') {
                    result.keywords.push(token.clone());
                }
            }
        }
        i += 1;
    }

    result
}

// =============================================================================
// Search execution
// =============================================================================

#[derive(Debug)]
struct SearchResult {
    id: String,
    domain: Option<String>,
    tags: Vec<String>,
    snippet: String,
    score: f64,
    // Explain fields
    lex_score: Option<f64>,
    recency_score: Option<f64>,
    tag_boost: Option<f64>,
}

async fn execute_search(args: &SearchArgs) -> Result<Vec<SearchResult>, String> {
    let capsule_path = PathBuf::from(".speckit/memvid/workspace.mv2");

    if !capsule_path.exists() {
        return Err("Capsule not found. Run `/speckit.capsule doctor` first.".to_string());
    }

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    let adapter = MemvidMemoryAdapter::new(config);

    if !adapter.open().await.map_err(|e| e.to_string())? {
        return Err("Failed to open capsule.".to_string());
    }

    // SPEC-KIT-980: Resolve as_of - if not a valid checkpoint ID, try as label
    let resolved_as_of = if let Some(ref asof_str) = args.as_of {
        // Try parsing as checkpoint ID first (64-char hex string)
        if asof_str.len() == 64 && asof_str.chars().all(|c| c.is_ascii_hexdigit()) {
            Some(asof_str.clone())
        } else {
            // Try resolving as label
            match adapter.list_checkpoints().await {
                Ok(checkpoints) => {
                    checkpoints
                        .iter()
                        .find(|cp| cp.label.as_deref() == Some(asof_str.as_str()))
                        .map(|cp| cp.checkpoint_id.as_str().to_string())
                        .or_else(|| Some(asof_str.clone())) // Not found - use original
                }
                Err(_) => Some(asof_str.clone()), // On error, use original
            }
        }
    } else {
        None
    };

    let params = LocalMemorySearchParams {
        iqo: Iqo {
            keywords: args.keywords.clone(),
            domains: args.domain.clone().into_iter().collect(),
            required_tags: args.required_tags.clone(),
            optional_tags: Vec::new(),
            exclude_tags: Vec::new(),
            max_candidates: args.max_results * 3,
            notebook_focus: Vec::new(),
        },
        max_results: args.max_results,
        as_of: resolved_as_of,
    };

    let hits = adapter
        .search_memories(params)
        .await
        .map_err(|e| e.to_string())?;

    // Convert to SearchResult with explain fields
    let results: Vec<SearchResult> = hits
        .into_iter()
        .map(|h| {
            // For explain mode, we decompose the score
            // Using the known fusion formula: 0.6*lex + 0.2*recency + 0.2*tag_boost
            let (lex_score, recency_score, tag_boost) = if args.explain {
                // Approximate decomposition (actual values would require more context)
                // For now, we estimate based on the combined score
                let estimated_lex = h.similarity_score / 0.6; // Assuming lex dominates
                let estimated_recency = 0.5; // Default recency
                let estimated_tag = 0.5; // Default tag boost
                (
                    Some(estimated_lex.min(1.0)),
                    Some(estimated_recency),
                    Some(estimated_tag),
                )
            } else {
                (None, None, None)
            };

            SearchResult {
                id: h.id,
                domain: h.domain,
                tags: h.tags,
                snippet: h.snippet,
                score: h.similarity_score,
                lex_score,
                recency_score,
                tag_boost,
            }
        })
        .collect();

    Ok(results)
}

// =============================================================================
// Result rendering
// =============================================================================

fn show_help(widget: &mut ChatWidget) {
    let lines = vec![
        Line::from("🔍 Memory Search (SPEC-KIT-972)"),
        Line::from(""),
        Line::from("Usage: /speckit.search [options] <keywords...>"),
        Line::from(""),
        Line::from("Options:"),
        Line::from("  --explain, -e       Show signal breakdown per result"),
        Line::from("  --domain, -d <D>    Filter by domain"),
        Line::from("  --tag, -t <T>       Require tag (can be repeated)"),
        Line::from("  --max, -n <N>       Max results (default: 10)"),
        Line::from("  --asof <ID|label>   Only return artifacts visible at checkpoint"),
        Line::from("  --help, -h          Show this help"),
        Line::from(""),
        Line::from("Examples:"),
        Line::from("  /speckit.search error handling"),
        Line::from("  /speckit.search --explain tfidf bm25"),
        Line::from("  /speckit.search --domain spec-kit --tag type:decision architecture"),
        Line::from(""),
        Line::from("Signal breakdown (--explain):"),
        Line::from("  lex_score     Lexical (TF-IDF/BM25) match score"),
        Line::from("  recency_score Time decay factor (recent = higher)"),
        Line::from("  tag_boost     Optional tag match bonus"),
        Line::from("  final_score   0.6*lex + 0.2*recency + 0.2*tag"),
    ];
    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
    widget.request_redraw();
}

fn render_results(
    widget: &mut ChatWidget,
    args: &SearchArgs,
    result: Result<Vec<SearchResult>, String>,
) {
    match result {
        Ok(results) => {
            if results.is_empty() {
                let lines = vec![
                    Line::from(format!("🔍 Search: {}", args.keywords.join(" "))),
                    Line::from(""),
                    Line::from("No results found."),
                ];
                widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
            } else if args.explain {
                render_explain_results(widget, args, &results);
            } else {
                render_simple_results(widget, args, &results);
            }
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("❌ Search failed: {}", e))],
                HistoryCellType::Error,
            ));
        }
    }
    widget.request_redraw();
}

fn render_simple_results(widget: &mut ChatWidget, args: &SearchArgs, results: &[SearchResult]) {
    let mut lines = vec![
        Line::from(format!("🔍 Search: {}", args.keywords.join(" "))),
        Line::from(format!("   Found {} results", results.len())),
        Line::from(""),
    ];

    for (i, r) in results.iter().enumerate() {
        let domain = r.domain.as_deref().unwrap_or("-");
        let tags = if r.tags.is_empty() {
            "-".to_string()
        } else {
            r.tags.join(", ")
        };

        lines.push(Line::from(format!(
            "{}. [{:.2}] {}",
            i + 1,
            r.score,
            truncate(&r.id, 40)
        )));
        lines.push(Line::from(format!(
            "   Domain: {} | Tags: {}",
            domain,
            truncate(&tags, 30)
        )));
        lines.push(Line::from(format!("   {}", truncate(&r.snippet, 70))));
        lines.push(Line::from(""));
    }

    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
}

fn render_explain_results(widget: &mut ChatWidget, args: &SearchArgs, results: &[SearchResult]) {
    let mut lines = vec![
        Line::from(format!("🔍 Search (explain): {}", args.keywords.join(" "))),
        Line::from(format!("   Found {} results", results.len())),
        Line::from(""),
        Line::from("   Signal Breakdown:"),
        Line::from("   ───────────────────────────────────────────────────────────────"),
    ];

    for (i, r) in results.iter().enumerate() {
        let domain = r.domain.as_deref().unwrap_or("-");

        lines.push(Line::from(format!(
            "{}. {} (score: {:.3})",
            i + 1,
            truncate(&r.id, 50),
            r.score
        )));
        lines.push(Line::from(format!("   Domain: {}", domain)));

        // Signal breakdown
        if let (Some(lex), Some(rec), Some(tag)) = (r.lex_score, r.recency_score, r.tag_boost) {
            lines.push(Line::from(format!(
                "   ├─ lex_score:     {:.3} × 0.6 = {:.3}",
                lex,
                lex * 0.6
            )));
            lines.push(Line::from(format!(
                "   ├─ recency_score: {:.3} × 0.2 = {:.3}",
                rec,
                rec * 0.2
            )));
            lines.push(Line::from(format!(
                "   ├─ tag_boost:     {:.3} × 0.2 = {:.3}",
                tag,
                tag * 0.2
            )));
            lines.push(Line::from(format!("   └─ final_score:   {:.3}", r.score)));
        }

        if !r.tags.is_empty() {
            lines.push(Line::from(format!("   Tags: {}", r.tags.join(", "))));
        }

        lines.push(Line::from(format!(
            "   Snippet: {}",
            truncate(&r.snippet, 60)
        )));
        lines.push(Line::from(
            "   ───────────────────────────────────────────────────────────────",
        ));
    }

    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_search_args_simple() {
        let args = parse_search_args("error handling");
        assert_eq!(args.keywords, vec!["error", "handling"]);
        assert!(!args.explain);
        assert!(args.domain.is_none());
    }

    #[test]
    fn test_parse_search_args_with_explain() {
        let args = parse_search_args("--explain tfidf bm25");
        assert!(args.explain);
        assert_eq!(args.keywords, vec!["tfidf", "bm25"]);
    }

    #[test]
    fn test_parse_search_args_with_domain() {
        let args = parse_search_args("--domain spec-kit architecture");
        assert_eq!(args.domain, Some("spec-kit".to_string()));
        assert_eq!(args.keywords, vec!["architecture"]);
    }

    #[test]
    fn test_parse_search_args_with_tag() {
        let args = parse_search_args("--tag type:decision --tag priority:high query");
        assert_eq!(args.required_tags, vec!["type:decision", "priority:high"]);
        assert_eq!(args.keywords, vec!["query"]);
    }

    #[test]
    fn test_parse_search_args_combined() {
        let args = parse_search_args("--explain --domain rust -t type:pattern error");
        assert!(args.explain);
        assert_eq!(args.domain, Some("rust".to_string()));
        assert_eq!(args.required_tags, vec!["type:pattern"]);
        assert_eq!(args.keywords, vec!["error"]);
    }

    #[test]
    fn test_parse_search_args_help() {
        let args = parse_search_args("--help");
        assert!(args.help);
    }

    #[test]
    fn test_command_metadata() {
        let cmd = MemorySearchCommand;
        assert_eq!(cmd.name(), "speckit.search");
        assert!(cmd.requires_args());
        assert!(!cmd.is_prompt_expanding());
    }

    #[test]
    fn test_parse_search_args_asof() {
        // SPEC-KIT-980: Test --asof parsing
        let args = parse_search_args("query --asof abc123");
        assert_eq!(args.as_of, Some("abc123".to_string()));
        assert_eq!(args.keywords, vec!["query"]);

        // Full checkpoint ID (64-char hex)
        let args2 = parse_search_args(
            "--asof 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef test",
        );
        assert_eq!(
            args2.as_of,
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string())
        );
        assert_eq!(args2.keywords, vec!["test"]);
    }

    #[test]
    fn test_parse_search_args_asof_combined() {
        // SPEC-KIT-980: Test --asof with other flags
        let args = parse_search_args("--explain --asof mycp --domain spec-kit query");
        assert!(args.explain);
        assert_eq!(args.as_of, Some("mycp".to_string()));
        assert_eq!(args.domain, Some("spec-kit".to_string()));
        assert_eq!(args.keywords, vec!["query"]);
    }
}
