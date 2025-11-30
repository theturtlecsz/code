# Technical Specification: Personalization, Proactivity & Precision

**SPEC-ID:** SPEC-KIT-PPP
**Version:** 1.0.0
**Status:** Draft
**Created:** 2025-11-30

---

## 1. Data Structures

### 1.1 PersonalizationConfig (Rust)

```rust
// codex-rs/core/src/config.rs

/// User personalization preferences
#[derive(Deserialize, Debug, Clone, Default)]
pub struct PersonalizationToml {
    /// Communication verbosity: "terse" | "balanced" | "verbose"
    #[serde(default = "default_verbosity")]
    pub verbosity: Verbosity,

    /// ISO 639-1 language code (e.g., "en", "it", "ja")
    #[serde(default)]
    pub language: Option<String>,

    /// Agent proactivity level
    #[serde(default = "default_proactivity")]
    pub proactivity: ProactivityLevel,

    /// Vagueness detection threshold (0.0-1.0)
    #[serde(default = "default_vagueness_threshold")]
    pub vagueness_threshold: f64,

    /// Tone adjectives for response style
    #[serde(default)]
    pub tone: Vec<String>,

    /// Enable vagueness check middleware
    #[serde(default)]
    pub check_vagueness: bool,
}

fn default_verbosity() -> Verbosity { Verbosity::Balanced }
fn default_proactivity() -> ProactivityLevel { ProactivityLevel::Suggest }
fn default_vagueness_threshold() -> f64 { 0.5 }
```

### 1.2 ProactivityLevel Enum

```rust
// codex-rs/core/src/config.rs

/// Agent proactivity configuration
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProactivityLevel {
    /// Always ask clarifying questions before acting
    AskFirst,

    /// Suggest actions but wait for confirmation
    #[default]
    Suggest,

    /// Act autonomously with minimal confirmation
    Autonomous,
}

impl ProactivityLevel {
    /// Returns true if agent should check for vagueness before acting
    pub fn requires_vagueness_check(&self) -> bool {
        matches!(self, Self::AskFirst | Self::Suggest)
    }

    /// Returns prompt modifier for this proactivity level
    pub fn prompt_instruction(&self) -> &'static str {
        match self {
            Self::AskFirst => "Before taking any action, ask clarifying questions to ensure you understand the request completely.",
            Self::Suggest => "Suggest your approach and wait for confirmation before making changes.",
            Self::Autonomous => "Proceed with your best judgment. Ask only when truly ambiguous.",
        }
    }
}
```

### 1.3 Verbosity Enum

```rust
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    /// Minimal explanations, code-focused
    Terse,

    /// Standard explanations
    #[default]
    Balanced,

    /// Detailed explanations, educational
    Verbose,
}

impl Verbosity {
    pub fn prompt_instruction(&self) -> &'static str {
        match self {
            Self::Terse => "Be concise. Minimize explanations. Focus on code and direct answers.",
            Self::Balanced => "Provide clear explanations with appropriate detail.",
            Self::Verbose => "Explain your reasoning thoroughly. Include context and alternatives.",
        }
    }
}
```

### 1.4 VaguenessCheckResult

```rust
// codex-rs/core/src/vagueness.rs (NEW FILE)

/// Result of vagueness analysis on user input
#[derive(Debug, Clone)]
pub struct VaguenessCheckResult {
    /// Clarity score (0.0 = very vague, 1.0 = very clear)
    pub clarity_score: f64,

    /// Whether the input is considered vague
    pub is_vague: bool,

    /// Specific aspects that need clarification
    pub unclear_aspects: Vec<String>,

    /// Suggested clarifying questions
    pub suggested_questions: Vec<String>,
}
```

### 1.5 ConsensusVerdict Extension

```rust
// codex-rs/tui/src/chatwidget/spec_kit/consensus.rs

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ConsensusVerdict {
    // ... existing fields ...

    /// Numeric interaction quality score (0.0-1.0)
    /// Measures: relevance, clarity, user satisfaction signals
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_score: Option<f64>,

    /// Breakdown of score components
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_breakdown: Option<InteractionScoreBreakdown>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InteractionScoreBreakdown {
    pub relevance: f64,      // How relevant was the response
    pub clarity: f64,        // How clear was the communication
    pub efficiency: f64,     // Token efficiency (lower is better)
    pub user_signals: f64,   // Derived from user actions (edits, regenerates)
}
```

---

## 2. Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           User Input Flow                                    │
└─────────────────────────────────────────────────────────────────────────────┘

  ┌──────────┐     ┌──────────────────┐     ┌─────────────────┐
  │   User   │────▶│   TUI Input      │────▶│  Op::UserMessage │
  │  Prompt  │     │   Handler        │     │                  │
  └──────────┘     └──────────────────┘     └────────┬────────┘
                                                      │
                                                      ▼
                   ┌──────────────────────────────────────────────┐
                   │           Submission Loop (codex.rs)          │
                   │                                                │
                   │  ┌────────────────────────────────────────┐   │
                   │  │     VAGUENESS CHECK MIDDLEWARE         │   │
                   │  │     (NEW - Interception Point)         │   │
                   │  │                                        │   │
                   │  │  if config.personalization.check_vague │   │
                   │  │    && proactivity.requires_check()     │   │
                   │  │  then:                                 │   │
                   │  │    analyze_vagueness(prompt)           │   │
                   │  │    if vague:                           │   │
                   │  │      emit ClarificationNeeded          │───┼──▶ Back to User
                   │  │      PAUSE execution                   │   │
                   │  │    else:                               │   │
                   │  │      continue to Agent Loop            │   │
                   │  └────────────────────────────────────────┘   │
                   │                      │                        │
                   │                      ▼                        │
                   │  ┌────────────────────────────────────────┐   │
                   │  │          PROMPT COMPOSER               │   │
                   │  │                                        │   │
                   │  │  Inject personalization instructions:  │   │
                   │  │  - verbosity.prompt_instruction()      │   │
                   │  │  - proactivity.prompt_instruction()    │   │
                   │  │  - language preference                 │   │
                   │  │  - tone adjectives                     │   │
                   │  └────────────────────────────────────────┘   │
                   │                      │                        │
                   │                      ▼                        │
                   │  ┌────────────────────────────────────────┐   │
                   │  │            run_turn()                  │   │
                   │  │                                        │   │
                   │  │  Execute agent loop with personalized  │   │
                   │  │  prompt and configuration              │   │
                   │  └────────────────────────────────────────┘   │
                   │                      │                        │
                   └──────────────────────┼────────────────────────┘
                                          │
                                          ▼
                   ┌──────────────────────────────────────────────┐
                   │      ResponseEvent::Completed Handler        │
                   │                                              │
                   │  ┌────────────────────────────────────┐      │
                   │  │   INTERACTION LOGGER (MCP)         │      │
                   │  │   (Fire-and-forget, non-blocking)  │      │
                   │  │                                    │      │
                   │  │   tokio::spawn(async {             │      │
                   │  │     mcp.call_tool(                 │      │
                   │  │       "interaction-logger",        │      │
                   │  │       "log_interaction",           │      │
                   │  │       { response_id, tokens, ... } │      │
                   │  │     );                             │      │
                   │  │   });                              │      │
                   │  └────────────────────────────────────┘      │
                   └──────────────────────────────────────────────┘
```

---

## 3. Config Schema (TOML)

### 3.1 config.toml.example Addition

```toml
# ~/.code/config.toml

# ... existing configuration ...

# ============================================================================
# PERSONALIZATION (NEW)
# ============================================================================
# Configure agent behavior to match your preferences

[personalization]
# Communication style
# Options: "terse" | "balanced" | "verbose"
# - terse: Minimal explanations, code-focused responses
# - balanced: Standard explanations (default)
# - verbose: Detailed explanations, educational
verbosity = "balanced"

# Preferred response language (ISO 639-1 code)
# Examples: "en", "it", "ja", "es", "de"
# Leave unset for auto-detect based on input
# language = "en"

# Agent proactivity level
# Options: "ask_first" | "suggest" | "autonomous"
# - ask_first: Always ask clarifying questions before coding
# - suggest: Suggest approach, wait for confirmation (default)
# - autonomous: Act with minimal confirmation
proactivity = "suggest"

# Vagueness detection threshold (0.0-1.0)
# Higher = stricter (more likely to ask for clarification)
# Only applies when proactivity is "ask_first" or "suggest"
vagueness_threshold = 0.5

# Enable vagueness check middleware
# When true, analyzes prompts before processing
check_vagueness = false

# Tone modifiers (adjectives describing response style)
# Examples: "professional", "friendly", "direct", "patient"
# tone = ["professional", "direct"]
```

### 3.2 CLI Flags Mapping

| CLI Flag | Config Path | Type | Example |
|----------|-------------|------|---------|
| `--lang <code>` | `personalization.language` | String | `--lang it` |
| `--verbosity <level>` | `personalization.verbosity` | Enum | `--verbosity terse` |
| `--terse` | `personalization.verbosity = "terse"` | Shorthand | `--terse` |
| `--proactivity <level>` | `personalization.proactivity` | Enum | `--proactivity ask_first` |
| `--ask-first` | `personalization.proactivity = "ask_first"` | Shorthand | `--ask-first` |

---

## 4. API Changes

### 4.1 New Event Type

```rust
// codex-rs/core/src/event.rs

pub enum Event {
    // ... existing variants ...

    /// Agent requests clarification before proceeding
    ClarificationNeeded {
        /// Submission ID that triggered this
        sub_id: String,
        /// Why clarification is needed
        reason: String,
        /// Specific questions to ask
        questions: Vec<String>,
        /// Original prompt for context
        original_prompt: String,
    },
}
```

### 4.2 ConfigOverrides Extension

```rust
// codex-rs/core/src/config.rs

pub struct ConfigOverrides {
    // ... existing fields ...

    /// Language preference override (--lang)
    pub personalization_language: Option<String>,

    /// Verbosity override (--verbosity, --terse)
    pub personalization_verbosity: Option<Verbosity>,

    /// Proactivity override (--proactivity, --ask-first)
    pub personalization_proactivity: Option<ProactivityLevel>,
}
```

---

## 5. Database Schema Changes

### 5.1 Migration V3 (Optional - for column approach)

```sql
-- migration_v3: Add interaction_score column
-- Only needed if storing as column vs JSON

ALTER TABLE consensus_runs ADD COLUMN interaction_score REAL;

CREATE INDEX IF NOT EXISTS idx_consensus_score
    ON consensus_runs(interaction_score);
```

### 5.2 JSON Approach (Preferred - No Migration)

Store in existing `synthesis_json` TEXT field:

```json
{
  "output_markdown": "...",
  "status": "ok",
  "interaction_score": 0.85,
  "score_breakdown": {
    "relevance": 0.9,
    "clarity": 0.8,
    "efficiency": 0.85,
    "user_signals": 0.85
  }
}
```

---

## 6. File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `core/src/config.rs` | Modify | Add `PersonalizationToml`, `Verbosity`, `ProactivityLevel` |
| `core/src/config.rs` | Modify | Add to `ConfigToml.personalization` |
| `core/src/config.rs` | Modify | Extend `ConfigOverrides` |
| `core/src/codex.rs` | Modify | Add vagueness check in submission loop |
| `core/src/codex.rs` | Modify | Add MCP logger in Completed handler |
| `core/src/event.rs` | Modify | Add `ClarificationNeeded` variant |
| `core/src/vagueness.rs` | New | Vagueness analysis module |
| `tui/src/chatwidget/spec_kit/consensus.rs` | Modify | Add `interaction_score` field |
| `core/src/db/migrations.rs` | Modify (optional) | Add migration_v3 |
