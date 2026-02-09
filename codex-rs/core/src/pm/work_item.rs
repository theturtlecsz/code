//! Work item types for the PM registry (SPEC-PM-001).
//!
//! Work items are the core entities managed by the PM system:
//! - **Feature**: High-level user-facing capability
//! - **Spec**: Technical specification (PRD + design)
//! - **Task**: Atomic unit of work
//!
//! ## Decision References
//! - PM-D1: NeedsResearch/NeedsReview hybrid states with `return_state`
//! - PM-D2: Immutable fields: `id`, `type`, `created_at`, `parent_id`
//! - PM-D3: Shared + level-specific fields

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Work item level/type (PM-D3).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemType {
    Feature,
    Spec,
    Task,
}

/// Lifecycle state for a work item (PM-D1).
///
/// NeedsResearch and NeedsReview are hybrid states: the item enters the state,
/// a bot run executes, and on completion the item auto-returns to `return_state`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemState {
    /// Initial state after creation.
    Draft,
    /// Actively being worked on.
    InProgress,
    /// Blocked on external dependency or decision.
    Blocked,
    /// Bot research run requested (PM-D1 hybrid state).
    NeedsResearch {
        /// State to return to when the research run completes.
        return_state: Box<WorkItemState>,
    },
    /// Bot review run requested (PM-D1 hybrid state).
    NeedsReview {
        /// State to return to when the review run completes.
        return_state: Box<WorkItemState>,
    },
    /// Ready for validation/acceptance.
    ReadyForReview,
    /// Completed successfully.
    Done,
    /// Cancelled/abandoned.
    Cancelled,
}

impl WorkItemState {
    /// Whether this is a hybrid bot-triggered state.
    pub fn is_bot_state(&self) -> bool {
        matches!(
            self,
            WorkItemState::NeedsResearch { .. } | WorkItemState::NeedsReview { .. }
        )
    }

    /// Extract the return state for hybrid states, or None for regular states.
    pub fn return_state(&self) -> Option<&WorkItemState> {
        match self {
            WorkItemState::NeedsResearch { return_state } => Some(return_state),
            WorkItemState::NeedsReview { return_state } => Some(return_state),
            _ => None,
        }
    }
}

/// Priority level for features.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Shared fields present on all work items (PM-D3).
///
/// Immutable fields (PM-D2): `id`, `item_type`, `created_at`, `parent_id`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorkItemCore {
    /// Unique identifier (immutable, PM-D2).
    pub id: String,

    /// Work item type (immutable, PM-D2).
    pub item_type: WorkItemType,

    /// Human-readable title (mutable).
    pub title: String,

    /// Description / body (mutable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Creation timestamp, RFC3339 (immutable, PM-D2).
    pub created_at: String,

    /// Last modification timestamp, RFC3339 (mutable, auto-updated).
    pub updated_at: String,

    /// Parent work item ID (immutable, PM-D2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// Current lifecycle state (mutable).
    pub state: WorkItemState,
}

/// Feature-specific fields (PM-D3).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FeatureFields {
    /// Acceptance criteria for completion.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance_criteria: Vec<String>,

    /// Priority level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
}

/// Spec-specific fields (PM-D3).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SpecFields {
    /// URI to the PRD document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prd_uri: Option<String>,

    /// Quality score from review (0.0 - 1.0, stored as u8 percentage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_score: Option<u8>,
}

/// Task-specific fields (PM-D3).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TaskFields {
    /// Assigned agent or user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,

    /// Task result summary on completion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// A complete work item with level-specific fields.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum WorkItem {
    #[serde(rename = "feature")]
    Feature {
        #[serde(flatten)]
        core: WorkItemCore,
        #[serde(flatten)]
        fields: FeatureFields,
    },
    #[serde(rename = "spec")]
    Spec {
        #[serde(flatten)]
        core: WorkItemCore,
        #[serde(flatten)]
        fields: SpecFields,
    },
    #[serde(rename = "task")]
    Task {
        #[serde(flatten)]
        core: WorkItemCore,
        #[serde(flatten)]
        fields: TaskFields,
    },
}

impl WorkItem {
    /// Get the shared core fields.
    pub fn core(&self) -> &WorkItemCore {
        match self {
            WorkItem::Feature { core, .. } => core,
            WorkItem::Spec { core, .. } => core,
            WorkItem::Task { core, .. } => core,
        }
    }

    /// Get the work item ID.
    pub fn id(&self) -> &str {
        &self.core().id
    }

    /// Get the current state.
    pub fn state(&self) -> &WorkItemState {
        &self.core().state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_item_state_roundtrip() {
        let state = WorkItemState::NeedsResearch {
            return_state: Box::new(WorkItemState::InProgress),
        };
        let json = serde_json::to_string(&state).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: WorkItemState =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        assert_eq!(state, back);
    }

    #[test]
    fn work_item_feature_roundtrip() {
        let now = "2026-02-09T00:00:00Z".to_string();
        let item = WorkItem::Feature {
            core: WorkItemCore {
                id: "FEAT-001".to_string(),
                item_type: WorkItemType::Feature,
                title: "Test feature".to_string(),
                description: Some("A test feature".to_string()),
                created_at: now.clone(),
                updated_at: now,
                parent_id: None,
                state: WorkItemState::Draft,
            },
            fields: FeatureFields {
                acceptance_criteria: vec!["AC1".to_string()],
                priority: Some(Priority::High),
            },
        };

        let json = serde_json::to_string_pretty(&item).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: WorkItem =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        assert_eq!(item.id(), back.id());
    }

    #[test]
    fn hybrid_state_return() {
        let state = WorkItemState::NeedsReview {
            return_state: Box::new(WorkItemState::InProgress),
        };
        assert!(state.is_bot_state());
        assert_eq!(state.return_state(), Some(&WorkItemState::InProgress));
    }

    #[test]
    fn regular_state_no_return() {
        let state = WorkItemState::Draft;
        assert!(!state.is_bot_state());
        assert!(state.return_state().is_none());
    }
}
