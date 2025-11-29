# NotebookLM Prompts for Claude Code Integration
## Research-to-Implementation Export Prompts

This document contains a series of prompts designed to extract actionable information from NotebookLM research into formats that Claude Code can immediately implement.

**Usage Instructions:**
1. Run these prompts sequentially in NotebookLM (recommended order provided)
2. Copy each NotebookLM response into a separate file
3. Provide the responses to Claude Code for implementation
4. Each prompt produces copy-paste-ready outputs in formats Claude Code expects

**Recommended Sequence:**
1. Prompt 6 (MVP) - Get the quickest win defined
2. Prompt 4 (Data Formats) - Nail down the interchange format
3. Prompt 3 (Implementation Tasks) - Get concrete tasklist
4. Prompt 1 (Architecture) - Full picture for SPEC doc
5. Prompt 2 (Use Cases) - Validate against real scenarios
6. Prompt 5 (Decision Framework) - Update CLAUDE.md
7. Prompt 7 (Risks) - Complete the picture

---

## Prompt Set 1: Export Integration Architecture

```
Based on your research into integrating NotebookLM with Claude Code's PPP framework, please create a structured specification document following this format:

# Integration Architecture: NotebookLM ↔ Claude Code

## Overview
[2-3 sentence summary of the integration approach]

## Integration Points
For each integration point:
- **Stage**: [Which PPP stage: specify/plan/tasks/implement/validate/audit/unlock]
- **Trigger**: [When/how NotebookLM gets involved]
- **Input**: [What NotebookLM receives from Claude Code]
- **Process**: [What NotebookLM does]
- **Output Format**: [Exact format Claude Code expects]
- **Handoff Method**: [How the output transfers]

## Data Flow Diagrams
[Text-based flowcharts using ASCII or markdown]

## File Structure
List specific files that need creation/modification:
- Path: `docs/SPEC-XXX-notebooklm-integration/`
- Path: `codex-rs/crates/codex-tui/src/`
- [etc.]

## Dependencies
- New tools/libraries needed
- MCP servers required
- Configuration changes

## Implementation Phases
Phase 1: [Minimal viable integration]
Phase 2: [Enhanced capabilities]
Phase 3: [Full automation]

Format this as a markdown document ready to save as a SPEC file.
```

**Expected Output:** Save as `integration-architecture.md`

---

## Prompt Set 2: Export Concrete Use Cases

```
From your integration research, extract 3-5 HIGH-VALUE use cases where NotebookLM + Claude Code collaboration would provide immediate benefits.

For each use case, provide:

## Use Case: [Name]

### Business Value
- Problem solved: [specific pain point]
- Time saved: [estimate]
- Quality improvement: [measurable outcome]

### Workflow Steps
1. **User Action**: [What triggers this workflow]
2. **Claude Code**: [Automated step with specific /speckit.* command]
3. **→ NotebookLM Handoff**: [What data/context transfers]
4. **NotebookLM Processing**: [Specific research/analysis task]
5. **← Claude Code Handoff**: [What format/data returns]
6. **Claude Code**: [How it consumes the output]
7. **Result**: [Final deliverable to user]

### Example Scenario
[Concrete example with real SPEC-ID, feature name, actual file paths]

### Input/Output Samples
**Claude Code exports to NotebookLM:**
```json
{
  "specId": "SPEC-KIT-XXX",
  "context": "...",
  "question": "..."
}
```

**NotebookLM returns to Claude Code:**
```markdown
[exact format Claude Code expects]
```

### Success Metrics
- How to measure if this use case is working

Prioritize use cases by ROI (quick wins first).
```

**Expected Output:** Save as `use-cases.md`

---

## Prompt Set 3: Export Implementation Tasks

```
Break down the NotebookLM ↔ Claude Code integration into implementable tasks suitable for Claude Code's task tracker.

Format as a SPEC.md-style task table:

| Order | Task ID | Title | Complexity | Dependencies | Acceptance Criteria | Files Involved |
|-------|---------|-------|------------|--------------|---------------------|----------------|
| 1 | T1 | [action verb + noun] | S/M/L | - | [testable criteria] | [specific paths] |
| 2 | T2 | ... | ... | T1 | ... | ... |

Guidelines:
- Complexity: S (< 2 hours), M (2-8 hours), L (> 8 hours)
- Break L tasks into smaller chunks
- Include specific file paths (e.g., `codex-rs/crates/codex-tui/src/spec_kit/notebooklm.rs`)
- Acceptance criteria must be testable
- Order by dependency chain (no parallel tasks yet)

Then provide a separate "Quick Wins" section:
Tasks that can be done independently for immediate value, requiring < 4 hours each.
```

**Expected Output:** Save as `implementation-tasks.md`

---

## Prompt Set 4: Export Data Format Specifications

```
Define the exact data interchange formats for NotebookLM ↔ Claude Code communication.

For each integration point you identified, specify:

## Format: [Integration Point Name]

### Direction: Claude Code → NotebookLM

**Purpose**: [Why this data is sent]

**Format**: JSON / Markdown / TOML
```json
{
  "schema_version": "1.0",
  "required_field_1": "type: string, description: ...",
  "required_field_2": "type: array, description: ...",
  "optional_field_1": "type: object, description: ..."
}
```

**Example**:
```
[full working example with real data]
```

**Validation Rules**:
- Field X must match regex: ...
- Field Y must be one of: ...

---

### Direction: NotebookLM → Claude Code

**Purpose**: [Why this data is returned]

**Format**: JSON / Markdown / TOML
```
[schema]
```

**Example**:
```
[full working example]
```

**Claude Code Consumption**:
- File destination: `docs/SPEC-XXX/research-brief.md`
- Parser: `parse_notebooklm_response()` in `notebooklm_handler.rs`
- Next action: `/speckit.plan SPEC-XXX --context research-brief.md`

---

Include schemas for at least:
1. Research request (Claude → NotebookLM)
2. Research brief (NotebookLM → Claude)
3. Review request (Claude → NotebookLM)
4. Review report (NotebookLM → Claude)
5. Pattern analysis request
6. Pattern analysis results
```

**Expected Output:** Save as `data-formats.md`

---

## Prompt Set 5: Export Decision Framework

```
Create a decision tree for when to use NotebookLM vs. Claude Code's existing tools (multi-agent consensus, native heuristics, single-agent).

Format as a markdown flowchart:

# Tool Selection Decision Tree

## Question: [Task description]

START → Is the task deterministic (rule-based, no judgment)?
  ├─ YES → Use Native Rust Tools
  │         Examples: /speckit.clarify, /speckit.analyze
  │         Cost: $0, Time: <1s
  │
  └─ NO → Requires reasoning/judgment
      │
      ├─ Does it need broad context synthesis from multiple documents?
      │   ├─ YES → Use NotebookLM
      │   │         Examples: Cross-SPEC pattern analysis, domain research
      │   │         Cost: Manual, Time: 5-15 min
      │   │
      │   └─ NO → Specific technical decision
      │       │
      │       ├─ Is it a simple strategic decision (1 right answer)?
      │       │   ├─ YES → Use Single-Agent (Tier 1)
      │       │   │         Examples: /speckit.specify, /speckit.tasks
      │       │   │         Cost: ~$0.10, Time: 3-5 min
      │       │   │
      │       │   └─ NO → Complex decision or code generation
      │       │       │
      │       │       ├─ Is it critical (security/compliance/ship decision)?
      │       │       │   ├─ YES → Use Premium Multi-Agent (Tier 3)
      │       │       │   │         Examples: /speckit.audit, /speckit.unlock
      │       │       │   │         Cost: ~$0.80, Time: 10-12 min
      │       │       │   │
      │       │       │   └─ NO → Use Standard Multi-Agent (Tier 2)
      │       │       │             Examples: /speckit.plan, /speckit.validate
      │       │       │             Cost: ~$0.35, Time: 8-12 min

Also provide a comparison table:

| Capability | Native Tools | Single-Agent | Multi-Agent | NotebookLM |
|------------|--------------|--------------|-------------|------------|
| Pattern matching | ✅ Best | ❌ | ❌ | ❌ |
| Document synthesis | ❌ | ⚠️ Limited | ⚠️ Limited | ✅ Best |
| Code generation | ❌ | ⚠️ Simple | ✅ Good | ❌ |
| Cost | FREE | $0.10 | $0.35-$0.80 | Manual |
| Speed | <1s | 3-5min | 8-12min | 5-15min |
| Context window | N/A | 128K | 128K-200K | 2M tokens |

And specific scenarios:

**Use NotebookLM when:**
- [ ] Need to synthesize insights from 10+ documents
- [ ] Research domain knowledge outside codebase
- [ ] Compare architectural approaches across industry
- [ ] Identify cross-SPEC patterns/conflicts
- [ ] Generate learning summaries from evidence

**Don't use NotebookLM when:**
- [ ] Task is deterministic (use native)
- [ ] Need code generation (use Claude Code)
- [ ] Time-sensitive (use automation)
- [ ] Information exists in 1-2 docs (direct read)
```

**Expected Output:** Save as `decision-framework.md`

---

## Prompt Set 6: Export MVP Implementation Guide

```
Identify the SINGLE HIGHEST-VALUE integration to implement first as an MVP (Minimum Viable Product).

Provide:

# MVP: [Integration Name]

## Why This First
[Justification: highest ROI, lowest complexity, proves concept]

## User Story
As a [role], I want [goal] so that [benefit].

## Acceptance Criteria
Given [context]
When [action]
Then [expected result]

[List 3-5 testable criteria]

## Implementation Guide

### Step 1: Setup
Files to create:
- `docs/SPEC-KIT-XXX-notebooklm-mvp/spec.md`
- `codex-rs/crates/codex-tui/src/spec_kit/notebooklm.rs`

Dependencies to add (Cargo.toml):
```toml
[dependencies]
# ...
```

### Step 2: Core Functionality
Pseudocode for key functions:

```rust
// File: notebooklm.rs
pub struct NotebookLMExport {
    // fields
}

pub fn export_research_request(spec_id: &str) -> Result<String> {
    // 1. Load SPEC
    // 2. Extract context
    // 3. Format for NotebookLM
    // 4. Return markdown
}

pub fn import_research_brief(file_path: &Path) -> Result<ResearchBrief> {
    // 1. Parse markdown/JSON
    // 2. Validate schema
    // 3. Return structured data
}
```

### Step 3: CLI Integration
Add to `codex-rs/crates/codex-tui/src/routing.rs`:

```rust
SpecKitCommand::ExportForNotebookLM { spec_id } => {
    // handler logic
}
```

New slash command:
```bash
/speckit.export-research SPEC-ID
# Outputs: File saved to /tmp/notebooklm-export.md
```

### Step 4: Manual Workflow (MVP)
1. User runs `/speckit.export-research SPEC-KIT-073`
2. Claude Code outputs markdown to `/tmp/notebooklm-export.md`
3. User copies content to NotebookLM
4. NotebookLM processes, user copies response
5. User saves to `docs/SPEC-KIT-073/research-brief.md`
6. User runs `/speckit.plan SPEC-KIT-073 --context research-brief.md`

### Step 5: Testing
Test cases:
```rust
#[test]
fn test_export_research_request() {
    // Test valid SPEC export
}

#[test]
fn test_import_research_brief() {
    // Test parsing NotebookLM response
}
```

### Step 6: Documentation
Add to CLAUDE.md:
```markdown
## NotebookLM Integration (MVP)

Export research requests:
/speckit.export-research SPEC-ID

[usage instructions]
```

## Success Metrics
- [ ] Export completes in <2s
- [ ] NotebookLM can parse without manual editing
- [ ] Research brief loads into /speckit.plan successfully
- [ ] User saves 10+ minutes on domain research

## Time Estimate
Total: 4-6 hours

## Next Steps After MVP
[What to build in v2 after validating this works]
```

**Expected Output:** Save as `mvp-implementation-guide.md`

---

## Prompt Set 7: Export Integration Risks & Mitigations

```
Identify potential risks and failure modes for NotebookLM ↔ Claude Code integration.

For each risk, provide:

## Risk: [Name]

**Category**: Technical / Process / UX / Cost

**Likelihood**: High / Medium / Low

**Impact**: High / Medium / Low

**Description**: [What could go wrong]

**Scenario**:
1. [Step where failure occurs]
2. [Cascading effect]
3. [User-visible symptom]

**Mitigation Strategy**:
- **Prevention**: [How to avoid]
- **Detection**: [How to catch early]
- **Recovery**: [What to do if it happens]

**Example**:
```
[Concrete example of the risk materializing]
```

**Test Case**:
```rust
#[test]
fn test_risk_mitigation_XXX() {
    // Test that mitigation works
}
```

---

Cover at least:
- Format incompatibility (NotebookLM output changes)
- Context overflow (SPEC too large for export)
- Manual handoff errors (copy-paste mistakes)
- Stale data (NotebookLM working on old SPEC version)
- Cost creep (overusing manual process)
- User confusion (when to use which tool)
```

**Expected Output:** Save as `risks-and-mitigations.md`

---

## Usage Workflow

### Phase 1: Research Collection
1. Copy each prompt (1-7) into NotebookLM
2. Let NotebookLM generate responses based on its research
3. Save each response to the suggested filename
4. Collect all responses in a folder: `notebooklm-integration-outputs/`

### Phase 2: Handoff to Claude Code
Provide Claude Code with:
```
I've completed the NotebookLM research. Here are the outputs:

1. [paste mvp-implementation-guide.md]
2. [paste data-formats.md]
3. [paste implementation-tasks.md]
... etc
```

### Phase 3: Implementation
Claude Code will:
1. Analyze the MVP proposal
2. Create SPEC-KIT-XXX for the integration
3. Break down into tasks in SPEC.md
4. Start implementation
5. Validate against use cases
6. Update CLAUDE.md with decision framework

### Phase 4: Iteration
After MVP validation:
1. Return to NotebookLM with learnings
2. Request refinements for Phase 2
3. Repeat the cycle

---

## Tips for Best Results

**When running prompts in NotebookLM:**
- Reference your existing research sources about the PPP framework
- Ask for concrete, actionable outputs (not abstract concepts)
- Request specific file paths, function names, test cases
- Demand examples with real data (not placeholders)

**When providing outputs to Claude Code:**
- Include all sections (don't summarize)
- Preserve code blocks and formatting
- Note any areas where NotebookLM had uncertainty
- Highlight dependencies between prompts

**Quality checks:**
- Does the output include specific file paths?
- Are there concrete examples (not just schemas)?
- Can Claude Code start implementing immediately?
- Are acceptance criteria testable?

---

## Document Version
Version: 1.0
Created: 2025-11-29
Purpose: Enable structured research-to-implementation workflow between NotebookLM and Claude Code
Context: PPP framework integration exploration
