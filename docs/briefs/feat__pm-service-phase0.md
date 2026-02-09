# Session Brief — feat/pm-service-phase0

**Date**: 2026-02-09

## Goal

Land the Phase-0 PM bot service walking skeleton as a clean, reviewable PR.

Implements:

* Core PM types (work items, artifacts) in `codex-core`
* `codex-pm-service` crate: IPC server, bot run manager, stub engines
* CLI surface (`speckit pm`) for submitting and managing bot runs
* systemd user unit for service lifecycle

## Scope

* Work item and artifact types (`codex-rs/core/src/pm/`)
* PM service crate (`codex-rs/pm-service/`)
* CLI subcommands (`codex-rs/cli/src/pm_cmd.rs`)
* systemd unit file (`systemd/codex-pm-service.service`)

## Non-Goals

* Real research/review engines (Phase-2)
* Capsule persistence (Phase-1)
* Socket activation / exit-when-idle (Phase-1)

## Decisions Referenced

* D135: Lightweight service, systemd-managed
* D136: Unix domain socket IPC
* PM-D1 through PM-D24: Architecture decisions for async bot runs

## Validation

* `cargo test -p codex-core --lib -- pm::`
* `cargo test -p codex-pm-service`
* `cargo clippy --workspace --all-targets --all-features -- -D warnings`
* `cargo fmt --all -- --check`
* `bash .githooks/pre-commit`

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

## Product Knowledge (auto)

- Query: `PM bot service walking skeleton phase-0`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-feat-pm-service-phase0/artifact/briefs/feat__pm-service-phase0/pending.md`
- Capsule checkpoint: `brief-feat__pm-service-phase0-pending`

No capsule checkpoint available yet. Brief created manually for Phase-0 landing.

<!-- END: SPECKIT_BRIEF_REFRESH -->
