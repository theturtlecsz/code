//! Budget tracking with hourly granularity.
//!
//! Stores usage in .codex/architect/usage.json with per-hour breakdown
//! and 7-day history for trend analysis.

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Default daily limit for NotebookLM Plus tier.
const DEFAULT_DAILY_LIMIT: u32 = 500;

/// Warning threshold (80% of daily limit).
const DEFAULT_WARN_THRESHOLD: u32 = 400;

/// Maximum history days to retain.
const MAX_HISTORY_DAYS: usize = 7;

/// Usage data persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    /// Current date (YYYY-MM-DD in local time).
    pub date: String,
    /// Queries per hour (0-23 keys).
    pub hourly: HashMap<u8, u32>,
    /// Total queries today.
    pub total: u32,
    /// 7-day history for trends.
    pub history: Vec<DailyUsage>,
}

impl Default for UsageData {
    fn default() -> Self {
        Self {
            date: today_str(),
            hourly: HashMap::new(),
            total: 0,
            history: Vec::new(),
        }
    }
}

/// Historical daily usage record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String,
    pub total: u32,
}

/// Budget tracker with persistence.
pub struct BudgetTracker {
    usage: UsageData,
    file_path: PathBuf,
    daily_limit: u32,
    warn_threshold: u32,
}

impl BudgetTracker {
    /// Load budget tracker from vault path.
    pub fn load(vault_path: &Path) -> Result<Self> {
        let file_path = vault_path.join("usage.json");
        let usage = if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)
                .context("Failed to read usage.json")?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            UsageData::default()
        };

        let mut tracker = Self {
            usage,
            file_path,
            daily_limit: DEFAULT_DAILY_LIMIT,
            warn_threshold: DEFAULT_WARN_THRESHOLD,
        };

        // Roll over to new day if needed
        tracker.maybe_rollover();

        Ok(tracker)
    }

    /// Check if today changed and roll over usage if needed.
    fn maybe_rollover(&mut self) {
        let today = today_str();
        if self.usage.date != today {
            // Archive yesterday's usage
            if self.usage.total > 0 {
                self.usage.history.push(DailyUsage {
                    date: self.usage.date.clone(),
                    total: self.usage.total,
                });
                // Trim history to max days
                while self.usage.history.len() > MAX_HISTORY_DAYS {
                    self.usage.history.remove(0);
                }
            }
            // Reset for new day
            self.usage.date = today;
            self.usage.hourly.clear();
            self.usage.total = 0;
        }
    }

    /// Save usage data to disk.
    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.usage)?;
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.file_path, content)?;
        Ok(())
    }

    /// Record a query (increments counter, persists to disk).
    pub fn record_query(&mut self) -> Result<()> {
        self.maybe_rollover();

        let hour = Local::now().hour() as u8;
        *self.usage.hourly.entry(hour).or_insert(0) += 1;
        self.usage.total += 1;

        self.save()
    }

    /// Get remaining queries for today.
    pub fn remaining(&self) -> u32 {
        self.daily_limit.saturating_sub(self.usage.total)
    }

    /// Check if budget is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.usage.total >= self.daily_limit
    }

    /// Check if we need confirmation (past warning threshold).
    pub fn needs_confirmation(&self) -> bool {
        self.usage.total >= self.warn_threshold
    }

    /// Get current usage count.
    pub fn used(&self) -> u32 {
        self.usage.total
    }

    /// Get daily limit.
    pub fn limit(&self) -> u32 {
        self.daily_limit
    }

    /// Format status line.
    pub fn format_status(&self) -> String {
        let pct = (self.usage.total as f64 / self.daily_limit as f64) * 100.0;
        format!(
            "Used: {}/{} ({:.1}%)",
            self.usage.total, self.daily_limit, pct
        )
    }

    /// Get time until midnight UTC reset.
    pub fn time_until_reset(&self) -> String {
        let now = Utc::now();
        let tomorrow = (now + chrono::Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let tomorrow_utc: DateTime<Utc> = DateTime::from_naive_utc_and_offset(tomorrow, Utc);
        let remaining = tomorrow_utc.signed_duration_since(now);

        let hours = remaining.num_hours();
        let minutes = remaining.num_minutes() % 60;
        format!("{}h {}m", hours, minutes)
    }

    /// Format hourly breakdown for display.
    pub fn hourly_breakdown(&self) -> String {
        if self.usage.hourly.is_empty() {
            return "No queries today".to_string();
        }

        let mut lines = vec![format!(
            "Hourly breakdown for {} (local time):",
            self.usage.date
        )];

        // Find min/max hours with data
        let mut hours: Vec<u8> = self.usage.hourly.keys().copied().collect();
        hours.sort();

        for hour in hours {
            let count = self.usage.hourly.get(&hour).unwrap_or(&0);
            let bar = "█".repeat((*count).min(20) as usize);
            lines.push(format!("  {:02}:00  {:>3}  {}", hour, count, bar));
        }

        lines.join("\n")
    }

    /// Get usage history for trend display.
    pub fn history_summary(&self) -> String {
        if self.usage.history.is_empty() {
            return "No historical data".to_string();
        }

        let mut lines = vec!["7-day history:".to_string()];
        for day in &self.usage.history {
            let pct = (day.total as f64 / self.daily_limit as f64) * 100.0;
            lines.push(format!("  {}  {:>3} ({:.0}%)", day.date, day.total, pct));
        }

        lines.join("\n")
    }
}

/// Get today's date string in YYYY-MM-DD format (local time).
fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_budget_tracker_new() {
        let tmp = TempDir::new().unwrap();
        let tracker = BudgetTracker::load(tmp.path()).unwrap();

        assert_eq!(tracker.used(), 0);
        assert_eq!(tracker.remaining(), 500);
        assert!(!tracker.is_exhausted());
        assert!(!tracker.needs_confirmation());
    }

    #[test]
    fn test_record_query() {
        let tmp = TempDir::new().unwrap();
        let mut tracker = BudgetTracker::load(tmp.path()).unwrap();

        tracker.record_query().unwrap();
        assert_eq!(tracker.used(), 1);
        assert_eq!(tracker.remaining(), 499);

        // Verify persistence
        let tracker2 = BudgetTracker::load(tmp.path()).unwrap();
        assert_eq!(tracker2.used(), 1);
    }

    #[test]
    fn test_warning_threshold() {
        let tmp = TempDir::new().unwrap();
        let mut tracker = BudgetTracker::load(tmp.path()).unwrap();

        // Set usage just below threshold
        tracker.usage.total = 399;
        assert!(!tracker.needs_confirmation());

        // At threshold
        tracker.usage.total = 400;
        assert!(tracker.needs_confirmation());
    }

    #[test]
    fn test_exhausted() {
        let tmp = TempDir::new().unwrap();
        let mut tracker = BudgetTracker::load(tmp.path()).unwrap();

        tracker.usage.total = 499;
        assert!(!tracker.is_exhausted());

        tracker.usage.total = 500;
        assert!(tracker.is_exhausted());
    }

    #[test]
    fn test_format_status() {
        let tmp = TempDir::new().unwrap();
        let mut tracker = BudgetTracker::load(tmp.path()).unwrap();

        tracker.usage.total = 25;
        assert_eq!(tracker.format_status(), "Used: 25/500 (5.0%)");
    }
}
