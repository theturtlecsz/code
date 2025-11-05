# Documentation Index

**Last Updated**: 2025-10-29
**Project**: codex-rs (theturtlecsz/code fork)

This is the central navigation hub for all project documentation.

---

## 🚀 Quick Start

- [Getting Started](getting-started.md) - Installation and basic usage
- [Product Requirements](../product-requirements.md) - Canonical product scope
- [Planning Document](../PLANNING.md) - High-level architecture and goals
- [CLAUDE.md](../CLAUDE.md) - How Claude Code works in this repo
- [README](../README.md) - Project overview

---

## 📋 Core Documentation

### Project Management
- [SPEC.md](../SPEC.md) - Single source of truth for task tracking
- [Memory Policy](../codex-rs/MEMORY-POLICY.md) - Local-memory MCP usage policy
- [Constitution](../memory/constitution.md) - Project charter and guardrail canon
- [Review](../codex-rs/REVIEW.md) - Architecture review and fork-specific features

### Spec-Kit Framework
- [Spec-Kit Overview](spec-kit/) - Multi-agent automation framework
- [Evidence Policy](spec-kit/evidence-policy.md) - Evidence retention and archival
- [Testing Policy](spec-kit/testing-policy.md) - Testing strategy and standards
- [Consensus Runner Design](spec-kit/consensus-runner-design.md) - Multi-agent consensus architecture
- [Command Registry Design](spec-kit/COMMAND_REGISTRY_DESIGN.md) - Dynamic command system
- [Prompts](spec-kit/prompts.json) - Agent prompt templates

### Configuration
- [Config Documentation](config.md) - Configuration options and examples
- [MCP Configuration](../mcp.json) - Model Context Protocol servers
- [Example Config](../config.toml.example) - Configuration template

---

## 🔧 Active SPECs

### In Progress
Check [SPEC.md](../SPEC.md) for current "In Progress" items.

### Recent SPECs

#### Infrastructure & Optimization
- [SPEC-KIT-070](SPEC-KIT-070-model-cost-optimization/) - Model cost optimization (Tier routing, native tools)
- [SPEC-KIT-071](SPEC-KIT-071-memory-system-optimization/) - Memory system optimization
- [SPEC-KIT-072](SPEC-KIT-072-consensus-storage-separation/) - Consensus storage separation

#### Quality & Testing
- [SPEC-KIT-068](SPEC-KIT-068-analyze-and-fix-quality-gates/) - Quality gates analysis and fixes
- [SPEC-KIT-069](SPEC-KIT-069-address-speckit-validate-multiple-agent-calls-and-incorrect-spawning/) - Validate command fixes
- [SPEC-KIT-045](SPEC-KIT-045-design-systematic-testing-framework-for/) - Systematic testing framework
- [SPEC-KIT-060](SPEC-KIT-060-template-validation-test/) - Template validation testing

#### Features & Tools
- [SPEC-KIT-067](SPEC-KIT-067-add-search-command-to-find-text-in-conversation-history/) - Search command for conversation history
- [SPEC-KIT-066](SPEC-KIT-066-native-tool-migration/) - Native tool migration
- [SPEC-KIT-035](SPEC-KIT-035-spec-status-diagnostics/) - Spec status diagnostics
- [SPEC-KIT-040](SPEC-KIT-040-add-simple-config-validation-utility/) - Config validation utility

#### Documentation & Migration
- [SPEC-KIT-030](SPEC-KIT-030-add-documentation-for-rebasing-from/) - Rebase documentation
- [SPEC-KIT-025](SPEC-KIT-025-add-automated-conflict-resolution-with/) - Automated conflict resolution
- [SPEC-KIT-018](SPEC-KIT-018-hal-http-mcp/) - HAL HTTP MCP integration
- [SPEC-KIT-014](SPEC-KIT-014-docs-refresh/) - Documentation refresh
- [SPEC-KIT-013](SPEC-KIT-013-telemetry-schema-guard/) - Telemetry schema guards
- [SPEC-KIT-010](SPEC-KIT-010-local-memory-migration/) - Local memory migration

#### Smoke Testing
- [SPEC-KIT-900](SPEC-KIT-900-generic-smoke/) - Generic smoke test framework
- [SPEC-KIT-DEMO](SPEC-KIT-DEMO/) - Demo SPEC for testing

---

## 🏗️ Architecture

### Core Systems
- [Protocol v1](../codex-rs/docs/protocol_v1.md) - Core protocol specification
- [ACE Integration](../ACE_INTEGRATION.md) - Agentic Context Engine integration
- [ACE Test Plan](../ACE_TEST_PLAN.md) - ACE testing strategy
- [ACE Testing Guide](../ACE_TESTING_GUIDE.md) - ACE testing procedures
- [ACE Learning Usage](../ACE_LEARNING_USAGE.md) - ACE learning system usage

### Analysis & Planning
- [Project Status](../PROJECT_STATUS_ULTRATHINK.md) - Current project status
- [Analysis Summary](../ANALYSIS_SUMMARY.md) - System analysis summary
- [Fork Analysis](../FORK_SPEC_KIT_ANALYSIS.md) - Fork-specific feature analysis
- [Optimization Plan](../OPTIMIZATION_PLAN.md) - System optimization roadmap

### Design Documents
- [Architecture Docs](archive/design-docs/) - Historical design documents
- [Model Design](archive/design-docs/model.md) - Model architecture

---

## 🔍 Spec-Kit Internals

### Implementation Details
- [Consensus Cost Audit](spec-kit/consensus-cost-audit-packet.md) - Cost analysis
- [Consensus Degradation Playbook](spec-kit/consensus-degradation-playbook.md) - Handling degraded consensus
- [Evidence Baseline](spec-kit/evidence-baseline.md) - Evidence collection standards
- [Adoption Dashboard](spec-kit/adoption-dashboard.md) - Feature adoption tracking
- [QA Sweep Checklist](spec-kit/qa-sweep-checklist.md) - Quality assurance procedures
- [Security Review Template](spec-kit/security-review-template.md) - Security review process

### Maintenance Plans
- [MAINT-10 Extraction Plan](spec-kit/MAINT-10-EXTRACTION-PLAN.md) - Spec-kit crate extraction plan
- [MAINT-10 Execution Plan](spec-kit/MAINT-10-EXECUTION-PLAN.md) - Execution details
- [Service Traits Analysis](spec-kit/SERVICE_TRAITS_DEEP_ANALYSIS.md) - Service trait design
- [Refactoring Status](spec-kit/REFACTORING_FINAL_STATUS.md) - Refactoring completion status

### Testing Infrastructure
- [Phase 3 Test Plan](spec-kit/PHASE_3_DAY_4_TESTING_PLAN.md) - Integration testing plan
- [Phase 4 Test Plan](spec-kit/PHASE4_TEST_PLAN.md) - System testing plan
- [Rebase Safety Matrix](spec-kit/REBASE_SAFETY_MATRIX_T80-T90.md) - Rebase safety guidelines

---

## 📂 SPEC-OPS (Guardrails & Evidence)

### SPEC-OPS-004: Integrated Coder Hooks
- [Main Documentation](SPEC-OPS-004-integrated-coder-hooks/) - Guardrail system
- [Evidence Repository](SPEC-OPS-004-integrated-coder-hooks/evidence/) - Test artifacts and telemetry
- [Command Evidence](SPEC-OPS-004-integrated-coder-hooks/evidence/commands/) - Per-SPEC evidence

### Evidence Monitoring
- **Evidence Stats**: Run `/spec-evidence-stats` to check footprint
- **Soft Limit**: 25 MB per SPEC
- **Retention Policy**: See [evidence-policy.md](spec-kit/evidence-policy.md)

---

## 📚 Archive

### Completed SPECs
- [Archived SPECs](archive/completed-specs/) - Successfully completed specifications
- [SPEC-KIT-045-mini](SPEC-KIT-045-mini/) - Mini testing framework (archived)

### Session Archives
- [2025 Sessions](archive/2025-sessions/) - Historical session summaries
- [Session Handoff 2025-10-18](archive/2025-sessions/SESSION-HANDOFF-2025-10-18.md) - Major handoff document
- [Restart Documentation](archive/2025-sessions/RESTART.md) - Session restart procedures

### Historical Documents
- [Changelog](../CHANGELOG.md) - Historical changelog
- [Interrupt Resume Postmortem](interrupt-resume-postmortem.md) - Incident analysis
- [Documentation Cleanup Plan](DOCUMENTATION_CLEANUP_PLAN.md) - This cleanup initiative

---

## 🎯 By Topic

### Cost Optimization
- [SPEC-KIT-070](SPEC-KIT-070-model-cost-optimization/) - Model cost optimization
- [Phase 1 Complete](SPEC-KIT-070-model-cost-optimization/PHASE1_COMPLETE.md)
- [Phase 1A Results](SPEC-KIT-070-model-cost-optimization/PHASE1A_RESULTS.md)
- [Phase 2 Complexity Routing](SPEC-KIT-070-model-cost-optimization/PHASE2_COMPLEXITY_ROUTING.md)
- [Consensus Cost Audit](spec-kit/consensus-cost-audit-packet.md)

### Memory Systems
- [SPEC-KIT-071](SPEC-KIT-071-memory-system-optimization/) - Memory optimization
- [Root Cause Analysis](SPEC-KIT-071-memory-system-optimization/ROOT_CAUSE_ANALYSIS.md)
- [SPEC-KIT-010](SPEC-KIT-010-local-memory-migration/) - Local memory migration
- [Memory Policy](../codex-rs/MEMORY-POLICY.md) - Usage policy

### Testing & Quality
- [SPEC-KIT-045](SPEC-KIT-045-design-systematic-testing-framework-for/) - Testing framework
- [SPEC-KIT-068](SPEC-KIT-068-analyze-and-fix-quality-gates/) - Quality gates
- [SPEC-KIT-060](SPEC-KIT-060-template-validation-test/) - Template validation
- [Testing Policy](spec-kit/testing-policy.md) - Testing standards
- [QA Sweep Checklist](spec-kit/qa-sweep-checklist.md)

### ACE (Agentic Context Engine)
- [ACE Integration](../ACE_INTEGRATION.md) - Main integration guide
- [ACE Learning Usage](../ACE_LEARNING_USAGE.md) - Learning system
- [ACE Test Plan](../ACE_TEST_PLAN.md) - Testing strategy
- [ACE Testing Guide](../ACE_TESTING_GUIDE.md) - Testing procedures
- [ACE Injection Fix Report](../codex-rs/ACE_INJECTION_FIX_REPORT.md) - Bug fixes

### Upstream Sync
- [SPEC-KIT-030](SPEC-KIT-030-add-documentation-for-rebasing-from/) - Rebase documentation
- [SPEC-KIT-015](SPEC-KIT-015-nightly-sync/) - Nightly sync automation
- [Upstream Sync](../docs/UPSTREAM-SYNC.md) - Sync procedures
- [Rebase Safety Matrix](spec-kit/REBASE_SAFETY_MATRIX_T80-T90.md)

---

## 🔗 External Resources

### Upstream
- **This Fork**: https://github.com/theturtlecsz/code
- **Upstream**: https://github.com/just-every/code
- **Origin**: OpenAI Codex (community fork)

### Tools & Libraries
- [Ratatui](https://ratatui.rs/) - TUI framework
- [Model Context Protocol](https://modelcontextprotocol.io/) - MCP specification

---

## 📊 Health Reports

### Documentation Health
- [Latest Report](report/docs-report.md) - Auto-generated health analysis
- [Index JSON](report/docs-index.json) - Machine-readable index
- **Run**: `node /path/to/doc-curator/scripts/docscan.mjs --full` to regenerate

### Evidence Health
- **Run**: `/spec-evidence-stats` to check evidence footprint
- **Current Status**: All SPECs within 25 MB soft limit ✅

---

## 📝 Contributing

### For New SPECs
1. Run `/speckit.new <description>` to create SPEC
2. Follow the spec-kit workflow: Plan → Tasks → Implement → Validate → Audit → Unlock
3. Document evidence in `docs/SPEC-OPS-004-.../evidence/commands/<SPEC-ID>/`
4. Update this SUMMARY.md with link to new SPEC
5. Update [SPEC.md](../SPEC.md) task tracker

### For Documentation Updates
1. Keep this SUMMARY.md updated when adding new docs
2. Add cross-references between related documents
3. Use git mv for file reorganization (preserve history)
4. Run doc health scan periodically to identify issues

---

## 🏷️ Tags & Categories

**By Status**: `active`, `completed`, `archived`, `deprecated`
**By Domain**: `infrastructure`, `testing`, `cost-optimization`, `memory`, `quality`, `documentation`
**By Priority**: `p0-critical`, `p1-high`, `p2-medium`, `p3-low`

See individual SPEC directories for detailed categorization.

---

**Questions or Issues?** Check [CLAUDE.md](../CLAUDE.md) for operational guidelines or [SPEC.md](../SPEC.md) for current work items.
