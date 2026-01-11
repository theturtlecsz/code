# SPEC-KIT-099: Research-to-Code Context Bridge

**Status**: ⛔ DEPRECATED
**Author**: Claude (Opus 4.5)
**Created**: 2025-11-30
**Deprecated**: 2025-12-14
**Effort**: Large (multi-session implementation)

> ## ⚠️ DEPRECATION NOTICE
>
> **This specification is DEPRECATED and should NOT be used for new implementation.**
>
> **For current implementation, see: [SPEC-KIT-102](../SPEC-KIT-102-notebooklm-integration/spec.md)**
>
> Key changes since this spec:
> - MCP-based integration → HTTP API (v2.0.0)
> - Multi-agent consensus → Single-owner pipeline (GR-001)
> - Execution: Architect → Implementer → Judge (no voting)
>
> Policy reference: `docs/MODEL-POLICY.md` (v1.0.0)
>
> This document is retained for historical context only. All "consensus synthesis"
> and "multi-agent debate" patterns described below are explicitly forbidden per GR-001.

---

## Executive Summary

This specification defines a **Persistent Context Layer** that ingests authoritative research ("Divine Truth") from NotebookLM via MCP and enforces it during agent coding sessions to prevent hallucination. The system treats external research as immutable legislative constraints rather than optional suggestions.

### Key Design Decision: Integrated 4-Stage Pipeline

Research ingestion is **integrated into `/speckit.auto`** as Stage 0, transforming the pipeline:

```
OLD (3-Stage):  Plan → Tasks → Implement
NEW (4-Stage):  RESEARCH → Plan → Tasks → Implement → Validate
                    ↑
                MCP-driven, automatic, with Circuit Breaker
```

**Zero Friction**: No manual `/speckit.ingest` needed—just run `/speckit.auto` and research is automatically loaded from NotebookLM.

**Universal Enforcement**: The "Divine Truth" is injected into every stage prompt, preventing Context Drift where later agents forget initial constraints.

**Graceful Degradation**: Circuit breaker handles MCP failures (skip/warn/pause/fail) and Reference Rot detection.

---

## 1. Architecture Overview

### 1.1 Context-Aware Pipeline (4-Stage)

The research ingestion is **integrated into speckit.auto** as Stage 0, transforming the pipeline from 3 stages to 4 stages with automatic context injection.

```
┌─────────────────────────────────────────────────────────────────────┐
│                    /speckit.auto SPEC-KIT-XXX                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  STAGE 0: RESEARCH (New - MCP-Driven)                              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  1. Read PRD.md from SPEC directory                         │   │
│  │  2. Call NotebookLM MCP: fetch_research_brief(spec_id, prd) │   │
│  │  3. Receive structured ResearchBrief JSON                    │   │
│  │  4. Validate code anchors (semantic hashing)                 │   │
│  │  5. Persist to .code/context/active_brief.json               │   │
│  │  6. Lock into Session State                                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│         │                                                           │
│         │ Circuit Breaker: Soft Fail (skip) or Hard Fail (pause)   │
│         ▼                                                           │
│  STAGE 1: PLAN                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  "Divine Truth" injection from active_brief.json             │   │
│  │  → Plan agent generates architecture adhering to constraints │   │
│  └─────────────────────────────────────────────────────────────┘   │
│         │                                                           │
│         ▼                                                           │
│  STAGE 2: TASKS                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Same "Divine Truth" injection (Session persistence)         │   │
│  │  → Task breakdown respects architectural decisions           │   │
│  └─────────────────────────────────────────────────────────────┘   │
│         │                                                           │
│         ▼                                                           │
│  STAGE 3: IMPLEMENT                                                 │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Same "Divine Truth" injection (no Context Drift)            │   │
│  │  → Code generated strictly follows constraints               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│         │                                                           │
│         ▼                                                           │
│  STAGE 4: VALIDATE                                                  │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Validate implementation against research constraints        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 Component Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Pipeline Coordinator                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────┐  │
│  │ ResearchStage    │───▶│ Validation       │───▶│ Session      │  │
│  │ Executor         │    │ Engine           │    │ State Lock   │  │
│  └──────────────────┘    └──────────────────┘    └──────────────┘  │
│         │                       │                       │           │
│         ▼                       ▼                       ▼           │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────┐  │
│  │ MCP Transport    │    │ Semantic Hash    │    │ Persistent   │  │
│  │ (notebooklm)     │    │ Validator        │    │ Context      │  │
│  └──────────────────┘    └──────────────────┘    └──────────────┘  │
│         │                       │                       │           │
│         └───────────────────────┴───────────────────────┘           │
│                                 │                                   │
│                                 ▼                                   │
│                    ┌──────────────────────────────────────┐        │
│                    │     Prompt Injection Layer           │        │
│                    │  (Legislative Branch - All Stages)   │        │
│                    └──────────────────────────────────────┘        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Schema Design

### 2.1 ResearchBrief (Root Structure)

```rust
// Location: codex-rs/tui/src/chatwidget/spec_kit/research_brief.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The authoritative research context ("Divine Truth") ingested from external sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBrief {
    /// Schema version for forward compatibility
    pub version: String,  // "1.0.0"

    /// Source information (MCP or local file)
    pub source: BriefSource,

    /// When this brief was ingested
    pub created_at: DateTime<Utc>,

    /// When this brief was last validated against the codebase
    pub validated_at: Option<DateTime<Utc>>,

    /// Validation status
    pub validation_status: ValidationStatus,

    /// Immutable constraints from research
    pub constraints: Vec<Constraint>,

    /// Architectural decisions with rationale
    pub decisions: Vec<ArchitecturalDecision>,

    /// Code anchors that tie research to specific code locations
    pub code_anchors: Vec<CodeAnchor>,

    /// Free-form metadata
    pub metadata: BriefMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BriefSource {
    /// Ingested via MCP from NotebookLM
    Mcp {
        notebook_id: String,
        session_id: Option<String>,
        query_used: String,
    },
    /// Imported from local file
    LocalFile {
        path: String,
        checksum: String,  // SHA-256
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationStatus {
    /// All code anchors verified against current codebase
    Valid,
    /// Some anchors have drifted (Reference Rot detected)
    Degraded {
        stale_anchors: Vec<String>,  // anchor IDs
        message: String,
    },
    /// Brief has not been validated since last edit
    Unvalidated,
    /// Validation failed completely
    Invalid { reason: String },
}
```

### 2.2 Constraint Schema

```rust
/// An immutable constraint from research that agents MUST respect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Unique identifier for tracking
    pub id: String,

    /// Human-readable title
    pub title: String,

    /// Full constraint description
    pub description: String,

    /// Constraint severity/priority
    pub severity: ConstraintSeverity,

    /// What happens if violated
    pub violation_action: ViolationAction,

    /// Optional code anchors this constraint applies to
    pub applies_to: Vec<String>,  // CodeAnchor IDs

    /// Source reference in the research
    pub source_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintSeverity {
    /// Must be followed - blocks agent if violated
    Critical,
    /// Should be followed - warns but allows override
    High,
    /// Recommendation - informational
    Medium,
    /// Suggestion - low priority
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationAction {
    /// Block the operation entirely
    Block,
    /// Warn and require explicit confirmation
    WarnAndConfirm,
    /// Log warning but continue
    WarnOnly,
    /// Just log for audit
    LogOnly,
}
```

### 2.3 Architectural Decision Schema

```rust
/// An architectural decision with full ADR-style rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalDecision {
    /// Unique identifier (ADR-style: "ADR-001")
    pub id: String,

    /// Decision title
    pub title: String,

    /// The decision itself
    pub decision: String,

    /// Why this decision was made
    pub rationale: String,

    /// What alternatives were considered
    pub alternatives_considered: Vec<String>,

    /// What are the consequences/tradeoffs
    pub consequences: Vec<String>,

    /// Current status
    pub status: DecisionStatus,

    /// Related code anchors
    pub related_anchors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded { by: String },
}
```

### 2.4 Code Anchor Schema

```rust
/// A reference from research to specific code location
/// Used for validation (detecting Reference Rot)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnchor {
    /// Unique identifier
    pub id: String,

    /// Human-readable label
    pub label: String,

    /// The anchor type
    pub anchor_type: AnchorType,

    /// Semantic hash for drift detection
    pub semantic_hash: SemanticHash,

    /// When this anchor was last verified
    pub last_verified: Option<DateTime<Utc>>,

    /// Current verification status
    pub status: AnchorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnchorType {
    /// Reference to a function/method signature
    FunctionSignature {
        file_path: String,
        function_name: String,
        /// Full signature for matching: "fn foo(x: i32) -> Result<T, E>"
        signature: String,
    },
    /// Reference to a struct/type definition
    TypeDefinition {
        file_path: String,
        type_name: String,
        /// Field names for structural matching
        fields: Vec<String>,
    },
    /// Reference to a module/file
    Module {
        path: String,
        /// Key exports for matching
        exports: Vec<String>,
    },
    /// Reference to a code pattern (regex-based)
    Pattern {
        file_glob: String,
        pattern: String,  // regex
        description: String,
    },
    /// Reference to a configuration value
    Config {
        file_path: String,
        key_path: String,  // e.g., "database.pool_size"
        expected_type: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticHash {
    /// Hash algorithm used
    pub algorithm: String,  // "sha256"

    /// The hash value
    pub hash: String,

    /// What was hashed (for debugging)
    pub source_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnchorStatus {
    /// Anchor verified against current codebase
    Verified,
    /// Anchor exists but has drifted (signature changed)
    Drifted {
        expected_hash: String,
        actual_hash: String,
    },
    /// Anchor target not found in codebase
    Missing,
    /// Not yet verified
    Unverified,
}
```

### 2.5 Brief Metadata

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BriefMetadata {
    /// Project this brief applies to
    pub project_name: Option<String>,

    /// Git repository root (for relative paths)
    pub repo_root: Option<String>,

    /// Git branch when brief was created
    pub branch: Option<String>,

    /// Commit hash when brief was created
    pub commit_hash: Option<String>,

    /// Custom tags for organization
    pub tags: Vec<String>,

    /// Original query/prompt used to generate brief
    pub generation_prompt: Option<String>,

    /// Free-form notes
    pub notes: Option<String>,

    /// Extension fields for future use
    pub extensions: HashMap<String, serde_json::Value>,
}
```

---

## 3. Component Interaction

### 3.1 New Files to Create

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/spec_kit/research_brief.rs` | Schema definitions |
| `tui/src/chatwidget/spec_kit/brief_ingest.rs` | Ingestion logic (MCP + file) |
| `tui/src/chatwidget/spec_kit/brief_validator.rs` | Semantic hash validation |
| `tui/src/chatwidget/spec_kit/brief_injector.rs` | Prompt injection logic |
| `tui/src/chatwidget/spec_kit/commands/ingest.rs` | `/speckit.ingest` command |
| `tui/src/chatwidget/spec_kit/commands/brief.rs` | `/speckit.brief` status command |

### 3.2 Files to Modify

| File | Modification |
|------|--------------|
| `tui/src/chatwidget/spec_kit/mod.rs` | Add new module exports |
| `tui/src/chatwidget/spec_kit/commands/mod.rs` | Register new commands |
| `tui/src/chatwidget/spec_kit/command_registry.rs` | Add to `SPEC_KIT_REGISTRY` |
| `core/src/slash_commands.rs` | Integrate brief injection into `format_subagent_command()` |
| `core/src/config.rs` | Add `ContextBridgeConfig` to `Config` |
| `core/src/config_types.rs` | Define `ContextBridgeConfig` struct |

### 3.3 MCP Integration Flow

```
/speckit.ingest "How should authentication be implemented?"
        │
        ▼
┌──────────────────────────────────────────────────────────────┐
│ IngestCommand::execute()                                     │
│   1. Check MCP server availability                           │
│   2. Build structured query for NotebookLM                   │
│   3. Call mcp__notebooklm__ask_question()                    │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────┐
│ McpConnectionManager::call_tool()                            │
│   server: "notebooklm"                                       │
│   tool: "ask_question"                                       │
│   arguments: {                                               │
│     "question": structured_query,                            │
│     "notebook_id": active_notebook_id                        │
│   }                                                          │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────┐
│ Response Processing                                          │
│   1. Parse NotebookLM response                               │
│   2. Extract constraints, decisions, anchors                 │
│   3. Generate semantic hashes for anchors                    │
│   4. Validate anchors against current codebase               │
│   5. Construct ResearchBrief                                 │
│   6. Persist to .code/context/active_brief.json              │
└──────────────────────────────────────────────────────────────┘
```

### 3.4 Command Implementation Pattern

```rust
// Location: tui/src/chatwidget/spec_kit/commands/ingest.rs

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use super::super::brief_ingest::{IngestSource, ingest_research_brief};

/// Command: /speckit.ingest
/// Ingest research from NotebookLM or local file
pub struct SpecKitIngestCommand;

impl SpecKitCommand for SpecKitIngestCommand {
    fn name(&self) -> &'static str {
        "speckit.ingest"
    }

    fn aliases(&self) -> &[&'static str] {
        &["ingest-research"]
    }

    fn description(&self) -> &'static str {
        "ingest research context from NotebookLM or local file"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // Parse args to determine source
        let source = if args.starts_with("file:") {
            let path = args.strip_prefix("file:").unwrap().trim();
            IngestSource::LocalFile(path.to_string())
        } else {
            // Default to MCP query
            IngestSource::McpQuery(args)
        };

        // Delegate to async ingest handler
        widget.handle_research_ingest(source);
    }

    fn requires_args(&self) -> bool {
        true
    }
}
```

---

## 4. Persistence Strategy

### 4.1 Directory Structure

```
.code/
├── context/
│   ├── active_brief.json          # Currently active ResearchBrief
│   ├── brief_history/             # Historical briefs for audit
│   │   ├── 2025-11-30T10-30-00_auth.json
│   │   └── 2025-11-29T15-45-00_database.json
│   └── anchor_cache/              # Cached semantic hashes
│       ├── functions.json
│       └── types.json
└── ace/
    └── playbooks_normalized.sqlite3  # Existing ACE storage
```

### 4.2 Persistence Operations

```rust
// Location: tui/src/chatwidget/spec_kit/brief_ingest.rs

use std::path::{Path, PathBuf};
use std::fs;

const CONTEXT_DIR: &str = ".code/context";
const ACTIVE_BRIEF_FILE: &str = "active_brief.json";
const HISTORY_DIR: &str = "brief_history";

/// Get the context directory path
pub fn context_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(CONTEXT_DIR)
}

/// Get the active brief path
pub fn active_brief_path(repo_root: &Path) -> PathBuf {
    context_dir(repo_root).join(ACTIVE_BRIEF_FILE)
}

/// Load the active research brief (if any)
pub fn load_active_brief(repo_root: &Path) -> Option<ResearchBrief> {
    let path = active_brief_path(repo_root);
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save a research brief as the active brief
pub fn save_active_brief(repo_root: &Path, brief: &ResearchBrief) -> std::io::Result<()> {
    let dir = context_dir(repo_root);
    fs::create_dir_all(&dir)?;

    let path = active_brief_path(repo_root);
    let content = serde_json::to_string_pretty(brief)?;
    fs::write(&path, content)?;

    // Also archive to history
    archive_brief(repo_root, brief)?;

    Ok(())
}

/// Archive a brief to history
fn archive_brief(repo_root: &Path, brief: &ResearchBrief) -> std::io::Result<()> {
    let history_dir = context_dir(repo_root).join(HISTORY_DIR);
    fs::create_dir_all(&history_dir)?;

    let timestamp = brief.created_at.format("%Y-%m-%dT%H-%M-%S");
    let filename = format!("{}_{}.json",
        timestamp,
        sanitize_filename(&brief.metadata.project_name.clone().unwrap_or_default())
    );

    let path = history_dir.join(filename);
    let content = serde_json::to_string_pretty(brief)?;
    fs::write(&path, content)
}
```

### 4.3 Session State Integration

```rust
// Addition to: tui/src/chatwidget/mod.rs

pub struct ChatWidget {
    // ... existing fields ...

    /// Active research brief (loaded from .code/context/active_brief.json)
    /// Cached in memory for prompt injection performance
    active_research_brief: Option<ResearchBrief>,

    /// Whether the brief has been validated this session
    brief_validated_this_session: bool,
}

impl ChatWidget {
    /// Load research brief on widget initialization
    fn load_research_brief(&mut self) {
        if let Some(repo_root) = self.get_repo_root() {
            self.active_research_brief = load_active_brief(&repo_root);
            self.brief_validated_this_session = false;
        }
    }

    /// Get the active research brief (triggers validation if needed)
    pub fn get_active_brief(&mut self) -> Option<&ResearchBrief> {
        if !self.brief_validated_this_session {
            if let Some(ref mut brief) = self.active_research_brief {
                // Validate anchors on first access
                validate_brief_anchors(brief, self.config.cwd.as_path());
                self.brief_validated_this_session = true;
            }
        }
        self.active_research_brief.as_ref()
    }
}
```

---

## 5. Validation Engine (Semantic Hashing)

### 5.1 Hash Generation

```rust
// Location: tui/src/chatwidget/spec_kit/brief_validator.rs

use sha2::{Sha256, Digest};
use std::path::Path;

/// Generate semantic hash for a function signature
pub fn hash_function_signature(
    file_path: &Path,
    function_name: &str,
) -> Option<SemanticHash> {
    let content = std::fs::read_to_string(file_path).ok()?;

    // Extract function signature using regex or tree-sitter
    let signature = extract_function_signature(&content, function_name)?;

    // Normalize signature (remove whitespace, comments)
    let normalized = normalize_signature(&signature);

    // Hash
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    Some(SemanticHash {
        algorithm: "sha256".to_string(),
        hash,
        source_description: format!("{}::{}", file_path.display(), function_name),
    })
}

/// Extract function signature from Rust source
fn extract_function_signature(content: &str, function_name: &str) -> Option<String> {
    // Pattern: pub? async? fn name<generics>?(params) -> ReturnType
    let pattern = format!(
        r"(?:pub\s+)?(?:async\s+)?fn\s+{}\s*(?:<[^>]*>)?\s*\([^)]*\)\s*(?:->\s*[^{{]+)?",
        regex::escape(function_name)
    );

    let re = regex::Regex::new(&pattern).ok()?;
    re.find(content).map(|m| m.as_str().to_string())
}

/// Normalize signature for consistent hashing
fn normalize_signature(sig: &str) -> String {
    sig.split_whitespace()
       .collect::<Vec<_>>()
       .join(" ")
}
```

### 5.2 Validation Flow

```rust
/// Validate all code anchors in a brief against the current codebase
pub fn validate_brief_anchors(brief: &mut ResearchBrief, repo_root: &Path) {
    let mut stale_anchors = Vec::new();

    for anchor in &mut brief.code_anchors {
        let current_hash = match &anchor.anchor_type {
            AnchorType::FunctionSignature { file_path, function_name, .. } => {
                let full_path = repo_root.join(file_path);
                hash_function_signature(&full_path, function_name)
            }
            AnchorType::TypeDefinition { file_path, type_name, .. } => {
                let full_path = repo_root.join(file_path);
                hash_type_definition(&full_path, type_name)
            }
            AnchorType::Module { path, exports } => {
                let full_path = repo_root.join(path);
                hash_module(&full_path, exports)
            }
            AnchorType::Pattern { file_glob, pattern, .. } => {
                hash_pattern(repo_root, file_glob, pattern)
            }
            AnchorType::Config { file_path, key_path, .. } => {
                let full_path = repo_root.join(file_path);
                hash_config_value(&full_path, key_path)
            }
        };

        match current_hash {
            Some(hash) if hash.hash == anchor.semantic_hash.hash => {
                anchor.status = AnchorStatus::Verified;
                anchor.last_verified = Some(Utc::now());
            }
            Some(hash) => {
                anchor.status = AnchorStatus::Drifted {
                    expected_hash: anchor.semantic_hash.hash.clone(),
                    actual_hash: hash.hash,
                };
                stale_anchors.push(anchor.id.clone());
            }
            None => {
                anchor.status = AnchorStatus::Missing;
                stale_anchors.push(anchor.id.clone());
            }
        }
    }

    // Update brief validation status
    if stale_anchors.is_empty() {
        brief.validation_status = ValidationStatus::Valid;
    } else {
        brief.validation_status = ValidationStatus::Degraded {
            stale_anchors,
            message: "Some code references have changed since research was ingested".to_string(),
        };
    }

    brief.validated_at = Some(Utc::now());
}
```

---

## 6. Prompt Injection Strategy

### 6.1 Injection Priority Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│ PRIORITY 1: System Base Instructions                        │
│ (model safety, core behavior)                               │
├─────────────────────────────────────────────────────────────┤
│ PRIORITY 2: RESEARCH BRIEF CONSTRAINTS [LEGISLATIVE]        │
│ (immutable, from /speckit.ingest - "Divine Truth")          │
├─────────────────────────────────────────────────────────────┤
│ PRIORITY 3: Project Documentation (AGENTS.md)               │
│ (project-level instructions)                                │
├─────────────────────────────────────────────────────────────┤
│ PRIORITY 4: ACE Playbook Heuristics                         │
│ (learned patterns, softer guidance)                         │
├─────────────────────────────────────────────────────────────┤
│ PRIORITY 5: Command-Specific Instructions                   │
│ (from SubagentCommandConfig)                                │
├─────────────────────────────────────────────────────────────┤
│ PRIORITY 6: User Task                                       │
│ (the actual request)                                        │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 Legislative Framing

```rust
// Location: tui/src/chatwidget/spec_kit/brief_injector.rs

/// Format research brief as legislative constraints for prompt injection
pub fn format_legislative_section(brief: &ResearchBrief) -> String {
    let mut lines = Vec::new();

    // Header with strong framing
    lines.push("## LEGISLATIVE CONSTRAINTS (BINDING)".to_string());
    lines.push(String::new());
    lines.push("The following constraints are derived from authoritative research and".to_string());
    lines.push("MUST be treated as immutable requirements. Do not deviate from these".to_string());
    lines.push("constraints without explicit user override.".to_string());
    lines.push(String::new());

    // Validation status warning
    if let ValidationStatus::Degraded { message, stale_anchors } = &brief.validation_status {
        lines.push(format!("⚠️ WARNING: Reference Rot Detected"));
        lines.push(format!("   {}", message));
        lines.push(format!("   Stale anchors: {}", stale_anchors.join(", ")));
        lines.push(String::new());
    }

    // Critical constraints first
    let critical: Vec<_> = brief.constraints.iter()
        .filter(|c| matches!(c.severity, ConstraintSeverity::Critical))
        .collect();

    if !critical.is_empty() {
        lines.push("### CRITICAL CONSTRAINTS (BLOCKING)".to_string());
        for constraint in critical {
            lines.push(format!("- **{}**: {}", constraint.title, constraint.description));
            if let Some(ref source) = constraint.source_reference {
                lines.push(format!("  Source: {}", source));
            }
        }
        lines.push(String::new());
    }

    // High priority constraints
    let high: Vec<_> = brief.constraints.iter()
        .filter(|c| matches!(c.severity, ConstraintSeverity::High))
        .collect();

    if !high.is_empty() {
        lines.push("### HIGH PRIORITY CONSTRAINTS".to_string());
        for constraint in high {
            lines.push(format!("- **{}**: {}", constraint.title, constraint.description));
        }
        lines.push(String::new());
    }

    // Architectural decisions
    let accepted: Vec<_> = brief.decisions.iter()
        .filter(|d| matches!(d.status, DecisionStatus::Accepted))
        .collect();

    if !accepted.is_empty() {
        lines.push("### ARCHITECTURAL DECISIONS (ACCEPTED)".to_string());
        for decision in accepted {
            lines.push(format!("- **{}** ({}): {}", decision.id, decision.title, decision.decision));
            lines.push(format!("  Rationale: {}", decision.rationale));
        }
        lines.push(String::new());
    }

    lines.push("---".to_string());
    lines.push(String::new());

    lines.join("\n")
}
```

### 6.3 Integration with `format_subagent_command()`

```rust
// Modification to: core/src/slash_commands.rs

pub fn format_subagent_command(
    name: &str,
    task: &str,
    agents: Option<&[AgentConfig]>,
    commands: Option<&[SubagentCommandConfig]>,
    research_brief: Option<&ResearchBrief>,  // NEW PARAMETER
) -> SubagentResolution {
    // ... existing code ...

    // Build legislative section from research brief
    let legislative_section = research_brief
        .map(|brief| format_legislative_section(brief))
        .unwrap_or_default();

    // Compose unified prompt with legislative layer
    let prompt = format!(
        "Please perform /{name} using the <constraints>, <tools>, <instructions> and <task> below.\n\
        <constraints>\n{legislative_section}</constraints>\n\
        <tools>\n{tools_section}</tools>\n\
        <instructions>\n{instr_text}</instructions>\n\
        <task>\n{task}</task>",
        legislative_section = legislative_section,
        tools_section = tools_section,
        instr_text = instr_text,
        task = task,
    );

    // ... rest of function ...
}
```

### 6.4 Constraint Enforcement Hooks

```rust
// Location: tui/src/chatwidget/spec_kit/brief_injector.rs

/// Check if an operation would violate critical constraints
pub fn check_constraint_violations(
    brief: &ResearchBrief,
    operation: &str,  // e.g., "create file", "modify function"
    target: &str,     // e.g., file path, function name
) -> Vec<ConstraintViolation> {
    let mut violations = Vec::new();

    for constraint in &brief.constraints {
        if matches!(constraint.severity, ConstraintSeverity::Critical) {
            // Check if operation targets an anchored location
            for anchor_id in &constraint.applies_to {
                if let Some(anchor) = brief.code_anchors.iter().find(|a| &a.id == anchor_id) {
                    if anchor_matches_target(anchor, target) {
                        violations.push(ConstraintViolation {
                            constraint_id: constraint.id.clone(),
                            constraint_title: constraint.title.clone(),
                            action: constraint.violation_action.clone(),
                            message: format!(
                                "Operation '{}' on '{}' may violate constraint: {}",
                                operation, target, constraint.description
                            ),
                        });
                    }
                }
            }
        }
    }

    violations
}

#[derive(Debug, Clone)]
pub struct ConstraintViolation {
    pub constraint_id: String,
    pub constraint_title: String,
    pub action: ViolationAction,
    pub message: String,
}
```

---

## 7. MCP Query Protocol

### 7.1 NotebookLM Query Structure

```rust
/// Build a structured query for NotebookLM that extracts ResearchBrief data
pub fn build_research_query(user_query: &str) -> String {
    format!(r#"
Based on the research materials, provide a structured analysis for: "{}"

Please respond with the following sections:

## Constraints
List any hard requirements, rules, or limitations that MUST be followed.
Format each as:
- CRITICAL: [title] - [description]
- HIGH: [title] - [description]

## Architectural Decisions
List any architectural decisions that have been made with rationale.
Format each as:
- ADR-XXX: [title]
  - Decision: [what was decided]
  - Rationale: [why]
  - Alternatives: [what else was considered]

## Code References
List any specific code patterns, functions, types, or files mentioned.
Format each as:
- Function: [name] in [file] - [signature if known]
- Type: [name] in [file] - [key fields]
- Pattern: [description] - [where it applies]

## Implementation Guidance
Any other relevant implementation guidance not captured above.
"#, user_query)
}
```

### 7.2 Response Parsing

```rust
/// Parse NotebookLM response into ResearchBrief components
pub fn parse_research_response(response: &str) -> ParsedResearch {
    let mut constraints = Vec::new();
    let mut decisions = Vec::new();
    let mut anchors = Vec::new();

    // Parse sections using regex or structured parsing
    let constraint_section = extract_section(response, "## Constraints");
    let decision_section = extract_section(response, "## Architectural Decisions");
    let code_section = extract_section(response, "## Code References");

    // Parse constraints
    if let Some(section) = constraint_section {
        for line in section.lines() {
            if let Some(constraint) = parse_constraint_line(line) {
                constraints.push(constraint);
            }
        }
    }

    // Parse decisions
    if let Some(section) = decision_section {
        for decision in parse_decisions(&section) {
            decisions.push(decision);
        }
    }

    // Parse code anchors
    if let Some(section) = code_section {
        for anchor in parse_code_references(&section) {
            anchors.push(anchor);
        }
    }

    ParsedResearch {
        constraints,
        decisions,
        anchors,
        raw_guidance: extract_section(response, "## Implementation Guidance"),
    }
}
```

---

## 8. Research Stage Executor

### 8.1 Stage Integration with PipelineCoordinator

```rust
// Location: tui/src/chatwidget/spec_kit/research_stage.rs

use super::pipeline_coordinator::StageExecutor;
use super::research_brief::{ResearchBrief, ValidationStatus};
use super::brief_ingest::{ingest_from_mcp, save_active_brief};
use super::brief_validator::validate_brief_anchors;
use crate::config_types::{CircuitBreakerAction, ResearchStageConfig};

/// Research Stage Executor - Stage 0 of the Context-Aware Pipeline
pub struct ResearchStageExecutor {
    config: ResearchStageConfig,
}

impl ResearchStageExecutor {
    pub fn new(config: ResearchStageConfig) -> Self {
        Self { config }
    }

    /// Execute the research stage
    pub async fn execute(
        &self,
        spec_id: &str,
        prd_content: &str,
        repo_root: &Path,
    ) -> ResearchStageResult {
        tracing::info!("STAGE 0: RESEARCH - Starting for {}", spec_id);

        // Step 1: Call NotebookLM MCP
        let brief = match self.fetch_research_brief(spec_id, prd_content).await {
            Ok(brief) => brief,
            Err(e) => return self.handle_mcp_failure(e),
        };

        // Step 2: Validate code anchors
        let mut brief = brief;
        validate_brief_anchors(&mut brief, repo_root);

        // Step 3: Check for Reference Rot
        if let ValidationStatus::Degraded { stale_anchors, message } = &brief.validation_status {
            tracing::warn!("Reference Rot detected: {}", message);
            match self.config.on_reference_rot {
                CircuitBreakerAction::Pause => {
                    return ResearchStageResult::PauseForConfirmation {
                        brief,
                        reason: format!(
                            "⚠️ Reference Rot Detected\n{}\nStale anchors: {:?}",
                            message, stale_anchors
                        ),
                    };
                }
                CircuitBreakerAction::Fail => {
                    return ResearchStageResult::Failed {
                        reason: format!("Reference Rot: {}", message),
                    };
                }
                CircuitBreakerAction::Warn | CircuitBreakerAction::Skip => {
                    tracing::warn!("Proceeding despite Reference Rot");
                }
                CircuitBreakerAction::Prompt => {
                    return ResearchStageResult::PauseForConfirmation {
                        brief,
                        reason: "Reference Rot detected - confirm to proceed".to_string(),
                    };
                }
            }
        }

        // Step 4: Persist to .code/context/active_brief.json
        if let Err(e) = save_active_brief(repo_root, &brief) {
            tracing::error!("Failed to persist research brief: {}", e);
        }

        // Step 5: Return success with brief locked into session
        ResearchStageResult::Success { brief }
    }

    async fn fetch_research_brief(
        &self,
        spec_id: &str,
        prd_content: &str,
    ) -> Result<ResearchBrief, McpError> {
        ingest_from_mcp(
            &self.config.tool,
            spec_id,
            prd_content,
            self.config.notebook_id.as_deref(),
            self.config.query_template.as_deref(),
        ).await
    }

    fn handle_mcp_failure(&self, error: McpError) -> ResearchStageResult {
        tracing::warn!("MCP unavailable: {}", error);

        match self.config.on_mcp_unavailable {
            CircuitBreakerAction::Skip => {
                tracing::info!("⚠️ Research unavailable - Proceeding with standard context");
                ResearchStageResult::Skipped {
                    reason: format!("MCP unavailable: {}", error),
                }
            }
            CircuitBreakerAction::Fail => {
                ResearchStageResult::Failed {
                    reason: format!("MCP required but unavailable: {}", error),
                }
            }
            CircuitBreakerAction::Pause | CircuitBreakerAction::Prompt => {
                ResearchStageResult::PauseForConfirmation {
                    brief: ResearchBrief::default(),
                    reason: format!("MCP unavailable: {}. Continue without research?", error),
                }
            }
            CircuitBreakerAction::Warn => {
                tracing::warn!("MCP unavailable, continuing with warning");
                ResearchStageResult::Skipped {
                    reason: format!("MCP unavailable (warned): {}", error),
                }
            }
        }
    }
}

/// Result of the Research Stage execution
#[derive(Debug)]
pub enum ResearchStageResult {
    /// Research loaded successfully, brief locked into session
    Success {
        brief: ResearchBrief,
    },
    /// Stage skipped (soft fail), proceed without research
    Skipped {
        reason: String,
    },
    /// Pipeline paused, waiting for user confirmation
    PauseForConfirmation {
        brief: ResearchBrief,
        reason: String,
    },
    /// Stage failed (hard fail), pipeline should abort
    Failed {
        reason: String,
    },
}
```

### 8.2 Integration with SpecAutoState

```rust
// Modification to: tui/src/chatwidget/spec_kit/state.rs

pub struct SpecAutoState {
    // ... existing fields ...

    /// Active research brief loaded during Stage 0
    /// Injected into all subsequent stage prompts
    pub active_research_brief: Option<ResearchBrief>,

    /// Whether research stage was skipped (for UI display)
    pub research_skipped: bool,

    /// Research skip reason (if skipped)
    pub research_skip_reason: Option<String>,
}

impl SpecAutoState {
    /// Lock research brief into session state
    pub fn lock_research_brief(&mut self, brief: ResearchBrief) {
        self.active_research_brief = Some(brief);
        self.research_skipped = false;
        self.research_skip_reason = None;
    }

    /// Mark research as skipped
    pub fn mark_research_skipped(&mut self, reason: String) {
        self.active_research_brief = None;
        self.research_skipped = true;
        self.research_skip_reason = Some(reason);
    }

    /// Get research brief for prompt injection
    pub fn get_research_brief(&self) -> Option<&ResearchBrief> {
        self.active_research_brief.as_ref()
    }
}
```

### 8.3 Pipeline Coordinator Integration

```rust
// Modification to: tui/src/chatwidget/spec_kit/pipeline_coordinator.rs

impl PipelineCoordinator {
    /// Execute the full context-aware pipeline
    pub async fn execute_auto_pipeline(&mut self, spec_id: &str) -> PipelineResult {
        // Load PRD content
        let prd_content = self.load_prd_content(spec_id)?;

        // Stage 0: Research (if enabled in config)
        if self.config.stages.contains(&"research".to_string()) {
            let research_executor = ResearchStageExecutor::new(
                self.config.research_stage.clone()
            );

            match research_executor.execute(spec_id, &prd_content, &self.repo_root).await {
                ResearchStageResult::Success { brief } => {
                    self.state.lock_research_brief(brief);
                    self.emit_stage_complete("research", StageOutcome::Success);
                }
                ResearchStageResult::Skipped { reason } => {
                    self.state.mark_research_skipped(reason.clone());
                    self.emit_stage_warning("research", &reason);
                }
                ResearchStageResult::PauseForConfirmation { brief, reason } => {
                    // Show confirmation modal, wait for user input
                    return PipelineResult::PausedForConfirmation {
                        stage: "research".to_string(),
                        reason,
                        pending_brief: Some(brief),
                    };
                }
                ResearchStageResult::Failed { reason } => {
                    return PipelineResult::Failed {
                        stage: "research".to_string(),
                        reason,
                    };
                }
            }
        }

        // Stage 1: Plan (with research injection)
        self.execute_plan_stage(spec_id).await?;

        // Stage 2: Tasks (with research injection)
        self.execute_tasks_stage(spec_id).await?;

        // Stage 3: Implement (with research injection)
        self.execute_implement_stage(spec_id).await?;

        // Stage 4: Validate (with research injection)
        self.execute_validate_stage(spec_id).await?;

        PipelineResult::Complete
    }
}
```

---

## 9. Command Reference

### 9.1 Primary Flow: `/speckit.auto` (Integrated Research)

The primary flow uses `/speckit.auto` with automatic research ingestion:

```
Usage: /speckit.auto SPEC-ID [options]

Execute the 4-stage context-aware pipeline:
  Stage 0: Research (MCP-driven, automatic)
  Stage 1: Plan
  Stage 2: Tasks
  Stage 3: Implement
  Stage 4: Validate

Examples:
  /speckit.auto SPEC-KIT-099              # Full pipeline with research
  /speckit.auto SPEC-KIT-099 --from plan  # Skip research, start at plan
  /speckit.auto SPEC-KIT-099 --no-research  # Disable research stage

Flags:
  --from <stage>     Start from specific stage (skips prior stages)
  --no-research      Skip research stage entirely
  --notebook <id>    Override default NotebookLM notebook
  --research-only    Run only the research stage, then stop
```

### 9.2 Manual Override: `/speckit.ingest`

For manual research ingestion outside the pipeline:

```
Usage: /speckit.ingest <query>
       /speckit.ingest file:<path>

Manually ingest research context (use when not running /speckit.auto).

Arguments:
  <query>       Natural language query for NotebookLM
  file:<path>   Import from local JSON file

Examples:
  /speckit.ingest How should user authentication be implemented?
  /speckit.ingest file:./research/auth-design.json

Flags:
  --notebook <id>   Use specific NotebookLM notebook
  --force           Overwrite existing brief without confirmation
  --validate        Validate anchors immediately after ingest

Note: When using /speckit.auto, research is automatically ingested.
      Use /speckit.ingest only for:
      - Pre-loading research before running individual stage commands
      - Importing research from local files
      - Updating research mid-session
```

### 9.3 `/speckit.brief`

```
Usage: /speckit.brief [subcommand]

Manage active research brief.

Subcommands:
  status      Show current brief status and validation
  validate    Re-validate all code anchors against current codebase
  clear       Remove active brief from session
  export      Export brief to file
  history     Show brief history

Examples:
  /speckit.brief status
  /speckit.brief validate
  /speckit.brief export ./backup/brief.json
  /speckit.brief clear
```

---

## 10. Configuration

### 10.1 Pipeline Stage Configuration

The pipeline stages are now configurable, with "research" as the new Stage 0:

```toml
# .code/config.toml

[speckit.pipeline.auto]
# The context-aware automated sequence (4-stage)
stages = [
  "research",    # Stage 0: MCP-driven research ingestion
  "plan",        # Stage 1: Architecture planning
  "tasks",       # Stage 2: Task breakdown
  "implement",   # Stage 3: Code generation
  "validate"     # Stage 4: Validation
]

[speckit.stage.research]
# MCP provider configuration
provider = "mcp"
tool = "notebooklm"

# Circuit breaker behavior
on_mcp_unavailable = "skip"      # "skip" | "fail" | "prompt"
on_reference_rot = "pause"       # "pause" | "warn" | "fail"

# NotebookLM notebook selection
notebook_id = "default"          # Use default from context_bridge config
# notebook_id = "notebook_abc123" # Or specify explicit ID

# Query template for PRD analysis
query_template = """
Analyze the following PRD against the knowledge base and extract:
1. Constraints (CRITICAL/HIGH/MEDIUM)
2. Architectural decisions with rationale
3. Code references and patterns
4. Implementation guidance

PRD Content:
{prd_content}

SPEC ID: {spec_id}
"""
```

### 10.2 Context Bridge Config

```rust
// Addition to: core/src/config_types.rs

/// Configuration for the Research-to-Code Context Bridge
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContextBridgeConfig {
    /// Enable research brief functionality
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Auto-validate anchors on session start
    #[serde(default = "default_true")]
    pub auto_validate: bool,

    /// Warn on degraded briefs
    #[serde(default = "default_true")]
    pub warn_on_degraded: bool,

    /// Block operations on critical constraint violations
    #[serde(default = "default_true")]
    pub enforce_critical_constraints: bool,

    /// Default NotebookLM notebook ID
    pub default_notebook_id: Option<String>,

    /// Maximum constraint count in prompt (to avoid context overflow)
    #[serde(default = "default_max_constraints")]
    pub max_constraints_in_prompt: usize,

    /// Archive briefs to history
    #[serde(default = "default_true")]
    pub archive_briefs: bool,
}

/// Research stage configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResearchStageConfig {
    /// MCP provider type
    #[serde(default = "default_provider")]
    pub provider: String,

    /// MCP tool name
    #[serde(default = "default_tool")]
    pub tool: String,

    /// Behavior when MCP is unavailable
    #[serde(default)]
    pub on_mcp_unavailable: CircuitBreakerAction,

    /// Behavior when Reference Rot is detected
    #[serde(default = "default_pause")]
    pub on_reference_rot: CircuitBreakerAction,

    /// NotebookLM notebook ID (or "default")
    pub notebook_id: Option<String>,

    /// Query template for PRD analysis
    pub query_template: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CircuitBreakerAction {
    /// Skip the stage, log warning, continue pipeline
    #[default]
    Skip,
    /// Pause pipeline, prompt user for confirmation
    Pause,
    /// Warn but continue without pausing
    Warn,
    /// Fail the entire pipeline
    Fail,
    /// Prompt user interactively
    Prompt,
}

fn default_true() -> bool { true }
fn default_max_constraints() -> usize { 20 }
fn default_provider() -> String { "mcp".to_string() }
fn default_tool() -> String { "notebooklm".to_string() }
fn default_pause() -> CircuitBreakerAction { CircuitBreakerAction::Pause }
```

### 10.3 Full TOML Example

```toml
# ~/.code/config.toml

[context_bridge]
enabled = true
auto_validate = true
warn_on_degraded = true
enforce_critical_constraints = true
default_notebook_id = "notebook_abc123"
max_constraints_in_prompt = 20
archive_briefs = true

[speckit.pipeline.auto]
stages = ["research", "plan", "tasks", "implement", "validate"]

[speckit.stage.research]
provider = "mcp"
tool = "notebooklm"
on_mcp_unavailable = "skip"
on_reference_rot = "pause"
```

---

## 11. Testing Strategy

### 11.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_hash_function() {
        let source = r#"
pub fn calculate_total(items: &[Item], tax_rate: f64) -> f64 {
    items.iter().map(|i| i.price).sum::<f64>() * (1.0 + tax_rate)
}
"#;
        // Write to temp file and hash
        // Verify hash is deterministic
    }

    #[test]
    fn test_constraint_parsing() {
        let line = "- CRITICAL: No SQL Injection - All queries must use parameterized statements";
        let constraint = parse_constraint_line(line);
        assert!(constraint.is_some());
        assert_eq!(constraint.unwrap().severity, ConstraintSeverity::Critical);
    }

    #[test]
    fn test_anchor_validation_detects_drift() {
        // Create brief with known anchor
        // Modify source file
        // Validate and confirm drift detected
    }

    #[test]
    fn test_legislative_formatting() {
        let brief = create_test_brief();
        let section = format_legislative_section(&brief);
        assert!(section.contains("LEGISLATIVE CONSTRAINTS"));
        assert!(section.contains("CRITICAL"));
    }
}
```

### 11.2 Integration Tests

```rust
#[tokio::test]
async fn test_mcp_ingest_flow() {
    // Mock MCP server
    // Execute /speckit.ingest
    // Verify brief created and persisted
}

#[tokio::test]
async fn test_prompt_injection_with_brief() {
    // Load test brief
    // Execute /speckit.implement
    // Verify prompt contains legislative section
}
```

---

## 12. Implementation Blueprint (Rust Architecture)

This section provides concrete implementation guidance for the Principal Rust Architect.

### 12.1 The Four Pillars

#### Pillar 1: The Data Contract (`ResearchBrief`)

**Module**: `codex-rs/core/src/research/schema.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBrief {
    pub metadata: BriefMetadata,
    pub architectural_decisions: Vec<Adr>,
    pub constraints: Vec<String>,  // Immutable laws, e.g., "No unwrap()"
    pub code_snippets: Vec<CodeAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefMetadata {
    pub title: String,
    pub date: DateTime<Utc>,
    pub source_hash: String,  // SHA-256 of source content
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adr {
    pub id: String,           // "ADR-001"
    pub title: String,
    pub status: AdrStatus,
    pub decision: String,
    pub rationale: String,
    pub consequences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnchor {
    pub file_path: String,
    pub target_symbol: String,     // e.g., "calculate_total"
    pub signature_hash: String,    // Semantic hash of function signature
}
```

#### Pillar 2: The MCP Transport Layer

**Command**: `tui/src/chatwidget/spec_kit/commands/research.rs`

```rust
pub struct ResearchCommand;

impl SpecKitCommand for ResearchCommand {
    fn name(&self) -> &'static str { "speckit.research" }
    fn aliases(&self) -> &[&'static str] { &["research"] }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // 1. Check MCP first
        if !args.contains("--file") {
            match widget.mcp_client.call_tool(
                "notebooklm",
                "fetch_brief",
                serde_json::json!({ "spec_id": args })
            ) {
                Ok(response) => { /* deserialize ResearchBrief */ }
                Err(_) => { /* fall through to file */ }
            }
        }

        // 2. Fallback: read from --file or default location
        let brief = std::fs::read_to_string(".code/context/active_brief.json")?;

        // 3. Persist
        save_active_brief(&brief)?;
    }
}
```

#### Pillar 3: Semantic Integrity ("Reference Rot" Protection)

**Module**: `codex-rs/core/src/research/validator.rs`

**Dependencies**: Add to `core/Cargo.toml`:
```toml
syn = { version = "2", features = ["full", "parsing"] }
```

```rust
use sha2::{Sha256, Digest};
use syn::{parse_file, Item};
use std::path::Path;

pub struct ValidationWarning {
    pub anchor_id: String,
    pub message: String,
    pub expected_hash: String,
    pub actual_hash: String,
}

/// Validate all code anchors against the current codebase
pub fn validate_anchors(
    brief: &ResearchBrief,
    repo_root: &Path,
) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();

    for anchor in &brief.code_snippets {
        let file_path = repo_root.join(&anchor.file_path);
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => {
                warnings.push(ValidationWarning {
                    anchor_id: anchor.target_symbol.clone(),
                    message: format!("File not found: {}", anchor.file_path),
                    expected_hash: anchor.signature_hash.clone(),
                    actual_hash: "FILE_NOT_FOUND".to_string(),
                });
                continue;
            }
        };

        // Parse with syn
        if let Ok(syntax) = parse_file(&content) {
            if let Some(actual_hash) = extract_signature_hash(&syntax, &anchor.target_symbol) {
                if actual_hash != anchor.signature_hash {
                    warnings.push(ValidationWarning {
                        anchor_id: anchor.target_symbol.clone(),
                        message: "Reference Rot detected: signature changed".to_string(),
                        expected_hash: anchor.signature_hash.clone(),
                        actual_hash,
                    });
                }
            }
        }
    }

    warnings
}

/// Extract and hash a function signature from AST
fn extract_signature_hash(syntax: &syn::File, symbol: &str) -> Option<String> {
    for item in &syntax.items {
        if let Item::Fn(func) = item {
            if func.sig.ident == symbol {
                // Normalize: strip whitespace/comments, hash signature
                let sig_str = quote::quote!(#func.sig).to_string();
                let normalized = sig_str.split_whitespace().collect::<Vec<_>>().join(" ");

                let mut hasher = Sha256::new();
                hasher.update(normalized.as_bytes());
                return Some(format!("{:x}", hasher.finalize()));
            }
        }
    }
    None
}
```

#### Pillar 4: "Divine Truth" Prompt Injection

**File**: `codex-rs/core/src/slash_commands.rs`

```rust
pub fn format_subagent_command(
    name: &str,
    task: &str,
    agents: Option<&[AgentConfig]>,
    commands: Option<&[SubagentCommandConfig]>,
    research_brief: Option<&ResearchBrief>,  // NEW PARAMETER
) -> SubagentResolution {
    // ... existing code ...

    // Inject Divine Truth at the TOP of the prompt
    let divine_truth = research_brief
        .map(|brief| format_divine_truth_injection(brief))
        .unwrap_or_default();

    let prompt = format!(
        "{divine_truth}\
        Please perform /{name} using the <tools>, <instructions> and <task> below.\n\
        ...",
        divine_truth = divine_truth,
        // ... rest of format
    );
}

fn format_divine_truth_injection(brief: &ResearchBrief) -> String {
    let constraints_json = serde_json::to_string_pretty(&brief.constraints).unwrap();
    let adrs_json = serde_json::to_string_pretty(&brief.architectural_decisions).unwrap();

    format!(r#"
!!! SYSTEM OVERRIDE: ACTIVE !!!
You are operating in STRICT ARCHITECTURAL MODE.
The following JSON is the AUTHORITATIVE SPECIFICATION.

## CONSTRAINTS (IMMUTABLE)
{constraints}

## ARCHITECTURAL DECISIONS
{adrs}

Violation of these constraints is NOT permitted.
---

"#,
        constraints = constraints_json,
        adrs = adrs_json,
    )
}
```

### 12.2 Module Structure

```
codex-rs/
├── core/
│   ├── src/
│   │   ├── research/
│   │   │   ├── mod.rs           # Module exports
│   │   │   ├── schema.rs        # ResearchBrief, Adr, CodeAnchor
│   │   │   ├── validator.rs     # Semantic hashing, Reference Rot
│   │   │   └── persistence.rs   # .code/context/ file operations
│   │   ├── slash_commands.rs    # Modified for injection
│   │   └── lib.rs               # Add: pub mod research;
│   └── Cargo.toml               # Add: syn = "2"
└── tui/
    └── src/
        └── chatwidget/
            └── spec_kit/
                └── commands/
                    └── research.rs  # /speckit.research command
```

### 12.3 Step-by-Step Execution Order

**Step 1: Data & Validation Layer (Core)**
1. Add `syn = { version = "2", features = ["full", "parsing"] }` to `core/Cargo.toml`
2. Create `core/src/research/mod.rs` with module exports
3. Implement `core/src/research/schema.rs` with serde structs
4. Implement `core/src/research/validator.rs` with syn parsing
5. Add `pub mod research;` to `core/src/lib.rs`
6. **Verification**: Write unit test detecting signature change

**Step 2: Command Layer (TUI)**
1. Create `tui/src/chatwidget/spec_kit/commands/research.rs`
2. Wire MCP client: `mcp_client.call_tool("notebooklm", "fetch_brief", ...)`
3. Add file persistence: `.code/context/active_brief.json`
4. Register in `command_registry.rs`

**Step 3: Injection Layer (Prompts)**
1. Modify `slash_commands.rs` to accept `Option<&ResearchBrief>`
2. Add `format_divine_truth_injection()` function
3. Load persisted brief from `.code/context/` at prompt time
4. **Verification**: Test prompt output contains constraints

---

## 13. Implementation Phases

### Phase 1: Schema & Persistence (Est. 2-3 hours)
- [ ] Create `research_brief.rs` with all schema types
- [ ] Implement persistence functions (`.code/context/`)
- [ ] Add `ContextBridgeConfig` and `ResearchStageConfig` to config_types
- [ ] Update config.toml parsing

### Phase 2: Validation Engine (Est. 2-3 hours)
- [ ] Implement `brief_validator.rs`
- [ ] Function signature hashing (Rust AST or regex)
- [ ] Type definition hashing
- [ ] Pattern matching validation
- [ ] Reference Rot detection

### Phase 3: MCP Integration (Est. 3-4 hours)
- [ ] Create `brief_ingest.rs`
- [ ] Implement query building with PRD content
- [ ] Implement NotebookLM response parsing
- [ ] Handle MCP errors with circuit breaker

### Phase 4: Research Stage Executor (Est. 3-4 hours)
- [ ] Create `research_stage.rs` with `ResearchStageExecutor`
- [ ] Integrate with `PipelineCoordinator`
- [ ] Add `ResearchStageResult` enum
- [ ] Implement confirmation modal for pause states
- [ ] Add to `SpecAutoState` (lock_research_brief, etc.)

### Phase 5: Prompt Injection (Est. 2-3 hours)
- [ ] Create `brief_injector.rs`
- [ ] Implement legislative formatting (BINDING constraints)
- [ ] Modify `format_subagent_command()` to accept ResearchBrief
- [ ] Add constraint enforcement hooks
- [ ] Integrate with all stage prompts (Plan/Tasks/Implement/Validate)

### Phase 6: Commands & Config (Est. 2-3 hours)
- [ ] Implement `/speckit.ingest` command (manual override)
- [ ] Implement `/speckit.brief` command
- [ ] Add `--no-research` and `--notebook` flags to `/speckit.auto`
- [ ] Register commands in registry
- [ ] Pipeline stage configuration parsing

### Phase 7: Testing & Polish (Est. 2-3 hours)
- [ ] Unit tests for all components
- [ ] Integration tests (mock MCP server)
- [ ] Circuit breaker behavior tests
- [ ] Documentation
- [ ] Error handling review

**Total Estimated Effort**: ~18-23 hours (multi-session)

---

## 13. Design Decisions (Resolved)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **MCP Strategy** | `ask_question` + JSON prompt | Use existing NotebookLM tool, no server extension needed |
| **Injection Style** | Aggressive (`!!! OVERRIDE !!!`) | Stronger model compliance |
| **Reference Rot Handling** | Pause with modal | User must acknowledge before proceeding |
| **Schema Complexity** | TBD (needs research) | Evaluate tradeoffs between `Vec<String>` vs `Vec<Constraint>` |

---

## 14. Open Questions

1. **Schema Complexity**: Should constraints be `Vec<String>` (simple) or `Vec<Constraint>` (with severity)?
   - Research needed: Can NotebookLM reliably output structured severity levels?

2. **Anchor Granularity**: How fine-grained should code anchors be? Function-level? Line-level?

3. **Conflict Resolution**: What happens when research constraints conflict with AGENTS.md instructions?

4. **Multi-Brief Support**: Should we support multiple active briefs (one per domain)?

5. **`impl` Block Support**: Current validator only finds top-level functions. Methods need recursive traversal.

---

## 15. References

- `codex-rs/tui/src/chatwidget/spec_kit/ace_prompt_injector.rs` - Existing prompt injection pattern
- `codex-rs/core/src/slash_commands.rs` - Current prompt construction
- `codex-rs/core/src/mcp_connection_manager.rs` - MCP integration
- `codex-rs/core/src/project_doc.rs` - Project documentation handling
- `codex-rs/tui/src/chatwidget/spec_kit/context.rs` - SpecKitContext trait

---

## 16. Hardened Implementation Guide

> **Context**: Research confirmed that `notebooklm-mcp` relies on heavy browser automation (Headless Chrome), not a lightweight API. This section defines "hardened" engineering constraints.

### 16.1 Architecture: The "Hardened" Bridge

**Topology**:
```
CLI (Rust) <-> MCP (Node.js) <-> Headless Chrome <-> NotebookLM (Web)
```

**Constraint**: The Research Stage is **blocking and heavy**. It must **never run in parallel**.

**Data Flow**:
```
speckit.auto starts
    -> Check Auth
    -> Fetch Research (MCP)
    -> Parse "Double-Encoded" JSON
    -> Inject as "Divine Truth"
    -> Run Agents
```

### 16.2 Critical Implementation Constraints

#### A. Data Layer: Robust "Double-Decoding"

The MCP server returns complex data serialized inside JSON strings.

**Requirement**: In `codex-rs/core/src/research/schema.rs`, ensure deserialization handles double-encoding.

**Pattern**:
```rust
// Don't just #[derive(Deserialize)] blindly.
// Implement custom logic to peel the outer string layer if detected.
let content: String = mcp_response.content.as_text()?;
let brief: ResearchBrief = serde_json::from_str(&content)
    .or_else(|_| serde_json::from_str(serde_json::from_str::<String>(&content)?))?;
```

#### B. Transport Layer: The "Death Pact"

To prevent "Zombie Chrome" processes if the CLI crashes:

**Requirement**: Implement a process supervisor in `codex-rs/core/src/mcp_connection_manager.rs`.

**Logic**: When spawning the `notebooklm-mcp` server, strictly bind its lifecycle to the parent. Use explicit signal handling (`SIGTERM`) in the `Drop` implementation of the MCP Client to ensure the Node.js sidecar is killed on exit.

```rust
impl Drop for McpSidecar {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill(); // Best-effort cleanup
            tracing::info!("Killed MCP sidecar process (Death Pact)");
        }
    }
}
```

#### C. Security Layer: "Interactive Setup" Flow

Authentication cannot be automated cleanly.

**New Command**: Implement `/speckit.research setup` (or `/research.login`).

**Behavior**: Calls the MCP tool `setup_auth` which launches a visible browser window.

**Pipeline Logic**: The `speckit.auto` pipeline runs in Headless mode. If it receives a "Login required" error from MCP, it **PAUSES** and prompts the user to run the setup command, rather than crashing.

```rust
match research_result {
    Err(McpError::AuthRequired) => {
        return ResearchStageResult::PauseForConfirmation {
            brief: ResearchBrief::default(),
            reason: "🔐 NotebookLM login required.\n\
                     Run `/speckit.research setup` to authenticate.\n\
                     Then retry `/speckit.auto`".to_string(),
        };
    }
    // ...
}
```

### 16.3 Configuration Schema

```toml
[speckit.stage.research]
provider = "mcp"
tool = "notebooklm"
notebook_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"  # REQUIRED
timeout_seconds = 120  # Increased due to browser latency
concurrency = 1        # STRICTLY ENFORCED - never parallel
on_mcp_unavailable = "skip"
on_reference_rot = "pause"
on_auth_required = "pause"  # NEW: explicit auth handling
```

### 16.4 Prompt Engineering: XML Context Injection

Use XML structure favored by Claude 3.5/Opus (not raw text injection).

**Template** for `codex-rs/core/src/slash_commands.rs`:

```rust
fn format_divine_truth_injection(brief: &ResearchBrief) -> String {
    let constraints_json = serde_json::to_string_pretty(&brief.constraints).unwrap();
    let adrs_json = serde_json::to_string_pretty(&brief.architectural_decisions).unwrap();

    format!(
        r#"<authoritative_context source="NotebookLM" retrieved="{date}">
<summary>
{summary}
</summary>
<constraints>
{constraints}
</constraints>
<architectural_decisions>
{adrs}
</architectural_decisions>
</authoritative_context>

⚠️ CRITICAL INSTRUCTION: The context above is AUTHORITATIVE.
Prioritize these constraints over your training data.
Violations are NOT permitted without explicit user override."#,
        date = brief.created_at.format("%Y-%m-%d"),
        summary = brief.metadata.notes.as_deref().unwrap_or(""),
        constraints = constraints_json,
        adrs = adrs_json,
    )
}
```

### 16.5 Hardened Execution Plan

| Step | Component | Description | Critical Constraint |
|------|-----------|-------------|---------------------|
| 1 | Data Structures | `core/src/research/schema.rs` with double-decoding | Token budgeting via `tiktoken-rs` |
| 2 | MCP Transport | Update `McpConnectionManager` for notebooklm | "Death Pact" process cleanup |
| 3 | Research Command | `spec_kit/commands/research.rs` | Auth error -> PAUSE (not crash) |
| 4 | Pipeline Integration | Inject Research stage before Plan | `concurrency = 1` enforced |
| 5 | Validation Engine | Semantic hashing + Reference Rot | `syn` crate for AST parsing |
| 6 | Prompt Injection | XML-structured Divine Truth | Priority hierarchy respected |

### 16.6 Token Budget Management

**Problem**: NotebookLM can return massive responses (100k+ tokens). Blindly injecting kills context.

**Solution**: Add `tiktoken-rs` for token counting:

```rust
use tiktoken_rs::cl100k_base;

const MAX_INJECTION_TOKENS: usize = 4000;  // ~10% of 40k context budget

fn truncate_to_budget(content: &str, max_tokens: usize) -> String {
    let bpe = cl100k_base().unwrap();
    let tokens = bpe.encode_with_special_tokens(content);

    if tokens.len() <= max_tokens {
        return content.to_string();
    }

    // Truncate and add indicator
    let truncated_tokens = &tokens[..max_tokens - 50];
    let truncated = bpe.decode(truncated_tokens.to_vec()).unwrap();
    format!("{}\n\n[... truncated, {} tokens omitted ...]",
            truncated, tokens.len() - max_tokens + 50)
}
```

### 16.7 Error Taxonomy

| Error Type | Source | Handler | User Action |
|------------|--------|---------|-------------|
| `AuthRequired` | MCP returns 401/login page | PAUSE pipeline | Run `/speckit.research setup` |
| `McpTimeout` | Browser hang (>120s) | Circuit breaker | Retry or skip |
| `McpUnavailable` | Server not running | Configurable (skip/fail) | Start MCP server |
| `ReferenceRot` | Anchor hash mismatch | PAUSE + show diff | Acknowledge or re-ingest |
| `TokenBudgetExceeded` | Response too large | Auto-truncate + warn | Review truncation |
| `ParseError` | Malformed response | Retry once, then fail | Check NotebookLM sources |
| `ZombieProcess` | Chrome leak | Death Pact cleanup | Automatic |

### 16.8 Testing Requirements

#### Unit Tests
```rust
#[test]
fn test_double_decode_handles_nested_json() {
    let double_encoded = r#""{\"constraints\":[\"no unwrap\"]}""#;
    let brief = parse_mcp_response(double_encoded);
    assert!(brief.is_ok());
}

#[test]
fn test_token_truncation_preserves_structure() {
    let large_content = "x".repeat(50000);
    let truncated = truncate_to_budget(&large_content, 1000);
    assert!(truncated.contains("[... truncated"));
}
```

#### Integration Tests
```rust
#[tokio::test]
async fn test_auth_error_pauses_pipeline() {
    let mock_mcp = MockMcpManager::new()
        .with_error("notebooklm", McpError::AuthRequired);

    let result = ResearchStageExecutor::new(config)
        .execute("SPEC-001", "prd content", &repo_root)
        .await;

    assert!(matches!(result, ResearchStageResult::PauseForConfirmation { .. }));
}
```

---

## Appendix: Model & Runtime (Spec Overrides)

Policy: docs/MODEL-POLICY.md (version: 1.0.0)

> **Note**: This spec is partially superseded by SPEC-KIT-102 (NotebookLM v2.0.0).
> See legacy notice at top of document.

Roles exercised by this spec:
- Stage0 Tier2 (NotebookLM): YES (via MCP - now HTTP API per 102)
- Architect/Planner: YES (receives "Divine Truth" injection)
- Implementer/Rust Ace: YES (receives "Divine Truth" injection)
- Librarian: NO
- Tutor: NO
- Auditor/Judge: YES (validation stage)

Routing mode: single-owner pipeline (Architect → Implementer → Judge)
Execution: No consensus; quality enforced by compiler/tests and guardrails

Primary tiers:
- fast_local: Local 14B planner + Local 32B coder (vLLM)
- tier2_synthesis: NotebookLM (cloud, citation-grounded)
- premium_judge: GPT-5.1 High / Claude Opus

Privacy:
- local_only = false (NotebookLM integration requires cloud)

High-risk:
- HR = NO (context injection is read-only)

Overrides:
- "Divine Truth" injection is legislative (binding on all stages)
- Reference Rot detection required before execution

---

*This specification defines the architecture for SPEC-KIT-099. Implementation code should not be written until this design is approved.*
