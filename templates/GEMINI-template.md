# GEMINI.md - \[PROJECT\_NAME] Instructions

## Repository Context

**Project**: \[PROJECT\_NAME]
**Created**: \[DATE]
**Type**: \[PROJECT\_TYPE]

## Local Memory Integration (MANDATORY)

**Policy**: CLI + REST only. No MCP.

### Golden Path vs Manual

| Mode                | When                   | Memory Handling                                        |
| ------------------- | ---------------------- | ------------------------------------------------------ |
| **`/speckit.auto`** | Primary workflow       | Stage0 orchestrates memory recall + Tier2 (NotebookLM) |
| **Ad-hoc work**     | Debugging, exploration | Use `lm` commands manually (below)                     |

### Manual Commands (Non-Golden-Path Only)

**Before proposing changes** (if NOT using `/speckit.auto`):

```bash
lm recall "<task keywords>" --limit 5
lm domain  # Verify domain resolution
```

**After significant work** (importance >= 8 only):

```bash
lm remember "<insight>" --type <TYPE> --importance 8 --tags "component:..."
```

**Canonical types**: `decision`, `pattern`, `bug-fix`, `milestone`, `discovery`, `limitation`, `architecture`

**Policy reference**: `~/.claude/skills/local-memory/SKILL.md`

## Spec-Kit Workflow

This project uses spec-kit for structured development:

* `/speckit.new <description>` - Create new SPEC
* `/speckit.auto SPEC-ID` - Full automation pipeline (Stage0 handles memory)
* `/speckit.status SPEC-ID` - Check progress

## Getting Started

1. Define your first feature with `/speckit.new`
2. Review generated PRD in `docs/SPEC-*/PRD.md`
3. Run `/speckit.auto` to implement (recommended: uses Stage0)

## Project Structure

* `docs/` - SPEC directories and documentation
* `memory/` - Project charter and context
* `SPEC.md` - Task tracking table

## Build Commands

```bash
[BUILD_COMMAND]
```

## Testing

```bash
[TEST_COMMAND]
```
