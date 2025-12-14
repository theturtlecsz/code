# Getting Started

## Build & Run

From the repo root:

```bash
bash scripts/setup-hooks.sh
./build-fast.sh run
```

This runs the `code` binary (Planner) with the interactive TUI.

## First Spec-Kit Workflow

In the TUI:

1. (Optional) Scaffold a new project:
   - `/speckit.project rust my-rust-lib`
2. Create a new SPEC:
   - `/speckit.new <feature description>`
3. Run the pipeline:
   - `/speckit.auto SPEC-KIT-###`

See `docs/config.md` for configuration and templates.
