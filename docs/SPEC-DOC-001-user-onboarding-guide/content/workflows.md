# Common Workflows

Comprehensive guide to common Code CLI usage patterns and workflows.

---

## Table of Contents

1. [Overview](#overview)
2. [Spec-Kit Automation Framework](#spec-kit-automation-framework)
3. [Manual Coding Workflows](#manual-coding-workflows)
4. [Code Review and Refactoring](#code-review-and-refactoring)
5. [Testing and Validation](#testing-and-validation)
6. [Documentation Generation](#documentation-generation)
7. [CI/CD Integration](#cicd-integration)
8. [Multi-Agent Workflows](#multi-agent-workflows)
9. [Browser Integration](#browser-integration)
10. [Best Practices](#best-practices)

---

## Overview

Code CLI supports two main workflow categories:

**1. Spec-Kit Automation** (Fork Feature)
- Multi-agent consensus-driven development
- Full PRD → Plan → Tasks → Implementation → Validation → Audit pipeline
- Quality gates at each stage
- Automated or manual step-through

**2. Manual Coding Assistance**
- Interactive chat for code questions
- Code generation and refactoring
- Bug fixing and debugging
- Documentation and testing

---

## Spec-Kit Automation Framework

**What is Spec-Kit?**
A unique fork feature that provides automated, multi-agent software development workflows with quality gates and consensus validation.

### Full Automation Pipeline

**Use Case**: Complete feature implementation with automated quality checks

**Command**: `/speckit.auto SPEC-ID`

**What It Does**:
1. **Specify** - PRD refinement (single agent)
2. **Clarify** - Ambiguity detection (native heuristics, FREE)
3. **Plan** - Work breakdown (3 agents: gemini, claude, gpt-5)
4. **Tasks** - Task decomposition (single agent)
5. **Implement** - Code generation (gpt-5-codex + validator)
6. **Validate** - Test strategy (3 agents)
7. **Audit** - Compliance check (3 premium agents)
8. **Unlock** - Final approval (3 premium agents)

**Example**:

```bash
# Step 1: Create new SPEC
code
/speckit.new Add OAuth2 user authentication with JWT tokens

# Output: Created SPEC-KIT-123

# Step 2: Run full automation
/speckit.auto SPEC-KIT-123

# The pipeline will:
# - Generate comprehensive PRD
# - Detect ambiguities and inconsistencies
# - Create multi-agent consensus plan
# - Break down into tasks
# - Generate implementation code
# - Design test strategy
# - Run compliance checks
# - Provide ship/no-ship recommendation
```

**Time**: ~45-50 minutes
**Cost**: ~$2.70 (75% cheaper than previous $11)

---

### Manual Stage-by-Stage Workflow

**Use Case**: Greater control over each stage, inspect outputs before proceeding

**Example Workflow**:

```bash
# 1. Create SPEC
/speckit.new Implement rate limiting for API endpoints

# Output: SPEC-KIT-124

# 2. Optional: Run quality checks
/speckit.clarify SPEC-KIT-124    # Detect vague requirements (FREE)
/speckit.analyze SPEC-KIT-124    # Check consistency (FREE)
/speckit.checklist SPEC-KIT-124  # Quality scoring (FREE)

# 3. Plan stage (multi-agent consensus)
/speckit.plan SPEC-KIT-124
# Review plan.md output
# Time: ~10-12 min, Cost: ~$0.35

# 4. Tasks stage (task decomposition)
/speckit.tasks SPEC-KIT-124
# Review tasks.md output
# Time: ~3-5 min, Cost: ~$0.10

# 5. Implementation (code generation)
/speckit.implement SPEC-KIT-124
# Review generated code
# Time: ~8-12 min, Cost: ~$0.11

# 6. Validation (test strategy)
/speckit.validate SPEC-KIT-124
# Review test plan
# Time: ~10-12 min, Cost: ~$0.35

# 7. Audit (compliance)
/speckit.audit SPEC-KIT-124
# Review security/compliance report
# Time: ~10-12 min, Cost: ~$0.80

# 8. Unlock (ship decision)
/speckit.unlock SPEC-KIT-124
# Review final recommendation
# Time: ~10-12 min, Cost: ~$0.80
```

**Advantages**:
- ✅ Inspect outputs at each stage
- ✅ Iterate on specific stages
- ✅ Stop early if issues found
- ✅ Greater understanding of process

---

### Quality Gates Configuration

**Customize agent selection per stage** to balance cost and quality:

```toml
# ~/.code/config.toml

[quality_gates]
# Native (FREE, instant)
# - /speckit.new, /speckit.clarify, /speckit.analyze, /speckit.checklist

# Single agent (cheap, ~$0.10, 3-5 min)
tasks = ["gemini"]  # Gemini Flash: 12x cheaper than GPT-4o

# Multi-agent consensus (balanced, ~$0.35, 10-12 min)
plan = ["gemini", "claude", "code"]
validate = ["gemini", "claude", "code"]

# Premium agents (critical, ~$0.80, 10-12 min)
audit = ["gemini-pro", "claude-opus", "gpt-5"]
unlock = ["gemini-pro", "claude-opus", "gpt-5"]
```

**Cost Strategies**:
- **Minimum cost**: Use `["gemini"]` for all stages (~$0.50 total)
- **Balanced** (recommended): Above configuration (~$2.70 total)
- **Maximum quality**: Premium for all stages (~$11 total)

---

### Spec-Kit Best Practices

**1. Write Clear Descriptions**:
```bash
# ❌ Bad: Vague
/speckit.new Add auth

# ✅ Good: Specific
/speckit.new Add OAuth2 authentication with Google and GitHub providers, JWT token management, and session persistence
```

**2. Run Quality Checks Early**:
```bash
# After creating SPEC, run native checks (FREE)
/speckit.clarify SPEC-KIT-125   # Finds vague language
/speckit.analyze SPEC-KIT-125   # Finds inconsistencies
/speckit.checklist SPEC-KIT-125 # Quality score

# Fix issues before running expensive multi-agent stages
```

**3. Monitor Evidence Footprint**:
```bash
# Check evidence size after large runs
/spec-evidence-stats --spec SPEC-KIT-125

# Output shows per-SPEC evidence sizes
# Recommended limit: 25 MB per SPEC
```

**4. Use Guardrail Commands** for validation:
```bash
# Run guardrail checks before implementation
/guardrail.plan SPEC-KIT-125
/guardrail.implement SPEC-KIT-125

# Full guardrail pipeline
/guardrail.auto SPEC-KIT-125 --from plan
```

---

## Manual Coding Workflows

### Code Generation

**Use Case**: Generate new code from scratch

**Example 1**: Create a new function

```bash
code
```

**Prompt**:
```
Create a Python function that validates email addresses using regex. Include:
- Comprehensive regex pattern (RFC 5322 compliant)
- Docstring with examples
- Type hints
- Error handling for invalid inputs
```

**Expected Output**:
```python
import re
from typing import Union

def validate_email(email: str) -> bool:
    """
    Validate email address using RFC 5322 compliant regex.

    Args:
        email: Email address string to validate

    Returns:
        True if email is valid, False otherwise

    Raises:
        TypeError: If email is not a string

    Examples:
        >>> validate_email("user@example.com")
        True
        >>> validate_email("invalid.email")
        False
    """
    if not isinstance(email, str):
        raise TypeError("Email must be a string")

    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
    return bool(re.match(pattern, email))
```

---

**Example 2**: Generate a complete module

**Prompt**:
```
Create a Python module called database.py that provides:
- Database connection manager (singleton pattern)
- CRUD operations for a User model
- Transaction support
- Connection pooling
- Error handling
Use SQLAlchemy and include docstrings.
```

**Code will**:
1. Create `database.py`
2. Implement all requested features
3. Add comprehensive docstrings
4. Include type hints
5. Ask for approval before writing

---

### Code Modification

**Use Case**: Modify existing code

**Example 1**: Refactor for readability

**Prompt**:
```
Refactor src/utils.py to improve readability:
- Use descriptive variable names
- Add type hints
- Add docstrings to all functions
- Break long functions into smaller ones
- Add comments for complex logic
```

---

**Example 2**: Optimize performance

**Prompt**:
```
Optimize the data processing pipeline in pipeline.py:
- Replace loops with vectorized operations (NumPy/Pandas)
- Add caching for expensive operations
- Use multiprocessing for parallel tasks
- Profile the code and identify bottlenecks
```

---

**Example 3**: Add error handling

**Prompt**:
```
Add comprehensive error handling to api.py:
- Catch specific exceptions (not bare except)
- Add logging for errors
- Return appropriate HTTP status codes
- Add retry logic for network errors
- Validate inputs before processing
```

---

## Code Review and Refactoring

### Code Review

**Use Case**: Review code changes for quality, bugs, security issues

**Example**:

```bash
# Review specific file
code "Review auth.py for security vulnerabilities, coding best practices, and potential bugs. Provide specific recommendations."

# Review changes in git
code "Review the changes in the last commit and suggest improvements."

# Review entire module
code "Perform a comprehensive code review of the api/ module. Focus on:
- Security vulnerabilities
- Performance issues
- Code duplication
- Missing error handling
- Potential bugs"
```

**Code will provide**:
- Identified issues with severity (critical, high, medium, low)
- Specific line numbers
- Recommended fixes
- Code examples

---

### Refactoring Patterns

**Example 1**: Extract method

**Prompt**:
```
Refactor the process_order() function in orders.py:
- Extract validation logic into validate_order()
- Extract payment logic into process_payment()
- Extract shipping logic into create_shipment()
- Ensure each function has single responsibility
```

---

**Example 2**: Design patterns

**Prompt**:
```
Refactor database.py to use the Repository pattern:
- Create UserRepository class
- Implement CRUD operations
- Separate database logic from business logic
- Add interface for easy testing/mocking
```

---

**Example 3**: Remove code duplication

**Prompt**:
```
Identify and refactor code duplication in:
- controllers/user_controller.py
- controllers/admin_controller.py
- controllers/api_controller.py

Extract common logic into base controller or helper functions.
```

---

## Testing and Validation

### Test Generation

**Example 1**: Unit tests

**Prompt**:
```
Generate comprehensive unit tests for validators.py. Include:
- Happy path tests
- Edge cases (empty, null, boundary values)
- Error conditions
- Parametrized tests for multiple inputs
- Mock external dependencies
Use pytest framework.
```

---

**Example 2**: Integration tests

**Prompt**:
```
Create integration tests for the API endpoints in api/users.py:
- Test GET /users (list, pagination, filtering)
- Test POST /users (create, validation errors)
- Test PUT /users/:id (update, not found errors)
- Test DELETE /users/:id (delete, cascade behavior)
Use pytest and FastAPI TestClient.
```

---

**Example 3**: End-to-end tests

**Prompt**:
```
Generate E2E tests for the user registration flow:
1. Navigate to registration page
2. Fill form with valid data
3. Submit and verify success message
4. Verify email confirmation sent
5. Activate account via email link
6. Verify user can login
Use Playwright for browser automation.
```

---

### Test Execution and Debugging

**Example 1**: Run tests and fix failures

**Prompt**:
```
Run the test suite and fix any failing tests. For each failure:
- Identify root cause
- Propose fix
- Show before/after diff
- Re-run tests to verify fix
```

---

**Example 2**: Improve test coverage

**Prompt**:
```
Analyze test coverage for src/auth/ module and:
- Identify untested code paths
- Generate tests for uncovered lines
- Target 90%+ line coverage
- Include branch coverage for conditionals
```

---

## Documentation Generation

### API Documentation

**Example**:

**Prompt**:
```
Generate comprehensive API documentation for api/ module:
- OpenAPI/Swagger spec
- Endpoint descriptions
- Request/response examples
- Authentication requirements
- Error codes and meanings
- Rate limiting info
Output as docs/api.md
```

---

### README Generation

**Example**:

**Prompt**:
```
Generate README.md for this project including:
- Project overview and purpose
- Features list
- Installation instructions
- Quick start guide
- Usage examples
- Configuration options
- Contributing guidelines
- License information
```

---

### Code Documentation

**Example**:

**Prompt**:
```
Add comprehensive docstrings to all functions in utils.py:
- Google-style docstrings
- Parameter descriptions with types
- Return value descriptions
- Raises section for exceptions
- Examples section
- Notes for non-obvious behavior
```

---

## CI/CD Integration

### Non-Interactive Mode

Code CLI can run in non-interactive mode for automation:

```bash
# Run tests and fix failures (no approval prompts)
code exec "run the test suite and fix any failures"

# Code quality check
code exec --read-only "analyze code quality and generate report"

# Generate reports
code exec --config output_format=json "list all TODO comments"
```

---

### GitHub Actions Integration

**Example workflow** (`.github/workflows/ai-code-review.yml`):

```yaml
name: AI Code Review

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Code CLI
        run: |
          npm install -g @just-every/code

      - name: Run AI Code Review
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          code exec --read-only --config output_format=json \
            "Review the changes in this PR for:
             - Security vulnerabilities
             - Code quality issues
             - Performance problems
             - Missing tests
             Output as structured JSON." > review.json

      - name: Post Review Comment
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const review = JSON.parse(fs.readFileSync('review.json', 'utf8'));
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: review.summary
            });
```

---

### Pre-Commit Integration

**Example** (`.git/hooks/pre-commit`):

```bash
#!/bin/bash

# Run Code CLI to check for common issues
code exec --read-only --no-approval \
  "Check the staged changes for:
   - Console.log statements
   - Hardcoded secrets
   - TODOs without issue references
   - Missing error handling
   Exit 1 if issues found."

exit $?
```

---

## Multi-Agent Workflows

**Prerequisites**: Multi-provider setup ([setup guide](first-time-setup.md#step-4-multi-provider-setup-optional))

### Multi-Agent Planning

**Command**: `/plan "task description"`

**What It Does**:
- All agents (Claude, Gemini, GPT-5) review the task
- Each agent proposes a plan independently
- Code synthesizes a consolidated plan from all perspectives
- Provides consensus view with highlighted disagreements

**Example**:

```bash
code
/plan "Migrate the authentication system from session-based to JWT tokens"
```

**Output**:
```
📋 Multi-Agent Plan (Consensus: 3/3 agents)

## Agreed Approach:
1. Create JWT token service module
2. Update authentication middleware
3. Migrate existing sessions to JWT
4. Update frontend to use JWT
5. Add token refresh logic
6. Deprecate session storage

## Points of Disagreement:
- Gemini suggests immediate cutover
- Claude recommends gradual migration with feature flag
- GPT-5 proposes dual-auth support during transition

## Recommended: Gradual migration (2 agents in favor)

[Detailed plan follows...]
```

---

### Multi-Agent Problem Solving

**Command**: `/solve "problem description"`

**What It Does**:
- Multiple agents race to solve the problem
- Fastest solution is presented first
- Other solutions shown for comparison
- Best solution selected based on correctness and speed

**Example**:

```bash
/solve "Why does the user login fail with 'NoneType object has no attribute email'?"
```

**Output**:
```
🏁 Solution Race Results:

🥇 Claude (4.2s): Root cause found
   - auth.py line 45: User.query.filter() returns None
   - Fix: Add null check before accessing user.email
   - Proposed fix: [code shown]

🥈 Gemini (5.8s): Same root cause
   - Additional suggestion: Add logging for failed login attempts

🥉 GPT-5 (7.1s): Same root cause
   - Additional suggestion: Consider using user.get('email') with default

Recommended fix: Claude's solution with Gemini's logging addition
```

---

### Multi-Agent Code Generation

**Command**: `/code "feature description"`

**What It Does**:
- Multiple agents generate code independently
- Creates separate git worktrees for each implementation
- Evaluates all implementations
- Merges the optimal solution

**Example**:

```bash
/code "Add dark mode support with theme toggle and persistence"
```

**Output**:
```
🔨 Multi-Agent Code Generation

Creating worktrees:
- worktree/claude: Claude's implementation
- worktree/gemini: Gemini's implementation
- worktree/gpt5: GPT-5's implementation

Evaluation results:
✅ Claude: Clean implementation, uses CSS variables
✅ Gemini: Similar approach, includes transition animations
✅ GPT-5: More complex, includes automatic theme detection

Selected: Claude's implementation
Enhancements added from others:
- Gemini's transition animations
- GPT-5's automatic theme detection

Merging to main worktree...
```

---

## Browser Integration

### External Chrome Connection

**Use Case**: Control existing Chrome browser for debugging, testing, scraping

**Setup**:

```bash
# Launch Chrome with remote debugging
google-chrome --remote-debugging-port=9222

# Or on macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222
```

**In Code**:

```bash
code
/chrome 9222
```

**Example Tasks**:

```
Navigate to https://example.com and screenshot the homepage

Fill the login form with username "test@example.com" and click submit

Extract all product prices from the current page

Monitor network requests and identify slow API calls
```

---

### Internal Headless Browser

**Use Case**: Automated testing, scraping without visible browser

**In Code**:

```bash
/browser https://example.com
```

**Example Tasks**:

```
Navigate to the search page, search for "rust programming", and extract the first 10 results

Test the user registration flow:
1. Fill form with test data
2. Submit
3. Verify success message appears
4. Screenshot the confirmation page

Check if the website renders correctly on mobile (375x667 viewport)
```

---

## Best Practices

### General Best Practices

**1. Be Specific in Prompts**:

❌ Bad:
```
Fix the bug
```

✅ Good:
```
Fix the NullPointerException in UserService.authenticate() at line 42. The error occurs when the email parameter is null. Add validation and return appropriate error message.
```

---

**2. Provide Context**:

❌ Bad:
```
Optimize this
```

✅ Good:
```
Optimize the data processing pipeline in pipeline.py. Current performance: 10 records/second. Target: 100 records/second. Focus on database queries (N+1 problem) and data transformations (use vectorization).
```

---

**3. Review Before Approving**:

- Always review diffs before approving changes
- Understand why changes were made
- Check for unintended side effects
- Verify tests still pass

---

**4. Use Appropriate Approval Policy**:

```toml
# For exploration/learning
approval_policy = "untrusted"  # Ask before running untrusted commands

# For development
approval_policy = "on-request"  # Model decides when to ask (recommended)

# For automation/CI
approval_policy = "never"  # Full auto (use with read-only or in isolated environment)
```

---

**5. Monitor Costs** (API key usage):

```bash
# Use cheaper models for simple tasks
code --model gpt-4o-mini "simple formatting task"

# Use premium models for complex tasks
code --model o3 --config model_reasoning_effort=high "complex architectural decision"

# For Spec-Kit: Use balanced quality gates configuration
# Cheap agents for simple stages, premium for critical
```

---

### Spec-Kit Best Practices

**1. Start with Quality Checks** (FREE):

```bash
# Before running expensive multi-agent stages
/speckit.clarify SPEC-ID    # Find vague requirements
/speckit.analyze SPEC-ID    # Find inconsistencies
/speckit.checklist SPEC-ID  # Quality score

# Fix issues, then proceed
/speckit.auto SPEC-ID
```

**2. Use Manual Workflow for Learning**:

```bash
# Step through each stage to understand the process
/speckit.plan SPEC-ID
# Review plan.md
/speckit.tasks SPEC-ID
# Review tasks.md
# etc.
```

**3. Monitor Evidence Footprint**:

```bash
# Check after large runs
/spec-evidence-stats --spec SPEC-ID

# Keep evidence under 25 MB per SPEC
# Archive or clean old evidence if needed
```

---

### Security Best Practices

**1. Use Read-Only Mode for Untrusted Code**:

```bash
code --read-only "analyze this repository for security issues"
```

**2. Review Sandbox Mode**:

```toml
# Default: safe for most workflows
sandbox_mode = "workspace_write"

# More restrictive
sandbox_mode = "read-only"

# Only in isolated environments (Docker, etc.)
sandbox_mode = "danger-full-access"
```

**3. Never Commit Secrets**:

```bash
# Code will warn if detecting potential secrets
# But always review diffs before committing

# Use environment variables for secrets
export API_KEY="secret"

# Not hardcoded in files
api_key = "secret"  # ❌ Bad
```

---

## Next Steps

Now that you understand common workflows:

1. **FAQ** → [faq.md](faq.md)
   - Common questions
   - Cost management
   - Privacy and security
   - Comparison with other tools

2. **Troubleshooting** → [troubleshooting.md](troubleshooting.md)
   - Installation errors
   - Authentication issues
   - Performance problems
   - Common mistakes

3. **Advanced Configuration** → [../../config.md](../../config.md)
   - Custom model providers
   - Project hooks
   - Validation harnesses

---

**Master the workflows!** → Continue to [FAQ](faq.md)
