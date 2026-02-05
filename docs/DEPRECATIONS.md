# Deprecations Register

Single canonical list of deprecated/superseded documentation and their replacements.

**Why this exists**

- Reduce roadmap/spec drift by making deprecations explicit and discoverable.
- Preserve historical context without letting legacy docs override current truth.

**Policy**

- When a doc is no longer canonical, add a **top-of-file banner** in the doc and an entry here.
- Deprecated PRDs are **archived into zip packs under `archive/` and removed from the working tree** (no stubs); this register is the canonical pointer.
- For historical/frozen docs (notably under `docs/SPEC-KIT-*`), prefer adding an entry here and only add minimal banner text when needed.

**Status meanings**

- **Deprecated**: Retained for history; do not treat as current guidance.
- **Superseded**: Replaced by a newer doc (link provided); old doc should not be updated further.
- **Needs refresh**: Still useful, but not safe to treat as canonical until updated.

## Register

| Document | Status | Replacement / Canonical Reference | Notes | Deprecated On |
| --- | --- | --- | --- | --- |
| `codex-rs/docs/NEXT_FOCUS_ROADMAP.md` | Deprecated | `SPEC.md` → `codex-rs/SPEC.md` | Historical roadmap; conflicts resolved by `codex-rs/SPEC.md` doc precedence order. | 2026-02-05 |
| `codex-rs/docs/SPEC-KIT-900-gold-run/spec.md` | Superseded | `docs/SPEC-DOGFOOD-002/spec.md` + `codex-rs/SPEC.md` (Planned) | `SPEC-KIT-900` is completed work; gold-run validation is tracked separately as `SPEC-DOGFOOD-002`. | 2026-02-05 |
| `codex-rs/docs/GOLD_RUN_PLAYBOOK.md` | Needs refresh | `docs/SPEC-DOGFOOD-002/spec.md` + `codex-rs/SPEC.md` (Planned) | Keep as playbook, but acceptance criteria lives in the SPEC and tracker. | 2026-02-05 |

## Planned: Capsule-backed tracking

Long-term, deprecations should be emitted as capsule events and projected into this register. Track design/implementation in `codex-rs/SPEC.md` (Planned: `SPEC-PM-001`).
