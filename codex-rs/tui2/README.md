# codex-tui2 -- SCAFFOLD ONLY

> **This crate is NOT the production UI.**
> `tui` (codex-tui) is the canonical, production TUI with full Spec-Kit support.
> `tui2` is an upstream-aligned scaffold retained for sync friction reduction and selective backports.

## What tui2 IS

* An upstream-aligned (just-every/code) viewport-style UI scaffold
* A reference for testing upstream UI behavior and contracts
* A source for selective cherry-picks into the production `tui`

## What tui2 is NOT

* **Not a replacement for tui** -- see [ADR-002](../../docs/adr/ADR-002-tui2-purpose-and-future.md)
* **Not a Spec-Kit surface** -- `/speckit.*` commands, Stage0, pipeline orchestration all live in `tui`
* **Not the default build target** -- excluded from `default-members` in workspace Cargo.toml

## Building

tui2 is excluded from default workspace builds. To build it explicitly:

```bash
cd codex-rs
cargo build -p codex-tui2
cargo test -p codex-tui2
```

## Prohibited

The following MUST NOT be added to tui2 (enforced by CI guardrail):

* Spec-Kit integration (`spec_kit::`, `codex_spec_kit`, `chatwidget/spec_kit`)
* Stage0 orchestration (`codex_stage0`, `codex-stage0`)
* Slash command implementations (`/speckit.*`)

If shared functionality is needed by both UIs, it must be extracted into a shared core crate first (per ADR-002 constraint 3).

## Stub Inventory

See [docs/SPEC-TUI2-STUBS.md](../../docs/SPEC-TUI2-STUBS.md) for what is stubbed/missing compared to `tui`.

## References

* [ADR-002: TUI2 Purpose and Future](../../docs/adr/ADR-002-tui2-purpose-and-future.md)
* [Constitution](../../memory/constitution.md) -- "tui is primary; tui2 is upstream scaffold/reference only"
* [VISION.md](../../docs/VISION.md) -- Spec-Kit lives in tui
