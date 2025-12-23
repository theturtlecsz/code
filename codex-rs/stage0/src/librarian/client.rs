//! LocalMemoryClient trait for Librarian MCP integration
//!
//! SPEC-KIT-103 P98: Defines the interface between Librarian and local-memory.
//!
//! ## Architecture
//!
//! - `LocalMemoryClient` trait defined in stage0 (this file)
//! - MCP implementation in TUI (`LocalMemoryMcpClient`)
//! - Mock implementation for tests (`MockLocalMemoryClient`)
//!
//! This hybrid approach allows:
//! - stage0 to remain independent of MCP runtime
//! - TUI to wire in real MCP client
//! - Tests to use mocks without MCP dependency

use crate::errors::Result;
use serde::{Deserialize, Serialize};

/// Parameters for listing memories
#[derive(Debug, Clone, Default)]
pub struct ListParams {
    /// Filter by domains (empty = all domains)
    pub domains: Vec<String>,
    /// Maximum memories to return
    pub limit: usize,
    /// Minimum importance to include (0 = all)
    pub min_importance: Option<i32>,
}

impl ListParams {
    /// Create params for a specific domain
    pub fn domain(domain: impl Into<String>) -> Self {
        Self {
            domains: vec![domain.into()],
            limit: 100,
            min_importance: None,
        }
    }

    /// Create params with a limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set minimum importance
    pub fn with_min_importance(mut self, min_importance: i32) -> Self {
        self.min_importance = Some(min_importance);
        self
    }
}

/// Minimal memory metadata returned from list operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMeta {
    /// Unique memory ID
    pub id: String,
    /// Content preview or full content
    pub content: String,
    /// Tags associated with the memory
    pub tags: Vec<String>,
    /// Importance level (1-10)
    pub importance: Option<i32>,
    /// Domain the memory belongs to
    pub domain: Option<String>,
}

/// Full memory data returned from get operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique memory ID
    pub id: String,
    /// Full content
    pub content: String,
    /// Tags associated with the memory
    pub tags: Vec<String>,
    /// Importance level (1-10)
    pub importance: Option<i32>,
    /// Domain the memory belongs to
    pub domain: Option<String>,
    /// Creation timestamp
    pub created_at: Option<String>,
}

impl From<Memory> for MemoryMeta {
    fn from(mem: Memory) -> Self {
        Self {
            id: mem.id,
            content: mem.content,
            tags: mem.tags,
            importance: mem.importance,
            domain: mem.domain,
        }
    }
}

/// Change to apply to a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChange {
    /// New content (if changed)
    pub content: Option<String>,
    /// New tags (if changed)
    pub tags: Option<Vec<String>>,
    /// New importance (if changed)
    pub importance: Option<i32>,
}

impl MemoryChange {
    /// Create a content-only change
    pub fn content(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tags: None,
            importance: None,
        }
    }

    /// Create a tags-only change
    pub fn tags(tags: Vec<String>) -> Self {
        Self {
            content: None,
            tags: Some(tags),
            importance: None,
        }
    }

    /// Create a change with content and tags
    pub fn content_and_tags(content: impl Into<String>, tags: Vec<String>) -> Self {
        Self {
            content: Some(content.into()),
            tags: Some(tags),
            importance: None,
        }
    }
}

/// Client interface for local-memory operations
///
/// Implementations:
/// - `LocalMemoryMcpClient` (TUI) - Real MCP integration
/// - `MockLocalMemoryClient` (tests) - In-memory mock
pub trait LocalMemoryClient: Send + Sync {
    /// List memories matching the given parameters
    ///
    /// Returns memory metadata for efficient scanning.
    /// Use `get_memory` for full content when needed.
    fn list_memories(&self, params: &ListParams) -> Result<Vec<MemoryMeta>>;

    /// Get a single memory by ID with full content
    fn get_memory(&self, id: &str) -> Result<Memory>;

    /// Update a memory with the given changes
    ///
    /// Only non-None fields in `MemoryChange` are updated.
    fn update_memory(&self, id: &str, change: &MemoryChange) -> Result<()>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Relationships Client (SPEC-KIT-103 P98 Task 6)
// ─────────────────────────────────────────────────────────────────────────────

/// Input for creating a relationship between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipInput {
    /// Source memory ID
    pub source_id: String,
    /// Target memory ID
    pub target_id: String,
    /// Type of relationship (causes, blocks, enables, relates_to)
    pub relationship_type: String,
    /// Strength of the relationship (0.0 - 1.0)
    pub strength: f32,
    /// Explanation for why this relationship exists
    pub context: Option<String>,
}

impl RelationshipInput {
    /// Create a new relationship input
    pub fn new(
        source_id: impl Into<String>,
        target_id: impl Into<String>,
        relationship_type: impl Into<String>,
        strength: f32,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
            relationship_type: relationship_type.into(),
            strength,
            context: None,
        }
    }

    /// Add context explaining the relationship
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// Client interface for memory relationship operations
///
/// Creates causal edges between memories in local-memory.
pub trait RelationshipsClient: Send + Sync {
    /// Create a relationship between two memories
    fn create_relationship(&self, input: &RelationshipInput) -> Result<()>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Mock implementation for tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(any(test, feature = "test-utils"))]
#[allow(clippy::unwrap_used)] // Mock code: panicking on poisoned lock is acceptable in tests
pub mod mock {
    use super::*;
    use crate::errors::Stage0Error;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    /// Mock implementation of LocalMemoryClient for testing
    #[derive(Debug, Default)]
    pub struct MockLocalMemoryClient {
        memories: Arc<RwLock<HashMap<String, Memory>>>,
        /// Track update calls for verification
        updates: Arc<RwLock<Vec<(String, MemoryChange)>>>,
    }

    impl MockLocalMemoryClient {
        /// Create a new empty mock client
        pub fn new() -> Self {
            Self::default()
        }

        /// Add a memory to the mock store
        pub fn add_memory(&self, memory: Memory) {
            self.memories
                .write()
                .unwrap()
                .insert(memory.id.clone(), memory);
        }

        /// Add multiple memories at once
        pub fn add_memories(&self, memories: Vec<Memory>) {
            let mut store = self.memories.write().unwrap();
            for memory in memories {
                store.insert(memory.id.clone(), memory);
            }
        }

        /// Get the list of updates made (for test verification)
        pub fn get_updates(&self) -> Vec<(String, MemoryChange)> {
            self.updates.read().unwrap().clone()
        }

        /// Clear all updates (for test reset)
        pub fn clear_updates(&self) {
            self.updates.write().unwrap().clear();
        }

        /// Get current memory count
        pub fn memory_count(&self) -> usize {
            self.memories.read().unwrap().len()
        }
    }

    impl LocalMemoryClient for MockLocalMemoryClient {
        fn list_memories(&self, params: &ListParams) -> Result<Vec<MemoryMeta>> {
            let store = self.memories.read().unwrap();
            let mut results: Vec<MemoryMeta> = store
                .values()
                .filter(|m| {
                    // Domain filter
                    if !params.domains.is_empty() {
                        if let Some(ref domain) = m.domain {
                            if !params.domains.contains(domain) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }

                    // Importance filter
                    if let Some(min_importance) = params.min_importance
                        && m.importance.unwrap_or(0) < min_importance
                    {
                        return false;
                    }

                    true
                })
                .map(|m| MemoryMeta {
                    id: m.id.clone(),
                    content: m.content.clone(),
                    tags: m.tags.clone(),
                    importance: m.importance,
                    domain: m.domain.clone(),
                })
                .collect();

            // Apply limit
            results.truncate(params.limit);

            Ok(results)
        }

        fn get_memory(&self, id: &str) -> Result<Memory> {
            let store = self.memories.read().unwrap();
            store
                .get(id)
                .cloned()
                .ok_or_else(|| Stage0Error::local_memory(format!("memory not found: {id}")))
        }

        fn update_memory(&self, id: &str, change: &MemoryChange) -> Result<()> {
            // Record the update for test verification
            self.updates
                .write()
                .unwrap()
                .push((id.to_string(), change.clone()));

            // Apply the change
            let mut store = self.memories.write().unwrap();
            let memory = store
                .get_mut(id)
                .ok_or_else(|| Stage0Error::local_memory(format!("memory not found: {id}")))?;

            if let Some(ref content) = change.content {
                memory.content = content.clone();
            }
            if let Some(ref tags) = change.tags {
                memory.tags = tags.clone();
            }
            if let Some(importance) = change.importance {
                memory.importance = Some(importance);
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockLocalMemoryClient;
    use super::*;

    fn sample_memory(id: &str, domain: &str, importance: i32) -> Memory {
        Memory {
            id: id.to_string(),
            content: format!("Content for {id}"),
            tags: vec!["type:pattern".to_string()],
            importance: Some(importance),
            domain: Some(domain.to_string()),
            created_at: None,
        }
    }

    #[test]
    fn test_mock_client_add_and_get() {
        let client = MockLocalMemoryClient::new();
        client.add_memory(sample_memory("mem-001", "spec-kit", 8));

        let mem = client.get_memory("mem-001").expect("should exist");
        assert_eq!(mem.id, "mem-001");
        assert_eq!(mem.importance, Some(8));
    }

    #[test]
    fn test_mock_client_get_not_found() {
        let client = MockLocalMemoryClient::new();
        let result = client.get_memory("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_client_list_all() {
        let client = MockLocalMemoryClient::new();
        client.add_memories(vec![
            sample_memory("mem-001", "spec-kit", 8),
            sample_memory("mem-002", "spec-kit", 7),
            sample_memory("mem-003", "other", 9),
        ]);

        let params = ListParams::default().with_limit(10);
        let results = client.list_memories(&params).expect("should work");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_mock_client_list_by_domain() {
        let client = MockLocalMemoryClient::new();
        client.add_memories(vec![
            sample_memory("mem-001", "spec-kit", 8),
            sample_memory("mem-002", "spec-kit", 7),
            sample_memory("mem-003", "other", 9),
        ]);

        let params = ListParams::domain("spec-kit").with_limit(10);
        let results = client.list_memories(&params).expect("should work");
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|m| m.domain.as_deref() == Some("spec-kit"))
        );
    }

    #[test]
    fn test_mock_client_list_with_min_importance() {
        let client = MockLocalMemoryClient::new();
        client.add_memories(vec![
            sample_memory("mem-001", "spec-kit", 8),
            sample_memory("mem-002", "spec-kit", 7),
            sample_memory("mem-003", "spec-kit", 9),
        ]);

        let params = ListParams::domain("spec-kit")
            .with_limit(10)
            .with_min_importance(8);
        let results = client.list_memories(&params).expect("should work");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|m| m.importance.unwrap_or(0) >= 8));
    }

    #[test]
    fn test_mock_client_list_with_limit() {
        let client = MockLocalMemoryClient::new();
        client.add_memories(vec![
            sample_memory("mem-001", "spec-kit", 8),
            sample_memory("mem-002", "spec-kit", 7),
            sample_memory("mem-003", "spec-kit", 9),
        ]);

        let params = ListParams::domain("spec-kit").with_limit(2);
        let results = client.list_memories(&params).expect("should work");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_mock_client_update() {
        let client = MockLocalMemoryClient::new();
        client.add_memory(sample_memory("mem-001", "spec-kit", 8));

        let change = MemoryChange::content("Updated content");
        client
            .update_memory("mem-001", &change)
            .expect("should work");

        // Verify update was recorded
        let updates = client.get_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].0, "mem-001");

        // Verify memory was changed
        let mem = client.get_memory("mem-001").expect("should exist");
        assert_eq!(mem.content, "Updated content");
    }

    #[test]
    fn test_mock_client_update_tags() {
        let client = MockLocalMemoryClient::new();
        client.add_memory(sample_memory("mem-001", "spec-kit", 8));

        let change = MemoryChange::tags(vec!["type:decision".to_string()]);
        client
            .update_memory("mem-001", &change)
            .expect("should work");

        let mem = client.get_memory("mem-001").expect("should exist");
        assert_eq!(mem.tags, vec!["type:decision".to_string()]);
    }

    #[test]
    fn test_mock_client_update_content_and_tags() {
        let client = MockLocalMemoryClient::new();
        client.add_memory(sample_memory("mem-001", "spec-kit", 8));

        let change = MemoryChange::content_and_tags(
            "New structured content",
            vec![
                "type:decision".to_string(),
                "component:librarian".to_string(),
            ],
        );
        client
            .update_memory("mem-001", &change)
            .expect("should work");

        let mem = client.get_memory("mem-001").expect("should exist");
        assert_eq!(mem.content, "New structured content");
        assert_eq!(mem.tags.len(), 2);
    }

    #[test]
    fn test_mock_client_update_not_found() {
        let client = MockLocalMemoryClient::new();
        let change = MemoryChange::content("Whatever");
        let result = client.update_memory("nonexistent", &change);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_params_builder() {
        let params = ListParams::domain("spec-kit")
            .with_limit(50)
            .with_min_importance(7);

        assert_eq!(params.domains, vec!["spec-kit".to_string()]);
        assert_eq!(params.limit, 50);
        assert_eq!(params.min_importance, Some(7));
    }
}
