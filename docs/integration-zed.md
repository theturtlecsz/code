# Zed Integration (ACP/MCP, experimental)

Planner includes an experimental ACP-compatible server exposed via `code mcp` (alias: `code acp`).

To point Zed at the ACP server, add this block to `settings.json` (use an absolute path to avoid collisions with other `code` binaries on your system):

```jsonc
{
  "agent_servers": {
    "Planner": {
      "command": "/absolute/path/to/code",
      "args": ["mcp"]
    }
  }
}
```

Build-from-source tip: after running `./build-fast.sh`, a typical dev build lives at `codex-rs/target/dev-fast/code`.

## Zed prerequisites

- Zed Stable `0.201.5` (released August 27, 2025) or newer adds ACP support with the Agent Panel. Update via `Zed → Check for Updates` before wiring Code in. Zed’s docs call out ACP as the mechanism powering Gemini CLI and other external agents.
- Zed Stable `0.201.5` (released August 27, 2025) or newer adds ACP support with the Agent Panel. Update via `Zed → Check for Updates` before wiring Planner in. Zed’s docs call out ACP as the mechanism powering Gemini CLI and other external agents.
- External agents live inside the Agent Panel (`cmd-?`). Use the `+` button to start a new thread and pick `Planner` from the external agent list. Zed runs our CLI as a subprocess over JSON‑RPC, so all prompts and diff previews stay local.

## How Planner implements ACP

- The Rust MCP server exposes ACP tools: `session/new`, `session/prompt`, and fast interrupts via `session/cancel`. These are backed by the same conversation manager that powers the TUI, so approvals, confirm guards, and sandbox policies remain intact.
- Streaming `session/update` notifications bridge Planner events into Zed. You get Answer/Reasoning updates, shell command progress, approvals, and apply_patch diffs in the Zed UI without losing terminal parity.
- MCP configuration stays centralized in `CODEX_HOME/config.toml`. Use `[experimental_client_tools]` to delegate file read/write and permission requests back to Zed when you want its UI to handle approvals.

## Tips and troubleshooting

- Need to inspect the handshake? Run Zed’s `dev: open acp logs` command from the Command Palette; the log shows JSON‑RPC requests and Planner replies.
- If prompts hang, make sure no other process is bound to the same MCP port and that your `CODEX_HOME` points to the intended config directory. The ACP server inherits all of Planner’s sandbox settings, so restrictive policies (e.g., `approval_policy = "never"`) still apply.
- Zed currently skips history restores and checkpoint UI for third-party agents. Stick to the TUI if you rely on those features; ACP support is still evolving upstream.
