# Spec-Kit Workflow System - Research & Reference Index

**Document Generated**: 2025-10-29  
**Repository**: https://github.com/theturtlecsz/code (FORK)  
**Framework**: Multi-agent spec-kit automation, native Rust, MCP integration  

---

## QUICK START FOR DIAGRAM CREATION

### Key Architectural Layers (6 layers)

1. **Routing Layer** (`routing.rs`) - Command dispatch, config merging
2. **Command Registry** (`command_registry.rs`) - Dynamic trait-based pattern
3. **Pipeline Coordination** (`pipeline_coordinator.rs`) - State machine, stage progression
4. **Agent Orchestration** (`agent_orchestrator.rs`) - Agent spawning, ACE routing, cost tracking
5. **Consensus** (`consensus.rs`, `consensus_coordinator.rs`) - Agreement validation, local-memory synthesis
6. **Quality Gates** (`quality.rs`, `quality_gate_handler.rs`) - 3 checkpoints, auto-resolution

### Core Data Structures

```
SpecAutoState (current pipeline state)
  ├── spec_id: String
  ├── current_index: usize
  ├── phase: SpecAutoPhase (enum: Guardrail, ExecutingAgents, CheckingConsensus, QualityGate*)
  ├── stages: Vec<SpecStage> (Plan, Tasks, Implement, Validate, Audit, Unlock)
  └── cost_summary: CostSummary

SpecAutoPhase (state machine phases)
  ├── Guardrail
  ├── ExecutingAgents { expected_agents, completed_agents }
  ├── CheckingConsensus
  ├── QualityGateExecuting { checkpoint, gates, active_gates, expected_agents, completed_agents, results }
  ├── QualityGateProcessing { checkpoint, auto_resolved, escalated }
  ├── QualityGateValidating { checkpoint, auto_resolved, pending_validations, completed_validations }
  └── QualityGateAwaitingHuman { checkpoint, escalated_issues, escalated_questions, answers }
```

### File Organization

```
/home/thetu/code/
├── docs/spec-kit/                           # Documentation (11 guides + prompts.json)
│   ├── prompts.json                        # 28 KB, 13 stage prompts (13 agents × stages)
│   ├── plan-template.md / tasks-template.md / implement-template.md (11 total)
│   └── COMMAND_INVENTORY.md, COMMAND_REGISTRY_DESIGN.md, QUALITY_GATES_DESIGN.md
├── templates/                               # Markdown templates (11 files)
│   ├── spec-template.md, plan-template.md, tasks-template.md
│   ├── implement-template.md, validate-template.md
│   ├── audit-template.md, unlock-template.md
│   └── clarify-template.md, analyze-template.md, checklist-template.md, PRD-template.md
├── codex-rs/tui/src/chatwidget/spec_kit/   # Rust implementation (33 modules, 375 KB)
│   ├── mod.rs                              # Module exports (43 lines)
│   ├── handler.rs                          # Re-exports (backward compat, 36 lines)
│   ├── routing.rs                          # Command dispatch (205 lines)
│   ├── command_registry.rs                 # Dynamic registry (17 KB)
│   ├── pipeline_coordinator.rs             # State machine (27 KB)
│   ├── agent_orchestrator.rs               # Agent spawning (18 KB)
│   ├── consensus.rs                        # Consensus logic (35 KB)
│   ├── consensus_coordinator.rs            # Native MCP (7 KB)
│   ├── quality.rs                          # Issue classification (30 KB)
│   ├── quality_gate_handler.rs             # Checkpoints (45 KB)
│   ├── state.rs                            # State machine enum (28 KB)
│   ├── cost_tracker.rs                     # Cost tracking (18 KB)
│   ├── ace_*.rs                            # ACE modules (9-22 KB each)
│   ├── evidence.rs                         # Repository abstraction (22 KB)
│   ├── error.rs                            # Error types (7 KB)
│   ├── context.rs                          # SpecKitContext trait (11 KB)
│   ├── commands/                           # Command implementations
│   │   ├── mod.rs, plan.rs, quality.rs, special.rs, status.rs, guardrail.rs
│   │   └── ... (22 command handlers)
│   └── tests/                              # 8 test files, 604 tests, 100% pass
├── docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
│   ├── commands/<SPEC-ID>/                 # Guardrail telemetry
│   │   ├── spec-plan_<TIMESTAMP>.json
│   │   ├── spec-tasks_<TIMESTAMP>.json
│   │   ├── spec-implement_<TIMESTAMP>.json
│   │   ├── spec-validate_<TIMESTAMP>.json
│   │   ├── spec-audit_<TIMESTAMP>.json
│   │   ├── spec-unlock_<TIMESTAMP>.json
│   │   └── cost-summary.json
│   └── consensus/<SPEC-ID>/                # Multi-agent consensus
│       ├── spec-plan/
│       │   ├── gemini_<UUID>.json
│       │   ├── claude_<UUID>.json
│       │   ├── gpt_pro_<UUID>.json
│       │   ├── consensus_verdict.json
│       │   └── spec-plan_synthesis.json
│       ├── spec-tasks/, spec-implement/, spec-validate/, spec-audit/, spec-unlock/
│       └── ... (similar structure)
└── docs/SPEC-KIT-<ID>-<slug>/              # Per-SPEC artifacts
    ├── spec.md                             # SPEC document
    ├── plan.md                             # Work breakdown
    ├── tasks.md                            # Task decomposition
    └── PRD.md                              # Optional: Product requirements
```

---

## COMPLETE STAGE PIPELINE

### 1. PLAN Stage
**Agents**: Gemini (Researcher), Claude (Synthesizer), GPT-Pro (Executor/QA)  
**Input**: SPEC document, constitution, PRD  
**Output**: `docs/SPEC-KIT-<ID>/plan.md` + consensus artifacts  
**Cost**: ~$1.00 | **Time**: 10-12 min  

**Sub-steps**:
1. Guardrail shell: `/guardrail.plan <SPEC-ID>` (bash validation)
2. Prompt building: `build_stage_prompt(SpecStage::Plan, spec_id)`
3. Agent dispatch: 3 agents get (Gemini research, Claude synthesis, GPT-Pro QA) prompts
4. JSON collection: 3 agent responses → local-memory
5. Consensus: `run_consensus_with_retry()` → consensus_verdict.json
6. Quality gate: Post-plan consistency check (optional)
7. Advance: `advance_spec_auto()` → next stage or escalate

### 2. TASKS Stage
**Agents**: Gemini (Researcher), Claude (Synthesizer), GPT-Pro (Executor/QA)  
**Input**: SPEC, plan.md, constitution  
**Output**: `docs/SPEC-KIT-<ID>/tasks.md` + consensus  
**Cost**: ~$1.00 | **Time**: 10-12 min  

### 3. IMPLEMENT Stage (Tier 3: Quad-Agent)
**Agents**: Gemini (Researcher), Claude (Strategist), GPT-Codex (Code), GPT-Pro (Executor/QA)  
**Input**: SPEC, plan.md, tasks.md, repo context  
**Output**: `docs/SPEC-KIT-<ID>/implement-notes.md` + code diffs  
**Cost**: ~$2.00 | **Time**: 15-20 min  

### 4. VALIDATE Stage
**Agents**: Gemini (Researcher), Claude (Analyzer), GPT-Pro (Executor/QA)  
**Input**: Test scenarios, telemetry, acceptance criteria  
**Output**: Test strategy consensus, validation report  
**Cost**: ~$1.00 | **Time**: 10-12 min  
**Special**: Single-flight guard (SPEC-KIT-069) prevents duplicate dispatch

### 5. AUDIT Stage
**Agents**: Gemini (Researcher), Claude (Analyzer), GPT-Pro (Executor/QA)  
**Input**: Diffs, telemetry, guardrail outputs  
**Output**: Compliance checklist, audit memo  
**Cost**: ~$1.00 | **Time**: 10-12 min  

### 6. UNLOCK Stage (Final)
**Agents**: Gemini (Researcher), Claude (Analyzer), GPT-Pro (Executor/QA)  
**Input**: Branch state, PRs, guardrail status  
**Output**: Final approval or escalation  
**Cost**: ~$1.00 | **Time**: 10-12 min  

---

## QUALITY GATES (3 Checkpoints)

### Checkpoint 1: Pre-Planning
**Trigger**: Before /speckit.plan  
**Commands**:
- `/speckit.clarify <SPEC-ID>` (resolve ambiguities)
- `/speckit.checklist <SPEC-ID>` (requirement quality)

**Resolution**:
- High confidence + unanimous → Auto-apply
- Medium confidence + unanimous → Auto-apply with flag
- Low confidence → Escalate to user

### Checkpoint 2: Post-Plan
**Trigger**: After plan.md generated  
**Commands**:
- `/speckit.analyze <SPEC-ID>` (consistency vs spec)
- Validation: acceptance mapping coverage

**Auto-Fixes**:
- Minor terminology issues
- Coverage gaps (suggest additions)
- Contradictions (flag for review)

### Checkpoint 3: Post-Tasks
**Trigger**: After tasks.md generated  
**Commands**:
- `/speckit.analyze <SPEC-ID>` (coverage vs plan)
- Validation: task ordering, dependencies

**Issue Classification**:
```
Confidence: High | Medium | Low
Magnitude: Critical | Important | Minor
Resolvability: AutoFix | SuggestFix | NeedHuman
```

---

## CONSENSUS TYPES & RESOLUTION

### Type 1: Unanimous (All 3 agents agree)
- **Rate**: 55% typical
- **Resolution**: Auto-apply immediately
- **No escalation**: User doesn't see this

### Type 2: Majority (2/3 agents)
- **Rate**: 35% typical
- **Validation**: GPT-5 checks if answer aligns with SPEC intent
- **Decision Tree**:
  - If GPT-5 validates AND confidence high → Apply
  - If GPT-5 validates AND confidence medium → Apply with flag
  - If GPT-5 disagrees OR confidence low → Escalate

### Type 3: Conflict (No majority)
- **Rate**: <5% (0% observed in 26 tasks)
- **Resolution**: Quality gate modal → User answers
- **Storage**: User answer → Auto-modify artifacts

---

## COST BREAKDOWN (SPEC-KIT-070)

### Model Pricing (Oct 2025)

| Model | Input $/M | Output $/M | Primary Use | Status |
|-------|-----------|-----------|------------|--------|
| Claude Haiku | 0.25 | 1.25 | Fast analysis (12x cheaper) | Default |
| Gemini 2.5 Flash | 0.10 | 0.40 | Research (12.5x cheaper) | Deployed |
| GPT-4o | 2.50 | 10.00 | Aggregator backup | Rate limited |
| GPT-5 (est.) | 10.00 | 30.00 | Code gen, validation | Future |

### Cost Per Stage

**Plan**: 
- Gemini: ~0.40 (research)
- Claude: ~0.40 (synthesis)
- GPT-Pro: ~0.20 (QA)
- **Total**: ~$1.00

**Implement** (Tier 3):
- Gemini: ~0.50
- Claude: ~0.50
- GPT-Codex: ~0.60
- GPT-Pro: ~0.40
- **Total**: ~$2.00

**Full Pipeline** (all 6 stages):
- **Before**: $11.00 (~$1,148/month at 104 runs/month)
- **After Phase 1**: $5.50-6.60 (~$550-660/month)
- **Savings**: 40-50% ($488-598/month)

### Cost Telemetry JSON
```json
{
  "spec_id": "SPEC-KIT-070",
  "total_cost_usd": 6.47,
  "agents": {
    "gemini": { "input_tokens": 2500, "output_tokens": 1200, "cost": 0.41 },
    "claude": { "input_tokens": 3000, "output_tokens": 1500, "cost": 1.50 },
    "gpt_pro": { "input_tokens": 2800, "output_tokens": 900, "cost": 2.56 }
  },
  "stages": {
    "plan": { "cost": 1.47, "agents": ["gemini", "claude", "gpt_pro"] },
    "tasks": { "cost": 1.35, "agents": ["gemini", "claude", "gpt_pro"] },
    "implement": { "cost": 2.14, "agents": ["gemini", "claude", "gpt_codex", "gpt_pro"] },
    "validate": { "cost": 0.95, "agents": ["gemini", "claude", "gpt_pro"] },
    "audit": { "cost": 0.32, "agents": ["gemini", "claude"] },
    "unlock": { "cost": 0.24, "agents": ["claude"] }
  },
  "budget_remaining": 93.53
}
```

---

## KEY MODULES DEPENDENCY MAP

```
routing.rs
  └─→ command_registry.rs (find command)
      └─→ commands/*.rs (execute or expand_prompt)
          └─→ pipeline_coordinator.rs (handle_spec_auto)
              └─→ agent_orchestrator.rs (auto_submit_spec_stage_prompt)
                  ├─→ ace_route_selector.rs (decide routing)
                  ├─→ cost_tracker.rs (track costs)
                  └─→ consensus_coordinator.rs (block_on_sync MCP)
                      └─→ consensus.rs (run_consensus_with_retry)
                          ├─→ evidence.rs (write_consensus_synthesis)
                          ├─→ quality.rs (parse_quality_issue_from_agent)
                          └─→ state.rs (update phase)
                              └─→ quality_gate_handler.rs (determine_quality_checkpoint)
                                  └─→ on_quality_gate_agents_complete (execute gates)
                                      └─→ pipeline_coordinator.rs (advance_spec_auto)
```

---

## COMMANDS REFERENCE

### Tier 0 (Native, $0)
- `/speckit.status <SPEC-ID>` - TUI dashboard (instant)

### Tier 2-lite (Dual, ~$0.35)
- `/speckit.checklist <SPEC-ID>` - Claude + Code agents

### Tier 2 (Triple, ~$0.80-1.00)
- `/speckit.new <description>` - Create new SPEC
- `/speckit.specify <SPEC-ID>` - Draft/update PRD
- `/speckit.clarify <SPEC-ID>` - Resolve ambiguities
- `/speckit.analyze <SPEC-ID>` - Cross-artifact check
- `/speckit.plan <SPEC-ID>` - Work breakdown
- `/speckit.tasks <SPEC-ID>` - Task decomposition
- `/speckit.validate <SPEC-ID>` - Test strategy
- `/speckit.audit <SPEC-ID>` - Compliance check
- `/speckit.unlock <SPEC-ID>` - Final approval

### Tier 3 (Quad, ~$2.00)
- `/speckit.implement <SPEC-ID>` - Code generation

### Tier 4 (Dynamic, ~$11)
- `/speckit.auto <SPEC-ID>` - Full 6-stage pipeline

### Guardrail Wrappers (bash validation)
- `/guardrail.plan`, `/guardrail.tasks`, `/guardrail.implement`
- `/guardrail.validate`, `/guardrail.audit`, `/guardrail.unlock`
- `/guardrail.auto`

### Utilities
- `/spec-consensus <SPEC-ID> <stage>` - Query local-memory
- `/spec-evidence-stats` - Monitor footprint (25 MB soft limit)

---

## TESTING STATISTICS

**Test Count**: 604 total, 100% pass rate
- Unit tests: 434 (256 new + 178 baseline)
- Integration: 60 across 5 categories
- E2E: 21 pipeline tests
- Property-based: 10 tests × 256 cases = 2,560 generative

**Coverage**: 42-48% (estimated, spec-kit module)
- Target: 40% by Q1 2026
- Achievement: Oct 2025 (4 months early)

**Test Files** (8 main suites):
```
spec_kit_handler_orchestration_tests.rs       (58 tests)
spec_kit_consensus_logic_tests.rs             (42 tests)
spec_kit_quality_resolution_tests.rs          (33 tests)
spec_kit_evidence_tests.rs                    (24 tests)
spec_kit_guardrail_tests.rs                   (25 tests)
spec_kit_state_tests.rs                       (27 tests)
spec_kit_schemas_tests.rs                     (21 tests)
spec_kit_error_tests.rs                       (26 tests)
spec_kit_edge_cases_tests.rs                  (25 tests)
spec_kit_property_based_tests.rs              (10 tests)
```

---

## DOCUMENTATION ARTIFACTS

**Location**: `/home/thetu/code/SPEC_KIT_ARCHITECTURE_COMPLETE.md` (866 lines)

**Covers**:
1. Executive summary + key stats
2. 6 stages with agent roles, costs, timings
3. 6-layer architecture with module responsibilities
4. Prompt system and template references
5. File system structure (SPEC dirs, evidence repo)
6. Consensus types and synthesis flow
7. Quality gates (3 checkpoints, issue classification)
8. Cost tracking and optimization (SPEC-KIT-070)
9. Command handlers (13 prompt-expanding + 6 direct)
10. State machine (SpecAutoPhase enum, transitions)
11. Validation lifecycle tracking (single-flight guard)
12. Module responsibilities (14 key modules, 375 KB code)
13. Entry points and dispatch flow
14. Backward compatibility and aliases
15. Fork-specific markers (80+ across 33 files)
16. Telemetry schema (command + consensus)
17. Environment and config
18. Error handling and resilience (AR-1 through AR-4)
19. Testing infrastructure (604 tests, 42-48% coverage)
20. Documentation ecosystem (12 key docs)

---

## KEY STATISTICS

| Metric | Value |
|--------|-------|
| **Rust Code** | 375 KB (spec-kit module) |
| **Markdown Docs** | 11 templates + 12 guides |
| **Commands** | 13 /speckit.* + 7 /guardrail.* + 3 utilities |
| **Stages** | 6 sequential (Plan → Unlock) |
| **Agents** | 5 (Gemini, Claude, Code, GPT-Codex, GPT-Pro) |
| **Tiers** | 4 (0, 2-lite, 2, 3, 4) |
| **Quality Checkpoints** | 3 (pre-plan, post-plan, post-tasks) |
| **Tests** | 604 (100% pass, 42-48% coverage) |
| **Evidence Dirs** | 2 (commands/, consensus/) |
| **Cost Reduction** | 40-50% ($488-598/month) |
| **MCP Latency** | 8.7 ms (native) |
| **Fork Isolation** | 98.8% (80+ markers, 33 files) |
| **Lines of Tests** | ~8,800 (604 tests) |
| **SPEC-KIT-<ID> Dirs** | 25 MB soft limit per spec |
| **Evidence Retention** | 30d archive, 90d offload, 180d purge |

---

## FOR DIAGRAM CREATION

### High-Level Flow (User → Result)

```
User Input "/speckit.auto SPEC-KIT-070"
  ↓
[Routing] try_dispatch_spec_kit_command()
  ↓
[Registry] Find "speckit.auto" in command registry
  ↓
[Config Merge] Combine user + subagent_defaults
  ↓
[Prompt Build] build_stage_prompt_with_mcp(SpecStage::Plan, spec_id)
  ↓
[ACE Routing] decide_stage_routing() → aggregator effort
  ↓
[Agent Dispatch] MCP manager spawns 3-4 agents
  ├─ Gemini (research)
  ├─ Claude (synthesis)
  ├─ Code/GPT-Codex/GPT-Pro (validation/code)
  └─ (async, parallel execution)
  ↓
[Callback] on_spec_auto_agents_complete()
  ↓
[Consensus] run_consensus_with_retry()
  ├─ Unanimous? → Auto-apply (55%)
  ├─ 2/3? → GPT-5 validate (10-15%)
  └─ Conflict? → Escalate to user (<5%)
  ↓
[Quality Gate] determine_quality_checkpoint()
  ├─ Auto-fix minor issues
  ├─ Validate majority with GPT-5
  └─ Escalate critical
  ↓
[Storage] write_consensus_synthesis() + cost_summary.json
  ↓
[Advance] advance_spec_auto() → next stage
  ↓
[Output] Plan created + consensus artifacts stored
```

### State Machine Diagram

```
START
  ↓
[Guardrail] Execute shell validation
  ├─ Failed? → Halt with error
  └─ Success? → Continue
  ↓
[ExecutingAgents] Fan out 3-4 agents
  ├─ Track expected_agents
  ├─ Track completed_agents (via callback)
  └─ Retry up to 3 times (AR-2, AR-3)
  ↓
[CheckingConsensus] Validate agreement
  ├─ Unanimous (55%) → QualityGate
  ├─ 2/3 (35%) → QualityGateValidating (GPT-5)
  └─ Conflict (<5%) → QualityGateAwaitingHuman
  ↓
[QualityGate*] If checkpoint active:
  ├─ QualityGateExecuting (spawn agents)
  ├─ QualityGateProcessing (classify issues)
  ├─ QualityGateValidating (GPT-5 for 2/3)
  └─ QualityGateAwaitingHuman (user answers)
  ↓
[Store Results] write_consensus_synthesis()
  ↓
[Advance] Move to next stage or COMPLETE
```

---

## REFERENCE DOCUMENTS IN REPO

```
/home/thetu/code/
├── SPEC_KIT_ARCHITECTURE_COMPLETE.md   ← Main reference (866 lines)
├── SPEC.md                              ← Task tracker
├── CLAUDE.md                            ← Operator runbook
├── PLANNING.md                          ← Architecture & constraints
├── product-requirements.md              ← Product scope
├── docs/UPSTREAM-SYNC.md                ← Rebase strategy
├── docs/spec-kit/
│   ├── prompts.json                     ← 13 stage prompts (28 KB)
│   ├── COMMAND_INVENTORY.md             ← 22 commands
│   ├── COMMAND_REGISTRY_DESIGN.md       ← Dynamic pattern
│   ├── QUALITY_GATES_DESIGN.md          ← 3 checkpoints
│   ├── spec-auto-full-automation-plan.md ← Tier strategy
│   ├── evidence-policy.md               ← Retention policy
│   └── testing-policy.md                ← Coverage roadmap
└── templates/                           ← 11 markdown guides
```

