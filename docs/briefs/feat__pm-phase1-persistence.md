# feat/pm-phase1-persistence

Phase-1 PM bot service: persistence layer, capsule dual-write, auto-resume,
systemd socket activation, and CLI `--wait` support.

## Scope

* Artifact persistence (local cache + capsule)
* Engine expansion (research/review bot execution, NotebookLM integration)
* IPC expansion (bot.run, bot.status, bot.show, bot.runs, bot.cancel, bot.resume)
* CLI `pm bot run --wait` with terminal notifications
* systemd units for socket activation and auto-resume after reboot (D135)
* Walking skeleton acceptance tests including D135 compliance

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

## Product Knowledge (auto)

* Query: `pm-phase1-persistence capsule dual-write auto-resume`
* Domain: `codex-product`
* Capsule URI: `mv2://default/WORKFLOW/brief-20260211T142347Z/artifact/briefs/feat__pm-phase1-persistence/20260211T142347Z.md`
* Capsule checkpoint: `brief-feat__pm-phase1-persistence-20260211T142347Z`

Decision locks: D135 (auto-resume), D136 (capsule dual-write).
47 PM-service tests + 10 codex-core pm tests passing.

<!-- END: SPECKIT_BRIEF_REFRESH -->
