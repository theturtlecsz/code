# P6-SYNC Continuation: Phases 5-6 + Enhancements

_Generated: 2025-11-29_
_Commit: fe7db9ac1 (P6-SYNC Phases 2-4 complete)_

## Session Context

### Completed Infrastructure
- **Phase 2**: SessionMetrics - Token tracking with sliding window estimation
- **Phase 3**: Fault Injection - Feature-gated dev-faults framework
- **Phase 4**: Branch-Aware Resume - Pipeline run isolation via branch_id

### Remaining Work
1. **Phase 6**: TokenMetrics UI Integration (1-2h)
2. **Phase 5**: Device Code Auth for OpenAI + Google (2-3h) - **CONFIRMED NEEDED**
3. **Enhancements**: Context warnings, cost estimation, per-stage breakdown

### Why Phase 5 is Confirmed Necessary

**User's confirmed scenario**: Regularly uses TUI via SSH/headless environments where browser-based OAuth (PKCE) is impossible.

**Device Code Flow solves this**:
- SSH into server → TUI shows "Visit URL, enter code ABCD-1234"
- User opens URL on phone/laptop → enters code → logs in
- TUI polls in background → receives token automatically

**Providers confirmed needed**:
- OpenAI (ChatGPT) - User needs OAuth, not just API key
- Google (Gemini) - User needs OAuth, not just API key

This is a legitimate pain point, not speculative infrastructure.

---

## Startup Commands

```bash
# Verify baseline
cd ~/code/codex-rs
cargo test -p codex-tui --test write_path_cutover -- branch
cargo test -p codex-spec-kit --features dev-faults -- faults
cargo build -p codex-tui

# Check SessionMetrics exists
grep -n "pub struct SessionMetrics" tui/src/chatwidget/spec_kit/session_metrics.rs
```

---

## Phase 6: TokenMetrics UI Integration (Priority 1)

### Goal
Wire SessionMetrics to TUI status bar for real-time token tracking with predictive estimates.

### 6.1 Create TokenMetrics Display Widget
Location: `codex-rs/tui/src/token_metrics_widget.rs` (NEW)

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::chatwidget::spec_kit::session_metrics::SessionMetrics;

/// Token metrics display for status bar
pub struct TokenMetricsWidget {
    pub total_input: u64,
    pub total_output: u64,
    pub turn_count: u32,
    pub estimated_next: u64,
    pub context_utilization: f64,  // 0.0 - 1.0
    pub estimated_cost_usd: Option<f64>,
}

impl TokenMetricsWidget {
    /// Create from SessionMetrics with model context window
    pub fn from_session_metrics(
        metrics: &SessionMetrics,
        context_window: u64,
        cost_per_1k_input: Option<f64>,
        cost_per_1k_output: Option<f64>,
    ) -> Self {
        let total = metrics.running_total();
        let utilization = if context_window > 0 {
            metrics.blended_total() as f64 / context_window as f64
        } else {
            0.0
        };

        let cost = match (cost_per_1k_input, cost_per_1k_output) {
            (Some(in_cost), Some(out_cost)) => {
                Some(
                    (total.input_tokens as f64 / 1000.0 * in_cost)
                    + (total.output_tokens as f64 / 1000.0 * out_cost)
                )
            }
            _ => None,
        };

        Self {
            total_input: total.input_tokens,
            total_output: total.output_tokens,
            turn_count: metrics.turn_count(),
            estimated_next: metrics.estimated_next_prompt_tokens(),
            context_utilization: utilization,
            estimated_cost_usd: cost,
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
            Color::Red  // Critical
        } else if self.context_utilization > 0.8 {
            Color::Yellow  // Warning
        } else if self.context_utilization > 0.6 {
            Color::Cyan  // Moderate
        } else {
            Color::Green  // Healthy
        }
    }

    /// Render full format: "Tokens: 12.5k in / 3.2k out | Turn 5 | Est: ~4k | Ctx: 45% | $0.12"
    pub fn render_full(&self) -> Line<'static> {
        let mut spans = vec![
            Span::raw("Tokens: "),
            Span::styled(Self::format_tokens(self.total_input), Style::default().bold()),
            Span::raw(" in / "),
            Span::styled(Self::format_tokens(self.total_output), Style::default().bold()),
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
            Span::styled(Self::format_tokens(self.total_input), Style::default().bold()),
            Span::raw("/"),
            Span::styled(Self::format_tokens(self.total_output), Style::default().bold()),
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
        let widget = TokenMetricsWidget {
            total_input: 0, total_output: 0, turn_count: 0,
            estimated_next: 0, context_utilization: 0.5, estimated_cost_usd: None,
        };
        assert_eq!(widget.utilization_color(), Color::Green);

        let widget = TokenMetricsWidget { context_utilization: 0.85, ..widget };
        assert_eq!(widget.utilization_color(), Color::Yellow);

        let widget = TokenMetricsWidget { context_utilization: 0.95, ..widget };
        assert_eq!(widget.utilization_color(), Color::Red);
    }

    #[test]
    fn test_from_session_metrics() {
        let metrics = make_metrics(10_000, 5_000, 3);
        let widget = TokenMetricsWidget::from_session_metrics(
            &metrics,
            200_000,  // 200k context window
            Some(0.003),  // $3/1M input
            Some(0.015),  // $15/1M output
        );

        assert_eq!(widget.total_input, 10_000);
        assert_eq!(widget.total_output, 5_000);
        assert_eq!(widget.turn_count, 3);
        assert!(widget.context_utilization < 0.1);  // 15k / 200k = 7.5%
        assert!(widget.estimated_cost_usd.is_some());
    }

    #[test]
    fn test_warning_thresholds() {
        let widget = TokenMetricsWidget {
            total_input: 0, total_output: 0, turn_count: 0,
            estimated_next: 0, context_utilization: 0.85, estimated_cost_usd: None,
        };
        assert!(widget.is_warning());
        assert!(!widget.is_critical());

        let widget = TokenMetricsWidget { context_utilization: 0.95, ..widget };
        assert!(widget.is_warning());
        assert!(widget.is_critical());
    }
}
```

### 6.2 Export from lib.rs
Location: `codex-rs/tui/src/lib.rs`

Add:
```rust
mod token_metrics_widget;
pub use token_metrics_widget::TokenMetricsWidget;
```

### 6.3 Model Context Window Config
Location: `codex-rs/core/src/config_types.rs` or similar

Add context window sizes per model:
```rust
pub fn model_context_window(model_id: &str) -> u64 {
    match model_id {
        m if m.contains("gpt-5") => 128_000,
        m if m.contains("claude-opus") => 200_000,
        m if m.contains("claude-sonnet") => 200_000,
        m if m.contains("gemini-2.5-pro") => 1_000_000,
        m if m.contains("gemini-2.5-flash") => 1_000_000,
        _ => 128_000,  // Default
    }
}

pub fn model_pricing(model_id: &str) -> (f64, f64) {
    // Returns (input_per_1k, output_per_1k) in USD
    match model_id {
        m if m.contains("gpt-5") && m.contains("codex") => (0.015, 0.060),
        m if m.contains("gpt-5") => (0.010, 0.030),
        m if m.contains("claude-opus") => (0.015, 0.075),
        m if m.contains("claude-sonnet") => (0.003, 0.015),
        m if m.contains("claude-haiku") => (0.00025, 0.00125),
        m if m.contains("gemini-2.5-pro") => (0.00125, 0.005),
        m if m.contains("gemini-2.5-flash") => (0.000075, 0.0003),
        _ => (0.003, 0.015),  // Default (Sonnet-ish)
    }
}
```

### 6.4 Wire to ChatWidget
Location: `codex-rs/tui/src/chatwidget/` (find render method)

When rendering status area:
```rust
if let Some(state) = &self.spec_auto_state {
    if let Some(metrics) = &state.session_metrics {
        let model_id = state.current_model.as_deref().unwrap_or("unknown");
        let context_window = model_context_window(model_id);
        let (in_price, out_price) = model_pricing(model_id);

        let widget = TokenMetricsWidget::from_session_metrics(
            metrics,
            context_window,
            Some(in_price),
            Some(out_price),
        );

        // Render warning if critical
        if widget.is_critical() {
            // Show warning message
        }

        widget.render(metrics_area, buf);
    }
}
```

### 6.5 Enhancement: Per-Stage Token Breakdown
Location: `codex-rs/tui/src/chatwidget/spec_kit/state.rs`

Add to SpecAutoState:
```rust
/// Token usage per stage
pub stage_metrics: HashMap<SpecStage, SessionMetrics>,
```

Update `record_agent_costs()` to track per-stage:
```rust
pub fn record_agent_costs(&mut self, stage: SpecStage, usage: &TokenUsage) {
    // Update global metrics
    self.session_metrics.record_turn(usage);

    // Update per-stage metrics
    self.stage_metrics
        .entry(stage)
        .or_insert_with(SessionMetrics::default)
        .record_turn(usage);
}
```

### Acceptance Criteria (Phase 6)
- [ ] TokenMetricsWidget created with full/compact rendering
- [ ] Context window config per model
- [ ] Pricing config per model
- [ ] Integrated into status bar
- [ ] Context utilization warnings (>80% yellow, >90% red)
- [ ] Cost estimation displayed
- [ ] Per-stage breakdown tracked
- [ ] 5+ unit tests for widget

---

## Phase 5: Device Code Auth for OpenAI + Google (Priority 2)

### Goal
Implement OAuth device code flow for native authentication without browser redirect.

### 5.1 Device Code Auth Trait
Location: `codex-rs/login/src/device_code.rs` (NEW)

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Device code response from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    /// Code for the device (used internally for polling)
    pub device_code: String,
    /// Code for the user to enter at verification URL
    pub user_code: String,
    /// URL where user enters the code
    pub verification_uri: String,
    /// Optional direct URL with code embedded
    pub verification_uri_complete: Option<String>,
    /// Seconds until the code expires
    pub expires_in: u64,
    /// Seconds to wait between polls
    pub interval: u64,
}

/// Token received after successful authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Errors during device code flow
#[derive(Debug, thiserror::Error)]
pub enum DeviceCodeError {
    #[error("Authorization pending - user hasn't completed login")]
    AuthorizationPending,
    #[error("Slow down - polling too fast")]
    SlowDown,
    #[error("Access denied by user")]
    AccessDenied,
    #[error("Device code expired")]
    ExpiredToken,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Provider-agnostic device code authentication
#[async_trait]
pub trait DeviceCodeAuth: Send + Sync {
    /// Provider name for display
    fn provider_name(&self) -> &'static str;

    /// Request device code from provider
    async fn request_device_code(&self) -> Result<DeviceCodeResponse, DeviceCodeError>;

    /// Poll for token (call after user enters code)
    async fn poll_for_token(
        &self,
        device_code: &str
    ) -> Result<AuthToken, DeviceCodeError>;

    /// Refresh an expired token
    async fn refresh_token(
        &self,
        refresh_token: &str
    ) -> Result<AuthToken, DeviceCodeError>;

    /// Recommended poll interval
    fn poll_interval(&self) -> Duration {
        Duration::from_secs(5)
    }
}
```

### 5.2 OpenAI Device Code Implementation
Location: `codex-rs/login/src/providers/openai.rs` (NEW)

```rust
use super::super::device_code::*;
use async_trait::async_trait;
use reqwest::Client;

const OPENAI_DEVICE_AUTH_URL: &str = "https://auth.openai.com/oauth/device/code";
const OPENAI_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_CLIENT_ID: &str = "app-codex-cli";  // Use appropriate client ID

pub struct OpenAIDeviceAuth {
    client: Client,
    client_id: String,
}

impl OpenAIDeviceAuth {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: OPENAI_CLIENT_ID.to_string(),
        }
    }

    pub fn with_client_id(client_id: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
        }
    }
}

#[async_trait]
impl DeviceCodeAuth for OpenAIDeviceAuth {
    fn provider_name(&self) -> &'static str {
        "OpenAI"
    }

    async fn request_device_code(&self) -> Result<DeviceCodeResponse, DeviceCodeError> {
        let response = self.client
            .post(OPENAI_DEVICE_AUTH_URL)
            .form(&[
                ("client_id", &self.client_id),
                ("scope", &"openid profile email".to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DeviceCodeError::InvalidResponse(
                format!("HTTP {}", response.status())
            ));
        }

        response.json().await.map_err(|e|
            DeviceCodeError::InvalidResponse(e.to_string())
        )
    }

    async fn poll_for_token(&self, device_code: &str) -> Result<AuthToken, DeviceCodeError> {
        let response = self.client
            .post(OPENAI_TOKEN_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", device_code),
                ("client_id", &self.client_id),
            ])
            .send()
            .await?;

        let status = response.status();
        let body: serde_json::Value = response.json().await?;

        if status.is_success() {
            return serde_json::from_value(body).map_err(|e|
                DeviceCodeError::InvalidResponse(e.to_string())
            );
        }

        // Handle OAuth errors
        match body.get("error").and_then(|e| e.as_str()) {
            Some("authorization_pending") => Err(DeviceCodeError::AuthorizationPending),
            Some("slow_down") => Err(DeviceCodeError::SlowDown),
            Some("access_denied") => Err(DeviceCodeError::AccessDenied),
            Some("expired_token") => Err(DeviceCodeError::ExpiredToken),
            _ => Err(DeviceCodeError::InvalidResponse(
                body.to_string()
            )),
        }
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, DeviceCodeError> {
        let response = self.client
            .post(OPENAI_TOKEN_URL)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", &self.client_id),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DeviceCodeError::InvalidResponse(
                format!("HTTP {}", response.status())
            ));
        }

        response.json().await.map_err(|e|
            DeviceCodeError::InvalidResponse(e.to_string())
        )
    }
}
```

### 5.3 Google Device Code Implementation
Location: `codex-rs/login/src/providers/google.rs` (NEW)

```rust
use super::super::device_code::*;
use async_trait::async_trait;
use reqwest::Client;

const GOOGLE_DEVICE_AUTH_URL: &str = "https://oauth2.googleapis.com/device/code";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub struct GoogleDeviceAuth {
    client: Client,
    client_id: String,
    client_secret: Option<String>,  // Some flows require secret
}

impl GoogleDeviceAuth {
    pub fn new(client_id: String, client_secret: Option<String>) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
        }
    }
}

#[async_trait]
impl DeviceCodeAuth for GoogleDeviceAuth {
    fn provider_name(&self) -> &'static str {
        "Google"
    }

    async fn request_device_code(&self) -> Result<DeviceCodeResponse, DeviceCodeError> {
        let response = self.client
            .post(GOOGLE_DEVICE_AUTH_URL)
            .form(&[
                ("client_id", &self.client_id),
                ("scope", &"https://www.googleapis.com/auth/generative-language".to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DeviceCodeError::InvalidResponse(
                format!("HTTP {}", response.status())
            ));
        }

        response.json().await.map_err(|e|
            DeviceCodeError::InvalidResponse(e.to_string())
        )
    }

    async fn poll_for_token(&self, device_code: &str) -> Result<AuthToken, DeviceCodeError> {
        let mut form = vec![
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
            ("client_id", self.client_id.as_str()),
        ];

        // Google requires client_secret for some app types
        let secret_ref;
        if let Some(secret) = &self.client_secret {
            secret_ref = secret.as_str();
            form.push(("client_secret", secret_ref));
        }

        let response = self.client
            .post(GOOGLE_TOKEN_URL)
            .form(&form)
            .send()
            .await?;

        let status = response.status();
        let body: serde_json::Value = response.json().await?;

        if status.is_success() {
            return serde_json::from_value(body).map_err(|e|
                DeviceCodeError::InvalidResponse(e.to_string())
            );
        }

        match body.get("error").and_then(|e| e.as_str()) {
            Some("authorization_pending") => Err(DeviceCodeError::AuthorizationPending),
            Some("slow_down") => Err(DeviceCodeError::SlowDown),
            Some("access_denied") => Err(DeviceCodeError::AccessDenied),
            Some("expired_token") => Err(DeviceCodeError::ExpiredToken),
            _ => Err(DeviceCodeError::InvalidResponse(body.to_string())),
        }
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, DeviceCodeError> {
        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
        ];

        let secret_ref;
        if let Some(secret) = &self.client_secret {
            secret_ref = secret.as_str();
            form.push(("client_secret", secret_ref));
        }

        let response = self.client
            .post(GOOGLE_TOKEN_URL)
            .form(&form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DeviceCodeError::InvalidResponse(
                format!("HTTP {}", response.status())
            ));
        }

        response.json().await.map_err(|e|
            DeviceCodeError::InvalidResponse(e.to_string())
        )
    }
}
```

### 5.4 Provider Registry
Location: `codex-rs/login/src/providers/mod.rs` (NEW)

```rust
mod openai;
mod google;

pub use openai::OpenAIDeviceAuth;
pub use google::GoogleDeviceAuth;

use super::device_code::DeviceCodeAuth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthProvider {
    OpenAI,
    Google,
}

impl AuthProvider {
    pub fn from_model_id(model_id: &str) -> Option<Self> {
        if model_id.contains("gpt") || model_id.contains("o1") {
            Some(Self::OpenAI)
        } else if model_id.contains("gemini") {
            Some(Self::Google)
        } else {
            None  // Claude uses API key, no device code
        }
    }
}

pub fn get_auth_handler(provider: AuthProvider) -> Box<dyn DeviceCodeAuth> {
    match provider {
        AuthProvider::OpenAI => Box::new(OpenAIDeviceAuth::new()),
        AuthProvider::Google => Box::new(GoogleDeviceAuth::new(
            std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            std::env::var("GOOGLE_CLIENT_SECRET").ok(),
        )),
    }
}
```

### 5.5 Token Storage
Location: `codex-rs/login/src/token_store.rs` (NEW)

```rust
use crate::device_code::AuthToken;
use crate::providers::AuthProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenStore {
    tokens: HashMap<String, StoredToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub provider: String,
}

impl TokenStore {
    pub fn load(path: &PathBuf) -> Result<Self, std::io::Error> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e|
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        )
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }

    pub fn store_token(&mut self, provider: AuthProvider, token: AuthToken) {
        let expires_at = token.expires_in.map(|secs|
            chrono::Utc::now() + chrono::Duration::seconds(secs as i64)
        );

        self.tokens.insert(
            format!("{:?}", provider),
            StoredToken {
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                expires_at,
                provider: format!("{:?}", provider),
            },
        );
    }

    pub fn get_token(&self, provider: AuthProvider) -> Option<&StoredToken> {
        self.tokens.get(&format!("{:?}", provider))
    }

    pub fn is_expired(&self, provider: AuthProvider) -> bool {
        self.get_token(provider)
            .and_then(|t| t.expires_at)
            .map(|exp| exp < chrono::Utc::now())
            .unwrap_or(true)
    }

    pub fn clear(&mut self, provider: AuthProvider) {
        self.tokens.remove(&format!("{:?}", provider));
    }
}
```

### 5.6 CLI Integration
Location: Update `/model` command handler

```rust
// When user selects a model requiring device code auth:
async fn handle_model_auth(model_id: &str) -> Result<(), Error> {
    let provider = AuthProvider::from_model_id(model_id);

    if let Some(provider) = provider {
        let store_path = get_token_store_path();
        let mut store = TokenStore::load(&store_path)?;

        // Check if we have a valid token
        if !store.is_expired(provider) {
            return Ok(());  // Already authenticated
        }

        // Try refresh first
        if let Some(token) = store.get_token(provider) {
            if let Some(refresh) = &token.refresh_token {
                let handler = get_auth_handler(provider);
                match handler.refresh_token(refresh).await {
                    Ok(new_token) => {
                        store.store_token(provider, new_token);
                        store.save(&store_path)?;
                        return Ok(());
                    }
                    Err(_) => {} // Fall through to device code flow
                }
            }
        }

        // Device code flow
        let handler = get_auth_handler(provider);
        let device_code = handler.request_device_code().await?;

        // Display to user
        println!("\n🔐 Authentication required for {}", handler.provider_name());
        println!("   Visit: {}", device_code.verification_uri);
        println!("   Enter code: {}", device_code.user_code);
        println!("\n   Waiting for authorization...\n");

        // Poll for token
        let interval = handler.poll_interval();
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_secs(device_code.expires_in);

        while std::time::Instant::now() < deadline {
            tokio::time::sleep(interval).await;

            match handler.poll_for_token(&device_code.device_code).await {
                Ok(token) => {
                    store.store_token(provider, token);
                    store.save(&store_path)?;
                    println!("   ✅ Authenticated successfully!\n");
                    return Ok(());
                }
                Err(DeviceCodeError::AuthorizationPending) => continue,
                Err(DeviceCodeError::SlowDown) => {
                    tokio::time::sleep(interval).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(DeviceCodeError::ExpiredToken.into())
    } else {
        Ok(())  // No device code auth needed (e.g., Claude uses API key)
    }
}
```

### Acceptance Criteria (Phase 5)
- [ ] DeviceCodeAuth trait defined
- [ ] OpenAI implementation complete
- [ ] Google implementation complete
- [ ] Provider registry working
- [ ] Token storage with persistence
- [ ] Token refresh on expiry
- [ ] CLI integration for /model command
- [ ] Tests for each provider
- [ ] Error handling for all OAuth errors

---

## Testing Commands

```bash
# Phase 6 tests
cargo test -p codex-tui -- token_metrics

# Phase 5 tests
cargo test -p codex-login -- device_code
cargo test -p codex-login -- providers

# Full validation
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

---

## Commit Strategy

After each phase:

```bash
# Phase 6 commit
git add -A && git commit -m "feat(tui): Add TokenMetrics UI with context warnings and cost estimation

- TokenMetricsWidget with full/compact rendering
- Context utilization color coding (green/yellow/red)
- Per-model context window and pricing config
- Per-stage token breakdown tracking
- Cost estimation display

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"

# Phase 5 commit
git add -A && git commit -m "feat(login): Add device code OAuth for OpenAI and Google

- DeviceCodeAuth trait for provider-agnostic flow
- OpenAI device code implementation
- Google device code implementation
- TokenStore for secure token persistence
- Auto-refresh on token expiry
- CLI integration for /model command

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Priority Summary

1. **Phase 6 FIRST** (~1-2h) - Smaller scope, builds on existing SessionMetrics
2. **Phase 5 SECOND** (~2-3h) - Larger scope, requires OAuth integration testing
3. **All Enhancements included** - Context warnings, cost estimation, per-stage breakdown

---

## Files to Create/Modify

### Phase 6 (5 files)
- `codex-rs/tui/src/token_metrics_widget.rs` (NEW)
- `codex-rs/tui/src/lib.rs` (export)
- `codex-rs/core/src/config_types.rs` (model config)
- `codex-rs/tui/src/chatwidget/spec_kit/state.rs` (per-stage tracking)
- `codex-rs/tui/src/chatwidget/` (integration)

### Phase 5 (6 files)
- `codex-rs/login/src/device_code.rs` (NEW)
- `codex-rs/login/src/providers/mod.rs` (NEW)
- `codex-rs/login/src/providers/openai.rs` (NEW)
- `codex-rs/login/src/providers/google.rs` (NEW)
- `codex-rs/login/src/token_store.rs` (NEW)
- `codex-rs/login/src/lib.rs` (exports)
