# Quick Start Tutorial

Get productive with Code CLI in 5 minutes.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Your First Command (1 minute)](#your-first-command-1-minute)
3. [Understanding the TUI (2 minutes)](#understanding-the-tui-2-minutes)
4. [Example Workflows (2 minutes)](#example-workflows-2-minutes)
5. [Essential Commands](#essential-commands)
6. [Next Steps](#next-steps)

---

## Prerequisites

Before starting, ensure:
- ✅ Code CLI installed ([installation guide](installation.md))
- ✅ Authentication configured ([setup guide](first-time-setup.md))
- ✅ You're in a directory with code files (for testing)

**Time Required**: 5 minutes

---

## Your First Command (1 minute)

### Interactive Mode

Launch Code and ask a simple question:

```bash
# Start interactive TUI
code
```

**In the chat composer, type**:
```
What files are in this directory?
```

**Press Enter** and watch Code:
1. Analyze your request
2. Execute appropriate tool (file listing)
3. Return results in the chat

**Exit**: Press `Ctrl+C` or type `/exit`

---

### Non-Interactive Mode

Run a command directly without opening the TUI:

```bash
# Execute a single task
code "list all Python files in this directory"
```

Code will:
- Process your request
- Show output in the terminal
- Exit automatically when done

---

### With Initial Prompt

Start the TUI with a pre-filled prompt:

```bash
# Launch with initial prompt (doesn't auto-execute)
code "explain the architecture of this codebase"
```

This opens the TUI with your prompt ready to send (press Enter to submit).

---

## Understanding the TUI (2 minutes)

### TUI Layout

When you launch `code`, you'll see:

```
┌─────────────────────────────────────────────┐
│  📝  Chat History                           │  ← Conversation history
│                                             │
│  User: What files are here?                 │
│  Assistant: I found 3 Python files:         │
│  - main.py                                  │
│  - utils.py                                 │
│  - tests.py                                 │
│                                             │
├─────────────────────────────────────────────┤
│  ✏️  Composer (Type here)                   │  ← Your input area
│  > |                                        │
│                                             │
├─────────────────────────────────────────────┤
│  ℹ️  Status: Model: gpt-5 | Auth: ChatGPT  │  ← Status bar
└─────────────────────────────────────────────┘
```

### Key Components

**1. Chat History** (top):
- Shows conversation with the AI
- Model responses with reasoning (if enabled)
- Tool executions and results
- Scroll with arrow keys or mouse

**2. Composer** (middle):
- Type your prompts here
- Multi-line input supported (Shift+Enter for new line)
- Submit with Enter

**3. Status Bar** (bottom):
- Current model
- Authentication method
- Workspace directory
- Sandbox mode

---

### Essential Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **Enter** | Send message (submit prompt) |
| **Shift+Enter** | New line in composer (multi-line input) |
| **Ctrl+C** | Exit Code |
| **Esc** | Clear composer (or cancel current operation) |
| **Esc Esc** | Edit previous message (backtrack) |
| **Ctrl+V / Cmd+V** | Paste image (from clipboard) |
| **@** | Fuzzy file search (type @ then filename) |
| **Up/Down** | Scroll chat history |
| **Tab** | Auto-complete (in file search) |

---

### Special Commands (Slash Commands)

Type `/` followed by a command name:

| Command | Purpose | Example |
|---------|---------|---------|
| **/new** | Start new conversation | `/new` |
| **/model** | Switch model or reasoning level | `/model` |
| **/reasoning** | Adjust reasoning effort | `/reasoning high` |
| **/themes** | Change TUI theme | `/themes` |
| **/status** | Show current configuration | `/status` |
| **/exit** | Exit Code | `/exit` |
| **/help** | Show help | `/help` |

**Spec-Kit Commands** (multi-agent automation):
- `/speckit.new` - Create new specification
- `/speckit.auto` - Full automation pipeline
- `/speckit.plan` - Generate work breakdown
- `/speckit.implement` - Code generation
- See [workflows.md](workflows.md) for details

---

## Example Workflows (2 minutes)

### Example 1: Code Explanation

**Goal**: Understand a code file

```bash
code
```

**In chat**:
```
Explain what src/main.py does and summarize its key functions.
```

**Expected Output**:
- File overview
- Function summaries
- Dependencies identified
- Potential improvements

---

### Example 2: Code Refactoring

**Goal**: Refactor code for better readability

**Prompt**:
```
Refactor the calculate_total() function in utils.py to use more descriptive variable names and add docstrings.
```

**Code will**:
1. Read `utils.py`
2. Identify the function
3. Propose refactored version
4. Show diff (before/after)
5. Ask for approval (unless in auto mode)
6. Apply changes if approved

**Review the diff**:
```diff
- def calculate_total(items):
-     t = 0
-     for i in items:
-         t += i.price
-     return t
+ def calculate_total(items):
+     """Calculate the total price of all items.
+
+     Args:
+         items: List of items with price attribute
+
+     Returns:
+         Total sum of all item prices
+     """
+     total_price = 0
+     for item in items:
+         total_price += item.price
+     return total_price
```

---

### Example 3: Writing Tests

**Goal**: Generate unit tests for a function

**Prompt**:
```
Write comprehensive unit tests for the validate_email() function in validators.py. Include edge cases and error conditions.
```

**Code will**:
1. Analyze the function
2. Generate test file (e.g., `test_validators.py`)
3. Include test cases:
   - Valid emails
   - Invalid formats
   - Edge cases (empty, null, special chars)
   - Boundary conditions
4. Ask for approval
5. Run tests to verify they pass

---

### Example 4: Bug Investigation

**Goal**: Find and fix a bug

**Prompt**:
```
Users are reporting that the login function returns "500 Internal Server Error". Investigate the issue in auth.py and propose a fix.
```

**Code will**:
1. Read `auth.py`
2. Identify potential issues (e.g., unhandled exceptions)
3. Propose fix with explanation
4. Show before/after diff
5. Suggest additional error handling

---

### Example 5: Documentation

**Goal**: Generate documentation for a module

**Prompt**:
```
Generate a comprehensive README.md for the database/ module, including setup instructions, API reference, and usage examples.
```

**Code will**:
1. Analyze all files in `database/`
2. Extract function signatures and purposes
3. Generate structured README:
   - Overview
   - Installation
   - API reference
   - Examples
   - Troubleshooting
4. Ask for approval
5. Create `database/README.md`

---

## Essential Commands

### CLI Usage

**Basic**:
```bash
code                           # Interactive TUI
code "prompt"                  # TUI with initial prompt
code exec "prompt"             # Non-interactive execution
```

**With Options**:
```bash
code --model o3                # Use specific model
code --read-only "explain"     # Read-only mode (no writes)
code --no-approval "task"      # Skip approval prompts (auto mode)
code --debug                   # Enable debug logging
code --sandbox workspace-write # Set sandbox mode
code --cd /path/to/project     # Change working directory
```

**Configuration**:
```bash
code --config model=o3                    # Override config value
code --config approval_policy=never       # Full auto mode
code --profile premium                    # Use named profile
code --version                            # Show version
code --help                               # Show help
```

---

### In-TUI Slash Commands

**Conversation**:
```
/new                           # Start new conversation
/exit                          # Exit Code
```

**Model & Settings**:
```
/model                         # Switch model
/reasoning low                 # Set reasoning effort (low/medium/high)
/themes                        # Change theme
/status                        # Show configuration
```

**Browser Integration** (if enabled):
```
/chrome                        # Connect to external Chrome
/chrome 9222                   # Connect to Chrome on port 9222
/browser https://example.com   # Open URL in internal browser
```

**Multi-Agent Commands** (requires multi-provider setup):
```
/plan "task"                   # Multi-agent planning (Claude, Gemini, GPT-5)
/solve "problem"               # Race multiple models (fastest wins)
/code "feature"                # Multi-agent code generation
```

**Spec-Kit Automation** (fork feature):
```
/speckit.new "description"     # Create new spec
/speckit.auto SPEC-ID          # Run full automation pipeline
/speckit.plan SPEC-ID          # Generate plan
/speckit.implement SPEC-ID     # Generate code
/speckit.validate SPEC-ID      # Run tests
```

---

### File Operations

**Attach files/images**:
```bash
# Via CLI
code --image screenshot.png "explain this error"
code -i img1.png,img2.png "compare these diagrams"

# Via TUI
# Ctrl+V / Cmd+V to paste image from clipboard
```

**File search in composer**:
```
# Type @ to trigger fuzzy file search
@main.py       # Searches for main.py
@test          # Searches for files matching "test"
# Use Up/Down to select, Tab/Enter to insert
```

---

## Next Steps

Now that you've completed the quick start:

1. **Learn Common Workflows** → [workflows.md](workflows.md)
   - Spec-kit automation (multi-agent PRD workflows)
   - Code review and refactoring
   - Test generation and validation
   - CI/CD integration

2. **Explore Configuration** → [first-time-setup.md](first-time-setup.md)
   - Custom model providers
   - MCP servers (extend functionality)
   - Multi-provider setup
   - Quality gates

3. **Read FAQ** → [faq.md](faq.md)
   - Common questions
   - Comparison with other tools
   - Cost management
   - Privacy and data handling

4. **Troubleshooting** → [troubleshooting.md](troubleshooting.md)
   - Installation errors
   - Authentication issues
   - Performance problems
   - Common mistakes

---

## Quick Reference Card

### Most Common Tasks

| Task | Command |
|------|---------|
| **Explain code** | `code "explain main.py"` |
| **Refactor function** | `code "refactor calculate() in utils.py"` |
| **Write tests** | `code "write tests for auth.py"` |
| **Fix bug** | `code "fix the login bug in auth.py"` |
| **Generate docs** | `code "generate README for api/ module"` |
| **Code review** | `code "review the changes in src/"` |
| **Add feature** | `code "add user authentication"` |

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **Enter** | Send message |
| **Shift+Enter** | New line |
| **Ctrl+C** | Exit |
| **Esc Esc** | Edit previous message |
| **@** | File search |

### Essential Slash Commands

| Command | Purpose |
|---------|---------|
| **/new** | New conversation |
| **/model** | Switch model |
| **/reasoning high** | Increase reasoning |
| **/status** | Show config |
| **/exit** | Exit Code |

---

**Ready to dive deeper?** → Continue to [Common Workflows](workflows.md)
