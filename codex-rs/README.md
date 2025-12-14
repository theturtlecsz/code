# Planner (Rust Workspace)

This directory is the Rust workspace that builds the `code` binary (Planner).

Canonical docs live at `../docs/KEY_DOCS.md`.

## Build & Run (Recommended)

From the repo root:

```bash
./build-fast.sh run
```

## Command Surface (High Level)

- Default UX: interactive TUI (run `code` with no subcommand).
- Primary workflows: TUI slash commands under `/speckit.*` (and `/guardrail.*` where applicable).
- Non-interactive mode: `code exec "PROMPT"` (prints output to stdout/stderr and exits).

## Configuration

- Configuration is loaded from `~/.code/config.toml` (some legacy reads from `~/.codex/` exist for compatibility).
- Configuration reference: `../docs/config.md`.

## MCP (Model Context Protocol)

- Planner can act as an MCP client and connect to MCP servers on startup (see `../docs/config.md#mcp_servers`).
- Planner can also launch as an MCP server via the `code mcp` subcommand.

## ACE (Local Strategy Memory) (Optional)

ACE (Agentic Context Engine) provides data-only “playbook” memory for Spec‑Kit workflows via MCP.

- Usage and configuration: `ACE_LEARNING_USAGE.md`
- Pin constitution bullets: `/speckit.constitution`

## Shell Completions

```shell
code completion bash
code completion zsh
code completion fish
```

## Sandbox + Debug Helpers

### Experimenting with sandboxing

On supported platforms, you can run sandbox debug helpers:

```text
# macOS
code debug seatbelt [--full-auto] [COMMAND]...

# Linux
code debug landlock [--full-auto] [COMMAND]...
```

### Selecting a sandbox policy via `--sandbox`

```shell
# Default sandbox
code --sandbox read-only

# Allow writes inside workspace
code --sandbox workspace-write

# Danger: disable sandboxing (use only in an isolated environment)
code --sandbox danger-full-access
```

The same setting can be persisted in `~/.code/config.toml` via `sandbox_mode = "MODE"`.

## Debugging Virtual Cursor

Use these console helpers to diagnose motion/cancellation behavior when testing in a real browser:

- Disable clickPulse transforms and force long CSS duration:

  `window.__vc && (window.__vc.clickPulse = () => (console.debug('[VC] clickPulse disabled'), 0), window.__vc.setMotion({ engine: 'css', cssDurationMs: 10000 }))`

- Wrap `moveTo` to log duplicates with sequence and inter-call delta:

  `(() => { const vc = window.__vc; if (!vc || vc.__wrapped) return; const orig = vc.moveTo; let seq=0, last=0; vc.moveTo = function(x,y,o){ const now=Date.now(); console.debug('[VC] moveTo call',{seq:++seq,x,y,o,sincePrevMs:last?now-last:null}); last=now; return orig.call(this,x,y,o); }; vc.__wrapped = true; console.debug('[VC] moveTo wrapper installed'); })();`

- Trigger a test move (adjust coordinates as needed):

  `window.__vc && window.__vc.moveTo(200, 200)`

## Code Organization

This folder is the root of a Cargo workspace. Many crates retain upstream naming (`codex-*`) even though the product is Planner.

- [`cli/`](./cli): builds the `code` multitool binary (interactive by default).
- [`tui/`](./tui): fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`exec/`](./exec): “headless” execution mode for automation.
- [`spec-kit/`](./spec-kit): shared Spec‑Kit library crate.
- [`core/`](./core): core business logic and protocol/types used by the CLI/TUI.

## Fork Lineage

- Repo: https://github.com/theturtlecsz/code
- Upstream: https://github.com/just-every/code
- Note: “Codex” refers to model naming and historical lineage; the product/CLI in this repo is Planner (binary: `code`).
