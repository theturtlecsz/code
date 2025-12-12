## Advanced

## Non-interactive / CI mode

Run `code` headless in pipelines. Example GitHub Action step (build-from-source):

```yaml
- name: Update changelog via Planner
  run: |
    ./build-fast.sh
    export OPENAI_API_KEY="${{ secrets.OPENAI_KEY }}"
    ./codex-rs/target/dev-fast/code exec --full-auto "update CHANGELOG for next release"
```

### Resuming non-interactive sessions

You can resume a previous headless run to continue the same conversation context and append to the same rollout file.

Interactive TUI equivalent:

```shell
code resume             # picker
code resume --last      # most recent
code resume <SESSION_ID>
```

Compatibility:

- Source builds include `code exec resume` (examples below).

```shell
# Resume the most recent recorded session and run with a new prompt (source builds)
code exec "ship a release draft changelog" resume --last

# Alternatively, pass the prompt via stdin (source builds)
# Note: omit the trailing '-' to avoid it being parsed as a SESSION_ID
echo "ship a release draft changelog" | code exec resume --last

# Or resume a specific session by id (UUID) (source builds)
code exec resume 7f9f9a2e-1b3c-4c7a-9b0e-123456789abc "continue the task"
```

Notes:

- When using `--last`, Planner picks the newest recorded session; if none exist, it behaves like starting fresh.
- Resuming appends new events to the existing session file and maintains the same conversation id.

## Tracing / verbose logging

Because Planner is written in Rust, it honors the `RUST_LOG` environment variable to configure its logging behavior.

The TUI defaults to `RUST_LOG=codex_core=info,codex_tui=info` and log messages are written to `~/.code/log/codex-tui.log` (Planner still reads the legacy `~/.codex/log/` path), so you can leave the following running in a separate terminal to monitor log messages as they are written:

```
tail -F ~/.code/log/codex-tui.log
```

By comparison, the non-interactive mode (`code exec`) defaults to `RUST_LOG=error`, but messages are printed inline, so there is no need to monitor a separate file.

See the Rust documentation on [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) for more information on the configuration options.

## Model Context Protocol (MCP)

Planner can be configured to leverage MCP servers by defining an [`mcp_servers`](./config.md#mcp_servers) section in `~/.code/config.toml` (Planner will also read a legacy `~/.codex/config.toml`). It is intended to mirror how tools such as Claude and Cursor define `mcpServers` in their respective JSON config files, though the format here is TOML rather than JSON, e.g.:

```toml
# IMPORTANT: the top-level key is `mcp_servers` rather than `mcpServers`.
[mcp_servers.server-name]
command = "npx"
args = ["-y", "mcp-server"]
env = { "API_KEY" = "value" }
```

## Using `code` as an MCP Server
> [!TIP]
> It is somewhat experimental, but the `code` binary can also be run as an MCP _server_ via `code mcp`. If you launch it with an MCP client and send it a `tools/list` request, you will see a single tool that accepts a grab-bag of inputs, including a catch-all `config` map for anything you might want to override.
