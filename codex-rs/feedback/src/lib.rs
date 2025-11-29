//! Ring buffer logging infrastructure for feedback collection.
//!
//! Provides a fixed-capacity ring buffer that captures tracing output,
//! keeping only the most recent bytes when capacity is exceeded.

use std::collections::VecDeque;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::writer::MakeWriter;

/// Default maximum buffer size: 4 MiB
const DEFAULT_MAX_BYTES: usize = 4 * 1024 * 1024;

/// Thread-safe feedback collector with ring buffer storage.
#[derive(Clone)]
pub struct CodexFeedback {
    inner: Arc<FeedbackInner>,
}

impl Default for CodexFeedback {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexFeedback {
    /// Create a new feedback collector with default capacity (4 MiB).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_BYTES)
    }

    /// Create a new feedback collector with specified capacity in bytes.
    pub fn with_capacity(max_bytes: usize) -> Self {
        Self {
            inner: Arc::new(FeedbackInner::new(max_bytes)),
        }
    }

    /// Get a writer factory for use with tracing-subscriber.
    pub fn make_writer(&self) -> FeedbackMakeWriter {
        FeedbackMakeWriter {
            inner: self.inner.clone(),
        }
    }

    /// Take a snapshot of the current buffer contents.
    pub fn snapshot(&self, session_id: Option<&str>) -> CodexLogSnapshot {
        let bytes = {
            let guard = self.inner.ring.lock().expect("mutex poisoned");
            guard.snapshot_bytes()
        };
        CodexLogSnapshot {
            bytes,
            thread_id: session_id
                .map(String::from)
                .unwrap_or_else(|| format!("no-active-thread-{}", uuid_v4_simple())),
        }
    }

    /// Get the current buffer size in bytes.
    pub fn len(&self) -> usize {
        let guard = self.inner.ring.lock().expect("mutex poisoned");
        guard.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear the buffer contents.
    pub fn clear(&self) {
        let mut guard = self.inner.ring.lock().expect("mutex poisoned");
        guard.clear();
    }
}

struct FeedbackInner {
    ring: Mutex<RingBuffer>,
}

impl FeedbackInner {
    fn new(max_bytes: usize) -> Self {
        Self {
            ring: Mutex::new(RingBuffer::new(max_bytes)),
        }
    }
}

/// Writer factory for tracing-subscriber integration.
#[derive(Clone)]
pub struct FeedbackMakeWriter {
    inner: Arc<FeedbackInner>,
}

impl<'a> MakeWriter<'a> for FeedbackMakeWriter {
    type Writer = FeedbackWriter;

    fn make_writer(&'a self) -> Self::Writer {
        FeedbackWriter {
            inner: self.inner.clone(),
        }
    }
}

/// Writer that appends to the ring buffer.
pub struct FeedbackWriter {
    inner: Arc<FeedbackInner>,
}

impl Write for FeedbackWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self.inner.ring.lock().map_err(|_| io::ErrorKind::Other)?;
        guard.push_bytes(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Fixed-capacity ring buffer that evicts oldest bytes when full.
struct RingBuffer {
    max: usize,
    buf: VecDeque<u8>,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            max: capacity,
            buf: VecDeque::with_capacity(capacity),
        }
    }

    fn len(&self) -> usize {
        self.buf.len()
    }

    fn clear(&mut self) {
        self.buf.clear();
    }

    fn push_bytes(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        // If the incoming chunk is larger than capacity, keep only the trailing bytes.
        if data.len() >= self.max {
            self.buf.clear();
            let start = data.len() - self.max;
            self.buf.extend(data[start..].iter().copied());
            return;
        }

        // Evict from the front if we would exceed capacity.
        let needed = self.len() + data.len();
        if needed > self.max {
            let to_drop = needed - self.max;
            self.buf.drain(..to_drop);
        }

        self.buf.extend(data.iter().copied());
    }

    fn snapshot_bytes(&self) -> Vec<u8> {
        self.buf.iter().copied().collect()
    }
}

/// Snapshot of log buffer contents.
pub struct CodexLogSnapshot {
    bytes: Vec<u8>,
    /// Identifier for this snapshot (session ID or generated).
    pub thread_id: String,
}

impl CodexLogSnapshot {
    /// Get the raw bytes of the snapshot.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Get the snapshot as a UTF-8 string (lossy conversion).
    pub fn as_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.bytes)
    }

    /// Get the size of the snapshot in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Check if the snapshot is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Save the snapshot to a temporary file.
    pub fn save_to_temp_file(&self) -> io::Result<PathBuf> {
        let dir = std::env::temp_dir();
        let filename = format!("codex-feedback-{}.log", self.thread_id);
        let path = dir.join(filename);
        fs::write(&path, self.as_bytes())?;
        Ok(path)
    }

    /// Save the snapshot to a specific path.
    pub fn save_to_path(&self, path: &std::path::Path) -> io::Result<()> {
        fs::write(path, self.as_bytes())
    }
}

/// Generate a simple UUID v4-like string for thread IDs.
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:016x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_drops_front_when_full() {
        let fb = CodexFeedback::with_capacity(8);
        {
            let mut w = fb.make_writer().make_writer();
            w.write_all(b"abcdefgh").unwrap();
            w.write_all(b"ij").unwrap();
        }
        let snap = fb.snapshot(None);
        // Capacity 8: after writing 10 bytes, we should keep the last 8.
        pretty_assertions::assert_eq!(std::str::from_utf8(snap.as_bytes()).unwrap(), "cdefghij");
    }

    #[test]
    fn ring_buffer_handles_oversized_write() {
        let fb = CodexFeedback::with_capacity(4);
        {
            let mut w = fb.make_writer().make_writer();
            // Write more than capacity in one go
            w.write_all(b"abcdefghij").unwrap();
        }
        let snap = fb.snapshot(None);
        // Should keep only last 4 bytes
        pretty_assertions::assert_eq!(std::str::from_utf8(snap.as_bytes()).unwrap(), "ghij");
    }

    #[test]
    fn ring_buffer_empty_write_is_noop() {
        let fb = CodexFeedback::with_capacity(8);
        {
            let mut w = fb.make_writer().make_writer();
            w.write_all(b"abc").unwrap();
            w.write_all(b"").unwrap();
        }
        let snap = fb.snapshot(None);
        pretty_assertions::assert_eq!(std::str::from_utf8(snap.as_bytes()).unwrap(), "abc");
    }

    #[test]
    fn feedback_clear_empties_buffer() {
        let fb = CodexFeedback::with_capacity(8);
        {
            let mut w = fb.make_writer().make_writer();
            w.write_all(b"hello").unwrap();
        }
        assert_eq!(fb.len(), 5);
        fb.clear();
        assert!(fb.is_empty());
    }

    #[test]
    fn snapshot_with_session_id() {
        let fb = CodexFeedback::with_capacity(8);
        let snap = fb.snapshot(Some("test-session-123"));
        assert_eq!(snap.thread_id, "test-session-123");
    }

    #[test]
    fn snapshot_save_to_temp_file() {
        let fb = CodexFeedback::with_capacity(64);
        {
            let mut w = fb.make_writer().make_writer();
            w.write_all(b"test log content").unwrap();
        }
        let snap = fb.snapshot(Some("test-save"));
        let path = snap.save_to_temp_file().unwrap();
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "test log content");
        // Cleanup
        let _ = fs::remove_file(path);
    }
}
