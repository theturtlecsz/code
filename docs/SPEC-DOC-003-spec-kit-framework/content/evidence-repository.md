# Evidence Repository

Comprehensive guide to artifact storage and telemetry collection.

---

## Overview

The **Evidence Repository** captures auditable logs and artifacts from all Spec-Kit operations:

- **Telemetry**: Execution metadata (cost, duration, status)
- **Agent outputs**: Raw responses from each agent
- **Consensus artifacts**: Synthesized results
- **Quality gate results**: Checkpoint outcomes
- **Guardrail logs**: Validation results

**Location**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

**Purpose**:
- **Audit trail**: Complete history of automation decisions
- **Debugging**: Investigate pipeline failures
- **Cost tracking**: Per-stage cost breakdown
- **Quality validation**: Evidence of quality gate compliance
- **Reproducibility**: Re-run consensus from cached artifacts

**Retention**: 25 MB soft limit per SPEC (monitored via `/spec-evidence-stats`)

---

## Directory Structure

### Top-Level Layout

```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
├── .locks/                     # Lockfiles for concurrent access
├── archive/                    # Archived old evidence (>30 days)
├── commands/                   # Per-SPEC command execution logs
│   ├── SPEC-KIT-001/
│   ├── SPEC-KIT-002/
│   └── SPEC-KIT-070/          # Example SPEC
├── consensus/                  # MCP consensus artifacts
│   ├── runs/                  # Consensus run metadata
│   └── agents/                # Agent response cache
└── quality_gates/              # Quality gate checkpoint results
```

---

### Per-SPEC Structure

**Example**: `evidence/commands/SPEC-KIT-070/`

```
SPEC-KIT-070/
├── plan/
│   ├── plan_execution.json       # Guardrail telemetry (10 KB)
│   ├── agent_1_gemini-flash.txt  # Agent output (15 KB)
│   ├── agent_2_claude-haiku.txt  # Agent output (15 KB)
│   ├── agent_3_gpt5-medium.txt   # Agent output (15 KB)
│   ├── consensus.json            # MCP synthesis (5 KB)
│   └── baseline_check.log        # Guardrail validation (2 KB)
├── tasks/
│   ├── tasks_execution.json      # Guardrail telemetry (8 KB)
│   ├── agent_1_gpt5-low.txt      # Agent output (10 KB)
│   ├── consensus.json            # MCP synthesis (3 KB)
│   └── tool_check.log            # Guardrail validation (1 KB)
├── implement/
│   ├── implement_execution.json  # Guardrail telemetry (12 KB)
│   ├── agent_1_gpt_codex.txt     # Code specialist (20 KB)
│   ├── agent_2_claude-haiku.txt  # Validator (8 KB)
│   ├── consensus.json            # MCP synthesis (4 KB)
│   ├── cargo_fmt.log             # Code formatting (2 KB)
│   ├── cargo_clippy.log          # Linting (5 KB)
│   └── build_check.log           # Build validation (3 KB)
├── validate/
│   ├── validate_execution.json   # Guardrail telemetry (12 KB)
│   ├── payload_hash_abc123.json  # Deduplication record (2 KB)
│   ├── agent_1_gemini-flash.txt  # Agent output (15 KB)
│   ├── agent_2_claude-haiku.txt  # Agent output (15 KB)
│   ├── agent_3_gpt5-medium.txt   # Agent output (15 KB)
│   ├── consensus.json            # MCP synthesis (5 KB)
│   └── lifecycle_state.json      # Attempt tracking (1 KB)
├── audit/
│   ├── audit_execution.json      # Guardrail telemetry (12 KB)
│   ├── agent_1_gemini-pro.txt    # Premium agent (18 KB)
│   ├── agent_2_claude-sonnet.txt # Premium agent (18 KB)
│   ├── agent_3_gpt5-high.txt     # Premium agent (18 KB)
│   ├── consensus.json            # MCP synthesis (6 KB)
│   └── compliance_checks.json    # OWASP, dependencies (8 KB)
├── unlock/
│   ├── unlock_execution.json     # Guardrail telemetry (10 KB)
│   ├── agent_1_gemini-pro.txt    # Premium agent (18 KB)
│   ├── agent_2_claude-sonnet.txt # Premium agent (18 KB)
│   ├── agent_3_gpt5-high.txt     # Premium agent (18 KB)
│   ├── consensus.json            # MCP synthesis (6 KB)
│   └── ship_decision.json        # Final verdict (3 KB)
└── quality_gates/
    ├── BeforeSpecify_clarify.json   # Clarify gate (5 KB)
    ├── AfterSpecify_checklist.json  # Checklist gate (8 KB)
    ├── AfterTasks_analyze.json      # Analyze gate (6 KB)
    ├── gpt5_validations/            # GPT-5 validation logs
    │   ├── issue_001_validation.json
    │   └── issue_002_validation.json
    ├── user_escalations/             # User decision logs
    │   ├── issue_003_question.json
    │   └── issue_003_answer.json
    └── completed_checkpoints.json    # Memoization tracking (1 KB)
```

**Total**: ~350 KB per SPEC (full 6-stage pipeline with quality gates)

---

## Telemetry Schema

### Schema Version 1.0

All telemetry files follow this base schema:

```json
{
  "command": "plan",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T14:32:00Z",
  "schemaVersion": "1.0",
  "artifacts": ["docs/SPEC-KIT-070-dark-mode/plan.md"],
  "exit_code": 0
}
```

**Required Fields** (all stages):
- `command`: Stage name ("plan", "tasks", "implement", "validate", "audit", "unlock")
- `specId`: SPEC-ID ("SPEC-KIT-070")
- `sessionId`: Unique session identifier (UUID)
- `timestamp`: ISO 8601 timestamp
- `schemaVersion`: "1.0"
- `artifacts`: Array of created files
- `exit_code`: 0 (success) or non-zero (failure)

---

### Stage-Specific Schemas

#### Plan Stage

```json
{
  // Base schema
  "command": "plan",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T14:32:00Z",
  "schemaVersion": "1.0",

  // Plan-specific fields
  "baseline": {
    "mode": "file",                // "file" or "stdin"
    "artifact": "docs/SPEC-KIT-070-dark-mode/spec.md",
    "status": "exists"             // "exists" or "missing"
  },

  "hooks": {
    "session": {
      "start": "passed"           // "passed" or "failed"
    }
  },

  "agents": [
    {
      "name": "gemini-flash",
      "model": "gemini-1.5-flash-latest",
      "cost": 0.12,
      "input_tokens": 5000,
      "output_tokens": 1500,
      "duration_ms": 8500,
      "status": "success"
    },
    {
      "name": "claude-haiku",
      "model": "claude-3-5-haiku-20241022",
      "cost": 0.11,
      "input_tokens": 6000,
      "output_tokens": 2000,
      "duration_ms": 9200,
      "status": "success"
    },
    {
      "name": "gpt5-medium",
      "model": "gpt-5-medium",
      "cost": 0.12,
      "input_tokens": 7000,
      "output_tokens": 2500,
      "duration_ms": 10500,
      "status": "success"
    }
  ],

  "consensus": {
    "status": "ok",                // "ok", "degraded", "conflict", "unknown"
    "present_agents": ["gemini-flash", "claude-haiku", "gpt5-medium"],
    "missing_agents": [],
    "conflicts": [],
    "mcp_calls": 1,
    "mcp_duration_ms": 8.7
  },

  "artifacts": ["docs/SPEC-KIT-070-dark-mode/plan.md"],

  "total_cost": 0.40,              // Agents ($0.35) + MCP validation ($0.05)
  "total_duration_ms": 11200,

  "exit_code": 0
}
```

---

#### Tasks Stage

```json
{
  // Base schema
  "command": "tasks",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T14:45:00Z",
  "schemaVersion": "1.0",

  // Tasks-specific fields
  "tool": {
    "status": "success",          // "success" or "failure"
    "tool_name": "gpt5-low"
  },

  "agents": [
    {
      "name": "gpt5-low",
      "model": "gpt-5-low",
      "cost": 0.10,
      "input_tokens": 4000,
      "output_tokens": 1200,
      "duration_ms": 3500,
      "status": "success"
    }
  ],

  "artifacts": ["docs/SPEC-KIT-070-dark-mode/tasks.md", "SPEC.md"],

  "total_cost": 0.10,
  "total_duration_ms": 3500,

  "exit_code": 0
}
```

---

#### Implement Stage

```json
{
  // Base schema
  "command": "implement",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T14:50:00Z",
  "schemaVersion": "1.0",

  // Implement-specific fields
  "lock_status": {
    "git_clean": true,            // Git tree clean?
    "conflicts": []
  },

  "hook_status": {
    "pre_commit": "passed",      // "passed" or "failed"
    "post_commit": "passed"
  },

  "agents": [
    {
      "name": "gpt_codex",
      "model": "gpt-5-codex-high",
      "cost": 0.08,
      "input_tokens": 8000,
      "output_tokens": 3000,
      "duration_ms": 12000,
      "status": "success",
      "specialization": "code"
    },
    {
      "name": "claude-haiku",
      "model": "claude-3-5-haiku-20241022",
      "cost": 0.03,
      "input_tokens": 10000,
      "output_tokens": 1000,
      "duration_ms": 4000,
      "status": "success",
      "specialization": "validator"
    }
  ],

  "validations": {
    "cargo_fmt": {
      "status": "passed",
      "duration_ms": 450
    },
    "cargo_clippy": {
      "status": "passed",
      "warnings": 0,
      "duration_ms": 3200
    },
    "build_check": {
      "status": "passed",
      "duration_ms": 8500
    }
  },

  "artifacts": [
    "codex-rs/tui/src/ui/dark_mode.rs",
    "codex-rs/tui/src/ui/mod.rs",
    "docs/SPEC-KIT-070-dark-mode/implementation_notes.md"
  ],

  "total_cost": 0.11,
  "total_duration_ms": 27700,    // 12s agents + 12s validations + 3.7s overhead

  "exit_code": 0
}
```

---

#### Validate Stage

```json
{
  // Base schema
  "command": "validate",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T15:00:00Z",
  "schemaVersion": "1.0",

  // Validate-specific fields
  "lifecycle": {
    "payload_hash": "abc123def456",
    "attempt_number": 1,
    "outcome": "fresh"           // "fresh", "duplicate", "retry"
  },

  "scenarios": [
    {
      "name": "Dark mode toggle renders correctly",
      "status": "passed"
    },
    {
      "name": "Theme persists across sessions",
      "status": "passed"
    },
    {
      "name": "Accessibility contrast ratios meet WCAG AA",
      "status": "passed"
    }
  ],

  "agents": [
    {
      "name": "gemini-flash",
      "model": "gemini-1.5-flash-latest",
      "cost": 0.12,
      "input_tokens": 6000,
      "output_tokens": 1800,
      "duration_ms": 9000,
      "status": "success"
    },
    {
      "name": "claude-haiku",
      "model": "claude-3-5-haiku-20241022",
      "cost": 0.11,
      "input_tokens": 6500,
      "output_tokens": 2000,
      "duration_ms": 9500,
      "status": "success"
    },
    {
      "name": "gpt5-medium",
      "model": "gpt-5-medium",
      "cost": 0.12,
      "input_tokens": 7000,
      "output_tokens": 2200,
      "duration_ms": 10000,
      "status": "success"
    }
  ],

  "artifacts": ["docs/SPEC-KIT-070-dark-mode/test_plan.md"],

  "total_cost": 0.40,
  "total_duration_ms": 11000,

  "exit_code": 0
}
```

---

#### Audit Stage

```json
{
  // Base schema
  "command": "audit",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T15:12:00Z",
  "schemaVersion": "1.0",

  // Audit-specific fields
  "scenarios": [
    {
      "name": "OWASP Top 10 compliance",
      "status": "passed",
      "checks": [
        {"id": "A01", "name": "Broken Access Control", "status": "passed"},
        {"id": "A02", "name": "Cryptographic Failures", "status": "passed"},
        {"id": "A03", "name": "Injection", "status": "passed"}
      ]
    },
    {
      "name": "Dependency vulnerabilities",
      "status": "passed",
      "vulnerabilities_found": 0
    },
    {
      "name": "License compliance",
      "status": "passed",
      "incompatible_licenses": []
    }
  ],

  "agents": [
    {
      "name": "gemini-pro",
      "model": "gemini-1.5-pro-latest",
      "cost": 0.28,
      "input_tokens": 8000,
      "output_tokens": 2500,
      "duration_ms": 11000,
      "status": "success"
    },
    {
      "name": "claude-sonnet",
      "model": "claude-3-5-sonnet-20241022",
      "cost": 0.30,
      "input_tokens": 8500,
      "output_tokens": 2800,
      "duration_ms": 11500,
      "status": "success"
    },
    {
      "name": "gpt5-high",
      "model": "gpt-5-high",
      "cost": 0.27,
      "input_tokens": 9000,
      "output_tokens": 2600,
      "duration_ms": 12000,
      "status": "success"
    }
  ],

  "artifacts": ["docs/SPEC-KIT-070-dark-mode/audit_report.md"],

  "total_cost": 0.85,
  "total_duration_ms": 12000,

  "exit_code": 0
}
```

---

#### Unlock Stage

```json
{
  // Base schema
  "command": "unlock",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T15:25:00Z",
  "schemaVersion": "1.0",

  // Unlock-specific fields
  "unlock_status": {
    "decision": "approved",      // "approved" or "rejected"
    "blockers": [],
    "consensus": true            // 2/3+ agents agree?
  },

  "agents": [
    {
      "name": "gemini-pro",
      "model": "gemini-1.5-pro-latest",
      "cost": 0.28,
      "input_tokens": 10000,
      "output_tokens": 3000,
      "duration_ms": 12000,
      "status": "success",
      "decision": "approved"
    },
    {
      "name": "claude-sonnet",
      "model": "claude-3-5-sonnet-20241022",
      "cost": 0.30,
      "input_tokens": 10500,
      "output_tokens": 3200,
      "duration_ms": 12500,
      "status": "success",
      "decision": "approved"
    },
    {
      "name": "gpt5-high",
      "model": "gpt-5-high",
      "cost": 0.27,
      "input_tokens": 11000,
      "output_tokens": 2900,
      "duration_ms": 13000,
      "status": "success",
      "decision": "approved"
    }
  ],

  "artifacts": ["docs/SPEC-KIT-070-dark-mode/unlock_approval.md"],

  "total_cost": 0.85,
  "total_duration_ms": 13000,

  "exit_code": 0
}
```

---

## Agent Output Files

### Format

**Filename**: `agent_{index}_{name}.txt`

**Example**: `agent_1_gemini-flash.txt`

```
=== Agent Execution ===
Name: gemini-flash
Model: gemini-1.5-flash-latest
Stage: plan
Spec: SPEC-KIT-070
Session: abc123
Timestamp: 2025-10-18T14:32:15Z

=== Prompt ===
You are tasked with creating a work breakdown plan for the following specification:

[PRD content here...]

Please provide a detailed plan including:
1. Work Breakdown Structure
2. Acceptance Mapping
3. Risks & Unknowns
4. Exit Criteria

=== Response ===
# Plan: Dark Mode Toggle

## Inputs
- Spec: docs/SPEC-KIT-070-dark-mode/spec.md (v1, hash: abc123)
- Constitution: memory/constitution.md (v2, hash: def456)

## Work Breakdown

### Phase 1: UI Components (3 days)
1.1 Create toggle component
1.2 Add theme context provider
1.3 Update existing components for theme support

### Phase 2: State Management (2 days)
2.1 Implement theme persistence (localStorage)
2.2 Add system preference detection
2.3 Create theme switching logic

### Phase 3: Styling (2 days)
3.1 Define dark mode color palette
3.2 Update CSS-in-JS styles
3.3 Test contrast ratios (WCAG AA)

## Acceptance Mapping

| Requirement (Spec) | Validation Step | Test/Check Artifact |
| --- | --- | --- |
| FR-001: Toggle UI | Manual inspection | Screenshot + accessibility audit |
| FR-002: Theme persistence | Automated test | test_theme_persistence.rs |
| NFR-001: <200ms switch | Performance benchmark | benchmark_theme_switch.rs |

## Risks & Unknowns

- **Risk**: Existing components may hardcode light theme colors
  - Mitigation: Audit all components, refactor to use theme context

- **Unknown**: System preference detection browser support
  - Research: Check MDN for prefers-color-scheme support

## Exit Criteria (Done)

- [ ] All acceptance checks pass
- [ ] WCAG AA contrast ratios met
- [ ] Theme preference persists across sessions
- [ ] <200ms switching latency (p95)
- [ ] PR approved and merged

=== Metadata ===
Input tokens: 5000
Output tokens: 1500
Cost: $0.12
Duration: 8500ms
Status: success
```

---

## Consensus Artifacts

### Consensus JSON

**Location**: `{stage}/consensus.json`

```json
{
  "spec_id": "SPEC-KIT-070",
  "stage": "plan",
  "run_id": "run-abc123",
  "timestamp": "2025-10-18T14:35:00Z",

  "inputs": {
    "agent_count": 3,
    "agents": ["gemini-flash", "claude-haiku", "gpt5-medium"],
    "artifacts": [
      "docs/SPEC-KIT-070-dark-mode/spec.md",
      "memory/constitution.md"
    ]
  },

  "synthesis": {
    "method": "mcp_local_memory",
    "mcp_duration_ms": 8.7,
    "prompt_tokens": 15000,
    "completion_tokens": 3000
  },

  "verdict": {
    "status": "ok",
    "present_agents": ["gemini-flash", "claude-haiku", "gpt5-medium"],
    "missing_agents": [],
    "degraded": false,
    "conflicts": []
  },

  "synthesized_output": "# Plan: Dark Mode Toggle\n\n## Consensus Summary\n\nAll three agents (gemini-flash, claude-haiku, gpt5-medium) agree on a phased approach:\n\n**Phase 1: UI Components** (gemini suggests 3 days, claude 2 days, gpt5 3 days → consensus: 3 days)\n- Toggle component\n- Theme context provider\n- Component updates\n\n**Phase 2: State Management** (unanimous 2 days)\n- Persistence (localStorage)\n- System preference detection\n- Switching logic\n\n**Phase 3: Styling** (unanimous 2 days)\n- Color palette definition\n- CSS-in-JS updates\n- WCAG AA compliance testing\n\n**Key Insights**:\n- gemini emphasized accessibility testing (WCAG AA)\n- claude highlighted system preference detection\n- gpt5 focused on performance (<200ms switching)\n\n**Synthesis**: Combined all perspectives into unified plan with acceptance mapping, risks, and exit criteria.\n\n...[full synthesized plan content]...",

  "cost": 0.40,
  "duration_ms": 11200
}
```

---

## Quality Gate Evidence

### Checkpoint Result

**Location**: `quality_gates/{checkpoint}_{gate_type}.json`

**Example**: `quality_gates/AfterSpecify_checklist.json`

```json
{
  "checkpoint": "AfterSpecify",
  "spec_id": "SPEC-KIT-070",
  "gate_type": "checklist",
  "timestamp": "2025-10-18T14:40:00Z",

  "native_result": {
    "overall_score": 82.0,
    "grade": "B",
    "category_scores": {
      "completeness": 90.0,
      "clarity": 65.0,
      "testability": 85.0,
      "consistency": 80.0
    },
    "issues": [
      {
        "id": "CHK-001",
        "category": "clarity",
        "severity": "IMPORTANT",
        "description": "3 quantifiers without metrics",
        "impact": "-15.0 points",
        "suggestion": "Add specific metrics to 'fast', 'scalable', etc."
      },
      {
        "id": "CHK-002",
        "category": "testability",
        "severity": "IMPORTANT",
        "description": "Acceptance criteria covers 3 of 4 requirements (75%)",
        "impact": "-7.5 points",
        "suggestion": "Add acceptance criteria for all requirements"
      }
    ]
  },

  "gpt5_validations": [
    {
      "issue_id": "CHK-001",
      "majority_answer": "Add '<200ms response time (p95)' after 'fast'",
      "gpt5_verdict": {
        "agrees_with_majority": true,
        "reasoning": "Specific metric aligns with spec intent, measurable, industry-standard",
        "recommended_answer": "<200ms response time (p95)",
        "confidence": "high"
      },
      "resolution": "auto_applied"
    }
  ],

  "user_escalations": [
    {
      "issue_id": "CHK-002",
      "question": "FR-004 has no acceptance criteria. What should we test?",
      "user_answer": "Test: (1) Theme persists after browser restart, (2) System preference detection works, (3) Manual toggle overrides system preference",
      "resolution": "applied"
    }
  ],

  "outcome": {
    "status": "passed",
    "initial_score": 82.0,
    "final_score": 95.0,
    "grade_change": "B → A",
    "auto_resolved": 1,
    "gpt5_validated": 1,
    "user_escalated": 1
  },

  "modified_files": [
    "docs/SPEC-KIT-070-dark-mode/spec.md",
    "docs/SPEC-KIT-070-dark-mode/plan.md"
  ],

  "cost": 0.05,
  "duration_ms": 1200
}
```

---

## Evidence Stats & Monitoring

### /spec-evidence-stats Command

**Purpose**: Monitor evidence footprint, ensure <25 MB per SPEC

**Location**: `scripts/spec_ops_004/evidence_stats.sh`

**Usage**:

```bash
# All SPECs
/spec-evidence-stats

# Specific SPEC
/spec-evidence-stats --spec SPEC-KIT-070
```

**Output**:

```
Evidence Footprint Report
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Global Stats:
  Total SPECs: 12
  Total Size: 3.8 MB
  Largest SPEC: SPEC-KIT-070 (580 KB)
  Average per SPEC: 316 KB

Per-SPEC Breakdown:
┌─────────────┬──────────┬────────┬───────────┬────────────┐
│ SPEC-ID     │ Size     │ Files  │ Stages    │ Status     │
├─────────────┼──────────┼────────┼───────────┼────────────┤
│ SPEC-KIT-001│ 150 KB   │ 18     │ 3/6       │ ✅ OK      │
│ SPEC-KIT-002│ 320 KB   │ 45     │ 6/6       │ ✅ OK      │
│ SPEC-KIT-070│ 580 KB   │ 78     │ 6/6       │ ✅ OK      │
│ ...         │ ...      │ ...    │ ...       │ ...        │
└─────────────┴──────────┴────────┴───────────┴────────────┘

SPEC-KIT-070 Detail:
  Total: 580 KB (2.3% of 25 MB limit)
  Breakdown:
    plan/           120 KB (62 files: telemetry + 3 agents + consensus)
    tasks/           45 KB (18 files: telemetry + 1 agent + consensus)
    implement/      110 KB (85 files: telemetry + 2 agents + validation logs)
    validate/       135 KB (68 files: telemetry + 3 agents + lifecycle + scenarios)
    audit/           95 KB (52 files: telemetry + 3 agents + compliance checks)
    unlock/          50 KB (38 files: telemetry + 3 agents + ship decision)
    quality_gates/   25 KB (15 files: 3 checkpoints + validations + escalations)

Recommendations:
  ✅ All SPECs within 25 MB soft limit
  ✅ No archival needed
```

---

### Evidence Retention Policy

**Soft Limit**: 25 MB per SPEC

**Actions When Approaching Limit**:

1. **20-25 MB**: Warning, consider archival
2. **>25 MB**: Automatic archival of old evidence (>30 days)
3. **>50 MB**: Manual intervention required

**Archival Strategy**:

```bash
# Move old evidence to archive/
mv evidence/commands/SPEC-KIT-070/ evidence/archive/SPEC-KIT-070-2025-10-18/

# Compress archive
tar -czf evidence/archive/SPEC-KIT-070-2025-10-18.tar.gz evidence/archive/SPEC-KIT-070-2025-10-18/
rm -rf evidence/archive/SPEC-KIT-070-2025-10-18/

# Keep only compressed archives >30 days old
```

**What to Archive**:
- ✅ Agent output text files (largest contributors)
- ✅ Verbose guardrail logs
- ❌ Telemetry JSON (small, frequently referenced)
- ❌ Consensus JSON (critical for reproduction)

---

## Evidence Queries

### Find All Consensus Runs for SPEC

```bash
find evidence/commands/SPEC-KIT-070/ -name "consensus.json"
```

**Output**:
```
evidence/commands/SPEC-KIT-070/plan/consensus.json
evidence/commands/SPEC-KIT-070/tasks/consensus.json
evidence/commands/SPEC-KIT-070/implement/consensus.json
evidence/commands/SPEC-KIT-070/validate/consensus.json
evidence/commands/SPEC-KIT-070/audit/consensus.json
evidence/commands/SPEC-KIT-070/unlock/consensus.json
```

---

### Extract Total Cost for SPEC

```bash
# Sum all stage costs
jq -s 'map(.total_cost) | add' evidence/commands/SPEC-KIT-070/*/execution.json
```

**Output**: `2.71` (total cost for full pipeline)

---

### Find Failed Stages

```bash
# Find all non-zero exit codes
grep -r '"exit_code": [^0]' evidence/commands/SPEC-KIT-070/
```

---

### List Quality Gate Results

```bash
ls -lh evidence/commands/SPEC-KIT-070/quality_gates/
```

**Output**:
```
BeforeSpecify_clarify.json      (5 KB)
AfterSpecify_checklist.json     (8 KB)
AfterTasks_analyze.json         (6 KB)
completed_checkpoints.json      (1 KB)
gpt5_validations/               (dir)
user_escalations/               (dir)
```

---

## Best Practices

### Evidence Organization

**DO**:
- ✅ Use consistent naming (`{stage}_execution.json`)
- ✅ Include schemaVersion for all JSON files
- ✅ Compress agent outputs >100 KB
- ✅ Archive evidence >30 days old
- ✅ Monitor footprint with `/spec-evidence-stats`

**DON'T**:
- ❌ Store sensitive data (credentials, API keys)
- ❌ Duplicate artifacts across stages
- ❌ Omit timestamps or session IDs
- ❌ Mix schema versions in same SPEC

---

### Evidence Hygiene

**Weekly**:
- Run `/spec-evidence-stats` to check footprint
- Archive completed SPECs >30 days old

**Monthly**:
- Review archived evidence, delete >90 days
- Compress large agent output files

**Per-SPEC**:
- Keep evidence until SPEC is merged or abandoned
- Archive before deleting SPEC directory

---

## Troubleshooting

### Missing Telemetry

**Problem**: `{stage}_execution.json` missing

**Causes**:
- Guardrail script failed before telemetry write
- Disk full
- Permissions issue

**Solution**:
1. Check guardrail logs: `logs/guardrail_{stage}.log`
2. Re-run stage: `/speckit.{stage} SPEC-ID`
3. Verify disk space: `df -h`

---

### Schema Validation Failures

**Problem**: `/speckit.auto` halts with "Invalid telemetry schema"

**Causes**:
- Missing required field (`command`, `specId`, `exit_code`)
- Wrong schema version
- Malformed JSON

**Solution**:
1. Validate JSON: `jq . evidence/commands/SPEC-ID/{stage}/execution.json`
2. Check schema version: `jq .schemaVersion evidence/...`
3. Fix or regenerate telemetry

---

### Evidence Footprint Exceeded

**Problem**: SPEC >25 MB soft limit

**Causes**:
- Large agent outputs (>50 KB each)
- Many quality gate iterations
- Verbose guardrail logs

**Solution**:
1. Run `/spec-evidence-stats --spec SPEC-ID` to identify largest contributors
2. Compress or archive agent outputs: `gzip evidence/commands/SPEC-ID/*/agent_*.txt`
3. Archive old quality gate iterations
4. Offload to external storage if >50 MB

---

## Summary

**Evidence Repository Highlights**:

1. **Complete Audit Trail**: Telemetry, agent outputs, consensus artifacts, quality gates
2. **Telemetry Schema v1.0**: Consistent JSON structure across all stages
3. **Per-SPEC Organization**: Evidence organized by SPEC-ID → stage → files
4. **25 MB Soft Limit**: Monitored via `/spec-evidence-stats`, archival for old evidence
5. **Reproducibility**: Consensus can be re-run from cached agent outputs
6. **Cost Tracking**: Total cost extractable from telemetry files
7. **Quality Validation**: Evidence of quality gate compliance

**Next Steps**:
- [Cost Tracking](cost-tracking.md) - Per-stage cost breakdown and analysis
- [Agent Orchestration](agent-orchestration.md) - Multi-agent coordination
- [Workflow Patterns](workflow-patterns.md) - Common usage scenarios

---

**File References**:
- Evidence root: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`
- Evidence stats: `scripts/spec_ops_004/evidence_stats.sh`
- Telemetry schema: (in guardrail scripts, standard v1.0)
