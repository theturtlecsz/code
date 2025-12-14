# Planner

&ensp;

<p align="center">
  <img src="docs/logo.png" alt="Planner Logo" width="400">
</p>

&ensp;

Planner is a terminal TUI focused on **Spec-Kit workflows** (slash commands under `/speckit.*`).

The executable name remains `code` and is built from source.

## What Is Spec-Kit?

Spec-Kit is a structured workflow for turning a feature description into a SPEC directory and running staged execution (plan → tasks → implement → validate → audit → unlock) with artifacts stored under `docs/`.

## What Problems It Solves

- Scaffold new projects with Spec-Kit workflow files
- Create SPEC directories and stage artifacts (spec/plan/tasks)
- Run an end-to-end pipeline with evidence captured under `docs/`

## Quickstart

```bash
bash scripts/setup-hooks.sh
./build-fast.sh run
```

In the TUI:

```text
/speckit.project rust my-rust-lib   (optional)
/speckit.new <feature description>
/speckit.auto SPEC-KIT-###
```

## How It Works

- Spec-Kit is exposed as `/speckit.*` slash commands.
- The implementation lives in `codex-rs/tui/src/chatwidget/spec_kit/` (TUI + orchestration) and `codex-rs/spec-kit/` (shared library).

## Safety Model

- Removed legacy commands (`/plan`, `/solve`, `/code`) show a migration message and do not run tools.

## Configuration

- See `docs/config.md`.

## Documentation

- Start here: `docs/KEY_DOCS.md`
- Vision: `docs/VISION.md`
- Removed commands: `docs/DEPRECATIONS.md`

## Development

Build:

```bash
./build-fast.sh
./build-fast.sh run
PROFILE=release ./build-fast.sh
```

Tests:

```bash
cd codex-rs
cargo test -p codex-core
```
