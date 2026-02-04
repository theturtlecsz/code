# Spec-Kit Commands Reference

Consolidated reference for all `/speckit.*` TUI commands and `code speckit` CLI commands.

***

## TUI Commands

Interactive commands available in the TUI chat interface.

| Command                             | Aliases           | Description                           | SPEC             |
| ----------------------------------- | ----------------- | ------------------------------------- | ---------------- |
| `/speckit.new <AREA> <desc>`        | `/spec.new`       | Create new SPEC with intake questions | -                |
| `/speckit.projectnew <type> <name>` | -                 | Create project scaffold with vision   | SPEC-KIT-960     |
| `/speckit.capsule <subcommand>`     | `/capsule.doctor` | Capsule management                    | SPEC-KIT-971/974 |
| `/speckit.projections rebuild`      | -                 | Regenerate filesystem from SoR        | WP-A             |
| `/speckit.status [spec-id]`         | -                 | Show SPEC status and progress         | -                |
| `/speckit.plan`                     | -                 | Generate implementation plan          | -                |
| `/speckit.quality`                  | -                 | Run quality gates                     | -                |
| `/speckit.msearch <query>`          | -                 | Memory search (hybrid retrieval)      | SPEC-KIT-972     |
| `/speckit.timeline`                 | -                 | Time-travel UI for capsule events     | SPEC-KIT-973     |
| `/speckit.reflex`                   | -                 | Local inference routing               | SPEC-KIT-978     |
| `/speckit.policy`                   | -                 | Policy management                     | SPEC-KIT-977     |

### Capsule Subcommands

Commands for `/speckit.capsule` (alias: `/capsule.doctor`):

| Subcommand                       | Description                                       |
| -------------------------------- | ------------------------------------------------- |
| `doctor`                         | Diagnose capsule health and integrity             |
| `stats`                          | Show storage statistics (events, artifacts, size) |
| `checkpoints`                    | List all checkpoints with labels                  |
| `commit <label>`                 | Create named checkpoint                           |
| `export [--spec ID] [--encrypt]` | Export capsule or spec to archive                 |
| `import <path>`                  | Import capsule from archive                       |
| `gc`                             | Garbage collection (remove unreferenced objects)  |

***

## CLI Commands

Headless commands for automation and scripting.

### `code speckit new`

Create a new SPEC from intake answers.

```bash
code speckit new --area <AREA> --desc "Feature description" --answers answers.json [--deep] [--json]
```

| Option                  | Description                                   |
| ----------------------- | --------------------------------------------- |
| `--area <AREA>`         | Feature area (required)                       |
| `--desc <text>`         | Feature description (required)                |
| `--answers <path>`      | Path to answers JSON file                     |
| `--answers-json <json>` | Inline answers JSON                           |
| `--deep`                | Enable deep mode (requires additional fields) |
| `--json`                | Output JSON for scripting                     |

### `code speckit projectnew`

Create a new project scaffold with vision.

```bash
code speckit projectnew <type> <name> --answers answers.json [--deep] [--json]
```

| Argument/Option       | Description                                                   |
| --------------------- | ------------------------------------------------------------- |
| `<type>`              | Project type: `rust`, `python`, `typescript`, `go`, `generic` |
| `<name>`              | Project name                                                  |
| `--answers <path>`    | Path to wrapper answers JSON                                  |
| `--deep`              | Enable deep mode                                              |
| `--no-bootstrap-spec` | Skip bootstrap spec creation                                  |
| `--json`              | Output JSON for scripting                                     |

### `code speckit brief refresh`

Generate or update the current **feature-branch session brief** at:

```
docs/briefs/<branch>.md
```

Where `<branch>` is your git branch name with `/` replaced by `__` (same rule as `.githooks/pre-commit`).

```bash
code speckit brief refresh --query "Stage0" [--domain codex-product] [--limit 10] [--ollama-model qwen2.5:3b] [--dry-run] [--json]
```

| Option                   | Description                                        |
| ------------------------ | -------------------------------------------------- |
| `--query <text>`         | Search query for product knowledge                 |
| `--domain <domain>`      | local-memory domain (default: `codex-product`)     |
| `--limit <n>`            | Max results from local-memory (default: 10)        |
| `--max-content-length n` | Max characters per memory item (default: 800)      |
| `--ollama-model <model>` | Ollama model for synthesis (default: `qwen2.5:3b`) |
| `--dry-run`              | Print the generated block instead of writing       |
| `--json`                 | Output JSON for scripting                          |

### `code speckit brief init`

Initialize the current feature-branch session brief with a minimal template.

```bash
code speckit brief init [--force] [--json]
```

| Option    | Description                            |
| --------- | -------------------------------------- |
| `--force` | Overwrite existing brief with template |
| `--json`  | Output JSON for scripting              |

**Behavior**:

* On `main` or detached HEAD: exits with error (briefs are only for feature branches)
* If brief exists and non-empty: no-op (exits 0)
* If brief missing or empty: creates from template

### `code speckit brief check`

Validate the current branch session brief exists and is non-empty.

```bash
code speckit brief check [--json] [--require-refresh-block]
```

| Option                    | Description                          |
| ------------------------- | ------------------------------------ |
| `--json`                  | Output JSON for scripting            |
| `--require-refresh-block` | Also require the auto-refresh marker |

**Exit codes**: 0 (valid), 2 (missing/empty), 3 (infrastructure error)

### `code speckit projections rebuild`

Regenerate filesystem projections from capsule SoR.

```bash
code speckit projections rebuild [--spec ID] [--project ID] [--no-vision] [--dry-run] [--json]
```

| Option           | Description                |
| ---------------- | -------------------------- |
| `--spec <ID>`    | Rebuild specific SPEC      |
| `--project <ID>` | Rebuild specific project   |
| `--no-vision`    | Skip vision regeneration   |
| `--dry-run`      | Show what would be written |
| `--json`         | Output JSON for scripting  |

***

## Exit Codes

Standard exit codes for CLI commands (headless automation):

| Code | Constant      | Meaning                                    |
| ---- | ------------- | ------------------------------------------ |
| 0    | `SUCCESS`     | Operation completed successfully           |
| 2    | `HARD_FAIL`   | Validation failure, deep grounding failure |
| 3    | `INFRA_ERROR` | Capsule I/O error, network error           |
| 10   | `NEEDS_INPUT` | Missing required input (answers, intake)   |

***

## Deep Mode

Deep mode (`--deep`) requires additional intake fields for production-ready specs:

**Required for specs:**

* `security_posture` - Security considerations
* `release_rollout` - Deployment strategy
* At least 5 acceptance criteria

**Required for projects:**

* Architecture sketch
* Threat model
* Ops baseline

Deep mode also captures grounding artifacts (Architect Harvest + Project Intel).

***

## Examples

### Create a SPEC interactively (TUI)

```
/speckit.new <AREA> Add user authentication with OAuth2
```

### Create a SPEC headlessly (CLI)

```bash
# Prepare answers file
cat > answers.json << 'EOF'
{
  "feature_scope": "User authentication via OAuth2 providers",
  "success_criteria": "Users can sign in with Google, GitHub",
  "technical_approach": "Use passport.js with OAuth2 strategy"
}
EOF

# Create spec
code speckit new --area <AREA> --desc "OAuth2 authentication" --answers answers.json --json
```

### Rebuild projections after capsule restore

```bash
code speckit projections rebuild --spec SPEC-KIT-042 --json
```

***

## See Also

* [CAPSULE-NAMESPACES.md](CAPSULE-NAMESPACES.md) - URI scheme documentation
* [../OPERATIONS.md](../OPERATIONS.md) - Operational playbook
* [../SPEC-KIT.md](../SPEC-KIT.md) - Spec-kit overview
