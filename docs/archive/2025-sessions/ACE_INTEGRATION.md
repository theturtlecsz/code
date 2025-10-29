# ACE Playbook Integration Guide

**ultrathink mode: How to actually USE the ACE MCP server**

---

## Problem Statement

We have an MCP server (`ace-playbook`) with 3 tools, but no user-facing interface for:
- Bootstrapping initial strategies
- Pinning important patterns
- Viewing current playbook
- Managing bullets

---

## Architecture

```
You (user)
  ↓ Types command or natural language
CODE CLI / Claude Code
  ↓ Translates to MCP call
ace-playbook MCP server
  ↓ Returns data
CODE CLI / Claude Code
  ↓ Shows you results
```

**You don't call MCP directly - you tell the AI to use it.**

---

## Interaction Patterns

### Pattern 1: Natural Language (Current)

**You say:**
```
Use ace-playbook to get bullets for scope 'implement' in /home/thetu/code
```

**AI translates to:**
```javascript
mcp.call("ace-playbook", "playbook_slice", {
  repo_root: "/home/thetu/code",
  branch: "main",
  scope: "implement",
  k: 20
})
```

**AI shows you:** List of bullets

---

### Pattern 2: Slash Commands (RECOMMENDED)

**Create these in your code project:**

#### `/ace-pin`
Pin important strategies to playbook.

**Usage:**
```
/ace-pin global "Always validate user inputs"
/ace-pin implement "Use MCP types from mcp-types/ crate"
```

**Implementation:** Calls `playbook_pin` MCP tool

---

#### `/ace-list`
Show current playbook bullets.

**Usage:**
```
/ace-list global
/ace-list implement
/ace-list all
```

**Implementation:** Calls `playbook_slice` with k=100

---

#### `/ace-bootstrap`
Initialize playbook with starter strategies.

**Usage:**
```
/ace-bootstrap
```

**Implementation:**
1. Reads `playbook_bootstrap.json` from project
2. Pins all bullets to appropriate scopes
3. Reports what was added

---

#### `/ace-learn`
Manually trigger learning from feedback.

**Usage:**
```
/ace-learn success "Fixed authentication bug"
/ace-learn failure "Tests failed - null pointer"
```

**Implementation:** Calls `learn` with structured feedback

---

## Recommended Setup Flow

### Step 1: Create Slash Commands

**Create:** `/home/thetu/code/.code/commands/ace-pin.md`

```markdown
# Pin strategy to ACE playbook

**Arguments:**
- scope: global | specify | tasks | implement | test
- text: Strategy text to pin

**Usage:**
```
/ace-pin global "Always validate inputs"
/ace-pin implement "Use MCP types from mcp-types/"
```

**Implementation:**
Call ace-playbook MCP server:
- Tool: playbook_pin
- Input: {repo_root, branch, scope, bullets: [{text, kind: "helpful"}]}
- Report: "✅ Pinned 1 bullet to {scope} scope"
```

---

### Step 2: Create Bootstrap File

**Create:** `/home/thetu/code/playbook_bootstrap.json`

```json
{
  "global": [
    "Always validate user inputs before processing",
    "Include comprehensive error handling",
    "Write tests for new functionality",
    "Use type hints for better code clarity",
    "Handle edge cases explicitly"
  ],
  "implement": [
    "Use MCP types from mcp-types/ crate",
    "Use MockMCPServer for MCP client tests",
    "CLI commands derive clap::Parser in cli/src/commands/",
    "Prefer async/await over manual futures",
    "Use Result<T, E> for error propagation"
  ],
  "test": [
    "Use #[tokio::test] for async tests",
    "Include both unit and integration tests",
    "Mock external dependencies",
    "Test error paths explicitly"
  ]
}
```

---

### Step 3: Create Project CLAUDE.md

**Create:** `/home/thetu/code/CLAUDE.md`

```markdown
# ACE Playbook Integration

This project uses ACE (Agentic Context Engineering) for learned strategies.

## Available MCP Server: ace-playbook

**Tools:**
- `playbook_slice` - Get relevant strategies before code generation
- `playbook_pin` - Pin important patterns
- `playbook_learn` - Update scores from execution feedback

## When to Use ACE

### BEFORE generating code:
1. Call `playbook_slice` to get relevant strategies
2. Include bullet texts in your reasoning
3. Apply strategies to generation

Example:
```
Before implementing authentication:
1. Get bullets: playbook_slice(scope="implement", k=20)
2. Found strategies:
   - "Use MCP types from mcp-types/"
   - "Always validate inputs"
   - "Use Result<T,E> for errors"
3. Apply these in implementation
```

### AFTER user provides feedback:
1. Detect success/failure from user response
2. Call `playbook_learn` to update scores
3. Track which bullets were used

Example:
```
User: "This works perfectly!"
→ Call learn(compile_ok=true, tests_passed=true, bullet_ids_used=[1,2,3])
→ Bullets get +1.0 score boost
```

## Slash Commands

### /ace-pin <scope> <text>
Pin a strategy to the playbook.

Usage: `/ace-pin global "Always validate inputs"`

### /ace-list <scope>
Show current playbook bullets.

Usage: `/ace-list implement`

### /ace-bootstrap
Load initial strategies from playbook_bootstrap.json.

Usage: `/ace-bootstrap`

## Scope Mapping

Map user tasks to scopes:
- User asks for requirements/specs → `specify`
- User asks to plan tasks → `tasks`
- User asks to implement code → `implement`
- User asks about testing → `test`
- General principles → `global`

## Repository Context

**Current repo:** /home/thetu/code
**Branch:** main (default)
**Playbook location:** SQLite at ~/.code/ace/playbooks_normalized.sqlite3

## Integration Pattern

```javascript
// Pseudo-code for your implementation
async function generateCode(userQuestion) {
  // 1. Get playbook bullets
  const bullets = await mcp.call("ace-playbook", "playbook_slice", {
    repo_root: "/home/thetu/code",
    branch: "main",
    scope: detectScope(userQuestion),
    k: 20
  });

  // 2. Enhance prompt
  const strategies = bullets.bullets.map(b => b.text).join("\n- ");
  const enhancedPrompt = `${userQuestion}\n\nStrategies:\n- ${strategies}`;

  // 3. Generate with your LLM subscription
  const answer = await yourLLM.generate(enhancedPrompt);

  // 4. After execution feedback
  await mcp.call("ace-playbook", "playbook_learn", {
    repo_root: "/home/thetu/code",
    branch: "main",
    scope: scope,
    question: userQuestion,
    attempt: answer,
    feedback: executionResults,
    bullet_ids_used: bullets.bullets.map(b => b.id)
  });

  return answer;
}
```

## Bootstrap on First Use

On first use in this repo:
1. Check if playbook is empty
2. If empty, suggest: `/ace-bootstrap`
3. Loads strategies from playbook_bootstrap.json
4. Reports: "✅ Loaded 15 strategies across 4 scopes"

## Proactive Usage

**You should automatically:**
- Call `playbook_slice` before code generation (if scope detected)
- Call `playbook_learn` after user confirms success/failure
- Suggest `/ace-bootstrap` on first session in repo
- Show bullet count in status: "Using 12 ACE strategies"

## Example Session

```
User: Add OAuth2 authentication to the API

AI: [Automatically calls playbook_slice(scope="implement")]
    Found 8 relevant strategies including:
    - "Use MCP types from mcp-types/"
    - "Always validate inputs"
    - "Use Result<T,E> for errors"

    [Generates code applying these strategies]

User: The tests passed!

AI: [Automatically calls playbook_learn(success=true, bullet_ids=[1,2,3])]
    ✅ Updated 3 bullets (+1.0 score each)

    Next time I implement auth, these strategies will rank higher.
```

---

**This guide should be read by AI assistants working in this repository.**
```

---

### Step 4: Create Slash Command Files

**See next section for implementation.**

---

## Missing Pieces

1. **Slash commands** - Need to create in `/home/thetu/code/.code/commands/`
2. **Bootstrap flow** - Automated initial setup
3. **AI prompts** - Teach AI when/how to use ACE
4. **Feedback detection** - Auto-detect success/failure from user messages

---

## What You Can Do RIGHT NOW (Manual)

### Pin Your First Bullet

**In Claude Code or CODE CLI, say:**

```
Call the ace-playbook MCP tool playbook_pin with this input:
{
  "repo_root": "/home/thetu/code",
  "branch": "main",
  "scope": "global",
  "bullets": [
    {
      "text": "Always validate user inputs before processing",
      "kind": "helpful"
    }
  ]
}
```

**Expected response:** `{"status": "ok", "pinned_count": 1}`

---

### Verify It Worked

```
Call ace-playbook playbook_slice with:
{
  "repo_root": "/home/thetu/code",
  "branch": "main",
  "scope": "global",
  "k": 10
}
```

**Expected:** Returns your pinned bullet.

---

## Proper Solution (What I Should Build)

**You need:**
1. Slash commands for easy interaction
2. CLAUDE.md that teaches AI to use ACE proactively
3. Bootstrap script to load initial strategies
4. Auto-detection of success/failure for learning

**Want me to build these?**

---

**My apology:** I gave you server infrastructure but no user interface. That's why you're confused.

**Immediate next step:** Should I create the slash commands + CLAUDE.md + bootstrap flow?
