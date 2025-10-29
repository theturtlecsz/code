# Spec-Kit Framework

**Multi-agent automation framework for product requirements workflows**

---

## Overview

Spec-Kit is a fork-specific automation framework that orchestrates multi-agent consensus for specification development, implementation, and validation.

### Core Workflow

```
/speckit.new → /speckit.specify → /speckit.plan → /speckit.tasks →
/speckit.implement → /speckit.validate → /speckit.audit → /speckit.unlock
```

Or use `/speckit.auto` for full pipeline automation.

---

## Key Features

- **Multi-Agent Consensus**: 3-5 agents (Gemini, Claude, GPT models) reach consensus on specs
- **Quality Gates**: Automated validation checkpoints
- **Evidence Collection**: Comprehensive telemetry and artifacts
- **Native MCP Integration**: 5.3x faster than subprocess baseline
- **ACE Learning**: Compounding strategy memory from execution outcomes

---

## Documentation

### User Guides
- [CLAUDE.md](../../CLAUDE.md) - How Claude Code works in this repo
- [Evidence Policy](evidence-policy.md) - Evidence retention and archival
- [Testing Policy](testing-policy.md) - Testing strategy and standards

### Architecture
- [Consensus Runner Design](consensus-runner-design.md) - Multi-agent consensus architecture
- [Command Registry Design](COMMAND_REGISTRY_DESIGN.md) - Dynamic command system
- [Service Traits Analysis](SERVICE_TRAITS_DEEP_ANALYSIS.md) - Service trait design

### Maintenance
- [MAINT-10 Extraction Plan](MAINT-10-EXTRACTION-PLAN.md) - Spec-kit crate extraction (future)
- [MAINT-10 Execution Plan](MAINT-10-EXECUTION-PLAN.md) - Execution details
- [Refactoring Status](REFACTORING_FINAL_STATUS.md) - Refactoring completion

### Operations
- [Consensus Cost Audit](consensus-cost-audit-packet.md) - Cost analysis
- [Consensus Degradation Playbook](consensus-degradation-playbook.md) - Handling failures
- [Evidence Baseline](evidence-baseline.md) - Collection standards
- [Adoption Dashboard](adoption-dashboard.md) - Feature adoption tracking
- [QA Sweep Checklist](qa-sweep-checklist.md) - Quality procedures
- [Security Review Template](security-review-template.md) - Security review process

### Testing
- [Phase 3 Test Plan](PHASE_3_DAY_4_TESTING_PLAN.md) - Integration testing
- [Phase 4 Test Plan](PHASE4_TEST_PLAN.md) - System testing
- [Rebase Safety Matrix](REBASE_SAFETY_MATRIX_T80-T90.md) - Rebase guidelines

---

## Quick Start

### Create New SPEC
```bash
/speckit.new Add user authentication with OAuth2
```

### Run Full Pipeline
```bash
/speckit.auto SPEC-KIT-###
```

### Check Status
```bash
/speckit.status SPEC-KIT-###
```

---

## Model Tiers

- **Tier 0**: Native TUI (0 agents, $0, <1s) - `/speckit.status`
- **Tier 2**: 3 agents (gemini, claude, code/gpt_pro) - Most commands (~$0.80-1.00, 8-12 min)
- **Tier 3**: 4 agents (adds gpt_codex) - `/speckit.implement` (~$2.00, 15-20 min)
- **Tier 4**: Dynamic (3-5 agents) - `/speckit.auto` (~$11, 60 min)

---

## Evidence Collection

All spec-kit commands generate evidence under:
```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/<SPEC-ID>/
```

**Monitor footprint**: `/spec-evidence-stats`
**Soft limit**: 25 MB per SPEC

---

## See Also

- [Main Documentation Index](../SUMMARY.md)
- [SPEC.md](../../SPEC.md) - Task tracker
- [Product Requirements](../../product-requirements.md)
