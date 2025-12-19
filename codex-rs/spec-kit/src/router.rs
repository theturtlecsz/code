//! Router: Role → Worker Implementation Mapping
//!
//! The Router is responsible for selecting which Worker implements a given Role.
//! It reads from MODEL-POLICY.md / configuration and returns a WorkerSpec.
//!
//! **Key Principle**: Gate Policy decides roles and requirements.
//! Router decides worker implementations. They never cross boundaries.
//!
//! See `docs/spec-kit/GATE_POLICY.md` and `docs/MODEL-POLICY.md` for policy details.

use crate::gate_policy::{Role, Stage, StageContext};
use serde::{Deserialize, Serialize};

// ============================================================================
// Worker Specification
// ============================================================================

/// Worker implementation kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerKind {
    /// Local model (runs on user's machine)
    LocalModel,
    /// Cloud model (API call to provider)
    CloudModel,
    /// Human reviewer (escalation target)
    Human,
    /// Tool-only worker (no LLM, just tooling)
    ToolOnly,
}

/// Budget constraints for a worker.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Budget {
    /// Maximum input tokens (0 = unlimited)
    #[serde(default)]
    pub max_input_tokens: u32,
    /// Maximum output tokens (0 = unlimited)
    #[serde(default)]
    pub max_output_tokens: u32,
    /// Maximum cost in USD (0.0 = unlimited)
    #[serde(default)]
    pub max_cost_usd: f64,
    /// Maximum execution time in seconds (0 = unlimited)
    #[serde(default)]
    pub max_time_seconds: u32,
}

/// Tool permissions for a worker.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPermissions {
    /// Can read files
    #[serde(default = "default_true")]
    pub can_read_files: bool,
    /// Can write files
    #[serde(default)]
    pub can_write_files: bool,
    /// Can execute shell commands
    #[serde(default)]
    pub can_execute_shell: bool,
    /// Can make network requests
    #[serde(default)]
    pub can_network: bool,
    /// Can access MCP tools
    #[serde(default = "default_true")]
    pub can_use_mcp: bool,
}

fn default_true() -> bool {
    true
}

/// Full specification for a worker.
///
/// This is what the Router returns. It contains enough information
/// for the orchestrator to spawn the worker without knowing model details.
///
/// **Note**: The `provider` and `model` fields are intentionally opaque strings.
/// Gate policy code should never pattern-match on these values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerSpec {
    /// Unique identifier for this worker configuration
    pub id: String,
    /// The role this worker implements
    pub role: Role,
    /// Kind of worker
    pub kind: WorkerKind,
    /// Human-readable label (for UI/logs)
    pub label: Option<String>,
    /// Provider identifier (opaque to gate policy)
    pub provider: String,
    /// Model identifier (opaque to gate policy)
    pub model: String,
    /// Budget constraints
    pub budget: Budget,
    /// Tool permissions
    pub tool_permissions: ToolPermissions,
}

impl WorkerSpec {
    /// Create a new WorkerSpec with minimal required fields
    pub fn new(role: Role, provider: impl Into<String>, model: impl Into<String>) -> Self {
        let provider = provider.into();
        let model = model.into();
        let id = format!("{}:{}:{}", role_to_id(role), provider, model);

        Self {
            id,
            role,
            kind: WorkerKind::CloudModel,
            label: None,
            provider,
            model,
            budget: Budget::default(),
            tool_permissions: ToolPermissions::default(),
        }
    }

    /// Builder: set worker kind
    pub fn with_kind(mut self, kind: WorkerKind) -> Self {
        self.kind = kind;
        self
    }

    /// Builder: set label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Builder: set budget
    pub fn with_budget(mut self, budget: Budget) -> Self {
        self.budget = budget;
        self
    }

    /// Builder: set tool permissions
    pub fn with_permissions(mut self, permissions: ToolPermissions) -> Self {
        self.tool_permissions = permissions;
        self
    }
}

fn role_to_id(role: Role) -> &'static str {
    match role {
        Role::Architect => "architect",
        Role::Implementer => "implementer",
        Role::Validator => "validator",
        Role::Judge => "judge",
        Role::SidecarCritic => "sidecar_critic",
        Role::SecurityReviewer => "security_reviewer",
        Role::PerformanceReviewer => "perf_reviewer",
        Role::Librarian => "librarian",
    }
}

// ============================================================================
// Routing Context
// ============================================================================

/// Context for routing decisions.
///
/// Contains information the router needs to select the appropriate worker.
#[derive(Clone, Debug, Default)]
pub struct RoutingContext {
    pub stage: Option<Stage>,
    pub is_high_risk: bool,
    pub local_only: bool,
    pub retry_count: u32,
}

impl From<&StageContext> for RoutingContext {
    fn from(ctx: &StageContext) -> Self {
        Self {
            stage: ctx.stage, // Preserved from StageContext if set
            is_high_risk: ctx.is_high_risk,
            local_only: ctx.local_only,
            retry_count: ctx.retry_count,
        }
    }
}

// ============================================================================
// Router Trait
// ============================================================================

/// The Router interface.
///
/// Gate Policy asks the Router: "give me a WorkerSpec for this Role."
/// The Router reads MODEL-POLICY.md / config and returns the appropriate worker.
///
/// **Key Contract**:
/// - Gate Policy never imports model/provider names
/// - Router never encodes gate logic
pub trait Router: Send + Sync {
    /// Select a worker for the given role and context.
    fn select_worker(&self, role: Role, ctx: &RoutingContext) -> WorkerSpec;

    /// Check if a role is available (has a configured worker).
    fn is_role_available(&self, role: Role, ctx: &RoutingContext) -> bool {
        // Default: assume available
        let _ = (role, ctx);
        true
    }

    /// Get all available workers for a role (for fallback/retry).
    fn workers_for_role(&self, role: Role, ctx: &RoutingContext) -> Vec<WorkerSpec> {
        vec![self.select_worker(role, ctx)]
    }
}

// ============================================================================
// Default Router Implementation (Example/Development Fallback)
// ============================================================================

/// Example router with hardcoded role→model mappings for development.
///
/// **WARNING: This is NOT the canonical routing policy.**
///
/// This implementation provides sensible defaults for local development
/// and testing. Production deployments should:
///
/// 1. Use a config-driven router that reads from `MODEL-POLICY.md` or
///    structured configuration files.
/// 2. Implement the `Router` trait in the orchestration layer (TUI/CLI)
///    where runtime configuration is available.
///
/// The hardcoded provider/model values here are examples only and may
/// diverge from the actual MODEL-POLICY.md routing tables.
#[derive(Clone, Debug, Default)]
pub struct DefaultRouter {
    /// If true, prefer local models when available
    pub prefer_local: bool,
}

impl DefaultRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_local_preference(mut self, prefer_local: bool) -> Self {
        self.prefer_local = prefer_local;
        self
    }
}

impl Router for DefaultRouter {
    fn select_worker(&self, role: Role, ctx: &RoutingContext) -> WorkerSpec {
        // Example mappings for development (NOT canonical - see struct docs)
        let (provider, model) = if ctx.local_only || self.prefer_local {
            match role {
                Role::Architect | Role::Judge => ("local", "claude-code"),
                Role::Implementer => ("local", "claude-code"),
                Role::Validator => ("local", "claude-code"),
                Role::SidecarCritic => ("local", "claude-code"),
                Role::SecurityReviewer => ("local", "claude-code"),
                Role::PerformanceReviewer => ("local", "claude-code"),
                Role::Librarian => ("local", "claude-code"),
            }
        } else {
            match role {
                Role::Architect => ("anthropic", "claude-sonnet-4"),
                Role::Implementer => ("anthropic", "claude-sonnet-4"),
                Role::Validator => ("anthropic", "claude-sonnet-4"),
                Role::Judge => ("anthropic", "claude-opus-4"),
                Role::SidecarCritic => ("anthropic", "claude-haiku-4"),
                Role::SecurityReviewer => ("anthropic", "claude-sonnet-4"),
                Role::PerformanceReviewer => ("anthropic", "claude-haiku-4"),
                Role::Librarian => ("google", "gemini-2.0-flash"),
            }
        };

        let mut spec = WorkerSpec::new(role, provider, model);

        // Set appropriate permissions based on role
        spec.tool_permissions = match role {
            Role::Implementer => ToolPermissions {
                can_read_files: true,
                can_write_files: true,
                can_execute_shell: true,
                can_network: false,
                can_use_mcp: true,
            },
            Role::Validator => ToolPermissions {
                can_read_files: true,
                can_write_files: false,
                can_execute_shell: true, // For running tests
                can_network: false,
                can_use_mcp: true,
            },
            _ => ToolPermissions {
                can_read_files: true,
                can_write_files: false,
                can_execute_shell: false,
                can_network: false,
                can_use_mcp: true,
            },
        };

        // Set budget based on role complexity
        spec.budget = match role {
            Role::Architect | Role::Judge => Budget {
                max_input_tokens: 100_000,
                max_output_tokens: 16_000,
                max_cost_usd: 1.0,
                max_time_seconds: 300,
            },
            Role::Implementer => Budget {
                max_input_tokens: 150_000,
                max_output_tokens: 32_000,
                max_cost_usd: 2.0,
                max_time_seconds: 600,
            },
            _ => Budget {
                max_input_tokens: 50_000,
                max_output_tokens: 8_000,
                max_cost_usd: 0.5,
                max_time_seconds: 180,
            },
        };

        // Label for UI
        spec.label = Some(format!("{} ({})", role_to_id(role), model));

        spec
    }

    fn is_role_available(&self, role: Role, ctx: &RoutingContext) -> bool {
        // In local-only mode, only local models are available
        if ctx.local_only {
            // All roles have local fallback in default router
            true
        } else {
            // Cloud mode: all roles available
            match role {
                Role::Architect
                | Role::Implementer
                | Role::Validator
                | Role::Judge
                | Role::SidecarCritic
                | Role::Librarian => true,
                // Security/Performance reviewers might not be configured
                Role::SecurityReviewer | Role::PerformanceReviewer => !ctx.local_only,
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_spec_builder() {
        let spec = WorkerSpec::new(Role::Architect, "anthropic", "claude-sonnet-4")
            .with_kind(WorkerKind::CloudModel)
            .with_label("Architect (Sonnet)")
            .with_budget(Budget {
                max_cost_usd: 0.5,
                ..Default::default()
            });

        assert_eq!(spec.role, Role::Architect);
        assert_eq!(spec.provider, "anthropic");
        assert_eq!(spec.model, "claude-sonnet-4");
        assert_eq!(spec.label, Some("Architect (Sonnet)".into()));
        assert!((spec.budget.max_cost_usd - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_default_router_cloud() {
        let router = DefaultRouter::new();
        let ctx = RoutingContext::default();

        let architect = router.select_worker(Role::Architect, &ctx);
        assert_eq!(architect.provider, "anthropic");
        assert!(architect.model.contains("claude"));

        let implementer = router.select_worker(Role::Implementer, &ctx);
        assert!(implementer.tool_permissions.can_write_files);
        assert!(implementer.tool_permissions.can_execute_shell);

        let critic = router.select_worker(Role::SidecarCritic, &ctx);
        assert!(!critic.tool_permissions.can_write_files);
    }

    #[test]
    fn test_default_router_local_only() {
        let router = DefaultRouter::new();
        let ctx = RoutingContext {
            local_only: true,
            ..Default::default()
        };

        let architect = router.select_worker(Role::Architect, &ctx);
        assert_eq!(architect.provider, "local");
        assert_eq!(architect.model, "claude-code");
    }

    #[test]
    fn test_default_router_prefer_local() {
        let router = DefaultRouter::new().with_local_preference(true);
        let ctx = RoutingContext::default();

        let architect = router.select_worker(Role::Architect, &ctx);
        assert_eq!(architect.provider, "local");
    }

    #[test]
    fn test_routing_context_from_stage_context() {
        use crate::gate_policy::PolicyToggles;

        let stage_ctx = StageContext {
            spec_id: "SPEC-001".into(),
            stage: Some(Stage::Implement),
            local_only: true,
            is_high_risk: true,
            retry_count: 2,
            artifact_paths: vec![],
            policy: PolicyToggles::default(),
        };

        let routing_ctx = RoutingContext::from(&stage_ctx);
        assert_eq!(routing_ctx.stage, Some(Stage::Implement)); // Stage is preserved
        assert!(routing_ctx.local_only);
        assert!(routing_ctx.is_high_risk);
        assert_eq!(routing_ctx.retry_count, 2);
    }

    #[test]
    fn test_role_availability() {
        let router = DefaultRouter::new();

        let ctx = RoutingContext::default();
        assert!(router.is_role_available(Role::Architect, &ctx));
        assert!(router.is_role_available(Role::SecurityReviewer, &ctx));

        let local_ctx = RoutingContext {
            local_only: true,
            ..Default::default()
        };
        assert!(router.is_role_available(Role::Architect, &local_ctx));
    }

    #[test]
    fn test_workers_for_role_default() {
        let router = DefaultRouter::new();
        let ctx = RoutingContext::default();

        let workers = router.workers_for_role(Role::Architect, &ctx);
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].role, Role::Architect);
    }
}
