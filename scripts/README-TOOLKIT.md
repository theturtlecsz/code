# Spec-Kit Debug & Validation Toolkit

Comprehensive tooling for testing, debugging, validating, and auditing the spec-kit multi-agent workflow.

## Quick Start

```bash
# Master tool (recommended)
./scripts/spec-kit-tools.sh help

# Run complete workflow test
./scripts/spec-kit-tools.sh test SPEC-KIT-900 /speckit.plan

# Check status
./scripts/spec-kit-tools.sh status SPEC-KIT-900

# Validate deliverable quality
./scripts/spec-kit-tools.sh validate SPEC-KIT-900 plan
```

---

## Tool Categories

### 🚀 Workflow Execution

#### `test-spec-kit.sh` - Automated Workflow Test
**Purpose**: Run complete end-to-end test of a spec-kit stage

**Usage**:
```bash
bash scripts/test-spec-kit.sh <SPEC-ID> <command>

# Examples
bash scripts/test-spec-kit.sh SPEC-KIT-900 /speckit.plan
bash scripts/test-spec-kit.sh SPEC-KIT-900 /speckit.tasks
bash scripts/test-spec-kit.sh SPEC-KIT-900 /speckit.auto
```

**What it does**:
1. Builds binary with `build-fast.sh`
2. Cleans up old TUI sessions
3. Starts TUI and executes command
4. Waits for completion (up to 15 minutes)
5. Shows output and deliverable preview
6. Provides next steps

**When to use**: You want hands-off execution and automated validation

---

#### `tui-session.sh` - TUI Session Management
**Purpose**: Full control over background TUI sessions via tmux

**Usage**:
```bash
bash scripts/tui-session.sh <command> [args]

# Start session with command
bash scripts/tui-session.sh start "/speckit.plan SPEC-KIT-900"

# Send additional commands
bash scripts/tui-session.sh send "/speckit.status SPEC-KIT-900"

# View output
bash scripts/tui-session.sh logs
bash scripts/tui-session.sh capture > output.txt

# Attach interactively (Ctrl-b d to detach)
bash scripts/tui-session.sh attach

# Check status
bash scripts/tui-session.sh status

# Kill session
bash scripts/tui-session.sh kill
```

**When to use**: You need fine-grained control over long-running workflows

---

### 📊 Status & Monitoring

#### `workflow-status.sh` - Comprehensive Dashboard
**Purpose**: At-a-glance view of complete workflow state

**Usage**:
```bash
bash scripts/workflow-status.sh <SPEC-ID>

# Example
bash scripts/workflow-status.sh SPEC-KIT-900
```

**Shows**:
- ✓ Stage completion (which stages done)
- 🤖 Agent participation matrix
- 📄 Deliverable status
- 🗂️ Evidence file completeness
- 💰 Cost tracking
- ✨ Quality assessment
- ➡️ Next steps

**When to use**: You want to know "where am I?" in the workflow

---

#### `monitor-cost.sh` - Cost & Performance Tracking
**Purpose**: Track spending and performance metrics

**Usage**:
```bash
bash scripts/monitor-cost.sh <SPEC-ID>

# Example
bash scripts/monitor-cost.sh SPEC-KIT-900
```

**Shows**:
- Total cost across all stages
- Per-stage cost breakdown
- Per-agent token usage
- Performance metrics (stage latency)
- Cost efficiency vs targets
- Evidence footprint size

**When to use**: You need to track budget or diagnose performance issues

---

### 🐛 Debugging

#### `debug-consensus.sh` - Consensus Debugging
**Purpose**: Deep dive into multi-agent consensus for a specific stage

**Usage**:
```bash
bash scripts/debug-consensus.sh <SPEC-ID> <stage>

# Examples
bash scripts/debug-consensus.sh SPEC-KIT-900 spec-plan
bash scripts/debug-consensus.sh SPEC-KIT-900 spec-tasks
```

**Stages**: `spec-plan`, `spec-tasks`, `spec-implement`, `spec-validate`, `spec-audit`, `spec-unlock`

**Shows**:
- All agent artifacts (gemini, claude, gpt_pro)
- Output lengths and timestamps
- Synthesis summary
- Verdict file location and content
- Agent response previews (first 500 chars)
- Evidence file status

**When to use**: A stage produced unexpected output or you need to see what each agent said

---

### ✅ Validation

#### `validate-deliverable.sh` - Deliverable Quality Checker
**Purpose**: Programmatic validation of plan.md, tasks.md, etc.

**Usage**:
```bash
bash scripts/validate-deliverable.sh <SPEC-ID> <stage>

# Examples
bash scripts/validate-deliverable.sh SPEC-KIT-900 plan
bash scripts/validate-deliverable.sh SPEC-KIT-900 tasks
```

**Stages**: `plan`, `tasks`, `validate`, `implement`, `audit`, `unlock`

**Checks**:
1. **Existence**: File exists with substantial content
2. **Size**: Meets minimum length (2000+ bytes)
3. **Anti-patterns**: No debug logs, no raw JSON dumps
4. **Content**: Stage-specific keywords present
5. **SPEC-KIT-900 specific**: References "reminder", "microservice" (not meta-analysis)

**Exit codes**:
- 0: ACCEPTABLE (warnings OK)
- 1: MARGINAL (1-2 failures, needs review)
- 2: UNACCEPTABLE (3+ failures)

**When to use**: Verify deliverable quality, validate prompt fixes worked

---

#### `audit-evidence.sh` - Evidence Completeness Audit
**Purpose**: Verify all evidence files and database records are properly stored

**Usage**:
```bash
bash scripts/audit-evidence.sh <SPEC-ID> [run-id]

# Examples
bash scripts/audit-evidence.sh SPEC-KIT-900
bash scripts/audit-evidence.sh SPEC-KIT-900 abc123
```

**Checks**:
1. **Consensus files**: 12 files expected (2 per stage × 6 stages)
2. **Database integrity**: Artifacts and synthesis records present
3. **Agent participation**: All expected agents responded
4. **run_id propagation**: Consistent tracking across tables
5. **Evidence footprint**: Within 25 MB soft limit
6. **Schema validation**: JSON structure correct
7. **Deliverable files**: All stage outputs present

**When to use**: Verify evidence export worked correctly, check for missing data

---

### 🔄 Comparison

#### `compare-runs.sh` - Before/After Comparison
**Purpose**: Compare two runs of the same SPEC (e.g., before/after prompt fix)

**Usage**:
```bash
bash scripts/compare-runs.sh <SPEC-ID> <run1-id> <run2-id> [stage]

# Example (comparing prompt fix impact)
bash scripts/compare-runs.sh SPEC-KIT-900 before-fix after-fix plan
```

**Shows**:
- Agent participation changes
- Content length differences
- Keyword analysis (workload vs meta-analysis)
- Synthesis output diff
- Quality comparison

**When to use**: Validate prompt fixes, benchmark model routing changes (SPEC-KIT-070)

---

## Common Workflows

### Testing Prompt Fix

```bash
# 1. Run the workflow
bash scripts/test-spec-kit.sh SPEC-KIT-900 /speckit.plan

# 2. Check status
bash scripts/workflow-status.sh SPEC-KIT-900

# 3. Validate deliverable
bash scripts/validate-deliverable.sh SPEC-KIT-900 plan

# Expected: PASS with "reminder" and "microservice" keywords
# Bad: FAIL with debug logs or meta-analysis
```

---

### Debugging Failed Stage

```bash
# 1. Check overall status
bash scripts/workflow-status.sh SPEC-KIT-900

# 2. Debug the failing stage
bash scripts/debug-consensus.sh SPEC-KIT-900 spec-tasks

# 3. Check agent outputs individually
sqlite3 ~/.code/consensus_artifacts.db \
  "SELECT agent_id, substr(content, 1, 1000)
   FROM consensus_artifacts
   WHERE spec_id='SPEC-KIT-900' AND stage='spec-tasks';"

# 4. Validate evidence is properly stored
bash scripts/audit-evidence.sh SPEC-KIT-900
```

---

### Cost Analysis

```bash
# 1. Monitor cost during/after run
bash scripts/monitor-cost.sh SPEC-KIT-900

# 2. Compare to targets
# Full pipeline target: $2.70 (from SPEC-KIT-070)
# Your run: check output

# 3. Check evidence footprint
# Soft limit: 25 MB per SPEC
# Your footprint: check output
```

---

### Continuous Monitoring

```bash
# Watch workflow progress in real-time
while true; do
    clear
    bash scripts/workflow-status.sh SPEC-KIT-900
    sleep 30
done

# Or use the TUI session to monitor live
bash scripts/tui-session.sh start "/speckit.auto SPEC-KIT-900"
bash scripts/tui-session.sh attach  # Watch it run
```

---

## Master Tool: spec-kit-tools.sh

**Unified interface** to all toolkit functions:

```bash
./scripts/spec-kit-tools.sh <command> [args]

Commands:
  test <SPEC> <cmd>           Run workflow test
  session <action> [args]     Manage TUI session
  status <SPEC>               Show dashboard
  debug <SPEC> <stage>        Debug consensus
  validate <SPEC> <stage>     Validate deliverable
  monitor <SPEC>              Monitor cost
  audit <SPEC>                Audit evidence
  compare <SPEC> <r1> <r2>    Compare runs
  agents <SPEC> <stage>       Show agent outputs
```

---

## Dependencies

**Required**:
- `sqlite3` - Database queries (install: `apt-get install sqlite3`)
- `tmux` - Session management (install: `apt-get install tmux`)
- `bash` 4.0+ - Shell scripting

**Optional (enhances features)**:
- `jq` - JSON parsing for cost/evidence (install: `apt-get install jq`)
- `bc` - Float math for cost comparison (usually pre-installed)
- `diff` - Run comparison diffs (usually pre-installed)

---

## File Locations

### Generated Evidence
```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
├── consensus/
│   └── <SPEC-ID>/
│       ├── plan_synthesis.json
│       ├── plan_verdict.json
│       ├── tasks_synthesis.json
│       ├── tasks_verdict.json
│       └── ... (12 files total)
└── costs/
    └── <SPEC-ID>_cost_summary.json
```

### Deliverables
```
docs/<SPEC-ID>-generic-smoke/
├── spec.md
├── PRD.md
├── plan.md
├── tasks.md
├── implement.md
├── validate.md
├── audit.md
└── unlock.md
```

### Database
```
~/.code/consensus_artifacts.db
  - consensus_artifacts table (agent outputs)
  - consensus_synthesis table (merged outputs)
```

---

## Troubleshooting

### "Database not found"
```bash
# Check if database exists
ls -lh ~/.code/consensus_artifacts.db

# If missing, run at least one stage to create it
bash scripts/test-spec-kit.sh SPEC-KIT-900 /speckit.plan
```

### "Session already running"
```bash
# Kill existing session
bash scripts/tui-session.sh kill

# Or attach to it
bash scripts/tui-session.sh attach
```

### "Binary not found"
```bash
# Build the binary
bash scripts/build-fast.sh

# Verify it exists
ls -lh codex-rs/target/release/code
```

### "Validation fails with 'Missing reminder'"
**This means the prompt fix didn't work!** The agents are still doing meta-analysis.

```bash
# Check the actual content
cat docs/SPEC-KIT-900-generic-smoke/plan.md | head -100

# Debug what agents said
bash scripts/debug-consensus.sh SPEC-KIT-900 spec-plan

# Verify prompts.json was updated
grep "workload described in SPEC" docs/spec-kit/prompts.json
```

---

## Contributing

When adding new debug/validation tools:

1. **Make it scriptable**: Return exit codes (0=success, 1=warn, 2=fail)
2. **Add to spec-kit-tools.sh**: Register in master tool
3. **Document here**: Add usage examples
4. **Make executable**: `chmod +x scripts/your-script.sh`

---

## Related Documents

- `CLAUDE.md` - Main project documentation
- `docs/spec-kit/` - Spec-kit automation docs
- `SPEC.md` - Task tracking
- `docs/SPEC-KIT-900-generic-smoke/` - Smoke test SPEC
