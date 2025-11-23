# Project Overview: Spec-Kit Automation Framework

## What Is This Project?

This is a **multi-agent AI orchestration framework** called Spec-Kit, built as a fork of the `just-every/code` community project (itself a fork of OpenAI Codex). The framework automates software development workflows by coordinating multiple AI models to produce higher-quality outputs through consensus.

**NOT related to**: Anthropic's Claude Code (this is a different product entirely)

## Core Value Proposition

Spec-Kit transforms how software specifications become working code:

1. **Multi-Agent Consensus**: Instead of relying on a single AI model, Spec-Kit spawns 2-5 agents (Gemini, Claude, GPT-5 variants) that independently analyze the same problem, then synthesizes their outputs
2. **Quality Gates**: Automated checkpoints catch ambiguities, inconsistencies, and missing requirements BEFORE they become bugs
3. **Cost Optimization**: 75% reduction in API costs ($11 → $2.70 per full workflow) through strategic model routing
4. **Full Audit Trail**: Every decision, every agent output, every quality check is captured as evidence

## Key Capabilities

### 6-Stage Development Pipeline

```
Specify → Plan → Tasks → Implement → Validate → Audit → Unlock
```

Each stage can be run individually or as a fully automated pipeline with `/speckit.auto`.

### Tiered Model Strategy

- **Tier 0 (Native)**: Pattern matching, structural validation - $0, <1 second
- **Tier 1 (Single Agent)**: Simple analysis - ~$0.10, 3-5 minutes
- **Tier 2 (Multi-Agent)**: Complex planning, code generation - ~$0.35, 8-12 minutes
- **Tier 3 (Premium)**: Critical decisions (audit, ship/no-ship) - ~$0.80, 10-12 minutes

### Auto-Resolution

Quality issues are automatically resolved based on agent consensus:
- **3/3 agents agree** → Auto-apply fix
- **2/3 agree + validation** → Apply with confidence
- **No consensus** → Escalate to human

## Technical Stack

- **Language**: Rust (workspace with multiple crates)
- **UI**: Terminal User Interface (TUI)
- **AI Integration**: Native MCP (Model Context Protocol) - 5.3x faster than subprocess
- **Storage**: SQLite for consensus artifacts, local-memory MCP for knowledge base
- **Evidence**: JSON telemetry with 25 MB soft limit per SPEC

## Project Status

**Phase 3 Complete** (October 2025) - Production Ready

- 604 tests, 100% pass rate
- 42-48% estimated code coverage
- $10,536/year estimated savings from cost optimization
- 22 commands with 38 aliases fully operational

## Who Is This For?

1. **AI-Assisted Development Teams**: Get better outputs through consensus, not just single-model responses
2. **Quality-Focused Organizations**: Automated gates catch issues before they become expensive
3. **Cost-Conscious Operations**: Strategic model routing minimizes API spend without sacrificing quality
4. **Audit-Required Environments**: Full evidence trail for every decision

## Quick Start

```bash
# Create a new SPEC (instant, free)
/speckit.new Add user authentication with OAuth2

# Run the full automated pipeline (~$2.70, 45-50 min)
/speckit.auto SPEC-KIT-065

# Check progress anytime
/speckit.status SPEC-KIT-065
```

## Repository Structure

```
/home/user/code/
├── CLAUDE.md              # Main operational playbook
├── AGENTS.md              # Agent roster and guardrails
├── SPEC.md                # Live task tracker
├── codex-rs/              # Rust workspace
│   ├── tui/               # Terminal UI with spec-kit
│   ├── spec-kit/          # Core library (extracted)
│   └── core/              # Client library, MCP integration
├── docs/
│   ├── spec-kit/          # Framework documentation
│   └── SPEC-*/            # Individual specification directories
└── scripts/               # Automation and validation scripts
```

## Key Differentiators

| Feature | Traditional AI Dev | Spec-Kit |
|---------|-------------------|----------|
| Model Usage | Single model | 2-5 models in consensus |
| Quality Assurance | Manual review | Automated gates with auto-fix |
| Cost Control | Pay-per-request | Strategic routing by complexity |
| Audit Trail | Chat logs | Structured evidence + telemetry |
| Workflow | Ad-hoc prompts | 6-stage pipeline with checkpoints |

## Learn More

- **Commands**: See `COMMAND_REFERENCE.md`
- **Architecture**: See `ARCHITECTURE.md`
- **Multi-Agent System**: See `MULTI_AGENT_CONSENSUS.md`
- **Cost Details**: See `COST_MODEL.md`
- **Complete Example**: See `EXAMPLE_WORKFLOW.md`
