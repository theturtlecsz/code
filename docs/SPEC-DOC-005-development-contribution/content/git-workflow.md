# Git Workflow

Git branching strategy, commits, and PR process.

---

## Branching Strategy

### Main Branch

**Branch**: `main`
**Protection**: Protected, requires PR
**Purpose**: Stable production code

---

### Feature Branches

**Format**: `feature/description` or `username/description`

**Examples**:
- `feature/add-dark-mode`
- `fix/database-connection`
- `docs/api-documentation`

**Create**:
```bash
git checkout -b feature/add-dark-mode
```

---

## Conventional Commits

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `test`: Tests
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `chore`: Build/tooling changes

### Examples

```bash
feat(tui): add dark mode toggle
fix(mcp): resolve connection timeout
docs(api): add MCP integration guide
test(spec-kit): add consensus unit tests
```

---

## Commit Best Practices

**DO**:
```bash
# Atomic commits
git commit -m "feat(tui): add command history"
git commit -m "test(tui): add history tests"

# Descriptive messages
git commit -m "fix(db): resolve race condition in pool"

# Present tense
git commit -m "add feature" (not "added feature")
```

**DON'T**:
```bash
# Vague messages
git commit -m "fix stuff"

# Multiple changes
git commit -m "add feature, fix bug, update docs"
```

---

## Pull Request Process

### 1. Create PR

```bash
# Push branch
git push -u origin feature/add-dark-mode

# Create PR (via GitHub UI)
```

### 2. PR Template

```markdown
## Summary
Add dark mode toggle to TUI settings

## Changes
- Add dark mode theme
- Add toggle in settings
- Update color scheme

## Testing
- Tested on Linux, macOS
- All tests passing

## Checklist
- [x] Tests added
- [x] Documentation updated
- [x] Clippy passing
```

### 3. Review Process

- CI must pass (tests, clippy, fmt)
- At least 1 approval required
- Address review comments
- Squash/rebase if requested

### 4. Merge

- Squash and merge (default)
- Delete branch after merge

---

## Upstream Sync

**Frequency**: Quarterly

**Process**: See [Upstream Sync Guide](upstream-sync.md)

---

## Summary

**Workflow**:
1. Create feature branch
2. Make atomic commits (conventional format)
3. Push and create PR
4. Pass CI + review
5. Squash and merge

**Next**: [Code Style](code-style.md)
