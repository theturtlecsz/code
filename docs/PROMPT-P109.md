# P109 Continuation Prompt - GR001-001 Policy Compliance

## Pre-flight

```bash
# 1. Verify P108 commit
cd ~/code && git log -1 --oneline
# Expected: a015f5ea2 feat(architect): add --no-cache alias...

# 2. Build
./build-fast.sh

# 3. Read this prompt and MODEL-POLICY.md
# docs/PROMPT-P109.md
# docs/MODEL-POLICY.md (GR-001 section)
```

---

## Context Summary

### P108 Completed (5 commits pushed)

| Commit | Description |
|--------|-------------|
| a015f5ea2 | feat(architect): add --no-cache alias |
| b6e1adfce | feat(architect): add research command suite |
| c6d09f069 | docs(SPEC): add Policy Alignment section with GR001-001 |
| 94e1b1d96 | docs(meta): sync CLAUDE/AGENTS/GEMINI with MODEL-POLICY |
| efe0a07d3 | docs(SPEC-KIT-099): mark as deprecated |

### Current State

- **MODEL-POLICY.md** (v1.0.0) defines GR-001: No 3-agent debate/voting/swarm synthesis
- **SPEC-KIT-099** deprecated with explicit pointer to SPEC-KIT-102
- **Instruction files** synced with Policy Pointers section
- **GR001-001** tracked in SPEC.md (Policy Alignment section)
- **Research commands** implemented but untested against live service

---

## P109 Scope: GR001-001 Policy Compliance

### Primary Goal

Align `consensus.rs` runtime with MODEL-POLICY.md GR-001:
- Disable consensus flow by default
- Preserve critic-only path as optional sidecar
- Update CLI flags/config to match policy
- Add deprecation notices to consensus terminology

### Why This Matters

> Docs/specs say one thing, runtime does another = dangerous architectural drift.
> Getting runtime compliance done turns the policy from "paper" into "enforced reality."

---

## Phase 1: Analysis (Read-Only)

### 1.1 Understand Current consensus.rs Behavior

```bash
# Read the full file
codex-rs/tui/src/chatwidget/spec_kit/consensus.rs

# Key areas identified in P108:
# - L155-172: expected_agents_for_stage() - defines multi-agent roster
# - L740-752: GptPro aggregator/synthesizer pattern
# - L811-821: Consensus status gating (ok/conflict/degraded)
```

### 1.2 Map All Callers

Find all code paths that invoke consensus:

```bash
# Search for consensus references
grep -r "consensus" --include="*.rs" codex-rs/tui/src/
grep -r "run_spec_consensus" --include="*.rs" codex-rs/
grep -r "ConsensusStatus" --include="*.rs" codex-rs/
```

### 1.3 Document Current Flow

Create a flowchart or description of:
1. When is consensus invoked?
2. What happens if consensus fails?
3. Which stages use multi-agent patterns?

---

## Phase 2: Design Decisions

Before coding, answer these questions:

### 2.1 Default Behavior

| Question | Options |
|----------|---------|
| Should consensus be disabled by default? | Yes (per GR-001) |
| How to disable? | `SPEC_KIT_CONSENSUS=false` env var |
| What happens when disabled? | Skip consensus check, proceed with single agent output |

### 2.2 Critic-Only Sidecar

| Question | Answer |
|----------|--------|
| Is critic-only pattern allowed? | Yes, as non-authoritative sidecar |
| How to enable? | `SPEC_KIT_CRITIC=true` env var |
| What does critic do? | Reviews single agent output, logs concerns, does NOT block |

### 2.3 Migration Path

| Scenario | Behavior |
|----------|----------|
| No env vars set | Consensus disabled (GR-001 compliant) |
| `SPEC_KIT_CONSENSUS=true` | Legacy mode warning + consensus enabled |
| `SPEC_KIT_CRITIC=true` | Critic-only mode (non-blocking review) |

---

## Phase 3: Implementation

### 3.1 Add Feature Flags

In `pipeline_coordinator.rs` or appropriate config:

```rust
/// Consensus mode (deprecated per GR-001)
fn is_consensus_enabled() -> bool {
    std::env::var("SPEC_KIT_CONSENSUS")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false) // Default: disabled
}

/// Critic-only mode (non-blocking review)
fn is_critic_enabled() -> bool {
    std::env::var("SPEC_KIT_CRITIC")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false) // Default: disabled
}
```

### 3.2 Guard Consensus Invocation

Find the call site and add guard:

```rust
// Before: always runs consensus
let consensus_result = run_spec_consensus(...).await?;

// After: conditional with deprecation warning
let consensus_result = if is_consensus_enabled() {
    tracing::warn!("DEPRECATED: Consensus mode enabled via SPEC_KIT_CONSENSUS=true. See GR-001.");
    run_spec_consensus(...).await?
} else {
    // Skip consensus, use single agent output
    ConsensusResult::single_agent(agent_output)
};
```

### 3.3 Update expected_agents_for_stage()

Change default roster to single agent:

```rust
// Before: multiple agents expected
fn expected_agents_for_stage(stage: SpecStage) -> Vec<AgentType> {
    match stage {
        SpecStage::Implement => vec![AgentType::Gemini, AgentType::Claude, ...],
        ...
    }
}

// After: single agent (with consensus mode fallback)
fn expected_agents_for_stage(stage: SpecStage) -> Vec<AgentType> {
    if is_consensus_enabled() {
        return legacy_expected_agents_for_stage(stage);
    }
    match stage {
        SpecStage::Implement => vec![AgentType::preferred_implementer()],
        ...
    }
}
```

### 3.4 Add Deprecation Notices

Update function doc comments:

```rust
/// DEPRECATED: Multi-agent consensus is disabled by default per GR-001.
///
/// To enable legacy consensus mode (NOT RECOMMENDED):
///   export SPEC_KIT_CONSENSUS=true
///
/// See: docs/MODEL-POLICY.md (GR-001)
#[deprecated(since = "0.1.0", note = "Use single-owner pipeline per GR-001")]
pub fn run_spec_consensus(...) { ... }
```

---

## Phase 4: Testing

### 4.1 Verify Default Behavior

```bash
# With no env vars, consensus should be disabled
unset SPEC_KIT_CONSENSUS
unset SPEC_KIT_CRITIC
./build-fast.sh run
# Run /speckit.auto SPEC-KIT-900
# Observe: single agent per stage, no consensus check
```

### 4.2 Verify Legacy Mode Warning

```bash
# With consensus enabled, should warn
export SPEC_KIT_CONSENSUS=true
./build-fast.sh run
# Run /speckit.plan SPEC-KIT-900
# Observe: deprecation warning in logs
unset SPEC_KIT_CONSENSUS
```

### 4.3 Unit Tests

Add tests in `consensus.rs`:

```rust
#[test]
fn test_consensus_disabled_by_default() {
    std::env::remove_var("SPEC_KIT_CONSENSUS");
    assert!(!is_consensus_enabled());
}

#[test]
fn test_consensus_opt_in() {
    std::env::set_var("SPEC_KIT_CONSENSUS", "true");
    assert!(is_consensus_enabled());
    std::env::remove_var("SPEC_KIT_CONSENSUS");
}
```

---

## Phase 5: Documentation

### 5.1 Update SPEC.md

Change GR001-001 from "Tracked" to "Done" with evidence.

### 5.2 Update MODEL-POLICY.md

Add implementation note under GR-001:

```markdown
**Implementation**: `SPEC_KIT_CONSENSUS=false` (default).
Legacy mode available via `SPEC_KIT_CONSENSUS=true` with deprecation warning.
Critic-only mode via `SPEC_KIT_CRITIC=true` (non-blocking).
```

---

## Secondary Goal: Budget Warning at 100%

Add hard block when daily budget is exhausted.

### Implementation

In `budget.rs`, add new method:

```rust
/// Check if budget is exhausted (hard block at 100%)
pub fn is_exhausted(&self) -> bool {
    self.usage.total >= self.daily_limit
}

/// Check if past warning threshold (soft warning at 80%)
pub fn needs_confirmation(&self) -> bool {
    self.usage.total >= self.warn_threshold
}
```

In `architect_cmd.rs`, update `run_ask`:

```rust
// Before ask
if budget.is_exhausted() {
    bail!(
        "Daily limit ({}) reached. Resets in {}.\n\
         No queries allowed until reset.",
        budget.limit(),
        budget.time_until_reset()
    );
}
```

---

## Exit Criteria

### GR001-001 Complete When:

- [ ] Consensus disabled by default (`SPEC_KIT_CONSENSUS` env var)
- [ ] Legacy mode shows deprecation warning
- [ ] Critic-only mode available as non-blocking sidecar
- [ ] Unit tests verify default behavior
- [ ] SPEC.md updated: GR001-001 → Done
- [ ] MODEL-POLICY.md updated with implementation note

### Budget 100% Block Complete When:

- [ ] `is_exhausted()` returns true at 100%
- [ ] Ask command blocks with clear error message
- [ ] No queries allowed until reset

---

## Commit Strategy

| Order | Commit |
|-------|--------|
| 1 | `feat(spec-kit): add consensus/critic feature flags per GR-001` |
| 2 | `refactor(consensus): disable multi-agent patterns by default` |
| 3 | `test(consensus): add unit tests for feature flag behavior` |
| 4 | `docs(SPEC): mark GR001-001 as complete` |
| 5 | `feat(budget): add hard block at 100% daily limit` |

---

## Risk Notes

| Risk | Mitigation |
|------|------------|
| Breaking existing /speckit.* commands | Test all commands after changes |
| Unexpected consensus callers | Map all callers in Phase 1 |
| Env var not respected | Add integration test |
| Budget block too aggressive | Show clear reset time in error |

---

## Quick Start

```
P109 - GR001-001 Policy Compliance

## Pre-flight
1. Verify: git log -1 --oneline (expect a015f5ea2)
2. Build: ./build-fast.sh
3. Read: docs/PROMPT-P109.md + docs/MODEL-POLICY.md

## Phase 1: Analysis (read-only)
- Read consensus.rs fully
- Map all callers with grep
- Document current flow

## Phase 2: Design
- Confirm feature flag approach
- Confirm critic-only sidecar design

## Phase 3: Implementation
- Add SPEC_KIT_CONSENSUS and SPEC_KIT_CRITIC env vars
- Guard consensus invocation
- Update expected_agents_for_stage()
- Add deprecation notices

## Phase 4: Testing
- Verify default = disabled
- Verify legacy mode warns
- Add unit tests

## Phase 5: Documentation
- SPEC.md: GR001-001 → Done
- MODEL-POLICY.md: implementation note

## Secondary: Budget 100% block
- Add is_exhausted() method
- Block ask at 100% with clear message
```
