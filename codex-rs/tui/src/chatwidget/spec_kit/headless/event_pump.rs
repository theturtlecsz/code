//! Headless event pump for agent completion polling (SPEC-KIT-900)
//!
//! Polls for agent completion without requiring a TUI event loop.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::runner::HeadlessError;

/// Agent execution result
#[derive(Debug, Clone)]
pub enum AgentResult {
    /// Agent completed successfully with output
    Success(String),
    /// Agent failed with error
    Failed(String),
}

/// Headless event pump for polling agent completion
pub struct HeadlessEventPump {
    /// Poll interval
    pub poll_interval: Duration,
    /// Maximum timeout
    pub timeout: Duration,
}

impl Default for HeadlessEventPump {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            timeout: Duration::from_secs(300), // 5 minute default timeout
        }
    }
}

impl HeadlessEventPump {
    /// Create a new event pump with custom timeouts
    pub fn new(poll_interval: Duration, timeout: Duration) -> Self {
        Self {
            poll_interval,
            timeout,
        }
    }

    /// Wait for all agents to complete (blocking)
    ///
    /// Returns agent results indexed by agent ID.
    pub fn wait_for_agents_blocking(
        &self,
        agent_ids: &[String],
        agent_results: &std::sync::Arc<std::sync::RwLock<HashMap<String, AgentResult>>>,
    ) -> Result<HashMap<String, AgentResult>, HeadlessError> {
        let start = Instant::now();

        loop {
            // Check timeout
            if start.elapsed() > self.timeout {
                let completed: Vec<_> = {
                    let results = agent_results.read().unwrap();
                    results.keys().cloned().collect()
                };
                return Err(HeadlessError::Timeout {
                    expected: agent_ids.len(),
                    completed: completed.len(),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                });
            }

            // Check if all agents completed
            let results = agent_results.read().unwrap();
            let all_done = agent_ids.iter().all(|id| results.contains_key(id));

            if all_done {
                let mut final_results = HashMap::new();
                for id in agent_ids {
                    if let Some(result) = results.get(id) {
                        final_results.insert(id.clone(), result.clone());
                    }
                }
                return Ok(final_results);
            }

            drop(results);

            // Sleep before next poll
            std::thread::sleep(self.poll_interval);
        }
    }
}

/// Wait for agents to complete (async version)
///
/// This is the public async interface for agent polling.
pub async fn wait_for_agents(
    agent_ids: &[String],
    timeout: Duration,
) -> Result<HashMap<String, AgentResult>, HeadlessError> {
    // In headless mode, we need to poll AGENT_MANAGER directly
    // This is a stub that will be connected to the actual AGENT_MANAGER
    // when the runner is fully integrated

    let start = Instant::now();

    // Stub implementation: poll once then return
    // Real implementation will loop until agents complete or timeout
    if start.elapsed() > timeout {
        return Err(HeadlessError::Timeout {
            expected: agent_ids.len(),
            completed: 0,
            elapsed_ms: start.elapsed().as_millis() as u64,
        });
    }

    // TODO: Poll AGENT_MANAGER for agent status
    // For now, return empty results for stub
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stub: Return empty results
    // Real implementation will return actual agent outputs
    Ok(HashMap::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_pump_default() {
        let pump = HeadlessEventPump::default();
        assert_eq!(pump.poll_interval, Duration::from_millis(100));
        assert_eq!(pump.timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_event_pump_custom() {
        let pump = HeadlessEventPump::new(Duration::from_millis(50), Duration::from_secs(60));
        assert_eq!(pump.poll_interval, Duration::from_millis(50));
        assert_eq!(pump.timeout, Duration::from_secs(60));
    }
}
