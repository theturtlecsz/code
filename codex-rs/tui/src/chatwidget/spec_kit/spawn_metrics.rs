//! Agent spawn metrics tracking (SPEC-933 Component 3)
//!
//! Tracks spawn performance for parallel agent initialization:
//! - Per-agent spawn duration
//! - Total spawn time (baseline comparison)
//! - Success/failure rates
//! - p95 spawn latency
//!
//! Target: 150ms → 50ms (3× speedup) via parallel initialization

use std::sync::Mutex;
use std::time::Duration;
use once_cell::sync::Lazy;

/// Individual agent spawn metric
#[derive(Debug, Clone)]
pub struct AgentSpawnMetric {
    pub agent_name: String,
    pub spawn_duration_ms: u64,
    pub success: bool,
    pub timestamp: std::time::SystemTime,
}

/// Aggregate spawn metrics for a batch
#[derive(Debug, Clone)]
pub struct BatchSpawnMetrics {
    pub total_agents: usize,
    pub successful_agents: usize,
    pub total_duration_ms: u64,
    pub avg_spawn_duration_ms: u64,
    pub max_spawn_duration_ms: u64,
    pub min_spawn_duration_ms: u64,
    pub timestamp: std::time::SystemTime,
}

/// Global metrics storage
static SPAWN_METRICS: Lazy<Mutex<Vec<AgentSpawnMetric>>> = Lazy::new(|| Mutex::new(Vec::new()));
static BATCH_METRICS: Lazy<Mutex<Vec<BatchSpawnMetrics>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Record individual agent spawn metric
pub fn record_agent_spawn(agent_name: &str, duration: Duration, success: bool) {
    let metric = AgentSpawnMetric {
        agent_name: agent_name.to_string(),
        spawn_duration_ms: duration.as_millis() as u64,
        success,
        timestamp: std::time::SystemTime::now(),
    };

    if let Ok(mut metrics) = SPAWN_METRICS.lock() {
        metrics.push(metric.clone());

        // Keep last 1000 metrics (rolling window)
        if metrics.len() > 1000 {
            metrics.drain(0..100);
        }
    }

    tracing::info!(
        "📊 Spawn metric: {} took {:?} (success: {})",
        agent_name,
        duration,
        success
    );
}

/// Record batch spawn metrics
pub fn record_batch_spawn(
    total_agents: usize,
    successful_agents: usize,
    total_duration: Duration,
    individual_durations: &[Duration],
) {
    let total_duration_ms = total_duration.as_millis() as u64;

    let avg_spawn_duration_ms = if !individual_durations.is_empty() {
        individual_durations.iter().map(|d| d.as_millis() as u64).sum::<u64>()
            / individual_durations.len() as u64
    } else {
        0
    };

    let max_spawn_duration_ms = individual_durations
        .iter()
        .map(|d| d.as_millis() as u64)
        .max()
        .unwrap_or(0);

    let min_spawn_duration_ms = individual_durations
        .iter()
        .map(|d| d.as_millis() as u64)
        .min()
        .unwrap_or(0);

    let batch_metric = BatchSpawnMetrics {
        total_agents,
        successful_agents,
        total_duration_ms,
        avg_spawn_duration_ms,
        max_spawn_duration_ms,
        min_spawn_duration_ms,
        timestamp: std::time::SystemTime::now(),
    };

    if let Ok(mut metrics) = BATCH_METRICS.lock() {
        metrics.push(batch_metric.clone());

        // Keep last 100 batch metrics
        if metrics.len() > 100 {
            metrics.drain(0..10);
        }
    }

    tracing::warn!(
        "📊 BATCH SPAWN METRICS: {} agents, total={:?}, avg={:?}ms, min={:?}ms, max={:?}ms, success={}/{}",
        total_agents,
        Duration::from_millis(total_duration_ms),
        avg_spawn_duration_ms,
        min_spawn_duration_ms,
        max_spawn_duration_ms,
        successful_agents,
        total_agents
    );
}

/// Calculate p95 spawn latency from recent metrics
pub fn calculate_p95_spawn_latency() -> Option<Duration> {
    let metrics = SPAWN_METRICS.lock().ok()?;

    if metrics.len() < 20 {
        return None; // Need at least 20 samples for p95
    }

    let mut durations: Vec<u64> = metrics.iter().map(|m| m.spawn_duration_ms).collect();
    durations.sort_unstable();

    let p95_index = (durations.len() as f64 * 0.95) as usize;
    Some(Duration::from_millis(durations[p95_index]))
}

/// Get average spawn latency from recent metrics
pub fn calculate_avg_spawn_latency() -> Option<Duration> {
    let metrics = SPAWN_METRICS.lock().ok()?;

    if metrics.is_empty() {
        return None;
    }

    let sum: u64 = metrics.iter().map(|m| m.spawn_duration_ms).sum();
    let avg = sum / metrics.len() as u64;
    Some(Duration::from_millis(avg))
}

/// Get success rate from recent metrics
pub fn calculate_success_rate() -> Option<f64> {
    let metrics = SPAWN_METRICS.lock().ok()?;

    if metrics.is_empty() {
        return None;
    }

    let successful = metrics.iter().filter(|m| m.success).count();
    Some(successful as f64 / metrics.len() as f64)
}

/// Clear all metrics (for testing)
#[cfg(test)]
pub fn clear_metrics() {
    if let Ok(mut metrics) = SPAWN_METRICS.lock() {
        metrics.clear();
    }
    if let Ok(mut metrics) = BATCH_METRICS.lock() {
        metrics.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_record_agent_spawn() {
        clear_metrics();

        record_agent_spawn("test_agent", Duration::from_millis(100), true);

        let metrics = SPAWN_METRICS.lock().unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].agent_name, "test_agent");
        assert_eq!(metrics[0].spawn_duration_ms, 100);
        assert!(metrics[0].success);
    }

    #[test]
    #[serial]
    fn test_record_batch_spawn() {
        clear_metrics();

        let durations = vec![
            Duration::from_millis(100),
            Duration::from_millis(150),
            Duration::from_millis(200),
        ];

        record_batch_spawn(3, 3, Duration::from_millis(450), &durations);

        let metrics = BATCH_METRICS.lock().unwrap();
        assert!(metrics.len() >= 1);
        let last = &metrics[metrics.len() - 1];
        assert_eq!(last.total_agents, 3);
        assert_eq!(last.successful_agents, 3);
        assert_eq!(last.total_duration_ms, 450);
        assert_eq!(last.avg_spawn_duration_ms, 150);
        assert_eq!(last.max_spawn_duration_ms, 200);
        assert_eq!(last.min_spawn_duration_ms, 100);
    }

    #[test]
    #[serial]
    fn test_calculate_p95_spawn_latency() {
        clear_metrics();

        // Need at least 20 samples
        for i in 0..25 {
            record_agent_spawn(&format!("agent_{}", i), Duration::from_millis(i * 10), true);
        }

        let p95 = calculate_p95_spawn_latency();
        assert!(p95.is_some());

        // p95 of [0, 10, 20, ..., 240] should be around 228
        let p95_ms = p95.unwrap().as_millis();
        assert!(p95_ms >= 200 && p95_ms <= 240);
    }

    #[test]
    #[serial]
    fn test_calculate_avg_spawn_latency() {
        clear_metrics();

        record_agent_spawn("agent1", Duration::from_millis(100), true);
        record_agent_spawn("agent2", Duration::from_millis(200), true);
        record_agent_spawn("agent3", Duration::from_millis(300), true);

        let avg = calculate_avg_spawn_latency();
        assert!(avg.is_some());
        assert_eq!(avg.unwrap().as_millis(), 200);
    }

    #[test]
    #[serial]
    fn test_calculate_success_rate() {
        clear_metrics();

        record_agent_spawn("agent1", Duration::from_millis(100), true);
        record_agent_spawn("agent2", Duration::from_millis(200), true);
        record_agent_spawn("agent3", Duration::from_millis(300), false);
        record_agent_spawn("agent4", Duration::from_millis(400), true);

        let success_rate = calculate_success_rate();
        assert!(success_rate.is_some());
        assert_eq!(success_rate.unwrap(), 0.75); // 3/4
    }

    #[test]
    #[serial]
    fn test_metrics_rolling_window() {
        clear_metrics();

        // Add 1100 metrics (should trigger cleanup at 1000)
        for i in 0..1100 {
            record_agent_spawn(&format!("agent_{}", i), Duration::from_millis(i * 10), true);
        }

        let metrics = SPAWN_METRICS.lock().unwrap();
        // Should have drained first 100, keeping last 1000
        assert_eq!(metrics.len(), 1000);
    }
}
