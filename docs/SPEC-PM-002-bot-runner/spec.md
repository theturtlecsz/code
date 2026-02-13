# SPEC-PM-002: Bot Runner Interface (NeedsResearch / NeedsReview)

## Status: PLANNED (interface contract)

## Overview

Define the **product semantics** and Tier‑1 **interaction contract** for manual “Devin-style” automation bots that a PM can trigger by placing a work item into:

* `NeedsResearch` (run research bots)
* `NeedsReview` (run review/validator bots)

This document is intentionally an **interface spec**: how callers interact with bot runs (CLI/TUI/headless), what inputs are required, what outputs/artifacts are produced, and what safety/exit-code guarantees are provided.

The underlying **bot system architecture** (runner/service/tooling internals, queueing, IPC, allowlists) is tracked separately in `SPEC-PM-003`.

PRD: `docs/SPEC-PM-002-bot-runner/PRD.md`

## Definitions (v1)

* **Work Item**: A capsule-backed PM object with a stable ID and lifecycle state (defined in `SPEC-PM-001`).
* **Bot Kind**: `research | review`
* **Bot Run**: A single execution of a bot kind against a specific work item, identified by a unique `run_id`.
* **Write Mode** (review only): `none | worktree`
  * `none`: bot is read-only with respect to the repo
  * `worktree`: bot may stage suggested changes in a bot-owned worktree/branch
* **Run Configuration**: caller-provided settings that shape the run (e.g., an intensity preset plus include/exclude toggles for analysis scopes).
* **Artifact URI**: A capsule logical URI referencing an immutable artifact (authoritative SoR).
* **Filesystem Projection**: Best-effort human-readable mirror of artifacts; rebuildable from capsule state (not authoritative).

## Goals

* Define the minimum viable behaviors for:
  * Research bots (`NeedsResearch`)
  * Review bots (`NeedsReview`)
* Define Tier‑1 command surfaces (CLI/TUI/headless) and required parity.
* Define headless behavior contract (structured output + product exit codes; never prompt).
* Define safety boundaries as caller-visible guarantees (read-only default; write mode isolation).
* Define artifacts produced and their projection expectations.

## Non-Goals (initial)

* Defining the internal runner/service architecture (tracked in `SPEC-PM-003`).
* Running bots automatically on every PR/spec by default.
* Cross-platform support (Linux-only remains baseline).
* Auto-committing/pushing/merging as a default mode.
* Auto-transitioning PM lifecycle states **based on bot recommendations** (state changes remain an explicit PM action), except the locked holding-state **auto-return** behavior (D140).

### Engine Status (PM-D30)

Real research/review engines are deferred to a separate spec cycle. The next cycle uses **stub engines** that immediately succeed. The full run configuration surface (presets + scopes) must be defined and functional even with stubs, so engines can plug in cleanly when ready.

## Tier‑1 Constraints (Already Locked)

* **Multi-surface parity** (D113/D133): CLI/TUI/headless must share semantics for Tier‑1 behavior (commands, artifacts, gating semantics, exit codes).
* **Headless never prompts** (D133): missing requirements must return product exit codes + structured output.
* **Maieutic step always mandatory** (D130): bot automation must not create a bypass for required gates (especially `/speckit.auto`).
* **Explainability follows capture mode** (D131) + **over-capture is hard-blocked** (D119): artifacts must never exceed policy capture bounds; `capture=none` persists no explainability artifacts.
* **No silent destructive actions in headless** (NFR3 in `SPEC-PM-001`): write actions must be explicit and auditable.

## Product Semantics (v1)

### Holding States (D140)

* `NeedsResearch` and `NeedsReview` are **manual holding states** in the PM lifecycle (defined in `SPEC-PM-001`).
* Entering a holding state does **not** automatically start a bot; it only makes the item eligible for a manual bot run.
* Per D140, entering a holding state must record a `return_state`. When the bot run reaches a terminal state, the system auto-transitions the work item back to `return_state` (this is not a "bot recommendation" transition).

### State Transition Rules (v1)

* Bot runs do **not** automatically promote/demote work items based on bot recommendations.
* The only automatic transition is the D140 holding-state auto-return to `return_state`.
* A bot run may recommend a next state (e.g. “promote to Planned”), but the PM performs that state change explicitly.

### Visibility (Tier‑1)

* The current holding state and the latest bot run summary must be visible across CLI/TUI/headless status surfaces.
* Long-form details live in artifacts (capsule SoR) with projections for humans.
* Long-lived runs should expose a “latest checkpoint” summary so callers can understand progress without requiring streaming logs.

## Inputs

* Work item + attached PRD/intake form data.
* Capsule artifacts linked to the work item (intake/grounding/reports/evidence).
* `NeedsResearch` dependency posture is **policy-defined**:
  * If degraded operation is disallowed (`allow_degraded=false`) and NotebookLM (or equivalent Tier‑2 grounding) is unavailable, the run terminates as **BLOCKED** with structured output.
  * If degraded operation is allowed (`allow_degraded=true`; current implementation default), the run proceeds in degraded mode and outputs must be labeled degraded and preserve replay/audit inputs.
* Web research is allowed via both:
  * Tavily MCP (preferred; pinned locally), and
  * the client’s default/generic web research tooling.

## Outputs (Artifacts)

Artifact types (schemas start at v0; additive-only until locked):

* `ResearchReport`: synthesis + recommended options/tradeoffs (references `WebResearchBundle` as needed).
* `ReviewReport`: structured review notes with file/line references + risk assessment.
* `BotRunLog`: timing/cost summary + tool usage + success/failure diagnostics.
* `BotRunCheckpoint` (optional, long-lived runs): latest progress summary + resume metadata (no over-capture).
* `WebResearchBundle`: structured web research capture (defined in `SPEC-PM-001`; reused here).
* `PatchBundle` (review only, write mode): patch/diff + worktree/branch metadata + apply/inspect instructions.

All artifacts must respect capture mode (`none | prompts_only | full_io`) and export safety constraints (locked by policy).

## Command Surface (Tier‑1, proposal)

> Canonical CLI namespace remains: `code speckit pm ...` (see `SPEC-PM-001`).

### Run

* `code speckit pm bot run --id <WORK_ITEM_ID> --kind research [--wait]`
* `code speckit pm bot run --id <WORK_ITEM_ID> --kind review [--write-mode worktree] [--wait]`

Run configuration (locked; PM-D31):

* `--preset <name>`: named intensity preset (required; default: `standard`)
  * `quick`: minimal pass; surface-level checks only; minutes
  * `standard`: balanced analysis; recommended for most runs; minutes to hours
  * `deep`: thorough analysis with expanded source coverage; hours
  * `exhaustive`: comprehensive multi-pass analysis; hours to days
* `--scope <name>` / `--no-scope <name>`: include/exclude analysis scopes
  * Available scopes: `correctness`, `security`, `performance`, `style`, `architecture`
  * Default: all scopes included at preset intensity
  * Example: `--preset deep --no-scope style` (deep analysis, skip style checks)

### Status + Results

* `code speckit pm bot status --id <WORK_ITEM_ID> [--json]`
* `code speckit pm bot runs --id <WORK_ITEM_ID> [--limit N] [--json]`
* `code speckit pm bot show --id <WORK_ITEM_ID> --run <RUN_ID> [--format md|json]`
* `code speckit pm bot cancel --id <WORK_ITEM_ID> --run <RUN_ID> [--json]`
* `code speckit pm bot resume --id <WORK_ITEM_ID> --run <RUN_ID> [--json]`

### Service Management (Tier‑1, proposal)

* `code speckit pm service status [--json]`
* `code speckit pm service doctor [--json]`

### TUI Aliases (Tier‑1 parity)

* `/pm bot run <WORK_ITEM_ID> research`
* `/pm bot run <WORK_ITEM_ID> review`
* `/pm bot show <WORK_ITEM_ID> <RUN_ID>`

## Safety + Write Modes (caller-visible contract)

### Default

* Default is **read-only** with respect to the repo.
* Bot runs must never commit/push/merge by default.

### Review Write Mode: `worktree`

When `--write-mode worktree` is enabled for `NeedsReview` runs:

* All changes are staged in a bot-owned worktree/branch (the user’s primary working tree is not modified by default).
* The run result must include:
  * `worktree_path` (if created)
  * `branch_name` (if created)
  * A `PatchBundle` artifact URI containing a diff/patch reference and safe apply/inspect instructions

Worktree/branch naming conventions and enforcement details are specified in `SPEC-PM-003`.

## Headless Contract (Tier‑1, proposal)

### Never Prompt

* Headless mode must never prompt (D133). Missing requirements must return:
  * A non-zero exit code, and
  * Structured JSON describing the missing requirement and resolution steps.

### Exit Codes (proposal)

`pm bot run` is **async by default**: it submits a run request to the bot service, returns `run_id` + initial status JSON, and exits immediately.

Exit codes:

* For `pm bot run` (default, submit-and-exit):
  * `0`: submitted (a `run_id` was created/returned; completion is reported via `status`/`show` or `--wait`)
  * `3`: infrastructure error (unable to submit: service failure, I/O, capsule corruption)
  * `10`: needs input (invalid request / work item missing required data / work item not eligible / missing required flags)
  * `11`: needs approval (write-mode requested but not explicitly allowed in headless policy/config)
  * `13`: invariant violation (headless attempted to prompt)

- For `pm bot run --wait` (subscribe and wait for a terminal notification, then exit):
  * `0`: terminal `succeeded`
  * `2`: terminal `blocked`, `cancelled`, or `needs_attention` (D139; check `status` and `blocked_reason` in JSON for disambiguation)
  * `3`: terminal `failed`

### JSON Output Schema (v0)

All headless bot-run commands must emit JSON with:

* `schema_version`
* `tool_version`
* `work_item_id`
* `kind` (`research | review`)
* `run_id`
* `status` (`queued | running | succeeded | failed | blocked | needs_attention | cancelled`)
* `exit_code`
* `summary` (short, human-readable)
* `artifact_uris[]` (capsule logical URIs for produced artifacts)
* `projection_paths[]` (filesystem projections written, if any)
* `errors[]` (structured; includes `blocked_reason` for `status=blocked`)

### Long-Lived Runs (proposal)

Long-lived runs must remain usable in headless mode without streaming UI:

* callers can query status and the latest checkpoint summary deterministically,
* resume must never prompt (missing inputs/prereqs become structured “needs input/blocked” outcomes),
* `pm bot run` is async by default; `--wait` is an explicit opt-in for synchronous scripting.

## Artifact Schemas (v0 — proposal)

### BotRunLog

* `schema_version`
* `tool_version`
* `work_item_id`, `run_id`, `kind`
* `started_at`, `finished_at`, `duration_ms`
* `capture_mode`
* `tool_usage[]` (tool name + counts + timing; no over-capture)
* `status` + `errors[]`

### BotRunCheckpoint (optional)

* `schema_version`
* `tool_version`
* `work_item_id`, `run_id`, `kind`
* `checkpoint_at` (RFC3339)
* `phase` (coarse step label)
* `summary` (short, human-readable)
* `percent` (optional; best-effort)
* `resume_hint` (optional; what the service/runner will do next)

### ResearchReport

* `schema_version`
* `tool_version`
* `work_item_id`, `run_id`
* `inputs` (artifact URIs used)
* `executive_summary`
* `options[]` (each with tradeoffs + recommendation)
* `open_questions[]`
* `citations[]` (references into `WebResearchBundle` entries)

### ReviewReport

* `schema_version`
* `tool_version`
* `work_item_id`, `run_id`
* `summary`
* `must_fix[]` (file/line references + rationale)
* `suggestions[]`
* `risks[]`
* `commands_ran[]` (allowlisted command record)
* `patch_bundle_uri` (optional)

### PatchBundle (review, write mode only)

* `schema_version`
* `tool_version`
* `work_item_id`, `run_id`
* `worktree_path` (if created)
* `branch_name` (if created)
* `diff_uri` (capsule artifact URI for patch/diff)
* `apply_instructions` (how to apply/inspect safely)

## Filesystem Projections (best-effort, proposal)

> Projections are rebuildable from capsule artifacts; they are not authoritative (D114).

* Proposed root: `docs/specs/<WORK_ITEM_ID>/runs/bot/<RUN_ID>/`
* Suggested layout:
  * `run_log.json`
  * `research/research_report.md`
  * `research/web_research_bundle.json` (if captured)
  * `review/review_report.md`
  * `review/patch.diff` (if produced)

Capture-mode compliance:

* `capture=none`: rejected for bot runs (return “needs input”, exit code `10`).
* `prompts_only`: projections must be export-safe (bounded snippets + hashes as defined by `WebResearchBundle` rules in `SPEC-PM-001`).
* `full_io`: projections may include extracted content, but must remain excluded from safe export per policy.

## Minimal MVP (suggested)

* A command to place an item into `NeedsResearch` and run a single research pass that emits:
  * `WebResearchBundle` + a short `ResearchReport` (structured JSON + Markdown projection).
* A command to place an item into `NeedsReview` and run a single deterministic/static review pass that emits:
  * `ReviewReport` with "must fix" vs "suggestions" plus a summarized risk list.

## Open Questions

* ~~Should BLOCKED be a dedicated exit code~~ Resolved: exit 2 + structured `blocked_reason` (D139).
* Do we require optional **streaming progress** (NDJSON) for long runs, or is final JSON sufficient for v1? **Lower priority** (stubs are instant).
* ~~Canonical preset names~~ Resolved: `quick | standard | deep | exhaustive` (PM-D31).
* ~~Canonical scope toggles~~ Resolved: `correctness | security | performance | style | architecture` (PM-D31).
* ~~Filesystem projection root~~ Resolved: `docs/specs/<ID>/runs/` (PM-003 spec).

## References

* PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`
* Bot system architecture: `docs/SPEC-PM-003-bot-system/spec.md`
* Historical stub: `docs/SPEC-PM-001-project-management/TODO-bot-runner-spec.md`
