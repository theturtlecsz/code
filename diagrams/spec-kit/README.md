# Spec-Kit Workflow Diagrams

**Generated**: 2025-10-29
**Purpose**: Comprehensive visualization of spec-kit multi-agent automation workflow
**Diagrams**: 5 focused views covering user journey, technical workflow, and architecture

---

## 📊 Diagram Suite Overview

| # | Diagram | Purpose | Audience | Complexity |
|---|---------|---------|----------|------------|
| 1 | User Journey | High-level command flow | Product, Users | Simple |
| 2 | Pipeline State Machine | Detailed technical workflow | Engineers | Complex |
| 3 | Multi-Agent Consensus | How AI agents collaborate | Engineers, AI/ML | Moderate |
| 4 | Module Architecture | Refactored code structure | Engineers | Moderate |
| 5 | File System Artifacts | Where files live | Engineers, Ops | Simple |

---

## 1. User Journey Flowchart

**File**: `1-user-journey.{dot,svg,png}`
**View**: High-level user perspective

**Shows**:
- Command entry points (`/speckit.new`, `/speckit.auto`, etc.)
- Optional quality commands (`/speckit.clarify`, `/speckit.analyze`, `/speckit.checklist`)
- 6 sequential stages (Plan → Tasks → Implement → Validate → Audit → Unlock)
- Timing and cost per stage (~10min, ~$1 for most stages)
- Retry paths (validate can retry implement+validate up to 2x)
- Monitoring via `/speckit.status`

**Key Insights**:
- `/speckit.auto` runs full 6-stage pipeline (~60min, ~$11)
- Quality commands are optional but recommended for complex specs
- Each stage requires consensus before advancing
- Status command provides real-time dashboard

**Use Case**: Onboarding new users, explaining workflow to stakeholders

---

## 2. Pipeline State Machine

**File**: `2-pipeline-state-machine.{dot,svg,png}`
**View**: Detailed technical implementation

**Shows**:
- 4 pipeline phases: Guardrail → ExecutingAgents → CheckingConsensus → Advance
- Quality gate checkpoints (pre-planning, post-plan, post-tasks)
- Guardrail validation (shell scripts checking policy compliance)
- ACE routing selection (aggregator effort: low/medium/high)
- Agent spawn with tier selection (Tier 2: 3 agents, Tier 3: 4 agents, Tier 4: 5 agents)
- Retry logic at 3 levels:
  - Agent retry (max 3 attempts)
  - Consensus retry (max 3 attempts)
  - Validate retry (max 2 attempts, special case)
- Cost persistence and stage advancement

**Key Insights**:
- Pipeline is a loop: while (current_index < stages.len())
- Each stage goes through same 4-phase cycle
- Retries add context ("Previous attempt failed, be more thorough")
- Quality gates can block progression (human intervention)
- Validate stage has special retry logic (re-runs implement + validate together)

**Use Case**: Understanding system behavior, debugging pipeline issues, onboarding engineers

---

## 3. Multi-Agent Consensus Flow

**File**: `3-multi-agent-consensus.{dot,svg,png}`
**View**: How AI agents collaborate

**Shows**:
- Agent tier selection (Tier 2/3/4)
- 5 agents: Gemini, Claude, Code (native), GPT-Codex, GPT-Pro
- Parallel execution (all agents run simultaneously)
- Local-memory storage (each agent stores verdict with importance: 8)
- Consensus synthesis:
  - Unanimous (3/3 or 4/4): consensus_ok=true, degraded=false
  - Majority (2/3): consensus_ok=true, degraded=true (schedules follow-up checklist)
  - Conflict (<2/3): consensus_ok=false (retry or halt)
- Cost per agent (Gemini: $0.15, Claude: $0.30, Code: $0, GPT-Codex: $0.50, GPT-Pro: $0.40)
- Degraded consensus handling (auto-schedule `/speckit.checklist`)

**Key Insights**:
- Code agent is free (native Rust implementation)
- Degraded consensus still advances but schedules quality check
- Conflict triggers retry with "Previous attempt failed" context
- Each agent independently analyzes same prompt
- Consensus is determined by agreement on verdict fields

**Use Case**: Understanding multi-agent design, debugging consensus failures, cost analysis

---

## 4. Module Architecture

**File**: `4-module-architecture.{dot,svg,png}`
**View**: Refactored code structure (post-2025-10-29)

**Shows**:
- 7 architectural layers:
  1. Entry Points (routing, command registry)
  2. Command Handlers (132 lines) - NEW extraction
  3. Workflow Coordinators (pipeline: 677 lines, consensus: 190 lines) - NEW extractions
  4. Orchestrators (agent: 519 lines, validation: 170 lines) - NEW extractions
  5. Core Infrastructure (consensus, state, evidence, guardrail)
  6. Quality Gates & ACE (quality gate handler, broker, ACE modules)
  7. External Services (MCP, filesystem, AI agents)
- handler.rs as re-export facade (35 lines, was 1,561)
- Clean DAG: No circular dependencies
- Data flow: ChatWidget → Routing → Handlers → Coordinators → Core → External

**Key Insights**:
- handler.rs reduced by 98% (1,561 → 35 lines)
- 5 new modules with single responsibilities
- Free function pattern avoids borrow checker issues
- Re-export facade maintains backward compatibility
- Clear separation: routing → coordination → execution → storage

**Use Case**: Code review, refactoring validation, onboarding engineers to codebase

---

## 5. File System Artifacts Map

**File**: `5-filesystem-artifacts.{dot,svg,png}`
**View**: Where all files live

**Shows**:
- SPEC directories: `docs/SPEC-<ID>-<slug>/`
- SPEC artifacts:
  - spec.md (template applied by /speckit.new)
  - PRD.md (created by /speckit.specify)
  - plan.md (created by /speckit.plan)
  - tasks.md (created by /speckit.tasks)
- SPEC.md (single source of truth task tracker)
- Evidence repository: `docs/SPEC-OPS-004.../evidence/`
  - commands/<SPEC-ID>/ (guardrail telemetry)
  - consensus/<SPEC-ID>/ (agent artifacts)
  - archive/ (old runs)
- Cost summaries: `cost-summaries/<SPEC-ID>/`
- Templates (embedded in Rust code)
- Prompts (docs/spec-kit/prompts.json)
- Guardrail scripts (scripts/spec_ops_004/*.sh)
- Local-memory database (~/.local/share/local-memory/)

**Key Insights**:
- Evidence isolated from spec content (reduces SPEC directory clutter)
- Templates versioned with code (Rust constants)
- Guardrails are shell scripts (could be nativized)
- Local-memory is external service (MCP server)

**Use Case**: Understanding file locations, troubleshooting missing files, evidence audits

---

## 🎯 How to Use These Diagrams

### For New Team Members

**Start with**:
1. User Journey (understand commands)
2. File System (know where things are)
3. Pipeline State Machine (see the workflow)

### For Engineers

**Start with**:
1. Module Architecture (understand code structure)
2. Pipeline State Machine (debug issues)
3. Multi-Agent Consensus (optimize costs)

### For Debugging

**Pipeline Stuck?** → Check Pipeline State Machine for retry paths
**Consensus Failed?** → Check Multi-Agent Consensus for conflict resolution
**Missing Files?** → Check File System Artifacts for expected locations
**Module Confusion?** → Check Module Architecture for responsibilities

---

## 🔄 Regenerating Diagrams

### Prerequisites

```bash
# Install GraphViz
sudo apt-get install graphviz  # Ubuntu/Debian
brew install graphviz           # macOS
```

### Generate All Diagrams

```bash
cd diagrams/spec-kit/
for f in *.dot; do
  base="${f%.dot}"
  dot -Tsvg "$f" -o "${base}.svg"
  dot -Tpng "$f" -o "${base}.png"
  echo "Generated ${base}.{svg,png}"
done
```

### Generate Individual Diagram

```bash
dot -Tsvg 1-user-journey.dot -o 1-user-journey.svg
dot -Tpng 1-user-journey.dot -o 1-user-journey.png
```

### Customization

Edit `.dot` files to:
- Update node labels
- Add/remove stages
- Change colors (fillcolor="#XXXXXX")
- Adjust layout (rankdir=TB or LR)
- Add annotations

---

## 📁 File Structure

```
diagrams/spec-kit/
├── README.md (this file)
├── GAPS_AND_ISSUES.md (analysis findings)
│
├── 1-user-journey.dot (source)
├── 1-user-journey.svg (output)
├── 1-user-journey.png (output)
│
├── 2-pipeline-state-machine.dot
├── 2-pipeline-state-machine.svg
├── 2-pipeline-state-machine.png
│
├── 3-multi-agent-consensus.dot
├── 3-multi-agent-consensus.svg
├── 3-multi-agent-consensus.png
│
├── 4-module-architecture.dot
├── 4-module-architecture.svg
├── 4-module-architecture.png
│
├── 5-filesystem-artifacts.dot
├── 5-filesystem-artifacts.svg
└── 5-filesystem-artifacts.png
```

---

## 🎨 Color Coding

### Diagram Color Scheme

- **Blue** (#E3F2FD, #90CAF9): User commands, pipeline stages
- **Yellow** (#FFF9C4, #FFE082): Decision points, checkpoints
- **Green** (#C8E6C9, #A5D6A7): Success paths, outputs, core infrastructure
- **Orange** (#FFCCBC, #FFAB91): Retry logic, quality gates, warnings
- **Red** (#FFCDD2, #EF5350): Failures, halts, errors
- **Purple** (#E1BEE7, #CE93D8): Special modules (ChatWidget, agents)
- **Teal** (#B2DFDB, #80CBC4): Coordinators, external services

**Consistent across all diagrams** for easy mental mapping.

---

## 📖 Related Documentation

- **SPEC_KIT_ARCHITECTURE_COMPLETE.md**: Full 20-section architecture guide (866 lines)
- **SPEC_KIT_RESEARCH_INDEX.md**: Quick reference with statistics (497 lines)
- **CLAUDE.md**: User-facing playbook (how to use spec-kit)
- **docs/spec-kit/**: Configuration, prompts, policies

---

## 🔗 Cross-References

**From Diagrams to Code**:
- User Journey → `routing.rs:try_dispatch_spec_kit_command`
- Pipeline State Machine → `pipeline_coordinator.rs:advance_spec_auto`
- Multi-Agent Consensus → `consensus.rs:run_spec_consensus`
- Module Architecture → All 5 refactored modules
- File System → `evidence.rs`, `spec_prompts.rs`

**From Code to Diagrams**:
- When reading `pipeline_coordinator.rs` → See Diagram 2 for flow
- When debugging consensus → See Diagram 3 for agent interaction
- When understanding module boundaries → See Diagram 4
- When looking for a file → See Diagram 5

---

## 🤝 Contributing

### Adding New Diagrams

1. Create `N-diagram-name.dot` in this directory
2. Follow naming convention: `N-lowercase-with-hyphens.dot`
3. Use consistent color scheme (see above)
4. Generate outputs: `dot -Tsvg N-diagram-name.dot -o N-diagram-name.svg`
5. Update this README with description
6. Commit all three files (.dot, .svg, .png)

### Updating Existing Diagrams

1. Edit the `.dot` source file
2. Regenerate outputs (see "Regenerating Diagrams" above)
3. Commit changes with clear description of what changed
4. Update README if diagram purpose/scope changed

---

## ⚠️ Important Notes

### Diagram Accuracy

- Diagrams reflect system state as of **2025-10-29**
- If workflow changes (new stages, different retry logic), **update diagrams**
- Diagrams are documentation, not spec - code is source of truth

### Limitations

- Diagrams show happy path + major error paths (not every edge case)
- Some details omitted for clarity (e.g., telemetry schema internals)
- Timing/cost estimates are approximate (actual varies)

### When Diagrams Become Outdated

**Update diagrams if**:
- New stage added (e.g., /speckit.review)
- Retry logic changes (e.g., max attempts increased)
- Module refactoring occurs (e.g., split agent_orchestrator further)
- Consensus algorithm changes (e.g., different thresholds)
- File structure changes (e.g., new evidence subdirectory)

**How to detect**:
- Code review catches architectural changes
- Regular audits (quarterly)
- When onboarding new engineers (fresh eyes spot inconsistencies)

---

## 📚 Reading Guide

### First Time Readers

**Recommended order**:
1. Start with Diagram 1 (User Journey) - Get the big picture
2. Read SPEC_KIT_RESEARCH_INDEX.md - Quick facts
3. Examine Diagram 5 (File System) - Know where things are
4. Dive into Diagram 2 (Pipeline) - Understand workflow
5. Study Diagram 3 (Consensus) - See multi-agent magic
6. Review Diagram 4 (Architecture) - Understand code structure

### For Specific Questions

- **"How do I use spec-kit?"** → Diagram 1
- **"Why did my pipeline fail?"** → Diagram 2 + GAPS_AND_ISSUES.md
- **"How do agents reach consensus?"** → Diagram 3
- **"Where is this code?"** → Diagram 4
- **"Where is this file?"** → Diagram 5

---

## 🎓 Learning Path

### Beginner (Day 1)

- [x] Read CLAUDE.md (playbook)
- [x] View Diagram 1 (user journey)
- [x] Run `/speckit.status` to see dashboard
- [x] Create first SPEC with `/speckit.new`

### Intermediate (Week 1)

- [x] Study Diagram 2 (pipeline mechanics)
- [x] Understand retry logic
- [x] Monitor evidence with `/spec-evidence-stats`
- [x] Run full `/speckit.auto` workflow

### Advanced (Month 1)

- [x] Analyze Diagram 3 (consensus algorithm)
- [x] Examine Diagram 4 (module architecture)
- [x] Review quality gate logic
- [x] Contribute to spec-kit codebase

---

## 🔍 Analysis Findings

**Comprehensive analysis document**: `GAPS_AND_ISSUES.md`

**Summary**:
- ✅ **0 critical issues** - System is production-ready
- ⚠️ **3 high-priority gaps** - Mostly documentation and UX
- 🟢 **5 medium-priority opportunities** - Performance and features
- 💡 **7 low-priority enhancements** - Nice-to-haves

**Top Findings**:
1. Missing rollback/undo mechanism
2. Incomplete error recovery documentation
3. Quality gate human intervention UX unclear
4. Telemetry retention policy not automated
5. Cost tracking lacks trend analysis

**See GAPS_AND_ISSUES.md for full details and recommendations.**

---

## 🏗️ Architecture Validation

### Questions These Diagrams Answer

✅ **Is the module refactoring correct?**
→ Diagram 4 shows clean DAG with no circular dependencies

✅ **Does the workflow have gaps?**
→ Diagrams 1-2 show complete flow from user command to artifacts

✅ **Is the retry logic sound?**
→ Diagram 2 shows 3 independent retry systems for distinct failure modes

✅ **Can the system scale?**
→ Diagrams reveal sequential bottleneck (potential for parallel stages)

✅ **Is cost tracking comprehensive?**
→ Diagram 3 shows per-agent costs, Diagram 2 shows persistence

---

## 💾 Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-10-29 | Initial creation: 5 diagrams + analysis |

---

## 🤝 Maintenance

### Diagram Review Schedule

- **After major features**: Update affected diagrams
- **Quarterly audit**: Review all diagrams for accuracy
- **Before releases**: Verify diagrams match code

### Responsibilities

- **Diagram accuracy**: Engineering team
- **Generating outputs**: Automated (run dot commands)
- **Keeping README updated**: Document owner
- **Gap analysis**: Periodic ultrathink sessions

---

## 📞 Feedback

**Found inaccuracies?** Update the diagrams and open a PR
**Diagrams unclear?** Suggest improvements in issues
**Need different view?** Propose new diagram types

---

**Document Version**: 1.0
**Maintainer**: theturtlecsz/code team
**Last Updated**: 2025-10-29
