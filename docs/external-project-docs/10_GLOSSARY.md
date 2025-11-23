# Glossary: Key Terms and Concepts

## A

### ACE (Agentic Context Engine)
Learning system that extracts patterns from agent execution, analyzes outcomes, and updates prompts with lessons learned. Components: reflector, curator, orchestrator.

### Agent
An AI model instance executing a specific task. Spec-Kit agents include Gemini Flash, Claude Haiku, GPT-5 variants, and gpt-5-codex.

### Agent Orchestrator
Component that spawns multiple agents in parallel, collects results, and coordinates consensus synthesis.

### Analyze Gate
Quality gate that checks structural consistency between artifacts (spec ↔ plan, plan ↔ tasks). Uses native pattern matching ($0).

### Artifact
Any file produced by the pipeline: spec.md, plan.md, tasks.md, implementation.md, validation-report.md, audit-report.md.

### Auto-Resolution
Automatic application of fixes when agents achieve consensus. Uses confidence levels and magnitude classification.

---

## B

### Backoff (Exponential)
Retry strategy that increases wait time between attempts: 2s → 4s → 8s → 16s.

### Budget
Per-SPEC cost limit configurable via pipeline.toml. Default: $5.00. Alerts at 80% and 95%.

---

## C

### Checklist Gate
Quality gate that scores requirements on 4 dimensions (completeness, clarity, testability, consistency) using rubric-based evaluation. Native implementation ($0).

### Circuit Breaker
Pattern that prevents repeated calls to failing services. Opens after threshold failures, gradually recovers.

### Clarify Gate
Quality gate that detects ambiguities using pattern matching for vague language, missing sections, undefined terms. Native implementation ($0).

### Consensus
Agreement among multiple agents on a decision or artifact. Calculated as unanimous (3/3), majority (2/3), or no consensus.

### Consensus Database
SQLite storage for agent outputs, synthesis results, and consensus metadata. Located in project root.

### Cost Tracker
Component that monitors per-SPEC spending, generates alerts, and enforces budget limits.

### Coverage Matrix
Table mapping requirements to plan items, tasks, and validation steps. Used by Analyze gate to verify completeness.

---

## D

### Degradation (Graceful)
Continued operation when components fail. Example: 2/3 agents completing when 1 fails still produces valid consensus.

### Domain
Category for organizing memories in local-memory: spec-kit, infrastructure, rust, documentation, debugging.

---

## E

### Evidence
Telemetry and artifacts collected during pipeline execution. Stored in evidence directories with 25 MB soft limit per SPEC.

### Evidence Policy
Rules for retention, archival, and cleanup of evidence. Old evidence archived after 30 days, purged after 90.

---

## F

### Fixture
Test data (real agent outputs, example SPECs) used for testing without live API calls. Located in fixtures/ directory.

---

## G

### Gate (Quality)
Automated checkpoint that validates artifacts before proceeding. Four gates: Clarify, Checklist, Analyze (2x).

### Guardrail
Shell wrapper that adds validation, telemetry, and evidence management around spec-kit commands. Example: `/guardrail.plan`.

### GPT-5-Codex
Specialized GPT-5 variant optimized for code generation. Used in /speckit.implement stage.

---

## H

### Handler
Function that processes a specific command. Example: `handle_speckit_plan()` handles `/speckit.plan`.

---

## I

### Importance
Score (1-10) indicating memory value. Store only ≥8. Crisis events = 10, patterns = 9, milestones = 8.

---

## L

### Local-Memory
MCP-based knowledge persistence system. Stores curated insights, patterns, and decisions (not session history).

---

## M

### MCP (Model Context Protocol)
Protocol for communication between AI models and tools. Spec-Kit uses native MCP integration (5.3x faster than subprocess).

### Magnitude
Classification of issue severity: Minor (style), Important (architecture), Critical (blocking).

### Memory Policy
Rules for using local-memory: what to store, importance thresholds, tag schemas, maintenance procedures.

---

## N

### Native
Implemented in Rust without AI agents. Native commands cost $0 and execute in <1 second. Examples: /speckit.new, /speckit.clarify.

### Namespace (Tag)
Prefixed tag format for organization: `spec:SPEC-KIT-070`, `type:bug-fix`, `component:routing`.

---

## P

### Pipeline
The 6-stage workflow: Specify → Plan → Tasks → Implement → Validate → Audit → Unlock.

### Pipeline Coordinator
Component that orchestrates multi-stage execution with quality gate checkpoints.

### Pipeline Config
TOML configuration defining enabled stages, budgets, and quality gate settings for a SPEC.

### PRD (Product Requirements Document)
Detailed requirements document generated/refined by /speckit.specify.

### Premium Agent
High-capability model used for critical decisions: gemini-pro, claude-sonnet, gpt5-high. ~$0.80/run.

---

## Q

### Quality Gate Handler
Component that executes quality checks, classifies issues, and manages auto-resolution.

---

## R

### Registry (Command)
Data structure mapping command names to handler functions. Supports 22 commands with 38 aliases.

### Resolvability
Classification of how an issue should be resolved: AutoFix, SuggestFix, NeedHuman.

### Retry
Automatic re-execution after failure. Default: 3 retries with exponential backoff.

### Routing
Process of parsing commands and dispatching to appropriate handlers.

---

## S

### SPEC
A single unit of work with ID (e.g., SPEC-KIT-065), directory, and artifacts.

### SPEC.md
Master tracker file listing all SPECs with status, links, and task tables.

### Spec-Kit
The multi-agent automation framework implemented in this project.

### Stage
One phase of the pipeline: Specify, Plan, Tasks, Implement, Validate, Audit, Unlock.

### State Machine
Component tracking pipeline progress through phases: Guardrail, ExecutingAgents, CheckingConsensus, etc.

### Synthesis
Process of combining multiple agent outputs into unified result. Includes agreement analysis.

---

## T

### Tag
Metadata label for organizing memories. Uses namespaced format (spec:, type:, component:).

### Telemetry
Structured data about command execution: costs, timing, agent status, artifacts.

### Tier
Cost category for commands: Tier 0 ($0), Tier 1 (~$0.10), Tier 2 (~$0.35), Tier 3 (~$0.80), Tier 4 (~$2.70).

### TUI (Terminal User Interface)
The main application interface. Spec-Kit is integrated into the TUI as a module.

---

## U

### Unlock
Final stage that produces ship/no-ship decision based on complete review of all artifacts.

---

## V

### Validate
Stage that creates test strategy and coverage analysis for implementation.

### Vault (Encrypted)
Secure storage for secrets and tokens. Example implementation uses HashiCorp Vault.

---

## W

### Work Breakdown
Structured list of tasks in plan.md, mapping requirements to implementation steps.

### Workspace
Rust cargo workspace containing multiple crates: tui, spec-kit, core, mcp-*, etc.

---

## Model Reference

| Model | Agent Name | Cost | Use Case |
|-------|------------|------|----------|
| Gemini 2.5 Flash | gemini-flash | $0.075/1M | Cheap analysis |
| Claude 3.5 Haiku | claude-haiku | $0.25/1M | Edge cases |
| GPT-5 Low | gpt5-low | $0.10/run | Simple tasks |
| GPT-5 Medium | gpt5-medium | $0.35/run | Planning |
| GPT-5 High | gpt5-high | $0.80/run | Critical decisions |
| GPT-5-Codex | gpt_codex | $0.11/run | Code generation |
| Gemini 2.5 Pro | gemini-pro | $0.80/run | Premium analysis |
| Claude 4.5 Sonnet | claude-sonnet | $0.80/run | Security review |
| Native Rust | code | $0 | Heuristics |

---

## Command Quick Reference

| Command | Tier | Cost | Purpose |
|---------|------|------|---------|
| `/speckit.new` | 0 | $0 | Create SPEC |
| `/speckit.clarify` | 0 | $0 | Detect ambiguities |
| `/speckit.analyze` | 0 | $0 | Check consistency |
| `/speckit.checklist` | 0 | $0 | Score quality |
| `/speckit.status` | 0 | $0 | Show status |
| `/speckit.specify` | 1 | $0.10 | Refine PRD |
| `/speckit.tasks` | 1 | $0.10 | Decompose tasks |
| `/speckit.plan` | 2 | $0.35 | Create plan |
| `/speckit.validate` | 2 | $0.35 | Test strategy |
| `/speckit.implement` | 2 | $0.11 | Generate code |
| `/speckit.audit` | 3 | $0.80 | Security review |
| `/speckit.unlock` | 3 | $0.80 | Ship decision |
| `/speckit.auto` | 4 | $2.70 | Full pipeline |
