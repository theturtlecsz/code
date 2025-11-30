# PRD: Personalization, Proactivity & Precision Integration

**SPEC-ID:** SPEC-KIT-PPP
**Status:** Draft
**Created:** 2025-11-30
**Owner:** TBD

---

## 1. Problem Statement

Developers using the code TUI experience friction when:
- The agent generates verbose responses when terse output is preferred
- The agent immediately starts coding before the user has fully specified requirements
- Responses don't match the user's preferred communication style or language
- There's no way to configure how proactively the agent should act

This results in increased "stop generating" clicks, wasted tokens, and developer frustration.

---

## 2. User Stories

### Primary User Story
> As a developer who prefers terse output, I want to configure the agent to be "terse" and "ask before coding" so that I receive concise responses and get clarification questions before implementation begins.

### Secondary User Stories

1. **Multilingual Developer**: "As a developer who thinks in Italian, I want responses in my native language (--lang it) so that I can work more efficiently."

2. **Senior Developer**: "As an experienced developer, I want the agent to be less verbose with explanations and more proactive with suggestions, so that I can move faster."

3. **Junior Developer**: "As a learning developer, I want the agent to explain its reasoning thoroughly and ask before making changes, so that I understand what's happening."

4. **Team Lead**: "As a team lead, I want to configure personalization settings at the project level so that all team members have a consistent experience."

---

## 3. Success Metrics

| Metric | Current Baseline | Target | Measurement |
|--------|-----------------|--------|-------------|
| "Stop generating" clicks per session | TBD (needs telemetry) | -40% reduction | TUI event tracking |
| Clarification requests before coding | ~5% of prompts | +200% (15% of prompts) | Agent turn analysis |
| User satisfaction (prompt-to-resolution) | TBD | +25% improvement | Session completion rate |
| Token efficiency (output/input ratio) | ~8:1 | ~4:1 for "terse" mode | API telemetry |
| Time-to-first-code | Immediate | Configurable delay | Turn timing |

---

## 4. Features

### 4.1 Configuration Layer (`[personalization]` TOML table)

```toml
[personalization]
# Communication style: "terse" | "balanced" | "verbose"
verbosity = "terse"

# Language preference (ISO 639-1 code)
language = "en"

# Proactivity level: "ask_first" | "suggest" | "autonomous"
proactivity = "ask_first"

# Vagueness detection threshold (0.0-1.0, higher = stricter)
vagueness_threshold = 0.7

# Custom tone adjectives
tone = ["professional", "direct"]
```

### 4.2 Vagueness Check Middleware

When `proactivity = "ask_first"`:
1. User submits prompt
2. Middleware analyzes prompt for clarity
3. If vague (below threshold), agent asks clarifying questions BEFORE coding
4. Agent emits `ClarificationNeeded` event with specific questions
5. User responds, then agent proceeds

### 4.3 CLI Overrides

```bash
# Language override
codex --lang it "Crea una funzione..."

# Proactivity override
codex --proactivity ask_first "Implement auth"

# Verbosity override
codex --terse "Fix the bug"
```

### 4.4 Interaction Scoring (Consensus Enhancement)

Add `interaction_score` to consensus artifacts for tracking:
- Response relevance to user preferences
- Clarification effectiveness
- User satisfaction signals

---

## 5. Constraints (from Code Analysis)

### 5.1 Configuration Constraints
- **ConfigToml extension**: Must follow existing `Option<T>` pattern with `#[serde(default)]`
- **ConfigOverrides**: Must be updated for each CLI flag; requires full destructure pattern matching
- **Backward compatibility**: Empty `[personalization]` section must not break existing configs

### 5.2 SQLite Schema Constraints
- **Migration required**: Adding `interaction_score` to `consensus_runs` requires migration_v3
- **Schema version**: Must increment `SCHEMA_VERSION` from 2 to 3
- **JSON fallback**: Can store in `synthesis_json` TEXT field without schema change (safer)

### 5.3 Middleware Constraints
- **Non-blocking**: Vagueness check must not add latency to normal prompts
- **Opt-in**: Must be disabled by default to preserve existing behavior
- **Event model**: Must integrate with existing `Event` enum in `codex.rs`

### 5.4 MCP Integration Constraints
- **Fire-and-forget**: Logging must use `tokio::spawn` to avoid blocking UI
- **Failure tolerance**: MCP logger unavailability must not break core functionality
- **Tool registration**: `interaction-logger` must be registered in `mcp_servers` config

---

## 6. Non-Goals (Out of Scope)

- Real-time preference learning (requires separate ML system)
- Per-file or per-function personalization
- Voice/audio interface customization
- User authentication/identity management
- A/B testing infrastructure

---

## 7. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Vagueness detection false positives | Medium | High (annoyance) | Configurable threshold, easy override |
| SQLite migration failures | Low | High (data loss) | Rollback migration, dual-write phase |
| Performance degradation | Low | Medium | Async middleware, feature flags |
| Breaking existing configs | Medium | High | Extensive test coverage, default values |

---

## 8. Open Questions

1. Should personalization settings be per-project or global?
2. How do we handle conflicting settings (CLI vs TOML vs project)?
3. What's the minimum vagueness threshold that provides value without annoyance?
4. Should interaction scores influence future personalization recommendations?

---

## 9. Dependencies

- `codex-rs/core/src/config.rs` - Configuration extension
- `codex-rs/core/src/codex.rs` - Middleware hook point
- `codex-rs/core/src/db/migrations.rs` - Schema migration
- `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs` - Score integration
