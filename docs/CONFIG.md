# Configuration

## File Location

Planner (the `code` binary) reads configuration from:

- `~/.code/config.toml` (primary)
- legacy `~/.codex/*` may be read for compatibility in some areas

## Spec-Kit Settings (Common)

- Templates:
  - Project-local `./templates/*.md` (recommended for customization)
  - Embedded templates (fallback)
- Evidence:
  - `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

## Development Notes

- The Rust workspace is not published to crates.io (`codex-rs/Cargo.toml` sets `publish = false`).

