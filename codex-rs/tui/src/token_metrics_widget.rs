// FORK-SPECIFIC (just-every/code): P6-SYNC Phase 6 - TokenMetrics UI
//!
//! Status bar widget for real-time token tracking with context utilization
//! warnings and cost estimation. Uses SessionMetrics from Phase 2.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::chatwidget::spec_kit::cost_tracker::ModelPricing;
use crate::chatwidget::spec_kit::session_metrics::SessionMetrics;

/// Token metrics display for status bar
#[derive(Debug, Clone)]
pub struct TokenMetricsWidget {
    pub total_input: u64,
    pub total_output: u64,
    pub turn_count: u32,
    pub estimated_next: u64,
    pub context_utilization: f64, // 0.0 - 1.0
    pub estimated_cost_usd: Option<f64>,
}

impl TokenMetricsWidget {
    /// Create from SessionMetrics with model context window
    pub fn from_session_metrics(
        metrics: &SessionMetrics,
        context_window: u64,
        model_id: &str,
    ) -> Self {
        let total = metrics.running_total();
        let utilization = if context_window > 0 {
            metrics.blended_total() as f64 / context_window as f64
        } else {
            0.0
        };

        let pricing = ModelPricing::for_model(model_id);
        let cost = pricing.calculate(total.input_tokens, total.output_tokens);

        Self {
            total_input: total.input_tokens,
            total_output: total.output_tokens,
            turn_count: metrics.turn_count(),
            estimated_next: metrics.estimated_next_prompt_tokens(),
            context_utilization: utilization,
            estimated_cost_usd: Some(cost),
        }
    }

    /// Create with explicit values (for testing or custom scenarios)
    pub fn new(
        total_input: u64,
        total_output: u64,
        turn_count: u32,
        estimated_next: u64,
        context_utilization: f64,
        estimated_cost_usd: Option<f64>,
    ) -> Self {
        Self {
            total_input,
            total_output,
            turn_count,
            estimated_next,
            context_utilization,
            estimated_cost_usd,
        }
    }

    /// Format tokens for display (e.g., "12.5k")
    fn format_tokens(tokens: u64) -> String {
        if tokens >= 1_000_000 {
            format!("{:.1}M", tokens as f64 / 1_000_000.0)
        } else if tokens >= 1_000 {
            format!("{:.1}k", tokens as f64 / 1_000.0)
        } else {
            tokens.to_string()
        }
    }

    /// Get utilization color based on percentage
    fn utilization_color(&self) -> Color {
        if self.context_utilization > 0.9 {
            Color::Red // Critical
        } else if self.context_utilization > 0.8 {
            Color::Yellow // Warning
        } else if self.context_utilization > 0.6 {
            Color::Cyan // Moderate
        } else {
            Color::Green // Healthy
        }
    }

    /// Render full format: "Tokens: 12.5k in / 3.2k out | Turn 5 | Est: ~4k | Ctx: 45% | $0.12"
    pub fn render_full(&self) -> Line<'static> {
        let mut spans = vec![
            Span::raw("Tokens: "),
            Span::styled(
                Self::format_tokens(self.total_input),
                Style::default().bold(),
            ),
            Span::raw(" in / "),
            Span::styled(
                Self::format_tokens(self.total_output),
                Style::default().bold(),
            ),
            Span::raw(" out"),
            Span::raw(" | "),
            Span::raw(format!("Turn {}", self.turn_count)),
            Span::raw(" | "),
            Span::raw("Est: ~"),
            Span::raw(Self::format_tokens(self.estimated_next)),
            Span::raw(" | "),
            Span::raw("Ctx: "),
            Span::styled(
                format!("{:.0}%", self.context_utilization * 100.0),
                Style::default().fg(self.utilization_color()),
            ),
        ];

        if let Some(cost) = self.estimated_cost_usd {
            spans.push(Span::raw(" | "));
            spans.push(Span::styled(
                format!("${:.2}", cost),
                Style::default().dim(),
            ));
        }

        Line::from(spans)
    }

    /// Render compact format: "12.5k/3.2k | T5 | ~4k | 45%"
    pub fn render_compact(&self) -> Line<'static> {
        Line::from(vec![
            Span::styled(
                Self::format_tokens(self.total_input),
                Style::default().bold(),
            ),
            Span::raw("/"),
            Span::styled(
                Self::format_tokens(self.total_output),
                Style::default().bold(),
            ),
            Span::raw(" | T"),
            Span::raw(self.turn_count.to_string()),
            Span::raw(" | ~"),
            Span::raw(Self::format_tokens(self.estimated_next)),
            Span::raw(" | "),
            Span::styled(
                format!("{:.0}%", self.context_utilization * 100.0),
                Style::default().fg(self.utilization_color()),
            ),
        ])
    }

    /// Check if context utilization is critical (>90%)
    pub fn is_critical(&self) -> bool {
        self.context_utilization > 0.9
    }

    /// Check if context utilization is warning (>80%)
    pub fn is_warning(&self) -> bool {
        self.context_utilization > 0.8
    }

    /// Get warning message if context is near capacity
    pub fn warning_message(&self) -> Option<String> {
        if self.is_critical() {
            Some(format!(
                "Context {:.0}% full - consider compaction",
                self.context_utilization * 100.0
            ))
        } else if self.is_warning() {
            Some(format!(
                "Context approaching limit ({:.0}%)",
                self.context_utilization * 100.0
            ))
        } else {
            None
        }
    }
}

impl Widget for TokenMetricsWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = if area.width > 60 {
            self.render_full()
        } else {
            self.render_compact()
        };

        buf.set_line(area.x, area.y, &line, area.width);
    }
}

/// Get context window size for a model
///
/// Covers OpenAI, Claude, and Gemini models.
/// Returns reasonable defaults for unknown models.
pub fn model_context_window(model_id: &str) -> u64 {
    let model_lower = model_id.to_lowercase();

    // OpenAI models
    if model_lower.starts_with("gpt-5") || model_lower.starts_with("gpt5") {
        return 272_000;
    }
    if model_lower.contains("codex") {
        return 272_000;
    }
    if model_lower.starts_with("o3") || model_lower.starts_with("o4") {
        return 200_000;
    }
    if model_lower.starts_with("gpt-4.1") {
        return 1_047_576;
    }
    if model_lower.starts_with("gpt-4o") || model_lower.starts_with("gpt-4") {
        return 128_000;
    }

    // Claude models
    if model_lower.contains("claude-opus") || model_lower.contains("opus") {
        return 200_000;
    }
    if model_lower.contains("claude-sonnet") || model_lower.contains("sonnet") {
        return 200_000;
    }
    if model_lower.contains("claude-haiku") || model_lower.contains("haiku") {
        return 200_000;
    }

    // Gemini models
    if model_lower.contains("gemini-2.5-pro") || model_lower.contains("gemini-pro-2.5") {
        return 1_000_000;
    }
    if model_lower.contains("gemini-2.5-flash") || model_lower.contains("gemini-flash-2.5") {
        return 1_000_000;
    }
    if model_lower.contains("gemini-2.0") || model_lower.contains("gemini-1.5") {
        return 1_000_000;
    }
    if model_lower.contains("gemini-3") {
        return 1_000_000; // Gemini 3 family
    }

    // Default for unknown models
    128_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::protocol::TokenUsage;

    fn make_metrics(input: u64, output: u64, turns: u32) -> SessionMetrics {
        let mut m = SessionMetrics::default();
        for _ in 0..turns {
            m.record_turn(&TokenUsage {
                input_tokens: input / turns as u64,
                cached_input_tokens: 0,
                output_tokens: output / turns as u64,
                reasoning_output_tokens: 0,
                total_tokens: (input + output) / turns as u64,
            });
        }
        m
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(TokenMetricsWidget::format_tokens(500), "500");
        assert_eq!(TokenMetricsWidget::format_tokens(1_500), "1.5k");
        assert_eq!(TokenMetricsWidget::format_tokens(12_500), "12.5k");
        assert_eq!(TokenMetricsWidget::format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_utilization_colors() {
        let widget = TokenMetricsWidget::new(0, 0, 0, 0, 0.5, None);
        assert_eq!(widget.utilization_color(), Color::Green);

        let widget = TokenMetricsWidget::new(0, 0, 0, 0, 0.65, None);
        assert_eq!(widget.utilization_color(), Color::Cyan);

        let widget = TokenMetricsWidget::new(0, 0, 0, 0, 0.85, None);
        assert_eq!(widget.utilization_color(), Color::Yellow);

        let widget = TokenMetricsWidget::new(0, 0, 0, 0, 0.95, None);
        assert_eq!(widget.utilization_color(), Color::Red);
    }

    #[test]
    fn test_from_session_metrics() {
        let metrics = make_metrics(9_999, 5_001, 3); // Divisible by 3 to avoid rounding
        let widget = TokenMetricsWidget::from_session_metrics(
            &metrics, 200_000, // 200k context window
            "haiku",
        );

        assert_eq!(widget.total_input, 9_999);
        assert_eq!(widget.total_output, 5_001);
        assert_eq!(widget.turn_count, 3);
        assert!(widget.context_utilization < 0.1); // 15k / 200k = 7.5%
        assert!(widget.estimated_cost_usd.is_some());
    }

    #[test]
    fn test_warning_thresholds() {
        let widget = TokenMetricsWidget::new(0, 0, 0, 0, 0.85, None);
        assert!(widget.is_warning());
        assert!(!widget.is_critical());
        assert!(widget.warning_message().is_some());

        let widget = TokenMetricsWidget::new(0, 0, 0, 0, 0.95, None);
        assert!(widget.is_warning());
        assert!(widget.is_critical());
        assert!(widget.warning_message().unwrap().contains("compaction"));
    }

    #[test]
    fn test_model_context_windows() {
        // OpenAI
        assert_eq!(model_context_window("gpt-5"), 272_000);
        assert_eq!(model_context_window("gpt-5-codex"), 272_000);
        assert_eq!(model_context_window("o3"), 200_000);
        assert_eq!(model_context_window("gpt-4o"), 128_000);

        // Claude
        assert_eq!(model_context_window("claude-opus-4.5"), 200_000);
        assert_eq!(model_context_window("claude-sonnet-4"), 200_000);
        assert_eq!(model_context_window("haiku"), 200_000);

        // Gemini
        assert_eq!(model_context_window("gemini-2.5-pro"), 1_000_000);
        assert_eq!(model_context_window("gemini-2.5-flash"), 1_000_000);
        assert_eq!(model_context_window("gemini-3-pro"), 1_000_000);

        // Unknown
        assert_eq!(model_context_window("unknown-model"), 128_000);
    }

    #[test]
    fn test_render_formats() {
        let widget = TokenMetricsWidget::new(12_500, 3_200, 5, 4_000, 0.45, Some(0.12));

        let full = widget.render_full();
        let full_str: String = full.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(full_str.contains("12.5k"));
        assert!(full_str.contains("3.2k"));
        assert!(full_str.contains("Turn 5"));
        assert!(full_str.contains("45%"));
        assert!(full_str.contains("$0.12"));

        let compact = widget.render_compact();
        let compact_str: String = compact.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(compact_str.contains("12.5k"));
        assert!(compact_str.contains("T5"));
    }

    #[test]
    fn test_cost_calculation() {
        let metrics = make_metrics(100_000, 50_000, 5);

        // Haiku pricing: $1/M input, $5/M output
        let widget = TokenMetricsWidget::from_session_metrics(&metrics, 200_000, "haiku");

        // Expected: (100k/1M * 1.0) + (50k/1M * 5.0) = 0.10 + 0.25 = 0.35
        let cost = widget.estimated_cost_usd.unwrap();
        assert!((cost - 0.35).abs() < 0.01);
    }
}
