# Tasks: SPEC-PM-006 Packet Persistence

## Priority Tasks

- [ ] **T001** Define packet schema (`packet.yaml`) and versioning contract.
  **Validation**: `cd codex-rs && cargo test -p codex-core packet::schema`
  **Artifact**: `docs/SPEC-PM-006-packet-persistence/artifacts/schema/packet-schema-v1.yaml`

- [ ] **T002** Implement atomic packet writer and shared parser.
  **Validation**: `cd codex-rs && cargo test -p codex-core packet::atomic_write`
  **Artifact**: `docs/SPEC-PM-006-packet-persistence/artifacts/tests/atomic-write.txt`

- [ ] **T003** Enforce sacred-anchor immutability + amendment hooks.
  **Validation**: `cd codex-rs && cargo test -p codex-core packet::sacred_anchor_guard`
  **Artifact**: `docs/SPEC-PM-006-packet-persistence/artifacts/tests/anchor-guard.txt`

- [ ] **T004** Integrate startup restore into TUI/CLI/headless boot flow.
  **Validation**: `cd codex-rs && cargo test -p codex-tui packet::restore_on_startup`
  **Artifact**: `docs/SPEC-PM-006-packet-persistence/artifacts/tests/restart-restore.txt`

- [ ] **T005** Add corruption handling and deterministic recovery guidance.
  **Validation**: `cd codex-rs && cargo test -p codex-cli packet::recovery_messages`
  **Artifact**: `docs/SPEC-PM-006-packet-persistence/artifacts/tests/recovery.txt`

- [ ] **T006** Validate docs and command references.
  **Validation**: `python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-006-packet-persistence/artifacts/docs/doc-lint.txt`

## Definition of Done

- Packet survives restarts and preserves sacred/milestone state.
- All Tier-1 surfaces consume the same packet parsing/writing logic.
- Recovery paths are deterministic and user-actionable.
