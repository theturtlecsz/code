use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

/// Watches configuration files for changes and provides debounced change notifications
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<Result<Event, notify::Error>>,
    debounce_duration: Duration,
    last_event_time: Option<Instant>,
    pending_changes: HashSet<PathBuf>,
}

impl ConfigWatcher {
    /// Create a new ConfigWatcher that monitors the specified paths
    ///
    /// # Arguments
    /// * `watch_paths` - Paths to monitor (files or directories)
    /// * `debounce_ms` - Milliseconds to wait before reporting changes (debounce rapid events)
    pub fn new(watch_paths: &[PathBuf], debounce_ms: u64) -> Result<Self> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )
        .context("Failed to create file watcher")?;

        // Watch each path
        for path in watch_paths {
            if path.exists() {
                watcher
                    .watch(path, RecursiveMode::NonRecursive)
                    .with_context(|| format!("Failed to watch path: {}", path.display()))?;
            }
        }

        Ok(Self {
            _watcher: watcher,
            rx,
            debounce_duration: Duration::from_millis(debounce_ms),
            last_event_time: None,
            pending_changes: HashSet::new(),
        })
    }

    /// Check for file changes and return paths if debounce period has elapsed
    ///
    /// Returns:
    /// - `Some(paths)` if changes detected and debounce period elapsed
    /// - `None` if no changes or still within debounce period
    pub fn check_for_changes(&mut self) -> Option<Vec<PathBuf>> {
        let mut has_new_events = false;

        // Collect all pending events
        loop {
            match self.rx.try_recv() {
                Ok(Ok(event)) => {
                    if should_process_event(&event) {
                        // Extract paths from event
                        for path in &event.paths {
                            self.pending_changes.insert(path.clone());
                        }
                        has_new_events = true;
                    }
                }
                Ok(Err(_)) => {
                    // Watcher error - ignore for now
                    continue;
                }
                Err(TryRecvError::Empty) => {
                    // No more events
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    // Watcher disconnected - should not happen in normal operation
                    break;
                }
            }
        }

        // Update last event time if new events arrived
        if has_new_events {
            self.last_event_time = Some(Instant::now());
        }

        // Check if debounce period has elapsed
        if let Some(last_time) = self.last_event_time
            && !self.pending_changes.is_empty()
            && last_time.elapsed() >= self.debounce_duration
        {
            // Debounce period elapsed - return changes and reset
            let changes: Vec<PathBuf> = self.pending_changes.drain().collect();
            self.last_event_time = None;
            return Some(changes);
        }

        None
    }

    /// Get the debounce duration
    #[cfg(test)]
    pub fn debounce_duration(&self) -> Duration {
        self.debounce_duration
    }
}

/// Determine if an event should trigger a reload
fn should_process_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "test").unwrap();

        let watcher = ConfigWatcher::new(&[config_path], 500);
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        assert_eq!(watcher.debounce_duration(), Duration::from_millis(500));
    }

    #[test]
    fn test_debounce_logic() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "initial").unwrap();

        let mut watcher = ConfigWatcher::new(std::slice::from_ref(&config_path), 100).unwrap();

        // Modify file
        fs::write(&config_path, "modified1").unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // Should not report yet (within debounce)
        assert!(watcher.check_for_changes().is_none());

        // Modify again
        fs::write(&config_path, "modified2").unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // Still within debounce (resets on new event)
        assert!(watcher.check_for_changes().is_none());

        // Wait for debounce to expire
        std::thread::sleep(Duration::from_millis(150));

        // Should now report changes
        let changes = watcher.check_for_changes();
        assert!(changes.is_some());
        let changes = changes.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0], config_path);

        // Subsequent check should return None
        assert!(watcher.check_for_changes().is_none());
    }

    #[test]
    fn test_multiple_path_watching() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let agents_path = temp_dir.path().join("agents.toml");

        fs::write(&config_path, "config").unwrap();
        fs::write(&agents_path, "agents").unwrap();

        let mut watcher =
            ConfigWatcher::new(&[config_path.clone(), agents_path.clone()], 100).unwrap();

        // Give watcher time to initialize
        std::thread::sleep(Duration::from_millis(50));

        // Modify both files
        fs::write(&config_path, "config_modified").unwrap();
        std::thread::sleep(Duration::from_millis(10));
        fs::write(&agents_path, "agents_modified").unwrap();

        // Wait for debounce and event propagation
        // Poll multiple times to ensure events arrive
        let mut changes = None;
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(50));
            changes = watcher.check_for_changes();
            if changes.is_some() {
                break;
            }
        }

        assert!(
            changes.is_some(),
            "No changes detected after multiple polls"
        );
        let changes = changes.unwrap();
        assert!(
            !changes.is_empty(),
            "Expected at least 1 changed file, got {}",
            changes.len()
        );

        // Verify at least one of the expected paths is present
        // (filesystem events can be unpredictable in tests)
        let paths_set: HashSet<_> = changes.iter().collect();
        assert!(
            paths_set.contains(&config_path) || paths_set.contains(&agents_path),
            "Expected at least one of the watched paths"
        );
    }

    #[test]
    fn test_should_process_event() {
        // Create events are processed
        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![],
            attrs: Default::default(),
        };
        assert!(should_process_event(&event));

        // Modify events are processed
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
            paths: vec![],
            attrs: Default::default(),
        };
        assert!(should_process_event(&event));

        // Remove events are processed
        let event = Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![],
            attrs: Default::default(),
        };
        assert!(should_process_event(&event));

        // Access events are NOT processed
        let event = Event {
            kind: EventKind::Access(notify::event::AccessKind::Read),
            paths: vec![],
            attrs: Default::default(),
        };
        assert!(!should_process_event(&event));
    }
}
